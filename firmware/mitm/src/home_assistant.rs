//! Home Assistant side: one MQTT device discovery message, the state it reads, the commands it sends.
use alloc::format;
use alloc::string::String;

use linap::{CommandState, Control, PowerState, Setpoint, parse_halves};

use crate::bus::{Heard, Side, Snapshot};

macro_rules! topic {
    ($name:literal) => {
        concat!("sauter-marapi/", $name)
    };
}

/// One discovery component, reading its value from the shared state topic.
macro_rules! component {
    ($id:literal, $($fields:expr),+) => {
        concat!(
            "\"", $id, "\":{", $($fields),+, ",\"val_tpl\":\"{{ value_json.", $id, " }}\",\"uniq_id\":\"sauter_marapi_", $id, "\"}"
        )
    };
}

pub const AVAILABILITY: &str = topic!("availability");
/// Retained, so that Home Assistant has it as soon as it subscribes (after a restart, or a new discovery).
pub const STATE: &str = topic!("state");
pub const COMMANDS: &str = topic!("+/set");
/// Frames heard on the bus, when the frame log is on: one line each, for the analysis of the unknown frames.
pub const FRAMES: &str = topic!("frames");
pub const DISCOVERY: &str = "homeassistant/device/sauter_marapi/config";
const CONTROL: &str = topic!("control/set");
const SETPOINT: &str = topic!("setpoint/set");
const REPLY_GAP: &str = topic!("reply-gap/set");
const BOOST_DURATION: &str = topic!("boost-duration/set");
const FRAME_LOG: &str = topic!("frame-log/set");
const NOTE: &str = topic!("note/set");
const MINUTE_US: u64 = 60_000_000;
const MAX_BOOST_MINUTES: u8 = 120;

/// The configuration entities are retained by the broker: it hands them back after a restart of the board.
pub const DISCOVERY_PAYLOAD: &str = concat!(
    "{\"dev\":{\"ids\":[\"sauter_marapi\"],\"name\":\"Sauter Marapi\",\"mf\":\"Sauter\",\"mdl\":\"Marapi tactile\"},",
    "\"o\":{\"name\":\"sauter-marapi-mitm\"},",
    "\"avty_t\":\"",
    topic!("availability"),
    "\",\"stat_t\":\"",
    topic!("state"),
    "\",\"cmps\":{",
    component!(
        "keys_mode",
        "\"p\":\"sensor\",\"name\":\"Keys mode\",\"dev_cla\":\"enum\",\"ops\":[\"off\",\"boost\",\"on\"]"
    ),
    ",",
    component!(
        "keys_setpoint",
        "\"p\":\"sensor\",\"name\":\"Keys setpoint\",\"dev_cla\":\"temperature\",\"unit_of_meas\":\"°C\""
    ),
    ",",
    component!(
        "power",
        "\"p\":\"sensor\",\"name\":\"Power board\",\"dev_cla\":\"enum\",\"ops\":[\"idle\",\"heating\",\"boost\",\"boost_ending\",\"off\",\"unknown\"]"
    ),
    ",",
    component!("heating", "\"p\":\"binary_sensor\",\"name\":\"Heating\",\"dev_cla\":\"heat\""),
    ",",
    component!(
        "control",
        "\"p\":\"select\",\"name\":\"Control\",\"ops\":[\"relay\",\"setpoint\",\"boost\",\"off\"],\"cmd_t\":\"",
        topic!("control/set"),
        "\""
    ),
    ",",
    component!(
        "setpoint",
        "\"p\":\"number\",\"name\":\"Setpoint\",\"dev_cla\":\"temperature\",\"unit_of_meas\":\"°C\",\"min\":5,\"max\":30,\"step\":0.5,\"mode\":\"box\",\"cmd_t\":\"",
        topic!("setpoint/set"),
        "\""
    ),
    ",",
    component!(
        "boost_duration",
        "\"p\":\"number\",\"name\":\"Boost duration\",\"ent_cat\":\"config\",\"ret\":true,\"dev_cla\":\"duration\",\"unit_of_meas\":\"min\",\"min\":1,\"max\":120,\"step\":1,\"mode\":\"box\",\"cmd_t\":\"",
        topic!("boost-duration/set"),
        "\""
    ),
    ",",
    component!(
        "reply_gap",
        "\"p\":\"number\",\"name\":\"Reply gap\",\"ent_cat\":\"config\",\"ret\":true,\"unit_of_meas\":\"bits\",\"min\":0,\"max\":30,\"step\":0.5,\"mode\":\"box\",\"cmd_t\":\"",
        topic!("reply-gap/set"),
        "\""
    ),
    ",",
    component!(
        "frame_log",
        "\"p\":\"switch\",\"name\":\"Frame log\",\"ent_cat\":\"config\",\"ret\":true,\"cmd_t\":\"",
        topic!("frame-log/set"),
        "\""
    ),
    ",",
    component!(
        "note",
        "\"p\":\"text\",\"name\":\"Note\",\"ent_cat\":\"diagnostic\",\"max\":100,\"cmd_t\":\"",
        topic!("note/set"),
        "\""
    ),
    ",",
    component!(
        "answered",
        "\"p\":\"sensor\",\"name\":\"Polls answered\",\"ent_cat\":\"diagnostic\",\"unit_of_meas\":\"%\",\"stat_cla\":\"measurement\""
    ),
    "}}"
);

pub enum Command<'p> {
    Control(Control),
    Setpoint(Setpoint),
    ReplyGap(u32),
    /// How long an imposed Boost lasts, in microseconds.
    BoostDuration(u64),
    FrameLog(bool),
    /// A marker written in the frame log, between the frames it follows and the ones that follow it.
    Note(&'p str),
}

/// `setpoint` is the imposed setpoint, used when Home Assistant selects "setpoint".
pub fn parse_command<'p>(topic: &str, payload: &'p str, setpoint: Setpoint) -> Option<Command<'p>> {
    match topic {
        CONTROL => Some(Command::Control(match payload {
            "relay" => Control::Relay,
            "setpoint" => Control::Impose(CommandState::On(setpoint)),
            "boost" => Control::Impose(CommandState::Boost),
            "off" => Control::Impose(CommandState::Off),
            _ => return None,
        })),
        SETPOINT => Setpoint::parse(payload).map(Command::Setpoint),
        REPLY_GAP => parse_halves(payload).map(|halves| Command::ReplyGap(halves.into())),
        BOOST_DURATION => parse_halves(payload)
            .map(|halves| halves / 2)
            .filter(|minutes| (1..=MAX_BOOST_MINUTES).contains(minutes))
            .map(|minutes| Command::BoostDuration(u64::from(minutes) * MINUTE_US)),
        FRAME_LOG => match payload {
            "ON" => Some(Command::FrameLog(true)),
            "OFF" => Some(Command::FrameLog(false)),
            _ => None,
        },
        NOTE => Some(Command::Note(payload)),
        _ => None,
    }
}

/// The JSON of the state topic; unknown values are null, which Home Assistant shows as unknown.
/// The note is always empty: Home Assistant clears its field once the note is in the log.
pub fn state_json(bus: &Snapshot, setpoint: Setpoint, reply_gap_half_bits: u32, answered: Option<u32>) -> String {
    let keys_setpoint = match bus.keys {
        Some(CommandState::On(setpoint)) => format!("{setpoint}"),
        _ => String::from("null"),
    };
    format!(
        "{{\"keys_mode\":{},\"keys_setpoint\":{keys_setpoint},\"power\":{},\"heating\":{},\"control\":\"{}\",\
         \"setpoint\":{setpoint},\"boost_duration\":{},\"reply_gap\":{}.{},\"answered\":{},\"frame_log\":\"{}\",\"note\":\"\"}}",
        quoted(bus.keys.map(keys_name)),
        quoted(bus.power.map(PowerState::name)),
        quoted(bus.power.map(|power| if power.is_heating() { "ON" } else { "OFF" })),
        control_name(bus.control),
        bus.boost_us / MINUTE_US,
        reply_gap_half_bits / 2,
        if reply_gap_half_bits % 2 == 1 { 5 } else { 0 },
        answered.map_or(String::from("null"), |percent| format!("{percent}")),
        if bus.frame_log { "ON" } else { "OFF" },
    )
}

/// "12.345 P C7 80 E0 38 58": seconds since boot, half of DATA (P power board, C command board), frame.
pub fn frame_line(heard: &Heard) -> String {
    let side = match heard.side {
        Side::Power => 'P',
        Side::Command => 'C',
    };
    format!("{} {side} {}", seconds(heard.at_us), heard.frame)
}

pub fn note_line(at_us: u64, note: &str) -> String {
    format!("{} note {note}", seconds(at_us))
}

fn seconds(at_us: u64) -> String {
    format!("{}.{:03}", at_us / 1_000_000, at_us / 1_000 % 1_000)
}

fn quoted(text: Option<&str>) -> String {
    text.map_or(String::from("null"), |text| format!("\"{text}\""))
}

fn keys_name(state: CommandState) -> &'static str {
    match state {
        CommandState::Off => "off",
        CommandState::Boost => "boost",
        CommandState::On(_) => "on",
    }
}

fn control_name(control: Control) -> &'static str {
    match control {
        Control::Relay => "relay",
        Control::Impose(CommandState::On(_)) => "setpoint",
        Control::Impose(CommandState::Boost) => "boost",
        Control::Impose(CommandState::Off) => "off",
    }
}
