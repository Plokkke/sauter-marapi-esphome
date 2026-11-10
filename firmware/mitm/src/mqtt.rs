//! MQTT link to Home Assistant: discovery, the state on every change and every 30 s, the commands.
use core::net::Ipv4Addr;
use core::num::NonZero;
use core::str::FromStr;

use alloc::string::String;
use embassy_futures::select::{Either4, select4};
use embassy_net::Stack;
use embassy_net::tcp::TcpSocket;
use embassy_time::{Duration, Timer};
use linap::{Changes, CommandState, Control, Setpoint};
use log::{info, warn};
use rust_mqtt::Bytes;
use rust_mqtt::buffer::AllocBuffer;
use rust_mqtt::client::Client;
use rust_mqtt::client::event::Event;
use rust_mqtt::client::options::{
    ConnectOptions, PublicationOptions, SubscriptionOptions, TopicReference, WillOptions,
};
use rust_mqtt::config::KeepAlive;
use rust_mqtt::header::FixedHeader;
use rust_mqtt::types::{MqttBinary, MqttString, TopicFilter, TopicName};

use crate::bus::{self, CHANGED, FRAMES, Heard, Snapshot};
use crate::home_assistant::{self as ha, Command};
use crate::secrets::{MQTT_HOST, MQTT_PASSWORD, MQTT_USER};

const PORT: u16 = 1883;
const CLIENT_ID: &str = "sauter-marapi";
const KEEP_ALIVE_S: u16 = 60;
const REFRESH: Duration = Duration::from_secs(30);
const RETRY: Duration = Duration::from_secs(5);
const EXPIRY_CHECK: Duration = Duration::from_secs(10);
/// Setpoint offered before the command board has told its own.
const DEFAULT_SETPOINT_HALVES: u8 = 38;

type Mqtt<'s, 'b> = Client<'s, 'b, TcpSocket<'s>, AllocBuffer, 1, 4, 1, 0, 0, 0, 0>;

/// What Home Assistant sees between two publications.
struct Remote {
    setpoint: Setpoint,
    counts: (u32, u32),
    /// The frame log only shows frames that changed.
    changes: Changes,
}

#[embassy_executor::task]
pub async fn run(stack: Stack<'static>) {
    let setpoint = Setpoint::from_halves(DEFAULT_SETPOINT_HALVES).expect("default setpoint in range");
    let mut remote = Remote { setpoint, counts: (0, 0), changes: Changes::default() };
    loop {
        stack.wait_config_up().await;
        session(stack, &mut remote).await;
        warn!("MQTT session ended");
        Timer::after(RETRY).await;
    }
}

/// Back to relaying when an imposed Boost is over, or when Home Assistant has been out of reach for too long.
#[embassy_executor::task]
pub async fn expire_control() {
    loop {
        Timer::after(EXPIRY_CHECK).await;
        if bus::with_mitm(|mitm, now| mitm.expire(now)) == Some(true) {
            info!("imposed state over: relaying the keys again");
            CHANGED.signal(());
        }
    }
}

async fn session(stack: Stack<'static>, remote: &mut Remote) {
    let (mut rx, mut tx) = ([0u8; 1024], [0u8; 1024]);
    let mut socket = TcpSocket::new(stack, &mut rx, &mut tx);
    socket.set_timeout(Some(Duration::from_secs(2 * u64::from(KEEP_ALIVE_S))));
    let host = Ipv4Addr::from_str(MQTT_HOST).expect("MQTT_HOST is an IPv4 address");
    if let Err(error) = socket.connect((host, PORT)).await {
        warn!("MQTT broker unreachable: {error:?}");
        return;
    }
    let mut buffer = AllocBuffer;
    let mut client: Mqtt = Client::new(&mut buffer);
    if connect(&mut client, socket).await.is_some() && announce(&mut client).await.is_some() {
        info!("MQTT connected");
        serve(&mut client, remote).await;
    }
}

async fn connect<'s>(client: &mut Mqtt<'s, '_>, socket: TcpSocket<'s>) -> Option<()> {
    let will = WillOptions::new(topic(ha::AVAILABILITY), MqttBinary::try_from("offline").ok()?).retain();
    let options = ConnectOptions::new().clean_start().keep_alive(KeepAlive::Seconds(NonZero::new(KEEP_ALIVE_S)?));
    // Empty user: anonymous connection, for a broker that allows it
    let options = if MQTT_USER.is_empty() {
        options
    } else {
        options.user_name(MqttString::try_from(MQTT_USER).ok()?).password(MqttBinary::try_from(MQTT_PASSWORD).ok()?)
    };
    let options = options.will(will);
    let client_id = MqttString::try_from(CLIENT_ID).ok();
    client.connect(socket, &options, client_id).await.map_err(|error| warn!("MQTT connect: {error:?}")).ok()?;
    Some(())
}

async fn announce(client: &mut Mqtt<'_, '_>) -> Option<()> {
    publish(client, ha::DISCOVERY, ha::DISCOVERY_PAYLOAD, true).await?;
    publish(client, ha::AVAILABILITY, "online", true).await?;
    let filter = TopicFilter::new(MqttString::try_from(ha::COMMANDS).ok()?)?;
    client
        .subscribe(filter, &SubscriptionOptions::new())
        .await
        .map_err(|error| warn!("MQTT subscribe: {error:?}"))
        .ok()?;
    Some(())
}

async fn serve(client: &mut Mqtt<'_, '_>, remote: &mut Remote) {
    loop {
        let served = match select4(client.poll_header(), CHANGED.wait(), FRAMES.receive(), Timer::after(REFRESH)).await
        {
            Either4::First(Ok(header)) => receive(client, header, remote).await,
            Either4::First(Err(error)) => return warn!("MQTT receive: {error:?}"),
            Either4::Third(heard) => {
                if log_frame(client, remote, heard).await.is_none() {
                    return;
                }
                continue;
            }
            Either4::Second(()) | Either4::Fourth(()) => Some(()),
        };
        bus::with_mitm(|mitm, now| mitm.home_assistant_seen(now));
        if served.is_none() || publish_state(client, remote).await.is_none() {
            return;
        }
    }
}

async fn receive(client: &mut Mqtt<'_, '_>, header: FixedHeader, remote: &mut Remote) -> Option<()> {
    let line = match client.poll_body(header).await {
        Ok(Event::Publish(publish)) => match &publish.topic {
            TopicReference::Name(name) => {
                obey(name.as_ref().as_str(), core::str::from_utf8(&publish.message).unwrap_or(""), remote)
            }
            _ => None,
        },
        Ok(_) => None,
        Err(error) => {
            warn!("MQTT receive: {error:?}");
            return None;
        }
    };
    match line {
        Some(line) => publish(client, ha::FRAMES, &line, false).await,
        None => Some(()),
    }
}

async fn log_frame(client: &mut Mqtt<'_, '_>, remote: &mut Remote, heard: Heard) -> Option<()> {
    if !remote.changes.is_new(heard.side as u8, heard.frame) {
        return Some(());
    }
    publish(client, ha::FRAMES, &ha::frame_line(&heard), false).await
}

/// The last order wins: an order from Home Assistant replaces the keys until they change again. Returns a line
/// for the frame log, if any.
fn obey(topic: &str, payload: &str, remote: &mut Remote) -> Option<String> {
    let control = match ha::parse_command(topic, payload, remote.setpoint) {
        Some(Command::Control(control)) => control,
        Some(Command::Setpoint(setpoint)) => {
            remote.setpoint = setpoint;
            Control::Impose(CommandState::On(setpoint))
        }
        Some(other) => return configure(other, remote),
        None => {
            warn!("ignored {payload:?} on {topic}");
            return None;
        }
    };
    info!("Home Assistant: {control:?}");
    bus::with_mitm(|mitm, now| mitm.set_control(control, now));
    None
}

fn configure(command: Command, remote: &mut Remote) -> Option<String> {
    match command {
        Command::ReplyGap(half_bits) => bus::set_reply_gap_half_bits(half_bits),
        Command::BoostDuration(boost_us) => bus::with_mitm(|mitm, _| mitm.set_boost_us(boost_us)).unwrap_or(()),
        Command::FrameLog(on) => {
            remote.changes = Changes::default();
            bus::set_frame_log(on);
        }
        Command::Note(note) => return Some(ha::note_line(bus::now_us(), note)),
        Command::Control(_) | Command::Setpoint(_) => {}
    }
    None
}

async fn publish_state(client: &mut Mqtt<'_, '_>, remote: &mut Remote) -> Option<()> {
    let snapshot = bus::snapshot()?;
    remote.setpoint = effective_setpoint(&snapshot).unwrap_or(remote.setpoint);
    let (polls, answered) = (snapshot.polls, snapshot.answered);
    let new_polls = polls.wrapping_sub(remote.counts.0);
    let percent = (new_polls > 0).then(|| answered.wrapping_sub(remote.counts.1) * 100 / new_polls);
    remote.counts = (polls, answered);
    let json = ha::state_json(&snapshot, remote.setpoint, bus::reply_gap_half_bits(), percent);
    publish(client, ha::STATE, &json, false).await
}

/// The setpoint the power board receives, when it receives one.
fn effective_setpoint(snapshot: &Snapshot) -> Option<Setpoint> {
    match (snapshot.control, snapshot.keys) {
        (Control::Impose(CommandState::On(setpoint)), _) | (Control::Relay, Some(CommandState::On(setpoint))) => {
            Some(setpoint)
        }
        _ => None,
    }
}

async fn publish(client: &mut Mqtt<'_, '_>, name: &'static str, payload: &str, retain: bool) -> Option<()> {
    let options = PublicationOptions::new(TopicReference::Name(topic(name)));
    let options = if retain { options.retain() } else { options };
    client
        .publish(&options, Bytes::from(payload.as_bytes()))
        .await
        .map_err(|error| warn!("MQTT publish: {error:?}"))
        .ok()?;
    Some(())
}

fn topic(name: &'static str) -> TopicName<'static> {
    TopicName::new(MqttString::try_from(name).expect("topic length")).expect("valid topic name")
}
