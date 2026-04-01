use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::Mutex as AMutex;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::tools::tools_description::{Tool, ToolDesc, ToolParam, ToolSource, ToolSourceType};
use crate::call_validation::{ChatMessage, ChatContent, ContextEnum};
use crate::files_correction::get_project_dirs;

pub struct ToolC2000ExampleList {
    pub config_path: String,
}

#[async_trait]
impl Tool for ToolC2000ExampleList {
    fn as_any(&self) -> &dyn std::any::Any { self }

    fn tool_description(&self) -> ToolDesc {
        ToolDesc {
            name: "c2000_example_list".to_string(),
            display_name: "C2000 Example List".to_string(),
            source: ToolSource {
                source_type: ToolSourceType::Builtin,
                config_path: self.config_path.clone(),
            },
            agentic: true,
            experimental: false,
            description: "List available TI C2000Ware examples with intelligent filtering by device family, peripheral, and type. Provides structured output with example descriptions, better than generic search tools. Understands C2000Ware directory structure and extracts metadata from example paths.".to_string(),
            parameters: vec![
                ToolParam {
                    name: "device_family".to_string(),
                    param_type: "string".to_string(),
                    description: "Device family filter (e.g., f28p65x, f28002x, f28004x, f2837xd, f2838x)".to_string(),
                },
                ToolParam {
                    name: "peripheral".to_string(),
                    param_type: "string".to_string(),
                    description: "Peripheral filter (e.g., spi, sci, i2c, adc, epwm, can, mcan, usb)".to_string(),
                },
                ToolParam {
                    name: "example_type".to_string(),
                    param_type: "string".to_string(),
                    description: "Example type filter (driverlib, device_support, training)".to_string(),
                },
                ToolParam {
                    name: "show_paths".to_string(),
                    param_type: "boolean".to_string(),
                    description: "Whether to show full paths to examples (default: true)".to_string(),
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
        let device_family = match args.get("device_family") {
            Some(Value::String(s)) => Some(s.clone()),
            Some(v) => return Err(format!("argument `device_family` is not a string: {:?}", v)),
            None => None
        };

        let peripheral = match args.get("peripheral") {
            Some(Value::String(s)) => Some(s.clone()),
            Some(v) => return Err(format!("argument `peripheral` is not a string: {:?}", v)),
            None => None
        };

        let example_type = match args.get("example_type") {
            Some(Value::String(s)) => Some(s.clone()),
            Some(v) => return Err(format!("argument `example_type` is not a string: {:?}", v)),
            None => None
        };

        let show_paths = match args.get("show_paths") {
            Some(Value::Bool(b)) => *b,
            Some(v) => return Err(format!("argument `show_paths` is not a boolean: {:?}", v)),
            None => true
        };

        // Get project directories from GlobalContext (the directories provided at instance creation)
        let gcx = ccx.lock().await.global_context.clone();
        let project_dirs = get_project_dirs(gcx.clone()).await;
        
        if project_dirs.is_empty() {
            return Err("No project directories configured. Please ensure workspace folders are set.".to_string());
        }
        
        let mut messages: Vec<String> = Vec::new();
        let mut context_files = Vec::new();
        let mut found_examples = Vec::new();
        
        // Search within each project directory for .projectspec files
        for project_dir in &project_dirs {
            let project_dir_str = project_dir.to_string_lossy().to_string();
            messages.push(format!("🔍 Searching for C2000 examples in: {}", project_dir_str));
            
            if !project_dir.exists() {
                messages.push(format!("⚠️ Directory does not exist: {}", project_dir_str));
                continue;
            }

            // Use walkdir to recursively find all .projectspec files within project directory
            for entry in walkdir::WalkDir::new(&project_dir)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
            {
                if let Some(file_name) = entry.file_name().to_str() {
                    if file_name.ends_with(".projectspec") {
                        let path = entry.path();
                        let path_str = path.to_string_lossy().to_string();
                        
                        // Extract metadata from path
                        match self.parse_example_path(&path_str, &project_dir_str) {
                            Ok(mut example_info) => {
                                // Apply filters
                                if let Some(ref family) = device_family {
                                    if !example_info.device_family.to_lowercase().contains(&family.to_lowercase()) {
                                        continue;
                                    }
                                }
                                
                                if let Some(ref peri) = peripheral {
                                    if !example_info.peripheral.to_lowercase().contains(&peri.to_lowercase()) {
                                        continue;
                                    }
                                }
                                
                                if let Some(ref etype) = example_type {
                                    if !example_info.example_type.to_lowercase().contains(&etype.to_lowercase()) {
                                        continue;
                                    }
                                }
                                
                                // Try to get example description
                                if let Some(desc) = self.get_example_description(&path_str).await {
                                    example_info.description = Some(desc);
                                }
                                
                                found_examples.push(example_info);
                            }
                            Err(_) => {
                                // Skip if we can't parse the path
                                continue;
                            }
                        }
                    }
                }
            }
        }
        
        // Sort examples
        found_examples.sort_by(|a, b| {
            a.example_type.cmp(&b.example_type)
                .then(a.device_family.cmp(&b.device_family))
                .then(a.peripheral.cmp(&b.peripheral))
                .then(a.name.cmp(&b.name))
        });
        
        // Group by category for better presentation
        let mut grouped: std::collections::HashMap<String, Vec<&ExampleInfo>> = std::collections::HashMap::new();
        for example in &found_examples {
            let key = format!("{} - {}", example.example_type, example.device_family);
            grouped.entry(key).or_insert_with(Vec::new).push(example);
        }
        
        // Format output with better structure
        let searched_dirs: Vec<String> = project_dirs.iter().map(|p| p.to_string_lossy().to_string()).collect();
        if found_examples.is_empty() {
            messages.push("❌ No examples found matching the specified criteria".to_string());
            messages.push(format!("🔍 Searched in: {}", searched_dirs.join(", ")));
            if device_family.is_some() || peripheral.is_some() || example_type.is_some() {
                messages.push("💡 Try removing filters or check if the project directory contains C2000 examples".to_string());
            }
        } else {
            messages.push(format!("✅ Found {} examples:", found_examples.len()));
            
            // Group and display
            let mut sorted_groups: Vec<_> = grouped.iter().collect();
            sorted_groups.sort_by_key(|(k, _)| *k);
            
            for (category, examples) in sorted_groups {
                messages.push(format!("\n📁 {} ({} examples):", category, examples.len()));
                for example in examples {
                    if show_paths {
                        messages.push(format!(
                            "  📄 {} - {} ({})",
                            example.name, example.peripheral, example.path
                        ));
                    } else {
                        messages.push(format!(
                            "  📄 {} - {}",
                            example.name, example.peripheral
                        ));
                    }
                    
                    // Show description if available
                    if let Some(ref desc) = example.description {
                        messages.push(format!("     💡 {}", desc));
                    }
                }
            }
            
            // Add filtering info
            let mut filter_info = Vec::new();
            if let Some(ref family) = device_family {
                filter_info.push(format!("Device Family: {}", family));
            }
            if let Some(ref peri) = peripheral {
                filter_info.push(format!("Peripheral: {}", peri));
            }
            if let Some(ref etype) = example_type {
                filter_info.push(format!("Type: {}", etype));
            }
            
            if !filter_info.is_empty() {
                messages.push(format!("\n🔍 Applied filters: {}", filter_info.join(", ")));
            }
        }

        // Add example info to context
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

#[derive(Clone)]
struct ExampleInfo {
    name: String,
    path: String,
    device_family: String,
    peripheral: String,
    example_type: String,
    description: Option<String>,
}

impl ToolC2000ExampleList {
    fn parse_example_path(&self, path: &str, base_path: &str) -> Result<ExampleInfo, String> {
        // Parse path like: /path/to/C2000Ware/driverlib/f28p65x/examples/c28x/spi/CCS/spi_ex1_loopback.projectspec
        let relative_path = path.strip_prefix(base_path)
            .ok_or("Path not under base path")?
            .trim_start_matches('/');
        
        let parts: Vec<&str> = relative_path.split('/').collect();
        
        if parts.is_empty() {
            return Err("Empty path".to_string());
        }
        
        // Determine example type
        let example_type = if relative_path.starts_with("driverlib/") {
            "driverlib"
        } else if relative_path.starts_with("device_support/") {
            "device_support"
        } else if relative_path.starts_with("training/") {
            "training"
        } else {
            "unknown"
        }.to_string();
        
        // Extract device family (usually after driverlib/device_support)
        let device_family = parts.get(1)
            .unwrap_or(&"unknown")
            .to_string();
        
        // Extract peripheral - look for common peripheral names in the path
        let known_peripherals = [
            "spi", "sci", "i2c", "adc", "epwm", "ecap", "eqep", "cputimer", 
            "mcan", "lin", "fsi", "usb", "dma", "led", "gpio", "interrupt", 
            "timer", "uart", "can", "pwm", "qep", "cap"
        ];
        
        let peripheral = parts.iter()
            .find(|p| {
                let p_lower = p.to_lowercase();
                known_peripherals.iter().any(|&periph| p_lower.contains(periph))
            })
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                // Fallback: try to extract from example name
                let name = std::path::Path::new(path)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("");
                known_peripherals.iter()
                    .find(|&&periph| name.to_lowercase().contains(periph))
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "unknown".to_string())
            });
        
        // Extract example name from filename
        let name = std::path::Path::new(path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();
        
        Ok(ExampleInfo {
            name,
            path: path.to_string(),
            device_family,
            peripheral,
            example_type,
            description: None,
        })
    }
    
    async fn get_example_description(&self, projectspec_path: &str) -> Option<String> {
        // Try to find and read README or example description
        let projectspec_path_obj = std::path::Path::new(projectspec_path);
        let example_dir = projectspec_path_obj.parent()?.parent()?;
        
        // Look for README.md or README.txt
        for readme_name in &["README.md", "README.txt", "readme.md", "readme.txt"] {
            let readme_path = example_dir.join(readme_name);
            if readme_path.exists() {
                if let Ok(content) = tokio::fs::read_to_string(&readme_path).await {
                    // Extract first meaningful line as description
                    for line in content.lines() {
                        let trimmed = line.trim();
                        // Skip empty lines, headers, and very short lines
                        if !trimmed.is_empty() 
                            && !trimmed.starts_with('#') 
                            && trimmed.len() > 20 
                            && trimmed.len() < 200 {
                            return Some(trimmed.to_string());
                        }
                    }
                }
            }
        }
        
        None
    }
}
