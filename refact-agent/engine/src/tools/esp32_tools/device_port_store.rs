//! Persisted ESP32 serial port — survives refact-lsp restarts.

use std::path::PathBuf;
use std::sync::OnceLock;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use tokio::sync::OnceCell;

static PERSIST_DIR: OnceLock<PathBuf> = OnceLock::new();
static HYDRATED: OnceCell<()> = OnceCell::const_new();

const DEVICE_PORT_FILE: &str = "device_port.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PersistedDevicePort {
    pub port: String,
    #[serde(default)]
    pub chip: String,
    #[serde(default)]
    pub mac_address: String,
    #[serde(default)]
    pub flash_size: String,
    pub source: String,
    pub updated_at: String,
}

impl PersistedDevicePort {
    pub fn new(port: &str, chip: &str, mac_address: &str, flash_size: &str, source: &str) -> Self {
        Self {
            port: port.to_string(),
            chip: chip.to_string(),
            mac_address: mac_address.to_string(),
            flash_size: flash_size.to_string(),
            source: source.to_string(),
            updated_at: Utc::now().to_rfc3339(),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PortResolutionPolicy {
    /// flash/erase/info — config default only as last resort
    PreferKnown,
    /// monitor — fail if only the yaml default would be used
    RequireKnown,
}

/// Initialize on-disk persistence location (call once at LSP startup).
pub async fn init_device_port_persistence(cache_dir: PathBuf) {
    let dir = cache_dir.join("esp32");
    let _ = tokio::fs::create_dir_all(&dir).await;
    let _ = PERSIST_DIR.set(dir);
}

fn persist_dir() -> Option<&'static PathBuf> {
    PERSIST_DIR.get()
}

fn device_port_path() -> Option<PathBuf> {
    persist_dir().map(|d| d.join(DEVICE_PORT_FILE))
}

pub async fn load_persisted_device_port() -> Option<PersistedDevicePort> {
    let path = device_port_path()?;
    let bytes = tokio::fs::read(&path).await.ok()?;
    serde_json::from_slice(&bytes).ok()
}

pub fn hydration_complete() -> bool {
    HYDRATED.get().is_some()
}

pub fn mark_hydration_complete() {
    let _ = HYDRATED.set(());
}

pub async fn persist_device_port(record: &PersistedDevicePort) {
    let Some(path) = device_port_path() else { return };
    if let Some(parent) = path.parent() {
        let _ = tokio::fs::create_dir_all(parent).await;
    }
    if let Ok(json) = serde_json::to_vec_pretty(record) {
        let _ = tokio::fs::write(&path, json).await;
    }
}

/// When explicit `port` equals the yaml default but session/persisted know a different port,
/// treat the explicit value as stale (e.g. LLM guessing COM3).
pub fn resolve_port_from_candidates(
    explicit: Option<&str>,
    session_port: Option<&str>,
    persisted_port: Option<&str>,
    config_default: &str,
    policy: PortResolutionPolicy,
) -> Result<String, String> {
    let known = session_port.or(persisted_port);

    if let Some(explicit_port) = explicit.filter(|p| !p.is_empty()) {
        if let Some(known_port) = known {
            if explicit_port == config_default && explicit_port != known_port {
                return Ok(known_port.to_string());
            }
        }
        return Ok(explicit_port.to_string());
    }

    if let Some(port) = session_port {
        return Ok(port.to_string());
    }

    if let Some(port) = persisted_port {
        return Ok(port.to_string());
    }

    match policy {
        PortResolutionPolicy::RequireKnown => Err(
            "No device port in session. Run esp32_device(operation=\"detect\") first, or pass an explicit port.".to_string(),
        ),
        PortResolutionPolicy::PreferKnown => Ok(config_default.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignores_yaml_default_when_session_has_different_port() {
        let port = resolve_port_from_candidates(
            Some("COM3"),
            Some("COM96"),
            None,
            "COM3",
            PortResolutionPolicy::RequireKnown,
        )
        .unwrap();
        assert_eq!(port, "COM96");
    }

    #[test]
    fn monitor_fails_without_known_port() {
        let err = resolve_port_from_candidates(
            None,
            None,
            None,
            "COM3",
            PortResolutionPolicy::RequireKnown,
        )
        .unwrap_err();
        assert!(err.contains("detect"));
    }

    #[test]
    fn monitor_uses_persisted_port_after_restart() {
        let port = resolve_port_from_candidates(
            None,
            None,
            Some("COM96"),
            "COM3",
            PortResolutionPolicy::RequireKnown,
        )
        .unwrap();
        assert_eq!(port, "COM96");
    }

    #[test]
    fn explicit_non_default_port_is_honored() {
        let port = resolve_port_from_candidates(
            Some("COM7"),
            Some("COM96"),
            None,
            "COM3",
            PortResolutionPolicy::PreferKnown,
        )
        .unwrap();
        assert_eq!(port, "COM7");
    }
}
