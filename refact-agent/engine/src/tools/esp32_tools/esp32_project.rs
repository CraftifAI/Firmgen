use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::Mutex as AMutex;

use crate::at_commands::at_commands::{AtCommandsContext, vec_context_file_to_context_tools};
use crate::at_commands::at_search::execute_at_search;
use crate::tools::tools_description::{Tool, ToolDesc, ToolParam, ToolSource, ToolSourceType};
use crate::call_validation::{ChatMessage, ChatContent, ContextEnum};

use super::config::ESP32Config;
use super::esp32_path_resolve::resolve_esp32_projects_root;
use super::output_protocol::ToolOutput;
use super::idf_command::IdfCommand;
use super::global_state::{get_config, generate_suggested_actions, SuggestionContext};

pub struct ESP32Project {
    pub config_path: String,
}

#[async_trait]
impl Tool for ESP32Project {
    fn as_any(&self) -> &dyn std::any::Any { self }

    fn tool_description(&self) -> ToolDesc {
        ToolDesc {
            name: "esp32_project".to_string(),
            display_name: "ESP32 Project".to_string(),
            source: ToolSource {
                source_type: ToolSourceType::Builtin,
                config_path: self.config_path.clone(),
            },
            agentic: true,
            experimental: false,
            description: "Manage ESP32 projects. Operations: 'create' (create new project from template/example), 'list_examples' (list available IDF examples), 'validate' (validate project structure). IMPORTANT: Do NOT guess the template path — wrong guesses waste turns. Real IDF v5.5 example paths: 'get-started/blink', 'get-started/hello_world', 'provisioning/wifi_prov_mgr', 'wifi/getting_started/station', 'bluetooth/bluedroid/ble/gatt_server', 'mqtt/tcp'. If unsure of the exact path, use list_examples with a 'filter' keyword (e.g., filter: 'wifi') BEFORE calling create. Do NOT fall back to shell commands (cp, mkdir, etc.) to create projects manually.".to_string(),
            parameters: vec![
                ToolParam {
                    name: "operation".to_string(),
                    param_type: "string".to_string(),
                    description: "Operation: 'create' (create new project from template/example), 'list_examples' (list available IDF examples with optional filter keyword), 'validate' (validate project structure)".to_string(),
                },
                ToolParam {
                    name: "project_name".to_string(),
                    param_type: "string".to_string(),
                    description: "Project name (required for 'create')".to_string(),
                },
                ToolParam {
                    name: "template".to_string(),
                    param_type: "string".to_string(),
                    description: "Template/example path relative to $IDF_PATH/examples (e.g., 'get-started/blink', 'provisioning/wifi_prov_mgr'). Search first if you are not sure of the exact path.".to_string(),
                },
                ToolParam {
                    name: "target".to_string(),
                    param_type: "string".to_string(),
                    description: "Target chip: esp32, esp32s3, esp32c3, esp32c6, esp32p4 (default: from config)".to_string(),
                },
                ToolParam {
                    name: "project_path".to_string(),
                    param_type: "string".to_string(),
                    description: "Path to project (for 'validate')".to_string(),
                },
                ToolParam {
                    name: "filter".to_string(),
                    param_type: "string".to_string(),
                    description: "Filter examples by keyword (for 'list_examples')".to_string(),
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
            "esp32_project",
            operation,
            args.clone(),
        ).await;

        // Use global config (cached, configurable via env var)
        let config = get_config().await?;
        config.validate_paths()?;

        let gcx = ccx.lock().await.global_context.clone();
        let projects_root = resolve_esp32_projects_root(gcx, ccx.as_ref(), &config).await?;

        let result = match operation {
            "create" => self
                .create_project(&config, &projects_root, args)
                .await
                .map(|o| (o, vec![])),
            "list_examples" => self.list_examples(&config, args).await.map(|o| (o, vec![])),
            "search_examples" => return Err(
                "search_examples requires VecDB which is not available. Use 'list_examples' with a 'filter' parameter instead (e.g., filter: 'wifi').".to_string()
            ),
            "validate" => self.validate_project(&config, args).await.map(|o| (o, vec![])),
            _ => return Err(format!("Unknown operation: '{}'. Valid operations: 'create', 'list_examples', 'validate'.", operation)),
        };

        match &result {
            Ok((output, _)) => {
                crate::progressbar::record_tool_complete(
                    &chat_id,
                    tool_call_id,
                    "esp32_project",
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
                    "esp32_project",
                    operation,
                    args.clone(),
                    invoked_at,
                    e,
                ).await;
            }
        }

        let (output, mut context_files) = result?;

        context_files.push(ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: ChatContent::SimpleText(output.to_llm_context()),
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
            ..Default::default()
        }));

        Ok((false, context_files))
    }

    fn tool_depends_on(&self) -> Vec<String> {
        vec!["esp32".to_string()]
    }
}

impl ESP32Project {
    async fn create_project(
        &self,
        config: &ESP32Config,
        projects_root: &std::path::Path,
        args: &HashMap<String, Value>,
    ) -> Result<ToolOutput, String> {
        let project_name = args.get("project_name")
            .and_then(|v| v.as_str())
            .ok_or("Missing required parameter: project_name for 'create' operation")?;

        let template = args.get("template")
            .and_then(|v| v.as_str())
            .unwrap_or("get-started/blink");

        let target = args.get("target")
            .and_then(|v| v.as_str())
            .unwrap_or(&config.default_target);

        let project_path = projects_root.join(project_name);

        // Ensure projects directory exists (no-op if already present)
        std::fs::create_dir_all(projects_root)
            .map_err(|e| format!("Failed to create projects directory: {}", e))?;

        // Check if project already exists
        if project_path.exists() {
            return Err(format!("Project '{}' already exists at {}", project_name, project_path.display()));
        }

        // If template contains '/', it's an example path - use create-project-from-example
        if template.contains('/') {
            let example_full_path = format!("examples/{}", template);

            let result = IdfCommand::new("create-project-from-example")
                .arg(&example_full_path)
                .project_path(projects_root)
                .timeout_secs(120)
                .execute(config).await?;

            if !result.success {
                // Fallback: portable Rust recursive copy (no cp -r — not available on Windows).
                let example_path = std::path::PathBuf::from(&config.esp_idf_path)
                    .join("examples")
                    .join(template);

                if example_path.exists() {
                    copy_dir_recursive(&example_path, &project_path)
                        .map_err(|e| format!("Failed to copy example: {}", e))?;
                } else {
                    return Err(format!(
                        "Example '{}' not found. Check available examples with list_examples operation.",
                        template
                    ));
                }
            }

            // Rename the created project folder if needed (create-project-from-example uses example name)
            let example_name = template.split('/').last().unwrap_or(template);
            let created_path = projects_root.join(example_name);
            if created_path.exists() && created_path != project_path {
                std::fs::rename(&created_path, &project_path)
                    .map_err(|e| format!("Failed to rename project: {}", e))?;
            }
        } else {
            // Create empty project with idf.py create-project
            let result = IdfCommand::new("create-project")
                .arg(project_name)
                .project_path(projects_root)
                .timeout_secs(60)
                .execute(config).await?;

            if !result.success {
                return Err(format!("Project creation failed: {}", result.stderr));
            }
        }

        // Set target chip using IdfCommand
        if project_path.exists() {
            let result = IdfCommand::new("set-target")
                .arg(target)
                .project_path(&project_path)
                .timeout_secs(120)
                .execute(config).await?;

            if !result.success {
                return Err(format!("Failed to set target '{}': {}", target, result.stderr));
            }
        }

        let suggested_actions = generate_suggested_actions(
            "create",
            true,
            &SuggestionContext::new(),
        );

        Ok(ToolOutput {
            status: super::output_protocol::ToolStatus::Success,
            action_taken: "create".to_string(),
            data: serde_json::json!({
                "project_name": project_name,
                "project_path": project_path.to_string_lossy(),
                "target": target,
                "template": template,
            }),
            summary: format!("Created project '{}' [{}] from {}", project_name, target, template),
            details: Some(format!("Project path: {}", project_path.display())),
            state_delta: super::session_state::StateDelta::none(),
            suggested_actions,
            error: None,
        })
    }

    async fn list_examples(&self, config: &ESP32Config, args: &HashMap<String, Value>) -> Result<ToolOutput, String> {
        let examples_path = std::path::PathBuf::from(&config.esp_idf_path).join("examples");

        if !examples_path.exists() {
            return Err(format!("ESP-IDF examples directory not found at: {}/examples", config.esp_idf_path));
        }

        let filter = args.get("filter")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let mut examples = Vec::new();
        self.find_examples_recursive(&examples_path, &examples_path, &mut examples, filter)?;
        examples.sort();

        // Group by category
        let mut categories: HashMap<String, Vec<String>> = HashMap::new();
        for example in &examples {
            // Use Path-aware splitting so results are correct on both Unix and Windows.
            let rel = std::path::Path::new(example);
            let mut comps = rel.components();
            if let Some(cat) = comps.next() {
                let category = cat.as_os_str().to_string_lossy().to_string();
                let rest: std::path::PathBuf = comps.collect();
                let name = rest.to_string_lossy().replace('\\', "/");
                categories.entry(category).or_insert_with(Vec::new).push(name);
            } else {
                categories.entry("other".to_string()).or_insert_with(Vec::new).push(example.clone());
            }
        }

        Ok(ToolOutput::success(
            format!("Found {} ESP-IDF examples in {} categories", examples.len(), categories.len()),
            serde_json::json!({
                "examples": examples,
                "categories": categories,
                "count": examples.len(),
                "filter": if filter.is_empty() { "none" } else { filter },
            }),
        ))
    }

    fn find_examples_recursive(
        &self,
        base_path: &std::path::Path,
        current_path: &std::path::Path,
        examples: &mut Vec<String>,
        filter: &str,
    ) -> Result<(), String> {
        let entries = std::fs::read_dir(current_path)
            .map_err(|e| format!("Failed to read directory: {}", e))?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if path.join("CMakeLists.txt").exists() && path.join("main").exists() {
                    if let Ok(relative) = path.strip_prefix(base_path) {
                        // Normalise separators to `/` so example paths are always forward-slash
                        // style (matching IDF convention) regardless of the host OS.
                        let example_path = relative.to_string_lossy().replace('\\', "/");
                        if filter.is_empty() || example_path.to_lowercase().contains(&filter.to_lowercase()) {
                            examples.push(example_path);
                        }
                    }
                }
                let dir_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if !dir_name.starts_with('.') && dir_name != "build" && dir_name != "managed_components" {
                    self.find_examples_recursive(base_path, &path, examples, filter)?;
                }
            }
        }
        Ok(())
    }

    async fn validate_project(&self, _config: &ESP32Config, args: &HashMap<String, Value>) -> Result<ToolOutput, String> {
        let project_path = args.get("project_path")
            .and_then(|v| v.as_str())
            .map(|s| std::path::PathBuf::from(s))
            .ok_or("Missing project_path parameter")?;

        if !project_path.exists() {
            return Err(format!("Project path does not exist: {}", project_path.display()));
        }

        let mut issues = Vec::new();
        let mut valid = true;

        if !project_path.join("CMakeLists.txt").exists() {
            issues.push("Missing CMakeLists.txt".to_string());
            valid = false;
        }
        if !project_path.join("main").exists() {
            issues.push("Missing main/ directory".to_string());
            valid = false;
        }
        if !project_path.join("sdkconfig").exists() {
            issues.push("Missing sdkconfig (run 'idf.py reconfigure')".to_string());
        }

        if valid {
            Ok(ToolOutput::success(
                format!("Project '{}' is valid", project_path.display()),
                serde_json::json!({
                    "valid": true,
                    "project_path": project_path.to_string_lossy(),
                }),
            ))
        } else {
            Ok(ToolOutput {
                status: super::output_protocol::ToolStatus::PartialSuccess,
                action_taken: "validate".to_string(),
                data: serde_json::json!({
                    "valid": false,
                    "issues": issues,
                }),
                summary: format!("Project has {} issues", issues.len()),
                details: Some(issues.join("\n")),
                state_delta: super::session_state::StateDelta::none(),
                suggested_actions: vec![],
                error: None,
            })
        }
    }

    async fn search_examples(
        &self,
        config: &ESP32Config,
        ccx: Arc<AMutex<AtCommandsContext>>,
        args: &HashMap<String, Value>,
    ) -> Result<(ToolOutput, Vec<ContextEnum>), String> {
        let query = args.get("query")
            .and_then(|v| v.as_str())
            .ok_or("Missing required parameter: query for 'search_examples' operation")?;

        let top_n = args.get("top_n")
            .and_then(|v| v.as_u64())
            .unwrap_or(10) as usize;

        {
            let mut ccx_locked = ccx.lock().await;
            ccx_locked.top_n = top_n;
        }

        let all_search_results = execute_at_search(ccx.clone(), &query.to_string(), None).await
            .map_err(|e| format!("VecDB search failed: {}. Ensure VecDB is enabled with --static-vecdb or --vecdb", e))?;

        // Filter to only ESP-IDF examples. Use PathBuf for robust prefix comparison on all OSes.
        let examples_path = std::path::PathBuf::from(&config.esp_idf_path).join("examples");
        let search_results: Vec<_> = all_search_results
            .into_iter()
            .filter(|result| {
                std::path::Path::new(&result.file_name).starts_with(&examples_path)
            })
            .collect();

        if search_results.is_empty() {
            return Ok((
                ToolOutput {
                    status: super::output_protocol::ToolStatus::PartialSuccess,
                    action_taken: "search_examples".to_string(),
                    data: serde_json::json!({
                        "query": query,
                        "results_count": 0,
                    }),
                    summary: format!("No examples found matching '{}'", query),
                    details: Some("Try a different query or check if ESP-IDF examples VecDB is loaded".to_string()),
                    state_delta: super::session_state::StateDelta::none(),
                    suggested_actions: vec![],
                    error: None,
                },
                vec![],
            ));
        }

        let mut example_groups: HashMap<String, Vec<f32>> = HashMap::new();
        for result in &search_results {
            example_groups
                .entry(result.file_name.clone())
                .or_insert_with(Vec::new)
                .push(result.usefulness);
        }

        let mut example_summary = Vec::new();
        for (example_path, usefulness_scores) in &example_groups {
            let best_score = usefulness_scores.iter()
                .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                .copied()
                .unwrap_or(0.0);
            example_summary.push(serde_json::json!({
                "example_path": example_path,
                "matches": usefulness_scores.len(),
                "best_score": best_score,
            }));
        }

        example_summary.sort_by(|a, b| {
            let score_a = a.get("best_score").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let score_b = b.get("best_score").and_then(|v| v.as_f64()).unwrap_or(0.0);
            score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
        });

        let context_files = vec_context_file_to_context_tools(search_results);

        Ok((
            ToolOutput::success(
                format!("Found {} relevant example(s) matching '{}'", example_groups.len(), query),
                serde_json::json!({
                    "query": query,
                    "results_count": example_groups.len(),
                    "total_matches": context_files.len(),
                    "examples": example_summary,
                }),
            ),
            context_files,
        ))
    }
}

/// Portable recursive directory copy — replaces the Unix-only `cp -r` shell invocation.
///
/// Handles regular files, directories, and symlinks (symlinks are followed and their
/// content is copied, which is what `cp -r` does by default on Linux/macOS).
fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> Result<(), std::io::Error> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else if ty.is_file() {
            std::fs::copy(&src_path, &dst_path)?;
        } else if ty.is_symlink() {
            let meta = std::fs::metadata(&src_path)?;
            if meta.is_dir() {
                copy_dir_recursive(&src_path, &dst_path)?;
            } else if meta.is_file() {
                std::fs::copy(&src_path, &dst_path)?;
            }
        }
    }
    Ok(())
}
