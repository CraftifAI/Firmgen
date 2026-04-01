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

pub struct ToolC2000Flash {
    pub config_path: String,
}

#[async_trait]
impl Tool for ToolC2000Flash {
    fn as_any(&self) -> &dyn std::any::Any { self }

    fn tool_description(&self) -> ToolDesc {
        ToolDesc {
            name: "c2000_flash".to_string(),
            display_name: "C2000 Flash".to_string(),
            source: ToolSource {
                source_type: ToolSourceType::Builtin,
                config_path: self.config_path.clone(),
            },
            agentic: true,
            experimental: false,
            description: "Flash TI C2000 microcontroller project to target device using DSLite with automatic target detection, verification, and reset".to_string(),
            parameters: vec![
                ToolParam {
                    name: "project_name".to_string(),
                    param_type: "string".to_string(),
                    description: "Name of the project to flash".to_string(),
                },
                ToolParam {
                    name: "configuration".to_string(),
                    param_type: "string".to_string(),
                    description: "Build configuration to flash".to_string(),
                },
                ToolParam {
                    name: "target_config".to_string(),
                    param_type: "string".to_string(),
                    description: "Target configuration file (optional, auto-detected)".to_string(),
                },
                ToolParam {
                    name: "verify".to_string(),
                    param_type: "boolean".to_string(),
                    description: "Whether to verify after flashing (default: true)".to_string(),
                },
                ToolParam {
                    name: "reset".to_string(),
                    param_type: "boolean".to_string(),
                    description: "Whether to reset after flashing (default: true)".to_string(),
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
            None => return Err("Missing argument `project_name` in the c2000_flash() call.".to_string())
        };

        let configuration = match args.get("configuration") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => return Err(format!("argument `configuration` is not a string: {:?}", v)),
            None => return Err("Missing argument `configuration` in the c2000_flash() call.".to_string())
        };

        let target_config = match args.get("target_config") {
            Some(Value::String(s)) => Some(s.clone()),
            Some(v) => return Err(format!("argument `target_config` is not a string: {:?}", v)),
            None => None
        };

        let verify = match args.get("verify") {
            Some(Value::Bool(b)) => *b,
            Some(v) => return Err(format!("argument `verify` is not a boolean: {:?}", v)),
            None => true
        };

        let reset = match args.get("reset") {
            Some(Value::Bool(b)) => *b,
            Some(v) => return Err(format!("argument `reset` is not a boolean: {:?}", v)),
            None => true
        };

        // Get configuration from API
        // Load C2000 configuration from API with fallback
        let config = C2000Config::load_from_api("http://localhost:8002/v1/c2000-config").await?;

        // Determine target config file
        let ccxml_path = target_config.unwrap_or_else(|| {
            config.get_ccxml_path(&project_name).to_string_lossy().to_string()
        });

        // Check if target config exists
        if !std::path::Path::new(&ccxml_path).exists() {
            return Err(format!("Target configuration file not found: {}", ccxml_path));
        }

        // Get output file path
        let output_path = config.get_output_path(&project_name, &configuration);
        if !output_path.exists() {
            return Err(format!("Output file not found: {}. Please build the project first.", output_path.display()));
        }

        // Build DSLite command
        let mut dslite_cmd = Command::new(&format!("{}/ccs_base/DebugServer/bin/DSLite", config.ccs_path));
        dslite_cmd.args(&["flash", "-c", &ccxml_path]);

        // Add verification flag
        if verify {
            dslite_cmd.arg("-v");
        }

        // Add flash and erase flags
        dslite_cmd.args(&["-e", "-f", &output_path.to_string_lossy()]);

        // Add core and reset indices (typically 0)
        dslite_cmd.args(&["-n", "0"]);
        if reset {
            dslite_cmd.args(&["-r", "0"]);
        }

        // Execute command
        let output = dslite_cmd.output().await
            .map_err(|e| format!("Failed to execute DSLite command: {}", e))?;

        let mut messages: Vec<String> = Vec::new();
        let mut context_files = Vec::new();

        if output.status.success() {
            let success_msg = format!(
                "✅ Flash programming completed successfully\n📁 Project: {}\n⚙️ Configuration: {}\n🎯 Target: {}\n📄 Output: {}",
                project_name, configuration, ccxml_path, output_path.display()
            );
            
            if verify {
                messages.push("✅ Verification passed".to_string());
            }
            if reset {
                messages.push("✅ Device reset successful".to_string());
            }
            
            messages.push(success_msg);
            messages.push(format!(
                "Flash Details:\n- Project: {}\n- Configuration: {}\n- Target Config: {}\n- Output File: {}\n- Verify: {}\n- Reset: {}",
                project_name, configuration, ccxml_path, output_path.display(), verify, reset
            ));
        } else {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            let stdout_msg = String::from_utf8_lossy(&output.stdout);
            
            return Err(format!(
                "Flash programming failed for project '{}':\nSTDERR: {}\nSTDOUT: {}",
                project_name, error_msg, stdout_msg
            ));
        }

        // Add any stdout information
        if !output.stdout.is_empty() {
            let stdout_msg = String::from_utf8_lossy(&output.stdout);
            messages.push(format!("DSLite Output:\n{}", stdout_msg));
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


