use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::Mutex as AMutex;
use tokio::process::Command;
use tokio::io::{AsyncReadExt, BufReader};
use tokio::time::Duration;
use std::path::PathBuf;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::tools::tools_description::{Tool, ToolDesc, ToolParam, ToolSource, ToolSourceType};
use crate::call_validation::{ChatMessage, ChatContent, ContextEnum};

use super::config::C2000Config;

pub struct ToolC2000UartCapture {
    pub config_path: String,
}

#[async_trait]
impl Tool for ToolC2000UartCapture {
    fn as_any(&self) -> &dyn std::any::Any { self }

    fn tool_description(&self) -> ToolDesc {
        ToolDesc {
            name: "c2000_uart_capture".to_string(),
            display_name: "C2000 UART Capture".to_string(),
            source: ToolSource {
                source_type: ToolSourceType::Builtin,
                config_path: self.config_path.clone(),
            },
            agentic: true,
            experimental: false,
            description: "Capture UART output from TI C2000 device with intelligent logging, real-time analysis, and automatic error detection. Typically use /dev/ttyACM0 for application UART output. Only specify a different device if the user explicitly requests it or if /dev/ttyACM0 is unavailable.".to_string(),
            parameters: vec![
                ToolParam {
                    name: "device".to_string(),
                    param_type: "string".to_string(),
                    description: "UART device path. Default: /dev/ttyACM0 (use this unless user specifies otherwise). Note: /dev/ttyACM1 is typically for debug output, not application UART.".to_string(),
                },
                ToolParam {
                    name: "baud_rate".to_string(),
                    param_type: "integer".to_string(),
                    description: "Baud rate (default: 115200)".to_string(),
                },
                ToolParam {
                    name: "parity".to_string(),
                    param_type: "string".to_string(),
                    description: "Parity setting: odd, even, none (default: odd)".to_string(),
                },
                ToolParam {
                    name: "duration".to_string(),
                    param_type: "integer".to_string(),
                    description: "Capture duration in seconds (optional, for timed capture)".to_string(),
                },
                ToolParam {
                    name: "output_file".to_string(),
                    param_type: "string".to_string(),
                    description: "Log file path (optional, auto-generated if not provided)".to_string(),
                },
                ToolParam {
                    name: "analyze".to_string(),
                    param_type: "boolean".to_string(),
                    description: "Whether to analyze captured output (default: true)".to_string(),
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
        let device = match args.get("device") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => return Err(format!("argument `device` is not a string: {:?}", v)),
            None => {
                let config = C2000Config::load_from_api("http://localhost:8002/v1/c2000-config").await?;
                config.default_uart_device.clone()
            }
        };

        let baud_rate = match args.get("baud_rate") {
            Some(Value::Number(n)) => n.as_u64().unwrap_or(115200) as u32,
            Some(v) => return Err(format!("argument `baud_rate` is not a number: {:?}", v)),
            None => {
                let config = C2000Config::load_from_api("http://localhost:8002/v1/c2000-config").await?;
                config.default_uart_baud
            }
        };

        let parity = match args.get("parity") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => return Err(format!("argument `parity` is not a string: {:?}", v)),
            None => {
                let config = C2000Config::load_from_api("http://localhost:8002/v1/c2000-config").await?;
                config.default_uart_parity.clone()
            }
        };

        let duration = match args.get("duration") {
            Some(Value::Number(n)) => Some(n.as_u64().unwrap_or(10) as u64),
            Some(v) => return Err(format!("argument `duration` is not a number: {:?}", v)),
            None => None
        };

        let output_file = match args.get("output_file") {
            Some(Value::String(s)) => Some(s.clone()),
            Some(v) => return Err(format!("argument `output_file` is not a string: {:?}", v)),
            None => None
        };

        let analyze = match args.get("analyze") {
            Some(Value::Bool(b)) => *b,
            Some(v) => return Err(format!("argument `analyze` is not a boolean: {:?}", v)),
            None => true
        };

        // Validate parameters
        let valid_parity = ["odd", "even", "none"];
        if !valid_parity.contains(&parity.as_str()) {
            return Err(format!("Invalid parity '{}'. Valid options: {:?}", parity, valid_parity));
        }

        // Check if device exists
        if !std::path::Path::new(&device).exists() {
            return Err(format!("UART device not found: {}", device));
        }

        // Get configuration for workspace path
        let config = C2000Config::load_from_api("http://localhost:8002/v1/c2000-config").await?;
        
        // Generate output file path in workspace .uart_captures/ directory
        let uart_captures_dir = PathBuf::from(&config.workspace_path).join(".uart_captures");
        std::fs::create_dir_all(&uart_captures_dir).map_err(|e| {
            format!("Failed to create UART captures directory: {}", e)
        })?;
        
        let final_output_file = if let Some(file) = output_file {
            PathBuf::from(file)
        } else {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            uart_captures_dir.join(format!("uart_capture_{}.txt", timestamp))
        };

        let mut messages = Vec::new();
        let mut context_files = Vec::new();

        // Determine capture duration (default to 30 seconds if not specified)
        let capture_duration = duration.unwrap_or(30);
        
        // Get subchat_tx for streaming to GUI
        let subchat_tx = ccx.lock().await.subchat_tx.clone();

        // Use stty + cat for non-interactive UART capture
        // This is more reliable than minicom for programmatic use
        // We read from stdout incrementally and stream to GUI in real-time
        // stty configures the serial port, cat reads from it
        // timeout limits the capture duration
        
        // Configure parity settings
        let parity_flags = match parity.as_str() {
            "odd" => "parenb parodd",      // Odd parity
            "even" => "parenb -parodd",    // Even parity
            "none" | _ => "-parenb",       // No parity (default)
        };
        
        // Build command: configure serial port with stty, then read with cat
        // Output goes to stdout (piped) instead of file
        let cmd_str = format!(
            "timeout {}s sh -c 'stty -F {} {} cs8 -cstopb {} raw -echo -echoe -echok && cat {}'",
            capture_duration, device, baud_rate, parity_flags, device
        );
        
        let mut capture_cmd = Command::new("sh");
        capture_cmd
            .arg("-c")
            .arg(&cmd_str)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped()) // Pipe stdout for incremental reading
            .stderr(std::process::Stdio::piped()); // Capture stderr for errors

        // Spawn the process
        let mut child = capture_cmd.spawn()
            .map_err(|e| format!("Failed to spawn UART capture process: {}", e))?;

        // Get stdout and stderr handles
        let mut stdout = child.stdout.take()
            .ok_or("Failed to get stdout handle")?;
        let stderr_handle = child.stderr.take()
            .ok_or("Failed to get stderr handle")?;

        // Read stderr in background to capture errors
        let stderr_task = tokio::spawn(async move {
            let mut stderr_reader = BufReader::new(stderr_handle);
            let mut stderr_buf = String::new();
            let mut buf = [0u8; 1024];
            loop {
                match stderr_reader.read(&mut buf).await {
                    Ok(0) => break,
                    Ok(n) => {
                        stderr_buf.push_str(&String::from_utf8_lossy(&buf[..n]));
                    }
                    Err(_) => break,
                }
            }
            stderr_buf
        });

        // Read stdout incrementally and stream to GUI
        let mut accumulated_output = String::new();
        let mut buffer = [0u8; 4096];
        let start_time = std::time::Instant::now();
        let duration_limit = Duration::from_secs(capture_duration);
        let mut has_data = false;

        messages.push(format!(
            "📡 Starting UART capture from {} at {} baud ({}) for {} seconds...",
            device, baud_rate, parity, capture_duration
        ));

        // Stream initial message
        let initial_msg = serde_json::json!({
            "tool_call_id": tool_call_id,
            "add_message": {
                "role": "tool",
                "content": format!("📡 Starting UART capture from {} at {} baud...\n", device, baud_rate),
                "tool_call_id": tool_call_id.clone()
            }
        });
        let _ = subchat_tx.lock().await.send(initial_msg);

        loop {
            // Check timeout
            if start_time.elapsed() >= duration_limit {
                break;
            }

            // Check if process has exited
            if let Ok(Some(status)) = child.try_wait() {
                // Process exited, read any remaining data
                // Read remaining data before breaking
                loop {
                    match stdout.read(&mut buffer).await {
                        Ok(0) => break,
                        Ok(n) => {
                            let chunk = String::from_utf8_lossy(&buffer[..n]);
                            accumulated_output.push_str(&chunk);
                            has_data = true;
                            
                            // Stream chunk to GUI
                            let stream_msg = serde_json::json!({
                                "tool_call_id": tool_call_id,
                                "add_message": {
                                    "role": "tool",
                                    "content": chunk.to_string(),
                                    "tool_call_id": tool_call_id.clone()
                                }
                            });
                            let _ = subchat_tx.lock().await.send(stream_msg);
                        }
                        Err(_) => break,
                    }
                }
                // Check exit status after reading all data
                if !status.success() && status.code() != Some(124) {
                    // Exit code 124 is timeout (expected), other codes are errors
                    // We'll check stderr after the loop
                }
                break;
            }

            // Try to read data with timeout
            tokio::select! {
                result = stdout.read(&mut buffer) => {
                    match result {
                        Ok(0) => {
                            // EOF - process may have finished
                            tokio::time::sleep(Duration::from_millis(100)).await;
                            continue;
                        }
                        Ok(n) => {
                            let chunk = String::from_utf8_lossy(&buffer[..n]);
                            accumulated_output.push_str(&chunk);
                            has_data = true;
                            
                            // Stream chunk to GUI in real-time
                            let stream_msg = serde_json::json!({
                                "tool_call_id": tool_call_id,
                                "add_message": {
                                    "role": "tool",
                                    "content": chunk.to_string(),
                                    "tool_call_id": tool_call_id.clone()
                                }
                            });
                            let _ = subchat_tx.lock().await.send(stream_msg);
                        }
                        Err(e) => {
                            // Read error - might be normal if process finished
                            tracing::warn!("Error reading from stdout: {}", e);
                            break;
                        }
                    }
                }
                _ = tokio::time::sleep(Duration::from_millis(50)) => {
                    // Timeout - continue loop to check process status
                    continue;
                }
            }
        }

        // Wait for process to finish and get stderr
        let exit_status = child.wait().await;
        let stderr_output = stderr_task.await.unwrap_or_default();
        
        // Check for errors if process didn't exit successfully
        if let Ok(status) = exit_status {
            if !status.success() && status.code() != Some(124) {
                // Exit code 124 is timeout (expected), other codes are errors
                if !stderr_output.is_empty() {
                    return Err(format!(
                        "UART capture process failed (exit code: {:?})\nStderr: {}\n💡 Check device connection and permissions.",
                        status.code(),
                        stderr_output
                    ));
                }
            }
        }

        // Save accumulated output to file
        if !accumulated_output.is_empty() {
            tokio::fs::write(&final_output_file, &accumulated_output).await
                .map_err(|e| format!("Failed to write UART capture to file: {}", e))?;
        }

        // Build result messages
        if has_data {
            messages.push(format!(
                "✅ UART capture completed ({} seconds)\n📡 Device: {} at {} baud ({})\n📄 Log saved to: {}",
                capture_duration, device, baud_rate, parity, final_output_file.display()
            ));
        } else {
            messages.push(format!(
                "⚠️ UART capture completed but no data was captured\n📡 Device: {} at {} baud ({})\n💡 Possible reasons:\n   - Device is not sending data on this port\n   - Wrong device selected (try /dev/ttyACM0 for application UART)\n   - Device not connected or powered\n   - Wrong baud rate or parity settings",
                device, baud_rate, parity
            ));
        }

        if !stderr_output.is_empty() {
            messages.push(format!("⚠️ Stderr output: {}", stderr_output));
        }

        // Analyze captured output if requested
        if analyze && has_data && !accumulated_output.is_empty() {
            // Analyze from accumulated output directly
            let analysis = self.analyze_uart_output_content(&accumulated_output);
            messages.push(format!("📊 Analysis:\n{}", analysis));
        } else if analyze && final_output_file.exists() {
            // Fallback to file-based analysis
            let analysis = self.analyze_uart_output(&final_output_file.to_string_lossy().to_string()).await?;
            messages.push(format!("📊 Analysis:\n{}", analysis));
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

impl ToolC2000UartCapture {
    async fn analyze_uart_output(&self, file_path: &str) -> Result<String, String> {
        let content = tokio::fs::read_to_string(file_path)
            .await
            .map_err(|e| format!("Failed to read UART output file: {}", e))?;
        Ok(self.analyze_uart_output_content(&content))
    }

    fn analyze_uart_output_content(&self, content: &str) -> String {
        let mut analysis = Vec::new();
        
        // Basic analysis
        let lines: Vec<&str> = content.lines().collect();
        analysis.push(format!("📈 Total lines captured: {}", lines.len()));
        
        // Look for common patterns
        let mut spi_detected = false;
        let mut sci_detected = false;
        let mut error_detected = false;
        let mut boot_rom_detected = false;

        for line in lines.iter() {
            let line_lower = line.to_lowercase();
            if line_lower.contains("spi") || line_lower.contains("loopback") {
                spi_detected = true;
            }
            if line_lower.contains("sci") || line_lower.contains("echo") {
                sci_detected = true;
            }
            if line_lower.contains("error") || line_lower.contains("fail") {
                error_detected = true;
            }
            if line_lower.contains("boot") || line_lower.contains("rom") {
                boot_rom_detected = true;
            }
        }

        if spi_detected {
            analysis.push("🔌 SPI communication detected".to_string());
        }
        if sci_detected {
            analysis.push("📡 SCI/UART communication detected".to_string());
        }
        if error_detected {
            analysis.push("❌ Errors detected in output".to_string());
        }
        if boot_rom_detected {
            analysis.push("🚀 Boot ROM messages detected (normal)".to_string());
        }

        // Check for empty output
        if content.trim().is_empty() {
            analysis.push("⚠️ No output captured - check device connection and configuration".to_string());
        }

        // Detect if output looks like random debug data (binary/control characters)
        let non_printable_ratio = content.chars()
            .filter(|c| !c.is_ascii() || (!c.is_ascii_graphic() && !c.is_whitespace()))
            .count() as f32 / content.len().max(1) as f32;
        
        if non_printable_ratio > 0.3 && lines.len() > 0 {
            analysis.push("⚠️ Output contains many non-printable characters - may be debug/binary data, not application UART output".to_string());
            analysis.push("💡 Try /dev/ttyACM0 for application UART output instead".to_string());
        }

        analysis.join("\n")
    }
}


