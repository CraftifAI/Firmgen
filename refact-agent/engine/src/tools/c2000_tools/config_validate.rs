use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::Mutex as AMutex;
// Removed unused import: tokio::process::Command

use crate::at_commands::at_commands::AtCommandsContext;
use crate::tools::tools_description::{Tool, ToolDesc, ToolParam, ToolSource, ToolSourceType};
use crate::call_validation::{ChatMessage, ChatContent, ContextEnum};

use super::config::C2000Config;

pub struct ToolC2000ConfigValidate {
    pub config_path: String,
}

#[async_trait]
impl Tool for ToolC2000ConfigValidate {
    fn as_any(&self) -> &dyn std::any::Any { self }

    fn tool_description(&self) -> ToolDesc {
        ToolDesc {
            name: "c2000_config_validate".to_string(),
            display_name: "C2000 Config Validate".to_string(),
            source: ToolSource {
                source_type: ToolSourceType::Builtin,
                config_path: self.config_path.clone(),
            },
            agentic: true,
            experimental: false,
            description: "Validate TI C2000 project configuration, file paths, dependencies, and build settings for correctness".to_string(),
            parameters: vec![
                ToolParam {
                    name: "project_name".to_string(),
                    param_type: "string".to_string(),
                    description: "Name of the project to validate".to_string(),
                },
                ToolParam {
                    name: "check_paths".to_string(),
                    param_type: "boolean".to_string(),
                    description: "Whether to check file paths (default: true)".to_string(),
                },
                ToolParam {
                    name: "check_dependencies".to_string(),
                    param_type: "boolean".to_string(),
                    description: "Whether to check dependencies (default: true)".to_string(),
                }
            ],
            parameters_required: vec!["project_name".to_string()],
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
            None => return Err("Missing argument `project_name` in the c2000_config_validate() call.".to_string())
        };

        let check_paths = match args.get("check_paths") {
            Some(Value::Bool(b)) => *b,
            Some(v) => return Err(format!("argument `check_paths` is not a boolean: {:?}", v)),
            None => true
        };

        let check_dependencies = match args.get("check_dependencies") {
            Some(Value::Bool(b)) => *b,
            Some(v) => return Err(format!("argument `check_dependencies` is not a boolean: {:?}", v)),
            None => true
        };

        // Get configuration from API
        let config = C2000Config::load_from_api("http://localhost:8002/v1/c2000-config").await?;

        let mut messages = Vec::new();
        let mut context_files = Vec::new();
        let mut validation_results: Vec<String> = Vec::new();

        // Validate C2000 configuration
        match config.validate_paths() {
            Ok(_) => validation_results.push("✅ C2000 configuration paths are valid".to_string()),
            Err(e) => validation_results.push(format!("❌ C2000 configuration error: {}", e))
        }

        // Check project existence
        let project_path = std::path::Path::new(&config.workspace_path).join(&project_name);
        if project_path.exists() {
            validation_results.push(format!("✅ Project '{}' exists in workspace", project_name));
        } else {
            validation_results.push(format!("❌ Project '{}' not found in workspace", project_name));
        }

        // Check project files if requested
        if check_paths {
            let project_files = [
                "project.pjt",
                "targetConfigs/TMS320F28P650DK9.ccxml",
                "src",
                "include"
            ];

            for file in &project_files {
                let file_path = project_path.join(file);
                if file_path.exists() {
                    validation_results.push(format!("✅ Found: {}", file));
                } else {
                    validation_results.push(format!("⚠️ Missing: {}", file));
                }
            }
        }

        // Check build configurations
        let configurations = ["CPU1_LAUNCHXL_RAM", "CPU1_LAUNCHXL_FLASH", "CPU1_RAM", "CPU1_FLASH"];
        for config_name in &configurations {
            let config_path = project_path.join(config_name);
            if config_path.exists() {
                validation_results.push(format!("✅ Build configuration '{}' exists", config_name));
                
                // Check for output file
                let output_file = config_path.join(format!("{}.out", project_name));
                if output_file.exists() {
                    validation_results.push(format!("✅ Output file exists for '{}'", config_name));
                } else {
                    validation_results.push(format!("⚠️ No output file for '{}' (needs build)", config_name));
                }
            } else {
                validation_results.push(format!("⚠️ Build configuration '{}' not found", config_name));
            }
        }

        // Check dependencies if requested
        if check_dependencies {
            // Check for C2000Ware integration
            let c2000ware_check = std::path::Path::new(&config.c2000ware_path).exists();
            if c2000ware_check {
                validation_results.push("✅ C2000Ware integration available".to_string());
            } else {
                validation_results.push("❌ C2000Ware path not accessible".to_string());
            }

            // Check for CCS installation
            let ccs_cli_path = format!("{}/eclipse/ccs-server-cli.sh", config.ccs_path);
            if std::path::Path::new(&ccs_cli_path).exists() {
                validation_results.push("✅ CCS CLI available".to_string());
            } else {
                validation_results.push("❌ CCS CLI not found".to_string());
            }

            // Check for DSLite
            let dslite_path = format!("{}/ccs_base/DebugServer/bin/DSLite", config.ccs_path);
            if std::path::Path::new(&dslite_path).exists() {
                validation_results.push("✅ DSLite available".to_string());
            } else {
                validation_results.push("❌ DSLite not found".to_string());
            }
        }

        // Generate summary
        let error_count = validation_results.iter().filter(|s| s.starts_with("❌")).count();
        let warning_count = validation_results.iter().filter(|s| s.starts_with("⚠️")).count();
        let success_count = validation_results.iter().filter(|s| s.starts_with("✅")).count();

        let summary = format!(
            "📊 Validation Summary:\n- ✅ Passed: {}\n- ⚠️ Warnings: {}\n- ❌ Errors: {}",
            success_count, warning_count, error_count
        );

        messages.push(summary);
        let validation_count = validation_results.len();
        messages.extend(validation_results);

        // Add validation info to messages
        messages.push(format!(
            "Configuration Validation:\n- Project: {}\n- Check Paths: {}\n- Check Dependencies: {}\n- Total Checks: {}",
            project_name, check_paths, check_dependencies, validation_count
        ));

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


