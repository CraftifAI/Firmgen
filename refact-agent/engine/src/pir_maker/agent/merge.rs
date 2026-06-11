//! Merge AI-generated PIR with prior state, preserving user-owned nodes.

use std::collections::{HashMap, HashSet};
use std::io::Write;

use super::super::schema::{NodeAuthority, PirDocument, PirNode, TopologyDiff};

const DEBUG_LOG_PATH: &str = "debug-10e772.log";

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
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

fn node_signature(n: &PirNode) -> String {
    let file = n
        .ownership
        .primary_files
        .first()
        .map(|s| s.as_str())
        .unwrap_or("");
    format!("{}|{}", n.node_type, file)
}

/// Reuse stable node ids from the previous revision when type + primary file match.
fn stabilize_node_ids(agent_doc: &mut PirDocument, prev: &PirDocument) {
    let mut prev_by_sig: HashMap<String, String> = HashMap::new();
    for n in &prev.nodes {
        prev_by_sig.insert(node_signature(n), n.id.clone());
    }

    let mut id_remap: HashMap<String, String> = HashMap::new();
    for node in &agent_doc.nodes {
        let sig = node_signature(node);
        if let Some(prev_id) = prev_by_sig.get(&sig) {
            if prev_id != &node.id {
                id_remap.insert(node.id.clone(), prev_id.clone());
            }
        }
    }

    if id_remap.is_empty() {
        return;
    }

    for node in &mut agent_doc.nodes {
        if let Some(new_id) = id_remap.get(&node.id) {
            node.id = new_id.clone();
        }
    }

    let remap_edge = |node_id: &str| -> String {
        id_remap
            .get(node_id)
            .cloned()
            .unwrap_or_else(|| node_id.to_string())
    };

    for edge in &mut agent_doc.edges {
        edge.source_node_id = remap_edge(&edge.source_node_id);
        edge.target_node_id = remap_edge(&edge.target_node_id);
    }

    let remap_id =
        |id: &str| -> String { id_remap.get(id).cloned().unwrap_or_else(|| id.to_string()) };
    agent_doc.layers.physical = agent_doc
        .layers
        .physical
        .iter()
        .map(|id| remap_id(id))
        .collect();
    agent_doc.layers.runtime = agent_doc
        .layers
        .runtime
        .iter()
        .map(|id| remap_id(id))
        .collect();
    agent_doc.layers.network = agent_doc
        .layers
        .network
        .iter()
        .map(|id| remap_id(id))
        .collect();
    agent_doc.layers.system = agent_doc
        .layers
        .system
        .iter()
        .map(|id| remap_id(id))
        .collect();
}

pub fn merge_with_previous(
    agent_doc: &mut PirDocument,
    previous: Option<&PirDocument>,
) -> Option<TopologyDiff> {
    let Some(prev) = previous else {
        return None;
    };

    stabilize_node_ids(agent_doc, prev);

    let user_nodes: Vec<PirNode> = prev
        .nodes
        .iter()
        .filter(|n| matches!(n.authority, NodeAuthority::User | NodeAuthority::Hybrid))
        .cloned()
        .collect();

    for user_node in user_nodes {
        if let Some(existing) = agent_doc.nodes.iter_mut().find(|n| n.id == user_node.id) {
            if matches!(user_node.authority, NodeAuthority::User) {
                *existing = user_node.clone();
            } else {
                existing.properties = user_node.properties.clone();
                existing.editable_fields = user_node.editable_fields.clone();
                existing.authority = NodeAuthority::Hybrid;
            }
        } else if let Some(existing) = agent_doc
            .nodes
            .iter_mut()
            .find(|n| node_signature(n) == node_signature(&user_node))
        {
            if matches!(user_node.authority, NodeAuthority::User) {
                let stable_id = existing.id.clone();
                let mut restored = user_node.clone();
                restored.id = stable_id;
                *existing = restored;
            }
        } else {
            agent_doc.nodes.push(user_node);
        }
    }

    agent_doc.change_log = prev.change_log.clone();
    if prev.approval.status == super::super::schema::PirApprovalStatus::Approved
        && prev.revision == agent_doc.revision
    {
        agent_doc.approval = prev.approval.clone();
    }

    Some(super::super::builder::diff_documents(prev, agent_doc))
}

fn normalize_rel_path(path: &str) -> String {
    path.trim()
        .replace('\\', "/")
        .trim_start_matches("./")
        .to_string()
}

fn analyzed_file_set(pir: &PirDocument) -> HashSet<String> {
    pir.provenance
        .analyzed_files
        .iter()
        .map(|p| normalize_rel_path(p))
        .filter(|p| !p.is_empty())
        .collect()
}

fn source_refs_match_analyzed(
    refs: &[super::super::schema::SourceRef],
    analyzed_files: &HashSet<String>,
) -> bool {
    if refs.is_empty() || analyzed_files.is_empty() {
        return !refs.is_empty();
    }
    refs.iter()
        .any(|r| analyzed_files.contains(&normalize_rel_path(&r.file)))
}

fn node_has_file_evidence(node: &PirNode, analyzed_files: &HashSet<String>) -> bool {
    source_refs_match_analyzed(&node.source_refs, analyzed_files)
        || node
            .ownership
            .primary_files
            .iter()
            .map(|p| normalize_rel_path(p))
            .any(|p| analyzed_files.contains(&p))
}

/// Merge an AI refine patch (additive nodes/edges) into an existing static-built PIR.
pub fn merge_refine_patch(pir: &mut PirDocument, patch: &serde_json::Value) {
    use super::parser::{
        parse_diagrams_from_value, parse_edges_from_value, parse_layers, parse_nodes_from_value,
    };

    let analyzed_files = analyzed_file_set(pir);
    let mut rejection_notes: Vec<serde_json::Value> = Vec::new();
    let mut updated_existing_nodes = 0usize;
    let mut accepted_new_nodes = 0usize;
    let mut rejected_nodes_no_evidence = 0usize;
    let mut rejected_edges_missing_nodes = 0usize;
    let mut rejected_edges_outside_files = 0usize;
    let mut accepted_new_edges = 0usize;

    match parse_nodes_from_value(patch.get("nodes")) {
        Ok(new_nodes) => {
            for node in new_nodes {
                if let Some(existing) = pir.nodes.iter_mut().find(|n| n.id == node.id) {
                    if node.properties != serde_json::json!({}) {
                        existing.properties = node.properties.clone();
                    }
                    if !node.source_refs.is_empty() {
                        if source_refs_match_analyzed(&node.source_refs, &analyzed_files) {
                            existing.source_refs = node.source_refs.clone();
                        } else {
                            rejection_notes.push(serde_json::json!({
                                "kind": "llm_node_rejected",
                                "message": format!("Rejected source_refs update for node {} because files are not in analyzed set", node.id),
                                "node_id": node.id,
                                "strict_evidence": true,
                            }));
                        }
                    }
                    existing.confidence = node.confidence.max(existing.confidence);
                    updated_existing_nodes += 1;
                } else {
                    if !node_has_file_evidence(&node, &analyzed_files) {
                        rejection_notes.push(serde_json::json!({
                            "kind": "llm_node_rejected",
                            "message": format!("Rejected refine node {} ({}) because it has no file-backed evidence", node.id, node.node_type),
                            "node_id": node.id,
                            "node_type": node.node_type,
                            "strict_evidence": true,
                        }));
                        rejected_nodes_no_evidence += 1;
                        continue;
                    }
                    pir.nodes.push(node);
                    accepted_new_nodes += 1;
                }
            }
        }
        Err(err) => {
            rejection_notes.push(serde_json::json!({
                "kind": "llm_node_rejected",
                "message": format!("Rejected refine node patch: {}", err),
                "strict_evidence": true,
            }));
        }
    }

    let valid_node_ids: HashSet<String> = pir.nodes.iter().map(|n| n.id.clone()).collect();
    match parse_edges_from_value(patch.get("edges")) {
        Ok(new_edges) => {
            for edge in new_edges {
                if !valid_node_ids.contains(&edge.source_node_id)
                    || !valid_node_ids.contains(&edge.target_node_id)
                {
                    rejection_notes.push(serde_json::json!({
                        "kind": "llm_edge_rejected",
                        "message": format!("Rejected refine edge {} because source/target nodes are not evidence-backed", edge.id),
                        "edge_id": edge.id,
                        "strict_evidence": true,
                    }));
                    rejected_edges_missing_nodes += 1;
                    continue;
                }
                if !edge.source_refs.is_empty()
                    && !source_refs_match_analyzed(&edge.source_refs, &analyzed_files)
                {
                    rejection_notes.push(serde_json::json!({
                        "kind": "llm_edge_rejected",
                        "message": format!("Rejected refine edge {} because source_refs are outside analyzed files", edge.id),
                        "edge_id": edge.id,
                        "strict_evidence": true,
                    }));
                    rejected_edges_outside_files += 1;
                    continue;
                }
                if !pir.edges.iter().any(|e| e.id == edge.id) {
                    pir.edges.push(edge);
                    accepted_new_edges += 1;
                }
            }
        }
        Err(err) => {
            rejection_notes.push(serde_json::json!({
                "kind": "llm_edge_rejected",
                "message": format!("Rejected refine edge patch: {}", err),
                "strict_evidence": true,
            }));
        }
    }

    if let Some(layers) = patch.get("layers") {
        let parsed = parse_layers(layers);
        for id in parsed.physical {
            if valid_node_ids.contains(&id) && !pir.layers.physical.contains(&id) {
                pir.layers.physical.push(id);
            }
        }
        for id in parsed.runtime {
            if valid_node_ids.contains(&id) && !pir.layers.runtime.contains(&id) {
                pir.layers.runtime.push(id);
            }
        }
        for id in parsed.network {
            if valid_node_ids.contains(&id) && !pir.layers.network.contains(&id) {
                pir.layers.network.push(id);
            }
        }
        for id in parsed.system {
            if valid_node_ids.contains(&id) && !pir.layers.system.contains(&id) {
                pir.layers.system.push(id);
            }
        }
    }

    if let Some(summary) = patch.get("summary") {
        if let Ok(s) = serde_json::from_value::<super::super::schema::PirSummary>(summary.clone()) {
            pir.summary = Some(s);
        }
    }

    if let Some(diagrams) = patch.get("diagrams") {
        if let Some(d) = parse_diagrams_from_value(Some(diagrams)) {
            let existing = pir.diagrams.get_or_insert_with(Default::default);
            if d.hld.is_some() {
                existing.hld = d.hld;
            }
            if d.lld.is_some() {
                existing.lld = d.lld;
            }
            if d.sequence.is_some() {
                existing.sequence = d.sequence;
            }
            if d.hld_graph.is_some() {
                existing.hld_graph = d.hld_graph;
            }
            if d.lld_graph.is_some() {
                existing.lld_graph = d.lld_graph;
            }
            if d.sequence_graph.is_some() {
                existing.sequence_graph = d.sequence_graph;
            }
        }
    }

    if let Some(remaining) = patch.get("unresolved").and_then(|v| v.as_array()) {
        pir.unresolved = remaining.clone();
    }
    pir.unresolved.extend(rejection_notes);
    // #region agent log
    debug_mode_log(
        "critical-bugs-3",
        "H4",
        "merge.rs:merge_refine_patch",
        "merged refine patch with evidence gating",
        serde_json::json!({
            "analyzed_files_count": analyzed_files.len(),
            "updated_existing_nodes": updated_existing_nodes,
            "accepted_new_nodes": accepted_new_nodes,
            "rejected_nodes_no_evidence": rejected_nodes_no_evidence,
            "accepted_new_edges": accepted_new_edges,
            "rejected_edges_missing_nodes": rejected_edges_missing_nodes,
            "rejected_edges_outside_files": rejected_edges_outside_files,
            "unresolved_count_after_merge": pir.unresolved.len(),
        }),
    );
    // #endregion
}
