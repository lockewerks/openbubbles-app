use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use rand::Rng;
use rustpush::RelayConfig;
use uuid::Uuid;

pub const DEFAULT_RELAY_HOST: &str = "https://hw.openbubbles.app";

fn generate_udid() -> String {
    let bytes: [u8; 32] = rand::thread_rng().gen();
    bytes.iter().fold(String::with_capacity(64), |mut s, b| {
        use std::fmt::Write;
        let _ = write!(&mut s, "{:02X}", b);
        s
    })
}

pub async fn fetch_relay_config(
    host: &str,
    code: &str,
    beeper_token: Option<String>,
) -> Result<RelayConfig> {
    let versions = RelayConfig::get_versions(host, code, &beeper_token)
        .await
        .with_context(|| format!("fetching version info from {host}"))?;
    Ok(RelayConfig {
        version: versions,
        icloud_ua: "com.apple.iCloudHelper/282 CFNetwork/1408.0.4 Darwin/22.5.0".into(),
        aoskit_version: "com.apple.AOSKit/282 (com.apple.accountsd/113)".into(),
        dev_uuid: Uuid::new_v4().to_string(),
        protocol_version: 1660,
        host: host.to_string(),
        code: code.to_string(),
        beeper_token,
        udid: Some(generate_udid()),
    })
}

pub fn config_path(data_dir: &Path) -> PathBuf {
    data_dir.join("os_config.json")
}

pub fn save(path: &Path, config: &RelayConfig) -> Result<()> {
    let json = serde_json::to_string_pretty(config)?;
    std::fs::write(path, json).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

pub fn load(path: &Path) -> Result<Option<RelayConfig>> {
    if !path.exists() {
        return Ok(None);
    }
    let json = std::fs::read_to_string(path)?;
    Ok(Some(serde_json::from_str(&json)?))
}

pub fn clear(path: &Path) -> Result<()> {
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}
