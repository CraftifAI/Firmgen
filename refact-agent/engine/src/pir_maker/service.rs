//! PIR_maker orchestration — analyze, patch, approve.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::io::Write;

use tokio::sync::RwLock as ARwLock;

use crate::global_context::GlobalContext;

use super::agent;
use super::analyzer;
use super::apply_patch;
use super::builder;
use super::persistence;
use super::schema::{AnalysisFacts, PirAnalyzeResult, PirDocument};
use super::session::{self, PirSessionStatus};

const DEBUG_LOG_PATH: &str = "debug-10e772.log";

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
        "timestamp": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0),
    });
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(DEBUG_LOG_PATH)
    {
        let _ = writeln!(file, "{}", payload);
    }
}

fn has_valid_sequence_diagram(pir: &PirDocument) -> bool {
    let Some(sequence) = pir.diagrams.as_ref().and_then(|d| d.sequence.as_ref()) else {
        return false;
    };
    let Some(mermaid) = sequence
        .mermaid
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    else {
        return false;
    };
    if sequence
        .generation_error
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .is_some()
    {
        return false;
    }
    super::diagrams::validate_sequence_mermaid(mermaid).is_ok()
}

fn sequence_generation_pending(pir: &PirDocument) -> bool {
    !has_valid_sequence_diagram(pir)
}

fn persist_result_artifacts(path: &Path, result: &PirAnalyzeResult) {
    let save_pir = persistence::save_pir(path, &result.pir);
    let save_history = persistence::save_history_snapshot(path, &result.pir);
    let save_graph = persistence::save_graph_views(path, result);
    // #region agent log
    debug_mode_log(
        "critical-bugs",
        "H2",
        "service.rs:persist_result_artifacts",
        "persisted PIR artifacts after analyze",
        serde_json::json!({
            "generation_mode": result.pir.generation.mode.as_str(),
            "revision": result.pir.revision.as_str(),
            "save_pir_ok": save_pir.is_ok(),
            "save_history_snapshot_ok": save_history.is_ok(),
            "save_graph_views_ok": save_graph.is_ok(),
            "save_pir_err": save_pir.as_ref().err().map(|e| e.to_string()),
            "save_history_snapshot_err": save_history.as_ref().err().map(|e| e.to_string()),
            "save_graph_views_err": save_graph.as_ref().err().map(|e| e.to_string()),
            "path": path.display().to_string(),
        }),
    );
    // #endregion
}

pub async fn analyze_for_chat(
    gcx: Arc<ARwLock<GlobalContext>>,
    chat_id: &str,
    project_path: Option<&str>,
    incremental: bool,
    triggered_by: &str,
    chat_context: Option<&str>,
) -> Result<PirAnalyzeResult, String> {
    let t0 = std::time::Instant::now();
    let path = resolve_project_path(chat_id, project_path, gcx.clone()).await?;
    let session_snapshot = session::get(chat_id).await;

    if matches!(triggered_by, "agent_idle" | "agent_turn") && !project_has_codegen_artifacts(&path)
    {
        if let Some(session) = session_snapshot.as_ref() {
            if let Some(result) = session.last_result.clone() {
                return Ok(result);
            }
        }
        return Err(
            "PIR analyze deferred: main/app_config.h or main sources not found yet".to_string(),
        );
    }

    session::set_analyzing(chat_id, &path.to_string_lossy()).await;
    let mut previous_pir = if incremental {
        session_snapshot
            .as_ref()
            .and_then(|s| s.last_result.as_ref().map(|r| r.pir.clone()))
    } else {
        None
    };

    let mut previous_facts = if incremental {
        session_snapshot.as_ref().and_then(|s| s.last_facts.clone())
    } else {
        None
    };

    if previous_pir.is_none() {
        if let Some(disk) = persistence::load_pir(&path) {
            previous_pir = Some(disk);
        }
    }

    let (result, cached_facts) = if agent::is_legacy_ai_mode() {
        let r = agent::run_ai_analyze(
            gcx.clone(),
            chat_id,
            &path,
            incremental,
            triggered_by,
            previous_pir,
        )
        .await?;
        (r, None)
    } else {
        run_hybrid_analyze(
            gcx,
            chat_id,
            &path,
            incremental,
            triggered_by,
            previous_pir,
            previous_facts.as_ref(),
            agent::is_hybrid_mode() && agent::should_run_ai_refine(),
            chat_context,
        )
        .await?
    };

    session::set_ready(chat_id, result.clone(), cached_facts).await;
    persist_result_artifacts(&path, &result);
    Ok(result)
}

async fn run_hybrid_analyze(
    gcx: Arc<ARwLock<GlobalContext>>,
    chat_id: &str,
    path: &Path,
    incremental: bool,
    triggered_by: &str,
    previous_pir: Option<PirDocument>,
    previous_facts: Option<&AnalysisFacts>,
    ai_refine: bool,
    chat_context: Option<&str>,
) -> Result<(PirAnalyzeResult, Option<AnalysisFacts>), String> {
    let facts = analyzer::analyze_project(path, previous_facts, chat_context)?;

    let chat_driven = chat_context
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .is_some();
    // #region agent log
    debug_mode_log(
        "ai-facts-test",
        "H1",
        "service.rs:run_hybrid_analyze:mode_check",
        "evaluated mode branch for PIR analyze",
        serde_json::json!({
            "agent_mode": agent::agent_mode(),
            "is_ai_facts_mode": agent::is_ai_facts_mode(),
            "incremental": incremental,
            "chat_driven": chat_driven,
            "ai_refine": ai_refine,
            "facts_unresolved": facts.unresolved.len(),
            "facts_gpio": facts.gpio_facts.len(),
            "facts_tasks": facts.task_facts.len(),
            "facts_network": facts.network_facts.len(),
        }),
    );
    // #endregion

    if incremental && facts.unresolved.is_empty() && !chat_driven {
        if let Some(prev) = &previous_pir {
            let unchanged = prev.provenance.file_hashes == facts.file_hashes;
            let can_reuse_without_refine = !ai_refine || has_valid_sequence_diagram(prev);
            if unchanged && can_reuse_without_refine {
                let graph = builder::graph_from_pir(prev, &facts.project_name);
                let validation = crate::firmware_topology::validate_graph(&graph);
                return Ok((
                    PirAnalyzeResult {
                        status: "ready".to_string(),
                        pir: prev.clone(),
                        graph,
                        validation,
                        diff: None,
                    },
                    Some(facts),
                ));
            }
        }
    }

    if agent::is_ai_facts_mode() {
        // #region agent log
        debug_mode_log(
            "ai-facts-test",
            "H1",
            "service.rs:run_hybrid_analyze:ai_facts_path",
            "selected ai_facts execution path",
            serde_json::json!({
                "triggered_by": triggered_by,
                "incremental": incremental,
                "project_name": facts.project_name,
            }),
        );
        // #endregion
        let result = agent::run_ai_build_from_facts(
            gcx,
            chat_id,
            path,
            incremental,
            &facts,
            triggered_by,
            previous_pir.as_ref(),
        )
        .await?;
        return Ok((result, Some(facts)));
    }

    let revision = builder::compute_revision(&facts);
    let (mut pir, _graph, _validation, diff) = builder::build_pir_and_graph(
        &facts,
        path,
        Some(chat_id),
        &revision,
        previous_pir.as_ref(),
    );

    for node in &mut pir.nodes {
        node.sync.last_synced_revision = revision.clone();
    }

    pir.unresolved = facts.unresolved.clone();
    pir.generation.mode = "static".to_string();
    pir.generation.triggered_by = triggered_by.to_string();
    pir.generation.analyzed_at_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    pir.generation.input_files = facts.analyzed_files.clone();

    let (pir, graph, validation, diff) =
        builder::finalize_pir_result(pir, path, previous_pir.as_ref(), diff);

    let mut result = PirAnalyzeResult {
        status: "ready".to_string(),
        pir,
        graph,
        validation,
        diff,
    };

    let needs_refine = ai_refine
        && triggered_by != "patch"
        && (!facts.unresolved.is_empty() || sequence_generation_pending(&result.pir));

    if needs_refine {
        result = agent::run_ai_refine(
            gcx,
            chat_id,
            path,
            &facts,
            result.pir,
            triggered_by,
            previous_pir.as_ref(),
        )
        .await?;
    }

    Ok((result, Some(facts)))
}

pub async fn spawn_analyze_for_chat(
    gcx: Arc<ARwLock<GlobalContext>>,
    chat_id: &str,
    project_path: Option<&str>,
    incremental: bool,
    triggered_by: &str,
    chat_context: Option<String>,
) {
    let force = matches!(triggered_by, "user_refresh" | "user_full" | "mount");
    if !force {
        if let Some(s) = session::get(chat_id).await {
            if s.status == PirSessionStatus::Analyzing {
                return;
            }
        }
    }

    let chat_id = chat_id.to_string();
    let project_path = project_path.map(String::from);
    let triggered_by = triggered_by.to_string();
    let chat_context = chat_context.filter(|s| !s.trim().is_empty());
    tokio::spawn(async move {
        match analyze_for_chat(
            gcx,
            &chat_id,
            project_path.as_deref(),
            incremental,
            &triggered_by,
            chat_context.as_deref(),
        )
        .await
        {
            Ok(_) => {}
            Err(e) => {
                session::set_error(&chat_id, e).await;
            }
        }
    });
}

pub async fn apply_node_patch(
    _gcx: Arc<ARwLock<GlobalContext>>,
    chat_id: &str,
    node_id: &str,
    property_updates: serde_json::Map<String, serde_json::Value>,
    expected_revision: Option<&str>,
) -> Result<PirAnalyzeResult, String> {
    let session = session::get(chat_id).await.ok_or("no PIR session")?;
    let project_path = session
        .project_path
        .as_ref()
        .ok_or("no project_path in session")?;
    let path = PathBuf::from(project_path);

    let mut prior = session
        .last_result
        .as_ref()
        .ok_or("no analyzed document")?
        .clone();

    let mut pir = prior.pir.clone();

    let patch = apply_patch::apply_node_property_patch(
        &path,
        &mut pir,
        node_id,
        &property_updates,
        expected_revision,
    )?;

    if let Some(node) = pir.nodes.iter_mut().find(|n| n.id == node_id) {
        node.authority = super::schema::NodeAuthority::User;
        node.sync.state = super::schema::PirSyncState::Manual;
    }

    merge_properties_into_graph(&mut prior.graph, node_id, &property_updates);
    prior.pir = pir;

    // Always bump revision so the UI applies in-memory graph/property updates even when
    // no source file matched (e.g. macro name drift); file writes still tracked separately.
    prior.pir.revision = format!(
        "{}-p{}",
        prior.pir.revision, patch.change_entry.timestamp_ms
    );

    let pir_to_save = prior.pir.clone();
    let facts = session.last_facts.clone();
    session::set_ready(chat_id, prior.clone(), facts).await;
    let _ = persistence::save_pir(&path, &pir_to_save);
    let _ = persistence::save_graph_views(&path, &prior);
    Ok(prior)
}

pub async fn apply_structural_patch(
    gcx: Arc<ARwLock<GlobalContext>>,
    chat_id: &str,
    body: apply_patch::StructuralPatchRequest,
    expected_revision: Option<&str>,
) -> Result<PirAnalyzeResult, String> {
    let session = session::get(chat_id).await.ok_or("no PIR session")?;
    let project_path = session
        .project_path
        .as_ref()
        .ok_or("no project_path in session")?;
    let path = PathBuf::from(project_path);

    let mut prior = session
        .last_result
        .as_ref()
        .ok_or("no analyzed document")?
        .clone();

    apply_patch::apply_structural_patch(&path, &mut prior.pir, &body, expected_revision)?;

    session::set_analyzing(chat_id, project_path).await;
    analyze_for_chat(
        gcx,
        chat_id,
        Some(project_path.as_str()),
        true,
        "structural_patch",
        None,
    )
    .await
}

fn merge_properties_into_graph(
    graph: &mut crate::firmware_topology::FirmwareGraph,
    node_id: &str,
    updates: &serde_json::Map<String, serde_json::Value>,
) {
    if let Some(node) = graph.nodes.iter_mut().find(|n| n.id == node_id) {
        let mut map = node.properties.as_object().cloned().unwrap_or_default();
        for (k, v) in updates {
            map.insert(k.clone(), v.clone());
        }
        node.properties = serde_json::Value::Object(map);
    }
}

fn agent_topology_rules_block() -> Vec<String> {
    vec![
        "PIR_maker — LIVE FIRMWARE TOPOLOGY (Firmware topology block pinned under the codegen assistant turn in chat).".to_string(),
        "CRITICAL RULES FOR THE MAIN AGENT:".to_string(),
        "- Do NOT ask the user for SSID, WiFi password, GPIO pin numbers, task periods, debounce seconds, or similar tunables in chat.".to_string(),
        "- Point the user to the PIR_maker diagram and node inspector in the chat panel for all configuration values.".to_string(),
        "- Store all GPIO pins, WiFi/MQTT credentials, thresholds, and task intervals in `main/app_config.h` using `APP_*` #define macros — PIR_maker reads this file as the primary topology manifest.".to_string(),
        "- You implement firmware features in source code (new sensors, tasks, LED logic); PIR_maker re-analyzes `app_config.h` and updates the diagram after your turn ends.".to_string(),
        "- After code changes (e.g. add IR sensor triggering LED), the diagram gains matching nodes/edges on the next analyze pass.".to_string(),
        "- After codegen completes, send a SEPARATE assistant message for topology review before offering build — do not call esp32_build in the codegen turn.".to_string(),
        "- Tell the user to review the topology block under the codegen turn and click Approve on the card when it matches their intent.".to_string(),
        "- Do NOT call esp32_build (operation build) until the user confirms via [Build project] or explicit approval text.".to_string(),
        "- Do NOT call esp32_device flash/monitor until the user confirms via the offered action buttons.".to_string(),
        "- When approval is pending or stale, remind the user to approve the topology card before build.".to_string(),
    ]
}

/// Compact PIR context block for the main coding agent system prompt.
pub async fn agent_context_for_chat(chat_id: &str) -> Option<String> {
    let mut lines = agent_topology_rules_block();

    if let Some(session) = session::get(chat_id).await {
        if let Some(result) = session.last_result.as_ref() {
            let pir = &result.pir;
            lines.push(compact_summary_for_agent(result));
            lines.push(format!("Project path: {}", pir.provenance.project_path));
            lines.push(format!("Revision: {}", pir.revision));
            append_node_lines(&mut lines, result);
            return Some(lines.join("\n"));
        }
    }

    if let Some(path) = crate::progressbar::esp32_project_path_for_chat(chat_id).await {
        lines.push(format!(
            "Project path: {} (topology analyze in progress — user configures via diagram).",
            path.display()
        ));
        return Some(lines.join("\n"));
    }

    None
}

fn append_node_lines(lines: &mut Vec<String>, result: &PirAnalyzeResult) {
    let pir = &result.pir;
    for node in pir.nodes.iter().take(20) {
        let files = if node.ownership.primary_files.is_empty() {
            "?".to_string()
        } else {
            node.ownership.primary_files.join(",")
        };
        let props_display = format_node_props_for_agent(&node.node_type, &node.properties);
        lines.push(format!(
            "- node `{}` type={} layer={:?} files={} conf={:.2} {}",
            node.id, node.node_type, node.layer, files, node.confidence, props_display
        ));
    }
    if pir.nodes.len() > 20 {
        lines.push(format!("... and {} more nodes", pir.nodes.len() - 20));
    }
    if !result.validation.valid {
        lines.push(format!(
            "Validation: {} issue(s) — review topology before build.",
            result.validation.issues.len()
        ));
    }
}

pub async fn approve(chat_id: &str, comment: Option<String>) -> Result<(), String> {
    let pir = session::approve(chat_id, comment).await?;
    if let Some(snapshot) = session::get(chat_id).await {
        if let Some(path) = snapshot.project_path.as_deref() {
            let root = std::path::Path::new(path);
            let _ = persistence::save_pir(root, &pir);
            if let Some(result) = snapshot.last_result.as_ref() {
                let _ = persistence::save_graph_views(root, result);
            }
        }
    }
    Ok(())
}

async fn resolve_project_path(
    chat_id: &str,
    explicit: Option<&str>,
    gcx: Arc<ARwLock<GlobalContext>>,
) -> Result<PathBuf, String> {
    // 1. Caller-provided explicit path.
    if let Some(p) = explicit {
        let pb = PathBuf::from(p);
        if pb.is_dir() {
            return Ok(pb);
        }
        return Err(format!("project_path is not a directory: {}", p));
    }
    // 2. Path recorded via esp32_* tool calls (new-project and validate flows).
    if let Some(p) = crate::progressbar::esp32_project_path_for_chat(chat_id).await {
        return Ok(p);
    }
    // 3. Workspace-root fallback for existing-project flows where the agent may
    //    not have called esp32_project validate yet.  Scan registered workspace
    //    folders: if exactly one directory within them looks like an ESP-IDF
    //    project (has CMakeLists.txt AND a main/ sub-directory) use it
    //    unambiguously.  A parent-folder workspace with a single ESP-IDF child is
    //    also handled.
    let workspace_folders: Vec<PathBuf> = {
        let cx = gcx.read().await;
        cx.documents_state
            .workspace_folders
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or_default()
    };
    let mut espidf_candidates: Vec<PathBuf> = Vec::new();
    for folder in &workspace_folders {
        if folder.join("CMakeLists.txt").is_file() && folder.join("main").is_dir() {
            espidf_candidates.push(folder.clone());
        } else if let Ok(entries) = std::fs::read_dir(folder) {
            for entry in entries.flatten() {
                if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    continue;
                }
                let child = entry.path();
                if child.join("CMakeLists.txt").is_file() && child.join("main").is_dir() {
                    espidf_candidates.push(child);
                }
            }
        }
    }
    if espidf_candidates.len() == 1 {
        return Ok(espidf_candidates.remove(0));
    }
    Err("no ESP-IDF project_path for this chat; create a project or pass project_path".to_string())
}

fn approval_status_str(s: &super::schema::PirApprovalStatus) -> &'static str {
    match s {
        super::schema::PirApprovalStatus::Pending => "pending",
        super::schema::PirApprovalStatus::Approved => "approved",
        super::schema::PirApprovalStatus::Rejected => "rejected",
        super::schema::PirApprovalStatus::Stale => "stale",
    }
}

fn format_node_props_for_agent(node_type: &str, properties: &serde_json::Value) -> String {
    let obj = properties.as_object();
    match node_type {
        "wifi_manager" => {
            let ssid = obj
                .and_then(|o| o.get("ssid"))
                .and_then(|v| v.as_str())
                .unwrap_or("(not set)");
            let mode = obj
                .and_then(|o| o.get("mode"))
                .and_then(|v| v.as_str())
                .unwrap_or("station");
            let has_pass = obj
                .and_then(|o| o.get("password"))
                .map(|v| !v.is_null() && v.as_str().map(|s| !s.is_empty()).unwrap_or(true))
                .unwrap_or(false);
            format!(
                "ssid=\"{}\" mode=\"{}\" password={}",
                ssid,
                mode,
                if has_pass { "set" } else { "not set" }
            )
        }
        "mqtt_client" => {
            let broker = obj
                .and_then(|o| o.get("broker_url"))
                .and_then(|v| v.as_str())
                .unwrap_or("(not set)");
            let topic = obj
                .and_then(|o| o.get("topic"))
                .and_then(|v| v.as_str())
                .unwrap_or("(not set)");
            format!("broker_url=\"{}\" topic=\"{}\"", broker, topic)
        }
        _ => properties.to_string(),
    }
}

pub fn project_has_codegen_artifacts(project_root: &Path) -> bool {
    if project_root.join("main/app_config.h").is_file() {
        return true;
    }
    super::analyzer::manifest::main_dir_has_sources(project_root)
}

pub fn compact_summary_for_agent(result: &PirAnalyzeResult) -> String {
    let s = result.pir.summary.as_ref();
    format!(
        "PIR_maker [{}] {} — {} nodes, {} edges, approval={:?}, valid={}",
        result.pir.revision,
        s.map(|x| x.headline.as_str()).unwrap_or("topology ready"),
        result.pir.nodes.len(),
        result.pir.edges.len(),
        approval_status_str(&result.pir.approval.status),
        result.validation.valid
    )
}
