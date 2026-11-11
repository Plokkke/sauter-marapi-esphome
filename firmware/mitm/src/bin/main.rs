//! ESP32-C3 in the middle of the Linap bus (`knowledge/architecture.md`): owns the radiator's state, which the keys
//! and Home Assistant (over MQTT) change, and gives it to the power board.
#![no_std]
#![no_main]
#![deny(clippy::mem_forget, reason = "mem::forget is generally not safe with esp_hal types")]

use embassy_executor::Spawner;
use esp_backtrace as _;
use esp_hal::clock::CpuClock;
use esp_hal::gpio::Flex;
use esp_hal::timer::timg::TimerGroup;
use esp_storage::FlashStorage;
use log::info;
use mitm::bus::{self, Half};
use mitm::{console, mqtt, network, ota};

extern crate alloc;

esp_bootloader_esp_idf::esp_app_desc!();

#[allow(clippy::large_stack_frames, reason = "buffers are allocated in main")]
#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    esp_println::logger::init_logger_from_env();
    let peripherals = esp_hal::init(esp_hal::Config::default().with_cpu_clock(CpuClock::max()));
    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 66320);
    esp_alloc::heap_allocator!(size: 36 * 1024);
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0, peripherals.FROM_CPU_INTR0);

    // Pins: see the MITM schematic. TX drives the NPN that pulls DATA low, hence inverted in bus.rs.
    bus::start(
        Half { uart: peripherals.UART0, rx: Flex::new(peripherals.GPIO4), tx: Flex::new(peripherals.GPIO5) },
        Half { uart: peripherals.UART1, rx: Flex::new(peripherals.GPIO6), tx: Flex::new(peripherals.GPIO7) },
    );
    info!("Linap bus served");
    console::start(spawner, peripherals.USB_DEVICE);

    let mut flash = FlashStorage::new(peripherals.FLASH);
    ota::confirm_running_image(&mut flash);

    let stack = network::start(spawner, peripherals.WIFI);
    spawner.spawn(ota::serve(stack, flash).expect("OTA task"));
    spawner.spawn(mqtt::run(stack).expect("mqtt task"));
    spawner.spawn(mqtt::expire_boost().expect("expiry task"));
    spawner.spawn(mitm::command_side::play_power_board().expect("command side task"));
    loop {
        embassy_time::Timer::after_secs(3600).await;
    }
}
