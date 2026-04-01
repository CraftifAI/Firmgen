//! Global ESP32 State Manager
//!
//! Provides persistent state across tool calls using lazy_static.
//! This includes session state, cache, and configuration.

use lazy_static::lazy_static;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::cache::ESP32Cache;
use super::config::ESP32Config;
use super::session_state::ESP32SessionState;
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
    let state = get_state().await;
    state.session.get_context_summary(max_tokens)
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

