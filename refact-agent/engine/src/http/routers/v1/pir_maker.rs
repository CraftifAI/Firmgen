//! HTTP endpoints for PIR_maker — live firmware project intelligence.

use std::sync::Arc;

use axum::extract::Query;
use axum::http::{Response, StatusCode};
use axum::Extension;
use axum::Json;
use hyper::Body;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use tokio::sync::RwLock as ARwLock;

use crate::custom_error::ScratchError;
use crate::global_context::GlobalContext;
use crate::pir_maker::{
    analyze_for_chat, apply_node_patch, apply_structural_patch, approve, get, persistence,
    project_has_codegen_artifacts, spawn_analyze_for_chat, PirAnalyzeResult, PirApprovalStatus,
};
use crate::pir_maker::apply_patch::StructuralPatchRequest;

type SharedGlobalContext = Arc<ARwLock<GlobalContext>>;

#[derive(Deserialize)]
pub struct ChatIdQuery {
    pub chat_id: String,
}

#[derive(Deserialize)]
pub struct PirAnalyzePost {
    pub chat_id: String,
    #[serde(default)]
    pub project_path: Option<String>,
    #[serde(default)]
    pub incremental: bool,
    #[serde(default)]
    pub triggered_by: Option<String>,
    #[serde(default)]
    pub async_mode: bool,
    /// Recent main-chat user text — used to infer missing topology nodes.
    #[serde(default)]
    pub chat_context: Option<String>,
}

#[derive(Deserialize)]
pub struct PirWatchPost {
    pub chat_id: String,
    pub project_path: String,
}

#[derive(Deserialize)]
pub struct PirApplyPatchPost {
    pub chat_id: String,
    pub node_id: String,
    pub property_updates: JsonValue,
    #[serde(default)]
    pub expected_revision: Option<String>,
}

#[derive(Deserialize)]
pub struct PirStructuralPatchPost {
    pub chat_id: String,
    #[serde(default)]
    pub expected_revision: Option<String>,
    #[serde(flatten)]
    pub patch: StructuralPatchRequest,
}

#[derive(Deserialize)]
pub struct PirApprovePost {
    pub chat_id: String,
    #[serde(default)]
    pub comment: Option<String>,
}

#[derive(Deserialize)]
pub struct PirDiffQuery {
    pub chat_id: String,
    #[serde(default)]
    pub from_revision: Option<String>,
    #[serde(default)]
    pub to_revision: Option<String>,
}

#[derive(Serialize)]
pub struct PirStatusResponse {
    pub active: bool,
    pub chat_id: Option<String>,
    pub status: String,
    pub project_path: Option<String>,
    pub revision: Option<String>,
    pub approval_status: Option<String>,
    pub watch_enabled: bool,
    pub error: Option<String>,
    pub summary_headline: Option<String>,
    pub agent_mode: String,
    pub graph_version: Option<u32>,
}

#[derive(Serialize)]
pub struct PirDocumentResponse {
    pub result: PirAnalyzeResult,
}

#[derive(Deserialize)]
pub struct PirGraphViewQuery {
    pub chat_id: String,
    pub view: String,
}

fn json_response<T: Serialize>(value: &T) -> Result<Response<Body>, ScratchError> {
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(value).unwrap()))
        .unwrap())
}

fn slim_result_for_transport(mut result: PirAnalyzeResult) -> PirAnalyzeResult {
    if let Some(diagrams) = result.pir.diagrams.as_mut() {
        diagrams.hld_graph = None;
        diagrams.lld_graph = None;
        diagrams.sequence_graph = None;
    }
    result
}

pub async fn handle_v1_pir_maker_analyze(
    Extension(gcx): Extension<SharedGlobalContext>,
    Json(body): Json<PirAnalyzePost>,
) -> Result<Response<Body>, ScratchError> {
    let chat_id = body.chat_id.trim();
    if chat_id.is_empty() {
        return Err(ScratchError::new(
            StatusCode::BAD_REQUEST,
            "chat_id is required".to_string(),
        ));
    }
    let triggered_by = body.triggered_by.as_deref().unwrap_or(if body.incremental {
        "user_incremental"
    } else {
        "user_full"
    });

    if body.async_mode {
        spawn_analyze_for_chat(
            gcx,
            chat_id,
            body.project_path.as_deref(),
            body.incremental,
            triggered_by,
            body.chat_context.clone(),
        )
        .await;
        return json_response(&serde_json::json!({ "status": "analyzing" }));
    }

    let result = analyze_for_chat(
        gcx,
        chat_id,
        body.project_path.as_deref(),
        body.incremental,
        triggered_by,
        body.chat_context.as_deref(),
    )
    .await
    .map_err(|e| ScratchError::new(StatusCode::BAD_REQUEST, e))?;
    json_response(&slim_result_for_transport(result))
}

pub async fn handle_v1_pir_maker_status(
    Query(q): Query<ChatIdQuery>,
) -> Result<Response<Body>, ScratchError> {
    let chat_id = q.chat_id.trim();
    let session = get(chat_id).await;
    let resp = match session {
        None => PirStatusResponse {
            active: false,
            chat_id: None,
            status: "idle".to_string(),
            project_path: None,
            revision: None,
            approval_status: None,
            watch_enabled: false,
            error: None,
            summary_headline: None,
            agent_mode: crate::pir_maker::agent::agent_mode(),
            graph_version: None,
        },
        Some(s) => {
            let (revision, approval, headline, graph_version) = s
                .last_result
                .as_ref()
                .map(|r| {
                    (
                        Some(r.pir.revision.clone()),
                        Some(approval_status_str(&r.pir.approval.status)),
                        r.pir.summary.as_ref().map(|x| x.headline.clone()),
                        Some(r.pir.graph_version),
                    )
                })
                .unwrap_or((None, None, None, None));
            PirStatusResponse {
                active: true,
                chat_id: Some(s.chat_id),
                status: s.status.as_str().to_string(),
                project_path: s.project_path,
                revision,
                approval_status: approval,
                watch_enabled: s.watch_enabled,
                error: s.error,
                summary_headline: headline,
                agent_mode: crate::pir_maker::agent::agent_mode(),
                graph_version,
            }
        }
    };
    json_response(&resp)
}

pub async fn handle_v1_pir_maker_document(
    Query(q): Query<ChatIdQuery>,
) -> Result<Response<Body>, ScratchError> {
    let chat_id = q.chat_id.trim();
    let session = get(chat_id).await.ok_or_else(|| {
        ScratchError::new(
            StatusCode::NOT_FOUND,
            "no PIR_maker session for chat".to_string(),
        )
    })?;
    let result = session.last_result.ok_or_else(|| {
        ScratchError::new(
            StatusCode::NOT_FOUND,
            "no analyzed document yet; call POST /pir-maker/analyze first".to_string(),
        )
    })?;
    json_response(&PirDocumentResponse {
        result: slim_result_for_transport(result),
    })
}

pub async fn handle_v1_pir_maker_graph_view_document(
    Query(q): Query<PirGraphViewQuery>,
) -> Result<Response<Body>, ScratchError> {
    let chat_id = q.chat_id.trim();
    let session = get(chat_id).await.ok_or_else(|| {
        ScratchError::new(
            StatusCode::NOT_FOUND,
            "no PIR_maker session for chat".to_string(),
        )
    })?;
    let project_path = session.project_path.clone();
    let result = session.last_result.ok_or_else(|| {
        ScratchError::new(
            StatusCode::NOT_FOUND,
            "no analyzed document yet; call POST /pir-maker/analyze first".to_string(),
        )
    })?;
    let view = persistence::PirGraphView::from_query(q.view.trim()).ok_or_else(|| {
        ScratchError::new(
            StatusCode::BAD_REQUEST,
            "view must be one of: wiring, hld, lld, sequence".to_string(),
        )
    })?;
    let root = project_path
        .as_deref()
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from(&result.pir.provenance.project_path));

    if let Some(doc) = persistence::load_graph_view(&root, view) {
        if doc.revision == result.pir.revision && doc.graph_version == result.pir.graph_version {
            return json_response(&doc);
        }
    }

    let doc = persistence::build_graph_view_document(&result, view);
    let _ = persistence::save_graph_views(&root, &result);
    json_response(&doc)
}

pub async fn handle_v1_pir_maker_apply_patch(
    Extension(gcx): Extension<SharedGlobalContext>,
    Json(body): Json<PirApplyPatchPost>,
) -> Result<Response<Body>, ScratchError> {
    let updates = body.property_updates.as_object().cloned().ok_or_else(|| {
        ScratchError::new(
            StatusCode::BAD_REQUEST,
            "property_updates must be a JSON object".to_string(),
        )
    })?;
    let result = apply_node_patch(
        gcx,
        body.chat_id.trim(),
        body.node_id.trim(),
        updates,
        body.expected_revision.as_deref(),
    )
    .await
    .map_err(|e| ScratchError::new(StatusCode::BAD_REQUEST, e))?;
    json_response(&slim_result_for_transport(result))
}

pub async fn handle_v1_pir_maker_structural_patch(
    Extension(gcx): Extension<SharedGlobalContext>,
    Json(body): Json<PirStructuralPatchPost>,
) -> Result<Response<Body>, ScratchError> {
    let result = apply_structural_patch(
        gcx,
        body.chat_id.trim(),
        body.patch,
        body.expected_revision.as_deref(),
    )
    .await
    .map_err(|e| ScratchError::new(StatusCode::BAD_REQUEST, e))?;
    json_response(&slim_result_for_transport(result))
}

#[derive(Deserialize)]
pub struct PirCodegenReadyQuery {
    pub project_path: String,
}

#[derive(Serialize)]
pub struct PirCodegenReadyResponse {
    pub ready: bool,
}

pub async fn handle_v1_pir_maker_codegen_ready(
    Query(q): Query<PirCodegenReadyQuery>,
) -> Result<Response<Body>, ScratchError> {
    let path = std::path::PathBuf::from(q.project_path.trim());
    if !path.is_dir() {
        return Err(ScratchError::new(
            StatusCode::BAD_REQUEST,
            "project_path must be an existing directory".to_string(),
        ));
    }
    json_response(&PirCodegenReadyResponse {
        ready: project_has_codegen_artifacts(&path),
    })
}

pub async fn handle_v1_pir_maker_approve(
    Json(body): Json<PirApprovePost>,
) -> Result<Response<Body>, ScratchError> {
    approve(body.chat_id.trim(), body.comment)
        .await
        .map_err(|e| ScratchError::new(StatusCode::BAD_REQUEST, e))?;
    json_response(&serde_json::json!({ "ok": true }))
}

pub async fn handle_v1_pir_maker_watch(
    Json(body): Json<PirWatchPost>,
) -> Result<Response<Body>, ScratchError> {
    let path = std::path::PathBuf::from(body.project_path.trim());
    if !path.is_dir() {
        return Err(ScratchError::new(
            StatusCode::BAD_REQUEST,
            "project_path must be an existing directory".to_string(),
        ));
    }
    crate::pir_maker::sync::enable_watch(body.chat_id.trim(), path).await;
    crate::pir_maker::session::set_watch(body.chat_id.trim(), true).await;
    json_response(&serde_json::json!({ "watching": true }))
}

pub async fn handle_v1_pir_maker_unwatch(
    Json(body): Json<ChatIdQuery>,
) -> Result<Response<Body>, ScratchError> {
    crate::pir_maker::sync::disable_watch(body.chat_id.trim()).await;
    crate::pir_maker::session::set_watch(body.chat_id.trim(), false).await;
    json_response(&serde_json::json!({ "watching": false }))
}

pub async fn handle_v1_pir_maker_diff(
    Query(q): Query<PirDiffQuery>,
) -> Result<Response<Body>, ScratchError> {
    let session = get(q.chat_id.trim())
        .await
        .ok_or_else(|| ScratchError::new(StatusCode::NOT_FOUND, "no PIR session".to_string()))?;
    let result = session.last_result.ok_or_else(|| {
        ScratchError::new(StatusCode::NOT_FOUND, "no analyzed document".to_string())
    })?;

    if let Some(diff) = &result.diff {
        return json_response(diff);
    }

    if let Some(project_path) = session.project_path {
        let root = std::path::PathBuf::from(project_path);
        if let Some(from_rev) = q.from_revision {
            let hist = persistence::pir_history_dir(&root).join(format!("pir_{}.json", from_rev));
            if hist.is_file() {
                let from_text = std::fs::read_to_string(&hist)
                    .map_err(|e| ScratchError::new(StatusCode::NOT_FOUND, e.to_string()))?;
                let from_doc: crate::pir_maker::PirDocument = serde_json::from_str(&from_text)
                    .map_err(|e| ScratchError::new(StatusCode::BAD_REQUEST, e.to_string()))?;
                let diff = crate::pir_maker::diff_documents(&from_doc, &result.pir);
                return json_response(&diff);
            }
        }
    }

    Err(ScratchError::new(
        StatusCode::NOT_FOUND,
        "no diff available".to_string(),
    ))
}

pub async fn handle_v1_pir_maker_registry() -> Result<Response<Body>, ScratchError> {
    let types = crate::firmware_topology::list_node_types();
    json_response(&types)
}

fn approval_status_str(s: &PirApprovalStatus) -> String {
    match s {
        PirApprovalStatus::Pending => "pending",
        PirApprovalStatus::Approved => "approved",
        PirApprovalStatus::Rejected => "rejected",
        PirApprovalStatus::Stale => "stale",
    }
    .to_string()
}
