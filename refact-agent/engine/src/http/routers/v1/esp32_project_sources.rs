use std::path::{Path, PathBuf};
use std::process::Command;

use axum::extract::{Multipart, Query};
use axum::Json;
use hyper::StatusCode;
use serde::{Deserialize, Serialize};

use crate::custom_error::ScratchError;
use crate::global_context::SharedGlobalContext;
use crate::tools::esp32_tools::esp32_path_resolve::{
    ensure_project_sources_directory, sanitize_source_upload_filename,
};

#[derive(Debug, Deserialize)]
pub struct ProjectSourcesListQuery {
    pub project_root: String,
}

#[derive(Debug, Serialize)]
pub struct ProjectSourceFile {
    pub name: String,
    pub size_bytes: u64,
}

#[derive(Debug, Serialize)]
pub struct ProjectSourcesListResponse {
    pub directory: String,
    pub files: Vec<ProjectSourceFile>,
}

/// GET: list files in `<project_root>/sources`.
pub async fn handle_v1_esp32_project_sources_list(
    axum::Extension(gcx): axum::Extension<SharedGlobalContext>,
    Query(q): Query<ProjectSourcesListQuery>,
) -> Result<Json<ProjectSourcesListResponse>, ScratchError> {
    let root = PathBuf::from(q.project_root.trim());
    let sources_dir = ensure_project_sources_directory(gcx.clone(), &root)
        .await
        .map_err(|e| ScratchError::new(StatusCode::BAD_REQUEST, e))?;

    let mut files = Vec::new();
    let rd = std::fs::read_dir(&sources_dir).map_err(|e| {
        ScratchError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("read_dir: {}", e),
        )
    })?;
    for ent in rd.flatten() {
        let meta = match ent.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        if !meta.is_file() {
            continue;
        }
        let name = ent.file_name().to_string_lossy().to_string();
        files.push(ProjectSourceFile {
            name,
            size_bytes: meta.len(),
        });
    }
    files.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(Json(ProjectSourcesListResponse {
        directory: sources_dir.to_string_lossy().to_string(),
        files,
    }))
}

#[derive(Debug, Serialize)]
pub struct ProjectSourcesUploadResponse {
    pub saved: Vec<String>,
    pub directory: String,
}

/// POST multipart: field `project_root` (text) + one or more `file` parts.
pub async fn handle_v1_esp32_project_sources_upload(
    axum::Extension(gcx): axum::Extension<SharedGlobalContext>,
    mut multipart: Multipart,
) -> Result<Json<ProjectSourcesUploadResponse>, ScratchError> {
    let mut project_root_str: Option<String> = None;
    let mut file_parts: Vec<(Option<String>, Vec<u8>)> = Vec::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ScratchError::new(StatusCode::BAD_REQUEST, format!("multipart: {}", e)))?
    {
        let name = field.name().map(|s| s.to_string()).unwrap_or_default();
        match name.as_str() {
            "project_root" => {
                let text = field
                    .text()
                    .await
                    .map_err(|e| ScratchError::new(StatusCode::BAD_REQUEST, format!("{}", e)))?;
                project_root_str = Some(text);
            }
            "file" => {
                let orig = field.file_name().map(|s| s.to_string());
                let data = field
                    .bytes()
                    .await
                    .map_err(|e| ScratchError::new(StatusCode::BAD_REQUEST, format!("{}", e)))?;
                file_parts.push((orig, data.to_vec()));
            }
            _ => {}
        }
    }

    let prs = project_root_str.ok_or_else(|| {
        ScratchError::new(
            StatusCode::BAD_REQUEST,
            "missing multipart field project_root".to_string(),
        )
    })?;

    if file_parts.is_empty() {
        return Err(ScratchError::new(
            StatusCode::BAD_REQUEST,
            "no file parts (use field name \"file\")".to_string(),
        ));
    }

    let root = PathBuf::from(prs.trim());
    let sources_dir = ensure_project_sources_directory(gcx, &root)
        .await
        .map_err(|e| ScratchError::new(StatusCode::BAD_REQUEST, e))?;

    let mut saved = Vec::new();
    for (orig_name, data) in file_parts {
        let fallback = "upload.bin".to_string();
        let oname = orig_name.unwrap_or_else(|| fallback.clone());
        let safe = sanitize_source_upload_filename(&oname)
            .map_err(|e| ScratchError::new(StatusCode::BAD_REQUEST, e))?;
        let dest = sources_dir.join(&safe);
        // Traversal guard: dest must remain inside sources_dir.
        if !dest.starts_with(&sources_dir) {
            return Err(ScratchError::new(
                StatusCode::BAD_REQUEST,
                "invalid path after join".to_string(),
            ));
        }
        tokio::fs::write(&dest, &data)
            .await
            .map_err(|e| {
                ScratchError::new(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("write {}: {}", dest.display(), e),
                )
            })?;
        saved.push(safe.to_string_lossy().to_string());
    }

    Ok(Json(ProjectSourcesUploadResponse {
        saved,
        directory: sources_dir.to_string_lossy().to_string(),
    }))
}

#[derive(Debug, Deserialize)]
pub struct OpenProjectSourcesRequest {
    pub project_root: String,
}

#[derive(Debug, Serialize)]
pub struct OpenProjectSourcesResponse {
    pub opened: String,
}

/// Open `<project_root>` in the OS file manager (Explorer, Finder, xdg-open).
pub async fn handle_v1_esp32_project_sources_open(
    axum::Extension(gcx): axum::Extension<SharedGlobalContext>,
    Json(body): Json<OpenProjectSourcesRequest>,
) -> Result<Json<OpenProjectSourcesResponse>, ScratchError> {
    let root = PathBuf::from(body.project_root.trim());
    // Validate that `project_root` is allowed and ensure `<project_root>/sources` exists.
    let sources_dir = ensure_project_sources_directory(gcx, &root)
        .await
        .map_err(|e| ScratchError::new(StatusCode::BAD_REQUEST, e))?;

    let project_dir = sources_dir.parent().unwrap_or(&sources_dir).to_path_buf();
    let path = project_dir.clone();
    tokio::task::spawn_blocking(move || open_path_in_os_file_manager(&path))
        .await
        .map_err(|e| {
            ScratchError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("task join: {}", e),
            )
        })?
        .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, e))?;

    Ok(Json(OpenProjectSourcesResponse {
        opened: project_dir.to_string_lossy().to_string(),
    }))
}

fn open_path_in_os_file_manager(path: &Path) -> Result<(), String> {
    if !path.is_dir() {
        return Err(format!("path is not a directory: {}", path.display()));
    }
    #[cfg(target_os = "windows")]
    {
        Command::new("explorer")
            .arg(path)
            .spawn()
            .map_err(|e| format!("failed to open folder in Explorer: {}", e))?;
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(path)
            .spawn()
            .map_err(|e| format!("failed to open folder in Finder: {}", e))?;
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map_err(|e| format!("failed to open folder (xdg-open): {}", e))?;
    }
    #[cfg(not(any(
        target_os = "windows",
        target_os = "macos",
        all(unix, not(target_os = "macos"))
    )))]
    {
        return Err("opening a folder in the file manager is not supported on this OS".to_string());
    }
    Ok(())
}
