//! Workflow API endpoints for GUI integration
//! 
//! Provides REST API and SSE endpoints for:
//! - Viewing workflow status
//! - Managing tasks
//! - Real-time updates via Server-Sent Events

use std::convert::Infallible;
use std::sync::Arc;

use axum::extract::{Path, Query};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::{Extension, Json};
use futures::stream::Stream;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock as ARwLock;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::UnboundedReceiverStream;

use crate::global_context::GlobalContext;
use crate::workflow::{
    WorkflowType, WorkflowTask, 
    TaskResult, TaskPriority,
};

/// Query parameters for workflow endpoints that require chat_id
#[derive(Deserialize)]
pub struct ChatIdQuery {
    pub chat_id: String,
}

/// Response wrapper for API responses
#[derive(Serialize)]
struct ApiResponse<T: Serialize> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }
    
    fn error(message: impl Into<String>) -> ApiResponse<()> {
        ApiResponse {
            success: false,
            data: None,
            error: Some(message.into()),
        }
    }
}

/// GET /v1/workflow?chat_id=xxx - Get current workflow state for a specific chat
pub async fn handle_v1_workflow_get(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    Query(query): Query<ChatIdQuery>,
) -> Response {
    let workflow_engine = gcx.read().await.workflow_engine.clone();
    let engine = workflow_engine.read().await;
    
    let snapshot = engine.get_snapshot(&query.chat_id);
    
    Json(ApiResponse::success(snapshot)).into_response()
}

/// Request body for creating a workflow
#[derive(Deserialize)]
pub struct CreateWorkflowRequest {
    pub chat_id: String,
    pub name: String,
    pub device_context: Option<String>,
    pub tasks: Option<Vec<TaskDefinition>>,
}

#[derive(Deserialize)]
pub struct TaskDefinition {
    pub id: Option<String>,
    pub description: String,
    pub tool: Option<String>,
    pub dependencies: Option<Vec<String>>,
    pub priority: Option<String>,
    pub skippable: Option<bool>,
}

/// POST /v1/workflow - Create a new workflow for a specific chat
pub async fn handle_v1_workflow_create(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    Json(req): Json<CreateWorkflowRequest>,
) -> Response {
    let workflow_engine = gcx.read().await.workflow_engine.clone();
    let mut engine = workflow_engine.write().await;
    
    // Convert task definitions to WorkflowTasks
    let tasks: Vec<WorkflowTask> = req.tasks.unwrap_or_default()
        .into_iter()
        .enumerate()
        .map(|(idx, t)| {
            let id = t.id.unwrap_or_else(|| format!("task_{}", idx + 1));
            let mut task = WorkflowTask::new(&id, &t.description);
            
            if let Some(tool) = t.tool {
                task = task.with_tool(tool);
            }
            if let Some(deps) = t.dependencies {
                task = task.with_dependencies(deps);
            }
            if let Some(priority) = t.priority {
                let p = match priority.to_lowercase().as_str() {
                    "critical" => TaskPriority::Critical,
                    "high" => TaskPriority::High,
                    "low" => TaskPriority::Low,
                    _ => TaskPriority::Normal,
                };
                task = task.with_priority(p);
            }
            if let Some(false) = t.skippable {
                task = task.non_skippable();
            }
            task
        })
        .collect();
    
    let workflow_type = WorkflowType::Generic { name: req.name.clone() };
    
    match engine.create_workflow_with_tasks(&req.chat_id, workflow_type, tasks, req.device_context) {
        Ok(id) => {
            let snapshot = engine.get_snapshot(&req.chat_id);
            Json(ApiResponse::success(serde_json::json!({
                "workflow_id": id,
                "snapshot": snapshot
            }))).into_response()
        }
        Err(e) => {
            (StatusCode::BAD_REQUEST, Json(ApiResponse::<()>::error(e))).into_response()
        }
    }
}

/// DELETE /v1/workflow?chat_id=xxx - Cancel workflow for a specific chat
pub async fn handle_v1_workflow_cancel(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    Query(query): Query<ChatIdQuery>,
) -> Response {
    let workflow_engine = gcx.read().await.workflow_engine.clone();
    let mut engine = workflow_engine.write().await;
    
    match engine.cancel_workflow(&query.chat_id) {
        Ok(()) => Json(ApiResponse::success("Workflow cancelled")).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(ApiResponse::<()>::error(e))).into_response(),
    }
}

/// POST /v1/workflow/pause?chat_id=xxx - Pause workflow for a specific chat
pub async fn handle_v1_workflow_pause(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    Query(query): Query<ChatIdQuery>,
) -> Response {
    let workflow_engine = gcx.read().await.workflow_engine.clone();
    let mut engine = workflow_engine.write().await;
    
    match engine.pause_workflow(&query.chat_id) {
        Ok(()) => Json(ApiResponse::success("Workflow paused")).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(ApiResponse::<()>::error(e))).into_response(),
    }
}

/// POST /v1/workflow/resume?chat_id=xxx - Resume workflow for a specific chat
pub async fn handle_v1_workflow_resume(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    Query(query): Query<ChatIdQuery>,
) -> Response {
    let workflow_engine = gcx.read().await.workflow_engine.clone();
    let mut engine = workflow_engine.write().await;
    
    match engine.resume_workflow(&query.chat_id) {
        Ok(()) => Json(ApiResponse::success("Workflow resumed")).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(ApiResponse::<()>::error(e))).into_response(),
    }
}

/// Request body for adding a task
#[derive(Deserialize)]
pub struct AddTaskRequest {
    pub chat_id: String,
    pub id: Option<String>,
    pub description: String,
    pub tool: Option<String>,
    pub dependencies: Option<Vec<String>>,
    pub priority: Option<String>,
}

/// POST /v1/workflow/tasks - Add a task to a specific chat's workflow
pub async fn handle_v1_workflow_task_add(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    Json(req): Json<AddTaskRequest>,
) -> Response {
    let workflow_engine = gcx.read().await.workflow_engine.clone();
    let mut engine = workflow_engine.write().await;
    
    let id = req.id.unwrap_or_else(|| format!("task_{}", uuid::Uuid::new_v4().to_string().split('-').next().unwrap()));
    let mut task = WorkflowTask::new(&id, &req.description);
    
    if let Some(tool) = req.tool {
        task = task.with_tool(tool);
    }
    if let Some(deps) = req.dependencies {
        task = task.with_dependencies(deps);
    }
    if let Some(priority) = req.priority {
        let p = match priority.to_lowercase().as_str() {
            "critical" => TaskPriority::Critical,
            "high" => TaskPriority::High,
            "low" => TaskPriority::Low,
            _ => TaskPriority::Normal,
        };
        task = task.with_priority(p);
    }
    
    match engine.add_task(&req.chat_id, task) {
        Ok(()) => {
            let snapshot = engine.get_snapshot(&req.chat_id);
            Json(ApiResponse::success(serde_json::json!({
                "task_id": id,
                "snapshot": snapshot
            }))).into_response()
        }
        Err(e) => (StatusCode::BAD_REQUEST, Json(ApiResponse::<()>::error(e))).into_response(),
    }
}

/// Request body for task actions
#[derive(Deserialize)]
pub struct TaskActionRequest {
    pub chat_id: String,
    pub action: String,  // start, complete, skip, retry
    pub result: Option<String>,
    pub reason: Option<String>,
}

/// POST /v1/workflow/tasks/:task_id/action - Perform action on a task for a specific chat
pub async fn handle_v1_workflow_task_action(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    Path(task_id): Path<String>,
    Json(req): Json<TaskActionRequest>,
) -> Response {
    let workflow_engine = gcx.read().await.workflow_engine.clone();
    let mut engine = workflow_engine.write().await;
    
    let result = match req.action.as_str() {
        "start" => engine.start_task(&req.chat_id, &task_id),
        "complete" => {
            let result = TaskResult::success(req.result.unwrap_or_else(|| "Completed".to_string()));
            engine.complete_task(&req.chat_id, &task_id, result)
        }
        "skip" => {
            let reason = req.reason.unwrap_or_else(|| "Skipped".to_string());
            engine.skip_task(&req.chat_id, &task_id, reason)
        }
        _ => Err(format!("Unknown action: {}", req.action)),
    };
    
    match result {
        Ok(()) => {
            let snapshot = engine.get_snapshot(&req.chat_id);
            Json(ApiResponse::success(snapshot)).into_response()
        }
        Err(e) => (StatusCode::BAD_REQUEST, Json(ApiResponse::<()>::error(e))).into_response(),
    }
}

/// GET /v1/workflow/history - Get workflow history
pub async fn handle_v1_workflow_history(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
) -> Response {
    let workflow_engine = gcx.read().await.workflow_engine.clone();
    let engine = workflow_engine.read().await;
    
    let history: Vec<_> = engine.get_history(10)
        .into_iter()
        .map(|w| serde_json::json!({
            "id": w.workflow.id,
            "name": w.workflow.workflow_type.name(),
            "completed_at": w.completed_at,
            "success": w.success,
            "duration_ms": w.duration_ms,
            "task_count": w.workflow.tasks.len(),
        }))
        .collect();
    
    Json(ApiResponse::success(history)).into_response()
}

/// GET /v1/workflow/events - SSE stream for real-time updates
pub async fn handle_v1_workflow_events(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let workflow_engine = gcx.read().await.workflow_engine.clone();
    let rx = {
        let mut engine = workflow_engine.write().await;
        engine.subscribe()
    };
    
    let stream = UnboundedReceiverStream::new(rx)
        .map(|event| {
            let json = serde_json::to_string(&event).unwrap_or_default();
            Ok(Event::default().data(json))
        });
    
    Sse::new(stream).keep_alive(KeepAlive::default())
}

/// GET /v1/workflow/summary?chat_id=xxx - Get a text summary for LLM context for a specific chat
pub async fn handle_v1_workflow_summary(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    Query(query): Query<ChatIdQuery>,
) -> Response {
    let workflow_engine = gcx.read().await.workflow_engine.clone();
    let engine = workflow_engine.read().await;
    
    let summary = engine.get_summary(&query.chat_id, 200);
    
    Json(ApiResponse::success(serde_json::json!({
        "summary": summary,
        "has_active_workflow": engine.has_active_workflow(&query.chat_id),
    }))).into_response()
}

