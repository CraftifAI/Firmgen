use std::sync::Arc;
use std::path::PathBuf;
use tokio::sync::RwLock as ARwLock;
use tokio::fs;

use axum::Extension;
use axum::response::Result;
use hyper::{Body, Response, StatusCode};

use crate::custom_error::ScratchError;
use crate::global_context::GlobalContext;
use crate::files_correction::get_project_dirs;

pub async fn handle_v1_c2000_config(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
) -> Result<Response<Body>, ScratchError> {
    // Try to read the C2000 config file from the standard location
    let config_path = "/home/shubham/.cache/refact/c2000_tools.yaml";
    
    match fs::read_to_string(config_path).await {
        Ok(config_content) => {
            // Parse the YAML content
            match serde_yaml::from_str::<serde_json::Value>(&config_content) {
                Ok(config_json) => {
                    // Add c2000ware_path to workspace folders for read access
                    // This allows other C2000 tools to read from SDK paths (projectspec templates,
                    // sysconfig files, driverlib references, etc.)
                    // Note: c2000_example_list still only searches in original project directories
                    if let Some(c2000_config) = config_json.get("c2000_config") {
                        if let Some(c2000ware_path) = c2000_config.get("c2000ware_path").and_then(|v| v.as_str()) {
                            let c2000ware_pathbuf = PathBuf::from(c2000ware_path);
                            if c2000ware_pathbuf.exists() {
                                let current_dirs = get_project_dirs(gcx.clone()).await;
                                let already_exists = current_dirs.iter().any(|dir| {
                                    dir == &c2000ware_pathbuf || 
                                    c2000ware_pathbuf.starts_with(dir)
                                });
                                if !already_exists {
                                    crate::files_in_workspace::add_folder(gcx.clone(), &c2000ware_pathbuf).await;
                                    tracing::info!("Added C2000Ware path to workspace folders for read access: {}", c2000ware_path);
                                }
                            }
                        }
                    }
                    
                    let body = serde_json::to_string_pretty(&config_json).unwrap();
                    let response = Response::builder()
                        .header("Content-Type", "application/json")
                        .body(Body::from(body))
                        .unwrap();
                    Ok(response)
                }
                Err(e) => {
                    Err(ScratchError::new(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Error parsing C2000 config YAML: {}", e),
                    ))
                }
            }
        }
        Err(e) => {
            Err(ScratchError::new(
                StatusCode::NOT_FOUND,
                format!("C2000 config file not found at {}: {}", config_path, e),
            ))
        }
    }
}











