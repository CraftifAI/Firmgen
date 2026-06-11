//! Firmware execution topology graph — core types and engine.
//!
//! Models ESP32 firmware modules, RTOS tasks, peripherals, and data flow as a
//! structured graph for visualization, validation, and future code generation.

pub mod layout;
pub mod registry;
pub mod samples;
pub mod types;
pub mod validator;

pub use layout::{compute_layout, LayoutOrientation, LayoutResult};
pub use registry::{get_node_type_def, list_node_types, NodeTypeDef, PortDef};
pub use samples::{sample_blink, sample_pir_motion};
pub use types::*;
pub use validator::validate_graph;
