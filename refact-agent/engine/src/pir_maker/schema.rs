//! PIR (Project Intelligence Representation) JSON schema — v1.0.0

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

use crate::firmware_topology::{FirmwareGraph, ValidationReport};

pub const PIR_SCHEMA_VERSION: &str = "2.0.0";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PirApprovalStatus {
    Pending,
    Approved,
    Rejected,
    Stale,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PirSyncState {
    Synced,
    Stale,
    Conflict,
    Manual,
    Analyzing,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum NodeAuthority {
    #[default]
    Agent,
    User,
    Hybrid,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PirGenerationMeta {
    #[serde(default = "default_generation_mode")]
    pub mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default = "default_triggered_by")]
    pub triggered_by: String,
    pub analyzed_at_ms: u64,
    #[serde(default)]
    pub input_files: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_usage: Option<TokenUsage>,
}

fn default_generation_mode() -> String {
    "ai_full".to_string()
}
fn default_triggered_by() -> String {
    "agent_idle".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FilePatch {
    pub file: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PirPendingPatch {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edge_id: Option<String>,
    #[serde(default)]
    pub file_patches: Vec<FilePatch>,
    #[serde(default = "default_patch_status")]
    pub status: String,
}

fn default_patch_status() -> String {
    "proposed".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PirSyncMetadata {
    #[serde(default = "default_pir_project_file")]
    pub project_file: String,
    #[serde(default)]
    pub content_hash: String,
    #[serde(default)]
    pub pending_patches: Vec<PirPendingPatch>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_diff: Option<TopologyDiff>,
}

fn default_pir_project_file() -> String {
    ".craftif/pir.json".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PirValidationState {
    pub valid: bool,
    #[serde(default)]
    pub error_count: u32,
    #[serde(default)]
    pub warning_count: u32,
    pub validated_at_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PirEdgeKind {
    Execution,
    Data,
    Hardware,
    Dependency,
    Event,
    Network,
    Ota,
    Fsm,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceRef {
    pub file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    #[serde(default = "default_confidence")]
    pub confidence: f32,
    #[serde(default = "default_inferred_by")]
    pub inferred_by: String,
}

fn default_confidence() -> f32 {
    1.0
}
fn default_inferred_by() -> String {
    "static".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FileOwnership {
    pub primary_files: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub component_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PirNodeSync {
    pub state: PirSyncState,
    pub last_synced_revision: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PirNode {
    pub id: String,
    pub node_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default)]
    pub properties: JsonValue,
    #[serde(default)]
    pub source_refs: Vec<SourceRef>,
    pub sync: PirNodeSync,
    pub ownership: FileOwnership,
    #[serde(default)]
    pub editable_fields: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ai_summary: Option<String>,
    #[serde(default = "default_confidence")]
    pub confidence: f32,
    #[serde(default)]
    pub authority: NodeAuthority,
    #[serde(default)]
    pub semantic_tags: Vec<String>,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stale_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PirEdge {
    pub id: String,
    pub source_node_id: String,
    pub target_node_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_port_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_port_id: Option<String>,
    pub kind: PirEdgeKind,
    #[serde(default = "default_confidence")]
    pub confidence: f32,
    #[serde(default)]
    pub source_refs: Vec<SourceRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_label: Option<String>,
    #[serde(default)]
    pub validated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PirLayers {
    #[serde(default)]
    pub physical: Vec<String>,
    #[serde(default)]
    pub runtime: Vec<String>,
    #[serde(default)]
    pub network: Vec<String>,
    #[serde(default)]
    pub system: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PirPartition {
    pub name: String,
    #[serde(rename = "type")]
    pub partition_type: String,
    pub subtype: String,
    pub offset: String,
    pub size: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PirComponent {
    pub id: String,
    pub path: String,
    #[serde(default)]
    pub requires: Vec<String>,
    #[serde(default)]
    pub source_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PirProvenance {
    pub project_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chat_id: Option<String>,
    pub revision: String,
    pub generated_at_ms: u64,
    pub analyzer_version: String,
    #[serde(default)]
    pub analyzed_files: Vec<String>,
    #[serde(default)]
    pub file_hashes: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub board_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PirApproval {
    pub status: PirApprovalStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approved_at_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approved_revision: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
}

impl Default for PirApproval {
    fn default() -> Self {
        Self {
            status: PirApprovalStatus::Pending,
            approved_at_ms: None,
            approved_revision: None,
            comment: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PirSummary {
    pub headline: String,
    #[serde(default)]
    pub node_count: u32,
    #[serde(default)]
    pub edge_count: u32,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PirDiagramMermaid {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mermaid: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PirSequenceDiagram {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mermaid: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub participants: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub generated_from: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PirDiagrams {
    /// High-level diagram block generated by the PIR agent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hld: Option<PirDiagramMermaid>,
    /// Low-level diagram block generated by the PIR agent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lld: Option<PirDiagramMermaid>,
    /// Sequence diagram block generated by the PIR agent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sequence: Option<PirSequenceDiagram>,
    /// Backend-owned HLD diagram graph for interactive rendering (ReactFlow).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hld_graph: Option<FirmwareGraph>,
    /// Backend-owned LLD diagram graph for interactive rendering (ReactFlow).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lld_graph: Option<FirmwareGraph>,
    /// Backend-owned sequence diagram graph for interactive rendering (ReactFlow).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sequence_graph: Option<FirmwareGraph>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PirChangeEntry {
    pub id: String,
    pub revision: String,
    pub timestamp_ms: u64,
    pub node_id: String,
    pub property: String,
    pub old_value: JsonValue,
    pub new_value: JsonValue,
    pub files_patched: Vec<String>,
    pub reversible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PirDocument {
    pub schema_version: String,
    pub id: String,
    pub revision: String,
    pub provenance: PirProvenance,
    pub approval: PirApproval,
    pub nodes: Vec<PirNode>,
    pub edges: Vec<PirEdge>,
    pub layers: PirLayers,
    #[serde(default)]
    pub partitions: Vec<PirPartition>,
    #[serde(default)]
    pub components: Vec<PirComponent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<PirSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagrams: Option<PirDiagrams>,
    #[serde(default)]
    pub change_log: Vec<PirChangeEntry>,
    #[serde(default)]
    pub unresolved: Vec<JsonValue>,
    #[serde(default)]
    pub graph_version: u32,
    #[serde(default)]
    pub generation: PirGenerationMeta,
    #[serde(default)]
    pub sync_metadata: PirSyncMetadata,
    #[serde(default)]
    pub validation_state: PirValidationState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyDiff {
    pub from_revision: String,
    pub to_revision: String,
    pub nodes_added: Vec<String>,
    pub nodes_removed: Vec<String>,
    pub nodes_changed: Vec<String>,
    pub edges_added: Vec<String>,
    pub edges_removed: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PirAnalyzeResult {
    pub status: String,
    pub pir: PirDocument,
    pub graph: FirmwareGraph,
    pub validation: ValidationReport,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff: Option<TopologyDiff>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AnalysisFacts {
    pub project_name: String,
    pub target_chip: Option<String>,
    pub board_id: Option<String>,
    pub has_app_main: bool,
    pub app_main_file: Option<String>,
    pub gpio_facts: Vec<GpioFact>,
    pub task_facts: Vec<TaskFact>,
    pub network_facts: Vec<NetworkFact>,
    pub partitions: Vec<PirPartition>,
    pub components: Vec<PirComponent>,
    pub file_hashes: HashMap<String, String>,
    pub analyzed_files: Vec<String>,
    pub unresolved: Vec<JsonValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpioFact {
    pub node_id: String,
    pub node_type: String,
    pub label: String,
    pub pin: u8,
    pub file: String,
    pub line: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskFact {
    pub node_id: String,
    pub task_name: String,
    pub priority: Option<u8>,
    pub stack_size: Option<u32>,
    pub period_ms: Option<f64>,
    pub file: String,
    pub line: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkFact {
    pub node_id: String,
    pub node_type: String,
    pub label: String,
    pub file: String,
    pub properties: serde_json::Value,
}
