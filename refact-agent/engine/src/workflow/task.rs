//! Task definitions for the Workflow Engine

use std::collections::HashMap;
use std::time::Instant;
use serde::{Serialize, Deserialize};
use serde_json::Value;

/// Status of a workflow task
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    /// Task is waiting to be executed
    Pending,
    /// Task is currently being executed
    InProgress,
    /// Task completed successfully
    Completed,
    /// Task was skipped by user or system
    Skipped,
    /// Task failed with an error
    Failed,
    /// Task is blocked by dependencies
    Blocked,
    /// Task was cancelled
    Cancelled,
}

impl TaskStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(self, TaskStatus::Completed | TaskStatus::Skipped | TaskStatus::Failed | TaskStatus::Cancelled)
    }
    
    pub fn is_active(&self) -> bool {
        matches!(self, TaskStatus::InProgress)
    }
    
    pub fn can_transition_to(&self, new_status: &TaskStatus) -> bool {
        match self {
            TaskStatus::Pending => true,
            TaskStatus::InProgress => !matches!(new_status, TaskStatus::Pending),
            TaskStatus::Blocked => matches!(new_status, TaskStatus::Pending | TaskStatus::InProgress | TaskStatus::Skipped | TaskStatus::Cancelled),
            _ => false, // Terminal states can't transition
        }
    }
    
    pub fn icon(&self) -> &'static str {
        match self {
            TaskStatus::Pending => "⬜",
            TaskStatus::InProgress => "🔄",
            TaskStatus::Completed => "✅",
            TaskStatus::Skipped => "⏭️",
            TaskStatus::Failed => "❌",
            TaskStatus::Blocked => "🚫",
            TaskStatus::Cancelled => "🚫",
        }
    }
    
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskStatus::Pending => "pending",
            TaskStatus::InProgress => "in_progress",
            TaskStatus::Completed => "completed",
            TaskStatus::Skipped => "skipped",
            TaskStatus::Failed => "failed",
            TaskStatus::Blocked => "blocked",
            TaskStatus::Cancelled => "cancelled",
        }
    }
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Priority level for tasks
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskPriority {
    Critical,
    High,
    Normal,
    Low,
}

impl Default for TaskPriority {
    fn default() -> Self {
        TaskPriority::Normal
    }
}

impl TaskPriority {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskPriority::Critical => "critical",
            TaskPriority::High => "high",
            TaskPriority::Normal => "normal",
            TaskPriority::Low => "low",
        }
    }
}

/// Result of a completed task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub success: bool,
    pub summary: String,
    pub details: Option<String>,
    pub data: Option<Value>,
    pub error_message: Option<String>,
    pub error_category: Option<String>,
    pub suggestions: Vec<String>,
}

impl TaskResult {
    pub fn success(summary: impl Into<String>) -> Self {
        Self {
            success: true,
            summary: summary.into(),
            details: None,
            data: None,
            error_message: None,
            error_category: None,
            suggestions: vec![],
        }
    }
    
    pub fn success_with_data(summary: impl Into<String>, data: Value) -> Self {
        Self {
            success: true,
            summary: summary.into(),
            details: None,
            data: Some(data),
            error_message: None,
            error_category: None,
            suggestions: vec![],
        }
    }
    
    pub fn failure(summary: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            success: false,
            summary: summary.into(),
            details: None,
            data: None,
            error_message: Some(error.into()),
            error_category: None,
            suggestions: vec![],
        }
    }
    
    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }
    
    pub fn with_suggestions(mut self, suggestions: Vec<String>) -> Self {
        self.suggestions = suggestions;
        self
    }
}

/// A single task in a workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTask {
    /// Unique identifier for this task
    pub id: String,
    
    /// Human-readable description of the task
    pub description: String,
    
    /// Current status of the task
    pub status: TaskStatus,
    
    /// Priority level
    pub priority: TaskPriority,
    
    /// Tool to call to execute this task (optional)
    pub tool_to_call: Option<String>,
    
    /// Arguments to pass to the tool
    pub tool_args: HashMap<String, Value>,
    
    /// Result after task completion
    pub result: Option<TaskResult>,
    
    /// IDs of tasks this task depends on
    pub dependencies: Vec<String>,
    
    /// Whether this task can be skipped by the user
    pub skippable: bool,
    
    /// Whether this task can be retried after failure
    pub retryable: bool,
    
    /// Number of retry attempts made
    pub retry_count: u32,
    
    /// Maximum retry attempts allowed
    pub max_retries: u32,
    
    /// Timestamp when task started (not serialized directly)
    #[serde(skip)]
    pub started_at: Option<Instant>,
    
    /// Timestamp when task completed (not serialized directly)
    #[serde(skip)]
    pub completed_at: Option<Instant>,
    
    /// ISO timestamp strings for serialization
    pub started_at_iso: Option<String>,
    pub completed_at_iso: Option<String>,
    
    /// Duration in milliseconds
    pub duration_ms: Option<u64>,
    
    /// Optional metadata
    pub metadata: HashMap<String, Value>,
}

impl WorkflowTask {
    pub fn new(id: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            description: description.into(),
            status: TaskStatus::Pending,
            priority: TaskPriority::Normal,
            tool_to_call: None,
            tool_args: HashMap::new(),
            result: None,
            dependencies: vec![],
            skippable: true,
            retryable: true,
            retry_count: 0,
            max_retries: 3,
            started_at: None,
            completed_at: None,
            started_at_iso: None,
            completed_at_iso: None,
            duration_ms: None,
            metadata: HashMap::new(),
        }
    }
    
    pub fn with_tool(mut self, tool_name: impl Into<String>) -> Self {
        self.tool_to_call = Some(tool_name.into());
        self
    }
    
    pub fn with_tool_args(mut self, args: HashMap<String, Value>) -> Self {
        self.tool_args = args;
        self
    }
    
    pub fn with_priority(mut self, priority: TaskPriority) -> Self {
        self.priority = priority;
        self
    }
    
    pub fn with_dependencies(mut self, deps: Vec<String>) -> Self {
        self.dependencies = deps;
        self
    }
    
    pub fn non_skippable(mut self) -> Self {
        self.skippable = false;
        self
    }
    
    pub fn non_retryable(mut self) -> Self {
        self.retryable = false;
        self
    }
    
    /// Mark task as started
    pub fn start(&mut self) {
        self.status = TaskStatus::InProgress;
        self.started_at = Some(Instant::now());
        self.started_at_iso = Some(chrono::Utc::now().to_rfc3339());
    }
    
    /// Mark task as completed with a result
    pub fn complete(&mut self, result: TaskResult) {
        self.status = if result.success {
            TaskStatus::Completed
        } else {
            TaskStatus::Failed
        };
        self.completed_at = Some(Instant::now());
        self.completed_at_iso = Some(chrono::Utc::now().to_rfc3339());
        
        if let Some(start) = self.started_at {
            self.duration_ms = Some(start.elapsed().as_millis() as u64);
        }
        
        self.result = Some(result);
    }
    
    /// Mark task as skipped
    pub fn skip(&mut self, reason: impl Into<String>) {
        self.status = TaskStatus::Skipped;
        self.completed_at = Some(Instant::now());
        self.completed_at_iso = Some(chrono::Utc::now().to_rfc3339());
        self.result = Some(TaskResult {
            success: true,
            summary: format!("Skipped: {}", reason.into()),
            details: None,
            data: None,
            error_message: None,
            error_category: None,
            suggestions: vec![],
        });
    }
    
    /// Check if task can be executed (all dependencies met)
    pub fn can_execute(&self, completed_tasks: &[String]) -> bool {
        self.dependencies.iter().all(|dep| completed_tasks.contains(dep))
    }
    
    /// Get a summary line for this task (minimal tokens)
    pub fn summary_line(&self) -> String {
        format!("{} {} {}", 
            self.status.icon(),
            self.description,
            if let Some(result) = &self.result {
                if result.success {
                    result.summary.clone()
                } else {
                    format!("({})", result.error_message.as_deref().unwrap_or("failed"))
                }
            } else {
                String::new()
            }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_task_creation() {
        let task = WorkflowTask::new("task_1", "Build project")
            .with_tool("c2000_build")
            .with_priority(TaskPriority::High);
        
        assert_eq!(task.id, "task_1");
        assert_eq!(task.status, TaskStatus::Pending);
        assert_eq!(task.tool_to_call, Some("c2000_build".to_string()));
    }
    
    #[test]
    fn test_task_lifecycle() {
        let mut task = WorkflowTask::new("task_1", "Build project");
        
        assert_eq!(task.status, TaskStatus::Pending);
        
        task.start();
        assert_eq!(task.status, TaskStatus::InProgress);
        assert!(task.started_at.is_some());
        
        task.complete(TaskResult::success("Built successfully"));
        assert_eq!(task.status, TaskStatus::Completed);
        assert!(task.completed_at.is_some());
        assert!(task.duration_ms.is_some());
    }
    
    #[test]
    fn test_task_skip() {
        let mut task = WorkflowTask::new("task_1", "Optional step");
        task.skip("User requested skip");
        
        assert_eq!(task.status, TaskStatus::Skipped);
        assert!(task.result.is_some());
    }
}

