/// Wi-Fi and MQTT credentials, read from `../secrets.env` (not versioned, see `../secrets.example.env`).
const SECRETS: &str = "../secrets.env";
const SECRET_NAMES: [&str; 5] = ["WIFI_SSID", "WIFI_PASSWORD", "MQTT_HOST", "MQTT_USER", "MQTT_PASSWORD"];

fn main() {
    linker_be_nice();
    export_secrets();
    // make sure linkall.x is the last linker script (otherwise might cause problems with flip-link)
    println!("cargo:rustc-link-arg=-Tlinkall.x");
}

/// Called back by the linker on errors (esp-generate template): turns the usual ones into hints.
fn linker_be_nice() {
    let args: Vec<String> = std::env::args().collect();
    if let [_, kind, what, ..] = args.as_slice() {
        if kind == "undefined-symbol" {
            if let Some(hint) = linker_hint(what) {
                eprintln!("\n💡 {hint}\n");
            }
            std::process::exit(0);
        }
        std::process::exit(1);
    }
    let script = std::env::current_exe().expect("build script path");
    println!("cargo:rustc-link-arg=--error-handling-script={}", script.display());
}

fn linker_hint(symbol: &str) -> Option<&'static str> {
    match symbol {
        "_stack_start" => Some("Is the linker script `linkall.x` missing?"),
        s if s.starts_with("esp_rtos_") => Some("`esp-radio` has no scheduler: initialize `esp-rtos`"),
        "free"
        | "malloc"
        | "calloc"
        | "get_free_internal_heap_size"
        | "malloc_internal"
        | "realloc_internal"
        | "calloc_internal"
        | "free_internal" => Some("Did you forget the `esp-alloc` dependency?"),
        _ => None,
    }
}

fn export_secrets() {
    println!("cargo:rerun-if-changed={SECRETS}");
    let text = std::fs::read_to_string(SECRETS)
        .unwrap_or_else(|_| panic!("{SECRETS} is missing: copy secrets.example.env and fill it in"));
    for name in SECRET_NAMES {
        let value = text
            .lines()
            .filter_map(|line| line.split_once('='))
            .find(|(key, _)| key.trim() == name)
            .map(|(_, value)| value.trim().trim_matches('"'))
            .unwrap_or_else(|| panic!("{name} is missing from {SECRETS}"));
        println!("cargo:rustc-env={name}={value}");
    }
}
