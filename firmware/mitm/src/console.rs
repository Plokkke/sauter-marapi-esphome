//! Orders typed on the USB port, without Wi-Fi nor broker: one per line, named like the MQTT commands
//! (`mode on`, `setpoint 19.5`, `mode boost`, `frame-log ON`, `reply-gap 1.5`). Bench only: on the
//! radiator the board is at mains potential and USB is forbidden.
use embassy_time::Timer;
use esp_hal::Blocking;
use esp_hal::peripherals::USB_DEVICE;
use esp_hal::usb::usb_serial_jtag::{UsbSerialJtag, UsbSerialJtagRx};
use log::{info, warn};

use crate::commands;
use crate::home_assistant as ha;

const LINE_MAX: usize = 96;
const POLL_MS: u64 = 20;

pub fn start(spawner: embassy_executor::Spawner, usb: USB_DEVICE<'static>) {
    let (rx, _tx) = UsbSerialJtag::new(usb).split();
    spawner.spawn(read_orders(rx).expect("console task"));
}

#[embassy_executor::task]
async fn read_orders(mut rx: UsbSerialJtagRx<'static, Blocking>) {
    info!("console: one order per line, as `<command> <value>`");
    let mut line = [0u8; LINE_MAX];
    let mut len = 0;
    loop {
        Timer::after_millis(POLL_MS).await;
        while let Ok(byte) = rx.read_byte() {
            match byte {
                b'\r' | b'\n' => {
                    run(&line[..len]);
                    len = 0;
                }
                _ if len < LINE_MAX => {
                    line[len] = byte;
                    len += 1;
                }
                _ => {}
            }
        }
    }
}

fn run(line: &[u8]) {
    let Ok(text) = core::str::from_utf8(line) else { return warn!("console: not UTF-8") };
    let text = text.trim();
    if text.is_empty() {
        return;
    }
    let (name, value) = text.split_once(' ').unwrap_or((text, ""));
    info!("console: {text}");
    if let Some(note) = commands::obey(&ha::command_topic(name), value.trim()) {
        info!("{note}");
    }
}
