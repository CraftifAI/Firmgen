//! # Progress bar & tool activity tracker
//!
//! Tracks ESP32 pipeline tools (build/flash/…) and generic agent tools (cat,
//! `create_textdoc`, …) for observability and the GUI progress bar.
//!
//! ## Usage
//!
//! Use `Esp32ToolEvent` to record tool invocations. The `Esp32ToolRegistry`
//! provides metadata about all available ESP32 tools and their operations.
//!
//! ## Real-time progress for GUI
//!
//! The `ProgressStore` holds the latest tool events per chat_id. ESP32 tools
//! that embed `ToolOutput` call `record_tool_start` / `record_tool_complete`
//! locally. All other tools are recorded centrally from `tools_execute` via
//! `record_tool_start` + `record_generic_tool_success` / `record_tool_error`.
//! The GUI polls `/v1/progress?chat_id=...`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::OnceLock;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;

use crate::tools::esp32_tools::output_protocol as esp32_output;

/// Workflow node for the progress bar (Planning → Generation → Compiling → Flashing → Monitoring).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowNode {
    Planning,
    Generation,
    Compiling,
    Flashing,
    Monitoring,
}

impl WorkflowNode {
    pub fn ordinal(&self) -> u8 {
        match self {
            WorkflowNode::Planning => 0,
            WorkflowNode::Generation => 1,
            WorkflowNode::Compiling => 2,
            WorkflowNode::Flashing => 3,
            WorkflowNode::Monitoring => 4,
        }
    }
}

/// Agent/workspace tools that mutate artifacts → Generation stage.
/// Keep in sync with GUI `WORKSPACE_GENERATION_TOOLS` in useWorkflowStatus.ts.
pub const WORKSPACE_GENERATION_TOOL_NAMES: &[&str] = &[
    "create_textdoc",
    "update_textdoc",
    "update_textdoc_regex",
    "create_knowledge",
    "create_memory_bank",
    "rm",
    "mv",
];

/// Read-heavy / planning / discovery tools → Planning stage.
/// Keep in sync with GUI `WORKSPACE_PLANNING_TOOL_NAMES` in useWorkflowStatus.ts.
pub const WORKSPACE_PLANNING_TOOL_NAMES: &[&str] = &[
    "strategic_planning",
    "task_list",
    "tree",
    "cat",
    "search_semantic",
    "search_pattern",
    "locate",
    "search_symbol_definition",
    "search_symbol_usages",
    "web",
    "knowledge",
];

fn is_workspace_generation_tool(name: &str) -> bool {
    WORKSPACE_GENERATION_TOOL_NAMES.iter().any(|t| *t == name)
}

fn is_workspace_planning_tool(name: &str) -> bool {
    WORKSPACE_PLANNING_TOOL_NAMES.iter().any(|t| *t == name)
}

impl WorkflowNode {
    /// Map tool name + operation to workflow node.
    ///
    /// Order: ESP32 hardware pipeline (flash / compile / IDF generation), then
    /// workspace agent tools (explicit lists), then default Planning.
    pub fn from_tool_operation(tool_name: &str, operation: &str) -> WorkflowNode {
        let name = tool_name.to_lowercase();
        let op = operation.to_lowercase();
        let combined = format!("{} {}", name, op);

        // Monitoring: device monitor (runtime observation, separate from flashing)
        if combined.contains("esp32_device") && op.contains("monitor") {
            return WorkflowNode::Monitoring;
        }

        // Flashing: device detect, flash, erase
        if combined.contains("esp32_device") {
            if op.contains("detect") || op.contains("flash") || op.contains("erase") {
                return WorkflowNode::Flashing;
            }
        }

        // Compiling: build, clean, reconfigure
        if name == "esp32_build" {
            if op.contains("build") || op.contains("clean") || op.contains("reconfigure") {
                return WorkflowNode::Compiling;
            }
        }

        // Generation: project create, config changes, component add/remove
        if name == "esp32_project" && op.contains("create") {
            return WorkflowNode::Generation;
        }
        if name == "esp32_config" {
            return WorkflowNode::Generation;
        }
        if name == "esp32_component" && (op.contains("add") || op.contains("remove")) {
            return WorkflowNode::Generation;
        }

        // Workspace tools (text docs, knowledge, fs mutators, discovery)
        if is_workspace_generation_tool(&name) {
            return WorkflowNode::Generation;
        }
        if is_workspace_planning_tool(&name) {
            return WorkflowNode::Planning;
        }

        // ESP32 read-only ops and everything else → Planning
        WorkflowNode::Planning
    }

    pub fn as_stage_label(&self) -> &'static str {
        match self {
            WorkflowNode::Planning => "planning",
            WorkflowNode::Generation => "generation",
            WorkflowNode::Compiling => "compiling",
            WorkflowNode::Flashing => "flashing",
            WorkflowNode::Monitoring => "monitoring",
        }
    }
}

// =============================================================================
// Core Event Types
// =============================================================================

/// Execution status or output type for a tool call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolExecutionStatus {
    /// Tool call completed successfully.
    Success,
    /// Tool call completed with partial success (e.g., some operations succeeded).
    PartialSuccess,
    /// Tool call failed.
    Failure,
    /// Tool is currently executing.
    Ongoing,
    /// Tool call is queued or waiting to start.
    Pending,
    /// Result was served from cache (no actual execution).
    Cached,
    /// Tool call was skipped (e.g., preconditions not met).
    Skipped,
}

/// A single ESP32 tool event or function call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Esp32ToolEvent {
    /// Unique identifier for this tool call (e.g., tool_call_id from LLM).
    pub id: String,

    /// Tool name (e.g., "esp32_project", "esp32_build").
    pub tool_name: String,

    /// Operation within the tool (e.g., "create", "build", "flash").
    pub operation: String,

    /// Wall-clock time when the tool was invoked.
    pub invoked_at: SystemTime,

    /// Time when execution completed (None if ongoing or pending).
    pub completed_at: Option<SystemTime>,

    /// Current status or output type.
    pub status: ToolExecutionStatus,

    /// Input parameters passed to the tool.
    pub input_params: HashMap<String, serde_json::Value>,

    /// Output or response from the tool (summary, structured data).
    pub output: Option<ToolOutputSnapshot>,

    /// Error details, if any.
    pub error: Option<ErrorDetails>,

    /// Execution duration (computed from invoked_at and completed_at, or measured).
    pub execution_duration: Option<Duration>,

    /// Additional notes, logs, or debug information.
    pub notes: Vec<String>,
}

/// Snapshot of tool output for tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolOutputSnapshot {
    /// Brief summary (max ~50 tokens).
    pub summary: String,

    /// Optional detailed output.
    pub details: Option<String>,

    /// Structured data (JSON).
    pub data: Option<serde_json::Value>,

    /// Action taken (e.g., "cached", "build").
    pub action_taken: Option<String>,
}

/// API-friendly event DTO (timestamps as ISO strings for JSON).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressEventDto {
    pub id: String,
    pub tool_name: String,
    pub operation: String,
    pub node: WorkflowNode,
    pub status: ToolExecutionStatus,
    pub invoked_at_iso: String,
    pub completed_at_iso: Option<String>,
    pub summary: Option<String>,
    pub has_error: bool,
    pub execution_duration_secs: Option<f64>,
}

/// Progress session for a chat (events + derived state for GUI).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressSessionDto {
    pub chat_id: String,
    pub events: Vec<ProgressEventDto>,
    pub current_node: WorkflowNode,
    pub overall_percentage: u8,
    pub activity_label: String,
    pub has_active_run: bool,
    pub has_error: bool,
    /// True when the current stage had an error and the agent is actively fixing it
    /// (running tools from a previous stage).
    pub is_debugging: bool,
    /// Number of error→fix iterations at the current locked stage.
    pub debug_iteration: u32,
    pub esp32_project_path: Option<String>,
}

/// Error details for failed or partially failed tool calls.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorDetails {
    /// Error category (e.g., "Build", "Flash", "Hardware").
    pub category: String,

    /// Subcategory for finer classification.
    pub subcategory: String,

    /// Human-readable error message.
    pub message: String,

    /// Source file, if applicable.
    pub file: Option<String>,

    /// Line number, if applicable.
    pub line: Option<usize>,

    /// Column, if applicable.
    pub column: Option<usize>,

    /// Hints for fixing the issue.
    pub fix_hints: Vec<String>,

    /// Related documentation references.
    pub related_docs: Vec<String>,
}

// =============================================================================
// ESP32 Tool Registry
// =============================================================================

/// Metadata for a single tool operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolOperationInfo {
    pub name: String,
    pub description: String,
    pub required_params: Vec<String>,
    pub optional_params: Vec<String>,
}

/// Metadata for an ESP32 tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Esp32ToolInfo {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub operations: Vec<ToolOperationInfo>,
    pub depends_on: Vec<String>,
}

/// Registry of all ESP32 tools and their operations.
/// Use this for documentation, validation, and monitoring.
pub struct Esp32ToolRegistry;

impl Esp32ToolRegistry {
    /// Returns metadata for all ESP32 tools.
    pub fn all_tools() -> Vec<Esp32ToolInfo> {
        vec![
            Self::esp32_project(),
            Self::esp32_build(),
            Self::esp32_device(),
            Self::esp32_config(),
            Self::esp32_component(),
            Self::esp32_analyze(),
        ]
    }

    /// Get tool info by name.
    pub fn get_tool(name: &str) -> Option<Esp32ToolInfo> {
        Self::all_tools().into_iter().find(|t| t.name == name)
    }

    /// Get all tool names.
    pub fn tool_names() -> Vec<&'static str> {
        vec![
            "esp32_project",
            "esp32_build",
            "esp32_device",
            "esp32_config",
            "esp32_component",
            "esp32_analyze",
        ]
    }

    fn esp32_project() -> Esp32ToolInfo {
        Esp32ToolInfo {
            name: "esp32_project".to_string(),
            display_name: "ESP32 Project".to_string(),
            description: "Manage ESP32 projects. Operations: create, list_projects, list_examples, validate.".to_string(),
            operations: vec![
                ToolOperationInfo {
                    name: "create".to_string(),
                    description: "Create new project from template/example".to_string(),
                    required_params: vec!["operation".to_string(), "project_name".to_string()],
                    optional_params: vec![
                        "template".to_string(),
                        "target".to_string(),
                        "if_exists".to_string(),
                    ],
                },
                ToolOperationInfo {
                    name: "list_projects".to_string(),
                    description: "List existing ESP-IDF projects in the workspace folder".to_string(),
                    required_params: vec!["operation".to_string()],
                    optional_params: vec![],
                },
                ToolOperationInfo {
                    name: "list_examples".to_string(),
                    description: "List available ESP-IDF examples".to_string(),
                    required_params: vec!["operation".to_string()],
                    optional_params: vec!["filter".to_string()],
                },
                ToolOperationInfo {
                    name: "search_examples".to_string(),
                    description: "Semantic search in ESP-IDF examples via VecDB".to_string(),
                    required_params: vec!["operation".to_string(), "query".to_string()],
                    optional_params: vec!["top_n".to_string(), "project_path".to_string()],
                },
                ToolOperationInfo {
                    name: "validate".to_string(),
                    description: "Validate project structure".to_string(),
                    required_params: vec!["operation".to_string()],
                    optional_params: vec!["project_path".to_string()],
                },
            ],
            depends_on: vec!["esp32".to_string()],
        }
    }

    fn esp32_build() -> Esp32ToolInfo {
        Esp32ToolInfo {
            name: "esp32_build".to_string(),
            display_name: "ESP32 Build".to_string(),
            description: "Build ESP32 projects. Operations: build, clean, menuconfig, reconfigure.".to_string(),
            operations: vec![
                ToolOperationInfo {
                    name: "build".to_string(),
                    description: "Compile the project".to_string(),
                    required_params: vec!["operation".to_string()],
                    optional_params: vec!["project_path".to_string(), "target".to_string()],
                },
                ToolOperationInfo {
                    name: "clean".to_string(),
                    description: "Clean build artifacts".to_string(),
                    required_params: vec!["operation".to_string()],
                    optional_params: vec!["project_path".to_string()],
                },
                ToolOperationInfo {
                    name: "menuconfig".to_string(),
                    description: "Open interactive config menu".to_string(),
                    required_params: vec!["operation".to_string()],
                    optional_params: vec!["project_path".to_string()],
                },
                ToolOperationInfo {
                    name: "reconfigure".to_string(),
                    description: "Regenerate config from sdkconfig.defaults".to_string(),
                    required_params: vec!["operation".to_string()],
                    optional_params: vec!["project_path".to_string()],
                },
            ],
            depends_on: vec!["esp32".to_string()],
        }
    }

    fn esp32_device() -> Esp32ToolInfo {
        Esp32ToolInfo {
            name: "esp32_device".to_string(),
            display_name: "ESP32 Device".to_string(),
            description: "Interact with ESP32 devices. Operations: detect, verify, flash, monitor, erase, info.".to_string(),
            operations: vec![
                ToolOperationInfo {
                    name: "detect".to_string(),
                    description: "Find connected ESP32 devices".to_string(),
                    required_params: vec!["operation".to_string()],
                    optional_params: vec![],
                },
                ToolOperationInfo {
                    name: "verify".to_string(),
                    description: "Validate device against board_id".to_string(),
                    required_params: vec!["operation".to_string()],
                    optional_params: vec!["port".to_string(), "board_id".to_string()],
                },
                ToolOperationInfo {
                    name: "flash".to_string(),
                    description: "Program firmware to device".to_string(),
                    required_params: vec!["operation".to_string()],
                    optional_params: vec!["port".to_string(), "project_path".to_string()],
                },
                ToolOperationInfo {
                    name: "monitor".to_string(),
                    description: "Capture serial/UART output".to_string(),
                    required_params: vec!["operation".to_string()],
                    optional_params: vec!["port".to_string(), "project_path".to_string(), "baud_rate".to_string(), "duration".to_string()],
                },
                ToolOperationInfo {
                    name: "erase".to_string(),
                    description: "Erase flash".to_string(),
                    required_params: vec!["operation".to_string()],
                    optional_params: vec!["port".to_string()],
                },
                ToolOperationInfo {
                    name: "info".to_string(),
                    description: "Get chip information".to_string(),
                    required_params: vec!["operation".to_string()],
                    optional_params: vec!["port".to_string()],
                },
            ],
            depends_on: vec!["esp32".to_string()],
        }
    }

    fn esp32_config() -> Esp32ToolInfo {
        Esp32ToolInfo {
            name: "esp32_config".to_string(),
            display_name: "ESP32 Config".to_string(),
            description: "Configure ESP32 project settings. Operations: sdkconfig, auto_configure, preset, partition, wifi, gpio, show.".to_string(),
            operations: vec![
                ToolOperationInfo {
                    name: "sdkconfig".to_string(),
                    description: "Modify Kconfig options in sdkconfig.defaults".to_string(),
                    required_params: vec!["operation".to_string()],
                    optional_params: vec!["key".to_string(), "value".to_string(), "project_path".to_string(), "reconfigure".to_string()],
                },
                ToolOperationInfo {
                    name: "auto_configure".to_string(),
                    description: "Auto-generate sdkconfig.defaults from connected device".to_string(),
                    required_params: vec!["operation".to_string(), "port".to_string()],
                    optional_params: vec!["project_path".to_string(), "reconfigure".to_string()],
                },
                ToolOperationInfo {
                    name: "preset".to_string(),
                    description: "Apply board presets (e.g., wifi_remote_func_ev)".to_string(),
                    required_params: vec!["operation".to_string()],
                    optional_params: vec!["preset_name".to_string(), "board_id".to_string(), "project_path".to_string()],
                },
                ToolOperationInfo {
                    name: "partition".to_string(),
                    description: "Manage partition tables".to_string(),
                    required_params: vec!["operation".to_string()],
                    optional_params: vec!["project_path".to_string()],
                },
                ToolOperationInfo {
                    name: "wifi".to_string(),
                    description: "Configure Wi-Fi settings".to_string(),
                    required_params: vec!["operation".to_string()],
                    optional_params: vec!["key".to_string(), "value".to_string(), "project_path".to_string()],
                },
                ToolOperationInfo {
                    name: "gpio".to_string(),
                    description: "Configure GPIO pins".to_string(),
                    required_params: vec!["operation".to_string()],
                    optional_params: vec!["project_path".to_string()],
                },
                ToolOperationInfo {
                    name: "show".to_string(),
                    description: "Display current configuration".to_string(),
                    required_params: vec!["operation".to_string()],
                    optional_params: vec!["project_path".to_string()],
                },
            ],
            depends_on: vec!["esp32".to_string()],
        }
    }

    fn esp32_component() -> Esp32ToolInfo {
        Esp32ToolInfo {
            name: "esp32_component".to_string(),
            display_name: "ESP32 Component".to_string(),
            description: "Manage ESP-IDF components. Operations: add, remove, list, search.".to_string(),
            operations: vec![
                ToolOperationInfo {
                    name: "add".to_string(),
                    description: "Add component dependency".to_string(),
                    required_params: vec!["operation".to_string(), "component".to_string()],
                    optional_params: vec!["project_path".to_string(), "version".to_string()],
                },
                ToolOperationInfo {
                    name: "remove".to_string(),
                    description: "Remove component".to_string(),
                    required_params: vec!["operation".to_string(), "component".to_string()],
                    optional_params: vec!["project_path".to_string()],
                },
                ToolOperationInfo {
                    name: "list".to_string(),
                    description: "List installed components".to_string(),
                    required_params: vec!["operation".to_string()],
                    optional_params: vec!["project_path".to_string()],
                },
                ToolOperationInfo {
                    name: "search".to_string(),
                    description: "Search component registry".to_string(),
                    required_params: vec!["operation".to_string()],
                    optional_params: vec!["query".to_string(), "project_path".to_string()],
                },
            ],
            depends_on: vec!["esp32".to_string()],
        }
    }

    fn esp32_analyze() -> Esp32ToolInfo {
        Esp32ToolInfo {
            name: "esp32_analyze".to_string(),
            display_name: "ESP32 Analyze".to_string(),
            description: "Analyze ESP32 code quality using AI. Operations: evaluate, check_errors, suggest_fixes.".to_string(),
            operations: vec![
                ToolOperationInfo {
                    name: "evaluate".to_string(),
                    description: "Evaluate code for correctness and ESP-IDF issues".to_string(),
                    required_params: vec!["operation".to_string(), "file_path".to_string()],
                    optional_params: vec!["focus".to_string()],
                },
                ToolOperationInfo {
                    name: "check_errors".to_string(),
                    description: "Check for common errors".to_string(),
                    required_params: vec!["operation".to_string(), "file_path".to_string()],
                    optional_params: vec!["focus".to_string()],
                },
                ToolOperationInfo {
                    name: "suggest_fixes".to_string(),
                    description: "Suggest improvements".to_string(),
                    required_params: vec!["operation".to_string(), "file_path".to_string()],
                    optional_params: vec!["focus".to_string()],
                },
            ],
            depends_on: vec!["esp32".to_string(), "thinking".to_string()],
        }
    }
}

// =============================================================================
// Event Builder & Helpers
// =============================================================================

impl Esp32ToolEvent {
    /// Create a new event for tracking.
    pub fn new(
        id: impl Into<String>,
        tool_name: impl Into<String>,
        operation: impl Into<String>,
        input_params: HashMap<String, serde_json::Value>,
    ) -> Self {
        Self {
            id: id.into(),
            tool_name: tool_name.into(),
            operation: operation.into(),
            invoked_at: SystemTime::now(),
            completed_at: None,
            status: ToolExecutionStatus::Pending,
            input_params,
            output: None,
            error: None,
            execution_duration: None,
            notes: Vec::new(),
        }
    }

    /// Create event with explicit invoked_at (for error recording).
    pub fn new_with_invoked_at(
        id: impl Into<String>,
        tool_name: impl Into<String>,
        operation: impl Into<String>,
        input_params: HashMap<String, serde_json::Value>,
        invoked_at: SystemTime,
    ) -> Self {
        Self {
            id: id.into(),
            tool_name: tool_name.into(),
            operation: operation.into(),
            invoked_at,
            completed_at: None,
            status: ToolExecutionStatus::Pending,
            input_params,
            output: None,
            error: None,
            execution_duration: None,
            notes: Vec::new(),
        }
    }

    /// Mark the event as completed with status and output.
    pub fn complete(
        &mut self,
        status: ToolExecutionStatus,
        output: Option<ToolOutputSnapshot>,
        error: Option<ErrorDetails>,
    ) {
        self.completed_at = Some(SystemTime::now());
        self.status = status;
        self.output = output;
        self.error = error;
        self.execution_duration = self
            .invoked_at
            .elapsed()
            .ok()
            .or_else(|| self.completed_at.and_then(|c| c.duration_since(self.invoked_at).ok()));
    }

    /// Add a note or log entry.
    pub fn add_note(&mut self, note: impl Into<String>) {
        self.notes.push(note.into());
    }

    /// Mark as ongoing.
    pub fn set_ongoing(&mut self) {
        self.status = ToolExecutionStatus::Ongoing;
    }
}

impl ToolExecutionStatus {
    /// Human-readable label for UI/logging.
    pub fn label(&self) -> &'static str {
        match self {
            ToolExecutionStatus::Success => "success",
            ToolExecutionStatus::PartialSuccess => "partial_success",
            ToolExecutionStatus::Failure => "failure",
            ToolExecutionStatus::Ongoing => "ongoing",
            ToolExecutionStatus::Pending => "pending",
            ToolExecutionStatus::Cached => "cached",
            ToolExecutionStatus::Skipped => "skipped",
        }
    }
}

// =============================================================================
// Integration helpers: map ESP32 ToolOutput -> Esp32ToolEvent
// =============================================================================

impl From<&esp32_output::ClassifiedError> for ErrorDetails {
    fn from(src: &esp32_output::ClassifiedError) -> Self {
        let category = match &src.category {
            esp32_output::ErrorCategory::Build(kind) => format!("Build:{}", kind),
            esp32_output::ErrorCategory::Flash(kind) => format!("Flash:{}", kind),
            esp32_output::ErrorCategory::Hardware(kind) => format!("Hardware:{}", kind),
            esp32_output::ErrorCategory::Config(kind) => format!("Config:{}", kind),
            esp32_output::ErrorCategory::Network(kind) => format!("Network:{}", kind),
        };

        Self {
            category,
            subcategory: src.subcategory.clone(),
            message: src.message.clone(),
            file: src.file.clone(),
            line: src.line,
            column: src.column,
            fix_hints: src.fix_hints.clone(),
            related_docs: src.related_docs.clone(),
        }
    }
}

impl ToolExecutionStatus {
    /// Map low-level ESP32 `ToolStatus` into progress bar status.
    pub fn from_esp32_status(status: &esp32_output::ToolStatus) -> Self {
        match status {
            esp32_output::ToolStatus::Success => ToolExecutionStatus::Success,
            esp32_output::ToolStatus::PartialSuccess => ToolExecutionStatus::PartialSuccess,
            esp32_output::ToolStatus::Failed => ToolExecutionStatus::Failure,
            esp32_output::ToolStatus::Cached => ToolExecutionStatus::Cached,
            esp32_output::ToolStatus::Skipped => ToolExecutionStatus::Skipped,
        }
    }
}

impl Esp32ToolEvent {
    /// Build a full `Esp32ToolEvent` from an ESP32 `ToolOutput`.
    ///
    /// Call this at the end of a tool execution, passing:
    /// - `id`: tool_call_id or any unique identifier
    /// - `tool_name`: e.g. "esp32_build"
    /// - `operation`: e.g. "build"
    /// - `input_params`: raw arguments passed into the tool
    /// - `invoked_at`: timestamp captured when the tool was first invoked
    pub fn from_esp32_tool_output(
        id: impl Into<String>,
        tool_name: impl Into<String>,
        operation: impl Into<String>,
        input_params: HashMap<String, serde_json::Value>,
        invoked_at: SystemTime,
        output: &esp32_output::ToolOutput,
    ) -> Self {
        let completed_at = SystemTime::now();

        let status = ToolExecutionStatus::from_esp32_status(&output.status);

        let output_snapshot = ToolOutputSnapshot {
            summary: output.summary.clone(),
            details: output.details.clone(),
            data: Some(output.data.clone()),
            action_taken: if output.action_taken.is_empty() {
                None
            } else {
                Some(output.action_taken.clone())
            },
        };

        let error_details = output.error.as_ref().map(ErrorDetails::from);

        let execution_duration = completed_at
            .duration_since(invoked_at)
            .ok()
            .or_else(|| invoked_at.elapsed().ok());

        Esp32ToolEvent {
            id: id.into(),
            tool_name: tool_name.into(),
            operation: operation.into(),
            invoked_at,
            completed_at: Some(completed_at),
            status,
            input_params,
            output: Some(output_snapshot),
            error: error_details,
            execution_duration,
            notes: Vec::new(),
        }
    }
}

// =============================================================================
// Progress Store (in-memory, keyed by chat_id)
// =============================================================================

/// Per-chat progress session with high-water-mark tracking for stage locking.
#[derive(Debug, Clone)]
struct ProgressSession {
    events: Vec<Esp32ToolEvent>,
    /// The highest stage ever reached — the bar never regresses below this.
    high_water_mark: WorkflowNode,
    /// Per-stage count of error→fix cycles (incremented when a lower-stage tool
    /// runs while last_error_stage is set).
    debug_iterations: HashMap<WorkflowNode, u32>,
    /// The stage that most recently produced an error (cleared on success at same stage).
    last_error_stage: Option<WorkflowNode>,
}

impl Default for ProgressSession {
    fn default() -> Self {
        Self {
            events: Vec::new(),
            high_water_mark: WorkflowNode::Planning,
            debug_iterations: HashMap::new(),
            last_error_stage: None,
        }
    }
}

lazy_static::lazy_static! {
    static ref PROGRESS_STORE: Arc<RwLock<HashMap<String, ProgressSession>>> =
        Arc::new(RwLock::new(HashMap::new()));
}

static PROGRESS_PERSIST_DIR: OnceLock<PathBuf> = OnceLock::new();

/// Initialize on-disk persistence location for progress sessions.
pub async fn init_progress_persistence(cache_dir: PathBuf) {
    let dir = cache_dir.join("progress").join("sessions");
    let _ = tokio::fs::create_dir_all(&dir).await;
    let _ = PROGRESS_PERSIST_DIR.set(dir);
}

fn persist_dir() -> Option<&'static PathBuf> {
    PROGRESS_PERSIST_DIR.get()
}

fn progress_session_path(chat_id: &str) -> Option<PathBuf> {
    let base = persist_dir()?;
    let safe = chat_id
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect::<String>();
    Some(base.join(format!("{safe}.json")))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedProgressSession {
    events: Vec<Esp32ToolEvent>,
    high_water_mark: WorkflowNode,
    debug_iterations: HashMap<WorkflowNode, u32>,
    last_error_stage: Option<WorkflowNode>,
}

impl From<&ProgressSession> for PersistedProgressSession {
    fn from(s: &ProgressSession) -> Self {
        Self {
            events: s.events.clone(),
            high_water_mark: s.high_water_mark,
            debug_iterations: s.debug_iterations.clone(),
            last_error_stage: s.last_error_stage,
        }
    }
}

impl From<PersistedProgressSession> for ProgressSession {
    fn from(p: PersistedProgressSession) -> Self {
        Self {
            events: p.events,
            high_water_mark: p.high_water_mark,
            debug_iterations: p.debug_iterations,
            last_error_stage: p.last_error_stage,
        }
    }
}

async fn persist_session_best_effort(chat_id: &str, session: &ProgressSession) {
    let Some(path) = progress_session_path(chat_id) else { return };
    let tmp_path = path.with_extension("json.tmp");
    let payload = PersistedProgressSession::from(session);
    let Ok(bytes) = serde_json::to_vec(&payload) else { return };

    if tokio::fs::write(&tmp_path, bytes).await.is_ok() {
        let _ = tokio::fs::rename(&tmp_path, &path).await;
    }
}

async fn load_persisted_session(chat_id: &str) -> Option<ProgressSession> {
    let path = progress_session_path(chat_id)?;
    let bytes = tokio::fs::read(&path).await.ok()?;
    let persisted: PersistedProgressSession = match serde_json::from_slice(&bytes) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("progress persistence: failed to parse {}: {}", path.display(), e);
            return None;
        }
    };

    if persisted.events.len() > 50_000 {
        tracing::warn!(
            "progress persistence: refusing to load {} events for chat_id={}",
            persisted.events.len(),
            chat_id
        );
        return None;
    }
    Some(persisted.into())
}

fn system_time_to_iso(t: SystemTime) -> String {
    let secs = t.duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default().as_secs() as i64;
    chrono::DateTime::from_timestamp(secs, 0)
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_else(|| "1970-01-01T00:00:00Z".to_string())
}

/// Record that an ESP32 tool has started. Call at the beginning of tool_execute.
pub async fn record_tool_start(
    chat_id: &str,
    tool_call_id: &str,
    tool_name: &str,
    operation: &str,
    input_params: HashMap<String, serde_json::Value>,
) {
    let mut event = Esp32ToolEvent::new(
        tool_call_id,
        tool_name,
        operation,
        input_params,
    );
    event.set_ongoing();

    let node = WorkflowNode::from_tool_operation(tool_name, operation);

    let mut store = PROGRESS_STORE.write().await;
    let session = store.entry(chat_id.to_string()).or_default();

    if node.ordinal() > session.high_water_mark.ordinal() {
        session.high_water_mark = node;
    }

    // If a higher stage had an error and agent is now running lower-stage tools,
    // that counts as a debug iteration.
    if let Some(err_stage) = session.last_error_stage {
        if node.ordinal() < err_stage.ordinal() {
            let count = session.debug_iterations.entry(err_stage).or_insert(0);
            // Only increment on the first lower-stage tool per debug cycle (when transitioning)
            let already_debugging = session.events.iter().rev().take(3).any(|e| {
                let en = WorkflowNode::from_tool_operation(&e.tool_name, &e.operation);
                en.ordinal() < err_stage.ordinal()
                    && (e.status == ToolExecutionStatus::Ongoing || e.status == ToolExecutionStatus::Success)
            });
            if !already_debugging {
                *count += 1;
            }
        }
    }

    session.events.push(event);

    let session_clone = session.clone();
    let chat_id = chat_id.to_string();
    tokio::spawn(async move {
        persist_session_best_effort(&chat_id, &session_clone).await;
    });
}

/// Record that an ESP32 tool has completed (success or failure). Call at the end of tool_execute.
pub async fn record_tool_complete(
    chat_id: &str,
    tool_call_id: &str,
    tool_name: &str,
    operation: &str,
    input_params: HashMap<String, serde_json::Value>,
    invoked_at: SystemTime,
    output: &esp32_output::ToolOutput,
) {
    let event = Esp32ToolEvent::from_esp32_tool_output(
        tool_call_id,
        tool_name,
        operation,
        input_params,
        invoked_at,
        output,
    );

    let node = WorkflowNode::from_tool_operation(tool_name, operation);
    let is_failure = matches!(output.status, esp32_output::ToolStatus::Failed | esp32_output::ToolStatus::PartialSuccess);
    let is_success = matches!(output.status, esp32_output::ToolStatus::Success | esp32_output::ToolStatus::Cached | esp32_output::ToolStatus::Skipped);

    let mut store = PROGRESS_STORE.write().await;
    let session = store.entry(chat_id.to_string()).or_default();

    if is_failure {
        session.last_error_stage = Some(node);
    } else if is_success {
        // Clear error stage when the same stage OR any higher stage succeeds
        // (the agent moved on and recovered, even if it never retried the exact failed stage).
        if let Some(err_stage) = session.last_error_stage {
            if node.ordinal() >= err_stage.ordinal() {
                session.last_error_stage = None;
            }
        }
    }

    if let Some(last) = session.events.iter_mut().rev().find(|e| e.id == tool_call_id) {
        *last = event;
    } else {
        session.events.push(event);
    }

    let session_clone = session.clone();
    let chat_id = chat_id.to_string();
    tokio::spawn(async move {
        persist_session_best_effort(&chat_id, &session_clone).await;
    });
}

/// Record successful completion for tools that do not emit ESP32 `ToolOutput`
/// (generic agent tools). Call after `record_tool_start` in the same execution.
pub async fn record_generic_tool_success(
    chat_id: &str,
    tool_call_id: &str,
    tool_name: &str,
    operation: &str,
    input_params: HashMap<String, serde_json::Value>,
    invoked_at: SystemTime,
    summary: String,
) {
    let mut event = Esp32ToolEvent::new_with_invoked_at(
        tool_call_id,
        tool_name,
        operation,
        input_params,
        invoked_at,
    );
    event.complete(
        ToolExecutionStatus::Success,
        Some(ToolOutputSnapshot {
            summary,
            details: None,
            data: None,
            action_taken: None,
        }),
        None,
    );

    let node = WorkflowNode::from_tool_operation(tool_name, operation);

    let mut store = PROGRESS_STORE.write().await;
    let session = store.entry(chat_id.to_string()).or_default();

    // Clear error stage when this stage or any higher stage succeeds.
    if let Some(err_stage) = session.last_error_stage {
        if node.ordinal() >= err_stage.ordinal() {
            session.last_error_stage = None;
        }
    }

    if let Some(last) = session.events.iter_mut().rev().find(|e| e.id == tool_call_id) {
        *last = event;
    } else {
        session.events.push(event);
    }

    let session_clone = session.clone();
    let chat_id = chat_id.to_string();
    tokio::spawn(async move {
        persist_session_best_effort(&chat_id, &session_clone).await;
    });
}

/// Tools that already call `record_tool_start` / `record_tool_complete` internally.
pub fn tool_manages_own_progress(tool_name: &str) -> bool {
    matches!(
        tool_name,
        "esp32_build" | "esp32_device" | "esp32_config" | "esp32_component" | "esp32_project"
    )
}

/// Record that an ESP32 tool failed with an error string (before returning Err).
pub async fn record_tool_error(
    chat_id: &str,
    tool_call_id: &str,
    tool_name: &str,
    operation: &str,
    input_params: HashMap<String, serde_json::Value>,
    invoked_at: SystemTime,
    error_message: &str,
) {
    let mut event = Esp32ToolEvent::new_with_invoked_at(
        tool_call_id,
        tool_name,
        operation,
        input_params,
        invoked_at,
    );
    event.complete(
        ToolExecutionStatus::Failure,
        Some(ToolOutputSnapshot {
            summary: error_message.to_string(),
            details: None,
            data: None,
            action_taken: None,
        }),
        Some(ErrorDetails {
            category: "Tool".to_string(),
            subcategory: "execution".to_string(),
            message: error_message.to_string(),
            file: None,
            line: None,
            column: None,
            fix_hints: vec![],
            related_docs: vec![],
        }),
    );

    let node = WorkflowNode::from_tool_operation(tool_name, operation);

    let mut store = PROGRESS_STORE.write().await;
    let session = store.entry(chat_id.to_string()).or_default();

    session.last_error_stage = Some(node);

    if let Some(last) = session.events.iter_mut().rev().find(|e| e.id == tool_call_id) {
        *last = event;
    } else {
        session.events.push(event);
    }

    let session_clone = session.clone();
    let chat_id = chat_id.to_string();
    tokio::spawn(async move {
        persist_session_best_effort(&chat_id, &session_clone).await;
    });
}

/// Get progress for a chat_id. Returns a DTO suitable for JSON API.
///
/// `current_node` follows the highest stage with live or completed tool activity.
/// When a stage fails and the agent runs lower-stage tools to fix it,
/// `current_node` stays locked on the failed stage and `is_debugging` is set.
pub async fn get_progress(chat_id: &str) -> Option<ProgressSessionDto> {
    {
        let store = PROGRESS_STORE.read().await;
        if store.get(chat_id).is_none() {
            drop(store);
            if let Some(loaded) = load_persisted_session(chat_id).await {
                let mut store_w = PROGRESS_STORE.write().await;
                store_w.entry(chat_id.to_string()).or_insert(loaded);
            }
        }
    }

    let store = PROGRESS_STORE.read().await;
    let session = store.get(chat_id)?;
    if session.events.is_empty() {
        return None;
    }

    let order: Vec<WorkflowNode> = vec![
        WorkflowNode::Planning,
        WorkflowNode::Generation,
        WorkflowNode::Compiling,
        WorkflowNode::Flashing,
        WorkflowNode::Monitoring,
    ];

    let mut completed_nodes: Vec<WorkflowNode> = vec![];
    let mut has_error = false;
    let mut has_active = false;
    let mut activity_label = "Idle".to_string();

    let event_dtos: Vec<ProgressEventDto> = session.events
        .iter()
        .map(|e| {
            let node = WorkflowNode::from_tool_operation(&e.tool_name, &e.operation);
            if e.status == ToolExecutionStatus::Failure || e.status == ToolExecutionStatus::PartialSuccess {
                has_error = true;
            }
            if e.status == ToolExecutionStatus::Ongoing || e.status == ToolExecutionStatus::Pending {
                has_active = true;
                activity_label = format!("{} {}", e.tool_name, e.operation);
            }
            if e.status == ToolExecutionStatus::Success
                || e.status == ToolExecutionStatus::Cached
                || e.status == ToolExecutionStatus::Skipped
            {
                if !completed_nodes.contains(&node) {
                    completed_nodes.push(node);
                }
            }

            let summary = e.output.as_ref().map(|o| o.summary.clone());
            let duration_secs = e.execution_duration.map(|d| d.as_secs_f64());

            ProgressEventDto {
                id: e.id.clone(),
                tool_name: e.tool_name.clone(),
                operation: e.operation.clone(),
                node,
                status: e.status,
                invoked_at_iso: system_time_to_iso(e.invoked_at),
                completed_at_iso: e.completed_at.map(system_time_to_iso),
                summary,
                has_error: e.error.is_some(),
                execution_duration_secs: duration_secs,
            }
        })
        .collect();

    // Detect debugging state: a stage had an error and the agent is now running
    // tools at a lower stage to fix it.
    let is_debugging = session.last_error_stage.is_some() && has_active && {
        let err_ord = session.last_error_stage.unwrap().ordinal();
        event_dtos.iter().any(|e| {
            (e.status == ToolExecutionStatus::Ongoing || e.status == ToolExecutionStatus::Pending)
                && e.node.ordinal() < err_ord
        })
    };

    // current_node: node of the most recent actively-running tool (so the spinner
    // always follows what is happening RIGHT NOW, not the historical maximum).
    // Fallback to most recent completed tool; final fallback Planning.
    // During debugging lock to the failed stage so the user sees which stage is broken.
    let current_node = if is_debugging {
        session.last_error_stage.unwrap_or(session.high_water_mark)
    } else {
        let active_node = session.events.iter().rev()
            .find(|e| e.status == ToolExecutionStatus::Ongoing || e.status == ToolExecutionStatus::Pending)
            .map(|e| WorkflowNode::from_tool_operation(&e.tool_name, &e.operation));

        let completed_node = session.events.iter().rev()
            .find(|e| matches!(
                e.status,
                ToolExecutionStatus::Success
                    | ToolExecutionStatus::Cached
                    | ToolExecutionStatus::Skipped
            ))
            .map(|e| WorkflowNode::from_tool_operation(&e.tool_name, &e.operation));

        active_node.or(completed_node).unwrap_or(WorkflowNode::Planning)
    };

    let debug_iteration = session.last_error_stage
        .and_then(|s| session.debug_iterations.get(&s).copied())
        .unwrap_or(0);

    let completed_count = order.iter().filter(|n| completed_nodes.contains(n)).count();
    let overall_percentage = if order.is_empty() {
        0
    } else {
        ((completed_count as f64 / order.len() as f64) * 100.0).min(100.0) as u8
    };

    Some(ProgressSessionDto {
        chat_id: chat_id.to_string(),
        events: event_dtos,
        current_node,
        overall_percentage,
        activity_label,
        has_active_run: has_active,
        has_error,
        is_debugging,
        debug_iteration,
        esp32_project_path: project_path_from_events(&session.events)
            .map(|p| p.to_string_lossy().to_string()),
    })
}

/// ESP-IDF project path from the latest relevant `esp32_project` / `esp32_build` tool event.
pub async fn esp32_project_path_for_chat(chat_id: &str) -> Option<PathBuf> {
    {
        let store = PROGRESS_STORE.read().await;
        if store.get(chat_id).is_none() {
            drop(store);
            if let Some(loaded) = load_persisted_session(chat_id).await {
                let mut store_w = PROGRESS_STORE.write().await;
                store_w.entry(chat_id.to_string()).or_insert(loaded);
            }
        }
    }
    let store = PROGRESS_STORE.read().await;
    let session = store.get(chat_id)?;
    project_path_from_events(&session.events)
}

fn project_path_from_events(events: &[Esp32ToolEvent]) -> Option<PathBuf> {
    for e in events.iter().rev() {
        if let Some(p) = project_path_from_tool_event(e) {
            let pb = PathBuf::from(p);
            if pb.is_dir() {
                return Some(pb);
            }
        }
    }
    None
}

fn project_path_from_tool_event(e: &Esp32ToolEvent) -> Option<&str> {
    if e.tool_name.starts_with("esp32_") {
        if let Some(s) = e.input_params.get("project_path").and_then(|v| v.as_str()) {
            if !s.is_empty() {
                return Some(s);
            }
        }
        if let Some(ref out) = e.output {
            if let Some(ref data) = out.data {
                if let Some(s) = data.get("project_path").and_then(|v| v.as_str()) {
                    if !s.is_empty() {
                        return Some(s);
                    }
                }
            }
        }
    }
    None
}

pub async fn is_registered_project_path(path: &std::path::Path) -> bool {
    let store = PROGRESS_STORE.read().await;
    for session in store.values() {
        if let Some(p) = project_path_from_events(&session.events) {
            if p == path {
                return true;
            }
        }
    }
    false
}
