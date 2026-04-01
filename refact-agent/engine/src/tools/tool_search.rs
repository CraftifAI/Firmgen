use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;

use async_trait::async_trait;
use itertools::Itertools;
use tokio::sync::Mutex as AMutex;

use crate::at_commands::at_commands::{vec_context_file_to_context_tools, AtCommandsContext};
use crate::at_commands::at_search::execute_at_search;
use crate::call_validation::{ChatContent, ChatMessage, ChatUsage, ContextEnum, ContextFile};
use crate::files_in_workspace::get_file_text_from_memory_or_disk;
use crate::subchat::subchat_single;
use crate::tools::scope_utils::create_scope_filter;
use crate::tools::tools_description::{Tool, ToolDesc, ToolParam, ToolSource, ToolSourceType};


pub struct ToolSearch {
    pub config_path: String,
}

async fn execute_att_search(
    ccx: Arc<AMutex<AtCommandsContext>>,
    query: &String,
    scope: &String,
) -> Result<Vec<ContextFile>, String> {
    let gcx = ccx.lock().await.global_context.clone();
    
    // Use the common function to create a scope filter
    let filter = create_scope_filter(gcx.clone(), scope).await?;

    info!("att-search: filter: {:?}", filter);
    execute_at_search(ccx.clone(), &query, filter).await
}

/// Run an isolated summarizer subchat over the retrieved chunks.
/// Returns (compact_summary, usage, raw_input_chars) on success, Err on fallback.
async fn summarize_search_results(
    ccx: Arc<AMutex<AtCommandsContext>>,
    queries: &[String],
    deduped: &[ContextFile],
    tool_call_id: &String,
    context_hint: &str,
) -> Result<(String, ChatUsage, usize), String> {
    let subchat_params = crate::tools::tools_execute::unwrap_subchat_params(ccx.clone(), "search_semantic").await?;

    let gcx = ccx.lock().await.global_context.clone();

    // Build the subchat user message with file content chunks
    let mut chunks_text = String::new();
    for ctx_file in deduped.iter() {
        let file_path = PathBuf::from(&ctx_file.file_name);
        let file_text = match get_file_text_from_memory_or_disk(gcx.clone(), &file_path).await {
            Ok(text) => text,
            Err(e) => {
                info!("summarize_search_results: could not load {}: {}", ctx_file.file_name, e);
                continue;
            }
        };

        // Extract only the relevant line range (line1..line2, 1-indexed)
        let lines: Vec<&str> = file_text.lines().collect();
        let start = ctx_file.line1.saturating_sub(1).min(lines.len());
        let end = ctx_file.line2.min(lines.len());
        let snippet: String = lines[start..end].join("\n");

        chunks_text.push_str(&format!(
            "📎 {}:{}-{}\n```\n{}\n```\n\n",
            ctx_file.file_name, ctx_file.line1, ctx_file.line2, snippet
        ));
    }

    let queries_list = queries.iter()
        .map(|q| format!("- {}", q))
        .collect::<Vec<_>>()
        .join("\n");

    let user_message_text = format!(
        "You are a documentation research assistant helping a developer.\n\n\
         ## CONTEXT (what the developer is working on)\n\
         {}\n\n\
         ## SEARCH QUERIES\n\
         {}\n\n\
         ## RETRIEVED CODE CHUNKS\n\
         {}\n\
         ## YOUR TASK\n\
         Extract information relevant to both the CONTEXT and QUERIES above.\n\
         Respond using the following structure:\n\n\
         ### Key Findings\n\
         - Bullet points of the most relevant discoveries\n\n\
         ### Relevant APIs / Symbols\n\
         - `function_name(args)` — brief description (file:line)\n\n\
         ### Configuration / Setup Notes\n\
         - Any config options, env vars, or setup steps found (omit section if none)\n\n\
         ### References\n\
         - file:line — brief note on what's there\n\n\
         Be concise. Omit empty sections.",
        context_hint, queries_list, chunks_text
    );

    let messages = vec![ChatMessage {
        role: "user".to_string(),
        content: ChatContent::SimpleText(user_message_text),
        ..Default::default()
    }];

    let max_tokens = if subchat_params.subchat_max_new_tokens > 0 {
        Some(subchat_params.subchat_max_new_tokens)
    } else {
        Some(2000)
    };

    let mut usage = ChatUsage::default();

    let choices = subchat_single(
        ccx.clone(),
        subchat_params.subchat_model.as_str(),
        messages,
        Some(vec![]),   // no tools
        None,           // no tool_choice
        false,
        subchat_params.subchat_temperature,
        max_tokens,
        1,
        subchat_params.subchat_reasoning_effort.clone(),
        false,
        Some(&mut usage),
        Some(tool_call_id.clone()),
        Some(format!("{}-search-semantic-summarizer", tool_call_id)),
    ).await?;

    let raw_input_chars = chunks_text.len();

    // Extract the assistant's reply text
    let session = choices.into_iter().next()
        .ok_or_else(|| "subchat returned no choices".to_string())?;
    let reply = session.last()
        .ok_or_else(|| "subchat session was empty".to_string())?;

    match &reply.content {
        ChatContent::SimpleText(text) => Ok((text.clone(), usage, raw_input_chars)),
        ChatContent::Multimodal(_) => {
            let text = reply.content.content_text_only();
            if text.is_empty() {
                Err("subchat returned no text content".to_string())
            } else {
                Ok((text, usage, raw_input_chars))
            }
        }
    }
}

#[async_trait]
impl Tool for ToolSearch {
    fn as_any(&self) -> &dyn std::any::Any { self }

    fn tool_description(&self) -> ToolDesc {
        ToolDesc {
            name: "search_semantic".to_string(),
            display_name: "Search".to_string(),
            source: ToolSource {
                source_type: ToolSourceType::Builtin,
                config_path: self.config_path.clone(),
            },
            agentic: true,
            experimental: false,
            description: "Find semantically similar pieces of code or text using vector database (semantic search)".to_string(),
            parameters: vec![
                ToolParam {
                    name: "queries".to_string(),
                    param_type: "string".to_string(),
                    description: "Comma-separated list of queries. Each query can be a single line, paragraph or code sample to search for semantically similar content.".to_string(),
                },
                ToolParam {
                    name: "scope".to_string(),
                    param_type: "string".to_string(),
                    description: "'workspace' to search all files in workspace, 'dir/subdir/' to search in files within a directory, 'dir/file.ext' to search in a single file.".to_string(),
                },
                ToolParam {
                    name: "context_summary".to_string(),
                    param_type: "string".to_string(),
                    description: "Brief summary of the current task or goal driving this search. Helps the summarizer extract the most relevant information from results.".to_string(),
                },
            ],
            parameters_required: vec!["queries".to_string(), "scope".to_string()],
        }
    }
    
    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>,
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        let query_str = match args.get("queries") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => return Err(format!("argument `queries` is not a string: {:?}", v)),
            None => return Err("Missing argument `queries` in the search_semantic() call.".to_string())
        };
        let scope = match args.get("scope") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => return Err(format!("argument `scope` is not a string: {:?}", v)),
            None => return Err("Missing argument `scope` in the search_semantic() call.".to_string())
        };

        let queries: Vec<String> = query_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        if queries.is_empty() {
            return Err("No valid queries provided".to_string());
        }

        // Extract explicit context_summary or fall back to last user message
        let context_hint = match args.get("context_summary") {
            Some(Value::String(s)) if !s.trim().is_empty() => s.clone(),
            _ => {
                // Fallback: grab the last user message from the main conversation
                let ccx_lock = ccx.lock().await;
                ccx_lock.messages.iter().rev()
                    .find(|m| m.role == "user")
                    .map(|m| {
                        let text = m.content.content_text_only();
                        if text.len() > 500 { text[..500].to_string() } else { text }
                    })
                    .unwrap_or_else(|| "General code search".to_string())
            }
        };
        info!("att-search: context_hint={}", crate::nicer_logs::first_n_chars(&context_hint, 120));

        let top_n = ccx.lock().await.top_n;

        // Collect all results from all queries
        let mut all_query_results: Vec<Vec<ContextFile>> = Vec::new();
        let mut all_content = String::new();

        for (i, query) in queries.iter().enumerate() {
            if i > 0 {
                all_content.push_str("\n\n");
            }

            all_content.push_str(&format!("Results for query: \"{}\"\n", query));

            let vector_of_context_file = execute_att_search(ccx.clone(), query, &scope).await?;
            info!("att-search: vector_of_context_file={:?}", vector_of_context_file);

            if vector_of_context_file.is_empty() {
                all_content.push_str("No results found for this query.\n");
                continue;
            }

            all_content.push_str("Records found:\n\n");
            let mut file_results_to_reqs: HashMap<String, Vec<&ContextFile>> = HashMap::new();
            vector_of_context_file.iter().for_each(|rec| {
                file_results_to_reqs.entry(rec.file_name.clone()).or_insert(vec![]).push(rec)
            });

            let mut used_files: HashSet<String> = HashSet::new();
            for rec in vector_of_context_file.iter().sorted_by(|rec1, rec2| rec2.usefulness.total_cmp(&rec1.usefulness)) {
                if !used_files.contains(&rec.file_name) {
                    all_content.push_str(&format!("{}:\n", rec.file_name.clone()));
                    let file_recs = file_results_to_reqs.get(&rec.file_name).unwrap();
                    for file_req in file_recs.iter().sorted_by(|rec1, rec2| rec2.usefulness.total_cmp(&rec1.usefulness)) {
                        all_content.push_str(&format!("    lines {}-{} score {:.1}%\n", file_req.line1, file_req.line2, file_req.usefulness));
                    }
                    used_files.insert(rec.file_name.clone());
                }
            }

            all_query_results.push(vector_of_context_file);
        }

        // Deduplicate: for each file, keep only the entry with the highest usefulness score
        // across all queries. This prevents the same file appearing dozens of times when
        // multiple related queries return the same files.
        let mut best_per_file: HashMap<String, ContextFile> = HashMap::new();
        for results in all_query_results {
            for ctx_file in results {
                let entry = best_per_file.entry(ctx_file.file_name.clone()).or_insert_with(|| ctx_file.clone());
                if ctx_file.usefulness > entry.usefulness {
                    *entry = ctx_file;
                }
            }
        }

        if best_per_file.is_empty() {
            return Err("All searches produced no results, adjust the queries or try a different scope.".to_string());
        }

        // Sort deduplicated results by usefulness and cap at top_n files
        let mut deduped: Vec<ContextFile> = best_per_file.into_values().collect();
        deduped.sort_by(|a, b| b.usefulness.total_cmp(&a.usefulness));
        deduped.truncate(top_n);
        info!("att-search: deduplicated to {} unique files (capped at top_n={})", deduped.len(), top_n);

        // Attempt subchat summarization; fall back to raw content if not configured.
        // IMPORTANT: when summarization succeeds we only return the compact summary to the main
        // context — intentionally NOT the raw ContextFile chunks. The subagent already consumed
        // those in its isolated window; injecting them again would defeat the entire purpose and
        // roughly double the tokens charged to the main context.
        // When summarization fails we fall back to the previous behaviour (raw chunks + file list).
        match summarize_search_results(ccx.clone(), &queries, &deduped, tool_call_id, &context_hint).await {
            Ok((summary, usage, raw_chars)) => {
                let raw_tokens_est = raw_chars / 4;
                let saved_tokens_est = raw_tokens_est.saturating_sub(usage.prompt_tokens);
                info!(
                    "att-search: subchat summarization OK | model prompt_tokens={} completion_tokens={} | raw_input_chars={} (~{} tokens) | saved~{} tokens from main context",
                    usage.prompt_tokens, usage.completion_tokens, raw_chars, raw_tokens_est, saved_tokens_est
                );
                let tool_message_content = format!(
                    "{}

---
⚡ **search_semantic subchat stats** (summarizer ran separately, not in main agent context)
- Files summarized: {}
- Subchat prompt tokens: {}
- Subchat completion tokens: {}
- Raw chunk size: ~{} chars (~{} tokens est.)
- Tokens saved from main context: ~{}",
                    summary,
                    deduped.len(),
                    usage.prompt_tokens,
                    usage.completion_tokens,
                    raw_chars,
                    raw_tokens_est,
                    saved_tokens_est,
                );
                // Only the summary goes back — raw chunks stay out of the main context.
                let results = vec![ContextEnum::ChatMessage(ChatMessage {
                    role: "tool".to_string(),
                    content: ChatContent::SimpleText(tool_message_content),
                    tool_calls: None,
                    tool_call_id: tool_call_id.clone(),
                    ..Default::default()
                })];
                Ok((false, results))
            }
            Err(e) => {
                info!("att-search: subchat summarization not available ({}), falling back to raw content", e);
                // Fallback: return raw ContextFile chunks + the plain-text listing so the main
                // agent still has something useful to work with.
                let mut results = vec_context_file_to_context_tools(deduped);
                results.push(ContextEnum::ChatMessage(ChatMessage {
                    role: "tool".to_string(),
                    content: ChatContent::SimpleText(all_content),
                    tool_calls: None,
                    tool_call_id: tool_call_id.clone(),
                    ..Default::default()
                }));
                Ok((false, results))
            }
        }
    }

    fn tool_depends_on(&self) -> Vec<String> {
        vec!["vecdb".to_string()]
    }
}
