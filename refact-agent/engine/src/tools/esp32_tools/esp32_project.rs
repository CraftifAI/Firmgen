use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::Mutex as AMutex;

use crate::at_commands::at_commands::{AtCommandsContext, vec_context_file_to_context_tools};
use crate::at_commands::at_search::execute_at_search;
use crate::tools::tools_description::{Tool, ToolDesc, ToolParam, ToolSource, ToolSourceType};
use crate::call_validation::{ChatMessage, ChatContent, ContextEnum};

use super::config::ESP32Config;
use super::esp32_path_resolve::{resolve_esp32_projects_root, sanitize_esp_workspace_folder_name};
use super::output_protocol::{ToolOutput, ToolStatus};
use super::idf_command::{IdfCommand, is_esp_idf_project};
use super::global_state::{get_config, generate_suggested_actions, SuggestionContext};

const MAX_AUTO_SUFFIX: u32 = 99;
const DEFAULT_IF_EXISTS: &str = "auto_suffix";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IfExistsPolicy {
    Fail,
    Use,
    Replace,
    AutoSuffix,
}

impl IfExistsPolicy {
    fn parse(s: &str) -> Result<Self, String> {
        match s.trim().to_lowercase().as_str() {
            "fail" => Ok(Self::Fail),
            "use" | "use_existing" => Ok(Self::Use),
            "replace" | "delete" | "recreate" => Ok(Self::Replace),
            "auto_suffix" | "auto" | "suffix" => Ok(Self::AutoSuffix),
            other => Err(format!(
                "Unknown if_exists policy '{}'. Valid: fail, use, replace, auto_suffix",
                other
            )),
        }
    }
}

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
            description: "Manage ESP32 projects. Operations: 'create' (create from template/example), 'list_projects' (list existing projects in the workspace folder), 'list_examples' (list IDF examples), 'validate' (validate project structure). Before create, call list_projects when the workspace may already contain apps. Default if_exists is auto_suffix (picks a free name). Real IDF v5.5 example paths: 'get-started/blink', 'get-started/hello_world', 'provisioning/wifi_prov_mgr', 'wifi/getting_started/station', 'bluetooth/bluedroid/ble/gatt_server', 'mqtt/tcp'. If unsure of the exact path, use list_examples with a 'filter' keyword BEFORE create. Do NOT fall back to shell commands (cp, mkdir, etc.) to create projects manually.".to_string(),
            parameters: vec![
                ToolParam {
                    name: "operation".to_string(),
                    param_type: "string".to_string(),
                    description: "Operation: 'create', 'list_projects', 'list_examples', 'validate'".to_string(),
                },
                ToolParam {
                    name: "project_name".to_string(),
                    param_type: "string".to_string(),
                    description: "Project name (required for 'create')".to_string(),
                },
                ToolParam {
                    name: "template".to_string(),
                    param_type: "string".to_string(),
                    description: "Template/example path relative to $IDF_PATH/examples (e.g., 'get-started/blink').".to_string(),
                },
                ToolParam {
                    name: "target".to_string(),
                    param_type: "string".to_string(),
                    description: "Target chip: esp32, esp32s3, esp32c3, esp32c6, esp32p4 (default: from config)".to_string(),
                },
                ToolParam {
                    name: "if_exists".to_string(),
                    param_type: "string".to_string(),
                    description: "When project folder exists: 'auto_suffix' (default), 'use' (reuse if valid), 'replace' (delete and recreate), 'fail' (return conflict)".to_string(),
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

        let config = get_config().await?;
        config.validate_paths()?;

        let gcx = ccx.lock().await.global_context.clone();
        let projects_root = resolve_esp32_projects_root(gcx, ccx.as_ref(), &config).await?;

        let result = match operation {
            "create" => self
                .create_project(&config, &projects_root, args)
                .await
                .map(|o| (o, vec![])),
            "list_projects" => self
                .list_projects(&projects_root)
                .await
                .map(|o| (o, vec![])),
            "list_examples" => self.list_examples(&config, args).await.map(|o| (o, vec![])),
            "search_examples" => return Err(
                "search_examples requires VecDB which is not available. Use 'list_examples' with a 'filter' parameter instead (e.g., filter: 'wifi').".to_string()
            ),
            "validate" => self.validate_project(&config, args).await.map(|o| (o, vec![])),
            _ => return Err(format!(
                "Unknown operation: '{}'. Valid operations: 'create', 'list_projects', 'list_examples', 'validate'.",
                operation
            )),
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
        projects_root: &Path,
        args: &HashMap<String, Value>,
    ) -> Result<ToolOutput, String> {
        let raw_name = args.get("project_name")
            .and_then(|v| v.as_str())
            .ok_or("Missing required parameter: project_name for 'create' operation")?;
        let project_name = sanitize_esp_workspace_folder_name(raw_name)?;

        let template = args.get("template")
            .and_then(|v| v.as_str())
            .unwrap_or("get-started/blink");

        let target = args.get("target")
            .and_then(|v| v.as_str())
            .unwrap_or(&config.default_target);

        let if_exists = args.get("if_exists")
            .and_then(|v| v.as_str())
            .unwrap_or(DEFAULT_IF_EXISTS);
        let if_exists_policy = IfExistsPolicy::parse(if_exists)?;

        std::fs::create_dir_all(projects_root)
            .map_err(|e| format!("Failed to create projects directory: {}", e))?;

        let (resolved_name, project_path, reused_existing) = match resolve_create_target(
            projects_root,
            &project_name,
            if_exists_policy,
            template,
            target,
        ) {
            Ok(v) => v,
            Err(output) => return Ok(output),
        };

        if reused_existing {
            return Ok(build_create_success_output(
                &resolved_name,
                &project_path,
                target,
                template,
                true,
                if_exists,
            ));
        }

        let example_leaf = template.split('/').last().unwrap_or(template);
        cleanup_orphan_template_folder(projects_root, example_leaf, &resolved_name);

        let mut created_new = false;
        let create_result = if template.contains('/') {
            let example_path = PathBuf::from(&config.esp_idf_path)
                .join("examples")
                .join(template);

            if !example_path.exists() {
                return Err(format!(
                    "Example '{}' not found at {}. Use list_examples to find valid paths.",
                    template,
                    example_path.display()
                ));
            }

            copy_dir_recursive(&example_path, &project_path)
                .map_err(|e| format!("Failed to copy example to {}: {}", project_path.display(), e))?;
            created_new = true;
            Ok(())
        } else {
            let result = IdfCommand::new("create-project")
                .arg(&resolved_name)
                .project_path(projects_root)
                .timeout_secs(60)
                .execute(config).await?;

            if !result.success {
                Err(format!("Project creation failed: {}", result.stderr))
            } else {
                created_new = true;
                Ok(())
            }
        };

        if let Err(e) = create_result {
            rollback_new_project(&project_path, created_new);
            return Err(e);
        }

        if !project_path.exists() {
            rollback_new_project(&project_path, created_new);
            return Err(format!(
                "Project directory was not created at {}",
                project_path.display()
            ));
        }

        if !is_esp_idf_project(&project_path) {
            rollback_new_project(&project_path, created_new);
            return Err(format!(
                "Created folder at {} is not a valid ESP-IDF project (missing CMakeLists.txt or main/)",
                project_path.display()
            ));
        }

        let result = IdfCommand::new("set-target")
            .arg(target)
            .project_path(&project_path)
            .timeout_secs(120)
            .execute(config).await?;

        if !result.success {
            rollback_new_project(&project_path, created_new);
            return Err(format!("Failed to set target '{}': {}", target, result.stderr));
        }

        Ok(build_create_success_output(
            &resolved_name,
            &project_path,
            target,
            template,
            false,
            if_exists,
        ))
    }

    async fn list_projects(&self, projects_root: &Path) -> Result<ToolOutput, String> {
        let projects = scan_projects_in_root(projects_root)?;

        Ok(ToolOutput::success(
            format!("Found {} ESP-IDF project(s) in {}", projects.len(), projects_root.display()),
            serde_json::json!({
                "projects_root": projects_root.to_string_lossy(),
                "count": projects.len(),
                "projects": projects,
            }),
        ))
    }

    async fn list_examples(&self, config: &ESP32Config, args: &HashMap<String, Value>) -> Result<ToolOutput, String> {
        let examples_path = PathBuf::from(&config.esp_idf_path).join("examples");

        if !examples_path.exists() {
            return Err(format!("ESP-IDF examples directory not found at: {}/examples", config.esp_idf_path));
        }

        let filter = args.get("filter")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let mut examples = Vec::new();
        self.find_examples_recursive(&examples_path, &examples_path, &mut examples, filter)?;
        examples.sort();

        let mut categories: HashMap<String, Vec<String>> = HashMap::new();
        for example in &examples {
            let rel = Path::new(example);
            let mut comps = rel.components();
            if let Some(cat) = comps.next() {
                let category = cat.as_os_str().to_string_lossy().to_string();
                let rest: PathBuf = comps.collect();
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
        base_path: &Path,
        current_path: &Path,
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
            .map(PathBuf::from)
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
                status: ToolStatus::PartialSuccess,
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

        let examples_path = PathBuf::from(&config.esp_idf_path).join("examples");
        let search_results: Vec<_> = all_search_results
            .into_iter()
            .filter(|result| {
                Path::new(&result.file_name).starts_with(&examples_path)
            })
            .collect();

        if search_results.is_empty() {
            return Ok((
                ToolOutput {
                    status: ToolStatus::PartialSuccess,
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

fn build_create_success_output(
    project_name: &str,
    project_path: &Path,
    target: &str,
    template: &str,
    reused_existing: bool,
    if_exists: &str,
) -> ToolOutput {
    let suggested_actions = generate_suggested_actions(
        "create",
        true,
        &SuggestionContext::new(),
    );

    let summary = if reused_existing {
        format!("Using existing project '{}' at {}", project_name, project_path.display())
    } else {
        format!("Created project '{}' [{}] from {}", project_name, target, template)
    };

    ToolOutput {
        status: ToolStatus::Success,
        action_taken: if reused_existing { "use_existing".to_string() } else { "create".to_string() },
        data: serde_json::json!({
            "project_name": project_name,
            "project_path": project_path.to_string_lossy(),
            "target": target,
            "template": template,
            "reused_existing": reused_existing,
            "if_exists": if_exists,
        }),
        summary,
        details: Some(format!("Project path: {}", project_path.display())),
        state_delta: super::session_state::StateDelta::none(),
        suggested_actions,
        error: None,
    }
}

/// Resolve the final folder name/path for create, applying the if_exists policy.
fn resolve_create_target(
    projects_root: &Path,
    base_name: &str,
    policy: IfExistsPolicy,
    template: &str,
    target: &str,
) -> Result<(String, PathBuf, bool), ToolOutput> {
    let primary_path = projects_root.join(base_name);

    if !primary_path.exists() {
        return Ok((base_name.to_string(), primary_path, false));
    }

    match policy {
        IfExistsPolicy::AutoSuffix => {
            if let Some((name, path)) = find_available_name(projects_root, base_name) {
                return Ok((name, path, false));
            }
            return Err(name_collision_output(
                projects_root,
                base_name,
                &primary_path,
                policy,
                template,
                target,
                "No free name found (auto_suffix exhausted)",
            ));
        }
        IfExistsPolicy::Use => {
            if is_esp_idf_project(&primary_path) {
                return Ok((base_name.to_string(), primary_path, true));
            }
            return Err(name_collision_output(
                projects_root,
                base_name,
                &primary_path,
                policy,
                template,
                target,
                "Folder exists but is not a valid ESP-IDF project; use if_exists=replace or pick another name",
            ));
        }
        IfExistsPolicy::Replace => {
            if let Err(e) = std::fs::remove_dir_all(&primary_path) {
                return Err(ToolOutput {
                    status: ToolStatus::Failed,
                    action_taken: "create".to_string(),
                    data: serde_json::json!({}),
                    summary: format!("Failed to remove existing project: {}", e),
                    details: None,
                    state_delta: super::session_state::StateDelta::none(),
                    suggested_actions: vec![],
                    error: None,
                });
            }
            return Ok((base_name.to_string(), primary_path, false));
        }
        IfExistsPolicy::Fail => {
            return Err(name_collision_output(
                projects_root,
                base_name,
                &primary_path,
                policy,
                template,
                target,
                "Project folder already exists",
            ));
        }
    }
}

fn name_collision_output(
    projects_root: &Path,
    base_name: &str,
    project_path: &Path,
    policy: IfExistsPolicy,
    template: &str,
    target: &str,
    reason: &str,
) -> ToolOutput {
    let existing_valid = project_path.exists() && is_esp_idf_project(project_path);
    let suggested_names = suggest_alternate_names(projects_root, base_name, 3);
    let if_exists_str = match policy {
        IfExistsPolicy::Fail => "fail",
        IfExistsPolicy::Use => "use",
        IfExistsPolicy::Replace => "replace",
        IfExistsPolicy::AutoSuffix => "auto_suffix",
    };

    let suggested_actions = generate_suggested_actions(
        "create",
        false,
        &SuggestionContext::new()
            .with_create_params(base_name, suggested_names.clone())
            .with_create_template(template)
            .with_create_target(target),
    );

    ToolOutput {
        status: ToolStatus::PartialSuccess,
        action_taken: "create".to_string(),
        data: serde_json::json!({
            "conflict": "name_exists",
            "project_name": base_name,
            "project_path": project_path.to_string_lossy(),
            "existing_valid": existing_valid,
            "if_exists": if_exists_str,
            "suggested_names": suggested_names,
            "projects_root": projects_root.to_string_lossy(),
        }),
        summary: format!(
            "Cannot create '{}': {} at {}",
            base_name,
            reason,
            project_path.display()
        ),
        details: Some(format!(
            "Existing folder is {}valid ESP-IDF project. Suggested names: {}",
            if existing_valid { "" } else { "NOT a " },
            suggested_names.join(", ")
        )),
        state_delta: super::session_state::StateDelta::none(),
        suggested_actions,
        error: None,
    }
}

fn find_available_name(projects_root: &Path, base_name: &str) -> Option<(String, PathBuf)> {
    if !projects_root.join(base_name).exists() {
        return Some((base_name.to_string(), projects_root.join(base_name)));
    }
    for i in 2..=MAX_AUTO_SUFFIX {
        let candidate = format!("{}_{}", base_name, i);
        if let Ok(safe) = sanitize_esp_workspace_folder_name(&candidate) {
            let path = projects_root.join(&safe);
            if !path.exists() {
                return Some((safe, path));
            }
        }
    }
    None
}

fn suggest_alternate_names(projects_root: &Path, base_name: &str, count: usize) -> Vec<String> {
    let mut names = Vec::new();
    if !projects_root.join(base_name).exists() {
        names.push(base_name.to_string());
    }
    for i in 2..=MAX_AUTO_SUFFIX {
        if names.len() >= count {
            break;
        }
        let candidate = format!("{}_{}", base_name, i);
        if let Ok(safe) = sanitize_esp_workspace_folder_name(&candidate) {
            if !projects_root.join(&safe).exists() {
                names.push(safe);
            }
        }
    }
    names
}

fn scan_projects_in_root(projects_root: &Path) -> Result<Vec<serde_json::Value>, String> {
    if !projects_root.exists() {
        return Ok(vec![]);
    }

    let mut projects = Vec::new();
    let entries = std::fs::read_dir(projects_root)
        .map_err(|e| format!("Failed to read projects directory: {}", e))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') || name == "build" || name == "sources" {
            continue;
        }
        if !is_esp_idf_project(&path) {
            continue;
        }
        let modified = entry
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        projects.push(serde_json::json!({
            "name": name,
            "path": path.to_string_lossy(),
            "modified_unix": modified,
        }));
    }

    projects.sort_by(|a, b| {
        let na = a.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let nb = b.get("name").and_then(|v| v.as_str()).unwrap_or("");
        na.cmp(nb)
    });
    Ok(projects)
}

/// Remove stale intermediate folders left by old create-project-from-example runs.
fn cleanup_orphan_template_folder(
    projects_root: &Path,
    example_leaf: &str,
    resolved_name: &str,
) {
    if example_leaf == resolved_name {
        return;
    }
    let orphan = projects_root.join(example_leaf);
    if orphan.exists() && orphan.is_dir() {
        let _ = std::fs::remove_dir_all(&orphan);
    }
}

fn rollback_new_project(path: &Path, created: bool) {
    if created && path.exists() {
        let _ = std::fs::remove_dir_all(path);
    }
}

/// Portable recursive directory copy — replaces the Unix-only `cp -r` shell invocation.
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), std::io::Error> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_if_exists_policy() {
        assert_eq!(IfExistsPolicy::parse("auto_suffix").unwrap(), IfExistsPolicy::AutoSuffix);
        assert_eq!(IfExistsPolicy::parse("use").unwrap(), IfExistsPolicy::Use);
        assert_eq!(IfExistsPolicy::parse("replace").unwrap(), IfExistsPolicy::Replace);
        assert_eq!(IfExistsPolicy::parse("fail").unwrap(), IfExistsPolicy::Fail);
        assert!(IfExistsPolicy::parse("unknown").is_err());
    }

    #[test]
    fn find_available_name_skips_taken() {
        let dir = std::env::temp_dir().join(format!("esp32_proj_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::create_dir_all(dir.join("sensor")).unwrap();

        let (name, path) = find_available_name(&dir, "sensor").unwrap();
        assert_eq!(name, "sensor_2");
        assert_eq!(path, dir.join("sensor_2"));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
