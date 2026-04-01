//! Effective directory for ESP-IDF project creation (`esp32_project` create / idf.py cwd).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use tokio::sync::Mutex as AMutex;
use tokio::sync::RwLock as ARwLock;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::files_correction::{
    canonicalize_normalized_path, check_if_its_inside_a_workspace_or_config, get_project_dirs,
};
use crate::global_context::GlobalContext;

use super::config::ESP32Config;

/// Windows reserved device names that must not be used as file or folder names.
/// On Windows, CreateFile("NUL") opens the null device; "CON" opens the console, etc.
/// These names are also forbidden when followed by an extension (e.g. "NUL.bin").
const WINDOWS_RESERVED_NAMES: &[&str] = &[
    "CON", "PRN", "AUX", "NUL",
    "COM0", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9",
    "LPT0", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Returns `true` when `name` (case-insensitive) matches a Windows reserved device name,
/// optionally followed by a dot and extension (e.g. "nul.bin" is also reserved).
fn is_windows_reserved_name(name: &str) -> bool {
    let stem = name.split('.').next().unwrap_or(name);
    let upper = stem.to_uppercase();
    WINDOWS_RESERVED_NAMES.contains(&upper.as_str())
}

pub async fn resolve_esp32_projects_root(
    gcx: Arc<ARwLock<GlobalContext>>,
    ccx: &AMutex<AtCommandsContext>,
    config: &ESP32Config,
) -> Result<PathBuf, String> {
    let override_mb = {
        let cx = ccx.lock().await;
        cx.esp32_projects_path.clone()
    };

    let root = if let Some(p) = override_mb {
        if p.as_os_str().is_empty() {
            return Err("esp32_projects_path override is empty".to_string());
        }
        if !p.is_absolute() {
            return Err(format!(
                "esp32_projects_path must be an absolute path: {}",
                p.display()
            ));
        }

        let canon = canonicalize_normalized_path(p);
        let workspace_dirs = get_project_dirs(gcx.clone()).await;
        if !workspace_dirs.is_empty() {
            check_if_its_inside_a_workspace_or_config(gcx.clone(), &canon).await?;
        }

        canon
    } else {
        PathBuf::from(&config.projects_path)
    };

    if !root.exists() {
        std::fs::create_dir_all(&root).map_err(|e| {
            format!(
                "Failed to create ESP32 projects directory {}: {}",
                root.display(),
                e
            )
        })?;
    } else if !root.is_dir() {
        return Err(format!(
            "ESP32 projects path exists but is not a directory: {}",
            root.display()
        ));
    }

    Ok(root)
}

/// Sanitize a single path segment for a new folder under `parent_path`.
///
/// Rules:
/// - Only ASCII alphanumeric characters plus `._-` are allowed (blocks all Windows-forbidden chars).
/// - Windows reserved device names (`CON`, `NUL`, `COM1`, `LPT1`, …) are explicitly rejected
///   because they silently misbehave on Windows regardless of the allowed-character set.
/// - `.` and `..` are rejected.
/// - Length is capped at 128 characters.
pub fn sanitize_esp_workspace_folder_name(name: &str) -> Result<String, String> {
    let t = name.trim();
    if t.is_empty() {
        return Err("folder_name must not be empty".to_string());
    }
    if t.len() > 128 {
        return Err("folder_name must be at most 128 characters".to_string());
    }
    if t.contains('/') || t.contains('\\') || t.contains('\0') {
        return Err("folder_name must not contain path separators".to_string());
    }
    let ok = t
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.');
    if !ok {
        return Err(
            "folder_name may only contain ASCII letters, digits, ._-".to_string(),
        );
    }
    if t == "." || t == ".." {
        return Err("invalid folder_name".to_string());
    }
    // Reject Windows reserved device names (safe no-op on Linux/macOS).
    if is_windows_reserved_name(t) {
        return Err(format!(
            "folder_name '{}' is a reserved device name on Windows and cannot be used",
            t
        ));
    }
    Ok(t.to_string())
}

/// Create `parent_path / folder_name` after validating `parent_path` is allowed.
pub async fn create_esp_project_workspace_dir(
    gcx: Arc<ARwLock<GlobalContext>>,
    parent_path: &Path,
    folder_name: &str,
) -> Result<PathBuf, String> {
    if !parent_path.is_absolute() {
        return Err("parent_path must be absolute".to_string());
    }
    let parent_canon = canonicalize_normalized_path(parent_path.to_path_buf());
    if !parent_canon.is_dir() {
        return Err(format!(
            "parent_path is not a directory: {}",
            parent_canon.display()
        ));
    }

    let workspace_dirs = get_project_dirs(gcx.clone()).await;
    if !workspace_dirs.is_empty() {
        check_if_its_inside_a_workspace_or_config(gcx.clone(), &parent_canon).await?;
    }

    let seg = sanitize_esp_workspace_folder_name(folder_name)?;
    let full = parent_canon.join(&seg);

    if full.exists() {
        if full.is_dir() {
            return Ok(full);
        }
        return Err(format!(
            "Path exists and is not a directory: {}",
            full.display()
        ));
    }

    std::fs::create_dir_all(&full).map_err(|e| {
        format!(
            "Failed to create project workspace directory {}: {}",
            full.display(),
            e
        )
    })?;

    Ok(canonicalize_normalized_path(full))
}

/// Ensure `<project_root>/sources` exists. Validates `project_root` the same way as workspace project folders.
pub async fn ensure_project_sources_directory(
    gcx: Arc<ARwLock<GlobalContext>>,
    project_root: &Path,
) -> Result<PathBuf, String> {
    if !project_root.is_absolute() {
        return Err("project_root must be absolute".to_string());
    }
    let root = canonicalize_normalized_path(project_root.to_path_buf());
    if !root.is_dir() {
        return Err(format!(
            "project_root is not a directory: {}",
            root.display()
        ));
    }
    let workspace_dirs = get_project_dirs(gcx.clone()).await;
    if !workspace_dirs.is_empty() {
        check_if_its_inside_a_workspace_or_config(gcx.clone(), &root).await?;
    }
    let sources = root.join("sources");
    std::fs::create_dir_all(&sources).map_err(|e| {
        format!(
            "Failed to create sources directory {}: {}",
            sources.display(),
            e
        )
    })?;
    Ok(canonicalize_normalized_path(sources))
}

/// Sanitize an uploaded filename to a single path segment (no directory traversal).
///
/// Also rejects Windows reserved device names for the same reason as `sanitize_esp_workspace_folder_name`.
pub fn sanitize_source_upload_filename(original: &str) -> Result<std::ffi::OsString, String> {
    let p = Path::new(original);
    let file_name = p
        .file_name()
        .ok_or_else(|| "invalid filename".to_string())?;
    let s = file_name.to_string_lossy();
    if s.is_empty() || s == "." || s == ".." {
        return Err("invalid filename".to_string());
    }
    if s.contains('/') || s.contains('\\') || s.contains('\0') {
        return Err("invalid filename".to_string());
    }
    if s.len() > 255 {
        return Err("filename too long (max 255)".to_string());
    }
    // Reject Windows reserved device names (safe no-op on Linux/macOS).
    if is_windows_reserved_name(&s) {
        return Err(format!(
            "filename '{}' is a reserved device name on Windows and cannot be used",
            s
        ));
    }
    Ok(file_name.to_os_string())
}
