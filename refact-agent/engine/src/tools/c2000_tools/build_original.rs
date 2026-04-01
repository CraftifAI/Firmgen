use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::Mutex as AMutex;
use tokio::process::Command;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::tools::tools_description::{Tool, ToolDesc, ToolParam, ToolSource, ToolSourceType};
use crate::call_validation::{ChatMessage, ChatContent, ContextEnum};

use super::config::C2000Config;

pub struct ToolC2000Build {
    pub config_path: String,
}

#[async_trait]
impl Tool for ToolC2000Build {
    fn as_any(&self) -> &dyn std::any::Any { self }

    fn tool_description(&self) -> ToolDesc {
        ToolDesc {
            name: "c2000_build".to_string(),
            display_name: "C2000 Build".to_string(),
            source: ToolSource {
                source_type: ToolSourceType::Builtin,
                config_path: self.config_path.clone(),
            },
            agentic: true,
            experimental: false,
            description: "Build C2000 project for specific configuration with automatic dependency resolution".to_string(),
            parameters: vec![
                ToolParam {
                    name: "project_name".to_string(),
                    param_type: "string".to_string(),
                    description: "Name of the project to build".to_string(),
                },
                ToolParam {
                    name: "configuration".to_string(),
                    param_type: "string".to_string(),
                    description: "Build configuration (CPU1_LAUNCHXL_RAM, CPU1_LAUNCHXL_FLASH, CPU1_RAM, CPU1_FLASH)".to_string(),
                },
                ToolParam {
                    name: "build_type".to_string(),
                    param_type: "string".to_string(),
                    description: "Type of build: full, incremental, clean (default: full)".to_string(),
                },
                ToolParam {
                    name: "workspace_path".to_string(),
                    param_type: "string".to_string(),
                    description: "CCS workspace path (optional, uses default from config)".to_string(),
                }
            ],
            parameters_required: vec!["project_name".to_string(), "configuration".to_string()],
        }
    }

    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>,
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        // Extract parameters
        let project_name = match args.get("project_name") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => return Err(format!("argument `project_name` is not a string: {:?}", v)),
            None => return Err("Missing argument `project_name` in the c2000_build() call.".to_string())
        };

        let configuration = match args.get("configuration") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => return Err(format!("argument `configuration` is not a string: {:?}", v)),
            None => return Err("Missing argument `configuration` in the c2000_build() call.".to_string())
        };

        let build_type = match args.get("build_type") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => return Err(format!("argument `build_type` is not a string: {:?}", v)),
            None => "full".to_string()
        };

        let workspace_path = match args.get("workspace_path") {
            Some(Value::String(s)) => Some(s.clone()),
            Some(v) => return Err(format!("argument `workspace_path` is not a string: {:?}", v)),
            None => None
        };

        // Validate configuration
        let valid_configs = ["CPU1_LAUNCHXL_RAM", "CPU1_LAUNCHXL_FLASH", "CPU1_RAM", "CPU1_FLASH"];
        if !valid_configs.contains(&configuration.as_str()) {
            return Err(format!("Invalid configuration '{}'. Valid options: {:?}", configuration, valid_configs));
        }

        // Validate build type
        let valid_build_types = ["full", "incremental", "clean"];
        if !valid_build_types.contains(&build_type.as_str()) {
            return Err(format!("Invalid build type '{}'. Valid options: {:?}", build_type, valid_build_types));
        }

        // Get configuration
        let config = C2000Config::load_from_file(&self.config_path).await?;
        let resolved_workspace_path = workspace_path.unwrap_or(config.workspace_path.clone());

        // Check if project exists
        let project_path = std::path::Path::new(&resolved_workspace_path).join(&project_name);
        if !project_path.exists() {
            return Err(format!("Project '{}' not found in workspace: {}", project_name, resolved_workspace_path));
        }

        // Build CCS command
        let mut ccs_cmd = Command::new(&format!("{}/eclipse/ccs-server-cli.sh", config.ccs_path));
        ccs_cmd.args(&[
            "-workspace", &resolved_workspace_path,
            "-application", "projectBuild",
            "-ccs.projects", &project_name,
            "-ccs.configuration", &configuration,
            "-ccs.buildType", &build_type,
        ]);

        // Execute command
        let output = ccs_cmd.output().await
            .map_err(|e| format!("Failed to execute CCS build command: {}", e))?;

        let mut messages = Vec::new();
        let mut context_files = Vec::new();

        if output.status.success() {
            let output_path = config.get_output_path(&project_name, &configuration);
            
            let success_msg = format!(
                "✅ Build completed successfully for project '{}' with configuration '{}'\n📁 Output: {}\n🔧 Build type: {}",
                project_name, configuration, output_path.display(), build_type
            );
            
            messages.push(success_msg);
            
            // Add build info to context
            context_files.push(ContextEnum::ChatMessage(ChatMessage {
                role: "tool".to_string(),
                content: ChatContent::SimpleText(format!(
                    "Build Details:\n- Project: {}\n- Configuration: {}\n- Build Type: {}\n- Output: {}",
                    project_name, configuration, build_type, output_path.display()
                )),
                tool_calls: None,
            }));
        } else {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            let stdout_msg = String::from_utf8_lossy(&output.stdout);
            
            return Err(format!(
                "Build failed for project '{}' with configuration '{}':\nSTDERR: {}\nSTDOUT: {}",
                project_name, configuration, error_msg, stdout_msg
            ));
        }

        // Add build output information
        if !output.stdout.is_empty() {
            let stdout_msg = String::from_utf8_lossy(&output.stdout);
            messages.push(format!("Build Output:\n{}", stdout_msg));
        }

        let combined_message = messages.join("\n");
        context_files.push(ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: ChatContent::SimpleText(combined_message),
            tool_calls: None,
        }));

        Ok((false, context_files))
    }

    fn tool_depends_on(&self) -> Vec<String> {
        vec![] // No special dependencies
    }
}


