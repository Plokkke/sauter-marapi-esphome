//! The orders, from Home Assistant over MQTT or from the USB console on the bench. They change the board's state
//! like the keys do: the last order wins.
use core::cell::Cell;

use alloc::string::String;
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use log::{info, warn};

use crate::bus;
use crate::home_assistant::{self as ha, Command};

/// Set when the frame log is switched: its reader starts afresh.
static FRAME_LOG_SWITCHED: Mutex<CriticalSectionRawMutex, Cell<bool>> = Mutex::new(Cell::new(false));

/// True once after each switch of the frame log.
pub fn frame_log_switched() -> bool {
    FRAME_LOG_SWITCHED.lock(|switched| switched.replace(false))
}

/// Carries out the order on this command topic. Returns a line for the frame log, if any.
pub fn obey(topic: &str, payload: &str) -> Option<String> {
    match ha::parse_command(topic, payload) {
        Some(Command::Mode(mode)) => {
            bus::with_mitm(|mitm, now| mitm.set_mode(mode, now));
        }
        Some(Command::Setpoint(setpoint)) => {
            bus::with_mitm(|mitm, _| mitm.set_setpoint(setpoint));
        }
        Some(other) => return configure(other),
        None => {
            warn!("ignored {payload:?} on {topic}");
            return None;
        }
    }
    info!("order: {topic} {payload}");
    bus::CHANGED.signal(());
    None
}

/// The state published before the restart, handed back by the broker.
pub fn restore(json: &str) {
    let Some((mode, setpoint)) = ha::saved_state(json) else { return warn!("no saved state in {json:?}") };
    if bus::with_mitm(|mitm, _| mitm.restore(mode, setpoint)) == Some(true) {
        info!("saved state restored: {mode:?}, setpoint {setpoint}");
        bus::CHANGED.signal(());
    }
}

fn configure(command: Command) -> Option<String> {
    match command {
        Command::ReplyGap(half_bits) => bus::set_reply_gap_half_bits(half_bits),
        Command::BoostDuration(boost_us) => bus::with_mitm(|mitm, _| mitm.set_boost_us(boost_us)).unwrap_or(()),
        Command::FrameLog(on) => {
            FRAME_LOG_SWITCHED.lock(|switched| switched.set(true));
            bus::set_frame_log(on);
        }
        Command::Note(note) => return Some(ha::note_line(bus::now_us(), note)),
        Command::Ota(source) => crate::ota::request(source),
        Command::Mode(_) | Command::Setpoint(_) => {}
    }
    None
}
