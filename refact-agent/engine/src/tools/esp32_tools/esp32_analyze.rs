use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::Mutex as AMutex;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::tools::tools_description::{Tool, ToolDesc, ToolParam, ToolSource, ToolSourceType};
use crate::call_validation::{ChatMessage, ChatContent, ContextEnum};
use crate::subchat::subchat_single;

use super::output_protocol::ToolOutput;
use super::global_state::get_config;

pub struct ESP32Analyze {
    pub config_path: String,
}

#[async_trait]
impl Tool for ESP32Analyze {
    fn as_any(&self) -> &dyn std::any::Any { self }

    fn tool_description(&self) -> ToolDesc {
        ToolDesc {
            name: "esp32_analyze".to_string(),
            display_name: "ESP32 Analyze".to_string(),
            source: ToolSource {
                source_type: ToolSourceType::Builtin,
                config_path: self.config_path.clone(),
            },
            agentic: true,
            experimental: false,
            description: "Analyze ESP32 code quality, correctness, and ESP-IDF-specific issues using AI analysis. Operations: evaluate (evaluate code), check_errors (check for common errors), suggest_fixes (suggest improvements).".to_string(),
            parameters: vec![
                ToolParam {
                    name: "operation".to_string(),
                    param_type: "string".to_string(),
                    description: "Operation: 'evaluate' (evaluate code), 'check_errors' (check for errors), 'suggest_fixes' (suggest fixes)".to_string(),
                },
                ToolParam {
                    name: "file_path".to_string(),
                    param_type: "string".to_string(),
                    description: "Path to code file to analyze".to_string(),
                },
                ToolParam {
                    name: "focus".to_string(),
                    param_type: "string".to_string(),
                    description: "Focus area: 'functionality', 'performance', 'safety', 'esp32_specific'".to_string(),
                },
            ],
            parameters_required: vec!["operation".to_string(), "file_path".to_string()],
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

        let file_path = args.get("file_path")
            .and_then(|v| v.as_str())
            .ok_or("Missing required parameter: file_path")?;

        // Use global config (cached, configurable via env var)
        let _config = get_config().await?;

        let output = match operation {
            "evaluate" => self.evaluate_code(ccx.clone(), file_path, args).await?,
            "check_errors" => self.check_errors(file_path, args).await?,
            "suggest_fixes" => self.suggest_fixes(ccx.clone(), file_path, args).await?,
            _ => return Err(format!("Unknown operation: {}", operation)),
        };

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
        vec!["esp32".to_string(), "thinking".to_string()]
    }
}

impl ESP32Analyze {
    async fn evaluate_code(
        &self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        file_path: &str,
        args: &HashMap<String, Value>,
    ) -> Result<ToolOutput, String> {
        let focus = args.get("focus")
            .and_then(|v| v.as_str())
            .unwrap_or("functionality");

        let content = tokio::fs::read_to_string(file_path).await
            .map_err(|e| format!("Failed to read file: {}", e))?;

        let prompt = format!(
            r#"You are an expert ESP32/ESP-IDF code evaluator. Analyze this code for:

1. **Functional Correctness**: Does the code implement the intended functionality correctly?
2. **ESP-IDF-Specific Issues**: Are there any ESP-IDF-specific problems (API usage, memory management, FreeRTOS, etc.)?
3. **Code Quality**: Is the code well-structured, readable, and maintainable?
4. **Performance**: Are there any performance issues or inefficiencies?
5. **Safety**: Are there any potential safety issues or undefined behavior?

**Focus Area**: {}

**Code to Evaluate:**
```c
{}
```

Provide a structured evaluation with:
- Overall Score (1-10)
- Key Issues Found
- Recommendations for Improvement
- ESP-IDF-Specific Concerns
- Summary

Be thorough but concise. Focus on actionable feedback."#,
            focus, content
        );

        let current_model = ccx.lock().await.current_model.clone();
        let evaluation_message = ChatMessage {
            role: "user".to_string(),
            content: ChatContent::SimpleText(prompt),
            finish_reason: None,
            tool_calls: None,
            tool_call_id: "".to_string(),
            tool_failed: None,
            usage: None,
            checkpoints: vec![],
            thinking_blocks: None,
        };

        let evaluation_results = subchat_single(
            ccx.clone(),
            &current_model,
            vec![evaluation_message],
            None,
            None,
            false,
            Some(0.3),
            Some(2048),
            1,
            None,
            true,
            None,
            None,
            None,
        ).await.map_err(|e| format!("LLM evaluation failed: {}", e))?;

        let evaluation_text = evaluation_results.first()
            .and_then(|responses| responses.last())
            .map(|msg| msg.content.content_text_only())
            .unwrap_or_else(|| "No evaluation result received".to_string());

        Ok(ToolOutput::success(
            format!("Evaluated {}", file_path),
            serde_json::json!({
                "file": file_path,
                "focus": focus,
                "evaluation": evaluation_text,
            }),
        ))
    }

    async fn check_errors(&self, file_path: &str, _args: &HashMap<String, Value>) -> Result<ToolOutput, String> {
        let content = tokio::fs::read_to_string(file_path).await
            .map_err(|e| format!("Failed to read file: {}", e))?;

        let mut issues: Vec<serde_json::Value> = Vec::new();
        let lines: Vec<&str> = content.lines().collect();

        // === Memory Management ===
        if content.contains("malloc(") && !content.contains("heap_caps_malloc") && !content.contains("pvPortMalloc") {
            issues.push(serde_json::json!({
                "category": "memory",
                "severity": "warning",
                "message": "Using malloc() instead of heap_caps_malloc()",
                "fix": "Consider using heap_caps_malloc() for better control over memory allocation (DRAM/IRAM/SPIRAM)"
            }));
        }

        if content.contains("free(") && !content.contains("heap_caps_free") {
            if content.contains("heap_caps_malloc") {
                issues.push(serde_json::json!({
                    "category": "memory",
                    "severity": "error",
                    "message": "Using free() with heap_caps_malloc()",
                    "fix": "Use heap_caps_free() for memory allocated with heap_caps_malloc()"
                }));
            }
        }

        // === FreeRTOS ===
        for (i, line) in lines.iter().enumerate() {
            if line.contains("vTaskDelay(") && !line.contains("pdMS_TO_TICKS") && !line.contains("portTICK_PERIOD_MS") {
                // Check if it's a raw number
                if line.contains("vTaskDelay(1") || line.contains("vTaskDelay(2") || 
                   line.contains("vTaskDelay(5") || line.contains("vTaskDelay(10") {
                    issues.push(serde_json::json!({
                        "category": "freertos",
                        "severity": "warning",
                        "line": i + 1,
                        "message": "vTaskDelay() with raw tick count",
                        "fix": "Use vTaskDelay(pdMS_TO_TICKS(ms)) for portable delays"
                    }));
                }
            }

            if line.contains("xTaskCreate(") && !line.contains("xTaskCreatePinnedToCore") {
                issues.push(serde_json::json!({
                    "category": "freertos",
                    "severity": "info",
                    "line": i + 1,
                    "message": "Using xTaskCreate() instead of xTaskCreatePinnedToCore()",
                    "fix": "Consider xTaskCreatePinnedToCore() to control which CPU core runs the task"
                }));
            }

            // Stack size check
            if line.contains("xTaskCreate") {
                // Look for small stack sizes (less than 2048)
                if let Some(pos) = line.find("xTaskCreate") {
                    let after = &line[pos..];
                    for num in ["1024", "512", "256", "1000", "500"] {
                        if after.contains(&format!(", {}, ", num)) || after.contains(&format!(", {},", num)) {
                            issues.push(serde_json::json!({
                                "category": "freertos",
                                "severity": "warning",
                                "line": i + 1,
                                "message": format!("Small stack size ({} bytes) may cause stack overflow", num),
                                "fix": "ESP32 tasks typically need at least 2048-4096 bytes stack"
                            }));
                        }
                    }
                }
            }
        }

        // === Wi-Fi ===
        if content.contains("esp_wifi_") {
            if !content.contains("esp_netif_init") && !content.contains("ESP_ERROR_CHECK") {
                issues.push(serde_json::json!({
                    "category": "wifi",
                    "severity": "warning",
                    "message": "Wi-Fi code without proper initialization check",
                    "fix": "Ensure esp_netif_init() and esp_event_loop_create_default() are called"
                }));
            }
        }

        // === GPIO ===
        if content.contains("gpio_set_level") && !content.contains("gpio_set_direction") && !content.contains("gpio_config") {
            issues.push(serde_json::json!({
                "category": "gpio",
                "severity": "warning",
                "message": "Setting GPIO level without configuring direction",
                "fix": "Call gpio_set_direction() or gpio_config() before gpio_set_level()"
            }));
        }

        // === Error Handling ===
        let esp_calls = content.matches("esp_").count();
        let error_checks = content.matches("ESP_ERROR_CHECK").count() + content.matches("!= ESP_OK").count();
        if esp_calls > 5 && error_checks < esp_calls / 3 {
            issues.push(serde_json::json!({
                "category": "error_handling",
                "severity": "warning",
                "message": format!("Low error checking: {} ESP-IDF calls but only {} error checks", esp_calls, error_checks),
                "fix": "Use ESP_ERROR_CHECK() or check for ESP_OK return values"
            }));
        }

        // === Deprecated APIs ===
        let deprecated = [
            ("tcpip_adapter_init", "Use esp_netif_init() instead"),
            ("WIFI_MODE_NULL", "Use esp_wifi_stop() to disable Wi-Fi"),
            ("nvs_flash_init_partition", "Use nvs_flash_init() for default partition"),
        ];
        for (api, fix) in deprecated {
            if content.contains(api) {
                issues.push(serde_json::json!({
                    "category": "deprecated",
                    "severity": "warning",
                    "message": format!("Deprecated API: {}", api),
                    "fix": fix
                }));
            }
        }

        // === Interrupt Safety ===
        if content.contains("ISR") || content.contains("_isr") || content.contains("IRAM_ATTR") {
            if content.contains("printf") || content.contains("ESP_LOG") {
                issues.push(serde_json::json!({
                    "category": "interrupt",
                    "severity": "error",
                    "message": "Logging/printf in ISR context",
                    "fix": "Never use printf/ESP_LOG in ISR. Use a queue to pass data to a task."
                }));
            }
            if content.contains("malloc") || content.contains("free") {
                issues.push(serde_json::json!({
                    "category": "interrupt",
                    "severity": "error",
                    "message": "Memory allocation in ISR context",
                    "fix": "Never allocate/free memory in ISR. Pre-allocate buffers."
                }));
            }
        }

        if issues.is_empty() {
            Ok(ToolOutput::success(
                format!("No issues found in {}", file_path),
                serde_json::json!({ 
                    "issues": [],
                    "file": file_path,
                    "lines_analyzed": lines.len(),
                }),
            ))
        } else {
            // Group by category
            let by_severity: std::collections::HashMap<String, usize> = issues.iter()
                .filter_map(|i| i.get("severity").and_then(|s| s.as_str()).map(|s| s.to_string()))
                .fold(std::collections::HashMap::new(), |mut acc, s| {
                    *acc.entry(s).or_insert(0) += 1;
                    acc
                });

            Ok(ToolOutput {
                status: if by_severity.get("error").unwrap_or(&0) > &0 {
                    super::output_protocol::ToolStatus::Failed
                } else {
                    super::output_protocol::ToolStatus::PartialSuccess
                },
                action_taken: "check_errors".to_string(),
                data: serde_json::json!({ 
                    "issues": issues,
                    "file": file_path,
                    "by_severity": by_severity,
                }),
                summary: format!("Found {} issue(s): {} error(s), {} warning(s)", 
                    issues.len(),
                    by_severity.get("error").unwrap_or(&0),
                    by_severity.get("warning").unwrap_or(&0)),
                details: Some(issues.iter()
                    .filter_map(|i| i.get("message").and_then(|m| m.as_str()))
                    .collect::<Vec<_>>()
                    .join("\n")),
                state_delta: super::session_state::StateDelta::none(),
                suggested_actions: vec![],
                error: None,
            })
        }
    }

    async fn suggest_fixes(
        &self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        file_path: &str,
        _args: &HashMap<String, Value>,
    ) -> Result<ToolOutput, String> {
        // Similar to evaluate but focused on fixes
        self.evaluate_code(ccx, file_path, &HashMap::new()).await
    }
}

