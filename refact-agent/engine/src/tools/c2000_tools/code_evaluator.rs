use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::Mutex as AMutex;
use tokio::fs;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::tools::tools_description::{Tool, ToolDesc, ToolParam, ToolSource, ToolSourceType};
use crate::call_validation::{ChatMessage, ChatContent, ContextEnum};
use crate::subchat::subchat_single;
// Removed unused imports

pub struct ToolC2000CodeEvaluator {
    pub config_path: String,
}

static EVALUATION_PROMPT: &str = r#"You are an expert TI C2000 microcontroller code evaluator. Your task is to analyze and compare C2000 code files to determine:

1. **Functional Correctness**: Does the code implement the intended functionality correctly?
2. **C2000-Specific Issues**: Are there any TI C2000-specific problems (register usage, memory layout, peripheral configuration)?
3. **Code Quality**: Is the code well-structured, readable, and maintainable?
4. **Performance**: Are there any performance issues or inefficiencies?
5. **Safety**: Are there any potential safety issues or undefined behavior?

**Evaluation Criteria:**
- Hardware register access patterns
- Memory management (RAM/FLASH usage)
- Peripheral configuration correctness
- Interrupt handling
- Real-time constraints
- Power management
- Code organization and documentation

**Output Format:**
Provide a structured evaluation with:
- Overall Score (1-10)
- Key Issues Found
- Recommendations for Improvement
- C2000-Specific Concerns
- Summary

Be thorough but concise. Focus on actionable feedback."#;

#[async_trait]
impl Tool for ToolC2000CodeEvaluator {
    fn as_any(&self) -> &dyn std::any::Any { self }

    fn tool_description(&self) -> ToolDesc {
        ToolDesc {
            name: "c2000_code_evaluator".to_string(),
            display_name: "C2000 Code Evaluator".to_string(),
            source: ToolSource {
                source_type: ToolSourceType::Builtin,
                config_path: self.config_path.clone(),
            },
            agentic: true,
            experimental: false,
            description: "Evaluate TI C2000 code quality, correctness, and C2000-specific issues using AI analysis".to_string(),
            parameters: vec![
                ToolParam {
                    name: "golden_file".to_string(),
                    param_type: "string".to_string(),
                    description: "Path to the golden/reference C2000 code file".to_string(),
                },
                ToolParam {
                    name: "candidate_file".to_string(),
                    param_type: "string".to_string(),
                    description: "Path to the candidate C2000 code file to evaluate".to_string(),
                },
                ToolParam {
                    name: "evaluation_type".to_string(),
                    param_type: "string".to_string(),
                    description: "Type of evaluation: 'compare' (compare against golden), 'standalone' (evaluate candidate only), 'comprehensive' (detailed analysis)".to_string(),
                },
                ToolParam {
                    name: "focus_areas".to_string(),
                    param_type: "string".to_string(),
                    description: "Comma-separated focus areas: 'functionality', 'performance', 'safety', 'c2000_specific', 'code_quality'".to_string(),
                },
            ],
            parameters_required: vec!["candidate_file".to_string()],
        }
    }

    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>,
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        let candidate_file = args.get("candidate_file")
            .and_then(|v| v.as_str())
            .ok_or("Missing required parameter: candidate_file")?;

        let golden_file = args.get("golden_file")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let evaluation_type = args.get("evaluation_type")
            .and_then(|v| v.as_str())
            .unwrap_or("standalone");

        let focus_areas = args.get("focus_areas")
            .and_then(|v| v.as_str())
            .unwrap_or("functionality,c2000_specific,code_quality");

        let mut context_files = vec![];

        // Read candidate file
        let candidate_content = match fs::read_to_string(candidate_file).await {
            Ok(content) => content,
            Err(e) => {
                return Err(format!("Failed to read candidate file '{}': {}", candidate_file, e));
            }
        };

        // Read golden file if provided
        let golden_content = if !golden_file.is_empty() {
            match fs::read_to_string(golden_file).await {
                Ok(content) => Some(content),
                Err(e) => {
                    return Err(format!("Failed to read golden file '{}': {}", golden_file, e));
                }
            }
        } else {
            None
        };

        // Create evaluation prompt based on type
        let evaluation_prompt = match evaluation_type {
            "compare" => {
                if golden_content.is_none() {
                    return Err("Golden file is required for comparison evaluation".to_string());
                }
                format!(
                    "{}\n\n**COMPARISON EVALUATION**\n\n**Golden Reference Code:**\n```c\n{}\n```\n\n**Candidate Code to Evaluate:**\n```c\n{}\n```\n\n**Focus Areas:** {}\n\nCompare the candidate code against the golden reference and provide detailed analysis.",
                    EVALUATION_PROMPT,
                    golden_content.unwrap(),
                    candidate_content,
                    focus_areas
                )
            },
            "standalone" => {
                format!(
                    "{}\n\n**STANDALONE EVALUATION**\n\n**Code to Evaluate:**\n```c\n{}\n```\n\n**Focus Areas:** {}\n\nProvide a comprehensive evaluation of this C2000 code.",
                    EVALUATION_PROMPT,
                    candidate_content,
                    focus_areas
                )
            },
            "comprehensive" => {
                format!(
                    "{}\n\n**COMPREHENSIVE EVALUATION**\n\n**Code to Evaluate:**\n```c\n{}\n```\n\n**Focus Areas:** {}\n\nProvide an extremely detailed analysis covering all aspects of C2000 development.",
                    EVALUATION_PROMPT,
                    candidate_content,
                    focus_areas
                )
            },
            _ => {
                return Err(format!("Invalid evaluation type: {}. Must be 'compare', 'standalone', or 'comprehensive'", evaluation_type));
            }
        };

        // Get current model for evaluation
        let current_model = ccx.lock().await.current_model.clone();

        // Create evaluation message
        let evaluation_message = ChatMessage {
            role: "user".to_string(),
            content: ChatContent::SimpleText(evaluation_prompt),
            finish_reason: None,
            tool_calls: None,
            tool_call_id: "".to_string(),
            tool_failed: None,
            usage: None,
            checkpoints: vec![],
            thinking_blocks: None,
        };

        // Perform LLM evaluation using subchat_single
        let evaluation_results = subchat_single(
            ccx.clone(),
            &current_model,
            vec![evaluation_message],
            None, // tools_subset
            None, // tool_choice
            false, // only_deterministic_messages
            Some(0.3), // temperature - lower for more consistent evaluation
            Some(2048), // max_new_tokens
            1, // n - number of responses
            None, // reasoning_effort
            true, // prepend_system_prompt
            None, // usage_collector_mb
            None, // tx_toolid_mb
            None, // tx_chatid_mb
        ).await.map_err(|e| format!("LLM evaluation failed: {}", e))?;

        // Extract the evaluation response
        let evaluation_text = evaluation_results.first()
            .and_then(|responses| responses.last())
            .map(|msg| msg.content.content_text_only())
            .unwrap_or_else(|| "No evaluation result received".to_string());

        // Create response messages
        let mut messages = vec![];
        
        messages.push(format!("🔍 **C2000 Code Evaluation Complete**"));
        messages.push(format!("📁 **Candidate File:** {}", candidate_file));
        if !golden_file.is_empty() {
            messages.push(format!("📁 **Golden File:** {}", golden_file));
        }
        messages.push(format!("🎯 **Evaluation Type:** {}", evaluation_type));
        messages.push(format!("🎯 **Focus Areas:** {}", focus_areas));
        messages.push("".to_string());
        messages.push("📊 **AI Evaluation Results:**".to_string());
        messages.push("".to_string());
        messages.push(evaluation_text);

        let combined_message = messages.join("\n");

        context_files.push(ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: ChatContent::SimpleText(combined_message),
            finish_reason: None,
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
            tool_failed: None,
            usage: None, // Usage will be tracked by the subchat_single call
            checkpoints: vec![],
            thinking_blocks: None,
        }));

        Ok((false, context_files))
    }

    fn tool_depends_on(&self) -> Vec<String> {
        vec!["c2000".to_string(), "thinking".to_string()]
    }
}
