//! The two halves of DATA, one UART each, served from their RX interrupts: the answer to a poll byte must start
//! in the bit that follows it (`knowledge/linap.md`), whatever the Wi-Fi and MQTT tasks are doing.
use core::cell::RefCell;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use critical_section::Mutex;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::signal::Signal;
use enumset::EnumSet;
use esp_hal::gpio::Flex;
use esp_hal::gpio::interconnect::PeripheralInput;
use esp_hal::interrupt::InterruptHandler;
use esp_hal::time::Instant;
use esp_hal::uart::{Config, Instance, RxConfig, Uart, UartInterrupt};
use esp_hal::{Blocking, handler, ram};
use linap::{BIT_US, CommandState, Control, Frame, Mitm, PowerSide, PowerState, Wire, header};

/// Raised from the interrupts when something Home Assistant shows has changed.
pub static CHANGED: Signal<CriticalSectionRawMutex, ()> = Signal::new();

/// Frames heard, for the frame log; dropped when the log task lags behind.
pub static FRAMES: Channel<CriticalSectionRawMutex, Heard, 32> = Channel::new();
static FRAME_LOG: AtomicBool = AtomicBool::new(false);

/// Which half of DATA a frame was heard on.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// The power board's frames, and our answers to its polls.
    Power,
    /// The command board's answers to the polls we relay.
    Command,
}

#[derive(Clone, Copy)]
pub struct Heard {
    pub at_us: u64,
    pub side: Side,
    pub frame: Frame,
}

/// Idle time between the end of the poll byte and our answer, in half bits: the real command board leaves 1 bit.
static REPLY_GAP_HALF_BITS: AtomicU32 = AtomicU32::new(2);

static BUS: Mutex<RefCell<Option<Bus>>> = Mutex::new(RefCell::new(None));

const BAUD_RATE: u32 = 9600;

/// What the bus shows to Home Assistant.
#[derive(Clone, Copy, PartialEq)]
pub struct Snapshot {
    pub keys: Option<CommandState>,
    pub control: Control,
    pub power: Option<PowerState>,
    pub boost_us: u64,
    pub frame_log: bool,
    pub polls: u32,
    pub answered: u32,
}

/// One half of DATA: RX through the divider, TX through the NPN, which inverts it.
pub struct Half<U, Rx> {
    pub uart: U,
    pub rx: Rx,
    pub tx: Flex<'static>,
}

struct Bus {
    power: Uart<'static, Blocking>,
    command: Uart<'static, Blocking>,
    power_side: PowerSide,
    command_wire: Wire,
    mitm: Mitm,
    power_state: Option<PowerState>,
}

pub fn start<P, Pr, C, Cr>(power: Half<P, Pr>, command: Half<C, Cr>)
where
    P: Instance + 'static,
    C: Instance + 'static,
    Pr: PeripheralInput<'static>,
    Cr: PeripheralInput<'static>,
{
    let bus = Bus {
        power: open(power, on_power_bytes),
        command: open(command, on_command_bytes),
        power_side: PowerSide::default(),
        command_wire: Wire::default(),
        mitm: Mitm::new(),
        power_state: None,
    };
    critical_section::with(|cs| BUS.borrow_ref_mut(cs).replace(bus));
}

fn open<U: Instance + 'static, Rx: PeripheralInput<'static>>(
    half: Half<U, Rx>,
    handler: InterruptHandler,
) -> Uart<'static, Blocking> {
    let config = Config::default()
        .with_baudrate(BAUD_RATE)
        .with_rx(RxConfig::default().with_fifo_full_threshold(1).with_reported_errors(EnumSet::empty()));
    let mut uart = Uart::new(half.uart, config)
        .expect("UART configuration")
        .with_rx(half.rx)
        .with_tx(half.tx.into_peripheral_output().with_output_inverter(true));
    uart.set_interrupt_handler(handler);
    uart.clear_interrupts(UartInterrupt::RxFifoFull.into());
    uart.listen(UartInterrupt::RxFifoFull);
    uart
}

pub fn now_us() -> u64 {
    Instant::now().duration_since_epoch().as_micros()
}

/// Runs `f` on the MITM state, from a task.
pub fn with_mitm<T>(f: impl FnOnce(&mut Mitm, u64) -> T) -> Option<T> {
    critical_section::with(|cs| BUS.borrow_ref_mut(cs).as_mut().map(|bus| f(&mut bus.mitm, now_us())))
}

pub fn snapshot() -> Option<Snapshot> {
    critical_section::with(|cs| {
        BUS.borrow_ref(cs).as_ref().map(|bus| Snapshot {
            keys: bus.mitm.keys(),
            control: bus.mitm.control(),
            power: bus.power_state,
            boost_us: bus.mitm.boost_us(),
            frame_log: frame_log(),
            polls: bus.power_side.polls,
            answered: bus.power_side.answered,
        })
    })
}

pub fn set_frame_log(on: bool) {
    FRAME_LOG.store(on, Ordering::Relaxed);
}

pub fn frame_log() -> bool {
    FRAME_LOG.load(Ordering::Relaxed)
}

fn log_frame(side: Side, frame: Frame, at_us: u64) {
    if frame_log() {
        let _ = FRAMES.try_send(Heard { at_us, side, frame });
    }
}

pub fn reply_gap_half_bits() -> u32 {
    REPLY_GAP_HALF_BITS.load(Ordering::Relaxed)
}

pub fn set_reply_gap_half_bits(half_bits: u32) {
    REPLY_GAP_HALF_BITS.store(half_bits, Ordering::Relaxed);
}

#[handler(priority = esp_hal::interrupt::Priority::max())]
#[ram]
fn on_power_bytes() {
    critical_section::with(|cs| {
        if let Some(bus) = BUS.borrow_ref_mut(cs).as_mut() {
            bus.drain_power();
        }
    });
}

#[handler(priority = esp_hal::interrupt::Priority::max())]
#[ram]
fn on_command_bytes() {
    critical_section::with(|cs| {
        if let Some(bus) = BUS.borrow_ref_mut(cs).as_mut() {
            bus.drain_command();
        }
    });
}

impl Bus {
    fn drain_power(&mut self) {
        let mut bytes = [0u8; 16];
        let count = self.power.read_buffered(&mut bytes).unwrap_or(0);
        let now = now_us();
        for &byte in &bytes[..count] {
            let actions = self.power_side.on_byte(byte, now, &self.mitm);
            if let Some(reply) = actions.reply {
                wait_reply_gap(now);
                let _ = self.power.write(&reply);
            }
            if let Some(byte) = actions.forward {
                let _ = self.command.write(&[byte]);
            }
            if let Some(frame) = actions.frame {
                self.on_power_frame(frame);
                log_frame(Side::Power, frame, now);
            }
        }
        self.power.clear_interrupts(UartInterrupt::RxFifoFull.into());
    }

    fn on_power_frame(&mut self, frame: Frame) {
        let state = PowerState::decode(&frame);
        if state.is_some() && state != self.power_state {
            self.power_state = state;
            CHANGED.signal(());
        }
    }

    fn drain_command(&mut self) {
        let mut bytes = [0u8; 16];
        let count = self.command.read_buffered(&mut bytes).unwrap_or(0);
        let now = now_us();
        for &byte in &bytes[..count] {
            let Some(frame) = self.command_wire.push(byte, now).frame else { continue };
            if header::is_poll(frame.header()) {
                log_frame(Side::Command, frame, now);
            }
            let before = (self.mitm.keys(), self.mitm.control());
            self.mitm.from_command_board(frame);
            if frame.header() == header::STATE && before != (self.mitm.keys(), self.mitm.control()) {
                CHANGED.signal(());
            }
        }
        self.command.clear_interrupts(UartInterrupt::RxFifoFull.into());
    }
}

/// The byte was read in the middle of its stop bit: half a bit, then the gap.
fn wait_reply_gap(read_us: u64) {
    let until = read_us + BIT_US * u64::from(1 + reply_gap_half_bits()) / 2;
    while now_us() < until {}
}
