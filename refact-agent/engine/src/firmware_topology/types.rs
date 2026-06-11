use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PortDirection {
    Input,
    Output,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SignalMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dtype: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shape: Option<Vec<i64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_hz: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoding: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HardwareMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpio: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bus: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peripheral: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pin_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub i2c_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spi_host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uart_port: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Port {
    pub id: String,
    pub name: String,
    pub direction: PortDirection,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub datatype: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signal: Option<SignalMetadata>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hardware: Option<HardwareMetadata>,
    #[serde(default)]
    pub required: bool,
    #[serde(default = "default_multiplicity")]
    pub multiplicity: String,
}

fn default_multiplicity() -> String {
    "one".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExecutionMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stack_size: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub core_affinity: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub period_ms: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trigger: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VisualMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub y: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layer: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collapsed: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirmwareNode {
    pub id: String,
    pub node_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub ports: Vec<Port>,
    #[serde(default)]
    pub properties: JsonValue,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hardware: Option<HardwareMetadata>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution: Option<ExecutionMetadata>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visual: Option<VisualMetadata>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation_state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime_state: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LayoutConfig {
    #[serde(default = "default_orientation")]
    pub orientation: String,
    #[serde(default = "default_node_width")]
    pub node_width: f64,
    #[serde(default = "default_layer_gap")]
    pub layer_gap: f64,
    #[serde(default = "default_node_gap")]
    pub node_gap: f64,
}

fn default_orientation() -> String {
    "horizontal".to_string()
}
fn default_node_width() -> f64 {
    280.0
}
fn default_layer_gap() -> f64 {
    100.0
}
fn default_node_gap() -> f64 {
    60.0
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RuntimeMetadata {
    #[serde(default)]
    pub telemetry_enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_updated_ms: Option<u64>,
    #[serde(default)]
    pub overlays: HashMap<String, JsonValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirmwareGraph {
    pub schema_version: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub board_id: Option<String>,
    pub nodes: Vec<FirmwareNode>,
    pub connections: Vec<[String; 2]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layout: Option<LayoutConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime_metadata: Option<RuntimeMetadata>,
}

impl FirmwareGraph {
    pub fn new() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            id: None,
            name: None,
            description: None,
            board_id: None,
            nodes: Vec::new(),
            connections: Vec::new(),
            layout: None,
            runtime_metadata: None,
        }
    }

    pub fn port_index(&self) -> HashMap<String, (String, Port)> {
        let mut map = HashMap::new();
        for node in &self.nodes {
            for port in &node.ports {
                map.insert(port.id.clone(), (node.id.clone(), port.clone()));
            }
        }
        map
    }
}

impl Default for FirmwareGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssue {
    pub code: String,
    pub message: String,
    pub severity: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection: Option<[String; 2]>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    pub valid: bool,
    pub issues: Vec<ValidationIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyFieldDef {
    pub key: String,
    pub label: String,
    #[serde(rename = "type")]
    pub field_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<JsonValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    #[serde(default)]
    pub read_only: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeTypeDefResponse {
    pub node_type: String,
    pub category: String,
    pub label: String,
    pub color: String,
    pub icon: String,
    pub description: String,
    pub ports: Vec<PortDefResponse>,
    pub properties: Vec<PropertyFieldDef>,
    pub execution_semantics: ExecutionSemantics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortDefResponse {
    pub name: String,
    pub direction: PortDirection,
    pub datatype: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionSemantics {
    pub phase: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trigger: Option<String>,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutPosition {
    pub node_id: String,
    pub x: f64,
    pub y: f64,
    pub layer: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutResultResponse {
    pub positions: Vec<LayoutPosition>,
    pub has_cycles: bool,
    pub orientation: String,
}
