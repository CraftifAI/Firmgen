use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use serde_json::{Value, json};
use tokio::sync::Mutex as AMutex;
use std::path::Path;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::tools::tools_description::{Tool, ToolDesc, ToolParam, ToolSource, ToolSourceType};
use crate::call_validation::{ChatMessage, ChatContent, ContextEnum};

pub struct ToolC2000SysconfigModifyLlm {
    pub config_path: String,
}

#[async_trait]
impl Tool for ToolC2000SysconfigModifyLlm {
    fn as_any(&self) -> &dyn std::any::Any { self }

    fn tool_description(&self) -> ToolDesc {
        ToolDesc {
            name: "c2000_sysconfig_modify_llm".to_string(),
            display_name: "C2000 SysConfig Modify (LLM)".to_string(),
            source: ToolSource {
                source_type: ToolSourceType::Builtin,
                config_path: self.config_path.clone(),
            },
            agentic: true,
            experimental: true,
            description: "LLM-based fallback tool for modifying .syscfg files when the hardcoded c2000_sysconfig_modify tool fails or doesn't support a peripheral. Uses an LLM to dynamically analyze the .syscfg file structure and generate appropriate modifications. This tool should be used as a fallback when c2000_sysconfig_modify returns an error or doesn't support the requested peripheral type. The LLM will analyze existing patterns in the file and generate compatible code.".to_string(),
            parameters: vec![
                ToolParam {
                    name: "syscfg_file".to_string(),
                    param_type: "string".to_string(),
                    description: "Path to the .syscfg file to modify (relative to workspace root)".to_string(),
                },
                ToolParam {
                    name: "peripheral_type".to_string(),
                    param_type: "string".to_string(),
                    description: "Type of peripheral to add (e.g., 'uart_sci', 'spi', 'i2c', 'adc', 'epwm', 'ecap', 'eqep', 'cputimer', 'mcan', 'lin', 'fsi', 'usb', 'dma', 'led', or any other peripheral not yet supported by the hardcoded tool)".to_string(),
                },
                ToolParam {
                    name: "instance_name".to_string(),
                    param_type: "string".to_string(),
                    description: "Instance name for the peripheral (e.g., 'mySCIA', 'mySPIA', 'myLED0'). If not provided, a default name will be generated".to_string(),
                },
                ToolParam {
                    name: "device".to_string(),
                    param_type: "string".to_string(),
                    description: "Target C2000 device (e.g., 'F28P65x', 'F28004x', 'F28002x', 'F28388D')".to_string(),
                },
                ToolParam {
                    name: "board".to_string(),
                    param_type: "string".to_string(),
                    description: "Board type (e.g., 'LAUNCHXL-F28P65', 'LAUNCHXL_F280049C', 'CONTROLCARD-F280049C')".to_string(),
                },
                ToolParam {
                    name: "baud_rate".to_string(),
                    param_type: "number".to_string(),
                    description: "Baud rate for UART/SCI (default: 9600, common values: 9600, 115200)".to_string(),
                },
                ToolParam {
                    name: "rx_pin".to_string(),
                    param_type: "string".to_string(),
                    description: "RX pin assignment for UART/SCI (e.g., 'GPIO43', 'boosterpack2.43')".to_string(),
                },
                ToolParam {
                    name: "tx_pin".to_string(),
                    param_type: "string".to_string(),
                    description: "TX pin assignment for UART/SCI (e.g., 'GPIO42', 'boosterpack2.44')".to_string(),
                },
                ToolParam {
                    name: "additional_requirements".to_string(),
                    param_type: "string".to_string(),
                    description: "Additional requirements or configuration details for the peripheral (e.g., 'enable interrupts', 'use DMA', 'configure for master mode')".to_string(),
                },
            ],
            parameters_required: vec!["syscfg_file".to_string(), "peripheral_type".to_string()],
        }
    }

    async fn tool_execute(
        &mut self,
        _ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>,
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        // Parse parameters
        let syscfg_file = match args.get("syscfg_file") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => return Err(format!("argument `syscfg_file` is not a string: {:?}", v)),
            None => return Err("Missing argument `syscfg_file`".to_string())
        };

        let peripheral_type = match args.get("peripheral_type") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => return Err(format!("argument `peripheral_type` is not a string: {:?}", v)),
            None => return Err("Missing argument `peripheral_type`".to_string())
        };
        
        let device = args.get("device").and_then(|v| v.as_str()).map(|s| s.to_string());
        let board = args.get("board").and_then(|v| v.as_str()).map(|s| s.to_string());
        let instance_name = args.get("instance_name").and_then(|v| v.as_str()).map(|s| s.to_string());
        let baud_rate = args.get("baud_rate").and_then(|v| v.as_u64());
        let rx_pin = args.get("rx_pin").and_then(|v| v.as_str()).map(|s| s.to_string());
        let tx_pin = args.get("tx_pin").and_then(|v| v.as_str()).map(|s| s.to_string());
        let additional_requirements = args.get("additional_requirements").and_then(|v| v.as_str()).map(|s| s.to_string());

        let mut output = String::new();

        // Resolve syscfg file path
        let syscfg_path = Path::new(&syscfg_file);
        
        // Check if file exists
        if !syscfg_path.exists() {
            return Err(format!("Error: SysConfig file not found: {}", syscfg_file));
        }

        // Read the original syscfg file
        let syscfg_content = std::fs::read_to_string(syscfg_path)
            .map_err(|e| format!("Failed to read SysConfig file: {}", e))?;

        // Try to load recipe context for better LLM understanding
        let recipe_context = load_recipe_context_for_llm(&peripheral_type).await;

        // Build LLM prompt
        let prompt = build_llm_prompt(
            &syscfg_content,
            &peripheral_type,
            instance_name.as_deref(),
            device.as_deref(),
            board.as_deref(),
            baud_rate,
            rx_pin.as_deref(),
            tx_pin.as_deref(),
            additional_requirements.as_deref(),
            recipe_context.as_deref(),
        );

        // Call LLM via HTTP endpoint
        output.push_str(&format!("🤖 Using LLM to generate {} configuration...\n", peripheral_type));
        
        match call_llm_for_sysconfig_modification(&prompt).await {
            Ok(modified_content) => {
                // Validate that we got a modified file (not just an explanation)
                if modified_content.trim().is_empty() {
                    return Err("LLM returned empty response".to_string());
                }

                // Basic validation: check if it looks like a .syscfg file
                if !modified_content.contains("scripting.addModule") && !modified_content.contains("@cliArgs") {
                    return Err("LLM response doesn't appear to be a valid .syscfg file. It may have returned an explanation instead of code.".to_string());
                }

                // Write the modified content back
                std::fs::write(syscfg_path, &modified_content)
                    .map_err(|e| format!("Failed to write modified SysConfig file: {}", e))?;
                
                output.push_str(&format!("✅ Successfully modified {} using LLM\n", syscfg_file));
                output.push_str("Note: Board files (board.c/board.h) will be regenerated automatically on next build\n");
                output.push_str("⚠️  Please review the changes carefully as LLM-generated code may need adjustments\n");
            }
            Err(e) => {
                return Err(format!("LLM call failed: {}", e));
            }
        }

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

// Load recipe context for the LLM to understand the peripheral structure
async fn load_recipe_context_for_llm(peripheral_type: &str) -> Option<String> {
    // Try to load recipes from HTTP endpoint
    let api_url = "http://localhost:8002/v1/c2000-sysconfig-recipe";
    
    match reqwest::get(api_url).await {
        Ok(response) => {
            if response.status().is_success() {
                match response.json::<Value>().await {
                    Ok(json) => {
                        // Map peripheral_type to recipe key
                        let recipe_key = map_peripheral_type_to_recipe_key(peripheral_type);
                        
                        if let Some(peripherals) = json.get("peripherals") {
                            if let Some(peripheral) = peripherals.get(&recipe_key) {
                                // Extract relevant information for LLM context
                                if let (Some(module_path), Some(module_var), Some(instance_var), Some(instance_name), Some(imports), Some(config)) = (
                                    peripheral.get("module_path").and_then(|v| v.as_str()),
                                    peripheral.get("module_var").and_then(|v| v.as_str()),
                                    peripheral.get("instance_var").and_then(|v| v.as_str()),
                                    peripheral.get("instance_name").and_then(|v| v.as_str()),
                                    peripheral.get("imports").and_then(|v| v.as_array()),
                                    peripheral.get("config").and_then(|v| v.as_array()),
                                ) {
                                    let mut context = format!(
                                        "Recipe for {}:\nModule path: {}\nModule var: {}\nInstance var: {}\nInstance name: {}\n\n",
                                        recipe_key, module_path, module_var, instance_var, instance_name
                                    );
                                    
                                    context.push_str("Imports:\n");
                                    for import in imports {
                                        if let Some(s) = import.as_str() {
                                            context.push_str(&format!("  {}\n", s));
                                        }
                                    }
                                    
                                    context.push_str("\nConfig:\n");
                                    for cfg in config {
                                        if let Some(s) = cfg.as_str() {
                                            context.push_str(&format!("  {}\n", s));
                                        }
                                    }
                                    
                                    return Some(context);
                                }
                            }
                        }
                    }
                    Err(_) => {}
                }
            }
        }
        Err(_) => {}
    }
    
    None
}

// Map user-friendly peripheral_type to recipe key
fn map_peripheral_type_to_recipe_key(peripheral_type: &str) -> String {
    match peripheral_type.to_lowercase().as_str() {
        "uart_sci" | "sci" => "SCI_SCIA".to_string(),
        "spi" | "spia" => "SPIA".to_string(),
        "spib" => "SPIB".to_string(),
        "i2c" | "i2ca" => "I2CA".to_string(),
        "adc" | "adca" => "ADC_ADCA".to_string(),
        "epwm" | "epwm1" => "EPWM1".to_string(),
        "ecap" | "ecap1" => "ECAP1".to_string(),
        "eqep" | "eqep1" => "EQEP1".to_string(),
        "cputimer" | "cputimer0" => "CPUTIMER0".to_string(),
        "mcan" | "mcan0" => "MCAN0".to_string(),
        "lin" | "lina" => "LINA".to_string(),
        "fsi" => "FSI".to_string(),
        "usb" | "usb0" => "USB0".to_string(),
        "dma" | "dma0" => "DMA0".to_string(),
        "led" => "LED_BOARD".to_string(),
        _ => peripheral_type.to_uppercase(),
    }
}

// Build LLM prompt for sysconfig modification
fn build_llm_prompt(
    syscfg_content: &str,
    peripheral_type: &str,
    instance_name: Option<&str>,
    device: Option<&str>,
    board: Option<&str>,
    baud_rate: Option<u64>,
    rx_pin: Option<&str>,
    tx_pin: Option<&str>,
    additional_requirements: Option<&str>,
    recipe_context: Option<&str>,
) -> String {
    let mut prompt = String::new();
    
    prompt.push_str("You are an expert in TI C2000 SysConfig file modification. Your task is to modify a .syscfg file to add a new peripheral module.\n\n");
    
    prompt.push_str("## Current .syscfg File Content:\n```javascript\n");
    prompt.push_str(syscfg_content);
    prompt.push_str("\n```\n\n");
    
    prompt.push_str("## Task:\n");
    prompt.push_str(&format!("Add a {} peripheral to this .syscfg file.\n\n", peripheral_type));
    
    if let Some(inst_name) = instance_name {
        prompt.push_str(&format!("- Instance name: {}\n", inst_name));
    }
    if let Some(dev) = device {
        prompt.push_str(&format!("- Device: {}\n", dev));
    }
    if let Some(b) = board {
        prompt.push_str(&format!("- Board: {}\n", b));
    }
    if let Some(baud) = baud_rate {
        prompt.push_str(&format!("- Baud rate: {}\n", baud));
    }
    if let Some(rx) = rx_pin {
        prompt.push_str(&format!("- RX pin: {}\n", rx));
    }
    if let Some(tx) = tx_pin {
        prompt.push_str(&format!("- TX pin: {}\n", tx));
    }
    if let Some(req) = additional_requirements {
        prompt.push_str(&format!("- Additional requirements: {}\n", req));
    }
    
    prompt.push_str("\n## Requirements:\n");
    prompt.push_str("1. Analyze the existing .syscfg file structure and patterns\n");
    prompt.push_str("2. Add the module import (using `scripting.addModule`) in the imports section (typically after other module imports, before configuration blocks)\n");
    prompt.push_str("3. Add the instance creation (using `.addInstance()`) immediately after the module import\n");
    prompt.push_str("4. Add the configuration block with all required properties\n");
    prompt.push_str("5. Follow the exact same code style and formatting as the existing file\n");
    prompt.push_str("6. Ensure proper pin assignments based on device/board if specified\n");
    prompt.push_str("7. Do NOT modify any existing code, only ADD new code\n");
    prompt.push_str("8. Return ONLY the complete modified .syscfg file content, no explanations or markdown formatting\n");
    
    if let Some(recipe) = recipe_context {
        prompt.push_str("\n## Reference Recipe (if available):\n");
        prompt.push_str(recipe);
        prompt.push_str("\nUse this as a reference, but adapt it to match the existing file's style.\n");
    }
    
    prompt.push_str("\n## Important Notes:\n");
    prompt.push_str("- The file uses JavaScript-like syntax for SysConfig\n");
    prompt.push_str("- Module imports use: `const moduleVar = scripting.addModule(\"/path/to/module.js\", {}, false);`\n");
    prompt.push_str("- Instance creation uses: `const instanceVar = moduleVar.addInstance();`\n");
    prompt.push_str("- Configuration uses: `instanceVar.property = value;`\n");
    prompt.push_str("- Pin assignments use: `instanceVar.pin.$assign = \"GPIO42\";`\n");
    prompt.push_str("- Instance names use: `instanceVar.$name = \"myInstance\";`\n");
    prompt.push_str("- Preserve all existing @cliArgs, comments, and structure\n");
    
    prompt.push_str("\nReturn the complete modified .syscfg file content:\n");
    
    prompt
}

// Call LLM via HTTP endpoint
async fn call_llm_for_sysconfig_modification(prompt: &str) -> Result<String, String> {
    let api_url = "http://localhost:8002/v1/chat/completions";
    
    // Build the request payload
    let request_body = json!({
        "model": "gpt-4o-mini", // Use a capable model
        "messages": [
            {
                "role": "system",
                "content": "You are an expert in TI C2000 SysConfig file modification. You modify .syscfg files by adding peripheral modules following existing patterns. You return only the complete modified file content, no explanations."
            },
            {
                "role": "user",
                "content": prompt
            }
        ],
        "temperature": 0.1, // Low temperature for more deterministic code generation
        "max_tokens": 8000, // Enough for a complete .syscfg file
        "stream": false
    });
    
    let client = reqwest::Client::new();
    let response = client
        .post(api_url)
        .json(&request_body)
        .send()
        .await
        .map_err(|e| format!("Failed to send request to LLM endpoint: {}", e))?;
    
    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("LLM endpoint returned error status {}: {}", status, error_text));
    }
    
    let response_json: Value = response.json()
        .await
        .map_err(|e| format!("Failed to parse LLM response JSON: {}", e))?;
    
    // Extract the content from the response
    // OpenAI format: response.choices[0].message.content
    let content = response_json
        .get("choices")
        .and_then(|c| c.as_array())
        .and_then(|arr| arr.get(0))
        .and_then(|choice| choice.get("message"))
        .and_then(|msg| msg.get("content"))
        .and_then(|c| c.as_str())
        .ok_or("Failed to extract content from LLM response")?;
    
    // Clean up the response - remove markdown code blocks if present
    let mut cleaned_content = content.to_string();
    
    // Remove markdown code blocks
    if cleaned_content.starts_with("```") {
        // Find the first newline after ```
        if let Some(start) = cleaned_content.find('\n') {
            cleaned_content = cleaned_content[start + 1..].to_string();
        }
    }
    if cleaned_content.ends_with("```") {
        // Remove trailing ```
        let len = cleaned_content.len();
        if len >= 3 {
            cleaned_content = cleaned_content[..len - 3].trim_end().to_string();
        }
    }
    
    // Also handle ```javascript or ```js
    if cleaned_content.starts_with("```javascript") {
        if let Some(start) = cleaned_content.find('\n') {
            cleaned_content = cleaned_content[start + 1..].to_string();
        }
    }
    if cleaned_content.starts_with("```js") {
        if let Some(start) = cleaned_content.find('\n') {
            cleaned_content = cleaned_content[start + 1..].to_string();
        }
    }
    
    Ok(cleaned_content)
}

