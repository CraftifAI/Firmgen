//! Global ESP32 State Manager
//!
//! Provides persistent state across tool calls using lazy_static.
//! This includes session state, cache, and configuration.

use lazy_static::lazy_static;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use serde_json::Value;

use super::cache::ESP32Cache;
use super::config::ESP32Config;
use super::device_port_store::{
    self, PersistedDevicePort, PortResolutionPolicy,
};
use super::session_state::{DeviceConnection, ESP32SessionState};
use super::output_protocol::{SuggestedAction, ActionPriority};

lazy_static! {
    /// Global ESP32 state - persists across tool calls
    static ref ESP32_STATE: Arc<RwLock<ESP32GlobalState>> = Arc::new(RwLock::new(ESP32GlobalState::new()));
}

/// Combined global state for ESP32 tools
pub struct ESP32GlobalState {
    /// Session state (devices, projects, builds)
    pub session: ESP32SessionState,
    /// Caching layer
    pub cache: ESP32Cache,
    /// Cached configuration
    config: Option<ESP32Config>,
    /// API URL for configuration (can be overridden)
    api_url: String,
}

impl ESP32GlobalState {
    pub fn new() -> Self {
        Self {
            session: ESP32SessionState::new(),
            cache: ESP32Cache::new(),
            config: None,
            api_url: Self::default_api_url(),
        }
    }

    /// Get the default API URL, checking environment variable first
    fn default_api_url() -> String {
        std::env::var("REFACT_ESP32_CONFIG_URL")
            .unwrap_or_else(|_| "http://localhost:8002/v1/esp32-config".to_string())
    }

    /// Get or load configuration
    pub async fn get_config(&mut self) -> Result<ESP32Config, String> {
        // Return cached config if available
        if let Some(ref config) = self.config {
            return Ok(config.clone());
        }

        // Load from API or file
        let config = ESP32Config::load_from_api(&self.api_url).await?;
        self.config = Some(config.clone());
        Ok(config)
    }

    /// Force config reload
    pub fn invalidate_config(&mut self) {
        self.config = None;
    }

    /// Set custom API URL
    pub fn set_api_url(&mut self, url: String) {
        self.api_url = url;
        self.config = None; // Invalidate cached config
    }
}

/// Strip `/v1/esp32-config` suffix when present; otherwise return trimmed URL.
pub fn api_base_url_from_config_url(config_url: &str) -> String {
    let trimmed = config_url.trim_end_matches('/');
    if trimmed.ends_with("/v1/esp32-config") {
        trimmed
            .strip_suffix("/v1/esp32-config")
            .unwrap_or(trimmed)
            .to_string()
    } else {
        trimmed.to_string()
    }
}

/// API server base URL (scheme + host + port) for `/v1/boards/...` and similar routes.
///
/// `REFACT_ESP32_CONFIG_URL` is the full esp32-config endpoint
/// (e.g. `http://127.0.0.1:8002/v1/esp32-config`), not the server root — strip the path
/// before building board URLs. Optional override: `REFACT_API_BASE_URL`.
pub fn api_base_url() -> String {
    if let Ok(base) = std::env::var("REFACT_API_BASE_URL") {
        return base.trim_end_matches('/').to_string();
    }

    let config_url = std::env::var("REFACT_ESP32_CONFIG_URL")
        .unwrap_or_else(|_| "http://localhost:8002".to_string());
    api_base_url_from_config_url(&config_url)
}

/// Full URL for GET `/v1/boards/{board_id}`.
pub fn board_definition_url(board_id: &str) -> String {
    format!("{}/v1/boards/{}", api_base_url(), board_id)
}

#[cfg(test)]
mod api_url_tests {
    use super::*;

    #[test]
    fn api_base_url_strips_esp32_config_path() {
        assert_eq!(
            api_base_url_from_config_url("http://127.0.0.1:8002/v1/esp32-config"),
            "http://127.0.0.1:8002"
        );
        assert_eq!(
            format!("{}/v1/boards/{}", api_base_url_from_config_url("http://127.0.0.1:8002/v1/esp32-config"), "esp32-c6-devkitc-1"),
            "http://127.0.0.1:8002/v1/boards/esp32-c6-devkitc-1"
        );
    }

    #[test]
    fn api_base_url_passes_through_plain_base() {
        assert_eq!(
            api_base_url_from_config_url("http://localhost:8002"),
            "http://localhost:8002"
        );
    }
}

impl Default for ESP32GlobalState {
    fn default() -> Self {
        Self::new()
    }
}

/// Get the global state (read-only)
pub async fn get_state() -> tokio::sync::RwLockReadGuard<'static, ESP32GlobalState> {
    ESP32_STATE.read().await
}

/// Get the global state (read-write)
pub async fn get_state_mut() -> tokio::sync::RwLockWriteGuard<'static, ESP32GlobalState> {
    ESP32_STATE.write().await
}

/// Convenience function to get config.
/// Uses a read lock first to avoid blocking other readers when config is already cached.
pub async fn get_config() -> Result<ESP32Config, String> {
    // Fast path: check cached config with a read lock (non-blocking for other readers)
    {
        let state = get_state().await;
        if let Some(ref config) = state.config {
            return Ok(config.clone());
        }
    }
    // Slow path: acquire write lock to load and cache config
    let mut state = get_state_mut().await;
    state.get_config().await
}

/// Convenience function to get session state summary
pub async fn get_session_summary(max_tokens: usize) -> String {
    hydrate_session_from_disk().await;
    let state = get_state().await;
    state.session.get_context_summary(max_tokens)
}

/// Load persisted device port into in-memory session (once per process).
pub async fn hydrate_session_from_disk() {
    if device_port_store::hydration_complete() {
        return;
    }

    if let Some(persisted) = device_port_store::load_persisted_device_port().await {
        let mut state = get_state_mut().await;
        if state.session.active_device.is_none() {
            state.session.active_device = Some(DeviceConnection {
                port: persisted.port.clone(),
                chip: if persisted.chip.is_empty() {
                    "unknown".to_string()
                } else {
                    persisted.chip.clone()
                },
                mac_address: if persisted.mac_address.is_empty() {
                    "unknown".to_string()
                } else {
                    persisted.mac_address.clone()
                },
                flash_size: if persisted.flash_size.is_empty() {
                    "unknown".to_string()
                } else {
                    persisted.flash_size.clone()
                },
                connected_at: Instant::now(),
            });
        }
    }

    device_port_store::mark_hydration_complete();
}

/// Record the active device port in session and on disk.
pub async fn record_device_port(
    port: &str,
    chip: &str,
    mac_address: &str,
    flash_size: &str,
    source: &str,
) {
    {
        let mut state = get_state_mut().await;
        state.session.active_device = Some(DeviceConnection {
            port: port.to_string(),
            chip: chip.to_string(),
            mac_address: mac_address.to_string(),
            flash_size: flash_size.to_string(),
            connected_at: Instant::now(),
        });
    }

    let record = PersistedDevicePort::new(port, chip, mac_address, flash_size, source);
    device_port_store::persist_device_port(&record).await;
}

/// Update session + disk after flash/monitor using the port that was actually used.
pub async fn record_device_port_in_use(port: &str, source: &str) {
    let (chip, mac, flash_size) = {
        let state = get_state().await;
        if let Some(dev) = &state.session.active_device {
            (
                dev.chip.clone(),
                dev.mac_address.clone(),
                dev.flash_size.clone(),
            )
        } else {
            (
                "unknown".to_string(),
                "unknown".to_string(),
                "unknown".to_string(),
            )
        }
    };
    record_device_port(port, &chip, &mac, &flash_size, source).await;
}

/// Deterministic serial port resolution shared by esp32_device operations.
pub async fn resolve_device_port(
    config: &ESP32Config,
    args: &HashMap<String, Value>,
    policy: PortResolutionPolicy,
) -> Result<String, String> {
    hydrate_session_from_disk().await;

    let explicit = args.get("port").and_then(|v| v.as_str());
    let (session_port, persisted_port) = {
        let state = get_state().await;
        let session_port = state.session.active_device.as_ref().map(|d| d.port.clone());
        drop(state);
        let persisted_port = device_port_store::load_persisted_device_port()
            .await
            .map(|p| p.port);
        (session_port, persisted_port)
    };

    device_port_store::resolve_port_from_candidates(
        explicit,
        session_port.as_deref(),
        persisted_port.as_deref(),
        &config.default_serial_port,
        policy,
    )
}

/// Generate suggested actions based on operation result
pub fn generate_suggested_actions(
    operation: &str,
    success: bool,
    context: &SuggestionContext,
) -> Vec<SuggestedAction> {
    let mut actions = Vec::new();

    match (operation, success) {
        // After successful build
        ("build", true) => {
            actions.push(SuggestedAction {
                action: "esp32_device".to_string(),
                reason: "Flash the built firmware to device".to_string(),
                parameters: [
                    ("operation".to_string(), "flash".to_string()),
                ].iter().cloned().collect(),
                priority: ActionPriority::High,
            });
        }

        // After failed build
        ("build", false) => {
            // Note: esp32_analyze is disabled — suggest fixing and rebuilding instead
            if context.has_missing_component {
                actions.push(SuggestedAction {
                    action: "esp32_component".to_string(),
                    reason: "Search for and install the missing component".to_string(),
                    parameters: [
                        ("operation".to_string(), "search".to_string()),
                    ].iter().cloned().collect(),
                    priority: ActionPriority::High,
                });
            }
            actions.push(SuggestedAction {
                action: "esp32_build".to_string(),
                reason: "Retry build after applying fix from diagnostics above".to_string(),
                parameters: [
                    ("operation".to_string(), "build".to_string()),
                ].iter().cloned().collect(),
                priority: ActionPriority::High,
            });
        }

        // After successful flash
        ("flash", true) => {
            actions.push(SuggestedAction {
                action: "esp32_device".to_string(),
                reason: "Monitor device output".to_string(),
                parameters: [
                    ("operation".to_string(), "monitor".to_string()),
                ].iter().cloned().collect(),
                priority: ActionPriority::High,
            });
        }

        // After flash failure
        ("flash", false) => {
            actions.push(SuggestedAction {
                action: "esp32_device".to_string(),
                reason: "Check device connection".to_string(),
                parameters: [
                    ("operation".to_string(), "detect".to_string()),
                ].iter().cloned().collect(),
                priority: ActionPriority::High,
            });
            actions.push(SuggestedAction {
                action: "esp32_device".to_string(),
                reason: "Get chip information".to_string(),
                parameters: [
                    ("operation".to_string(), "info".to_string()),
                ].iter().cloned().collect(),
                priority: ActionPriority::Medium,
            });
        }

        // After device detection with no devices
        ("detect", true) if !context.devices_found => {
            actions.push(SuggestedAction {
                action: "manual".to_string(),
                reason: "Connect device and hold BOOT button while resetting".to_string(),
                parameters: [].iter().cloned().collect(),
                priority: ActionPriority::High,
            });
        }

        // After successful device detection with board_id set - suggest verify
        ("detect", true) if context.devices_found && context.board_id_set => {
            actions.push(SuggestedAction {
                action: "esp32_device".to_string(),
                reason: "Verify device matches board definition".to_string(),
                parameters: [
                    ("operation".to_string(), "verify".to_string()),
                ].iter().cloned().collect(),
                priority: ActionPriority::High,
            });
        }

        // After project creation
        ("create", true) => {
            actions.push(SuggestedAction {
                action: "esp32_build".to_string(),
                reason: "Build the new project".to_string(),
                parameters: [
                    ("operation".to_string(), "build".to_string()),
                ].iter().cloned().collect(),
                priority: ActionPriority::High,
            });
        }

        // After project creation conflict or failure
        ("create", false) => {
            if let Some(ref name) = context.create_project_name {
                let mut use_params = HashMap::new();
                use_params.insert("operation".to_string(), "create".to_string());
                use_params.insert("project_name".to_string(), name.clone());
                use_params.insert("if_exists".to_string(), "use".to_string());
                if let Some(ref t) = context.create_template {
                    use_params.insert("template".to_string(), t.clone());
                }
                if let Some(ref t) = context.create_target {
                    use_params.insert("target".to_string(), t.clone());
                }
                actions.push(SuggestedAction {
                    action: "esp32_project".to_string(),
                    reason: "Reuse the existing project folder if it is valid".to_string(),
                    parameters: use_params,
                    priority: ActionPriority::High,
                });

                let mut replace_params = HashMap::new();
                replace_params.insert("operation".to_string(), "create".to_string());
                replace_params.insert("project_name".to_string(), name.clone());
                replace_params.insert("if_exists".to_string(), "replace".to_string());
                if let Some(ref t) = context.create_template {
                    replace_params.insert("template".to_string(), t.clone());
                }
                if let Some(ref t) = context.create_target {
                    replace_params.insert("target".to_string(), t.clone());
                }
                actions.push(SuggestedAction {
                    action: "esp32_project".to_string(),
                    reason: "Delete the existing folder and recreate the project".to_string(),
                    parameters: replace_params,
                    priority: ActionPriority::Medium,
                });

                if let Some(alt) = context.create_suggested_names.first() {
                    let mut alt_params = HashMap::new();
                    alt_params.insert("operation".to_string(), "create".to_string());
                    alt_params.insert("project_name".to_string(), alt.clone());
                    alt_params.insert("if_exists".to_string(), "auto_suffix".to_string());
                    if let Some(ref t) = context.create_template {
                        alt_params.insert("template".to_string(), t.clone());
                    }
                    if let Some(ref t) = context.create_target {
                        alt_params.insert("target".to_string(), t.clone());
                    }
                    actions.push(SuggestedAction {
                        action: "esp32_project".to_string(),
                        reason: format!("Create under an available name '{}'", alt),
                        parameters: alt_params,
                        priority: ActionPriority::Medium,
                    });
                }
            }
            actions.push(SuggestedAction {
                action: "esp32_project".to_string(),
                reason: "List existing projects in the workspace before choosing a name".to_string(),
                parameters: [
                    ("operation".to_string(), "list_projects".to_string()),
                ].iter().cloned().collect(),
                priority: ActionPriority::Low,
            });
        }

        // After component search
        ("search", true) if context.has_results => {
            actions.push(SuggestedAction {
                action: "esp32_component".to_string(),
                reason: "Add a component from search results".to_string(),
                parameters: [
                    ("operation".to_string(), "add".to_string()),
                ].iter().cloned().collect(),
                priority: ActionPriority::Medium,
            });
        }

        // After component add
        ("add", true) => {
            actions.push(SuggestedAction {
                action: "esp32_build".to_string(),
                reason: "Rebuild project with new component".to_string(),
                parameters: [
                    ("operation".to_string(), "build".to_string()),
                ].iter().cloned().collect(),
                priority: ActionPriority::High,
            });
        }

        // Default: no suggestions
        _ => {}
    }

    actions
}

/// Context for generating suggestions
#[derive(Default)]
pub struct SuggestionContext {
    pub has_missing_component: bool,
    pub devices_found: bool,
    pub has_results: bool,
    pub project_path: Option<String>,
    pub board_id_set: bool,
    pub create_project_name: Option<String>,
    pub create_template: Option<String>,
    pub create_target: Option<String>,
    pub create_suggested_names: Vec<String>,
}

impl SuggestionContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_missing_component(mut self) -> Self {
        self.has_missing_component = true;
        self
    }

    pub fn with_devices(mut self, found: bool) -> Self {
        self.devices_found = found;
        self
    }

    pub fn with_results(mut self, has: bool) -> Self {
        self.has_results = has;
        self
    }

    pub fn with_board_id(mut self, set: bool) -> Self {
        self.board_id_set = set;
        self
    }

    pub fn with_create_params(
        mut self,
        project_name: &str,
        suggested_names: Vec<String>,
    ) -> Self {
        self.create_project_name = Some(project_name.to_string());
        self.create_suggested_names = suggested_names;
        self
    }

    pub fn with_create_template(mut self, template: &str) -> Self {
        self.create_template = Some(template.to_string());
        self
    }

    pub fn with_create_target(mut self, target: &str) -> Self {
        self.create_target = Some(target.to_string());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_global_state() {
        let state = get_state().await;
        assert!(state.config.is_none());
    }

    #[test]
    fn test_suggestions_after_build_success() {
        let actions = generate_suggested_actions("build", true, &SuggestionContext::new());
        assert!(!actions.is_empty());
        assert!(actions.iter().any(|a| a.action == "esp32_device"));
    }

    #[test]
    fn test_suggestions_after_build_failure() {
        let actions = generate_suggested_actions("build", false, &SuggestionContext::new());
        assert!(!actions.is_empty());
        assert!(actions.iter().any(|a| a.action == "esp32_build"));
    }
}

