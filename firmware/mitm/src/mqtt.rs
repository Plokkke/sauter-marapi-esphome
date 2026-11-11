//! MQTT link to Home Assistant: discovery, the state on every change and every 30 s, the commands.
use core::net::Ipv4Addr;
use core::num::NonZero;
use core::str::FromStr;

use embassy_futures::select::{Either4, select4};
use embassy_net::Stack;
use embassy_net::tcp::TcpSocket;
use embassy_time::{Duration, Timer, with_timeout};
use linap::Changes;
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

use crate::bus::{self, CHANGED, FRAMES, Heard};
use crate::commands;
use crate::home_assistant as ha;
use crate::network;
use crate::secrets::{MQTT_PASSWORD, MQTT_USER};

const PORT: u16 = 1883;
const CLIENT_ID: &str = "sauter-marapi";
const KEEP_ALIVE_S: u16 = 60;
const REFRESH: Duration = Duration::from_secs(30);
const RETRY: Duration = Duration::from_secs(5);
const EXPIRY_CHECK: Duration = Duration::from_secs(10);
/// The broker hands the state published before the restart back at once, when it has one.
const SAVED_STATE_WAIT: Duration = Duration::from_secs(3);

type Mqtt<'s, 'b> = Client<'s, 'b, TcpSocket<'s>, AllocBuffer, 2, 4, 1, 0, 0, 0, 0>;

/// What Home Assistant sees between two publications.
struct Remote {
    counts: (u32, u32),
    /// The frame log only shows frames that changed.
    changes: Changes,
    /// The saved state was read, once after start-up.
    restored: bool,
}

#[embassy_executor::task]
pub async fn run(stack: Stack<'static>) {
    let mut remote = Remote { counts: (0, 0), changes: Changes::default(), restored: false };
    loop {
        stack.wait_config_up().await;
        if let Some(config) = stack.config_v4() {
            info!("IP address {}", config.address);
        }
        session(stack, &mut remote).await;
        warn!("MQTT session ended");
        Timer::after(RETRY).await;
    }
}

/// Ends a Boost ordered by Home Assistant once its time is over.
#[embassy_executor::task]
pub async fn expire_boost() {
    loop {
        Timer::after(EXPIRY_CHECK).await;
        if bus::with_mitm(|mitm, now| mitm.expire(now)) == Some(true) {
            info!("ordered Boost over");
            CHANGED.signal(());
        }
    }
}

async fn session(stack: Stack<'static>, remote: &mut Remote) {
    let (mut rx, mut tx) = ([0u8; 1024], [0u8; 1024]);
    let mut socket = TcpSocket::new(stack, &mut rx, &mut tx);
    socket.set_timeout(Some(Duration::from_secs(2 * u64::from(KEEP_ALIVE_S))));
    let Ok(host) = Ipv4Addr::from_str(network::mqtt_host()) else {
        warn!("no MQTT broker (IPv4 address) for this network");
        return;
    };
    if let Err(error) = socket.connect((host, PORT)).await {
        warn!("MQTT broker unreachable: {error:?}");
        return;
    }
    info!("MQTT broker reached");
    let mut buffer = AllocBuffer;
    let mut client: Mqtt = Client::new(&mut buffer);
    if connect(&mut client, socket).await.is_some() && announce(&mut client, remote.restored).await.is_some() {
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
    info!("MQTT session open");
    Some(())
}

/// After start-up, also subscribes to its own state, to read the saved one.
async fn announce(client: &mut Mqtt<'_, '_>, restored: bool) -> Option<()> {
    publish(client, ha::DISCOVERY, ha::DISCOVERY_PAYLOAD, true).await?;
    publish(client, ha::AVAILABILITY, "online", true).await?;
    subscribe(client, ha::COMMANDS).await?;
    if !restored {
        subscribe(client, ha::STATE).await?;
    }
    Some(())
}

async fn subscribe(client: &mut Mqtt<'_, '_>, filter: &'static str) -> Option<()> {
    let filter = TopicFilter::new(MqttString::try_from(filter).ok()?)?;
    client
        .subscribe(filter, &SubscriptionOptions::new())
        .await
        .map_err(|error| warn!("MQTT subscribe: {error:?}"))
        .ok()?;
    Some(())
}

async fn serve(client: &mut Mqtt<'_, '_>, remote: &mut Remote) {
    // Read the saved state before publishing the current one over it
    if !remote.restored {
        let _ = with_timeout(SAVED_STATE_WAIT, read_saved_state(client, remote)).await;
        remote.restored = true;
    }
    CHANGED.signal(());
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
        if served.is_none() || publish_state(client, remote).await.is_none() {
            return;
        }
    }
}

async fn read_saved_state(client: &mut Mqtt<'_, '_>, remote: &mut Remote) -> Option<()> {
    while !remote.restored {
        let header = client.poll_header().await.ok()?;
        receive(client, header, remote).await?;
    }
    Some(())
}

async fn receive(client: &mut Mqtt<'_, '_>, header: FixedHeader, remote: &mut Remote) -> Option<()> {
    let line = match client.poll_body(header).await {
        Ok(Event::Publish(publish)) => match &publish.topic {
            TopicReference::Name(name) => {
                handle(name.as_ref().as_str(), core::str::from_utf8(&publish.message).unwrap_or(""), remote)
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

/// An order, or our own state: the saved one first, then our publications, ignored.
fn handle(topic: &str, payload: &str, remote: &mut Remote) -> Option<alloc::string::String> {
    if topic != ha::STATE {
        return commands::obey(topic, payload);
    }
    if !remote.restored {
        commands::restore(payload);
        remote.restored = true;
    }
    None
}

async fn log_frame(client: &mut Mqtt<'_, '_>, remote: &mut Remote, heard: Heard) -> Option<()> {
    if commands::frame_log_switched() {
        remote.changes = Changes::default();
    }
    if !remote.changes.is_new(heard.side as u8, heard.frame) {
        return Some(());
    }
    publish(client, ha::FRAMES, &ha::frame_line(&heard), false).await
}

async fn publish_state(client: &mut Mqtt<'_, '_>, remote: &mut Remote) -> Option<()> {
    let snapshot = bus::snapshot()?;
    let (polls, answered) = (snapshot.polls, snapshot.answered);
    let new_polls = polls.wrapping_sub(remote.counts.0);
    let percent = (new_polls > 0).then(|| answered.wrapping_sub(remote.counts.1) * 100 / new_polls);
    remote.counts = (polls, answered);
    let json = ha::state_json(&snapshot, bus::reply_gap_half_bits(), percent, network::rssi());
    publish(client, ha::STATE, &json, true).await?;
    info!("state published");
    Some(())
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
