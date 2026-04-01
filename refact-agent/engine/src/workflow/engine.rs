//! Workflow Engine - Core orchestration for task management

use std::time::Instant;
use serde::{Serialize, Deserialize};
use uuid::Uuid;

use super::task::{WorkflowTask, TaskStatus, TaskResult};
use super::context::{WorkflowContext, InterventionType};
use super::{WorkflowSnapshot, WorkflowProgress, TaskSnapshot};

/// Types of workflows (extensible for different devices/use cases)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WorkflowType {
    /// Generic workflow with custom name
    Generic { name: String },
    
    /// Build and deploy workflow
    BuildAndDeploy {
        project_name: String,
        target_device: Option<String>,
    },
    
    /// Debug/troubleshooting workflow
    DebugSession {
        issue_description: String,
    },
    
    /// Configuration/setup workflow
    Setup {
        component: String,
    },
    
    /// Custom workflow from external source
    Custom {
        name: String,
        source: String,
    },
}

impl WorkflowType {
    pub fn name(&self) -> String {
        match self {
            WorkflowType::Generic { name } => name.clone(),
            WorkflowType::BuildAndDeploy { project_name, .. } => {
                format!("Build & Deploy: {}", project_name)
            }
            WorkflowType::DebugSession { .. } => "Debug Session".to_string(),
            WorkflowType::Setup { component } => format!("Setup: {}", component),
            WorkflowType::Custom { name, .. } => name.clone(),
        }
    }
    
    pub fn type_string(&self) -> &'static str {
        match self {
            WorkflowType::Generic { .. } => "generic",
            WorkflowType::BuildAndDeploy { .. } => "build_and_deploy",
            WorkflowType::DebugSession { .. } => "debug_session",
            WorkflowType::Setup { .. } => "setup",
            WorkflowType::Custom { .. } => "custom",
        }
    }
}

/// An active workflow being executed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveWorkflow {
    /// Unique identifier for this workflow
    pub id: String,
    
    /// Type/category of the workflow
    pub workflow_type: WorkflowType,
    
    /// Device context (e.g., "c2000", "esp32", or None for generic)
    pub device_context: Option<String>,
    
    /// All tasks in this workflow
    pub tasks: Vec<WorkflowTask>,
    
    /// Index of the current task being executed
    pub current_task_idx: Option<usize>,
    
    /// Context accumulated during execution
    pub context: WorkflowContext,
    
    /// Whether the workflow is paused
    pub is_paused: bool,
    
    /// ISO timestamp when workflow was created
    pub created_at: String,
    
    /// Instant when workflow was created (not serialized)
    #[serde(skip)]
    pub created_instant: Option<Instant>,
}

impl ActiveWorkflow {
    pub fn new(workflow_type: WorkflowType) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            workflow_type,
            device_context: None,
            tasks: vec![],
            current_task_idx: None,
            context: WorkflowContext::new(),
            is_paused: false,
            created_at: chrono::Utc::now().to_rfc3339(),
            created_instant: Some(Instant::now()),
        }
    }
    
    pub fn with_device_context(mut self, device: impl Into<String>) -> Self {
        self.device_context = Some(device.into());
        self
    }
    
    pub fn with_tasks(mut self, tasks: Vec<WorkflowTask>) -> Self {
        self.tasks = tasks;
        self
    }
    
    /// Add a task to the workflow
    pub fn add_task(&mut self, task: WorkflowTask) {
        self.tasks.push(task);
    }
    
    /// Get current task (if any)
    pub fn current_task(&self) -> Option<&WorkflowTask> {
        self.current_task_idx.and_then(|idx| self.tasks.get(idx))
    }
    
    /// Get current task mutably
    pub fn current_task_mut(&mut self) -> Option<&mut WorkflowTask> {
        self.current_task_idx.and_then(|idx| self.tasks.get_mut(idx))
    }
    
    /// Get task by ID
    pub fn get_task(&self, task_id: &str) -> Option<&WorkflowTask> {
        self.tasks.iter().find(|t| t.id == task_id)
    }
    
    /// Get task by ID mutably
    pub fn get_task_mut(&mut self, task_id: &str) -> Option<&mut WorkflowTask> {
        self.tasks.iter_mut().find(|t| t.id == task_id)
    }
    
    /// Get all completed task IDs
    pub fn completed_task_ids(&self) -> Vec<String> {
        self.tasks.iter()
            .filter(|t| t.status == TaskStatus::Completed)
            .map(|t| t.id.clone())
            .collect()
    }
    
    /// Find the next task to execute
    pub fn find_next_task(&self) -> Option<usize> {
        let completed = self.completed_task_ids();
        self.tasks.iter()
            .position(|t| {
                t.status == TaskStatus::Pending && t.can_execute(&completed)
            })
    }
    
    /// Start the next available task
    pub fn start_next_task(&mut self) -> Option<&WorkflowTask> {
        if self.is_paused {
            return None;
        }
        
        if let Some(idx) = self.find_next_task() {
            self.tasks[idx].start();
            self.current_task_idx = Some(idx);
            Some(&self.tasks[idx])
        } else {
            None
        }
    }
    
    /// Complete the current task
    pub fn complete_current_task(&mut self, result: TaskResult) {
        if let Some(task) = self.current_task_mut() {
            task.complete(result);
        }
        self.current_task_idx = None;
    }
    
    /// Mark a specific task as completed
    pub fn complete_task(&mut self, task_id: &str, result: TaskResult) {
        if let Some(task) = self.get_task_mut(task_id) {
            task.complete(result);
            if self.current_task_idx.map(|idx| self.tasks.get(idx).map(|t| t.id == task_id)).flatten().unwrap_or(false) {
                self.current_task_idx = None;
            }
        }
    }
    
    /// Start a specific task
    pub fn start_task(&mut self, task_id: &str) -> Result<(), String> {
        if self.is_paused {
            return Err("Workflow is paused".to_string());
        }
        
        let task_idx = self.tasks.iter().position(|t| t.id == task_id)
            .ok_or_else(|| format!("Task '{}' not found", task_id))?;
        
        let completed = self.completed_task_ids();
        if !self.tasks[task_idx].can_execute(&completed) {
            return Err("Task dependencies not met".to_string());
        }
        
        self.tasks[task_idx].start();
        self.current_task_idx = Some(task_idx);
        Ok(())
    }
    
    /// Skip a task
    pub fn skip_task(&mut self, task_id: &str, reason: impl Into<String>) -> Result<(), String> {
        let task = self.get_task_mut(task_id)
            .ok_or_else(|| format!("Task '{}' not found", task_id))?;
        
        if !task.skippable {
            return Err("Task is not skippable".to_string());
        }
        
        let reason_str = reason.into();
        task.skip(&reason_str);
        self.context.record_intervention(InterventionType::Skip, Some(task_id.to_string()), Some(reason_str));
        Ok(())
    }
    
    /// Pause the workflow
    pub fn pause(&mut self) {
        self.is_paused = true;
        self.context.record_intervention(InterventionType::Pause, None, None);
    }
    
    /// Resume the workflow
    pub fn resume(&mut self) {
        self.is_paused = false;
        self.context.record_intervention(InterventionType::Resume, None, None);
    }
    
    /// Check if workflow is complete
    pub fn is_complete(&self) -> bool {
        self.tasks.iter().all(|t| t.status.is_terminal())
    }
    
    /// Check if workflow has any failures
    pub fn has_failures(&self) -> bool {
        self.tasks.iter().any(|t| t.status == TaskStatus::Failed)
    }
    
    /// Get workflow progress
    pub fn get_progress(&self) -> WorkflowProgress {
        let total = self.tasks.len();
        let completed = self.tasks.iter().filter(|t| t.status == TaskStatus::Completed).count();
        let in_progress = self.tasks.iter().filter(|t| t.status == TaskStatus::InProgress).count();
        let pending = self.tasks.iter().filter(|t| t.status == TaskStatus::Pending || t.status == TaskStatus::Blocked).count();
        let skipped = self.tasks.iter().filter(|t| t.status == TaskStatus::Skipped).count();
        let failed = self.tasks.iter().filter(|t| t.status == TaskStatus::Failed).count();
        
        WorkflowProgress {
            total,
            completed,
            in_progress,
            pending,
            skipped,
            failed,
            percentage: if total > 0 { (completed as f32 / total as f32) * 100.0 } else { 0.0 },
        }
    }
    
    /// Get a snapshot for GUI/API
    pub fn to_snapshot(&self) -> WorkflowSnapshot {
        WorkflowSnapshot {
            has_active_workflow: true,
            workflow_id: Some(self.id.clone()),
            workflow_name: Some(self.workflow_type.name()),
            workflow_type: Some(self.workflow_type.type_string().to_string()),
            device_context: self.device_context.clone(),
            progress: self.get_progress(),
            tasks: self.tasks.iter().map(|t| TaskSnapshot {
                id: t.id.clone(),
                description: t.description.clone(),
                status: t.status.as_str().to_string(),
                tool_to_call: t.tool_to_call.clone(),
                result_summary: t.result.as_ref().map(|r| r.summary.clone()),
                error_message: t.result.as_ref().and_then(|r| r.error_message.clone()),
                started_at: t.started_at_iso.clone(),
                completed_at: t.completed_at_iso.clone(),
                duration_ms: t.duration_ms,
                can_skip: t.skippable && !t.status.is_terminal(),
                can_retry: t.retryable && t.status == TaskStatus::Failed && t.retry_count < t.max_retries,
                dependencies: t.dependencies.clone(),
                priority: t.priority.as_str().to_string(),
            }).collect(),
            can_pause: !self.is_paused && !self.is_complete(),
            can_resume: self.is_paused,
            is_paused: self.is_paused,
        }
    }
    
    /// Get a summary string for LLM context (minimal tokens)
    pub fn get_summary(&self, max_tokens: usize) -> String {
        let progress = self.get_progress();
        let mut summary = format!(
            "WORKFLOW: {} | {}/{} tasks | {}%\n",
            self.workflow_type.name(),
            progress.completed,
            progress.total,
            progress.percentage as i32
        );
        
        // Add current task
        if let Some(task) = self.current_task() {
            summary.push_str(&format!("CURRENT: {}\n", task.description));
        }
        
        // Add recent/pending tasks
        let mut estimated_tokens = summary.len() / 4;
        summary.push_str("TASKS:\n");
        
        for task in &self.tasks {
            let line = task.summary_line();
            if estimated_tokens + line.len() / 4 > max_tokens {
                summary.push_str("  ...\n");
                break;
            }
            summary.push_str(&format!("  {}\n", line));
            estimated_tokens += line.len() / 4;
        }
        
        summary
    }
}

/// A completed workflow (for history)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletedWorkflow {
    pub workflow: ActiveWorkflow,
    pub completed_at: String,
    pub success: bool,
    pub duration_ms: Option<u64>,
}

/// The main Workflow Engine
#[derive(Debug, Default)]
pub struct WorkflowEngine {
    /// Active workflows keyed by chat_id (one workflow per chat)
    pub active_workflows: std::collections::HashMap<String, ActiveWorkflow>,
    
    /// History of completed workflows
    pub workflow_history: Vec<CompletedWorkflow>,
    
    /// Maximum history size
    pub max_history_size: usize,
    
    /// Event listeners (for GUI updates)
    event_listeners: Vec<tokio::sync::mpsc::UnboundedSender<WorkflowEvent>>,
}

/// Events emitted by the workflow engine
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum WorkflowEvent {
    WorkflowCreated { workflow_id: String },
    WorkflowCompleted { workflow_id: String, success: bool },
    WorkflowPaused { workflow_id: String },
    WorkflowResumed { workflow_id: String },
    TaskStarted { workflow_id: String, task_id: String },
    TaskCompleted { workflow_id: String, task_id: String, success: bool },
    TaskSkipped { workflow_id: String, task_id: String },
    TaskAdded { workflow_id: String, task_id: String },
}

impl WorkflowEngine {
    pub fn new() -> Self {
        Self {
            active_workflows: std::collections::HashMap::new(),
            workflow_history: vec![],
            max_history_size: 50,
            event_listeners: vec![],
        }
    }
    
    /// Create a new workflow for a specific chat
    pub fn create_workflow(&mut self, chat_id: &str, workflow_type: WorkflowType) -> Result<String, String> {
        if self.active_workflows.contains_key(chat_id) {
            return Err("A workflow is already active for this chat. Complete or cancel it first.".to_string());
        }
        
        let workflow = ActiveWorkflow::new(workflow_type);
        let id = workflow.id.clone();
        self.active_workflows.insert(chat_id.to_string(), workflow);
        
        self.emit_event(WorkflowEvent::WorkflowCreated { workflow_id: id.clone() });
        
        Ok(id)
    }
    
    /// Create a workflow with tasks for a specific chat
    pub fn create_workflow_with_tasks(
        &mut self, 
        chat_id: &str,
        workflow_type: WorkflowType, 
        tasks: Vec<WorkflowTask>,
        device_context: Option<String>,
    ) -> Result<String, String> {
        if self.active_workflows.contains_key(chat_id) {
            return Err("A workflow is already active for this chat. Complete or cancel it first.".to_string());
        }
        
        let mut workflow = ActiveWorkflow::new(workflow_type);
        workflow.tasks = tasks;
        workflow.device_context = device_context;
        
        let id = workflow.id.clone();
        self.active_workflows.insert(chat_id.to_string(), workflow);
        
        self.emit_event(WorkflowEvent::WorkflowCreated { workflow_id: id.clone() });
        
        Ok(id)
    }
    
    /// Get the active workflow for a specific chat
    pub fn get_active_workflow(&self, chat_id: &str) -> Option<&ActiveWorkflow> {
        self.active_workflows.get(chat_id)
    }
    
    /// Get the active workflow mutably for a specific chat
    pub fn get_active_workflow_mut(&mut self, chat_id: &str) -> Option<&mut ActiveWorkflow> {
        self.active_workflows.get_mut(chat_id)
    }
    
    /// Add a task to the active workflow for a specific chat
    pub fn add_task(&mut self, chat_id: &str, task: WorkflowTask) -> Result<(), String> {
        let workflow = self.active_workflows.get_mut(chat_id)
            .ok_or("No active workflow for this chat")?;
        
        let task_id = task.id.clone();
        let workflow_id = workflow.id.clone();
        workflow.add_task(task);
        
        self.emit_event(WorkflowEvent::TaskAdded { 
            workflow_id, 
            task_id,
        });
        
        Ok(())
    }
    
    /// Start a task for a specific chat
    pub fn start_task(&mut self, chat_id: &str, task_id: &str) -> Result<(), String> {
        let workflow = self.active_workflows.get_mut(chat_id)
            .ok_or("No active workflow for this chat")?;
        
        let workflow_id = workflow.id.clone();
        workflow.start_task(task_id)?;
        
        self.emit_event(WorkflowEvent::TaskStarted {
            workflow_id,
            task_id: task_id.to_string(),
        });
        
        Ok(())
    }
    
    /// Complete a task for a specific chat
    pub fn complete_task(&mut self, chat_id: &str, task_id: &str, result: TaskResult) -> Result<(), String> {
        let workflow = self.active_workflows.get_mut(chat_id)
            .ok_or("No active workflow for this chat")?;
        
        let workflow_id = workflow.id.clone();
        let success = result.success;
        workflow.complete_task(task_id, result);
        
        self.emit_event(WorkflowEvent::TaskCompleted {
            workflow_id,
            task_id: task_id.to_string(),
            success,
        });
        
        // Check if workflow is complete
        self.check_workflow_completion(chat_id);
        
        Ok(())
    }
    
    /// Skip a task for a specific chat
    pub fn skip_task(&mut self, chat_id: &str, task_id: &str, reason: impl Into<String>) -> Result<(), String> {
        let workflow = self.active_workflows.get_mut(chat_id)
            .ok_or("No active workflow for this chat")?;
        
        let workflow_id = workflow.id.clone();
        workflow.skip_task(task_id, reason)?;
        
        self.emit_event(WorkflowEvent::TaskSkipped {
            workflow_id,
            task_id: task_id.to_string(),
        });
        
        Ok(())
    }
    
    /// Pause the active workflow for a specific chat
    pub fn pause_workflow(&mut self, chat_id: &str) -> Result<(), String> {
        let workflow = self.active_workflows.get_mut(chat_id)
            .ok_or("No active workflow for this chat")?;
        
        let workflow_id = workflow.id.clone();
        workflow.pause();
        
        self.emit_event(WorkflowEvent::WorkflowPaused { workflow_id });
        
        Ok(())
    }
    
    /// Resume the active workflow for a specific chat
    pub fn resume_workflow(&mut self, chat_id: &str) -> Result<(), String> {
        let workflow = self.active_workflows.get_mut(chat_id)
            .ok_or("No active workflow for this chat")?;
        
        let workflow_id = workflow.id.clone();
        workflow.resume();
        
        self.emit_event(WorkflowEvent::WorkflowResumed { workflow_id });
        
        Ok(())
    }
    
    /// Cancel the active workflow for a specific chat
    pub fn cancel_workflow(&mut self, chat_id: &str) -> Result<(), String> {
        let workflow = self.active_workflows.remove(chat_id)
            .ok_or("No active workflow for this chat")?;
        
        let workflow_id = workflow.id.clone();
        
        // Move to history
        self.workflow_history.push(CompletedWorkflow {
            workflow,
            completed_at: chrono::Utc::now().to_rfc3339(),
            success: false,
            duration_ms: None,
        });
        
        self.trim_history();
        
        self.emit_event(WorkflowEvent::WorkflowCompleted {
            workflow_id,
            success: false,
        });
        
        Ok(())
    }
    
    /// Check if workflow is complete and move to history for a specific chat
    fn check_workflow_completion(&mut self, chat_id: &str) {
        let should_complete = self.active_workflows.get(chat_id)
            .map(|w| w.is_complete())
            .unwrap_or(false);
        
        if should_complete {
            if let Some(workflow) = self.active_workflows.remove(chat_id) {
                let workflow_id = workflow.id.clone();
                let success = !workflow.has_failures();
                let duration_ms = workflow.created_instant
                    .map(|start| start.elapsed().as_millis() as u64);
                
                self.workflow_history.push(CompletedWorkflow {
                    workflow,
                    completed_at: chrono::Utc::now().to_rfc3339(),
                    success,
                    duration_ms,
                });
                
                self.trim_history();
                
                self.emit_event(WorkflowEvent::WorkflowCompleted {
                    workflow_id,
                    success,
                });
            }
        }
    }
    
    /// Trim history to max size
    fn trim_history(&mut self) {
        while self.workflow_history.len() > self.max_history_size {
            self.workflow_history.remove(0);
        }
    }
    
    /// Get a snapshot for GUI/API for a specific chat
    pub fn get_snapshot(&self, chat_id: &str) -> WorkflowSnapshot {
        if let Some(workflow) = self.active_workflows.get(chat_id) {
            workflow.to_snapshot()
        } else {
            WorkflowSnapshot::default()
        }
    }
    
    /// Get workflow summary for LLM context for a specific chat
    pub fn get_summary(&self, chat_id: &str, max_tokens: usize) -> String {
        if let Some(workflow) = self.active_workflows.get(chat_id) {
            workflow.get_summary(max_tokens)
        } else {
            "No active workflow".to_string()
        }
    }
    
    /// Subscribe to workflow events
    pub fn subscribe(&mut self) -> tokio::sync::mpsc::UnboundedReceiver<WorkflowEvent> {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        self.event_listeners.push(tx);
        rx
    }
    
    /// Emit an event to all listeners
    fn emit_event(&mut self, event: WorkflowEvent) {
        // Remove closed channels
        self.event_listeners.retain(|tx| {
            tx.send(event.clone()).is_ok()
        });
    }
    
    /// Check if there's an active workflow for a specific chat
    pub fn has_active_workflow(&self, chat_id: &str) -> bool {
        self.active_workflows.contains_key(chat_id)
    }
    
    /// Get recent workflow history
    pub fn get_history(&self, limit: usize) -> Vec<&CompletedWorkflow> {
        self.workflow_history.iter()
            .rev()
            .take(limit)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    const TEST_CHAT_ID: &str = "test-chat-123";
    
    #[test]
    fn test_workflow_creation() {
        let mut engine = WorkflowEngine::new();
        
        let id = engine.create_workflow(TEST_CHAT_ID, WorkflowType::Generic { 
            name: "Test Workflow".to_string() 
        }).unwrap();
        
        assert!(engine.has_active_workflow(TEST_CHAT_ID));
        assert_eq!(engine.get_active_workflow(TEST_CHAT_ID).unwrap().id, id);
    }
    
    #[test]
    fn test_workflow_with_tasks() {
        let mut engine = WorkflowEngine::new();
        
        let tasks = vec![
            WorkflowTask::new("task_1", "First task"),
            WorkflowTask::new("task_2", "Second task").with_dependencies(vec!["task_1".to_string()]),
        ];
        
        engine.create_workflow_with_tasks(
            TEST_CHAT_ID,
            WorkflowType::BuildAndDeploy { 
                project_name: "test".to_string(),
                target_device: None,
            },
            tasks,
            None,
        ).unwrap();
        
        let workflow = engine.get_active_workflow(TEST_CHAT_ID).unwrap();
        assert_eq!(workflow.tasks.len(), 2);
    }
    
    #[test]
    fn test_task_execution() {
        let mut engine = WorkflowEngine::new();
        
        let tasks = vec![
            WorkflowTask::new("task_1", "First task"),
            WorkflowTask::new("task_2", "Second task"),
        ];
        
        engine.create_workflow_with_tasks(
            TEST_CHAT_ID,
            WorkflowType::Generic { name: "Test".to_string() },
            tasks,
            None,
        ).unwrap();
        
        // Start first task
        engine.start_task(TEST_CHAT_ID, "task_1").unwrap();
        assert_eq!(
            engine.get_active_workflow(TEST_CHAT_ID).unwrap().get_task("task_1").unwrap().status,
            TaskStatus::InProgress
        );
        
        // Complete first task
        engine.complete_task(TEST_CHAT_ID, "task_1", TaskResult::success("Done")).unwrap();
        assert_eq!(
            engine.get_active_workflow(TEST_CHAT_ID).unwrap().get_task("task_1").unwrap().status,
            TaskStatus::Completed
        );
    }
    
    #[test]
    fn test_workflow_pause_resume() {
        let mut engine = WorkflowEngine::new();
        
        engine.create_workflow(TEST_CHAT_ID, WorkflowType::Generic { 
            name: "Test".to_string() 
        }).unwrap();
        
        engine.pause_workflow(TEST_CHAT_ID).unwrap();
        assert!(engine.get_active_workflow(TEST_CHAT_ID).unwrap().is_paused);
        
        engine.resume_workflow(TEST_CHAT_ID).unwrap();
        assert!(!engine.get_active_workflow(TEST_CHAT_ID).unwrap().is_paused);
    }
    
    #[test]
    fn test_workflow_completion() {
        let mut engine = WorkflowEngine::new();
        
        let tasks = vec![
            WorkflowTask::new("task_1", "Only task"),
        ];
        
        engine.create_workflow_with_tasks(
            TEST_CHAT_ID,
            WorkflowType::Generic { name: "Test".to_string() },
            tasks,
            None,
        ).unwrap();
        
        engine.start_task(TEST_CHAT_ID, "task_1").unwrap();
        engine.complete_task(TEST_CHAT_ID, "task_1", TaskResult::success("Done")).unwrap();
        
        // Workflow should be moved to history
        assert!(!engine.has_active_workflow(TEST_CHAT_ID));
        assert_eq!(engine.workflow_history.len(), 1);
        assert!(engine.workflow_history[0].success);
    }
    
    #[test]
    fn test_separate_workflows_per_chat() {
        let mut engine = WorkflowEngine::new();
        
        let chat_1 = "chat-1";
        let chat_2 = "chat-2";
        
        // Create workflow for chat 1
        engine.create_workflow(chat_1, WorkflowType::Generic { 
            name: "Workflow 1".to_string() 
        }).unwrap();
        
        // Create workflow for chat 2
        engine.create_workflow(chat_2, WorkflowType::Generic { 
            name: "Workflow 2".to_string() 
        }).unwrap();
        
        // Both chats should have active workflows
        assert!(engine.has_active_workflow(chat_1));
        assert!(engine.has_active_workflow(chat_2));
        
        // Workflows should be different
        let w1 = engine.get_active_workflow(chat_1).unwrap();
        let w2 = engine.get_active_workflow(chat_2).unwrap();
        assert_ne!(w1.id, w2.id);
        assert_eq!(w1.workflow_type.name(), "Workflow 1");
        assert_eq!(w2.workflow_type.name(), "Workflow 2");
        
        // Cancel chat 1 workflow, chat 2 should still have its workflow
        engine.cancel_workflow(chat_1).unwrap();
        assert!(!engine.has_active_workflow(chat_1));
        assert!(engine.has_active_workflow(chat_2));
    }
}

