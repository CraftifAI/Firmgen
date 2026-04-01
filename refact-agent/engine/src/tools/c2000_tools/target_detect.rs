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

pub struct ToolC2000TargetDetect {
    pub config_path: String,
}

#[async_trait]
impl Tool for ToolC2000TargetDetect {
    fn as_any(&self) -> &dyn std::any::Any { self }

    fn tool_description(&self) -> ToolDesc {
        ToolDesc {
            name: "c2000_target_detect".to_string(),
            display_name: "C2000 Target Detect".to_string(),
            source: ToolSource {
                source_type: ToolSourceType::Builtin,
                config_path: self.config_path.clone(),
            },
            agentic: true,
            experimental: false,
            description: "Detect connected TI C2000 target devices, debug probes, and available cores with detailed system information".to_string(),
            parameters: vec![
                ToolParam {
                    name: "list_cores".to_string(),
                    param_type: "boolean".to_string(),
                    description: "Whether to list available cores (default: true)".to_string(),
                },
                ToolParam {
                    name: "list_reset_actions".to_string(),
                    param_type: "boolean".to_string(),
                    description: "Whether to list reset actions (default: true)".to_string(),
                }
            ],
            parameters_required: vec![],
        }
    }

    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>,
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        // Extract parameters
        let list_cores = match args.get("list_cores") {
            Some(Value::Bool(b)) => *b,
            Some(v) => return Err(format!("argument `list_cores` is not a boolean: {:?}", v)),
            None => true
        };

        let list_reset_actions = match args.get("list_reset_actions") {
            Some(Value::Bool(b)) => *b,
            Some(v) => return Err(format!("argument `list_reset_actions` is not a boolean: {:?}", v)),
            None => true
        };

        // Get configuration from API
        let config = C2000Config::load_from_api("http://localhost:8002/v1/c2000-config").await?;

        let mut messages: Vec<String> = Vec::new();
        let mut context_files = Vec::new();

        let mut detected_targets = Vec::new();
        let workspace_path = std::path::Path::new(&config.workspace_path);

        // Search for target configs in targetConfigs folders within workspace
        if workspace_path.exists() {
            for entry in walkdir::WalkDir::new(workspace_path)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_dir())
            {
                // Look for targetConfigs directories
                if let Some(dir_name) = entry.file_name().to_str() {
                    if dir_name == "targetConfigs" {
                        // Search for .ccxml files in this targetConfigs folder
                        if let Ok(entries) = std::fs::read_dir(entry.path()) {
                            for file_entry in entries.flatten() {
                                if let Some(file_name) = file_entry.file_name().to_str() {
                                    if file_name.ends_with(".ccxml") {
                                        detected_targets.push(
                                            file_entry.path().to_string_lossy().to_string()
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if !detected_targets.is_empty() {
            messages.push("🎯 Detected Target Configurations:".to_string());
            for target in &detected_targets {
                messages.push(format!("  📄 {}", target));
            }
        } else {
            messages.push("⚠️ No target configurations found in workspace".to_string());
            messages.push(format!("🔍 Searched in: {}", workspace_path.display()));
            messages.push("💡 Make sure targetConfigs folders exist in your workspace projects".to_string());
        }

        // Try to detect connected devices using DSLite
        let dslite_path = format!("{}/ccs_base/DebugServer/bin/DSLite", config.ccs_path);
        if std::path::Path::new(&dslite_path).exists() {
            // DSLite requires --config parameter with a ccxml file
            if let Some(target_config) = detected_targets.first() {
                // Use identifyProbe to detect connected debug probes
                let probe_cmd = Command::new(&dslite_path)
                    .arg("identifyProbe")
                    .arg(format!("--config={}", target_config))
                    .output().await;

                match probe_cmd {
                    Ok(output) => {
                        if output.status.success() {
                            let probe_output = String::from_utf8_lossy(&output.stdout);
                            if !probe_output.trim().is_empty() {
                                messages.push("🔍 Connected Debug Probes:".to_string());
                                for line in probe_output.lines() {
                                    if !line.trim().is_empty() {
                                        messages.push(format!("  {}", line.trim()));
                                    }
                                }
                            } else {
                                messages.push("⚠️ No debug probes detected (device may not be connected)".to_string());
                            }
                        } else {
                            let error_output = String::from_utf8_lossy(&output.stderr);
                            messages.push(format!("⚠️ Could not identify probe: {}", error_output));
                        }
                    },
                    Err(e) => {
                        messages.push(format!("⚠️ Failed to execute identifyProbe command: {}", e));
                    }
                }

                // Note: DSLite doesn't have direct commands to list cores or reset actions
                // These would typically be available through CCS GUI or by examining the .ccxml file
                if list_cores || list_reset_actions {
                    messages.push(format!(
                        "💡 Tip: To see cores and reset actions, examine the target config file: {}",
                        target_config
                    ));
                }
            } else {
                messages.push("⚠️ No target configuration (.ccxml) file found - cannot run identifyProbe. Please ensure a .ccxml file exists in a targetConfigs folder within the workspace.".to_string());
            }
        } else {
            messages.push(format!("❌ DSLite not found at: {} - cannot detect connected devices", dslite_path));
        }

        // Check for USB devices (common C2000 debug probes)
        let usb_devices = self.detect_usb_devices().await;
        if !usb_devices.is_empty() {
            messages.push("🔌 Detected USB Devices:".to_string());
            for device in &usb_devices {
                messages.push(format!("  {}", device));
            }
        }

        // Add detection info to messages
        messages.push(format!(
            "Target Detection Results:\n- Target Configs Found: {}\n- List Cores: {}\n- List Reset Actions: {}\n- USB Devices: {}",
            detected_targets.len(), list_cores, list_reset_actions, usb_devices.len()
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

impl ToolC2000TargetDetect {
    async fn detect_usb_devices(&self) -> Vec<String> {
        let mut devices = Vec::new();
        
        // Check for common C2000 debug probe devices
        let common_devices = [
            "/dev/ttyACM0", "/dev/ttyACM1", "/dev/ttyACM2",
            "/dev/ttyUSB0", "/dev/ttyUSB1", "/dev/ttyUSB2"
        ];

        for device in &common_devices {
            if std::path::Path::new(device).exists() {
                devices.push(device.to_string());
            }
        }

        // Try to get more detailed USB device info using lsusb
        if let Ok(output) = Command::new("lsusb").output().await {
            if output.status.success() {
                let lsusb_output = String::from_utf8_lossy(&output.stdout);
                for line in lsusb_output.lines() {
                    if line.contains("Texas Instruments") || 
                       line.contains("XDS") || 
                       line.contains("C2000") ||
                       line.contains("F28") {
                        devices.push(format!("USB: {}", line.trim()));
                    }
                }
            }
        }

        devices
    }
}


