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

// Helper function to create ChatMessage with all required fields
fn create_chat_message(role: &str, content: String, tool_call_id: &str) -> ChatMessage {
    ChatMessage {
        role: role.to_string(),
        content: ChatContent::SimpleText(content),
        finish_reason: None,
        tool_calls: None,
        tool_call_id: tool_call_id.to_string(),
        tool_failed: None,
        usage: None,
        checkpoints: vec![],
        thinking_blocks: None,
    }
}

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
                    description: "Name of the C2000 project to build".to_string(),
                },
                ToolParam {
                    name: "build_config".to_string(),
                    param_type: "string".to_string(),
                    description: "Build configuration: CPU1_RAM, CPU1_FLASH, CPU1_LAUNCHXL_RAM, etc.".to_string(),
                },
                ToolParam {
                    name: "workspace_path".to_string(),
                    param_type: "string".to_string(),
                    description: "CCS workspace path (optional, uses default from config)".to_string(),
                },
                ToolParam {
                    name: "clean_build".to_string(),
                    param_type: "boolean".to_string(),
                    description: "Perform clean build (default: false)".to_string(),
                },
            ],
            parameters_required: vec!["project_name".to_string(), "build_config".to_string()],
        }
    }

    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>,
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        let project_name = args.get("project_name")
            .and_then(|v| v.as_str())
            .ok_or("Missing required parameter: project_name")?;

        let build_config = args.get("build_config")
            .and_then(|v| v.as_str())
            .ok_or("Missing required parameter: build_config")?;

        let workspace_path = args.get("workspace_path")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let clean_build = args.get("clean_build")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let mut context_files = vec![];

        // Load C2000 configuration
        let config = C2000Config::load(&self.config_path).await
            .map_err(|e| format!("Failed to load C2000 config: {}", e))?;

        // Resolve workspace path
        let resolved_workspace_path = if workspace_path.is_empty() {
            config.workspace_path.clone()
        } else {
            workspace_path.to_string()
        };

        // Resolve project path
        let project_path = format!("{}/{}", resolved_workspace_path, project_name);

        // Build CCS CLI command
        let mut build_cmd = Command::new(&format!("{}/eclipse/ccs-server-cli.sh", config.ccs_path));
        build_cmd.arg("-application").arg("projectBuild");
        build_cmd.arg("-ccs.projects").arg(&project_path);
        build_cmd.arg("-ccs.config").arg(build_config);

        if clean_build {
            build_cmd.arg("-ccs.cleanBuild");
        }

        let mut messages = vec![];
        messages.push(format!("🔨 Building C2000 project '{}' with configuration '{}'", project_name, build_config));
        
        if clean_build {
            messages.push("🧹 Performing clean build".to_string());
        }

        // Execute build command
        let output = build_cmd.output().await
            .map_err(|e| format!("Failed to execute CCS build command: {}", e))?;

        if output.status.success() {
            let success_msg = format!(
                "✅ Build completed successfully for project '{}' with configuration '{}'",
                project_name, build_config
            );
            messages.push(success_msg);

            // Add build info to context
            context_files.push(ContextEnum::ChatMessage(create_chat_message(
                "tool",
                format!(
                    "Build Details:\n- Project: {}\n- Configuration: {}\n- Workspace: {}\n- Clean Build: {}",
                    project_name, build_config, resolved_workspace_path, clean_build
                ),
                tool_call_id
            )));
        } else {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            let stdout_msg = String::from_utf8_lossy(&output.stdout);
            return Err(format!(
                "C2000 build failed:\nSTDERR: {}\nSTDOUT: {}",
                error_msg, stdout_msg
            ));
        }

        // Add any stdout information
        if !output.stdout.is_empty() {
            let stdout_msg = String::from_utf8_lossy(&output.stdout);
            messages.push(format!("Build Output:\n{}", stdout_msg));
        }

        let combined_message = messages.join("\n");
        context_files.push(ContextEnum::ChatMessage(create_chat_message(
            "tool",
            combined_message,
            tool_call_id
        )));

        Ok((false, context_files))
    }

    fn tool_depends_on(&self) -> Vec<String> {
        vec![] // No special dependencies
    }
}











