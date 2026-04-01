use std::sync::Arc;
use tokio::sync::RwLock as ARwLock;
use tokio::fs;

use axum::Extension;
use axum::response::Result;
use hyper::{Body, Response, StatusCode};

use crate::custom_error::ScratchError;
use crate::global_context::GlobalContext;

pub async fn handle_v1_esp32_config(
    Extension(_gcx): Extension<Arc<ARwLock<GlobalContext>>>,
) -> Result<Response<Body>, ScratchError> {
    // Try to read the ESP32 config file from the standard location
    let home_dir = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let config_path = format!("{}/.cache/refact/esp32_tools.yaml", home_dir);
    
    match fs::read_to_string(&config_path).await {
        Ok(config_content) => {
            // Parse the YAML content
            match serde_yaml::from_str::<serde_json::Value>(&config_content) {
                Ok(config_json) => {
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
                        format!("Error parsing ESP32 config YAML: {}", e),
                    ))
                }
            }
        }
        Err(e) => {
            Err(ScratchError::new(
                StatusCode::NOT_FOUND,
                format!("ESP32 config file not found at {}: {}", config_path, e),
            ))
        }
    }
}

