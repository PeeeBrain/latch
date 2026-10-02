use cargo_packager_updater::{Config, Update, UpdaterBuilder};
use std::time::Duration;

// Same public trust key as the migration host; never load a replacement key
// from user settings or a remote manifest. Native and legacy feeds are separate.
const PUBLIC_KEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IEM5NEE2NkQxQjY5NUFFRkEKUldUNnJwVzIwV1pLeVRMQnVWTmNodW5Rck5WTHk3ektOYWZoZG5UaisyWUUyN3lscmZqcGFCYUsK";
const ENDPOINT: &str =
    "https://github.com/PeeeBrain/latch/releases/latest/download/native-latest.json";

pub fn check() -> Result<Option<Update>, String> {
    if cfg!(debug_assertions) {
        return Err("Updates are available only in installed release packages.".into());
    }
    let version = semver::Version::parse(env!("CARGO_PKG_VERSION"))
        .map_err(|_| "Invalid application version")?;
    let endpoint = url::Url::parse(ENDPOINT).map_err(|_| "Invalid update endpoint")?;
    let updater = UpdaterBuilder::new(
        version,
        Config {
            endpoints: vec![endpoint],
            pubkey: PUBLIC_KEY.into(),
            windows: None,
        },
    )
    .timeout(Duration::from_secs(30))
    .build()
    .map_err(|error| format!("Updates unavailable for this installation: {error}"))?;
    let update = updater
        .check()
        .map_err(|error| format!("Native update check unavailable: {error}"))?;
    if let Some(update) = &update
        && (update.download_url.scheme() != "https"
            || update.download_url.host_str() != Some("github.com"))
    {
        return Err("Update download must use the trusted HTTPS release host".into());
    }
    Ok(update)
}
