//! Wi-Fi station and IP stack (DHCP), on the home network or, failing that, on a fallback one (a phone hotspot
//! next to the radiator, for the tests).
use core::sync::atomic::{AtomicI32, AtomicUsize, Ordering};

use embassy_executor::Spawner;
use embassy_futures::select::{Either, select};
use embassy_net::{Runner, Stack, StackResources};
use embassy_time::{Duration, Timer, with_timeout};
use esp_hal::peripherals::WIFI;
use esp_hal::rng::Rng;
use esp_radio::wifi::sta::StationConfig;
use esp_radio::wifi::{AuthenticationMethodConfig, Config, ControllerConfig, Interface, PowerSaveMode, WifiController};
use log::{info, warn};
use static_cell::StaticCell;

use crate::secrets::{
    MQTT_FALLBACK_HOST, MQTT_HOST, WIFI_FALLBACK_PASSWORD, WIFI_FALLBACK_SSID, WIFI_PASSWORD, WIFI_SSID,
};

const RETRY: Duration = Duration::from_secs(5);
/// Associated but no address after this long: try the other network (a weak link loses the DHCP exchange).
const DHCP_TIMEOUT: Duration = Duration::from_secs(20);
const RSSI_REFRESH: Duration = Duration::from_secs(30);
/// In 0.25 dBm: 64 = 16 dBm, the board being powered on its own (the ribbon's ≈ 160 mA did not hold the C3). The
/// esp-radio default (5 dBm) loses DHCP, 8.5 dBm does not associate; above ~16 dBm some boards fail to authenticate.
const MAX_TX_POWER: i8 = 64;

struct Network {
    ssid: &'static str,
    password: &'static str,
    mqtt_host: &'static str,
}

const NETWORKS: [Network; 2] = [
    Network { ssid: WIFI_SSID, password: WIFI_PASSWORD, mqtt_host: MQTT_HOST },
    Network { ssid: WIFI_FALLBACK_SSID, password: WIFI_FALLBACK_PASSWORD, mqtt_host: MQTT_FALLBACK_HOST },
];

static CURRENT: AtomicUsize = AtomicUsize::new(0);
/// Signal of the access point in dBm, 0 while unknown.
static RSSI: AtomicI32 = AtomicI32::new(0);

/// Broker of the network the board is on.
pub fn mqtt_host() -> &'static str {
    NETWORKS[CURRENT.load(Ordering::Relaxed)].mqtt_host
}

pub fn rssi() -> Option<i32> {
    Some(RSSI.load(Ordering::Relaxed)).filter(|&rssi| rssi != 0)
}

pub fn start(spawner: Spawner, wifi: WIFI<'static>) -> Stack<'static> {
    let controller = WifiController::new(wifi, ControllerConfig::default().with_initial_config(station(&NETWORKS[0])))
        .expect("Wi-Fi controller");
    static RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();
    let rng = Rng::new();
    let seed = u64::from(rng.random()) << 32 | u64::from(rng.random());
    let (stack, runner) = embassy_net::new(
        Interface::station(),
        embassy_net::Config::dhcpv4(Default::default()),
        RESOURCES.init(StackResources::new()),
        seed,
    );
    spawner.spawn(connection(controller, stack).expect("connection task"));
    spawner.spawn(net(runner).expect("net task"));
    stack
}

fn station(network: &Network) -> Config {
    Config::Station(
        StationConfig::default().with_ssid(network.ssid.try_into().expect("SSID too long")).with_authentication(
            AuthenticationMethodConfig::Wpa2Personal(network.password.try_into().expect("Wi-Fi password too long")),
        ),
    )
}

/// Networks to try in turn: the fallback one only when it is configured.
fn network_count() -> usize {
    if WIFI_FALLBACK_SSID.is_empty() { 1 } else { NETWORKS.len() }
}

#[embassy_executor::task]
async fn connection(mut controller: WifiController<'static>, stack: Stack<'static>) {
    if let Err(error) = controller.set_max_tx_power(MAX_TX_POWER) {
        warn!("Wi-Fi TX power: {error:?}");
    }
    // Radio always awake: the board has its own supply, and the updates and commands come in without delay
    if let Err(error) = controller.set_power_saving(PowerSaveMode::None) {
        warn!("Wi-Fi power saving: {error:?}");
    }
    let mut index = 0;
    loop {
        if join(&mut controller, stack, index).await {
            stay(&controller).await;
            warn!("Wi-Fi network {} lost", index + 1);
        } else {
            warn!("Wi-Fi network {} unreachable", index + 1);
            let _ = controller.disconnect_async().await;
            index = (index + 1) % network_count();
        }
        RSSI.store(0, Ordering::Relaxed);
        Timer::after(RETRY).await;
    }
}

/// Associates with network `index` and waits for its address; true when the board is on the network.
async fn join(controller: &mut WifiController<'static>, stack: Stack<'static>, index: usize) -> bool {
    if let Err(error) = controller.set_config(&station(&NETWORKS[index])) {
        warn!("Wi-Fi configuration: {error:?}");
        return false;
    }
    info!("Wi-Fi: trying network {}", index + 1);
    if let Err(error) = controller.connect_async().await {
        warn!("Wi-Fi connection failed: {error:?}");
        return false;
    }
    let up = with_timeout(DHCP_TIMEOUT, stack.wait_config_up()).await.is_ok();
    if up {
        CURRENT.store(index, Ordering::Relaxed);
        refresh_rssi(controller);
        info!("Wi-Fi network {} up, RSSI {:?} dBm", index + 1, rssi());
    }
    up
}

/// Until the network is lost, refreshing the signal strength.
async fn stay(controller: &WifiController<'static>) {
    while let Either::Second(()) = select(controller.wait_for_disconnect_async(), Timer::after(RSSI_REFRESH)).await {
        refresh_rssi(controller);
    }
}

fn refresh_rssi(controller: &WifiController<'static>) {
    if let Ok(rssi) = controller.rssi() {
        RSSI.store(rssi, Ordering::Relaxed);
        info!("Wi-Fi signal {rssi} dBm");
    }
}

#[embassy_executor::task]
async fn net(mut runner: Runner<'static, Interface>) {
    runner.run().await
}
