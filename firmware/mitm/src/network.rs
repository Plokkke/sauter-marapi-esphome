//! Wi-Fi station and IP stack (DHCP).
use embassy_executor::Spawner;
use embassy_net::{Runner, Stack, StackResources};
use embassy_time::{Duration, Timer};
use esp_hal::peripherals::WIFI;
use esp_hal::rng::Rng;
use esp_radio::wifi::sta::StationConfig;
use esp_radio::wifi::{AuthenticationMethodConfig, Config, ControllerConfig, Interface, WifiController};
use log::{info, warn};
use static_cell::StaticCell;

use crate::secrets::{WIFI_PASSWORD, WIFI_SSID};

const RETRY: Duration = Duration::from_secs(5);
/// In 0.25 dBm: 44 = 11 dBm. The esp-radio default (5 dBm) associates but loses DHCP; above ~16 dBm some boards
/// fail to authenticate.
const MAX_TX_POWER: i8 = 44;

pub fn start(spawner: Spawner, wifi: WIFI<'static>) -> Stack<'static> {
    let station = Config::Station(
        StationConfig::default().with_ssid(WIFI_SSID.try_into().expect("SSID too long")).with_authentication(
            AuthenticationMethodConfig::Wpa2Personal(WIFI_PASSWORD.try_into().expect("Wi-Fi password too long")),
        ),
    );
    let controller =
        WifiController::new(wifi, ControllerConfig::default().with_initial_config(station)).expect("Wi-Fi controller");
    static RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();
    let rng = Rng::new();
    let seed = u64::from(rng.random()) << 32 | u64::from(rng.random());
    let (stack, runner) = embassy_net::new(
        Interface::station(),
        embassy_net::Config::dhcpv4(Default::default()),
        RESOURCES.init(StackResources::new()),
        seed,
    );
    spawner.spawn(connection(controller).expect("connection task"));
    spawner.spawn(net(runner).expect("net task"));
    stack
}

#[embassy_executor::task]
async fn connection(mut controller: WifiController<'static>) {
    if let Err(error) = controller.set_max_tx_power(MAX_TX_POWER) {
        warn!("Wi-Fi TX power: {error:?}");
    }
    loop {
        match controller.connect_async().await {
            Ok(_) => {
                info!("Wi-Fi connected");
                let _ = controller.wait_for_disconnect_async().await;
                warn!("Wi-Fi disconnected");
            }
            Err(error) => warn!("Wi-Fi connection failed: {error:?}"),
        }
        Timer::after(RETRY).await;
    }
}

#[embassy_executor::task]
async fn net(mut runner: Runner<'static, Interface>) {
    runner.run().await
}
