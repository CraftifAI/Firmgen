// ESP32 Tools Module
// Revolutionary architecture: Context-first, state-aware, LLM-optimized

pub mod config;
pub mod session_state;
pub mod output_protocol;
pub mod cache;
pub mod error_parser;
pub mod idf_command;
pub mod global_state;
pub mod device_port_store;

// Path resolution helpers (shared by HTTP routes and esp32_project tool)
pub mod esp32_path_resolve;

// Composite tools
pub mod esp32_project;
pub mod esp32_build;
pub mod esp32_device;
pub mod esp32_config;
pub mod esp32_component;
// Disabled: esp32_analyze uses subchat_single which causes UI state issues
// pub mod esp32_analyze;
pub mod board_definition;

// Re-export main tools for easy access
pub use esp32_project::ESP32Project;
pub use esp32_build::ESP32Build;
pub use esp32_device::ESP32Device;
pub use esp32_config::ESP32ConfigTool;
pub use esp32_component::ESP32Component;
// Disabled: ESP32Analyze causes UI flickering due to nested LLM calls
// pub use esp32_analyze::ESP32Analyze;

// Note: idf_command, global_state, session_state, cache are internal implementation details
// Tools use them internally but don't need to be re-exported

