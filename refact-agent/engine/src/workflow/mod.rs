//! Core Workflow Engine for Task Management
//! 
//! This module provides a device-agnostic workflow management system that enables:
//! - Task planning and tracking across tool calls
//! - Progress visualization for GUI integration
//! - Workflow state persistence across sessions
//! - User intervention (pause, skip, reorder tasks)
//!
//! Designed to work with any device tools (C2000, ESP32, STM32, etc.)

pub mod task;
pub mod context;
pub mod engine;

pub use task::{WorkflowTask, TaskStatus, TaskResult, TaskPriority};
pub use context::{WorkflowContext, InterventionType};
pub use engine::{WorkflowEngine, ActiveWorkflow, CompletedWorkflow, WorkflowType, WorkflowEvent};

use std::sync::Arc;
use tokio::sync::RwLock as ARwLock;
use serde::{Serialize, Deserialize};

/// Shared workflow engine type for use across the application
pub type SharedWorkflowEngine = Arc<ARwLock<WorkflowEngine>>;

/// Create a new shared workflow engine instance
pub fn create_workflow_engine() -> SharedWorkflowEngine {
    Arc::new(ARwLock::new(WorkflowEngine::new()))
}

/// Workflow state snapshot for GUI/API consumption
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowSnapshot {
    pub has_active_workflow: bool,
    pub workflow_id: Option<String>,
    pub workflow_name: Option<String>,
    pub workflow_type: Option<String>,
    pub device_context: Option<String>,
    pub progress: WorkflowProgress,
    pub tasks: Vec<TaskSnapshot>,
    pub can_pause: bool,
    pub can_resume: bool,
    pub is_paused: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowProgress {
    pub total: usize,
    pub completed: usize,
    pub in_progress: usize,
    pub pending: usize,
    pub skipped: usize,
    pub failed: usize,
    pub percentage: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSnapshot {
    pub id: String,
    pub description: String,
    pub status: String,
    pub tool_to_call: Option<String>,
    pub result_summary: Option<String>,
    pub error_message: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub duration_ms: Option<u64>,
    pub can_skip: bool,
    pub can_retry: bool,
    pub dependencies: Vec<String>,
    pub priority: String,
}

impl Default for WorkflowSnapshot {
    fn default() -> Self {
        Self {
            has_active_workflow: false,
            workflow_id: None,
            workflow_name: None,
            workflow_type: None,
            device_context: None,
            progress: WorkflowProgress {
                total: 0,
                completed: 0,
                in_progress: 0,
                pending: 0,
                skipped: 0,
                failed: 0,
                percentage: 0.0,
            },
            tasks: vec![],
            can_pause: false,
            can_resume: false,
            is_paused: false,
        }
    }
}

