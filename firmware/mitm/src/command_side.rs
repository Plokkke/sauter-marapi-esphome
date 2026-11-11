//! The command board half: the board plays the power board on its own clock, like the real one, and the command
//! board answers each poll (read by the RX interrupt in `bus`). Independent of the power board half: the board can
//! be checked with the command board alone, over USB, without mains.
use embassy_time::{Duration, Instant, Timer};
use linap::{Frame, Mode, POLL_DELAY_MS, PowerBoard, SLOT_MS, Setpoint, byte_delay_us};
use log::info;

use crate::bus;

#[embassy_executor::task]
pub async fn play_power_board() {
    info!("playing the power board on J2");
    let mut board = PowerBoard::default();
    let mut next = Instant::now();
    let mut last = None;
    loop {
        // After a stall, resume the rhythm instead of sending the missed slots back to back
        next = (next + Duration::from_millis(SLOT_MS)).max(Instant::now());
        Timer::at(next).await;
        let Some(frame) = board.next_slot() else { continue };
        send_paced(&frame).await;
        if let Some(poll) = board.poll_after(&frame, Instant::now().as_millis()) {
            Timer::after_millis(POLL_DELAY_MS).await;
            bus::write_command(&[poll]);
        }
        report(&mut last);
    }
}

/// One byte at a time, spaced like the real power board's.
async fn send_paced(frame: &Frame) {
    for (index, byte) in frame.0.iter().enumerate() {
        bus::write_command(&[*byte]);
        if index + 1 < frame.0.len() {
            Timer::after_micros(byte_delay_us(index)).await;
        }
    }
}

/// Logs the state when it changes, and the counters of both halves every 10 s.
fn report(last: &mut Option<(Mode, Setpoint)>) {
    let Some(snapshot) = bus::snapshot() else { return };
    let state = Some((snapshot.mode, snapshot.setpoint));
    if state != *last {
        info!("state: {:?}, setpoint {}", snapshot.mode, snapshot.setpoint);
        *last = state;
    }
    let seconds = Instant::now().as_secs();
    if seconds.is_multiple_of(10) && Instant::now().as_millis() % 1000 < SLOT_MS {
        info!(
            "J1: {} bytes, {} polls, {} answered; J2: {} frames heard (ours included), {} answers",
            snapshot.power_bytes, snapshot.polls, snapshot.answered, snapshot.command_frames, snapshot.command_answers
        );
    }
}
