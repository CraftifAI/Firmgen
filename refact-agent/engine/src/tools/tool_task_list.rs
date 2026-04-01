//! Task List Tool - Exposes workflow management to the agent
//!
//! This tool allows the agent to:
//! - View current workflow and task status
//! - Add new tasks to the workflow
//! - Complete, skip, or retry tasks
//! - Pause and resume workflows
//! - Create new workflows

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex as AMutex;
use async_trait::async_trait;
use serde_json::Value;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::{ChatContent, ChatMessage, ContextEnum};
use crate::tools::tools_description::{Tool, ToolDesc, ToolParam, ToolSource, ToolSourceType};
use crate::workflow::{
    WorkflowEngine, WorkflowType, WorkflowTask, TaskResult, TaskPriority,
};

pub struct ToolTaskList {
    pub config_path: String,
}

const TOOL_DESC: &str = r#"Manage workflow tasks: view current tasks, add new tasks, mark tasks complete, skip tasks, or control workflow execution.

Actions:
- "view": View current workflow status and all tasks
- "create": Create a new workflow with tasks
- "add": Add a new task to the current workflow
- "complete": Mark a task as completed
- "skip": Skip a task (with reason)
- "start": Start executing a specific task
- "pause": Pause the current workflow
- "resume": Resume a paused workflow
- "cancel": Cancel the current workflow

Use this tool to track progress on multi-step tasks and maintain visibility of what's been done and what remains."#;

#[async_trait]
impl Tool for ToolTaskList {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn tool_description(&self) -> ToolDesc {
        ToolDesc {
            name: "task_list".to_string(),
            display_name: "Task List".to_string(),
            source: ToolSource {
                source_type: ToolSourceType::Builtin,
                config_path: self.config_path.clone(),
            },
            agentic: true,
            experimental: false,
            description: TOOL_DESC.to_string(),
            parameters: vec![
                ToolParam {
                    name: "action".to_string(),
                    param_type: "string".to_string(),
                    description: "Action to perform: view, create, add, complete, skip, start, pause, resume, cancel".to_string(),
                },
                ToolParam {
                    name: "workflow_name".to_string(),
                    param_type: "string".to_string(),
                    description: "Name for a new workflow (required for 'create' action)".to_string(),
                },
                ToolParam {
                    name: "task_id".to_string(),
                    param_type: "string".to_string(),
                    description: "Task ID (required for complete, skip, start actions)".to_string(),
                },
                ToolParam {
                    name: "task_description".to_string(),
                    param_type: "string".to_string(),
                    description: "Description of a new task (required for 'add' action)".to_string(),
                },
                ToolParam {
                    name: "tasks".to_string(),
                    param_type: "string".to_string(),
                    description: "JSON array of tasks for 'create' action: [{\"id\": \"task_1\", \"description\": \"...\", \"tool\": \"optional_tool_name\"}]".to_string(),
                },
                ToolParam {
                    name: "result".to_string(),
                    param_type: "string".to_string(),
                    description: "Result summary (for 'complete' action)".to_string(),
                },
                ToolParam {
                    name: "reason".to_string(),
                    param_type: "string".to_string(),
                    description: "Reason for skipping (for 'skip' action)".to_string(),
                },
                ToolParam {
                    name: "tool_name".to_string(),
                    param_type: "string".to_string(),
                    description: "Tool to call for this task (optional for 'add' action)".to_string(),
                },
                ToolParam {
                    name: "dependencies".to_string(),
                    param_type: "string".to_string(),
                    description: "Comma-separated task IDs this task depends on (optional for 'add' action)".to_string(),
                },
                ToolParam {
                    name: "device_context".to_string(),
                    param_type: "string".to_string(),
                    description: "Device context for the workflow, e.g., 'c2000', 'esp32' (optional for 'create' action)".to_string(),
                },
            ],
            parameters_required: vec!["action".to_string()],
        }
    }

    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>,
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        let action = args.get("action")
            .and_then(|v| v.as_str())
            .ok_or("Missing required parameter: action")?;

        let (gcx, chat_id) = {
            let ccx_locked = ccx.lock().await;
            (ccx_locked.global_context.clone(), ccx_locked.chat_id.clone())
        };
        let workflow_engine = gcx.read().await.workflow_engine.clone();
        
        // Use read lock for read-only operations, write lock only for modifications
        let result = match action {
            "view" => {
                // Read-only operation - use read lock (allows concurrent reads)
                let engine = workflow_engine.read().await;
                handle_view(&*engine, &chat_id)
            },
            "create" | "add" | "complete" | "skip" | "start" | "pause" | "resume" | "cancel" => {
                // Write operations - use write lock
                let mut engine = workflow_engine.write().await;
                match action {
                    "create" => handle_create(&mut engine, &chat_id, args),
                    "add" => handle_add(&mut engine, &chat_id, args),
                    "complete" => handle_complete(&mut engine, &chat_id, args),
                    "skip" => handle_skip(&mut engine, &chat_id, args),
                    "start" => handle_start(&mut engine, &chat_id, args),
                    "pause" => handle_pause(&mut engine, &chat_id),
                    "resume" => handle_resume(&mut engine, &chat_id),
                    "cancel" => handle_cancel(&mut engine, &chat_id),
                    _ => unreachable!(),
                }
            },
            _ => Err(format!("Unknown action: {}. Valid actions: view, create, add, complete, skip, start, pause, resume, cancel", action)),
        }?;

        let message = ChatMessage {
            role: "tool".to_string(),
            content: ChatContent::SimpleText(result),
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
            ..Default::default()
        };

        Ok((false, vec![ContextEnum::ChatMessage(message)]))
    }

    fn tool_depends_on(&self) -> Vec<String> {
        vec![]
    }
}

fn handle_view(engine: &WorkflowEngine, chat_id: &str) -> Result<String, String> {
    let snapshot = engine.get_snapshot(chat_id);
    
    if !snapshot.has_active_workflow {
        return Ok("No active workflow. Use action='create' to start a new workflow.".to_string());
    }

    let mut output = format!(
        "## Workflow: {}\n",
        snapshot.workflow_name.as_deref().unwrap_or("Unknown")
    );
    
    if let Some(device) = &snapshot.device_context {
        output.push_str(&format!("Device: {}\n", device));
    }
    
    output.push_str(&format!(
        "Progress: {}/{} tasks ({}%)\n",
        snapshot.progress.completed,
        snapshot.progress.total,
        snapshot.progress.percentage as i32
    ));
    
    if snapshot.is_paused {
        output.push_str("⏸️ PAUSED\n");
    }
    
    output.push_str("\n### Tasks:\n");
    
    for task in &snapshot.tasks {
        let status_icon = match task.status.as_str() {
            "pending" => "⬜",
            "in_progress" => "🔄",
            "completed" => "✅",
            "skipped" => "⏭️",
            "failed" => "❌",
            "blocked" => "🚫",
            _ => "❓",
        };
        
        output.push_str(&format!(
            "{} **{}**: {}\n",
            status_icon,
            task.id,
            task.description
        ));
        
        if let Some(result) = &task.result_summary {
            output.push_str(&format!("   └─ {}\n", result));
        }
        
        if let Some(error) = &task.error_message {
            output.push_str(&format!("   └─ Error: {}\n", error));
        }
        
        if let Some(duration) = task.duration_ms {
            output.push_str(&format!("   └─ Duration: {}ms\n", duration));
        }
    }
    
    // Add available actions
    output.push_str("\n### Available Actions:\n");
    if snapshot.can_pause {
        output.push_str("- `pause`: Pause workflow\n");
    }
    if snapshot.can_resume {
        output.push_str("- `resume`: Resume workflow\n");
    }
    
    let pending_tasks: Vec<_> = snapshot.tasks.iter()
        .filter(|t| t.status == "pending")
        .collect();
    
    if !pending_tasks.is_empty() {
        output.push_str("- `start`: Start a pending task\n");
    }
    
    Ok(output)
}

fn handle_create(engine: &mut WorkflowEngine, chat_id: &str, args: &HashMap<String, Value>) -> Result<String, String> {
    let workflow_name = args.get("workflow_name")
        .and_then(|v| v.as_str())
        .ok_or("Missing required parameter: workflow_name")?;

    let device_context = args.get("device_context")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // Parse tasks if provided
    let tasks: Vec<WorkflowTask> = if let Some(tasks_json) = args.get("tasks").and_then(|v| v.as_str()) {
        parse_tasks_json(tasks_json)?
    } else {
        vec![]
    };

    let workflow_type = WorkflowType::Generic {
        name: workflow_name.to_string(),
    };

    let workflow_id = engine.create_workflow_with_tasks(chat_id, workflow_type, tasks, device_context)?;

    let snapshot = engine.get_snapshot(chat_id);
    
    Ok(format!(
        "✅ Created workflow '{}' (ID: {})\nTasks: {}\n\nUse action='view' to see task details, or action='start' with task_id to begin execution.",
        workflow_name,
        workflow_id,
        snapshot.progress.total
    ))
}

fn handle_add(engine: &mut WorkflowEngine, chat_id: &str, args: &HashMap<String, Value>) -> Result<String, String> {
    let task_description = args.get("task_description")
        .and_then(|v| v.as_str())
        .ok_or("Missing required parameter: task_description")?;

    // Generate task ID if not provided
    let task_id = args.get("task_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("task_{}", uuid::Uuid::new_v4().to_string().split('-').next().unwrap()));

    let mut task = WorkflowTask::new(&task_id, task_description);

    // Add optional tool
    if let Some(tool_name) = args.get("tool_name").and_then(|v| v.as_str()) {
        task = task.with_tool(tool_name);
    }

    // Add optional dependencies
    if let Some(deps) = args.get("dependencies").and_then(|v| v.as_str()) {
        let dep_ids: Vec<String> = deps.split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        task = task.with_dependencies(dep_ids);
    }

    engine.add_task(chat_id, task)?;

    Ok(format!("✅ Added task '{}': {}", task_id, task_description))
}

fn handle_complete(engine: &mut WorkflowEngine, chat_id: &str, args: &HashMap<String, Value>) -> Result<String, String> {
    let task_id = args.get("task_id")
        .and_then(|v| v.as_str())
        .ok_or("Missing required parameter: task_id")?;

    let result_summary = args.get("result")
        .and_then(|v| v.as_str())
        .unwrap_or("Completed");

    let result = TaskResult::success(result_summary);
    engine.complete_task(chat_id, task_id, result)?;

    let snapshot = engine.get_snapshot(chat_id);
    
    let mut output = format!(
        "✅ Completed task '{}'\nProgress: {}/{} ({}%)",
        task_id,
        snapshot.progress.completed,
        snapshot.progress.total,
        snapshot.progress.percentage as i32
    );
    
    // If not 100% complete, indicate there's more work and suggest next task
    if snapshot.progress.completed < snapshot.progress.total {
        let remaining = snapshot.progress.total - snapshot.progress.completed;
        
        // Find the next available task
        let next_task = snapshot.tasks.iter()
            .find(|t| t.status == "pending" || t.status == "blocked");
        
        if let Some(task) = next_task {
            output.push_str(&format!(
                "\n\n⚠️ {} task(s) remaining.\n**Next task**: {} - {}\nUse `task_list` with action='start' and task_id='{}' to begin.",
                remaining,
                task.id,
                task.description,
                task.id
            ));
        } else {
            output.push_str(&format!("\n\n⚠️ {} task(s) remaining. Use `task_list` with action='view' to see pending tasks.", remaining));
        }
    }
    
    Ok(output)
}

fn handle_skip(engine: &mut WorkflowEngine, chat_id: &str, args: &HashMap<String, Value>) -> Result<String, String> {
    let task_id = args.get("task_id")
        .and_then(|v| v.as_str())
        .ok_or("Missing required parameter: task_id")?;

    let reason = args.get("reason")
        .and_then(|v| v.as_str())
        .unwrap_or("Skipped by user");

    engine.skip_task(chat_id, task_id, reason)?;

    Ok(format!("⏭️ Skipped task '{}': {}", task_id, reason))
}

fn handle_start(engine: &mut WorkflowEngine, chat_id: &str, args: &HashMap<String, Value>) -> Result<String, String> {
    let task_id = args.get("task_id")
        .and_then(|v| v.as_str())
        .ok_or("Missing required parameter: task_id")?;

    engine.start_task(chat_id, task_id)?;

    let workflow = engine.get_active_workflow(chat_id)
        .ok_or("No active workflow")?;
    
    let task = workflow.get_task(task_id)
        .ok_or("Task not found")?;

    let mut output = format!("🔄 Started task '{}': {}\n", task_id, task.description);
    
    if let Some(tool) = &task.tool_to_call {
        output.push_str(&format!("Suggested tool: {}\n", tool));
    }

    Ok(output)
}

fn handle_pause(engine: &mut WorkflowEngine, chat_id: &str) -> Result<String, String> {
    engine.pause_workflow(chat_id)?;
    Ok("⏸️ Workflow paused. Use action='resume' to continue.".to_string())
}

fn handle_resume(engine: &mut WorkflowEngine, chat_id: &str) -> Result<String, String> {
    engine.resume_workflow(chat_id)?;
    Ok("▶️ Workflow resumed.".to_string())
}

fn handle_cancel(engine: &mut WorkflowEngine, chat_id: &str) -> Result<String, String> {
    engine.cancel_workflow(chat_id)?;
    Ok("🚫 Workflow cancelled.".to_string())
}

fn parse_tasks_json(json_str: &str) -> Result<Vec<WorkflowTask>, String> {
    let tasks_value: Value = serde_json::from_str(json_str)
        .map_err(|e| format!("Invalid JSON for tasks: {}", e))?;

    let tasks_array = tasks_value.as_array()
        .ok_or("Tasks must be a JSON array")?;

    let mut tasks = Vec::new();
    
    for (idx, task_value) in tasks_array.iter().enumerate() {
        let id = task_value.get("id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("task_{}", idx + 1));

        let description = task_value.get("description")
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("Task {} missing 'description' field", idx))?;

        let mut task = WorkflowTask::new(&id, description);

        if let Some(tool) = task_value.get("tool").and_then(|v| v.as_str()) {
            task = task.with_tool(tool);
        }

        if let Some(deps) = task_value.get("dependencies").and_then(|v| v.as_array()) {
            let dep_ids: Vec<String> = deps.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
            task = task.with_dependencies(dep_ids);
        }

        if let Some(priority) = task_value.get("priority").and_then(|v| v.as_str()) {
            let priority = match priority.to_lowercase().as_str() {
                "critical" => TaskPriority::Critical,
                "high" => TaskPriority::High,
                "low" => TaskPriority::Low,
                _ => TaskPriority::Normal,
            };
            task = task.with_priority(priority);
        }

        if let Some(skippable) = task_value.get("skippable").and_then(|v| v.as_bool()) {
            if !skippable {
                task = task.non_skippable();
            }
        }

        tasks.push(task);
    }

    Ok(tasks)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_tasks_json() {
        let json = r#"[
            {"id": "task_1", "description": "First task"},
            {"id": "task_2", "description": "Second task", "tool": "build", "dependencies": ["task_1"]}
        ]"#;

        let tasks = parse_tasks_json(json).unwrap();
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].id, "task_1");
        assert_eq!(tasks[1].dependencies, vec!["task_1".to_string()]);
    }
}

