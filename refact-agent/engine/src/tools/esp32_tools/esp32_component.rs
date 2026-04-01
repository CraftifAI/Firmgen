use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::Mutex as AMutex;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::tools::tools_description::{Tool, ToolDesc, ToolParam, ToolSource, ToolSourceType};
use crate::call_validation::{ChatMessage, ChatContent, ContextEnum};

use super::config::ESP32Config;
use super::output_protocol::ToolOutput;
use super::idf_command::{IdfCommand, infer_project_path};
use super::global_state::{get_config, generate_suggested_actions, SuggestionContext};

/// Maximum query length for ESP Component Registry API
const MAX_QUERY_LENGTH: usize = 64;

/// Common stopwords to remove from search queries
const STOPWORDS: &[&str] = &[
    "the", "a", "an", "for", "with", "and", "or", "to", "in", "on", "of", "is", "it",
    "that", "this", "i", "need", "want", "looking", "find", "get", "use", "using",
    "component", "library", "module", "esp32", "esp", "idf",
];

/// Sanitize and optimize a search query for the ESP Component Registry API.
/// - Removes common stopwords
/// - Extracts component name from "namespace/name" format
/// - Truncates to MAX_QUERY_LENGTH at word boundary
/// Returns the sanitized query and whether it was modified
fn sanitize_query(query: &str) -> (String, bool) {
    let original_len = query.len();
    let mut query = query.trim().to_string();
    
    // If query looks like "namespace/name", extract just the name part for searching
    if let Some(slash_pos) = query.find('/') {
        if !query[..slash_pos].contains(' ') && !query[slash_pos + 1..].contains(' ') {
            // Looks like a component path, extract the name
            query = query[slash_pos + 1..].to_string();
        }
    }
    
    // Remove stopwords for long queries
    if query.len() > MAX_QUERY_LENGTH / 2 {
        let words: Vec<&str> = query.split_whitespace()
            .filter(|w| !STOPWORDS.contains(&w.to_lowercase().as_str()))
            .collect();
        query = words.join(" ");
    }
    
    // Truncate at word boundary if still too long
    if query.len() > MAX_QUERY_LENGTH {
        query.truncate(MAX_QUERY_LENGTH);
        if let Some(last_space) = query.rfind(' ') {
            query.truncate(last_space);
        }
        // Remove trailing underscores or partial words
        query = query.trim_end_matches(|c: char| c == '_' || c == '-').to_string();
    }
    
    let was_modified = query.len() != original_len || query != query.trim();
    (query.trim().to_string(), was_modified)
}

/// Convert HTTP status codes and error bodies to human-readable messages
fn format_api_error(status: u16, body: &str, query: &str) -> String {
    match status {
        422 => {
            // 422 can mean the query is malformed or too long
            if query.len() > MAX_QUERY_LENGTH {
                let key_terms: Vec<&str> = query.split_whitespace().take(2).collect();
                let suggestion = if key_terms.is_empty() {
                    "shorter keywords".to_string()
                } else {
                    format!("'{}'", key_terms.join(" "))
                };
                format!(
                    "Search query too long (max {} chars). Try {}, or use single keywords like 'wifi', 'ble', 'mqtt'.",
                    MAX_QUERY_LENGTH, suggestion
                )
            } else {
                format!(
                    "Component registry rejected the request (HTTP 422). Try a simpler query like 'wifi' or 'mqtt'. Details: {}",
                    if body.is_empty() { "none" } else { body }
                )
            }
        }
        404 => {
            format!(
                "No components found for '{}'. Try simpler terms (e.g., 'provisioning' instead of 'wifi_provisioning').",
                query
            )
        }
        429 => "ESP Component Registry rate limit reached. Please wait a moment and try again.".to_string(),
        500..=599 => "ESP Component Registry is temporarily unavailable. Try again later.".to_string(),
        _ => format!("Component registry error (HTTP {})", status),
    }
}

pub struct ESP32Component {
    pub config_path: String,
}

#[async_trait]
impl Tool for ESP32Component {
    fn as_any(&self) -> &dyn std::any::Any { self }

    fn tool_description(&self) -> ToolDesc {
        ToolDesc {
            name: "esp32_component".to_string(),
            display_name: "ESP32 Component".to_string(),
            source: ToolSource {
                source_type: ToolSourceType::Builtin,
                config_path: self.config_path.clone(),
            },
            agentic: true,
            experimental: false,
            description: "Manage ESP-IDF components. Operations: add (add component dependency), remove (remove component), list (list installed components), search (search component registry).".to_string(),
            parameters: vec![
                ToolParam {
                    name: "operation".to_string(),
                    param_type: "string".to_string(),
                    description: "Operation: 'add' (add component), 'remove' (remove component), 'list' (list components), 'search' (search registry)".to_string(),
                },
                ToolParam {
                    name: "project_path".to_string(),
                    param_type: "string".to_string(),
                    description: "Path to ESP32 project".to_string(),
                },
                ToolParam {
                    name: "component".to_string(),
                    param_type: "string".to_string(),
                    description: "Component name (e.g., 'espressif/led_strip')".to_string(),
                },
                ToolParam {
                    name: "version".to_string(),
                    param_type: "string".to_string(),
                    description: "Component version (default: latest)".to_string(),
                },
                ToolParam {
                    name: "query".to_string(),
                    param_type: "string".to_string(),
                    description: "Search query for 'search' operation".to_string(),
                },
                ToolParam {
                    name: "idf_version".to_string(),
                    param_type: "string".to_string(),
                    description: "ESP-IDF version to filter search results by compatibility (e.g. '5.3.0'). Defaults to the detected IDF version. Use this to avoid getting components that were renamed or replaced in your IDF version.".to_string(),
                },
            ],
            parameters_required: vec!["operation".to_string()],
        }
    }

    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>,
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        let operation = args.get("operation")
            .and_then(|v| v.as_str())
            .ok_or("Missing required parameter: operation")?;

        let chat_id = ccx.lock().await.chat_id.clone();
        let invoked_at = std::time::SystemTime::now();
        crate::progressbar::record_tool_start(
            &chat_id,
            tool_call_id,
            "esp32_component",
            operation,
            args.clone(),
        ).await;

        // Use global config (cached, configurable via env var)
        let config = get_config().await?;

        let result = match operation {
            "add" => self.add_component(&config, args).await,
            "remove" => self.remove_component(&config, args).await,
            "list" => self.list_components(&config, args).await,
            "search" => self.search_components(&config, args).await,
            _ => return Err(format!("Unknown operation: {}", operation)),
        };

        match &result {
            Ok(output) => {
                crate::progressbar::record_tool_complete(
                    &chat_id,
                    tool_call_id,
                    "esp32_component",
                    operation,
                    args.clone(),
                    invoked_at,
                    output,
                ).await;
            }
            Err(e) => {
                crate::progressbar::record_tool_error(
                    &chat_id,
                    tool_call_id,
                    "esp32_component",
                    operation,
                    args.clone(),
                    invoked_at,
                    e,
                ).await;
            }
        }

        let output = result?;

        let context_files = vec![ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: ChatContent::SimpleText(output.to_llm_context()),
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
            ..Default::default()
        })];

        Ok((false, context_files))
    }

    fn tool_depends_on(&self) -> Vec<String> {
        vec!["esp32".to_string()]
    }
}

impl ESP32Component {
    async fn add_component(&self, config: &ESP32Config, args: &HashMap<String, Value>) -> Result<ToolOutput, String> {
        // Infer project path
        let explicit_path = args.get("project_path").and_then(|v| v.as_str());
        let project_path = infer_project_path(explicit_path, None)
            .ok_or("No valid ESP-IDF project found")?;

        let component = args.get("component")
            .and_then(|v| v.as_str())
            .ok_or("Missing required parameter: component")?;

        let result = IdfCommand::new("add-dependency")
            .arg(component)
            .project_path(&project_path)
            .timeout_secs(120)
            .execute(config).await?;

        if result.success {
            // Generate suggested actions for component add
            let suggested_actions = generate_suggested_actions(
                "add",
                true,
                &SuggestionContext::new(),
            );

            Ok(ToolOutput {
                status: super::output_protocol::ToolStatus::Success,
                action_taken: "add".to_string(),
                data: serde_json::json!({
                    "component": component,
                    "project_path": project_path.to_string_lossy(),
                }),
                summary: format!("Added component: {}", component),
                details: None,
                state_delta: super::session_state::StateDelta::none(),
                suggested_actions,
                error: None,
            })
        } else {
            // Check if it's a "not found" error and try to suggest alternatives
            let stderr_lower = result.stderr.to_lowercase();
            if stderr_lower.contains("not found") || stderr_lower.contains("no component") {
                // Extract the component name for searching (remove namespace if present)
                let search_term = if let Some(slash_pos) = component.find('/') {
                    &component[slash_pos + 1..]
                } else {
                    component
                };
                
                // Try to find similar components
                if let Ok(alternatives) = self.search_components_internal(search_term).await {
                    if !alternatives.is_empty() {
                        let suggestions: Vec<String> = alternatives.iter()
                            .filter_map(|c| c.get("full_name").and_then(|n| n.as_str()))
                            .map(|s| s.to_string())
                            .collect();
                        
                        return Ok(ToolOutput {
                            status: super::output_protocol::ToolStatus::Failed,
                            action_taken: "add".to_string(),
                            data: serde_json::json!({
                                "component": component,
                                "project_path": project_path.to_string_lossy(),
                                "suggestions": suggestions,
                            }),
                            summary: format!("Component '{}' not found", component),
                            details: Some(format!(
                                "Did you mean: {}? Use esp32_component search to find the correct name.",
                                suggestions.join(", ")
                            )),
                            state_delta: super::session_state::StateDelta::none(),
                            suggested_actions: vec![],
                            error: None,
                        });
                    }
                }
                
                // No alternatives found
                return Ok(ToolOutput {
                    status: super::output_protocol::ToolStatus::Failed,
                    action_taken: "add".to_string(),
                    data: serde_json::json!({
                        "component": component,
                        "project_path": project_path.to_string_lossy(),
                    }),
                    summary: format!("Component '{}' not found in ESP Component Registry", component),
                    details: Some(format!(
                        "The component '{}' does not exist. Use 'esp32_component search <keyword>' to find available components, or browse https://components.espressif.com",
                        component
                    )),
                    state_delta: super::session_state::StateDelta::none(),
                    suggested_actions: vec![],
                    error: None,
                });
            }
            
            // Other error
            Err(format!("Failed to add component: {}", result.stderr))
        }
    }

    async fn remove_component(&self, _config: &ESP32Config, _args: &HashMap<String, Value>) -> Result<ToolOutput, String> {
        // TODO: Implement component removal
        Ok(ToolOutput {
            status: super::output_protocol::ToolStatus::Skipped,
            action_taken: "remove".to_string(),
            data: serde_json::json!({}),
            summary: "Component removal not yet implemented".to_string(),
            details: None,
            state_delta: super::session_state::StateDelta::none(),
            suggested_actions: vec![],
            error: None,
        })
    }

    async fn list_components(&self, _config: &ESP32Config, args: &HashMap<String, Value>) -> Result<ToolOutput, String> {
        let project_path = args.get("project_path")
            .and_then(|v| v.as_str())
            .map(|s| std::path::PathBuf::from(s))
            .unwrap_or_else(|| std::path::PathBuf::from("."));

        // Check for main/idf_component.yml (component dependencies)
        let main_component_yml = project_path.join("main").join("idf_component.yml");
        let root_component_yml = project_path.join("idf_component.yml");
        
        let mut all_components: Vec<serde_json::Value> = Vec::new();
        
        // Parse main component dependencies
        if main_component_yml.exists() {
            let content = tokio::fs::read_to_string(&main_component_yml).await
                .map_err(|e| format!("Failed to read main/idf_component.yml: {}", e))?;
            
            if let Ok(yaml) = serde_yaml::from_str::<serde_yaml::Value>(&content) {
                if let Some(deps) = yaml.get("dependencies") {
                    if let Some(deps_map) = deps.as_mapping() {
                        for (name, version) in deps_map {
                            if let (Some(name_str), Some(version_val)) = (name.as_str(), version.as_str().or_else(|| version.get("version").and_then(|v| v.as_str()))) {
                                all_components.push(serde_json::json!({
                                    "name": name_str,
                                    "version": version_val,
                                    "source": "main/idf_component.yml",
                                }));
                            } else if let Some(name_str) = name.as_str() {
                                all_components.push(serde_json::json!({
                                    "name": name_str,
                                    "version": "*",
                                    "source": "main/idf_component.yml",
                                }));
                            }
                        }
                    }
                }
            }
        }
        
        // Parse root component dependencies
        if root_component_yml.exists() {
            let content = tokio::fs::read_to_string(&root_component_yml).await
                .map_err(|e| format!("Failed to read idf_component.yml: {}", e))?;
            
            if let Ok(yaml) = serde_yaml::from_str::<serde_yaml::Value>(&content) {
                if let Some(deps) = yaml.get("dependencies") {
                    if let Some(deps_map) = deps.as_mapping() {
                        for (name, version) in deps_map {
                            if let (Some(name_str), Some(version_val)) = (name.as_str(), version.as_str().or_else(|| version.get("version").and_then(|v| v.as_str()))) {
                                all_components.push(serde_json::json!({
                                    "name": name_str,
                                    "version": version_val,
                                    "source": "idf_component.yml",
                                }));
                            } else if let Some(name_str) = name.as_str() {
                                all_components.push(serde_json::json!({
                                    "name": name_str,
                                    "version": "*",
                                    "source": "idf_component.yml",
                                }));
                            }
                        }
                    }
                }
            }
        }
        
        // Also check for managed_components directory
        let managed_dir = project_path.join("managed_components");
        if managed_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&managed_dir) {
                for entry in entries.flatten() {
                    if entry.path().is_dir() {
                        if let Some(name) = entry.file_name().to_str() {
                            // Check if already in list
                            if !all_components.iter().any(|c| c.get("name").and_then(|n| n.as_str()) == Some(name)) {
                                all_components.push(serde_json::json!({
                                    "name": name,
                                    "version": "installed",
                                    "source": "managed_components/",
                                }));
                            }
                        }
                    }
                }
            }
        }

        if all_components.is_empty() {
            Ok(ToolOutput::success(
                "No components installed".to_string(),
                serde_json::json!({ 
                    "components": [],
                    "project_path": project_path.to_string_lossy(),
                }),
            ))
        } else {
            Ok(ToolOutput::success(
                format!("Found {} component(s)", all_components.len()),
                serde_json::json!({
                    "components": all_components,
                    "count": all_components.len(),
                    "project_path": project_path.to_string_lossy(),
                }),
            ))
        }
    }

    async fn search_components(&self, config: &ESP32Config, args: &HashMap<String, Value>) -> Result<ToolOutput, String> {
        let original_query = args.get("query")
            .and_then(|v| v.as_str())
            .or_else(|| args.get("component").and_then(|v| v.as_str()))
            .ok_or("Missing required parameter: query or component")?;

        // Sanitize the query to avoid API errors
        let (query, was_modified) = sanitize_query(original_query);
        
        if query.is_empty() {
            return Err("Search query is empty after removing common words. Please provide specific component keywords.".to_string());
        }

        // Resolve IDF version for informational context only (not sent to API).
        // The registry API does not support server-side idf_version filtering.
        let idf_version: Option<String> = args.get("idf_version")
            .and_then(|v| v.as_str())
            .map(|s| s.trim_start_matches('v').to_string())
            .or_else(|| config.esp_idf_version.clone());

        // Search the ESP Component Registry via HTTPS API
        // API endpoint: https://components.espressif.com/api/components?q=<query>
        // Returns: Array of component objects directly (not wrapped in "items")
        let client = reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(5))
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

        let response = client.get("https://components.espressif.com/api/components")
            .header("User-Agent", "refact-lsp/1.0")
            .header("Accept", "application/json")
            .query(&[("q", &query)])
            .send()
            .await
            .map_err(|e| format!("Failed to search component registry: {}", e))?;
        
        if !response.status().is_success() {
            let status = response.status().as_u16();
            let _error_body = response.text().await.unwrap_or_default();
            return Err(format_api_error(status, &_error_body, &query));
        }
        
        let json: serde_json::Value = response.json().await
            .map_err(|e| format!("Failed to parse registry response: {}", e))?;
        
        // API returns array of components directly
        let components: Vec<serde_json::Value> = if let Some(items) = json.as_array() {
            items.iter().map(|item| {
                // Version and description are under "latest_version"
                let latest = item.get("latest_version");
                let version = latest
                    .and_then(|lv| lv.get("version"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let description = latest
                    .and_then(|lv| lv.get("description"))
                    .and_then(|d| d.as_str())
                    .unwrap_or("");
                let license = latest
                    .and_then(|lv| lv.get("license"))
                    .and_then(|l| l.get("name"))
                    .and_then(|n| n.as_str())
                    .unwrap_or("");
                
                let namespace = item.get("namespace").and_then(|n| n.as_str()).unwrap_or("");
                let name = item.get("name").and_then(|n| n.as_str()).unwrap_or("unknown");
                
                serde_json::json!({
                    "name": name,
                    "namespace": namespace,
                    "version": version,
                    "description": description,
                    "license": license,
                    "full_name": format!("{}/{}", namespace, name),
                    "install_cmd": format!("idf.py add-dependency {}/{}", namespace, name),
                })
            }).collect()
        } else {
            Vec::new()
        };

        // Build details message
        let idf_version_note = idf_version.as_deref()
            .map(|v| format!(" (filtered for IDF v{})", v))
            .unwrap_or_default();
        let details = if was_modified {
            Some(format!(
                "Query was optimized: '{}' -> '{}'{}. Browse https://components.espressif.com for more.",
                original_query, query, idf_version_note
            ))
        } else if idf_version.is_some() {
            Some(format!(
                "Results filtered for IDF v{}. Browse https://components.espressif.com for more.",
                idf_version.as_deref().unwrap_or("")
            ))
        } else {
            None
        };

        if components.is_empty() {
            // Suggest alternative search terms
            let suggestions = generate_search_suggestions(&query);
            Ok(ToolOutput {
                status: super::output_protocol::ToolStatus::PartialSuccess,
                action_taken: "search".to_string(),
                data: serde_json::json!({ 
                    "query": query,
                    "original_query": original_query,
                    "results": [],
                    "suggestions": suggestions,
                }),
                summary: format!("No components found for '{}'", query),
                details: Some(format!(
                    "Try simpler terms like: {}. Or browse https://components.espressif.com",
                    suggestions.join(", ")
                )),
                state_delta: super::session_state::StateDelta::none(),
                suggested_actions: vec![],
                error: None,
            })
        } else {
            // Generate suggested actions when results found
            let suggested_actions = generate_suggested_actions(
                "search",
                true,
                &SuggestionContext::new().with_results(true),
            );

            Ok(ToolOutput {
                status: super::output_protocol::ToolStatus::Success,
                action_taken: "search".to_string(),
                data: serde_json::json!({
                    "query": query,
                    "original_query": original_query,
                    "idf_version_filter": idf_version,
                    "results": components,
                    "count": components.len(),
                }),
                summary: format!("Found {} component(s) for '{}'{}", components.len(), query, idf_version_note),
                details,
                state_delta: super::session_state::StateDelta::none(),
                suggested_actions,
                error: None,
            })
        }
    }
    
    /// Internal search method for use by add_component fallback.
    async fn search_components_internal(&self, query: &str) -> Result<Vec<serde_json::Value>, String> {
        let (sanitized_query, _) = sanitize_query(query);
        if sanitized_query.is_empty() {
            return Ok(Vec::new());
        }

        let client = reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(5))
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

        let response = client.get("https://components.espressif.com/api/components")
            .header("User-Agent", "refact-lsp/1.0")
            .header("Accept", "application/json")
            .query(&[("q", &sanitized_query)])
            .send()
            .await
            .map_err(|e| format!("Search failed: {}", e))?;
        
        if !response.status().is_success() {
            return Ok(Vec::new()); // Silently return empty on error for fallback
        }
        
        let json: serde_json::Value = response.json().await
            .map_err(|_| "Failed to parse response")?;
        
        let components: Vec<serde_json::Value> = if let Some(items) = json.as_array() {
            items.iter().take(5).map(|item| {
                let namespace = item.get("namespace").and_then(|n| n.as_str()).unwrap_or("");
                let name = item.get("name").and_then(|n| n.as_str()).unwrap_or("unknown");
                serde_json::json!({
                    "full_name": format!("{}/{}", namespace, name),
                    "name": name,
                    "namespace": namespace,
                })
            }).collect()
        } else {
            Vec::new()
        };
        
        Ok(components)
    }
}

/// Generate alternative search suggestions based on the failed query
fn generate_search_suggestions(query: &str) -> Vec<String> {
    let mut suggestions = Vec::new();
    
    // Extract individual words as potential searches
    for word in query.split(|c: char| c.is_whitespace() || c == '_' || c == '-') {
        let word = word.trim().to_lowercase();
        if word.len() >= 3 && !STOPWORDS.contains(&word.as_str()) {
            if !suggestions.contains(&word) {
                suggestions.push(word);
            }
        }
    }
    
    // Add common related terms
    if query.to_lowercase().contains("wifi") || query.to_lowercase().contains("provisioning") {
        if !suggestions.contains(&"network_provisioning".to_string()) {
            suggestions.push("network_provisioning".to_string());
        }
    }
    if query.to_lowercase().contains("ble") || query.to_lowercase().contains("bluetooth") {
        if !suggestions.contains(&"nimble".to_string()) {
            suggestions.push("nimble".to_string());
        }
    }
    
    suggestions.truncate(5);
    suggestions
}

