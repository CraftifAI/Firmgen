use std::path::{Path, PathBuf};
use std::process::Command;

use axum::extract::Query;
use axum::Json;
use hyper::StatusCode;
use serde::{Deserialize, Serialize};

use crate::custom_error::ScratchError;
use crate::global_context::SharedGlobalContext;
use crate::tools::esp32_tools::esp32_path_resolve::validate_esp32_agent_project_path;

const DEFAULT_MAX_DEPTH: u32 = 8;

#[derive(Debug, Deserialize)]
pub struct ProjectTreeListQuery {
    pub project_root: String,
    pub max_depth: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct ProjectTreeNode {
    pub name: String,
    pub path: String,
    #[serde(rename = "type")]
    pub node_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<ProjectTreeNode>>,
}

#[derive(Debug, Serialize)]
pub struct ProjectTreeListResponse {
    pub root: String,
    pub tree: Vec<ProjectTreeNode>,
}

/// GET: hierarchical file tree for `<project_root>` (ESP-IDF workspace folder for a chat).
pub async fn handle_v1_esp32_project_tree_list(
    axum::Extension(gcx): axum::Extension<SharedGlobalContext>,
    Query(q): Query<ProjectTreeListQuery>,
) -> Result<Json<ProjectTreeListResponse>, ScratchError> {
    let root = validate_esp32_agent_project_path(gcx.clone(), Path::new(q.project_root.trim()))
        .await
        .map_err(|e| ScratchError::new(StatusCode::BAD_REQUEST, e))?;

    let max_depth = q.max_depth.unwrap_or(DEFAULT_MAX_DEPTH).clamp(1, 20);
    let root_for_tree = root.clone();
    let tree = tokio::task::spawn_blocking(move || {
        build_project_tree(&root_for_tree, &root_for_tree, 0, max_depth)
    })
    .await
    .map_err(|e| {
        ScratchError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("project tree build task failed: {}", e),
        )
    })?;

    Ok(Json(ProjectTreeListResponse {
        root: root.to_string_lossy().to_string(),
        tree,
    }))
}

#[derive(Debug, Deserialize)]
pub struct OpenProjectTreeFileRequest {
    pub project_root: String,
    pub file_path: String,
}

#[derive(Debug, Serialize)]
pub struct OpenProjectTreeFileResponse {
    pub opened: String,
}

/// POST: open a file under `<project_root>` with the OS default application.
pub async fn handle_v1_esp32_project_tree_open(
    axum::Extension(gcx): axum::Extension<SharedGlobalContext>,
    Json(body): Json<OpenProjectTreeFileRequest>,
) -> Result<Json<OpenProjectTreeFileResponse>, ScratchError> {
    let root = validate_esp32_agent_project_path(gcx.clone(), Path::new(body.project_root.trim()))
        .await
        .map_err(|e| ScratchError::new(StatusCode::BAD_REQUEST, e))?;

    let file_path = PathBuf::from(body.file_path.trim());
    if !file_path.is_absolute() {
        return Err(ScratchError::new(
            StatusCode::BAD_REQUEST,
            "file_path must be absolute".to_string(),
        ));
    }
    if !file_path.starts_with(&root) {
        return Err(ScratchError::new(
            StatusCode::BAD_REQUEST,
            "file_path must be inside project_root".to_string(),
        ));
    }
    if !file_path.is_file() {
        return Err(ScratchError::new(
            StatusCode::BAD_REQUEST,
            format!("path is not a file: {}", file_path.display()),
        ));
    }

    let path = file_path.clone();
    tokio::task::spawn_blocking(move || open_path_with_os_default_app(&path))
        .await
        .map_err(|e| {
            ScratchError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("task join: {}", e),
            )
        })?
        .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, e))?;

    Ok(Json(OpenProjectTreeFileResponse {
        opened: file_path.to_string_lossy().to_string(),
    }))
}

#[derive(Debug, Deserialize)]
pub struct ChatProjectPathQuery {
    pub chat_id: String,
}

#[derive(Debug, Serialize)]
pub struct ChatProjectPathResponse {
    pub project_path: Option<String>,
}

/// GET: ESP-IDF project path created/used by the agent in this chat (from progress tool events).
pub async fn handle_v1_esp32_chat_project_path(
    Query(q): Query<ChatProjectPathQuery>,
) -> Result<Json<ChatProjectPathResponse>, ScratchError> {
    let chat_id = q.chat_id.trim();
    if chat_id.is_empty() {
        return Err(ScratchError::new(
            StatusCode::BAD_REQUEST,
            "chat_id is required".to_string(),
        ));
    }
    let project_path = crate::progressbar::esp32_project_path_for_chat(chat_id)
        .await
        .map(|p| p.to_string_lossy().to_string());
    Ok(Json(ChatProjectPathResponse { project_path }))
}

fn build_project_tree(
    project_root: &Path,
    current: &Path,
    depth: u32,
    max_depth: u32,
) -> Vec<ProjectTreeNode> {
    if depth >= max_depth {
        return Vec::new();
    }

    let rd = match std::fs::read_dir(current) {
        Ok(rd) => rd,
        Err(_) => return Vec::new(),
    };

    let mut entries: Vec<(PathBuf, bool)> = Vec::new();
    for ent in rd.flatten() {
        let path = ent.path();
        let name = ent.file_name().to_string_lossy().to_string();
        if should_exclude_entry_name(&name) {
            continue;
        }
        let is_dir = path.is_dir();
        entries.push((path, is_dir));
    }

    entries.sort_by(|(a_path, a_dir), (b_path, b_dir)| {
        match (a_dir, b_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a_path
                .file_name()
                .unwrap_or_default()
                .cmp(b_path.file_name().unwrap_or_default()),
        }
    });

    entries
        .into_iter()
        .filter_map(|(path, is_dir)| {
            let name = path.file_name()?.to_string_lossy().to_string();
            if is_dir {
                let children = build_project_tree(project_root, &path, depth + 1, max_depth);
                Some(ProjectTreeNode {
                    name: format!("{name}/"),
                    path: path.to_string_lossy().to_string(),
                    node_type: "dir".to_string(),
                    size_bytes: None,
                    children: if children.is_empty() {
                        Some(Vec::new())
                    } else {
                        Some(children)
                    },
                })
            } else {
                let size_bytes = std::fs::metadata(&path).ok().map(|m| m.len());
                Some(ProjectTreeNode {
                    name,
                    path: path.to_string_lossy().to_string(),
                    node_type: "file".to_string(),
                    size_bytes,
                    children: None,
                })
            }
        })
        .collect()
}

fn should_exclude_entry_name(name: &str) -> bool {
    const EXACT: &[&str] = &[
        ".git",
        "managed_components",
        "dependencies",
        "__pycache__",
        "node_modules",
        ".vscode",
        ".idea",
    ];
    if name.starts_with('.') {
        return true;
    }
    EXACT.iter().any(|ex| name.eq_ignore_ascii_case(ex))
}

fn open_path_with_os_default_app(path: &Path) -> Result<(), String> {
    if !path.is_file() {
        return Err(format!("path is not a file: {}", path.display()));
    }
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/C", "start", "", &path.to_string_lossy()])
            .spawn()
            .map_err(|e| format!("failed to open file: {}", e))?;
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(path)
            .spawn()
            .map_err(|e| format!("failed to open file: {}", e))?;
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map_err(|e| format!("failed to open file: {}", e))?;
    }
    #[cfg(not(any(
        target_os = "windows",
        target_os = "macos",
        all(unix, not(target_os = "macos"))
    )))]
    {
        return Err("opening a file with the OS default app is not supported on this OS".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::should_exclude_entry_name;

    #[test]
    fn excludes_hidden_and_vendor_dirs() {
        assert!(should_exclude_entry_name(".git"));
        assert!(should_exclude_entry_name("managed_components"));
        assert!(should_exclude_entry_name(".cache"));
    }

    #[test]
    fn includes_build_and_normal_project_entries() {
        assert!(!should_exclude_entry_name("build"));
        assert!(!should_exclude_entry_name("main"));
        assert!(!should_exclude_entry_name("CMakeLists.txt"));
        assert!(!should_exclude_entry_name("sources"));
    }
}
