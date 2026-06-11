//! PIR sub-agent — LLM-driven topology generation and hybrid refine pass.

pub mod context;
pub mod merge;
pub mod parser;
pub mod prompt;

use std::path::Path;
use std::sync::Arc;
use std::io::Write;

use tokio::sync::RwLock as ARwLock;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::{ChatContent, ChatMessage, ChatUsage};
use crate::caps::CodeAssistantCaps;
use crate::global_context::{try_load_caps_quickly_if_not_present, GlobalContext};
use crate::subchat::subchat_single;

use super::board_validate::apply_board_validation;
use super::builder::{compute_revision, finalize_pir_result, graph_from_pir};
use super::persistence::{compute_manifest_revision, save_history_snapshot, save_pir};
use super::schema::{AnalysisFacts, PirAnalyzeResult, PirDocument, PirGenerationMeta, TokenUsage};

const PIR_AGENT_N_CTX: usize = 64000;
const PIR_REFINE_N_CTX: usize = 16000;
const PIR_AGENT_TEMPERATURE: f32 = 0.2;
const PIR_KNOWLEDGE_MODEL: &str = "gpt-5.2";
const PIR_TOKEN_LOG_FILE: &str = "PIR_token";
const DEBUG_LOG_PATH: &str = "debug-10e772.log";

struct PirLlmSubchatResult {
    text: String,
    model_id: String,
    token_usage: Option<TokenUsage>,
}

fn debug_mode_log(
    run_id: &str,
    hypothesis_id: &str,
    location: &str,
    message: &str,
    data: serde_json::Value,
) {
    let payload = serde_json::json!({
        "sessionId": "10e772",
        "runId": run_id,
        "hypothesisId": hypothesis_id,
        "location": location,
        "message": message,
        "data": data,
        "timestamp": now_ms(),
    });
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(DEBUG_LOG_PATH)
    {
        let _ = writeln!(file, "{}", payload);
    }
}

fn resolve_pir_model_id(caps: &CodeAssistantCaps) -> String {
    let preferred = format!("{}/{}", caps.cloud_name, PIR_KNOWLEDGE_MODEL);
    if caps.chat_models.contains_key(&preferred) {
        preferred
    } else {
        caps.defaults.chat_default_model.clone()
    }
}

fn saturating_u32(value: usize) -> u32 {
    value.min(u32::MAX as usize) as u32
}

fn token_usage_from_chat_usage(usage: &ChatUsage) -> Option<TokenUsage> {
    if usage.prompt_tokens == 0 && usage.completion_tokens == 0 && usage.total_tokens == 0 {
        return None;
    }
    Some(TokenUsage {
        prompt_tokens: saturating_u32(usage.prompt_tokens),
        completion_tokens: saturating_u32(usage.completion_tokens),
    })
}

async fn append_pir_token_log(
    gcx: Arc<ARwLock<GlobalContext>>,
    chat_id: &str,
    stage: &str,
    model_id: &str,
    n_ctx: usize,
    request_chars: usize,
    response_chars: usize,
    usage: &ChatUsage,
) {
    let cache_dir = { gcx.read().await.cache_dir.clone() };
    let log_dir = resolve_pir_log_dir(&cache_dir);
    let log_file = log_dir.join(PIR_TOKEN_LOG_FILE);
    let payload = serde_json::json!({
        "timestamp_ms": now_ms(),
        "chat_id": chat_id,
        "stage": stage,
        "model": model_id,
        "n_ctx": n_ctx,
        "request_chars": request_chars,
        "response_chars": response_chars,
        "prompt_tokens": usage.prompt_tokens,
        "completion_tokens": usage.completion_tokens,
        "total_tokens": usage.total_tokens,
    });
    if std::fs::create_dir_all(&log_dir).is_ok() {
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_file)
        {
            let _ = writeln!(file, "{}", payload);
        }
    }
}

fn resolve_pir_log_dir(cache_dir: &std::path::Path) -> std::path::PathBuf {
    if let Ok(explicit) = std::env::var("CRAFTIF_LOG_DIR") {
        let explicit = explicit.trim();
        if !explicit.is_empty() {
            return std::path::PathBuf::from(explicit);
        }
    }

    #[cfg(windows)]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            let appdata = appdata.trim();
            if !appdata.is_empty() {
                return std::path::PathBuf::from(appdata)
                    .join("craftifai")
                    .join("logs");
            }
        }
    }

    cache_dir.join("logs")
}

pub fn agent_mode() -> String {
    std::env::var("PIR_AGENT_MODE").unwrap_or_else(|_| "hybrid".to_string())
}

pub fn is_static_mode() -> bool {
    agent_mode() == "static"
}

pub fn is_legacy_ai_mode() -> bool {
    agent_mode() == "ai"
}

pub fn is_ai_facts_mode() -> bool {
    agent_mode() == "ai_facts"
}

pub fn is_hybrid_mode() -> bool {
    !is_static_mode() && !is_legacy_ai_mode() && !is_ai_facts_mode()
}

pub fn should_run_ai_refine() -> bool {
    std::env::var("PIR_AI_REFINE")
        .map(|v| v != "0" && v.to_lowercase() != "false")
        .unwrap_or(true)
}

pub async fn run_ai_analyze(
    gcx: Arc<ARwLock<GlobalContext>>,
    chat_id: &str,
    project_path: &Path,
    incremental: bool,
    triggered_by: &str,
    previous: Option<PirDocument>,
) -> Result<PirAnalyzeResult, String> {
    let manifest = super::analyzer::manifest::build_manifest(project_path)?;
    let changed_only = if incremental {
        previous
            .as_ref()
            .map(|p| context::diff_changed_files(&p.provenance.file_hashes, &manifest.hashes))
    } else {
        None
    };

    let (context_files, file_hashes) =
        context::build_project_context(project_path, changed_only.as_ref())?;

    if incremental && context_files.is_empty() && previous.is_some() {
        let prev = previous.clone().unwrap();
        let graph = graph_from_pir(&prev, &context::project_name_from_path(project_path));
        let validation = crate::firmware_topology::validate_graph(&graph);
        return Ok(PirAnalyzeResult {
            status: "ready".to_string(),
            pir: prev,
            graph,
            validation,
            diff: None,
        });
    }

    let revision = compute_manifest_revision(&file_hashes);
    let analyzed_files: Vec<String> = context_files.iter().map(|f| f.rel_path.clone()).collect();

    let board_context: Option<String> = None;
    let board_id: Option<String> = None;

    let context_block = context::format_context_block(&context_files);
    let user_message = format!(
        "Project path: {}\nRevision: {}\nIncremental: {}\n\nBoard context:\n{}\n\nSource files:\n{}",
        project_path.display(),
        revision,
        incremental,
        board_context.as_deref().unwrap_or("(none)"),
        context_block
    );

    let llm_result = run_llm_subchat(
        gcx,
        chat_id,
        prompt::PIR_AGENT_SYSTEM_PROMPT,
        &user_message,
        PIR_AGENT_N_CTX,
        "ai_analyze",
    )
    .await?;

    let generation = PirGenerationMeta {
        mode: if incremental {
            "ai_incremental".to_string()
        } else {
            "ai_full".to_string()
        },
        model: Some(llm_result.model_id.clone()),
        triggered_by: triggered_by.to_string(),
        analyzed_at_ms: now_ms(),
        input_files: analyzed_files.clone(),
        token_usage: llm_result.token_usage.clone(),
    };

    let mut pir = parser::parse_agent_output(
        &llm_result.text,
        &project_path.to_string_lossy(),
        Some(chat_id),
        &revision,
        board_id.clone(),
        file_hashes.keys().cloned().collect(),
        file_hashes.clone(),
        generation,
        previous.as_ref(),
    )?;

    let diff = merge::merge_with_previous(&mut pir, previous.as_ref());
    apply_board_validation(&mut pir, board_context.as_deref(), board_id.as_deref());

    for node in &mut pir.nodes {
        node.sync.last_synced_revision = revision.clone();
    }

    let (pir, graph, validation, diff) =
        finalize_pir_result(pir, project_path, previous.as_ref(), diff);

    let _ = save_pir(project_path, &pir);
    let _ = save_history_snapshot(project_path, &pir);

    Ok(PirAnalyzeResult {
        status: "ready".to_string(),
        pir,
        graph,
        validation,
        diff,
    })
}

/// Full-build path: same AnalysisFacts input as static builder, but topology JSON comes from LLM.
pub async fn run_ai_build_from_facts(
    gcx: Arc<ARwLock<GlobalContext>>,
    chat_id: &str,
    project_path: &Path,
    incremental: bool,
    facts: &AnalysisFacts,
    triggered_by: &str,
    previous: Option<&PirDocument>,
) -> Result<PirAnalyzeResult, String> {
    let manifest = super::analyzer::manifest::build_manifest(project_path)?;
    let revision = compute_revision(facts);
    let manifest_only: Vec<String> = manifest
        .hashes
        .keys()
        .filter(|k| !facts.file_hashes.contains_key(*k))
        .take(5)
        .cloned()
        .collect();
    let manifest_only_count = manifest
        .hashes
        .keys()
        .filter(|k| !facts.file_hashes.contains_key(*k))
        .count();
    let facts_only: Vec<String> = facts
        .file_hashes
        .keys()
        .filter(|k| !manifest.hashes.contains_key(*k))
        .take(5)
        .cloned()
        .collect();
    let facts_only_count = facts
        .file_hashes
        .keys()
        .filter(|k| !manifest.hashes.contains_key(*k))
        .count();
    let mismatched_hashes: Vec<String> = manifest
        .hashes
        .iter()
        .filter_map(|(rel, hash)| match facts.file_hashes.get(rel) {
            Some(other) if other != hash => Some(rel.clone()),
            _ => None,
        })
        .take(5)
        .collect();
    let mismatch_count = manifest
        .hashes
        .iter()
        .filter(|(rel, hash)| {
            facts.file_hashes.get(*rel).map(|other| other != *hash).unwrap_or(false)
        })
        .count();
    // #region agent log
    debug_mode_log(
        "critical-bugs",
        "H4",
        "agent/mod.rs:run_ai_build_from_facts:manifest_hash_compare",
        "compared manifest hashes with facts file_hashes",
        serde_json::json!({
            "manifest_hash_count": manifest.hashes.len(),
            "facts_hash_count": facts.file_hashes.len(),
            "manifest_only_count": manifest_only_count,
            "facts_only_count": facts_only_count,
            "mismatch_count": mismatch_count,
            "manifest_only_sample": manifest_only,
            "facts_only_sample": facts_only,
            "mismatch_sample": mismatched_hashes,
            "revision_source": "compute_revision(facts)",
            "prompt_has_confidence_ge_1": prompt::PIR_AGENT_SYSTEM_PROMPT.contains("confidence >= 1.0"),
            "prompt_has_confidence_090": prompt::PIR_AGENT_SYSTEM_PROMPT.contains("0.90 confidence"),
        }),
    );
    // #endregion
    // #region agent log
    debug_mode_log(
        "ai-facts-test",
        "H2",
        "agent/mod.rs:run_ai_build_from_facts:entry",
        "entered ai_facts full-build",
        serde_json::json!({
            "incremental": incremental,
            "triggered_by": triggered_by,
            "gpio_facts": facts.gpio_facts.len(),
            "task_facts": facts.task_facts.len(),
            "network_facts": facts.network_facts.len(),
            "unresolved": facts.unresolved.len(),
            "analyzed_files": facts.analyzed_files.len(),
        }),
    );
    // #endregion
    let facts_json = serde_json::to_string_pretty(facts).unwrap_or_else(|_| "{}".to_string());
    let unresolved_json =
        serde_json::to_string_pretty(&facts.unresolved).unwrap_or_else(|_| "[]".to_string());
    let snippets =
        context::build_unresolved_snippets(project_path, &facts.unresolved, &manifest.contents);
    let previous_summary = previous
        .map(context::compact_pir_summary)
        .unwrap_or_default();

    let board_context: Option<String> = None;
    let board_id: Option<String> = None;

    let user_message = format!(
        "Project path: {}\nRevision: {}\nIncremental: {}\n\nBoard context:\n{}\n\nRust AnalysisFacts (canonical input):\n{}\n\nPrevious PIR summary:\n{}\n\nUnresolved gaps:\n{}\n\nTargeted snippets:\n{}",
        project_path.display(),
        revision,
        incremental,
        board_context.as_deref().unwrap_or("(none)"),
        facts_json,
        if previous_summary.is_empty() { "(none)" } else { &previous_summary },
        unresolved_json,
        snippets
    );

    let llm_result = run_llm_subchat(
        gcx,
        chat_id,
        prompt::PIR_BUILD_FROM_FACTS_SYSTEM_PROMPT,
        &user_message,
        PIR_AGENT_N_CTX,
        "ai_build_from_facts",
    )
    .await?;
    // #region agent log
    debug_mode_log(
        "ai-facts-test",
        "H3",
        "agent/mod.rs:run_ai_build_from_facts:llm_result",
        "llm output received for ai_facts",
        serde_json::json!({
            "output_chars": llm_result.text.len(),
            "has_nodes_key": llm_result.text.contains("\"nodes\""),
            "has_edges_key": llm_result.text.contains("\"edges\""),
            "has_diagrams_key": llm_result.text.contains("\"diagrams\""),
        }),
    );
    // #endregion

    let generation = PirGenerationMeta {
        mode: if incremental {
            "ai_facts_incremental".to_string()
        } else {
            "ai_facts_full".to_string()
        },
        model: Some(llm_result.model_id.clone()),
        triggered_by: triggered_by.to_string(),
        analyzed_at_ms: now_ms(),
        input_files: facts.analyzed_files.clone(),
        token_usage: llm_result.token_usage.clone(),
    };

    let mut pir = parser::parse_agent_output(
        &llm_result.text,
        &project_path.to_string_lossy(),
        Some(chat_id),
        &revision,
        board_id.clone(),
        facts.analyzed_files.clone(),
        facts.file_hashes.clone(),
        generation,
        previous,
    )?;
    // #region agent log
    debug_mode_log(
        "ai-facts-test",
        "H4",
        "agent/mod.rs:run_ai_build_from_facts:parse_ok",
        "parsed ai_facts PIR document",
        serde_json::json!({
            "nodes": pir.nodes.len(),
            "edges": pir.edges.len(),
            "unresolved": pir.unresolved.len(),
            "generation_mode": pir.generation.mode,
        }),
    );
    // #endregion

    let diff = merge::merge_with_previous(&mut pir, previous);
    apply_board_validation(&mut pir, board_context.as_deref(), board_id.as_deref());

    for node in &mut pir.nodes {
        node.sync.last_synced_revision = revision.clone();
    }

    let (pir, graph, validation, diff) = finalize_pir_result(pir, project_path, previous, diff);
    // #region agent log
    debug_mode_log(
        "ai-facts-test",
        "H5",
        "agent/mod.rs:run_ai_build_from_facts:finalize",
        "finalized ai_facts result",
        serde_json::json!({
            "graph_nodes": graph.nodes.len(),
            "graph_connections": graph.connections.len(),
            "pir_nodes": pir.nodes.len(),
            "pir_edges": pir.edges.len(),
            "validation_valid": validation.valid,
            "validation_issues": validation.issues.len(),
        }),
    );
    // #endregion
    // #region agent log
    debug_mode_log(
        "critical-bugs",
        "H2",
        "agent/mod.rs:run_ai_build_from_facts:return",
        "returning ai_facts result from agent layer",
        serde_json::json!({
            "direct_save_in_agent_layer": false,
            "generation_mode": pir.generation.mode.clone(),
            "revision": pir.revision.clone(),
            "nodes": pir.nodes.len(),
            "edges": pir.edges.len(),
        }),
    );
    // #endregion

    Ok(PirAnalyzeResult {
        status: "ready".to_string(),
        pir,
        graph,
        validation,
        diff,
    })
}

/// Hybrid refine pass — targeted LLM call for unresolved gaps and sequence Mermaid refresh.
pub async fn run_ai_refine(
    gcx: Arc<ARwLock<GlobalContext>>,
    chat_id: &str,
    project_path: &Path,
    facts: &AnalysisFacts,
    mut pir: PirDocument,
    triggered_by: &str,
    previous: Option<&PirDocument>,
) -> Result<PirAnalyzeResult, String> {
    let manifest = super::analyzer::manifest::build_manifest(project_path)?;
    let facts_json = serde_json::to_string_pretty(facts).unwrap_or_else(|_| "{}".to_string());
    let pir_summary = context::compact_pir_summary(&pir);
    let snippets =
        context::build_unresolved_snippets(project_path, &facts.unresolved, &manifest.contents);
    let unresolved_json =
        serde_json::to_string_pretty(&facts.unresolved).unwrap_or_else(|_| "[]".to_string());

    let board_context: Option<String> = None;
    let board_id: Option<String> = None;

    let user_message = format!(
        "Project path: {}\nRevision: {}\n\nBoard context:\n{}\n\nRust AnalysisFacts:\n{}\n\nBaseline PIR summary:\n{}\n\nSequence Mermaid required: true\n\nUnresolved gaps:\n{}\n\nTargeted snippets:\n{}",
        project_path.display(),
        pir.revision,
        board_context.as_deref().unwrap_or("(none)"),
        facts_json,
        pir_summary,
        unresolved_json,
        snippets
    );

    let llm_result = run_llm_subchat(
        gcx,
        chat_id,
        prompt::PIR_REFINE_SYSTEM_PROMPT,
        &user_message,
        PIR_REFINE_N_CTX,
        "ai_refine",
    )
    .await?;

    let patch = parser::extract_json_object_public(&llm_result.text)?;
    merge::merge_refine_patch(&mut pir, &patch);

    pir.generation = PirGenerationMeta {
        mode: "hybrid_refine".to_string(),
        model: Some(llm_result.model_id.clone()),
        triggered_by: triggered_by.to_string(),
        analyzed_at_ms: now_ms(),
        input_files: facts.analyzed_files.clone(),
        token_usage: llm_result.token_usage.clone(),
    };

    apply_board_validation(&mut pir, board_context.as_deref(), board_id.as_deref());

    for node in &mut pir.nodes {
        node.sync.last_synced_revision = pir.revision.clone();
    }

    let diff = previous.map(|p| super::builder::diff_documents(p, &pir));
    let (pir, graph, validation, diff) = finalize_pir_result(pir, project_path, previous, diff);
    // #region agent log
    debug_mode_log(
        "critical-bugs",
        "H2",
        "agent/mod.rs:run_ai_refine:return",
        "returning refine result from agent layer",
        serde_json::json!({
            "direct_save_in_agent_layer": false,
            "generation_mode": pir.generation.mode.clone(),
            "revision": pir.revision.clone(),
            "nodes": pir.nodes.len(),
            "edges": pir.edges.len(),
            "validation_valid": validation.valid,
        }),
    );
    // #endregion

    Ok(PirAnalyzeResult {
        status: "ready".to_string(),
        pir,
        graph,
        validation,
        diff,
    })
}

async fn run_llm_subchat(
    gcx: Arc<ARwLock<GlobalContext>>,
    chat_id: &str,
    system_prompt: &str,
    user_message: &str,
    n_ctx: usize,
    stage: &str,
) -> Result<PirLlmSubchatResult, String> {
    let caps = try_load_caps_quickly_if_not_present(gcx.clone(), 3600)
        .await
        .map_err(|e| e.message)?;
    let model_id = resolve_pir_model_id(&caps);

    let messages = vec![
        ChatMessage {
            role: "system".to_string(),
            content: ChatContent::SimpleText(system_prompt.to_string()),
            ..Default::default()
        },
        ChatMessage {
            role: "user".to_string(),
            content: ChatContent::SimpleText(user_message.to_string()),
            ..Default::default()
        },
    ];

    let ccx = Arc::new(tokio::sync::Mutex::new(
        AtCommandsContext::new(
            gcx.clone(),
            n_ctx,
            1,
            false,
            messages.clone(),
            chat_id.to_string(),
            false,
            model_id.clone(),
        )
        .await,
    ));

    let mut usage_collector = ChatUsage::default();
    let result = subchat_single(
        ccx,
        &model_id,
        messages,
        Some(vec![]),
        None,
        false,
        Some(PIR_AGENT_TEMPERATURE),
        None,
        1,
        None,
        true,
        Some(&mut usage_collector),
        None,
        None,
    )
    .await
    .map_err(|e| format!("PIR sub-agent error: {}", e))?;

    let llm_text = result
        .into_iter()
        .next()
        .and_then(|x| x.into_iter().last())
        .and_then(|last_m| match last_m.content {
            ChatContent::SimpleText(t) => Some(t),
            ChatContent::Multimodal(_) => None,
        })
        .ok_or_else(|| "PIR sub-agent returned empty output".to_string())?;

    append_pir_token_log(
        gcx,
        chat_id,
        stage,
        &model_id,
        n_ctx,
        user_message.len(),
        llm_text.len(),
        &usage_collector,
    )
    .await;

    Ok(PirLlmSubchatResult {
        text: llm_text,
        model_id,
        token_usage: token_usage_from_chat_usage(&usage_collector),
    })
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
