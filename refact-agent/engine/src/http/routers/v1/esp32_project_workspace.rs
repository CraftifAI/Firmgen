use std::path::PathBuf;

use axum::Extension;
use hyper::{Body, Response, StatusCode};
use serde::{Deserialize, Serialize};

use crate::custom_error::ScratchError;
use crate::global_context::SharedGlobalContext;
use crate::tools::esp32_tools::esp32_path_resolve::create_esp_project_workspace_dir;

#[derive(Debug, Deserialize)]
pub struct CreateEsp32ProjectWorkspaceRequest {
    pub parent_path: String,
    pub folder_name: String,
}

#[derive(Debug, Serialize)]
pub struct CreateEsp32ProjectWorkspaceResponse {
    pub path: String,
}

/// Creates `parent_path/folder_name` on disk (mkdir -p) for use as `meta.esp32_projects_path` in chat.
pub async fn handle_v1_esp32_create_project_workspace(
    Extension(gcx): Extension<SharedGlobalContext>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let req: CreateEsp32ProjectWorkspaceRequest =
        serde_json::from_slice(&body_bytes).map_err(|e| {
            ScratchError::new(
                StatusCode::BAD_REQUEST,
                format!("JSON problem: {}", e),
            )
        })?;

    let parent = PathBuf::from(req.parent_path.trim());
    let created = create_esp_project_workspace_dir(gcx.clone(), &parent, &req.folder_name)
        .await
        .map_err(|e| ScratchError::new(StatusCode::BAD_REQUEST, e))?;

    let resp = CreateEsp32ProjectWorkspaceResponse {
        path: created.to_string_lossy().to_string(),
    };
    let json = serde_json::to_string(&resp).map_err(|e| {
        ScratchError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("JSON problem: {}", e),
        )
    })?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(json))
        .unwrap())
}
