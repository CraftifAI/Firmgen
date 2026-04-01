use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::Mutex as AMutex;
use std::path::Path;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::tools::tools_description::{Tool, ToolDesc, ToolParam, ToolSource, ToolSourceType};
use crate::call_validation::{ChatMessage, ChatContent, ContextEnum};

use super::config::C2000Config;

// Recipe data structures (simplified - only what we need)
#[derive(Debug, Clone)]
struct PeripheralRecipe {
    module_path: String,
    module_var: String,
    instance_var: String,
    instance_name: String,
    imports: Vec<String>,
    config: Vec<String>,
}

#[derive(Debug)]
struct SysConfigRecipes {
    peripherals: HashMap<String, PeripheralRecipe>,
}

pub struct ToolC2000SysconfigModify {
    pub config_path: String,
}

#[async_trait]
impl Tool for ToolC2000SysconfigModify {
    fn as_any(&self) -> &dyn std::any::Any { self }

    fn tool_description(&self) -> ToolDesc {
        ToolDesc {
            name: "c2000_sysconfig_modify".to_string(),
            display_name: "C2000 SysConfig Modify".to_string(),
            source: ToolSource {
                source_type: ToolSourceType::Builtin,
                config_path: self.config_path.clone(),
            },
            agentic: true,
            experimental: false,
            description: "Modifies an existing .syscfg file to add peripheral modules to TI C2000 projects. Supports: 'uart_sci' (UART/SCI), 'spi' (SPI), 'i2c' (I2C), 'adc' (ADC), 'epwm' (ePWM), 'ecap' (eCAP), 'eqep' (eQEP), 'cputimer' (CPU Timer), 'mcan' (MCAN), 'lin' (LIN), 'fsi' (FSI), 'usb' (USB), 'dma' (DMA), and 'led' (board LED). The tool uses recipe-based configuration loaded from HTTP endpoint, with fallback to hardcoded logic. Automatically generates JavaScript module imports and instance configurations with proper pin assignments. After modification, board files (board.c/board.h) are regenerated on next CCS build.".to_string(),
            parameters: vec![
                ToolParam {
                    name: "syscfg_file".to_string(),
                    param_type: "string".to_string(),
                    description: "Path to the .syscfg file to modify (relative to workspace root)".to_string(),
                },
                ToolParam {
                    name: "peripheral_type".to_string(),
                    param_type: "string".to_string(),
                    description: "Type of peripheral to add. Supported: 'uart_sci' or 'sci' (UART/SCI), 'spi' (SPI), 'i2c' (I2C), 'adc' (ADC), 'epwm' (ePWM), 'ecap' (eCAP), 'eqep' (eQEP), 'cputimer' (CPU Timer), 'mcan' (MCAN), 'lin' (LIN), 'fsi' (FSI), 'usb' (USB), 'dma' (DMA), 'led' (board LED). Uses recipe-based configuration for proper pin assignments and module setup.".to_string(),
                },
                ToolParam {
                    name: "instance_name".to_string(),
                    param_type: "string".to_string(),
                    description: "Instance name for the peripheral (e.g., 'mySCIA', 'mySPIA', 'myLED0'). If not provided, a default name will be generated based on peripheral type".to_string(),
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
                    description: "RX pin assignment for UART/SCI. Can be GPIO number (e.g., 'GPIO43' for F28P65x LaunchPad) or SysConfig pin name (e.g., 'boosterpack2.43' for F28004x, 'boosterpack1.3' for F28002x). For F28P65x LaunchPad, use 'GPIO43' for backchannel UART.".to_string(),
                },
                ToolParam {
                    name: "tx_pin".to_string(),
                    param_type: "string".to_string(),
                    description: "TX pin assignment for UART/SCI. Can be GPIO number (e.g., 'GPIO42' for F28P65x LaunchPad) or SysConfig pin name. For F28P65x LaunchPad, use 'GPIO42' for backchannel UART.".to_string(),
                },
                ToolParam {
                    name: "operation".to_string(),
                    param_type: "string".to_string(),
                    description: "Operation to perform: 'add_peripheral' (default) to add a new peripheral module, 'list_supported' to show available peripherals. Note: 'modify_pins' is not yet implemented - pin modifications must be done manually in the .syscfg file or via SysConfig GUI.".to_string(),
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
        
        let operation = match args.get("operation") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => return Err(format!("argument `operation` is not a string: {:?}", v)),
            None => "add_peripheral".to_string()
        };
        
        let device = args.get("device").and_then(|v| v.as_str()).map(|s| s.to_string());
        let board = args.get("board").and_then(|v| v.as_str()).map(|s| s.to_string());
        let instance_name = args.get("instance_name").and_then(|v| v.as_str()).map(|s| s.to_string());
        let baud_rate = args.get("baud_rate").and_then(|v| v.as_u64());
        let rx_pin = args.get("rx_pin").and_then(|v| v.as_str()).map(|s| s.to_string());
        let tx_pin = args.get("tx_pin").and_then(|v| v.as_str()).map(|s| s.to_string());

        let mut output = String::new();

        // Load C2000 configuration
        let config = C2000Config::load_from_api("http://localhost:8002/v1/c2000-config").await
            .map_err(|e| format!("Failed to load C2000 config: {}", e))?;

        // Resolve path variables (e.g., $C2000WARE)
        let resolved_syscfg_file = config.resolve_path_variables(&syscfg_file);
        
        // Resolve syscfg file path
        let syscfg_path = Path::new(&resolved_syscfg_file);
        
        // Check if file exists
        if !syscfg_path.exists() {
            return Err(format!("Error: SysConfig file not found: {}", resolved_syscfg_file));
        }

        // Get safe path for modification (copies to CCS workspace if needed)
        let safe_path = config.get_safe_modification_path(&resolved_syscfg_file)
            .map_err(|e| format!("Failed to ensure file in CCS workspace: {}", e))?;
        
        if safe_path != syscfg_path {
            output.push_str(&format!("📋 Copied file to CCS workspace: {}\n", safe_path.display()));
            output.push_str(&format!("   (Original preserved at: {})\n", resolved_syscfg_file));
        }

        // Read the file (from CCS workspace if it was copied)
        let syscfg_content = std::fs::read_to_string(&safe_path)
            .map_err(|e| format!("Failed to read SysConfig file: {}", e))?;

        // Process based on operation type
        match operation.as_str() {
            "add_peripheral" => {
                // Add peripheral logic - try recipe-based first, then fallback to hardcoded
                match add_peripheral_to_syscfg(
                    &syscfg_content, 
                    &peripheral_type, 
                    instance_name.as_deref(),
                    device.as_deref(),
                    board.as_deref(),
                    baud_rate,
                    rx_pin.as_deref(),
                    tx_pin.as_deref(),
                ).await {
                    Ok(modified_content) => {
                        // Write the modified content to safe path (CCS workspace)
                        std::fs::write(&safe_path, &modified_content)
                            .map_err(|e| format!("Failed to write modified SysConfig file: {}", e))?;
                        
                        output.push_str(&format!("✅ Successfully added {} peripheral to {}\n", peripheral_type, safe_path.display()));
                        output.push_str("Note: Board files (board.c/board.h) will be regenerated automatically on next build\n");
                    }
                    Err(e) => {
                        return Err(format!("Failed to add peripheral: {}", e));
                    }
                }
            }
            "modify_pins" => {
                output.push_str("Pin modification is currently not yet implemented\n");
                output.push_str("Please edit the .syscfg file manually to change pin assignments\n");
            }
            "list_supported" => {
                // Try to load recipes to show actual supported peripherals
                match load_sysconfig_recipes().await {
                    Ok(recipes) => {
                        output.push_str("✅ Supported peripherals (from recipes):\n\n");
                        let mut peripheral_list: Vec<&String> = recipes.peripherals.keys().collect();
                        peripheral_list.sort();
                        for key in peripheral_list {
                            let recipe = &recipes.peripherals[key];
                            output.push_str(&format!("  - {}: {} ({})\n", 
                                map_recipe_key_to_user_name(key),
                                recipe.instance_name,
                                recipe.module_path
                            ));
                        }
                        output.push_str("\n💡 Use the peripheral_type parameter with the user-friendly name (e.g., 'uart_sci', 'spi', 'i2c').\n");
                    }
                    Err(_) => {
                        // Fallback to hardcoded list
                        output.push_str("✅ Currently Supported (hardcoded fallback):\n");
                        output.push_str("  - uart_sci: UART/SCI serial communication\n");
                        output.push_str("  - led: Board LED control\n");
                        output.push_str("  - cputimer: CPU timer with interrupt support\n");
                        output.push_str("\n⚠️  Recipe loading failed - using limited hardcoded support.\n");
                    }
                }
            }
            _ => {
                return Err(format!("Unknown operation: {}", operation));
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

async fn add_peripheral_to_syscfg(
    content: &str,
    peripheral_type: &str,
    instance_name: Option<&str>,
    device: Option<&str>,
    board: Option<&str>,
    baud_rate: Option<u64>,
    rx_pin: Option<&str>,
    tx_pin: Option<&str>,
) -> Result<String, String> {
    let mut modified = content.to_string();
    
    // Update board token in @cliArgs if board parameter is provided
    if let Some(board_name) = board {
        modified = update_board_token(&modified, board_name, device)?;
    }
    
    // Map user-friendly peripheral_type to recipe key
    let recipe_key = map_peripheral_type_to_recipe_key(peripheral_type);
    
    // Try to load and use recipe first
    if let Ok(recipes) = load_sysconfig_recipes().await {
        if let Some(recipe) = recipes.peripherals.get(&recipe_key) {
            // Use recipe-based approach
            return apply_recipe_to_syscfg(
                &modified,
                recipe,
                instance_name,
                peripheral_type,
                baud_rate,
                rx_pin,
                tx_pin,
            );
        }
    }
    
    // Fallback to hardcoded logic for backwards compatibility
    // Determine instance name if not provided
    let inst_name = instance_name.unwrap_or_else(|| {
        match peripheral_type {
            "uart_sci" => "mySCIA",
            "spi" => "mySPIA",
            "i2c" => "myI2CA",
            "epwm" => "myEPWM1",
            "adc" => "myADCA",
            "led" => "myBoardLED0",
            "cputimer" => "myCPUTIMER0",
            _ => "myInstance"
        }
    });

    match peripheral_type {
        "uart_sci" | "sci" => {
            // Check if SCI module already exists (more precise check)
            // Look for module declaration pattern: "const sci = scripting.addModule"
            if content.contains("const sci") && content.contains("scripting.addModule(\"/driverlib/sci.js\"") {
                return Err("SCI module already exists in this SysConfig file".to_string());
            }

            // Generate SCI configuration
            let baud = baud_rate.unwrap_or(9600);
            
            // Determine RX/TX pins based on board/device
            let (rx, tx) = determine_uart_pins(device, board, rx_pin, tx_pin)?;

            // Insert SCI module imports in imports section, config in config section
            // For now, insert both together after memcfg (typical last import)
            let insert_pos = find_insertion_point(&modified);
            
            let sci_config = format!(
r#"
const sci  = scripting.addModule("/driverlib/sci.js", {{}}, false);
const sci1 = sci.addInstance();

sci1.$name                 = "{inst_name}";
sci1.loopback              = false;
sci1.baudRates             = {baud};
sci1.sci.$assign           = "SCIA";
sci1.sci.sci_rxPin.$assign = "{rx}";
sci1.sci.sci_txPin.$assign = "{tx}";
"#,
                inst_name = inst_name, 
                baud = baud, 
                rx = rx, 
                tx = tx
            );

            modified.insert_str(insert_pos, &sci_config);
        }
        "led" => {
            // LED configuration logic
            // Check for LED module declaration pattern
            if content.contains("const led") && content.contains("scripting.addModule(\"/driverlib/board_components/led\"") {
                return Err("LED module already exists in this SysConfig file".to_string());
            }

            let led_config = format!(
r#"
const led  = scripting.addModule("/driverlib/board_components/led", {{}}, false);
const led1 = led.addInstance();

led1.$name     = "{inst_name}";
led1.$hardware = system.deviceData.board.components.LED5;
"#,
                inst_name = inst_name
            );

            let insert_pos = find_insertion_point(&modified);
            modified.insert_str(insert_pos, &led_config);
        }
        "cputimer" => {
            // CPU Timer configuration
            // Check for CPUTimer module declaration pattern
            if content.contains("const cputimer") && content.contains("scripting.addModule(\"/driverlib/cputimer.js\"") {
                return Err("CPUTimer module already exists in this SysConfig file".to_string());
            }

            let cputimer_config = format!(
r#"
const cputimer  = scripting.addModule("/driverlib/cputimer.js", {{}}, false);
const cputimer1 = cputimer.addInstance();

cputimer1.$name                    = "{inst_name}";
cputimer1.timerPrescaler           = 1;
cputimer1.enableInterrupt          = true;
cputimer1.registerInterrupts       = true;
cputimer1.timerPeriod              = 100000000;
cputimer1.timerInt.enableInterrupt = true;
"#,
                inst_name = inst_name
            );

            let insert_pos = find_insertion_point(&modified);
            modified.insert_str(insert_pos, &cputimer_config);
        }
        _ => {
            return Err(format!("Peripheral type '{}' not yet supported for automated addition", peripheral_type));
        }
    }

    Ok(modified)
}

fn determine_uart_pins(
    device: Option<&str>,
    board: Option<&str>,
    rx_pin: Option<&str>,
    tx_pin: Option<&str>,
) -> Result<(String, String), String> {
    // If pins are explicitly provided, use them
    if let (Some(rx), Some(tx)) = (rx_pin, tx_pin) {
        return Ok((rx.to_string(), tx.to_string()));
    }

    // Determine pins based on board/device
    match board {
        Some(b) if b.contains("LAUNCHXL_F280049C") || b.contains("F28004x") => {
            Ok(("boosterpack2.43".to_string(), "boosterpack2.44".to_string()))
        }
        Some(b) if b.contains("F28002x") => {
            Ok(("boosterpack1.3".to_string(), "boosterpack1.4".to_string()))
        }
        Some(b) if b.contains("LAUNCHXL") && (b.contains("F28P65") || b.contains("F28P55")) => {
            // F28P65x/F28P55x LaunchPad backchannel UART uses GPIO42/43 directly
            Ok(("GPIO43".to_string(), "GPIO42".to_string()))
        }
        _ => {
            // Try to determine from device
            match device {
                Some(d) if d.contains("F28P65") || d.contains("F28P55") => {
                    // For F28P65x/F28P55x, check if LaunchPad board was specified
                    // If board not specified or not LaunchPad, default to hsecDigital
                    if let Some(b) = board {
                        if b.contains("LAUNCHXL") {
                            Ok(("GPIO43".to_string(), "GPIO42".to_string()))
                        } else {
                            Ok(("hsecDigital.74".to_string(), "hsecDigital.75".to_string()))
                        }
                    } else {
                        // No board specified, default to LaunchPad GPIO pins
                        Ok(("GPIO43".to_string(), "GPIO42".to_string()))
                    }
                }
                _ => {
                    Err("Could not determine UART pins. Please specify rx_pin and tx_pin parameters".to_string())
                }
            }
        }
    }
}

fn find_insertion_point(content: &str) -> usize {
    // Strategy: Insert module imports in the imports section, config in the config section
    
    // First, try to find the end of the imports section (after memcfg or last module import)
    // Look for memcfg as it's typically the last module import
    if let Some(memcfg_pos) = content.find("const memcfg") {
        // Find the end of the memcfg line
        if let Some(newline_pos) = content[memcfg_pos..].find('\n') {
            let insert_pos = memcfg_pos + newline_pos + 1;
            // Check if we're still in the imports section (before "Write custom configuration")
            if let Some(config_start) = content.find("Write custom configuration") {
                if insert_pos < config_start {
                    return insert_pos;
                }
            } else {
                return insert_pos;
            }
        }
    }
    
    // Fallback: Find the last "const X = scripting.addModule" in imports section
    let lines: Vec<&str> = content.lines().collect();
    let mut last_import_line = 0;
    for (i, line) in lines.iter().enumerate() {
        if line.contains("scripting.addModule") && !line.contains("Write custom") {
            last_import_line = i;
        }
        // Stop at config section
        if line.contains("Write custom configuration") {
            break;
        }
    }
    
    if last_import_line > 0 {
        // Calculate position: sum of lengths of all lines up to and including last_import_line
        let mut pos = 0;
        for (i, line) in lines.iter().enumerate() {
            if i <= last_import_line {
                pos += line.len() + 1; // +1 for newline
            } else {
                break;
            }
        }
        return pos;
    }
    
    // Fallback: insert before the first "Write custom configuration" comment
    if let Some(pos) = content.find("Write custom configuration") {
        return pos;
    }
    
    // Last resort: find a reasonable spot after header comments
    if let Some(pos) = content.find("const ") {
        // Find the end of this statement
        if let Some(newline_pos) = content[pos..].find('\n') {
            return pos + newline_pos + 1;
        }
    }
    
    content.len() / 2 // Middle of file as absolute last resort
}

fn update_board_token(content: &str, board: &str, _device: Option<&str>) -> Result<String, String> {
    // Normalize board name to SysConfig format
    // Convert "LAUNCHXL-F28P65x" -> "/boards/LAUNCHXL_F28P65X"
    let normalized_board = normalize_board_token(board);
    
    // Find and update @cliArgs line
    let lines: Vec<&str> = content.lines().collect();
    let mut updated_lines = Vec::new();
    let mut found_cliargs = false;
    
    for line in lines {
        if line.contains("@cliArgs") {
            found_cliargs = true;
            // Extract existing @cliArgs and update board token
            if let Some(board_start) = line.find("--board") {
                // Find the board value (between quotes)
                let after_board = &line[board_start..];
                if let Some(quote1) = after_board.find('"') {
                    let after_quote1 = &after_board[quote1 + 1..];
                    if let Some(quote2) = after_quote1.find('"') {
                        // Replace the board value
                        let before = &line[..board_start + quote1 + 1];
                        let after = &after_quote1[quote2..];
                        let updated_line = format!("{}{}{}", before, normalized_board, after);
                        updated_lines.push(updated_line);
                        continue;
                    }
                }
            }
            // If we couldn't parse it, try to add/update --board parameter
            if !line.contains("--board") {
                // Add --board parameter before --device or at end
                let mut new_line = line.to_string();
                if let Some(device_pos) = line.find("--device") {
                    new_line.insert_str(device_pos, &format!("--board \"{}\" ", normalized_board));
                } else {
                    new_line.push_str(&format!(" --board \"{}\"", normalized_board));
                }
                updated_lines.push(new_line);
            } else {
                updated_lines.push(line.to_string());
            }
        } else {
            updated_lines.push(line.to_string());
        }
    }
    
    // If @cliArgs not found, we could add it, but that's complex - just return original
    if !found_cliargs {
        // Don't fail, just return original content
        return Ok(content.to_string());
    }
    
    Ok(updated_lines.join("\n"))
}

fn normalize_board_token(board: &str) -> String {
    // Convert various board name formats to SysConfig token format
    // Examples:
    // "LAUNCHXL-F28P65x" -> "/boards/LAUNCHXL_F28P65X"
    // "LAUNCHXL_F28P65X" -> "/boards/LAUNCHXL_F28P65X"
    // "LAUNCHXL_F280049C" -> "/boards/LAUNCHXL_F280049C"
    
    let normalized = board
        .replace("-", "_")
        .to_uppercase();
    
    // Remove common prefixes if present
    let normalized = if normalized.starts_with("/BOARDS/") {
        normalized
    } else if normalized.starts_with("BOARDS/") {
        format!("/{}", normalized)
    } else {
        format!("/boards/{}", normalized)
    };
    
    normalized
}

// Load SysConfig recipes from HTTP endpoint or file fallback
async fn load_sysconfig_recipes() -> Result<SysConfigRecipes, String> {
    // Try HTTP endpoint first
    let api_url = "http://localhost:8002/v1/c2000-sysconfig-recipe";
    
    match reqwest::get(api_url).await {
        Ok(response) => {
            if response.status().is_success() {
                match response.json::<Value>().await {
                    Ok(json) => {
                        return parse_recipes_from_json(&json);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to parse recipe JSON from API: {}", e);
                    }
                }
            } else {
                tracing::warn!("Recipe API returned error status: {}", response.status());
            }
        }
        Err(e) => {
            tracing::warn!("Failed to fetch recipes from API: {}", e);
        }
    }
    
    // Fallback to local file
    let recipe_path = Path::new("/home/shubham/sdk_agent/refact/sysconfig/launchxl_f28p65x_syscfg_recipes.json");
    if recipe_path.exists() {
        match tokio::fs::read_to_string(recipe_path).await {
            Ok(content) => {
                match serde_json::from_str::<Value>(&content) {
                    Ok(json) => {
                        return parse_recipes_from_json(&json);
                    }
                    Err(e) => {
                        return Err(format!("Failed to parse recipe JSON file: {}", e));
                    }
                }
            }
            Err(e) => {
                return Err(format!("Failed to read recipe file: {}", e));
            }
        }
    }
    
    Err("Could not load recipes from API or file".to_string())
}

// Parse recipes from JSON Value
fn parse_recipes_from_json(json: &Value) -> Result<SysConfigRecipes, String> {
    let peripherals_json = json.get("peripherals")
        .ok_or("Missing 'peripherals' key in recipe JSON")?
        .as_object()
        .ok_or("'peripherals' is not an object")?;
    
    let mut peripherals = HashMap::new();
    
    for (key, value) in peripherals_json {
        let recipe_obj = value.as_object()
            .ok_or(format!("Peripheral '{}' is not an object", key))?;
        
        let module_path = recipe_obj.get("module_path")
            .and_then(|v| v.as_str())
            .ok_or(format!("Missing module_path for {}", key))?
            .to_string();
        
        let module_var = recipe_obj.get("module_var")
            .and_then(|v| v.as_str())
            .ok_or(format!("Missing module_var for {}", key))?
            .to_string();
        
        let instance_var = recipe_obj.get("instance_var")
            .and_then(|v| v.as_str())
            .ok_or(format!("Missing instance_var for {}", key))?
            .to_string();
        
        let instance_name = recipe_obj.get("instance_name")
            .and_then(|v| v.as_str())
            .ok_or(format!("Missing instance_name for {}", key))?
            .to_string();
        
        let imports = recipe_obj.get("imports")
            .and_then(|v| v.as_array())
            .ok_or(format!("Missing or invalid imports for {}", key))?
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();
        
        let config = recipe_obj.get("config")
            .and_then(|v| v.as_array())
            .ok_or(format!("Missing or invalid config for {}", key))?
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();
        
        peripherals.insert(key.clone(), PeripheralRecipe {
            module_path,
            module_var,
            instance_var,
            instance_name,
            imports,
            config,
        });
    }
    
    Ok(SysConfigRecipes { peripherals })
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

// Map recipe key back to user-friendly name
fn map_recipe_key_to_user_name(recipe_key: &str) -> String {
    match recipe_key {
        "SCI_SCIA" => "uart_sci".to_string(),
        "SPIA" => "spi".to_string(),
        "SPIB" => "spib".to_string(),
        "I2CA" => "i2c".to_string(),
        "ADC_ADCA" => "adc".to_string(),
        "EPWM1" => "epwm".to_string(),
        "ECAP1" => "ecap".to_string(),
        "EQEP1" => "eqep".to_string(),
        "CPUTIMER0" => "cputimer".to_string(),
        "MCAN0" => "mcan".to_string(),
        "LINA" => "lin".to_string(),
        "FSI" => "fsi".to_string(),
        "USB0" => "usb".to_string(),
        "DMA0" => "dma".to_string(),
        "LED_BOARD" => "led".to_string(),
        _ => recipe_key.to_lowercase(),
    }
}

// Apply recipe to syscfg content
fn apply_recipe_to_syscfg(
    content: &str,
    recipe: &PeripheralRecipe,
    instance_name: Option<&str>,
    peripheral_type: &str,
    baud_rate: Option<u64>,
    rx_pin: Option<&str>,
    tx_pin: Option<&str>,
) -> Result<String, String> {
    let mut modified = content.to_string();
    
    // Check if module already exists
    let module_check = format!("const {} = scripting.addModule", recipe.module_var);
    if content.contains(&module_check) {
        return Err(format!("{} module already exists in this SysConfig file", recipe.module_var));
    }
    
    // Use provided instance name or recipe default
    let inst_name = instance_name.unwrap_or(&recipe.instance_name);
    
    // Build imports section
    let mut imports_section = String::new();
    for import_line in &recipe.imports {
        imports_section.push_str(import_line);
        imports_section.push('\n');
    }
    
    // Build config section with instance name replacement
    let mut config_section = String::new();
    for config_line in &recipe.config {
        // Replace instance name placeholders
        let mut line = config_line.clone();
        
        // Replace $name value if it's in the config
        let name_pattern = format!("{}.$name = \"", recipe.instance_var);
        if line.contains(&name_pattern) {
            // Replace the instance name value
            if let Some(name_start) = line.find(&name_pattern) {
                let pattern_len = name_pattern.len();
                if let Some(name_end_offset) = line[name_start + pattern_len..].find("\";") {
                    let name_end = name_start + pattern_len + name_end_offset;
                    let before = &line[..name_start + pattern_len];
                    let after = &line[name_end..];
                    line = format!("{}{}{}", before, inst_name, after);
                }
            }
        }
        
        // For UART/SCI, handle baud rate and pins
        if peripheral_type == "uart_sci" || peripheral_type == "sci" {
            // Replace baud rate if specified
            if let Some(baud) = baud_rate {
                if line.contains("baudRates") {
                    if let Some(baud_start) = line.find("baudRates") {
                        if let Some(baud_end) = line[baud_start..].find(';') {
                            let before = &line[..baud_start];
                            let after = &line[baud_start + baud_end..];
                            line = format!("{}baudRates = {};{}", before, baud, after);
                        }
                    }
                }
            }
            
            // Replace RX/TX pins if specified
            if let (Some(rx), Some(tx)) = (rx_pin, tx_pin) {
                if line.contains("sci_rxPin") {
                    if let Some(pin_start) = line.find("sci_rxPin.$assign = \"") {
                        if let Some(pin_end) = line[pin_start..].find("\";") {
                            let before = &line[..pin_start + "sci_rxPin.$assign = \"".len()];
                            let after = &line[pin_start + "sci_rxPin.$assign = \"".len() + pin_end..];
                            line = format!("{}{}\";{}", before, rx, after);
                        }
                    }
                }
                if line.contains("sci_txPin") {
                    if let Some(pin_start) = line.find("sci_txPin.$assign = \"") {
                        if let Some(pin_end) = line[pin_start..].find("\";") {
                            let before = &line[..pin_start + "sci_txPin.$assign = \"".len()];
                            let after = &line[pin_start + "sci_txPin.$assign = \"".len() + pin_end..];
                            line = format!("{}{}\";{}", before, tx, after);
                        }
                    }
                }
            }
        }
        
        config_section.push_str(&line);
        config_section.push('\n');
    }
    
    // Combine imports and config
    let full_config = format!("\n{}\n{}", imports_section.trim(), config_section.trim());
    
    // Find insertion point
    let insert_pos = find_insertion_point(&modified);
    
    // Insert the configuration
    modified.insert_str(insert_pos, &full_config);
    
    Ok(modified)
}
