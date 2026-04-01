use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::Mutex as AMutex;
use tokio::process::Command;
use tokio::fs;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::tools::tools_description::{Tool, ToolDesc, ToolParam, ToolSource, ToolSourceType};
use crate::call_validation::{ChatMessage, ChatContent, ContextEnum};

use super::config::{C2000Config, replace_project_name_in_projectspec};

pub struct ToolC2000ProjectCreate {
    pub config_path: String,
}

#[async_trait]
impl Tool for ToolC2000ProjectCreate {
    fn as_any(&self) -> &dyn std::any::Any { self }

    fn tool_description(&self) -> ToolDesc {
        ToolDesc {
            name: "c2000_project_create".to_string(),
            display_name: "C2000 Project Create".to_string(),
            source: ToolSource {
                source_type: ToolSourceType::Builtin,
                config_path: self.config_path.clone(),
            },
            agentic: true,
            experimental: false,
            description: "Create TI Code Composer Studio (CCS) project from .projectspec file with intelligent defaults and automatic workspace integration. Use this for creating C2000 projects from C2000Ware examples like SPI loopback, SCI echoback, LED blinky, etc.".to_string(),
            parameters: vec![
                ToolParam {
                    name: "projectspec_path".to_string(),
                    param_type: "string".to_string(),
                    description: "Path to the .projectspec file (can use $C2000WARE variable)".to_string(),
                },
                ToolParam {
                    name: "project_name".to_string(),
                    param_type: "string".to_string(),
                    description: "Name for the created project (optional, defaults to projectspec name)".to_string(),
                },
                ToolParam {
                    name: "workspace_path".to_string(),
                    param_type: "string".to_string(),
                    description: "CCS workspace path (optional, uses default from config)".to_string(),
                },
                ToolParam {
                    name: "copy_to_workspace".to_string(),
                    param_type: "boolean".to_string(),
                    description: "Whether to copy project into workspace (default: true)".to_string(),
                }
            ],
            parameters_required: vec!["projectspec_path".to_string()],
        }
    }

    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>,
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        // Extract parameters
        let projectspec_path = match args.get("projectspec_path") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => return Err(format!("argument `projectspec_path` is not a string: {:?}", v)),
            None => return Err("Missing argument `projectspec_path` in the c2000_project_create() call.".to_string())
        };

        let project_name = match args.get("project_name") {
            Some(Value::String(s)) => Some(s.clone()),
            Some(v) => return Err(format!("argument `project_name` is not a string: {:?}", v)),
            None => None
        };

        let workspace_path = match args.get("workspace_path") {
            Some(Value::String(s)) => Some(s.clone()),
            Some(v) => return Err(format!("argument `workspace_path` is not a string: {:?}", v)),
            None => None
        };

        let copy_to_workspace = match args.get("copy_to_workspace") {
            Some(Value::Bool(b)) => *b,
            Some(v) => return Err(format!("argument `copy_to_workspace` is not a boolean: {:?}", v)),
            None => true
        };

        // Get configuration from API with fallback
        let config = C2000Config::load_from_api("http://localhost:8002/v1/c2000-config").await?;

        // Validate configuration
        config.validate_paths().map_err(|e| format!("Configuration validation failed: {}", e))?;

        // Resolve variables in paths
        let resolved_projectspec_path = config.resolve_path_variables(&projectspec_path);
        let resolved_workspace_path = workspace_path.unwrap_or(config.workspace_path.clone());

        // Determine project name
        let final_project_name = project_name.unwrap_or_else(|| {
            // Extract project name from projectspec path
            std::path::Path::new(&resolved_projectspec_path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown_project")
                .to_string()
        });

        // Extract the original project name from the projectspec
        let original_project_name = std::path::Path::new(&resolved_projectspec_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown_project")
            .to_string();

        // Validate projectspec file exists
        if !std::path::Path::new(&resolved_projectspec_path).exists() {
            return Err(format!("Projectspec file not found: {}", resolved_projectspec_path));
        }

        // If a custom name was provided, modify the projectspec file before creating the project
        let projectspec_to_use = if final_project_name != original_project_name {
            // Read the original projectspec
            let projectspec_content = fs::read_to_string(&resolved_projectspec_path).await
                .map_err(|e| format!("Failed to read projectspec file: {}", e))?;

            // Safely replace only the name attribute in the FIRST <project ...> tag header
            let proj_open_idx = match projectspec_content.find("<project") {
                Some(i) => i,
                None => return Err("Invalid projectspec: missing <project tag".to_string()),
            };
            let tag_close_rel = match projectspec_content[proj_open_idx..].find('>') {
                Some(i) => i,
                None => return Err("Invalid projectspec: unterminated <project tag".to_string()),
            };
            let tag_close_idx = proj_open_idx + tag_close_rel;

            // Work within the tag header only
            let header = &projectspec_content[proj_open_idx..tag_close_idx];
            let name_attr = "name=\"";
            let name_pos_rel = match header.find(name_attr) {
                Some(i) => i,
                None => return Err("Invalid projectspec: <project> missing name=\"...\" attribute".to_string()),
            };
            let name_val_start = proj_open_idx + name_pos_rel + name_attr.len();
            let rest = &projectspec_content[name_val_start..tag_close_idx];
            let name_end_rel = match rest.find('"') {
                Some(i) => i,
                None => return Err("Invalid projectspec: unterminated name attribute".to_string()),
            };
            let name_val_end = name_val_start + name_end_rel;

            // Use shared function to modify the project name
            let modified = replace_project_name_in_projectspec(&projectspec_content, &final_project_name)?;

            // Create temporary projectspec file to use for creation
            let temp_projectspec_path = format!("{}.tmp", resolved_projectspec_path);
            fs::write(&temp_projectspec_path, &modified).await
                .map_err(|e| format!("Failed to create temporary projectspec: {}", e))?;

            temp_projectspec_path
        } else {
            resolved_projectspec_path.clone()
        };

        // Build CCS command
        let mut ccs_cmd = Command::new(&format!("{}/eclipse/ccs-server-cli.sh", config.ccs_path));
        ccs_cmd.args(&[
            "-noSplash",
            "-workspace", &resolved_workspace_path,
            "-application", "projectCreate",
            "-ccs.projectSpec", &projectspec_to_use,
            // No -ccs.renameTo needed since we already modified the projectspec
        ]);

        if copy_to_workspace {
            ccs_cmd.arg("-ccs.copyIntoWorkspace");
        }

        // Execute command
        let output = ccs_cmd.output().await
            .map_err(|e| format!("Failed to execute CCS command: {}", e))?;

        // Clean up temporary projectspec file if we created one
        if final_project_name != original_project_name {
            let _ = fs::remove_file(&projectspec_to_use).await;
        }

        let mut messages = Vec::new();
        let mut context_files = Vec::new();

        if output.status.success() {
            let success_msg = format!(
                "✅ Project '{}' created successfully from projectspec: {}\n📁 Location: {}/{}\n",
                final_project_name,
                resolved_projectspec_path,
                resolved_workspace_path,
                final_project_name
            );
            
            messages.push(success_msg);
            messages.push(format!("Project Details:\n- Name: {}\n- Source: {}\n- Workspace: {}\n- Copy to workspace: {}",
                final_project_name,
                resolved_projectspec_path,
                resolved_workspace_path,
                copy_to_workspace
            ));
        } else {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            let stdout_msg = String::from_utf8_lossy(&output.stdout);
            return Err(format!(
                "CCS project creation failed:\nSTDERR: {}\nSTDOUT: {}",
                error_msg, stdout_msg
            ));
        }

        // Add any stdout information
        if !output.stdout.is_empty() {
            let stdout_msg = String::from_utf8_lossy(&output.stdout);
            messages.push(format!("CCS Output:\n{}", stdout_msg));
        }

        let combined_message = messages.join("\n");
        context_files.push(ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: ChatContent::SimpleText(combined_message),
            
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
            ..Default::default()
            
            
            
            
        }));

        Ok((false, context_files))
    }

    fn tool_depends_on(&self) -> Vec<String> {
        vec!["c2000".to_string()]
    }
}

