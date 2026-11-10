//! Linap bus of the Sauter Marapi towel radiator (`knowledge/linap.md`): pure logic, no I/O, the time comes
//! from the caller in microseconds.
#![cfg_attr(not(test), no_std)]

mod byte;
mod changes;
mod echoes;
mod frame;
mod mitm;
mod power_side;
mod setpoint;
mod state;
mod wire;

pub use byte::{BIT_US, has_odd_parity, with_parity};
pub use changes::Changes;
pub use echoes::Echoes;
pub use frame::{Frame, header};
pub use mitm::{Control, DEFAULT_BOOST_US, Mitm};
pub use power_side::{Actions, PowerSide};
pub use setpoint::{Setpoint, parse_halves};
pub use state::{CommandState, PowerState};
pub use wire::{Heard, Wire};
