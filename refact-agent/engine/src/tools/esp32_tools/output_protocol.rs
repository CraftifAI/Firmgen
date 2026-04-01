use std::collections::HashMap;
use serde::Serialize;
use super::session_state::StateDelta;

/// Every tool returns this structured format
#[derive(Serialize, Debug, Clone)]
pub struct ToolOutput {
    // Core result (always present)
    pub status: ToolStatus,
    pub action_taken: String,
    
    // Structured data (LLM can query specific fields)
    pub data: serde_json::Value,
    
    // Context-efficient summary
    /// Always shown, max 50 tokens
    pub summary: String,
    /// Only if needed, max 200 tokens
    pub details: Option<String>,
    
    // State changes (not serialized - internal use only)
    #[serde(skip)]
    pub state_delta: StateDelta,
    
    // Guidance for next steps
    pub suggested_actions: Vec<SuggestedAction>,
    
    // Error info (if any)
    pub error: Option<ClassifiedError>,
}

#[derive(Serialize, Debug, Clone)]
pub enum ToolStatus {
    Success,
    PartialSuccess,
    Failed,
    Cached,
    Skipped,
}

#[derive(Serialize, Debug, Clone)]
pub struct SuggestedAction {
    pub action: String,
    pub reason: String,
    pub parameters: HashMap<String, String>,
    pub priority: ActionPriority,
}

#[derive(Serialize, Debug, Clone)]
pub enum ActionPriority {
    High,
    Medium,
    Low,
}

#[derive(Serialize, Debug, Clone)]
pub struct ClassifiedError {
    pub category: ErrorCategory,
    pub subcategory: String,
    pub severity: ErrorSeverity,
    pub message: String,
    pub file: Option<String>,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub fix_hints: Vec<String>,
    pub related_docs: Vec<String>,
    pub search_keywords: Vec<String>,
    // Cursor-style structured diagnostic fields
    /// Build phase: "build", "link", "cmake", "flash"
    pub phase: String,
    /// Toolchain binary that emitted the diagnostic, e.g. "xtensa-esp32s3-elf-gcc"
    pub tool: String,
    /// Machine-readable error kind: "missing_header", "undefined_reference", etc.
    pub kind: String,
    /// Human-readable root-cause explanation
    pub likely_cause: String,
    /// Optional structured patch suggestion
    pub suggested_patch: Option<SuggestedPatch>,
}

/// A structured patch suggestion that tells the LLM exactly what file to edit
#[derive(Serialize, Debug, Clone)]
pub struct SuggestedPatch {
    /// Relative path to the file to edit, e.g. "main/CMakeLists.txt"
    pub file: String,
    /// Human-readable action description, e.g. "Add esp_wifi to REQUIRES"
    pub action: String,
    /// Optional: text to search for in the target file
    pub search_text: Option<String>,
    /// Optional: replacement text
    pub replace_text: Option<String>,
}

#[derive(Serialize, Debug, Clone)]
pub enum ErrorCategory {
    Build(String),
    Flash(String),
    Hardware(String),
    Config(String),
    Network(String),
}

#[derive(Serialize, Debug, Clone)]
pub enum ErrorSeverity {
    Critical,
    Error,
    Warning,
    Info,
}

impl ToolOutput {
    pub fn success(summary: String, data: serde_json::Value) -> Self {
        Self {
            status: ToolStatus::Success,
            action_taken: String::new(),
            data,
            summary,
            details: None,
            state_delta: StateDelta::none(),
            suggested_actions: vec![],
            error: None,
        }
    }

    pub fn cached(summary: String) -> Self {
        Self {
            status: ToolStatus::Cached,
            action_taken: "cached".to_string(),
            data: serde_json::json!({}),
            summary,
            details: None,
            state_delta: StateDelta::none(),
            suggested_actions: vec![],
            error: None,
        }
    }

    pub fn failed(error: ClassifiedError) -> Self {
        Self {
            status: ToolStatus::Failed,
            action_taken: String::new(),
            data: serde_json::json!({}),
            summary: format!("Failed: {}", error.message),
            details: None,
            state_delta: StateDelta::none(),
            suggested_actions: vec![],
            error: Some(error),
        }
    }

    /// Convert to LLM-optimized string representation
    pub fn to_llm_context(&self) -> String {
        // Minimal format: status + summary + details (if available)
        let status_str = match self.status {
            ToolStatus::Success => "✓",
            ToolStatus::PartialSuccess => "⚠",
            ToolStatus::Failed => "✗",
            ToolStatus::Cached => "⊘",
            ToolStatus::Skipped => "⊘",
        };
        
        let mut result = format!("{} {}", status_str, self.summary);

        // Include details first if present — it contains the full Cursor-style diagnostic block
        // (multi-error format_error_diagnostics output from failed builds)
        if let Some(ref details) = self.details {
            result.push_str(&format!("\n{}", details));
        } else if let Some(ref error) = self.error {
            // No details set (e.g., flash errors, config errors) — render single error block
            result.push_str(&format_error_diagnostic(error, 1, 1));
        }
        
        // Include suggested actions so LLM knows what to do next
        if !self.suggested_actions.is_empty() {
            result.push_str("\nSuggested next steps:");
            for action in &self.suggested_actions {
                result.push_str(&format!("\n- {} ({})", action.action, action.reason));
            }
        }
        
        result
    }
}

impl Default for ToolOutput {
    fn default() -> Self {
        Self {
            status: ToolStatus::Success,
            action_taken: String::new(),
            data: serde_json::json!({}),
            summary: String::new(),
            details: None,
            state_delta: StateDelta::none(),
            suggested_actions: vec![],
            error: None,
        }
    }
}

/// Format a single ClassifiedError as a Cursor-style diagnostic block
pub fn format_error_diagnostic(error: &ClassifiedError, index: usize, total: usize) -> String {
    let severity_str = match error.severity {
        ErrorSeverity::Critical => "fatal",
        ErrorSeverity::Error => "error",
        ErrorSeverity::Warning => "warning",
        ErrorSeverity::Info => "info",
    };

    let mut block = format!("\n── Error {}/{} ({}) ──", index, total, severity_str);

    // File location
    if let Some(ref file) = error.file {
        let mut loc = file.clone();
        if let Some(line) = error.line {
            loc.push_str(&format!(":{}", line));
            if let Some(col) = error.column {
                loc.push_str(&format!(":{}", col));
            }
        }
        block.push_str(&format!("\nFile: {}", loc));
    }

    // Kind
    if !error.kind.is_empty() {
        block.push_str(&format!("\nKind: {}", error.kind));
    }

    // Message
    block.push_str(&format!("\nMessage: {}", error.message));

    // Likely cause
    if !error.likely_cause.is_empty() {
        block.push_str(&format!("\nCause: {}", error.likely_cause));
    }

    // Suggested patch
    if let Some(ref patch) = error.suggested_patch {
        block.push_str(&format!("\nFix: {} in {}", patch.action, patch.file));
    } else if !error.fix_hints.is_empty() {
        block.push_str(&format!("\nFix: {}", error.fix_hints[0]));
    }

    block
}

/// Format multiple errors as a Cursor-style diagnostic block (capped at max_errors)
pub fn format_error_diagnostics(errors: &[ClassifiedError], max_errors: usize) -> String {
    let total = errors.len();
    let show = errors.iter().take(max_errors);
    let mut result = String::new();
    for (i, error) in show.enumerate() {
        result.push_str(&format_error_diagnostic(error, i + 1, total));
    }
    if total > max_errors {
        result.push_str(&format!("\n... and {} more error(s)", total - max_errors));
    }
    result
}

