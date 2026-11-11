//! Compiled in from `../secrets.env` by `build.rs`.
pub const WIFI_SSID: &str = env!("WIFI_SSID");
pub const WIFI_PASSWORD: &str = env!("WIFI_PASSWORD");
pub const MQTT_HOST: &str = env!("MQTT_HOST");
pub const MQTT_USER: &str = env!("MQTT_USER");
pub const MQTT_PASSWORD: &str = env!("MQTT_PASSWORD");
/// First line of an OTA upload (`firmware/tools/ota.py`).
pub const OTA_PASSWORD: &str = env!("OTA_PASSWORD");
/// Optional, empty when absent: a phone hotspot next to the radiator, and the broker reachable through it.
pub const WIFI_FALLBACK_SSID: &str = env!("WIFI_FALLBACK_SSID");
pub const WIFI_FALLBACK_PASSWORD: &str = env!("WIFI_FALLBACK_PASSWORD");
pub const MQTT_FALLBACK_HOST: &str = env!("MQTT_FALLBACK_HOST");
