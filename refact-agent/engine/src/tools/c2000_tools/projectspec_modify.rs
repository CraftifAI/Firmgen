use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::Mutex as AMutex;
use std::path::Path;
use std::path::PathBuf;
use std::fs;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::tools::tools_description::{Tool, ToolDesc, ToolParam, ToolSource, ToolSourceType};
use crate::call_validation::{ChatMessage, ChatContent, ContextEnum};

use super::config::replace_project_name_in_projectspec;
use super::config::C2000Config;

pub struct ToolC2000ProjectspecModify {
    pub config_path: String,
}

#[async_trait]
impl Tool for ToolC2000ProjectspecModify {
    fn as_any(&self) -> &dyn std::any::Any { self }

    fn tool_description(&self) -> ToolDesc {
        ToolDesc {
            name: "c2000_projectspec_modify".to_string(),
            display_name: "C2000 Projectspec Modify".to_string(),
            source: ToolSource {
                source_type: ToolSourceType::Builtin,
                config_path: self.config_path.clone(),
            },
            agentic: true,
            experimental: false,
            description: "Modifies a C2000 .projectspec file to rename project, add/remove configurations, or change project settings. Use this before calling c2000_project_create to ensure the project is created with the desired name and settings. Critical for working around CCS CLI rename limitations.".to_string(),
            parameters: vec![
                ToolParam {
                    name: "projectspec_path".to_string(),
                    param_type: "string".to_string(),
                    description: "Path to the .projectspec file to modify (can use $C2000WARE variable)".to_string(),
                },
                ToolParam {
                    name: "project_name".to_string(),
                    param_type: "string".to_string(),
                    description: "New name for the project (will replace the name attribute in <project> tag)".to_string(),
                },
                ToolParam {
                    name: "output_path".to_string(),
                    param_type: "string".to_string(),
                    description: "Path where to save the modified projectspec (optional, defaults to same file if not specified)".to_string(),
                },
            ],
            parameters_required: vec!["projectspec_path".to_string(), "project_name".to_string()],
        }
    }

    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>,
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        // Parse parameters
        let projectspec_path = match args.get("projectspec_path") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => return Err(format!("argument `projectspec_path` is not a string: {:?}", v)),
            None => return Err("Missing argument `projectspec_path`".to_string())
        };

        let project_name = match args.get("project_name") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => return Err(format!("argument `project_name` is not a string: {:?}", v)),
            None => return Err("Missing argument `project_name`".to_string())
        };

        let output_path = match args.get("output_path") {
            Some(Value::String(s)) => Some(s.clone()),
            Some(v) => return Err(format!("argument `output_path` is not a string: {:?}", v)),
            None => None
        };

        let mut output = String::new();

        // Load C2000 configuration
        let config = C2000Config::load_from_api("http://localhost:8002/v1/c2000-config").await
            .map_err(|e| format!("Failed to load C2000 config: {}", e))?;

        // Resolve path variables (e.g., $C2000WARE)
        let resolved_projectspec_path = config.resolve_path_variables(&projectspec_path);
        
        // Resolve projectspec file path
        let projectspec_file = Path::new(&resolved_projectspec_path);
        
        // Check if file exists
        if !projectspec_file.exists() {
            return Err(format!("Error: Projectspec file not found: {}", resolved_projectspec_path));
        }

        // Get safe path for modification (copies to CCS workspace if needed)
        let source_path = config.get_safe_modification_path(&resolved_projectspec_path)
            .map_err(|e| format!("Failed to ensure file in CCS workspace: {}", e))?;
        
        if source_path != projectspec_file {
            output.push_str(&format!("📋 Copied file to CCS workspace: {}\n", source_path.display()));
            output.push_str(&format!("   (Original preserved at: {})\n", resolved_projectspec_path));
        }

        // Read the file (from CCS workspace if it was copied)
        let projectspec_content = fs::read_to_string(&source_path)
            .map_err(|e| format!("Failed to read projectspec file: {}", e))?;

        // Replace the project name in the <project> tag
        let modified_content = replace_project_name_in_projectspec(&projectspec_content, &project_name)?;

        // Determine output path - always in CCS workspace
        let output_file = if let Some(out_path) = output_path {
            // If output path is specified, ensure it's in CCS workspace
            let out = PathBuf::from(&out_path);
            if config.is_in_ccs_workspace(&out_path) {
                out
            } else {
                // If output path is outside workspace, put it in workspace
                let file_name = out.file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                PathBuf::from(&config.workspace_path).join(file_name)
            }
        } else {
            // Default: use CCS workspace directory
            PathBuf::from(&config.workspace_path)
                .join(format!("{}.projectspec", project_name))
        };

        // Write the modified content
        fs::write(&output_file, &modified_content)
            .map_err(|e| format!("Failed to write modified projectspec file: {}", e))?;
        
        output.push_str(&format!("✅ Successfully modified projectspec file\n"));
        output.push_str(&format!("  Input:  {}\n", source_path.display()));
        output.push_str(&format!("  Output: {}\n", output_file.display()));
        output.push_str(&format!("  Project name: {}\n", project_name));
        output.push_str("\nNote: This modified projectspec can now be used with c2000_project_create\n");

        let context_files = vec![ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: ChatContent::SimpleText(output),
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
            ..Default::default()
        })];

        Ok((false, context_files))
    }

    fn tool_depends_on(&self) -> Vec<String> {
        vec!["c2000".to_string()]
    }
}


