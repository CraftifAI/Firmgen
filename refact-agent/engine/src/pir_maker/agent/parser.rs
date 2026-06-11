//! Parse LLM JSON output into a PIR document shell.

use serde_json::Value as JsonValue;
use std::collections::{HashMap, HashSet};
use std::io::Write;

use crate::firmware_topology::registry::{default_editable_for_type, get_node_type_def};

use super::super::schema::{
    NodeAuthority, PirApproval, PirDiagramMermaid, PirDiagrams, PirDocument, PirEdge, PirEdgeKind,
    PirGenerationMeta, PirSequenceDiagram, PirLayers, PirNode, PirNodeSync, PirProvenance,
    PirSummary, PirSyncMetadata, PirSyncState, PirValidationState, PIR_SCHEMA_VERSION,
};

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

pub fn parse_agent_output(
    raw: &str,
    project_path: &str,
    chat_id: Option<&str>,
    revision: &str,
    board_id: Option<String>,
    analyzed_files: Vec<String>,
    file_hashes: std::collections::HashMap<String, String>,
    generation: PirGenerationMeta,
    previous: Option<&PirDocument>,
) -> Result<PirDocument, String> {
    let json = extract_json_object(raw)?;
    let analyzed_file_set = normalized_file_set(&analyzed_files);
    let parsed_nodes = parse_nodes_from_value(json.get("nodes"))?;
    let (canonical_nodes, node_aliases, mut rejection_notes) =
        canonicalize_component_identity_nodes(parsed_nodes);
    let (nodes, node_evidence_rejections) =
        filter_nodes_by_evidence(canonical_nodes, &analyzed_file_set);
    rejection_notes.extend(node_evidence_rejections);
    let mut parsed_edges = parse_edges_from_value(json.get("edges"))?;
    remap_edges_with_node_aliases(&mut parsed_edges, &node_aliases);
    let (edges, edge_rejections) =
        filter_edges_by_evidence(parsed_edges, &nodes, &analyzed_file_set);
    rejection_notes.extend(edge_rejections);
    let layers = parse_layers(json.get("layers").unwrap_or(&serde_json::json!({})));
    let summary = parse_summary(&json, nodes.len(), edges.len());
    let diagrams = parse_diagrams(&json);
    let partitions = json
        .get("partitions")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();
    let components = json
        .get("components")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();
    let unresolved = json
        .get("unresolved")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let mut unresolved = unresolved;
    unresolved.extend(rejection_notes);

    let project_name = std::path::Path::new(project_path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("esp32_project");

    let approval = if let Some(prev) = previous {
        if prev.revision != revision {
            super::super::schema::PirApproval {
                status: super::super::schema::PirApprovalStatus::Stale,
                ..Default::default()
            }
        } else {
            prev.approval.clone()
        }
    } else {
        PirApproval::default()
    };

    let graph_version = previous.map(|p| p.graph_version + 1).unwrap_or(1);

    Ok(PirDocument {
        schema_version: PIR_SCHEMA_VERSION.to_string(),
        id: format!("pir_{}", project_name),
        revision: revision.to_string(),
        provenance: PirProvenance {
            project_path: project_path.to_string(),
            chat_id: chat_id.map(String::from),
            revision: revision.to_string(),
            generated_at_ms: now_ms(),
            analyzer_version: "pir-agent-2.0".to_string(),
            analyzed_files,
            file_hashes,
            board_id,
        },
        approval,
        nodes,
        edges,
        layers,
        partitions,
        components,
        summary: Some(summary),
        diagrams,
        change_log: previous.map(|p| p.change_log.clone()).unwrap_or_default(),
        unresolved,
        graph_version,
        generation,
        sync_metadata: PirSyncMetadata {
            project_file: ".craftif/pir.json".to_string(),
            content_hash: revision.to_string(),
            pending_patches: Vec::new(),
            last_diff: None,
        },
        validation_state: PirValidationState {
            valid: true,
            error_count: 0,
            warning_count: 0,
            validated_at_ms: now_ms(),
        },
    })
}

fn normalize_rel_path(path: &str) -> String {
    path.trim()
        .replace('\\', "/")
        .trim_start_matches("./")
        .to_string()
}

fn normalized_file_set(files: &[String]) -> HashSet<String> {
    files
        .iter()
        .map(|f| normalize_rel_path(f))
        .filter(|f| !f.is_empty())
        .collect()
}

fn matches_analyzed_file(path: &str, analyzed_files: &HashSet<String>) -> bool {
    if analyzed_files.is_empty() {
        return true;
    }
    let normalized = normalize_rel_path(path);
    if normalized.is_empty() {
        return false;
    }
    analyzed_files.contains(&normalized)
}

fn node_is_file_backed(node: &PirNode, analyzed_files: &HashSet<String>) -> bool {
    node.source_refs
        .iter()
        .any(|r| matches_analyzed_file(&r.file, analyzed_files))
        || node
            .ownership
            .primary_files
            .iter()
            .any(|f| matches_analyzed_file(f, analyzed_files))
}

fn is_transport_component_type(node_type: &str) -> bool {
    matches!(
        node_type,
        "spi_device"
            | "i2c_device"
            | "uart_device"
            | "display_output"
            | "camera_capture"
            | "adc_reader"
    )
}

fn is_transport_binding_name(name: &str) -> bool {
    let lower = name.trim().to_ascii_lowercase();
    if lower.is_empty() {
        return false;
    }
    let segments: Vec<&str> = lower
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|s| !s.is_empty())
        .collect();
    if segments.iter().any(|seg| {
        matches!(
            *seg,
            "sclk"
                | "mosi"
                | "miso"
                | "cs"
                | "dc"
                | "rst"
                | "reset"
                | "sda"
                | "scl"
                | "tx"
                | "rx"
                | "rts"
                | "cts"
                | "clk"
                | "clock"
                | "mclk"
                | "bclk"
                | "lrck"
                | "ws"
                | "din"
                | "dout"
        )
    }) {
        return true;
    }
    if lower.starts_with('d') && lower[1..].chars().all(|c| c.is_ascii_digit()) {
        return true;
    }
    lower.starts_with("adc") && (lower.contains("channel") || lower.contains("_ch"))
}

fn node_transport_bindings(node: &PirNode) -> Vec<(String, u8)> {
    let mut out = Vec::new();
    if let Some(bindings) = node
        .properties
        .get("pin_bindings")
        .and_then(|v| v.as_object())
    {
        for (name, value) in bindings {
            if let Some(pin) = value.as_u64().and_then(|v| u8::try_from(v).ok()) {
                out.push((name.to_ascii_lowercase(), pin));
            }
        }
    }
    if let Some(props) = node.properties.as_object() {
        for (key, value) in props {
            if !key.ends_with("_pin") {
                continue;
            }
            if let Some(pin) = value.as_u64().and_then(|v| u8::try_from(v).ok()) {
                out.push((key.trim_end_matches("_pin").to_ascii_lowercase(), pin));
            }
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    out.dedup();
    out
}

fn node_component_signature(node: &PirNode) -> Option<String> {
    if matches!(
        node.node_type.as_str(),
        "gpio_input" | "gpio_output" | "sensor_input"
    ) {
        return None;
    }
    let bindings = node_transport_bindings(node);
    if bindings.is_empty() {
        return None;
    }
    let mut fields = vec![format!("type={}", node.node_type.to_ascii_lowercase())];
    if let Some(label) = node
        .label
        .as_ref()
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty())
    {
        fields.push(format!("label={label}"));
    }
    fields.push(format!(
        "pins={}",
        bindings
            .into_iter()
            .map(|(name, pin)| format!("{name}:{pin}"))
            .collect::<Vec<_>>()
            .join(",")
    ));
    for key in ["interface", "host", "address", "port", "bus", "channel"] {
        if let Some(value) = node
            .properties
            .get(key)
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_ascii_lowercase())
            .filter(|s| !s.is_empty())
        {
            fields.push(format!("{key}={value}"));
        } else if let Some(value) = node.properties.get(key).and_then(|v| v.as_u64()) {
            fields.push(format!("{key}={value}"));
        }
    }
    Some(fields.join("|"))
}

fn node_component_specificity(node_type: &str) -> u8 {
    match node_type {
        "spi_device" => 100,
        "i2c_device" => 95,
        "uart_device" => 90,
        "display_output" => 88,
        "camera_capture" => 86,
        "adc_reader" => 84,
        "pwm_output" => 82,
        "sensor_input" => 80,
        "rtos_task" => 70,
        "timer" => 68,
        "gpio_output" => 40,
        "gpio_input" => 35,
        _ => 60,
    }
}

fn node_transport_pin_set(node: &PirNode) -> HashSet<u8> {
    node_transport_bindings(node)
        .into_iter()
        .map(|(_, pin)| pin)
        .collect()
}

fn overlapping_transport_pins(left: &PirNode, right: &PirNode) -> Vec<u8> {
    let left_pins = node_transport_pin_set(left);
    let right_pins = node_transport_pin_set(right);
    if left_pins.is_empty() || right_pins.is_empty() {
        return Vec::new();
    }
    let mut overlap: Vec<u8> = left_pins
        .intersection(&right_pins)
        .copied()
        .collect::<Vec<u8>>();
    overlap.sort_unstable();
    overlap
}

fn should_merge_by_transport_overlap(left: &PirNode, right: &PirNode) -> bool {
    let overlap = overlapping_transport_pins(left, right);
    if overlap.is_empty() {
        return false;
    }
    let left_count = node_transport_pin_set(left).len();
    let right_count = node_transport_pin_set(right).len();
    if left_count == 0 || right_count == 0 {
        return false;
    }
    let min_count = left_count.min(right_count);
    overlap.len() >= 3 || overlap.len() * 2 > min_count
}

fn rewrite_alias_targets(aliases: &mut HashMap<String, String>, from: &str, to: &str) {
    if from == to {
        return;
    }
    for target in aliases.values_mut() {
        if target == from {
            *target = to.to_string();
        }
    }
}

fn dedupe_components_by_pin_overlap(
    nodes: Vec<PirNode>,
    aliases: &mut HashMap<String, String>,
    notes: &mut Vec<JsonValue>,
) -> Vec<PirNode> {
    let mut kept: Vec<PirNode> = Vec::new();

    for node in nodes {
        let overlap_candidate_idx = kept
            .iter()
            .enumerate()
            .find_map(|(idx, existing)| should_merge_by_transport_overlap(existing, &node).then_some(idx));

        let Some(existing_idx) = overlap_candidate_idx else {
            kept.push(node);
            continue;
        };

        let existing_node_type = kept[existing_idx].node_type.clone();
        let incoming_node_type = node.node_type.clone();
        let overlap_pins = overlapping_transport_pins(&kept[existing_idx], &node);
        let keep_incoming =
            node_component_specificity(&incoming_node_type) > node_component_specificity(&existing_node_type);

        if keep_incoming {
            let canonical_id = node.id.clone();
            let previous_canonical_id = kept[existing_idx].id.clone();
            let replaced = std::mem::replace(&mut kept[existing_idx], node);
            rewrite_alias_targets(aliases, &previous_canonical_id, &canonical_id);
            if replaced.id != canonical_id {
                aliases.insert(replaced.id.clone(), canonical_id.clone());
            }
            notes.push(serde_json::json!({
                "kind": "llm_node_deduped",
                "message": format!(
                    "Merged overlapping transport-pin nodes {} ({}) into {} ({})",
                    replaced.id,
                    existing_node_type,
                    canonical_id,
                    incoming_node_type
                ),
                "node_id": replaced.id,
                "canonical_node_id": canonical_id,
                "overlap_pins": overlap_pins,
                "strict_evidence": true,
            }));
            merge_component_node(&mut kept[existing_idx], replaced);
            continue;
        }

        let canonical_id = kept[existing_idx].id.clone();
        let duplicate_id = node.id.clone();
        if duplicate_id != canonical_id {
            aliases.insert(duplicate_id.clone(), canonical_id.clone());
        }
        notes.push(serde_json::json!({
            "kind": "llm_node_deduped",
            "message": format!(
                "Merged overlapping transport-pin nodes {} ({}) into {} ({})",
                duplicate_id,
                incoming_node_type,
                canonical_id,
                existing_node_type
            ),
            "node_id": duplicate_id,
            "canonical_node_id": canonical_id,
            "overlap_pins": overlap_pins,
            "strict_evidence": true,
        }));
        merge_component_node(&mut kept[existing_idx], node);
    }

    kept
}

fn merge_pin_binding_props(target: &mut JsonValue, incoming: JsonValue) {
    let Some(target_obj) = target.as_object_mut() else {
        *target = incoming;
        return;
    };
    let Some(in_obj) = incoming.as_object() else {
        return;
    };
    for (key, value) in in_obj {
        if key == "pin_bindings" {
            let target_pin_bindings = target_obj
                .entry("pin_bindings".to_string())
                .or_insert_with(|| serde_json::json!({}));
            if !target_pin_bindings.is_object() {
                *target_pin_bindings = serde_json::json!({});
            }
            if let (Some(dst), Some(src)) = (target_pin_bindings.as_object_mut(), value.as_object())
            {
                for (binding_name, pin_value) in src {
                    dst.entry(binding_name.clone())
                        .or_insert_with(|| pin_value.clone());
                }
            }
            continue;
        }
        let should_overwrite = target_obj
            .get(key)
            .map(|existing| existing.is_null())
            .unwrap_or(true);
        if should_overwrite {
            target_obj.insert(key.clone(), value.clone());
        }
    }
}

fn merge_component_node(target: &mut PirNode, incoming: PirNode) {
    if target.label.is_none() {
        target.label = incoming.label.clone();
    }
    if target.ai_summary.is_none() {
        target.ai_summary = incoming.ai_summary.clone();
    }
    if incoming.confidence > target.confidence {
        target.confidence = incoming.confidence;
    }
    if matches!(
        incoming.authority,
        NodeAuthority::User | NodeAuthority::Hybrid
    ) {
        target.authority = incoming.authority.clone();
    }
    for editable in incoming.editable_fields {
        if !target.editable_fields.contains(&editable) {
            target.editable_fields.push(editable);
        }
    }
    for tag in incoming.semantic_tags {
        if !target.semantic_tags.contains(&tag) {
            target.semantic_tags.push(tag);
        }
    }
    for dep in incoming.dependencies {
        if !target.dependencies.contains(&dep) {
            target.dependencies.push(dep);
        }
    }
    for file in incoming.ownership.primary_files {
        if !target.ownership.primary_files.contains(&file) {
            target.ownership.primary_files.push(file);
        }
    }
    if target.ownership.component_id.is_none() {
        target.ownership.component_id = incoming.ownership.component_id.clone();
    }
    let mut seen_refs: HashSet<String> = target
        .source_refs
        .iter()
        .map(|r| {
            format!(
                "{}|{}|{}|{}",
                r.file,
                r.line.unwrap_or(0),
                r.symbol.clone().unwrap_or_default(),
                r.inferred_by
            )
        })
        .collect();
    for source_ref in incoming.source_refs {
        let key = format!(
            "{}|{}|{}|{}",
            source_ref.file,
            source_ref.line.unwrap_or(0),
            source_ref.symbol.clone().unwrap_or_default(),
            source_ref.inferred_by
        );
        if seen_refs.insert(key) {
            target.source_refs.push(source_ref);
        }
    }
    merge_pin_binding_props(&mut target.properties, incoming.properties);
}

fn canonicalize_component_identity_nodes(
    nodes: Vec<PirNode>,
) -> (Vec<PirNode>, HashMap<String, String>, Vec<JsonValue>) {
    let mut kept: Vec<PirNode> = Vec::new();
    let mut component_signatures: HashMap<String, usize> = HashMap::new();
    let mut aliases: HashMap<String, String> = HashMap::new();
    let mut notes: Vec<JsonValue> = Vec::new();

    for node in nodes {
        let signature = node_component_signature(&node);
        if let Some(signature) = signature {
            if let Some(existing_idx) = component_signatures.get(&signature).copied() {
                let canonical_id = kept[existing_idx].id.clone();
                let duplicate_id = node.id.clone();
                if duplicate_id != canonical_id {
                    aliases.insert(duplicate_id.clone(), canonical_id.clone());
                }
                notes.push(serde_json::json!({
                    "kind": "llm_node_deduped",
                    "message": format!(
                        "Collapsed duplicate component node {} into {} to preserve single-component identity",
                        duplicate_id,
                        canonical_id
                    ),
                    "node_id": duplicate_id,
                    "canonical_node_id": canonical_id,
                    "strict_evidence": true,
                }));
                merge_component_node(&mut kept[existing_idx], node);
                continue;
            }
            component_signatures.insert(signature, kept.len());
        }
        kept.push(node);
    }
    let before_overlap_dedupe = kept.len();
    kept = dedupe_components_by_pin_overlap(kept, &mut aliases, &mut notes);
    // #region agent log
    debug_mode_log(
        "critical-bugs",
        "H2",
        "parser.rs:canonicalize_component_identity_nodes:overlap_dedupe",
        "applied transport-pin overlap component dedupe",
        serde_json::json!({
            "nodes_before_overlap_dedupe": before_overlap_dedupe,
            "nodes_after_overlap_dedupe": kept.len(),
            "alias_count": aliases.len(),
        }),
    );
    // #endregion

    let mut transport_pins: HashSet<u8> = HashSet::new();
    for node in &kept {
        let bindings = node_transport_bindings(node);
        if bindings.is_empty() {
            continue;
        }
        let transport_bindings: Vec<u8> = bindings
            .iter()
            .filter(|(name, _)| is_transport_binding_name(name))
            .map(|(_, pin)| *pin)
            .collect();
        if transport_bindings.is_empty() {
            continue;
        }
        if !is_transport_component_type(&node.node_type) && transport_bindings.len() < 2 {
            continue;
        }
        transport_pins.extend(transport_bindings);
    }

    let mut filtered = Vec::new();
    for node in kept {
        let pin = node
            .properties
            .get("pin")
            .and_then(|v| v.as_u64())
            .and_then(|v| u8::try_from(v).ok());
        let is_transport_gpio = matches!(node.node_type.as_str(), "gpio_input" | "gpio_output")
            && !matches!(node.authority, NodeAuthority::User | NodeAuthority::Hybrid)
            && pin.map(|p| transport_pins.contains(&p)).unwrap_or(false);
        if is_transport_gpio {
            notes.push(serde_json::json!({
                "kind": "llm_node_rejected",
                "message": format!(
                    "Rejected standalone GPIO node {} because pin {} is already represented inside a component pin_bindings map",
                    node.id,
                    pin.unwrap_or_default()
                ),
                "node_id": node.id,
                "node_type": node.node_type,
                "strict_evidence": true,
            }));
            continue;
        }
        filtered.push(node);
    }

    (filtered, aliases, notes)
}

fn remap_edges_with_node_aliases(edges: &mut [PirEdge], aliases: &HashMap<String, String>) {
    if aliases.is_empty() {
        return;
    }
    for edge in edges {
        if let Some(remapped) = aliases.get(&edge.source_node_id) {
            if edge.source_node_id != *remapped {
                edge.source_node_id = remapped.clone();
                edge.source_port_id = None;
            }
        }
        if let Some(remapped) = aliases.get(&edge.target_node_id) {
            if edge.target_node_id != *remapped {
                edge.target_node_id = remapped.clone();
                edge.target_port_id = None;
            }
        }
    }
}

fn filter_nodes_by_evidence(
    nodes: Vec<PirNode>,
    analyzed_files: &HashSet<String>,
) -> (Vec<PirNode>, Vec<JsonValue>) {
    let min_confidence = super::super::builder::PIR_GRAPH_MIN_NODE_CONFIDENCE;
    let mut kept = Vec::new();
    let mut rejected = Vec::new();
    let mut kept_below_confidence_count = 0usize;
    let mut kept_below_confidence_sample = Vec::new();
    for node in nodes {
        let below_confidence = node.confidence < min_confidence;
        if node_is_file_backed(&node, analyzed_files) {
            if below_confidence {
                kept_below_confidence_count += 1;
                if kept_below_confidence_sample.len() < 5 {
                    kept_below_confidence_sample
                        .push(format!("{}:{:.2}", node.id, node.confidence));
                }
            }
            kept.push(node);
            continue;
        }
        rejected.push(serde_json::json!({
            "kind": "llm_node_rejected",
            "message": format!(
                "Rejected node {} ({}) because it has no file-backed evidence in analyzed files",
                node.id,
                node.node_type
            ),
            "node_id": node.id,
            "node_type": node.node_type,
            "strict_evidence": true,
        }));
    }
    // #region agent log
    debug_mode_log(
        "critical-bugs",
        "H3",
        "parser.rs:filter_nodes_by_evidence",
        "evaluated evidence filter before confidence pruning",
        serde_json::json!({
            "min_confidence_applied_later": min_confidence,
            "nodes_kept_by_evidence": kept.len(),
            "nodes_rejected_by_evidence": rejected.len(),
            "kept_below_confidence_count": kept_below_confidence_count,
            "kept_below_confidence_sample": kept_below_confidence_sample,
        }),
    );
    // #endregion
    (kept, rejected)
}

fn filter_edges_by_evidence(
    edges: Vec<PirEdge>,
    nodes: &[PirNode],
    analyzed_files: &HashSet<String>,
) -> (Vec<PirEdge>, Vec<JsonValue>) {
    let node_ids: HashSet<&str> = nodes.iter().map(|n| n.id.as_str()).collect();
    let mut kept = Vec::new();
    let mut rejected = Vec::new();
    for edge in edges {
        let endpoints_ok = node_ids.contains(edge.source_node_id.as_str())
            && node_ids.contains(edge.target_node_id.as_str());
        if !endpoints_ok {
            rejected.push(serde_json::json!({
                "kind": "llm_edge_rejected",
                "message": format!(
                    "Rejected edge {} because source/target nodes are not in evidence-backed node set",
                    edge.id
                ),
                "edge_id": edge.id,
                "strict_evidence": true,
            }));
            continue;
        }

        let has_valid_source_ref = edge.source_refs.is_empty()
            || edge
                .source_refs
                .iter()
                .any(|r| matches_analyzed_file(&r.file, analyzed_files));
        if !has_valid_source_ref {
            rejected.push(serde_json::json!({
                "kind": "llm_edge_rejected",
                "message": format!(
                    "Rejected edge {} because source_refs do not map to analyzed files",
                    edge.id
                ),
                "edge_id": edge.id,
                "strict_evidence": true,
            }));
            continue;
        }
        kept.push(edge);
    }
    (kept, rejected)
}

pub fn parse_nodes_from_value(nodes_val: Option<&JsonValue>) -> Result<Vec<PirNode>, String> {
    let Some(nodes_val) = nodes_val else {
        return Ok(Vec::new());
    };
    let Some(arr) = nodes_val.as_array() else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for n in arr {
        let id = n
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or("node missing id")?
            .to_string();
        let node_type = n
            .get("node_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("node {} missing node_type", id))?
            .to_string();
        let editable: Vec<String> = n
            .get("editable_fields")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_else(|| default_editable_for_type(&node_type));
        let authority: NodeAuthority = n
            .get("authority")
            .and_then(|v| v.as_str())
            .map(|s| match s {
                "user" => NodeAuthority::User,
                "hybrid" => NodeAuthority::Hybrid,
                _ => NodeAuthority::Agent,
            })
            .unwrap_or(NodeAuthority::Agent);
        out.push(PirNode {
            id,
            node_type,
            label: n.get("label").and_then(|v| v.as_str()).map(String::from),
            properties: n
                .get("properties")
                .cloned()
                .unwrap_or(JsonValue::Object(Default::default())),
            source_refs: n
                .get("source_refs")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default(),
            sync: PirNodeSync {
                state: PirSyncState::Synced,
                last_synced_revision: String::new(),
                last_error: None,
            },
            ownership: n
                .get("ownership")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default(),
            editable_fields: editable,
            layer: n.get("layer").and_then(|v| v.as_str()).map(String::from),
            ai_summary: n
                .get("ai_summary")
                .and_then(|v| v.as_str())
                .map(String::from),
            confidence: n
                .get("confidence")
                .and_then(|v| v.as_f64())
                .map(|f| f as f32)
                .unwrap_or(0.75),
            authority,
            semantic_tags: n
                .get("semantic_tags")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default(),
            dependencies: n
                .get("dependencies")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default(),
            stale_reason: None,
        });
    }
    let unknown_types: Vec<String> = out
        .iter()
        .filter(|n| get_node_type_def(&n.node_type).is_none())
        .take(8)
        .map(|n| format!("{}:{}", n.id, n.node_type))
        .collect();
    let function_like_nodes: Vec<String> = out
        .iter()
        .filter(|n| {
            let text = format!(
                "{} {} {}",
                n.id,
                n.label.as_deref().unwrap_or(""),
                n.node_type
            )
            .to_lowercase();
            text.contains("function")
                || text.contains("handler")
                || text.contains("callback")
                || text.contains("method")
        })
        .take(8)
        .map(|n| format!("{}:{}", n.id, n.node_type))
        .collect();
    // #region agent log
    debug_mode_log(
        "pir-graph-quality",
        "H1",
        "parser.rs:parse_nodes_from_value",
        "parsed LLM node block",
        serde_json::json!({
            "nodes_count": out.len(),
            "unknown_node_type_count": unknown_types.len(),
            "unknown_node_types": unknown_types.clone(),
            "function_like_node_count": function_like_nodes.len(),
            "function_like_nodes": function_like_nodes.clone(),
        }),
    );
    // #endregion
    if !unknown_types.is_empty() {
        return Err(format!(
            "agent output contains unsupported node_type(s): {}",
            unknown_types.join(", ")
        ));
    }
    Ok(out)
}

pub fn parse_edges_from_value(edges_val: Option<&JsonValue>) -> Result<Vec<PirEdge>, String> {
    let Some(edges_val) = edges_val else {
        return Ok(Vec::new());
    };
    let Some(arr) = edges_val.as_array() else {
        return Ok(Vec::new());
    };
    let read_str = |obj: &JsonValue, keys: &[&str]| -> Option<String> {
        keys.iter()
            .find_map(|k| obj.get(*k).and_then(|v| v.as_str()))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    };
    let node_id_from_port_like = |port_like: &str| -> Option<String> {
        let trimmed = port_like.trim();
        if trimmed.is_empty() {
            return None;
        }
        let idx = trimmed.rfind('_')?;
        let maybe_index = &trimmed[idx + 1..];
        if maybe_index.parse::<usize>().is_err() {
            return None;
        }
        let without_index = &trimmed[..idx];
        let known_port_suffixes = [
            "boot_out",
            "exec_in",
            "exec_out",
            "trigger_in",
            "trigger_out",
            "data_in",
            "data_out",
            "event_in",
            "event_out",
            "network_in",
            "network_out",
            "gpio_in",
            "gpio_out",
            "assoc_in",
            "assoc_out",
        ];
        for suffix in known_port_suffixes {
            let marker = format!("_{}", suffix);
            if without_index.ends_with(&marker) {
                let node = without_index[..without_index.len() - marker.len()].to_string();
                if !node.is_empty() {
                    return Some(node);
                }
            }
        }
        None
    };
    let mut out = Vec::new();
    let mut recovered_from_alt_schema = 0usize;
    let mut recovered_ports_from_edge_id = 0usize;
    let mut recovered_node_ids_from_ports = 0usize;
    for e in arr {
        let kind_str = e
            .get("kind")
            .and_then(|v| v.as_str())
            .unwrap_or("execution");
        let kind = match kind_str {
            "data" => PirEdgeKind::Data,
            "hardware" => PirEdgeKind::Hardware,
            "dependency" => PirEdgeKind::Dependency,
            "event" => PirEdgeKind::Event,
            "network" => PirEdgeKind::Network,
            "ota" => PirEdgeKind::Ota,
            "fsm" => PirEdgeKind::Fsm,
            _ => PirEdgeKind::Execution,
        };
        let edge_id = e
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("edge")
            .to_string();
        let mut source_node_id = read_str(
            e,
            &[
                "source_node_id",
                "source_node",
                "sourceNodeId",
                "source",
                "from",
            ],
        )
        .unwrap_or_default();
        let mut target_node_id = read_str(
            e,
            &[
                "target_node_id",
                "target_node",
                "targetNodeId",
                "target",
                "to",
            ],
        )
        .unwrap_or_default();
        if !source_node_id.is_empty() {
            recovered_from_alt_schema += 1;
        }
        if !target_node_id.is_empty() {
            recovered_from_alt_schema += 1;
        }
        let mut source_port_id = read_str(
            e,
            &[
                "source_port_id",
                "source_port",
                "sourcePortId",
                "source_handle",
                "sourceHandle",
            ],
        );
        let mut target_port_id = read_str(
            e,
            &[
                "target_port_id",
                "target_port",
                "targetPortId",
                "target_handle",
                "targetHandle",
            ],
        );

        if source_port_id.is_none() || target_port_id.is_none() {
            if let Some((sp, tp)) = edge_id.split_once("__") {
                let sp = sp.trim();
                let tp = tp.trim();
                if !sp.is_empty() && !tp.is_empty() {
                    if source_port_id.is_none() {
                        source_port_id = Some(sp.to_string());
                        recovered_ports_from_edge_id += 1;
                    }
                    if target_port_id.is_none() {
                        target_port_id = Some(tp.to_string());
                        recovered_ports_from_edge_id += 1;
                    }
                }
            }
        }
        if source_node_id.is_empty() || source_node_id == source_port_id.clone().unwrap_or_default()
        {
            if let Some(from_port) = source_port_id
                .as_deref()
                .and_then(node_id_from_port_like)
                .or_else(|| node_id_from_port_like(&source_node_id))
            {
                source_node_id = from_port;
                recovered_node_ids_from_ports += 1;
            }
        }
        if target_node_id.is_empty() || target_node_id == target_port_id.clone().unwrap_or_default()
        {
            if let Some(from_port) = target_port_id
                .as_deref()
                .and_then(node_id_from_port_like)
                .or_else(|| node_id_from_port_like(&target_node_id))
            {
                target_node_id = from_port;
                recovered_node_ids_from_ports += 1;
            }
        }

        out.push(PirEdge {
            id: edge_id,
            source_node_id,
            target_node_id,
            source_port_id,
            target_port_id,
            kind,
            confidence: e
                .get("confidence")
                .and_then(|v| v.as_f64())
                .map(|f| f as f32)
                .unwrap_or(0.7),
            source_refs: e
                .get("source_refs")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default(),
            semantic_label: e
                .get("semantic_label")
                .or_else(|| e.get("relationship"))
                .and_then(|v| v.as_str())
                .map(String::from),
            validated: false,
        });
    }
    let missing_endpoints = out
        .iter()
        .filter(|e| e.source_node_id.trim().is_empty() || e.target_node_id.trim().is_empty())
        .count();
    let missing_ports = out
        .iter()
        .filter(|e| e.source_port_id.is_none() || e.target_port_id.is_none())
        .count();
    // #region agent log
    debug_mode_log(
        "pir-graph-quality",
        "H3",
        "parser.rs:parse_edges_from_value",
        "parsed LLM edge block",
        serde_json::json!({
            "edges_count": out.len(),
            "missing_endpoint_count": missing_endpoints,
            "missing_port_count": missing_ports,
            "recovered_from_alt_schema_count": recovered_from_alt_schema,
            "recovered_ports_from_edge_id_count": recovered_ports_from_edge_id,
            "recovered_node_ids_from_ports_count": recovered_node_ids_from_ports,
            "sample_edges": out.iter().take(5).map(|e| serde_json::json!({
                "id": e.id,
                "source_node_id": e.source_node_id,
                "target_node_id": e.target_node_id,
                "source_port_id": e.source_port_id,
                "target_port_id": e.target_port_id,
                "kind": format!("{:?}", e.kind),
            })).collect::<Vec<_>>(),
        }),
    );
    // #endregion
    Ok(out)
}

pub fn parse_layers(json: &JsonValue) -> PirLayers {
    serde_json::from_value(json.clone()).unwrap_or_default()
}

fn parse_summary(json: &JsonValue, node_count: usize, edge_count: usize) -> PirSummary {
    if let Some(s) = json.get("summary") {
        if let Ok(mut summary) = serde_json::from_value::<PirSummary>(s.clone()) {
            summary.node_count = node_count as u32;
            summary.edge_count = edge_count as u32;
            return summary;
        }
    }
    PirSummary {
        headline: format!(
            "{} nodes, {} edges — AI-generated topology",
            node_count, edge_count
        ),
        node_count: node_count as u32,
        edge_count: edge_count as u32,
        warnings: Vec::new(),
    }
}

fn parse_diagrams(json: &JsonValue) -> Option<PirDiagrams> {
    parse_diagrams_from_value(json.get("diagrams"))
}

fn parse_mermaid_block(v: &JsonValue, default_title: &str) -> Option<PirDiagramMermaid> {
    if let Some(mermaid) = v.as_str().map(str::trim).filter(|s| !s.is_empty()) {
        return Some(PirDiagramMermaid {
            title: Some(default_title.to_string()),
            mermaid: Some(mermaid.to_string()),
        });
    }
    let obj = v.as_object()?;
    let title = obj
        .get("title")
        .and_then(|x| x.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| Some(default_title.to_string()));
    let mermaid = obj
        .get("mermaid")
        .and_then(|x| x.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    if mermaid.is_none() {
        return None;
    }
    Some(PirDiagramMermaid { title, mermaid })
}

fn parse_string_array(v: Option<&JsonValue>) -> Vec<String> {
    let mut seen = HashSet::<String>::new();
    let mut out = Vec::new();
    if let Some(arr) = v.and_then(|x| x.as_array()) {
        for item in arr {
            let Some(s) = item.as_str().map(str::trim).filter(|s| !s.is_empty()) else {
                continue;
            };
            let value = s.to_string();
            if seen.insert(value.clone()) {
                out.push(value);
            }
        }
    }
    out
}

fn validate_sequence_block(mut sequence: PirSequenceDiagram) -> PirSequenceDiagram {
    let Some(mermaid) = sequence.mermaid.clone() else {
        return sequence;
    };
    match super::super::diagrams::validate_sequence_mermaid(&mermaid) {
        Ok(participants) => {
            sequence.participants = participants;
            sequence.generation_error = None;
        }
        Err(err) => {
            sequence.mermaid = None;
            sequence.generation_error =
                Some(format!("Sequence Mermaid validation failed: {}", err));
        }
    }
    sequence
}

fn parse_sequence_block(v: &JsonValue) -> Option<PirSequenceDiagram> {
    if let Some(mermaid) = v.as_str().map(str::trim).filter(|s| !s.is_empty()) {
        let sequence = PirSequenceDiagram {
            title: Some("Sequence Diagram".to_string()),
            mermaid: Some(mermaid.to_string()),
            participants: Vec::new(),
            generated_from: Vec::new(),
            generation_error: None,
        };
        return Some(validate_sequence_block(sequence));
    }

    let obj = v.as_object()?;
    let sequence = PirSequenceDiagram {
        title: obj
            .get("title")
            .and_then(|x| x.as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .or_else(|| Some("Sequence Diagram".to_string())),
        mermaid: obj
            .get("mermaid")
            .and_then(|x| x.as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()),
        participants: parse_string_array(obj.get("participants")),
        generated_from: parse_string_array(obj.get("generated_from")),
        generation_error: obj
            .get("generation_error")
            .and_then(|x| x.as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()),
    };

    if sequence.mermaid.is_none() && sequence.generation_error.is_none() {
        return None;
    }
    Some(validate_sequence_block(sequence))
}

pub fn parse_diagrams_from_value(diagrams_val: Option<&JsonValue>) -> Option<PirDiagrams> {
    let diagrams = diagrams_val?;
    let mut parsed = PirDiagrams::default();

    parsed.hld = diagrams
        .get("hld")
        .and_then(|v| parse_mermaid_block(v, "High-Level Design"))
        .or_else(|| {
            diagrams
                .get("hld_mermaid")
                .and_then(|v| parse_mermaid_block(v, "High-Level Design"))
        });
    parsed.lld = diagrams
        .get("lld")
        .and_then(|v| parse_mermaid_block(v, "Low-Level Design"))
        .or_else(|| {
            diagrams
                .get("lld_mermaid")
                .and_then(|v| parse_mermaid_block(v, "Low-Level Design"))
        });
    parsed.sequence = diagrams
        .get("sequence")
        .and_then(parse_sequence_block)
        .or_else(|| {
            diagrams
                .get("sequence_mermaid")
                .and_then(parse_sequence_block)
        });

    parsed.hld_graph = diagrams
        .get("hld_graph")
        .and_then(|v| serde_json::from_value(v.clone()).ok());
    parsed.lld_graph = diagrams
        .get("lld_graph")
        .and_then(|v| serde_json::from_value(v.clone()).ok());
    parsed.sequence_graph = diagrams
        .get("sequence_graph")
        .and_then(|v| serde_json::from_value(v.clone()).ok());

    let has_hld = parsed
        .hld
        .as_ref()
        .and_then(|d| d.mermaid.as_ref())
        .is_some();
    let has_lld = parsed
        .lld
        .as_ref()
        .and_then(|d| d.mermaid.as_ref())
        .is_some();
    let has_sequence = parsed
        .sequence
        .as_ref()
        .and_then(|d| d.mermaid.as_ref())
        .is_some()
        || parsed
            .sequence
            .as_ref()
            .and_then(|d| d.generation_error.as_ref())
            .is_some();
    let has_graphs =
        parsed.hld_graph.is_some() || parsed.lld_graph.is_some() || parsed.sequence_graph.is_some();

    if !(has_hld || has_lld || has_sequence || has_graphs) {
        return None;
    }

    Some(parsed)
}

fn extract_json_object(raw: &str) -> Result<JsonValue, String> {
    extract_json_object_public(raw)
}

pub fn extract_json_object_public(raw: &str) -> Result<JsonValue, String> {
    let trimmed = raw.trim();
    if let Ok(v) = serde_json::from_str(trimmed) {
        return Ok(v);
    }
    let start = trimmed.find('{').ok_or("no JSON object in agent output")?;
    let end = trimmed.rfind('}').ok_or("no JSON object in agent output")?;
    serde_json::from_str(&trimmed[start..=end]).map_err(|e| format!("invalid JSON: {}", e))
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{parse_agent_output, parse_nodes_from_value};
    use crate::pir_maker::schema::PirGenerationMeta;

    #[test]
    fn rejects_unknown_node_type() {
        let nodes = serde_json::json!([
            {
                "id": "mystery",
                "node_type": "mystery_node",
                "source_refs": [{"file": "main/main.c", "confidence": 1.0, "inferred_by": "ai"}],
                "ownership": {"primary_files": ["main/main.c"]}
            }
        ]);
        let err = parse_nodes_from_value(Some(&nodes)).expect_err("unknown node type must fail");
        assert!(err.contains("unsupported node_type"));
    }

    #[test]
    fn filters_nodes_without_file_evidence() {
        let raw = serde_json::json!({
            "nodes": [
                {
                    "id": "boot",
                    "node_type": "system_init",
                    "source_refs": [{"file": "main/main.c", "confidence": 1.0, "inferred_by": "ai"}],
                    "ownership": {"primary_files": ["main/main.c"]}
                },
                {
                    "id": "speculative_ble",
                    "node_type": "ble_manager",
                    "source_refs": [],
                    "ownership": {"primary_files": []}
                }
            ],
            "edges": [
                {
                    "id": "e_boot_ble",
                    "source_node_id": "boot",
                    "target_node_id": "speculative_ble",
                    "kind": "execution",
                    "source_refs": []
                }
            ],
            "layers": {
                "system": ["boot"],
                "network": ["speculative_ble"]
            }
        })
        .to_string();

        let generation = PirGenerationMeta {
            mode: "ai_full".to_string(),
            model: None,
            triggered_by: "test".to_string(),
            analyzed_at_ms: 0,
            input_files: vec!["main/main.c".to_string()],
            token_usage: None,
        };

        let parsed = parse_agent_output(
            &raw,
            "C:/tmp/demo",
            Some("chat-1"),
            "rev-1",
            None,
            vec!["main/main.c".to_string()],
            HashMap::new(),
            generation,
            None,
        )
        .expect("parse should succeed");

        assert_eq!(parsed.nodes.len(), 1);
        assert_eq!(parsed.nodes[0].id, "boot");
        assert!(parsed
            .unresolved
            .iter()
            .any(|u| u.get("kind").and_then(|v| v.as_str()) == Some("llm_node_rejected")));
    }

    #[test]
    fn collapses_duplicate_components_and_drops_transport_gpio_nodes() {
        let raw = serde_json::json!({
            "nodes": [
                {
                    "id": "oled_spi",
                    "node_type": "spi_device",
                    "label": "OLED SPI",
                    "properties": {
                        "host": "SPI2_HOST",
                        "pin_bindings": {
                            "sclk": 11,
                            "mosi": 10,
                            "cs": 9,
                            "dc": 8,
                            "rst": 13
                        }
                    },
                    "source_refs": [{"file": "main/app_config.h", "confidence": 1.0, "inferred_by": "ai"}],
                    "ownership": {"primary_files": ["main/app_config.h"]}
                },
                {
                    "id": "oled_spi_duplicate",
                    "node_type": "spi_device",
                    "label": "OLED SPI",
                    "properties": {
                        "host": "SPI2_HOST",
                        "pin_bindings": {
                            "sclk": 11,
                            "mosi": 10,
                            "cs": 9,
                            "dc": 8,
                            "rst": 13
                        }
                    },
                    "source_refs": [{"file": "main/app_config.h", "confidence": 1.0, "inferred_by": "ai"}],
                    "ownership": {"primary_files": ["main/app_config.h"]}
                },
                {
                    "id": "gpio_11",
                    "node_type": "gpio_output",
                    "properties": {"pin": 11},
                    "source_refs": [{"file": "main/app_config.h", "confidence": 1.0, "inferred_by": "ai"}],
                    "ownership": {"primary_files": ["main/app_config.h"]}
                },
                {
                    "id": "gpio_10",
                    "node_type": "gpio_output",
                    "properties": {"pin": 10},
                    "source_refs": [{"file": "main/app_config.h", "confidence": 1.0, "inferred_by": "ai"}],
                    "ownership": {"primary_files": ["main/app_config.h"]}
                },
                {
                    "id": "status_led",
                    "node_type": "gpio_output",
                    "properties": {"pin": 5},
                    "source_refs": [{"file": "main/app_config.h", "confidence": 1.0, "inferred_by": "ai"}],
                    "ownership": {"primary_files": ["main/app_config.h"]}
                }
            ],
            "edges": [
                {
                    "id": "e1",
                    "source_node_id": "oled_spi_duplicate",
                    "target_node_id": "gpio_11",
                    "kind": "hardware",
                    "source_refs": [{"file": "main/app_config.h", "confidence": 1.0, "inferred_by": "ai"}]
                }
            ],
            "layers": {
                "network": ["oled_spi", "oled_spi_duplicate"],
                "physical": ["gpio_11", "gpio_10", "status_led"]
            }
        })
        .to_string();

        let generation = PirGenerationMeta {
            mode: "ai_full".to_string(),
            model: None,
            triggered_by: "test".to_string(),
            analyzed_at_ms: 0,
            input_files: vec!["main/app_config.h".to_string()],
            token_usage: None,
        };

        let parsed = parse_agent_output(
            &raw,
            "C:/tmp/demo",
            Some("chat-1"),
            "rev-1",
            None,
            vec!["main/app_config.h".to_string()],
            HashMap::new(),
            generation,
            None,
        )
        .expect("parse should succeed");

        let spi_nodes: Vec<_> = parsed
            .nodes
            .iter()
            .filter(|n| n.node_type == "spi_device")
            .collect();
        assert_eq!(
            spi_nodes.len(),
            1,
            "duplicate component nodes should collapse"
        );
        assert!(
            parsed
                .nodes
                .iter()
                .filter(|n| n.node_type == "gpio_output")
                .all(
                    |n| n.properties.get("pin").and_then(|v| v.as_u64()) != Some(11)
                        && n.properties.get("pin").and_then(|v| v.as_u64()) != Some(10)
                ),
            "transport gpio nodes should be removed when represented in component pin_bindings"
        );
        assert!(
            parsed.nodes.iter().any(|n| n.id == "status_led"),
            "standalone gpio nodes should remain"
        );
        assert!(
            parsed
                .unresolved
                .iter()
                .any(|u| u.get("kind").and_then(|v| v.as_str()) == Some("llm_node_deduped")),
            "dedupe should be recorded in unresolved notes"
        );
    }

    #[test]
    fn merges_cross_type_components_with_overlapping_transport_pins() {
        let raw = serde_json::json!({
            "nodes": [
                {
                    "id": "oled_display",
                    "node_type": "display_output",
                    "label": "OLED Display",
                    "properties": {
                        "interface": "spi",
                        "pin_bindings": {
                            "sclk": 11,
                            "mosi": 10,
                            "cs": 9,
                            "dc": 8,
                            "rst": 13
                        }
                    },
                    "source_refs": [{"file": "main/app_config.h", "confidence": 1.0, "inferred_by": "ai"}],
                    "ownership": {"primary_files": ["main/app_config.h"]}
                },
                {
                    "id": "oled_spi",
                    "node_type": "spi_device",
                    "label": "OLED SPI",
                    "properties": {
                        "host": "SPI2_HOST",
                        "pin_bindings": {
                            "sclk": 11,
                            "mosi": 10,
                            "cs": 9,
                            "dc": 8,
                            "rst": 13
                        }
                    },
                    "source_refs": [{"file": "main/app_config.h", "confidence": 1.0, "inferred_by": "ai"}],
                    "ownership": {"primary_files": ["main/app_config.h"]}
                },
                {
                    "id": "servo_pwm",
                    "node_type": "pwm_output",
                    "properties": {
                        "pin_bindings": {"signal": 5}
                    },
                    "source_refs": [{"file": "main/app_config.h", "confidence": 1.0, "inferred_by": "ai"}],
                    "ownership": {"primary_files": ["main/app_config.h"]}
                }
            ],
            "edges": [],
            "layers": {
                "physical": ["oled_display", "oled_spi", "servo_pwm"]
            }
        })
        .to_string();

        let generation = PirGenerationMeta {
            mode: "ai_full".to_string(),
            model: None,
            triggered_by: "test".to_string(),
            analyzed_at_ms: 0,
            input_files: vec!["main/app_config.h".to_string()],
            token_usage: None,
        };

        let parsed = parse_agent_output(
            &raw,
            "C:/tmp/demo",
            Some("chat-1"),
            "rev-1",
            None,
            vec!["main/app_config.h".to_string()],
            HashMap::new(),
            generation,
            None,
        )
        .expect("parse should succeed");

        let oled_nodes: Vec<_> = parsed
            .nodes
            .iter()
            .filter(|n| n.id == "oled_display" || n.id == "oled_spi")
            .collect();
        assert_eq!(
            oled_nodes.len(),
            1,
            "cross-type OLED candidates should collapse into one peripheral node"
        );
        assert_eq!(
            oled_nodes[0].node_type, "spi_device",
            "more specific spi_device node type should win over display_output"
        );
        assert!(
            parsed.nodes.iter().any(|n| n.id == "servo_pwm"),
            "independent peripherals with different pins must remain"
        );
    }
}
