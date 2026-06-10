use std::path::PathBuf;
use std::sync::Arc;

use axum::Extension;
use axum::Json;
use hyper::StatusCode;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock as ARwLock;

use crate::at_commands::at_file::{file_repair_candidates, return_one_candidate_or_a_good_error};
use crate::custom_error::ScratchError;
use crate::files_correction::preprocess_path_for_normalization;
use crate::files_in_workspace::get_file_text_from_memory_or_disk;
use crate::global_context::GlobalContext;

const MAX_FILE_BYTES: usize = 512 * 1024;

#[derive(Deserialize)]
pub struct WorkspaceFileContentPost {
    pub path: String,
}

#[derive(Serialize)]
pub struct WorkspaceFileContentResponse {
    pub path: String,
    pub content: String,
    pub truncated: bool,
}

/// POST: read workspace file text for diff hydration in the chat UI.
pub async fn handle_v1_workspace_file_content(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    Json(body): Json<WorkspaceFileContentPost>,
) -> Result<Json<WorkspaceFileContentResponse>, ScratchError> {
    let path_raw = preprocess_path_for_normalization(body.path.trim().to_string());
    if path_raw.is_empty() {
        return Err(ScratchError::new(
            StatusCode::BAD_REQUEST,
            "path is required".to_string(),
        ));
    }

    let candidates = file_repair_candidates(gcx.clone(), &path_raw, 10, false).await;
    let resolved = return_one_candidate_or_a_good_error(
        gcx.clone(),
        &path_raw,
        &candidates,
        &vec![],
        false,
    )
    .await
    .map_err(|e| ScratchError::new(StatusCode::BAD_REQUEST, e))?;

    let file_path = PathBuf::from(resolved.clone());
    let content = get_file_text_from_memory_or_disk(gcx.clone(), &file_path)
        .await
        .map_err(|e| ScratchError::new(StatusCode::BAD_REQUEST, e))?;

    let truncated = content.len() > MAX_FILE_BYTES;
    let content = if truncated {
        content.chars().take(MAX_FILE_BYTES).collect::<String>()
    } else {
        content
    };

    Ok(Json(WorkspaceFileContentResponse {
        path: resolved,
        content,
        truncated,
    }))
}
