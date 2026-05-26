use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::Mutex as AMutex;
use tokio::process::Command;
use std::process::Stdio;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::tools::tools_description::{Tool, ToolDesc, ToolParam, ToolSource, ToolSourceType};
use crate::call_validation::{ChatMessage, ChatContent, ContextEnum};

use super::config::ESP32Config;
use super::output_protocol::ToolOutput;
use super::idf_command::{IdfCommand, infer_project_path, list_serial_ports as get_serial_ports};
use super::global_state::{
    get_config, get_state, get_state_mut, generate_suggested_actions, SuggestionContext,
    board_definition_url, resolve_device_port, record_device_port, record_device_port_in_use,
};
use super::device_port_store::PortResolutionPolicy;
use super::board_definition::BoardDefinition;

pub struct ESP32Device {
    pub config_path: String,
}

#[async_trait]
impl Tool for ESP32Device {
    fn as_any(&self) -> &dyn std::any::Any { self }

    fn tool_description(&self) -> ToolDesc {
        ToolDesc {
            name: "esp32_device".to_string(),
            display_name: "ESP32 Device".to_string(),
            source: ToolSource {
                source_type: ToolSourceType::Builtin,
                config_path: self.config_path.clone(),
            },
            agentic: true,
            experimental: false,
            description: "Interact with ESP32 devices. Operations: detect (find connected devices, reads chip info, sets active port in session, and performs fast heuristic compatibility check if board_id is set), verify (validate device - uses board_id from --board-definition CLI if set, or accepts override), flash (program firmware), monitor (capture serial output), erase (erase flash), info (get chip information).".to_string(),
            parameters: vec![
                ToolParam {
                    name: "operation".to_string(),
                    param_type: "string".to_string(),
                    description: "Operation: 'detect' (find devices), 'verify' (validate device vs board_id), 'flash' (program firmware), 'monitor' (serial monitor), 'erase' (erase flash), 'info' (chip info)".to_string(),
                },
                ToolParam {
                    name: "port".to_string(),
                    param_type: "string".to_string(),
                    description: "Serial port (e.g., COM96, /dev/ttyUSB0). Omit after detect — session and persisted port are used automatically. Do not pass the yaml default (COM3) if detect found a different port.".to_string(),
                },
                ToolParam {
                    name: "board_id".to_string(),
                    param_type: "string".to_string(),
                    description: "Board ID for verification (e.g., esp32-s3-devkitc-1-n32r8v). NOT NEEDED if --board-definition was set at startup - will use session board_id automatically. Only provide if you want to override the session board_id.".to_string(),
                },
                ToolParam {
                    name: "project_path".to_string(),
                    param_type: "string".to_string(),
                    description: "Path to ESP32 project (for 'flash' and 'monitor' operations)".to_string(),
                },
                ToolParam {
                    name: "baud_rate".to_string(),
                    param_type: "integer".to_string(),
                    description: "Serial baud rate (default: 115200)".to_string(),
                },
                ToolParam {
                    name: "duration".to_string(),
                    param_type: "integer".to_string(),
                    description: "Monitor duration in seconds (for 'monitor' operation, default: 10)".to_string(),
                },
            ],
            parameters_required: vec!["operation".to_string()],
        }
    }

    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>,
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        let operation = args.get("operation")
            .and_then(|v| v.as_str())
            .ok_or("Missing required parameter: operation")?;

        let chat_id = ccx.lock().await.chat_id.clone();
        let invoked_at = std::time::SystemTime::now();
        crate::progressbar::record_tool_start(
            &chat_id,
            tool_call_id,
            "esp32_device",
            operation,
            args.clone(),
        ).await;

        // Use global config (cached, configurable via env var)
        let config = get_config().await?;

        let result = match operation {
            "detect" => self.detect_devices(&config).await,
            "verify" => self.verify_device(&config, args).await,
            "flash" => self.flash_device(&config, args).await,
            "monitor" => self.monitor_device(&config, args).await,
            "erase" => self.erase_flash(&config, args).await,
            "info" => self.get_chip_info(&config, args).await,
            _ => return Err(format!("Unknown operation: {}", operation)),
        };

        match &result {
            Ok(output) => {
                crate::progressbar::record_tool_complete(
                    &chat_id,
                    tool_call_id,
                    "esp32_device",
                    operation,
                    args.clone(),
                    invoked_at,
                    output,
                ).await;
            }
            Err(e) => {
                crate::progressbar::record_tool_error(
                    &chat_id,
                    tool_call_id,
                    "esp32_device",
                    operation,
                    args.clone(),
                    invoked_at,
                    e,
                ).await;
            }
        }

        let output = result?;

        let context_files = vec![ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: ChatContent::SimpleText(output.to_llm_context()),
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
            ..Default::default()
        })];

        Ok((false, context_files))
    }

    fn tool_depends_on(&self) -> Vec<String> {
        vec!["esp32".to_string()]
    }
}

impl ESP32Device {
    fn parse_mb_from_string(s: &str) -> Option<u32> {
        let lower = s.to_lowercase();
        let mb_pos = lower.find("mb")?;
        let bytes = lower.as_bytes();
        let mut i = mb_pos;
        while i > 0 && bytes[i - 1].is_ascii_digit() {
            i -= 1;
        }
        if i == mb_pos {
            return None;
        }
        lower[i..mb_pos].parse::<u32>().ok()
    }

    fn parse_board_id_hints(board_id: &str) -> BoardIdHints {
        let id = board_id.trim().to_lowercase();
        let expected_chip = if id.contains("esp32-s3") || id.contains("esp32s3") {
            Some("esp32s3".to_string())
        } else if id.contains("esp32-s2") || id.contains("esp32s2") {
            Some("esp32s2".to_string())
        } else if id.contains("esp32-c3") || id.contains("esp32c3") {
            Some("esp32c3".to_string())
        } else if id.contains("esp32-c6") || id.contains("esp32c6") {
            Some("esp32c6".to_string())
        } else if id.contains("esp32-c5") || id.contains("esp32c5") {
            Some("esp32c5".to_string())
        } else if id.contains("esp32-c2") || id.contains("esp32c2") {
            Some("esp32c2".to_string())
        } else if id.contains("esp32-p4") || id.contains("esp32p4") {
            Some("esp32p4".to_string())
        } else if id.starts_with("esp32-") || id == "esp32" || id.contains("esp32") {
            Some("esp32".to_string())
        } else {
            None
        };

        // Parse NxxRyy pattern (flash/psram sizes encoded in board id).
        // e.g. "...-n16r8" => flash=16MB, psram=8MB
        let mut flash_mb = None;
        let mut psram_mb = None;
        let bytes = id.as_bytes();
        let mut idx = 0usize;
        while idx < bytes.len() {
            if bytes[idx] == b'n' {
                let mut j = idx + 1;
                while j < bytes.len() && bytes[j].is_ascii_digit() {
                    j += 1;
                }
                if j > idx + 1 {
                    if let Ok(n) = id[idx + 1..j].parse::<u32>() {
                        flash_mb = Some(n);
                    }
                    if j < bytes.len() && bytes[j] == b'r' {
                        let mut k = j + 1;
                        while k < bytes.len() && bytes[k].is_ascii_digit() {
                            k += 1;
                        }
                        if k > j + 1 {
                            if let Ok(r) = id[j + 1..k].parse::<u32>() {
                                psram_mb = Some(r);
                            }
                        }
                    }
                    break;
                }
            }
            idx += 1;
        }

        BoardIdHints {
            expected_chip,
            flash_mb,
            psram_mb,
        }
    }

    fn format_detect_details(
        port: Option<&str>,
        chip: Option<&str>,
        revision: Option<&str>,
        features: Option<&str>,
        mac: Option<&str>,
        flash_size: Option<&str>,
        session_board_id: Option<&str>,
        verified: bool,
        heuristic_verify: Option<&HeuristicVerifyResult>,
    ) -> String {
        let mut lines = Vec::new();
        lines.push("Detected device:".to_string());
        if let Some(port) = port {
            lines.push(format!("- port: {}", port));
        }
        if let Some(chip) = chip {
            lines.push(format!("- chip: {}", chip));
        }
        if let Some(rev) = revision {
            lines.push(format!("- revision: {}", rev));
        }
        if let Some(features) = features {
            if !features.trim().is_empty() {
                lines.push(format!("- features: {}", features));
            }
        }
        if let Some(mac) = mac {
            lines.push(format!("- mac: {}", mac));
        }
        if let Some(flash) = flash_size {
            lines.push(format!("- flash: {}", flash));
        }
        if let Some(bid) = session_board_id {
            lines.push(format!("- board_id in session: {}", bid));
            lines.push(format!("- verified: {}", if verified { "yes" } else { "no" }));
        }
        if let Some(hv) = heuristic_verify {
            lines.push(format!("- heuristic verification: {}", hv.status));
            if !hv.warnings.is_empty() {
                lines.push("  warnings:".to_string());
                for w in &hv.warnings {
                    lines.push(format!("  - {}", w));
                }
            }
        }
        if let Some(port) = port {
            lines.push(format!("Future esp32_device calls may omit port; session active port is {}.", port));
        }
        lines.join("\n")
    }

    async fn detect_devices(&self, config: &ESP32Config) -> Result<ToolOutput, String> {
        // First, list available serial ports (cross-platform)
        let ports = get_serial_ports();
        
        let mut detected = Vec::new();
        
        // Try esptool auto-detection first (no port specified)
        let auto_result = IdfCommand::esptool("chip-id")
            .timeout_secs(10)
            .no_parse_errors()
            .execute(config).await;
        
        if let Ok(result) = auto_result {
            if result.success {
                let chip = self.parse_chip_type(&result.stdout);
                if let Some(port) = self.extract_port_from_output(&result.stdout) {
                    detected.push(serde_json::json!({
                        "port": port,
                        "chip": chip,
                        "auto_detected": true,
                    }));
                }
            }
        }

        // Also try specific ports if auto-detect found nothing
        if detected.is_empty() {
            for port in &ports {
                let port_result = IdfCommand::esptool("chip-id")
                    .args(&["--port", port])
                    .timeout_secs(5)
                    .no_parse_errors()
                    .execute(config).await;
                
                if let Ok(result) = port_result {
                    if result.success {
                        let chip = self.parse_chip_type(&result.stdout);

                        // Avoid duplicates
                        if !detected.iter().any(|d| d.get("port").and_then(|p| p.as_str()) == Some(port.as_str())) {
                            detected.push(serde_json::json!({
                                "port": port,
                                "chip": chip,
                                "auto_detected": false,
                            }));
                        }
                    }
                }
            }
        }

        let devices_found = !detected.is_empty();
        let state = get_state().await;
        let board_id_set = state.session.board_id.is_some();
        drop(state);
        let suggested_actions = generate_suggested_actions(
            "detect",
            true,
            &SuggestionContext::new()
                .with_devices(devices_found)
                .with_board_id(board_id_set),
        );

        if detected.is_empty() {
            Ok(ToolOutput {
                status: super::output_protocol::ToolStatus::PartialSuccess,
                action_taken: "detect".to_string(),
                data: serde_json::json!({ 
                    "devices": [],
                    "available_ports": ports,
                }),
                summary: format!("No ESP32 devices detected. Available ports: {}", ports.join(", ")),
                details: Some("Ensure device is connected and in bootloader mode (hold BOOT button while connecting)".to_string()),
                state_delta: super::session_state::StateDelta::none(),
                suggested_actions,
                error: None,
            })
        } else {
            // Agentic: Get full device info and set active_device in session state.
            let mut detect_details: Option<String> = None;
            let mut heuristic_verification: Option<HeuristicVerifyResult> = None;
            let mut active_port_for_summary: Option<String> = None;
            
            // Get first detected device: try full chip info; on failure still set active_device and minimal details
            if let Some(first_device) = detected.first() {
                if let Some(port) = first_device.get("port").and_then(|p| p.as_str()) {
                    let chip_short = first_device.get("chip").and_then(|c| c.as_str()).unwrap_or("esp32");
                    let info_args = serde_json::json!({ "port": port });
                    let info_result = self.get_chip_info(config, &serde_json::from_value(info_args).unwrap_or_default()).await;

                    if let Ok(info_output) = info_result {
                        let info_data = info_output.data;
                        let chip = info_data.get("chip_type").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
                        let mac = info_data.get("mac_address").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
                        let flash_size = info_data.get("flash_size").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
                        let revision = info_data.get("revision").and_then(|v| v.as_str()).map(|s| s.to_string());
                        let features = info_data.get("features").and_then(|v| v.as_str()).map(|s| s.to_string());

                        record_device_port(port, &chip, &mac, &flash_size, "detect").await;
                        let (board_id, board_verified) = {
                            let state = get_state().await;
                            (state.session.board_id.clone(), state.session.board_verified)
                        };

                        // Offline heuristic verification from board_id name
                        if let Some(board_id) = board_id.as_deref() {
                            let expected = Self::parse_board_id_hints(board_id);
                            let actual_chip_norm = Self::normalize_chip_type(&chip);
                            let mut warnings = Vec::new();

                            if let Some(exp_chip) = &expected.expected_chip {
                                if exp_chip != &actual_chip_norm {
                                    warnings.push(format!("Chip mismatch: expected {}, detected {} ({})", exp_chip, actual_chip_norm, chip));
                                }
                            }

                            let actual_flash_mb = info_data
                                .get("flash_size")
                                .and_then(|v| v.as_str())
                                .and_then(Self::parse_mb_from_string);
                            if let (Some(exp_flash), Some(act_flash)) = (expected.flash_mb, actual_flash_mb) {
                                if exp_flash != act_flash {
                                    warnings.push(format!("Flash size mismatch: expected {}MB, detected {}MB", exp_flash, act_flash));
                                }
                            }

                            let actual_psram_mb = features
                                .as_deref()
                                .and_then(Self::parse_mb_from_string);
                            if let (Some(exp_psram), Some(act_psram)) = (expected.psram_mb, actual_psram_mb) {
                                if exp_psram != act_psram {
                                    warnings.push(format!("PSRAM size mismatch: expected {}MB, detected {}MB", exp_psram, act_psram));
                                }
                            }

                            let status = if expected.expected_chip.is_none() && expected.flash_mb.is_none() && expected.psram_mb.is_none() {
                                "unknown (board_id hints not parseable)".to_string()
                            } else if warnings.is_empty() {
                                "likely compatible".to_string()
                            } else {
                                "compatible with warnings".to_string()
                            };

                            heuristic_verification = Some(HeuristicVerifyResult { status: status.clone(), warnings: warnings.clone() });

                            // When heuristic says likely compatible, mark session as verified so UI shows verified: yes
                            if status == "likely compatible" {
                                let mut state = get_state_mut().await;
                                state.session.mark_board_verified();
                                drop(state);
                            }
                        }

                        let verified_for_display = board_verified
                            || heuristic_verification.as_ref().map(|h| h.status == "likely compatible").unwrap_or(false);
                        detect_details = Some(Self::format_detect_details(
                            Some(port),
                            Some(&chip),
                            revision.as_deref(),
                            features.as_deref(),
                            Some(&mac),
                            Some(&flash_size),
                            board_id.as_deref(),
                            verified_for_display,
                            heuristic_verification.as_ref(),
                        ));
                        active_port_for_summary = Some(port.to_string());
                    } else {
                        // get_chip_info failed on the initial port; try other candidate serial ports (e.g. /dev/ttyACM*).
                        heuristic_verification = None;
                        let mut fallback_handled = false;

                        for cand in &ports {
                            if cand == port {
                                continue;
                            }

                            let info_args = serde_json::json!({ "port": cand });
                            let info_res = self.get_chip_info(config, &serde_json::from_value(info_args).unwrap_or_default()).await;

                            if let Ok(info_output) = info_res {
                                let info_data = info_output.data;
                                let chip = info_data.get("chip_type").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
                                let mac = info_data.get("mac_address").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
                                let flash_size = info_data.get("flash_size").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
                                let revision = info_data.get("revision").and_then(|v| v.as_str()).map(|s| s.to_string());
                                let features = info_data.get("features").and_then(|v| v.as_str()).map(|s| s.to_string());

                                record_device_port(cand, &chip, &mac, &flash_size, "detect").await;
                                let (board_id, board_verified) = {
                                    let state = get_state().await;
                                    (state.session.board_id.clone(), state.session.board_verified)
                                };

                                if let Some(board_id) = board_id.as_deref() {
                                    let expected = Self::parse_board_id_hints(board_id);
                                    let actual_chip_norm = Self::normalize_chip_type(&chip);
                                    let mut warnings = Vec::new();

                                    if let Some(exp_chip) = &expected.expected_chip {
                                        if exp_chip != &actual_chip_norm {
                                            warnings.push(format!("Chip mismatch: expected {}, detected {} ({})", exp_chip, actual_chip_norm, chip));
                                        }
                                    }

                                    let actual_flash_mb = info_data
                                        .get("flash_size")
                                        .and_then(|v| v.as_str())
                                        .and_then(Self::parse_mb_from_string);
                                    if let (Some(exp_flash), Some(act_flash)) = (expected.flash_mb, actual_flash_mb) {
                                        if exp_flash != act_flash {
                                            warnings.push(format!("Flash size mismatch: expected {}MB, detected {}MB", exp_flash, act_flash));
                                        }
                                    }

                                    let actual_psram_mb = features
                                        .as_deref()
                                        .and_then(Self::parse_mb_from_string);
                                    if let (Some(exp_psram), Some(act_psram)) = (expected.psram_mb, actual_psram_mb) {
                                        if exp_psram != act_psram {
                                            warnings.push(format!("PSRAM size mismatch: expected {}MB, detected {}MB", exp_psram, act_psram));
                                        }
                                    }

                                    let status = if expected.expected_chip.is_none() && expected.flash_mb.is_none() && expected.psram_mb.is_none() {
                                        "unknown (board_id hints not parseable)".to_string()
                                    } else if warnings.is_empty() {
                                        "likely compatible".to_string()
                                    } else {
                                        "compatible with warnings".to_string()
                                    };

                                    heuristic_verification = Some(HeuristicVerifyResult { status: status.clone(), warnings: warnings.clone() });

                                    if status == "likely compatible" {
                                        let mut state = get_state_mut().await;
                                        state.session.mark_board_verified();
                                        drop(state);
                                    }
                                }

                                let verified_for_display = board_verified
                                    || heuristic_verification.as_ref().map(|h| h.status == "likely compatible").unwrap_or(false);
                                detect_details = Some(Self::format_detect_details(
                                    Some(cand),
                                    Some(&chip),
                                    revision.as_deref(),
                                    features.as_deref(),
                                    Some(&mac),
                                    Some(&flash_size),
                                    board_id.as_deref(),
                                    verified_for_display,
                                    heuristic_verification.as_ref(),
                                ));
                                active_port_for_summary = Some(cand.to_string());
                                fallback_handled = true;
                                break;
                            }
                        }

                        if !fallback_handled {
                            // As a last resort, remember the original auto-detected port with minimal info,
                            // so the user (or LLM) can still explicitly try info/flash on it.
                            record_device_port(
                                port,
                                chip_short,
                                "unknown",
                                "unknown",
                                "detect",
                            ).await;
                            let minimal = format!(
                                "Detected device:\n- port: {}\n- chip: {} (from esptool)\nFull chip info (MAC, flash) could not be read. Use esp32_device(operation=\"info\", port=\"{}\") to retry.",
                                port, chip_short, port
                            );
                            detect_details = Some(minimal);
                            active_port_for_summary = Some(port.to_string());
                        }
                    }
                }
            }
            
            let mut data = serde_json::json!({ 
                "devices": detected,
                "available_ports": ports,
            });
            if let Some(ref hv) = heuristic_verification {
                data["heuristic_verification"] = serde_json::json!({
                    "mode": "heuristic_from_board_id",
                    "status": hv.status,
                    "warnings": hv.warnings,
                });
            }
            
            // Add active_device and session board_id info if set
            let state = get_state().await;
            if let Some(active_dev) = &state.session.active_device {
                data["active_device"] = serde_json::json!({
                    "port": active_dev.port,
                    "chip": active_dev.chip,
                    "verified": state.session.board_verified,
                });
            }
            // Include session board_id so LLM knows it's available
            if let Some(ref bid) = state.session.board_id {
                data["session_board_id"] = serde_json::json!(bid);
                data["board_id_available"] = serde_json::json!(true);
            }
            drop(state);

            // Do not suggest verify after detect; verification is already included in detect output.
            let suggested_actions: Vec<_> = vec![];
            
            Ok(ToolOutput {
                status: super::output_protocol::ToolStatus::Success,
                action_taken: "detect".to_string(),
                data,
                summary: match active_port_for_summary.as_deref() {
                    Some(port) => format!("Detected {} ESP32 device(s) on {}", detected.len(), port),
                    None => format!("Detected {} ESP32 device(s)", detected.len()),
                },
                details: detect_details,
                state_delta: super::session_state::StateDelta::none(),
                suggested_actions,
                error: None,
            })
        }
    }

    fn parse_chip_type(&self, output: &str) -> &'static str {
        // Order matters: check more specific / longer names first
        if output.contains("ESP32-P4") {
            "esp32p4"
        } else if output.contains("ESP32-S31") {
            "esp32s31"
        } else if output.contains("ESP32-S3") {
            "esp32s3"
        } else if output.contains("ESP32-S2") {
            "esp32s2"
        } else if output.contains("ESP32-C61") {
            "esp32c61"
        } else if output.contains("ESP32-C6") {
            "esp32c6"
        } else if output.contains("ESP32-C5") {
            "esp32c5"
        } else if output.contains("ESP32-C3") {
            "esp32c3"
        } else if output.contains("ESP32-C2") || output.contains("ESP8684") {
            "esp32c2"
        } else if output.contains("ESP32-H21") {
            "esp32h21"
        } else if output.contains("ESP32-H4") {
            "esp32h4"
        } else if output.contains("ESP32-H2") {
            "esp32h2"
        } else {
            // Fallback to classic ESP32
            "esp32"
        }
    }

    fn extract_port_from_output(&self, output: &str) -> Option<String> {
        // esptool prints "Serial port /dev/ttyUSB0" or similar
        for line in output.lines() {
            if line.contains("Serial port") {
                if let Some(port) = line.split_whitespace().last() {
                    return Some(port.to_string());
                }
            }
        }
        None
    }

    async fn flash_device(&self, config: &ESP32Config, args: &HashMap<String, Value>) -> Result<ToolOutput, String> {
        let port = resolve_device_port(config, args, PortResolutionPolicy::PreferKnown).await?;

        // Infer project path
        let explicit_path = args.get("project_path").and_then(|v| v.as_str());
        let project_path = infer_project_path(explicit_path, None)
            .ok_or("No valid ESP-IDF project found")?;

        let baud_rate = args.get("baud_rate")
            .and_then(|v| v.as_u64())
            .or_else(|| config.default_flash_baud_rate.map(|v| v as u64))
            .unwrap_or(config.default_baud_rate as u64);

        // Check if build exists
        let build_dir = project_path.join("build");
        if !build_dir.exists() {
            return Err("Project not built. Run esp32_build with 'build' operation first.".to_string());
        }

        // Use IdfCommand for flash with proper environment setup
        let result = IdfCommand::new("flash")
            .args(&["-p", &port, "-b", &baud_rate.to_string()])
            .project_path(&project_path)
            .timeout_secs(300)  // 5 minute timeout
            .execute(config).await?;

        if result.success {
            record_device_port_in_use(&port, "flash").await;

            // Generate suggested actions for successful flash
            let suggested_actions = generate_suggested_actions(
                "flash",
                true,
                &SuggestionContext::new(),
            );

            Ok(ToolOutput {
                status: super::output_protocol::ToolStatus::Success,
                action_taken: "flash".to_string(),
                data: serde_json::json!({
                    "port": port,
                    "baud_rate": baud_rate,
                    "flash_time_seconds": result.duration.as_secs_f64(),
                    "project_path": project_path.to_string_lossy(),
                }),
                summary: format!("Flashed to {} in {:.1}s", port, result.duration.as_secs_f64()),
                details: None,
                state_delta: super::session_state::StateDelta::none(),
                suggested_actions,
                error: None,
            })
        } else {
            // Generate suggested actions for failed flash
            let suggested_actions = generate_suggested_actions(
                "flash",
                false,
                &SuggestionContext::new(),
            );

            let error = result.errors.first().cloned();

            Ok(ToolOutput {
                status: super::output_protocol::ToolStatus::Failed,
                action_taken: "flash".to_string(),
                data: serde_json::json!({
                    "port": port,
                    "error_count": result.errors.len(),
                }),
                summary: format!("Flash failed: {}", result.summary),
                details: Some(result.combined_output()),
                state_delta: super::session_state::StateDelta::none(),
                suggested_actions,
                error,
            })
        }
    }

    async fn monitor_device(&self, config: &ESP32Config, args: &HashMap<String, Value>) -> Result<ToolOutput, String> {
        let port = resolve_device_port(config, args, PortResolutionPolicy::RequireKnown).await?;

        let project_path = args.get("project_path")
            .and_then(|v| v.as_str())
            .map(|s| std::path::PathBuf::from(s))
            .unwrap_or_else(|| std::path::PathBuf::from("."));

        let duration = args.get("duration")
            .and_then(|v| v.as_u64())
            .unwrap_or(10);

        let baud_rate = args.get("baud_rate")
            .and_then(|v| v.as_u64())
            .or_else(|| config.default_monitor_baud_rate.map(|v| v as u64))
            .unwrap_or(config.default_baud_rate as u64);

        // Use pyserial with DTR/RTS reset logic (same reset workflow as before).
        // Capture uses raw byte reads for the full duration window so one-shot boot
        // logs are not discarded. Stale bytes are cleared only before reset, never after.
        // signal.SIGALRM / signal.alarm are Unix-only and not available on Windows.
        // The outer tokio::time::timeout already enforces the duration limit, so the
        // Python-side timeout loop (time.time() check) is sufficient on all platforms.
        let python_script = format!(
            r#"
import serial
import sys
import time

# Force UTF-8 encoding on stdout/stderr to avoid Windows charmap codec errors
# when serial data contains non-ASCII bytes (e.g. ESP32 boot garbage)
if hasattr(sys.stdout, 'reconfigure'):
    sys.stdout.reconfigure(encoding='utf-8', errors='replace')
if hasattr(sys.stderr, 'reconfigure'):
    sys.stderr.reconfigure(encoding='utf-8', errors='replace')

def read_available(ser):
    waiting = ser.in_waiting
    if waiting > 0:
        return ser.read(waiting)
    return ser.read(1)

try:
    # Open serial port
    ser = serial.Serial('{}', {}, timeout=0.2)

    # Drop stale bytes from port-open glitches before the intentional reset.
    ser.reset_input_buffer()

    # ESP32 reset sequence via DTR/RTS (triggers device reset)
    ser.setDTR(True)
    ser.setRTS(True)
    time.sleep(0.1)
    ser.setDTR(False)
    ser.setRTS(False)

    # Read for the full capture window (includes boot logs). Do not clear the RX
    # buffer after reset — that was dropping one-shot startup output.
    start_time = time.time()
    while time.time() - start_time < {}:
        chunk = read_available(ser)
        if chunk:
            sys.stdout.write(chunk.decode('utf-8', errors='replace'))
            sys.stdout.flush()
    ser.close()
except Exception as e:
    sys.stderr.write(f"Error: {{e}}\n")
    sys.exit(1)
"#,
            port,
            baud_rate,
            duration
        );

        // Windows ships `python.exe`, not `python3.exe`
        let mut cmd = Command::new(if cfg!(windows) { "python" } else { "python3" });
        cmd.arg("-c");
        cmd.arg(&python_script);
        cmd.current_dir(&project_path);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        // Force UTF-8 encoding for the subprocess (belt-and-suspenders with the in-script reconfigure)
        cmd.env("PYTHONIOENCODING", "utf-8");

        let output = tokio::time::timeout(
            std::time::Duration::from_secs(duration + 8),
            cmd.output()
        ).await
        .map_err(|_| format!("Monitor operation timed out after {}s", duration + 8))?
        .map_err(|e| format!("Failed to monitor device: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Treat any non-whitespace serial data as captured output. Line-based filtering
        // dropped valid boot logs that did not end with newlines.
        let filtered_output = stdout.trim().to_string();
        let has_serial_output = !filtered_output.is_empty()
            && !filtered_output.starts_with("Error:");
        
        let details = if has_serial_output {
            Some(format!("STDOUT\n```\n{}\n```", filtered_output))
        } else if !stderr.is_empty() {
            Some(format!("STDERR\n```\n{}\n```", stderr))
        } else {
            Some("No output captured. Check device connection and baud rate.".to_string())
        };
        
        // Determine status based on whether we got output
        let status = if has_serial_output {
            super::output_protocol::ToolStatus::Success
        } else if output.status.success() {
            super::output_protocol::ToolStatus::PartialSuccess
        } else {
            super::output_protocol::ToolStatus::Failed
        };

        if matches!(status, super::output_protocol::ToolStatus::Success | super::output_protocol::ToolStatus::PartialSuccess) {
            record_device_port_in_use(&port, "monitor").await;
        }
        
        Ok(ToolOutput {
            status,
            action_taken: format!("monitor_device({})", port),
            data: serde_json::json!({
                "port": port,
                "duration": duration,
                "baud_rate": baud_rate,
                "output_length": filtered_output.len(),
                "lines": filtered_output.lines().count(),
            }),
            summary: format!("Captured {}s of output from {} ({} lines)", duration, port, filtered_output.lines().count()),
            details,
            state_delta: super::session_state::StateDelta::none(),
            suggested_actions: vec![],
            error: None,  // Error details are included in details field
        })
    }

    async fn erase_flash(&self, config: &ESP32Config, args: &HashMap<String, Value>) -> Result<ToolOutput, String> {
        let port = resolve_device_port(config, args, PortResolutionPolicy::PreferKnown).await?;

        // Infer project path (allow explicit override like other operations)
        let explicit_path = args.get("project_path").and_then(|v| v.as_str());
        let project_path = infer_project_path(explicit_path, None)
            .ok_or("No valid ESP-IDF project found (CMakeLists.txt not found)")?;

        let result = IdfCommand::new("erase-flash")
            .args(&["-p", &port])
            .project_path(&project_path)
            .timeout_secs(120)
            .execute(config).await?;

        if result.success {
            Ok(ToolOutput::success(
                format!("Erased flash on {}", port),
                serde_json::json!({ "port": port, "project_path": project_path.to_string_lossy() }),
            ))
        } else {
            Err(format!("Erase failed: {}", result.stderr))
        }
    }

    async fn get_chip_info(&self, config: &ESP32Config, args: &HashMap<String, Value>) -> Result<ToolOutput, String> {
        let port = resolve_device_port(config, args, PortResolutionPolicy::PreferKnown).await?;

        // Get chip info using IdfCommand::esptool
        let chip_result = IdfCommand::esptool("chip-id")
            .args(&["--port", &port])
            .timeout_secs(30)
            .no_parse_errors()
            .execute(config).await?;

        if !chip_result.success {
            return Err(format!("Failed to get chip info: {}", chip_result.stderr));
        }

        let stdout = &chip_result.stdout;
        
        // Parse structured info from chip-id output
        // Actual format: "Chip is ESP32-P4 (revision v1.0)"
        let mut chip_type = None;
        let mut revision = None;
        let mut features = None;
        let mut crystal_freq = None;
        let mut mac = None;

        for line in stdout.lines() {
            // Match: "Chip is ESP32-P4 (revision v1.0)" or "Chip type: ESP32-P4"
            if line.contains("Chip is") || line.starts_with("Chip type:") {
                if line.contains("Chip is") {
                    // Format: "Chip is ESP32-P4 (revision v1.0)"
                    if let Some(start) = line.find("Chip is") {
                        let rest = &line[start + "Chip is".len()..].trim();
                        if let Some(rev_pos) = rest.find("(revision") {
                            chip_type = Some(rest[..rev_pos].trim().to_string());
                            let rev_part = &rest[rev_pos..];
                            if let Some(v_start) = rev_part.find('v') {
                                if let Some(end) = rev_part[v_start..].find(')') {
                                    revision = rev_part.get(v_start..v_start+end).map(|s| s.to_string());
                                }
                            }
                        } else {
                            chip_type = Some(rest.to_string());
                        }
                    }
                } else if line.starts_with("Chip type:") {
                    // Fallback format: "Chip type: ESP32-P4"
                    if let Some(colon_pos) = line.find(':') {
                        let rest = &line[colon_pos + 1..].trim();
                        if let Some(rev_pos) = rest.find("(revision") {
                            chip_type = Some(rest[..rev_pos].trim().to_string());
                            let rev_part = &rest[rev_pos..];
                            if let Some(v_start) = rev_part.find('v') {
                                if let Some(end) = rev_part[v_start..].find(')') {
                                    revision = rev_part.get(v_start..v_start+end).map(|s| s.to_string());
                                }
                            }
                        } else {
                            chip_type = Some(rest.to_string());
                        }
                    }
                }
            } else if line.starts_with("Features:") {
                if let Some(colon_pos) = line.find(':') {
                    features = Some(line[colon_pos + 1..].trim().to_string());
                }
            } else if line.contains("Crystal is") || line.starts_with("Crystal frequency:") {
                // Format: "Crystal is 40MHz" or "Crystal frequency: 40MHz"
                if line.contains("Crystal is") {
                    if let Some(start) = line.find("Crystal is") {
                        crystal_freq = Some(line[start + "Crystal is".len()..].trim().to_string());
                    }
                } else if line.starts_with("Crystal frequency:") {
                    if let Some(colon_pos) = line.find(':') {
                        crystal_freq = Some(line[colon_pos + 1..].trim().to_string());
                    }
                }
            } else if line.starts_with("MAC:") && mac.is_none() {
                // Get the first MAC address (may appear multiple times)
                if let Some(colon_pos) = line.find(':') {
                    let mac_str = line[colon_pos + 1..].trim();
                    // Extract MAC address (format: XX:XX:XX:XX:XX:XX)
                    if mac_str.len() >= 17 {
                        mac = Some(mac_str.to_string());
                    }
                }
            }
        }

        // Try to get flash ID using IdfCommand
        let mut flash_id = None;
        let mut flash_manufacturer = None;
        let mut flash_device = None;
        let mut flash_size = None;
        
        if let Ok(flash_result) = IdfCommand::esptool("flash-id")
            .args(&["--port", &port])
            .timeout_secs(30)
            .no_parse_errors()
            .execute(config).await
        {
            if flash_result.success {
                // Look for flash information
                // Format: "Manufacturer: c8", "Device: 4018", "Detected flash size: 16MB"
                for line in flash_result.stdout.lines() {
                    if line.contains("Flash ID:") {
                        if let Some(colon_pos) = line.find(':') {
                            flash_id = Some(line[colon_pos + 1..].trim().to_string());
                        }
                    } else if line.starts_with("Manufacturer:") {
                        if let Some(colon_pos) = line.find(':') {
                            flash_manufacturer = Some(line[colon_pos + 1..].trim().to_string());
                        }
                    } else if line.starts_with("Device:") {
                        if let Some(colon_pos) = line.find(':') {
                            flash_device = Some(line[colon_pos + 1..].trim().to_string());
                        }
                    } else if line.contains("Detected flash size:") {
                        if let Some(start) = line.find("Detected flash size:") {
                            flash_size = Some(line[start + "Detected flash size:".len()..].trim().to_string());
                        }
                    }
                }
            }
        }

        // Build summary with key info
        let mut summary_parts = Vec::new();
        if let Some(ref ct) = chip_type {
            summary_parts.push(ct.clone());
            if let Some(ref rev) = revision {
                summary_parts.push(format!("({})", rev));
            }
        }
        if let Some(ref f) = features {
            summary_parts.push(f.clone());
        }
        if let Some(ref cf) = crystal_freq {
            summary_parts.push(format!("{} crystal", cf));
        }
        if let Some(ref m) = mac {
            summary_parts.push(format!("MAC: {}", m));
        }
        
        let summary = if summary_parts.is_empty() {
            format!("Chip info from {}", port)
        } else {
            format!("Chip info from {}: {}", port, summary_parts.join(", "))
        };

        record_device_port(
            &port,
            chip_type.as_deref().unwrap_or("unknown"),
            mac.as_deref().unwrap_or("unknown"),
            flash_size.as_deref().unwrap_or("unknown"),
            "info",
        ).await;

        Ok(ToolOutput::success(
            summary,
            serde_json::json!({
                "port": port,
                "chip_type": chip_type,
                "revision": revision,
                "features": features,
                "crystal_frequency": crystal_freq,
                "mac_address": mac,
                "flash_id": flash_id,
                "flash_manufacturer": flash_manufacturer,
                "flash_device": flash_device,
                "flash_size": flash_size,
                "raw_output": stdout, // Keep raw for debugging
            }),
        ))
    }

    pub async fn verify_device(&self, config: &ESP32Config, args: &HashMap<String, Value>) -> Result<ToolOutput, String> {
        // Agentic: Use board_id from session state if not provided in args
        let board_id_from_args = args.get("board_id").and_then(|v| v.as_str());
        
        // Get board_id from session if not in args (async)
        let board_id = if let Some(bid) = board_id_from_args {
            Some(bid.to_string())
        } else {
            let state = get_state().await;
            state.session.board_id.clone()
        };
        
        let board_id = board_id.ok_or_else(|| {
            "Missing required parameter: board_id. Either provide it in the tool call or set it via --board-definition CLI argument.".to_string()
        })?;

        // Reuse get_chip_info to gather device data
        let info_output = self.get_chip_info(config, args).await?;
        let data = info_output.data.clone();

        // Fetch board definition
        let board_def = self.fetch_board_definition(&board_id).await?;

        // Normalize chip types for comparison
        let expected_chip = board_def.chip.chip_type.to_lowercase();
        let actual_chip_raw = data.get("chip_type")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        
        // Normalize actual chip type (ESP32-S3 -> esp32s3, ESP32-P4 -> esp32p4, etc.)
        let actual_chip = Self::normalize_chip_type(actual_chip_raw);

        let mut warnings = Vec::new();
        if expected_chip != actual_chip {
            warnings.push(format!("Chip mismatch: expected {}, detected {} ({})", expected_chip, actual_chip, actual_chip_raw));
        }

        if let Some(ident) = &board_def.identification {
            if let Some(range) = &ident.flash_size_range {
                if let Some(actual_flash) = data.get("flash_size").and_then(|v| v.as_str()) {
                    if !range.iter().any(|s| actual_flash.contains(s)) {
                        warnings.push(format!("Flash size mismatch: expected {:?}, detected {}", range, actual_flash));
                    }
                }
            }
        }

        let status = if warnings.is_empty() {
            super::output_protocol::ToolStatus::Success
        } else {
            super::output_protocol::ToolStatus::PartialSuccess
        };

        // Include info about whether board_id came from session or args
        let board_id_source = if board_id_from_args.is_some() { "provided" } else { "session (from --board-definition)" };
        
        Ok(ToolOutput {
            status,
            action_taken: "verify".to_string(),
            data: serde_json::json!({
                "board_id": board_id,
                "board_id_source": board_id_source,
                "chip_type": data.get("chip_type"),
                "warnings": warnings,
            }),
            summary: if warnings.is_empty() {
                format!("Device verified for board {} (from {})", board_id, board_id_source)
            } else {
                format!("Device verified with warnings for board {} (from {})", board_id, board_id_source)
            },
            details: if warnings.is_empty() { None } else { Some(warnings.join("\n")) },
            state_delta: super::session_state::StateDelta::none(),
            suggested_actions: vec![],
            error: None,
        })
    }

    async fn fetch_board_definition(&self, board_id: &str) -> Result<BoardDefinition, String> {
        let state = get_state().await;
        let cache = &state.cache;

        let board_url = board_definition_url(board_id);

        cache.get_board_definition(board_id, async {
            let client = reqwest::Client::builder()
                .connect_timeout(std::time::Duration::from_secs(5))
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .map_err(|e| format!("Failed to build HTTP client: {}", e))?;
            let response = client.get(&board_url)
                .send()
                .await
                .map_err(|e| format!("Failed to fetch board definition: {}", e))?;

            if !response.status().is_success() {
                return Err(format!("Server returned error: {}", response.status()));
            }

            let board_def: BoardDefinition = response.json()
                .await
                .map_err(|e| format!("Failed to parse board definition: {}", e))?;
            Ok(board_def)
        }).await
    }

    /// Normalize chip type string to match schema format.
    /// Strips parenthetical suffixes like "(QFN56)" or "(revision v0.2)" so "ESP32-S3 (QFN56)" -> esp32s3.
    fn normalize_chip_type(chip_type: &str) -> String {
        let base = chip_type.split('(').next().unwrap_or(chip_type).trim();
        let normalized = base.to_lowercase()
            .replace("esp32-", "esp32")
            .replace("esp32s3", "esp32s3")
            .replace("esp32s2", "esp32s2")
            .replace("esp32s31", "esp32s31")
            .replace("esp32c3", "esp32c3")
            .replace("esp32c6", "esp32c6")
            .replace("esp32c61", "esp32c61")
            .replace("esp32c5", "esp32c5")
            .replace("esp32c2", "esp32c2")
            .replace("esp32p4", "esp32p4")
            .replace("esp32h2", "esp32h2")
            .replace("esp32h4", "esp32h4")
            .replace("esp32h21", "esp32h21");
        
        // If still contains spaces or dashes, remove them
        normalized.replace("-", "").replace(" ", "").to_lowercase()
    }
}

struct BoardIdHints {
    expected_chip: Option<String>,
    flash_mb: Option<u32>,
    psram_mb: Option<u32>,
}

struct HeuristicVerifyResult {
    status: String,
    warnings: Vec<String>,
}

