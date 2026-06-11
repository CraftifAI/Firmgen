//! Mermaid diagram generation for PIR artifacts.
//!
//! Core principle: backend/PIR agent owns architecture reasoning + Mermaid code creation.

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet, VecDeque};

use super::diagram_view_builders::{self, build_ldd_edge_labels, sequence_node_order};
use super::schema::{PirDiagrams, PirDocument, PirEdge, PirEdgeKind, PirNode, PirSequenceDiagram};
use crate::firmware_topology::compute_layout;
use crate::firmware_topology::registry::get_node_type_def;
use crate::firmware_topology::types::{
    FirmwareGraph, FirmwareNode, LayoutConfig, Port, PortDirection, RuntimeMetadata,
    VisualMetadata, SCHEMA_VERSION,
};
use serde_json::json;

fn mermaid_safe_id(raw: &str) -> String {
    raw.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn mermaid_escape_label(raw: &str) -> String {
    // Mermaid flowchart labels inside ["..."] tolerate quotes poorly; escape minimal set.
    raw.replace('\\', "\\\\").replace('"', "\\\"")
}

fn node_display_label(n: &PirNode) -> String {
    n.label
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or(&n.node_type)
        .trim()
        .to_string()
}

fn edge_label(e: &PirEdge) -> String {
    if let Some(l) = e.semantic_label.as_deref() {
        let t = l.trim();
        if !t.is_empty() {
            return t.to_string();
        }
    }
    match e.kind {
        PirEdgeKind::Execution => "executes",
        PirEdgeKind::Data => "data",
        PirEdgeKind::Hardware => "uses",
        PirEdgeKind::Dependency => "depends_on",
        PirEdgeKind::Event => "event",
        PirEdgeKind::Network => "connects",
        PirEdgeKind::Ota => "ota",
        PirEdgeKind::Fsm => "fsm",
    }
    .to_string()
}

fn normalize_seq_symbol(raw: &str) -> String {
    raw.trim_matches(|c: char| c == '"' || c == '\'')
        .trim()
        .to_string()
}

fn parse_sequence_participant(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if !trimmed.starts_with("participant ") {
        return None;
    }
    let rest = trimmed.strip_prefix("participant ")?.trim();
    let id = rest.split_whitespace().next()?;
    let symbol = normalize_seq_symbol(id);
    if symbol.is_empty() {
        return None;
    }
    Some(symbol)
}

fn parse_sequence_message_endpoints(line: &str) -> Option<(String, String)> {
    const ARROWS: [&str; 5] = ["-->>", "->>", "-->", "->", "-x"];
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.starts_with("Note ")
        || trimmed.starts_with("loop ")
        || trimmed.starts_with("alt ")
        || trimmed.starts_with("opt ")
        || trimmed.starts_with("par ")
        || trimmed.starts_with("critical ")
        || trimmed.starts_with("rect ")
        || trimmed.starts_with("autonumber")
        || trimmed.starts_with("title ")
        || trimmed == "end"
        || trimmed.starts_with("activate ")
        || trimmed.starts_with("deactivate ")
    {
        return None;
    }

    let mut found: Option<(&str, usize)> = None;
    for arrow in ARROWS {
        if let Some(idx) = trimmed.find(arrow) {
            found = Some((arrow, idx));
            break;
        }
    }
    let (arrow, idx) = found?;
    let left = normalize_seq_symbol(trimmed[..idx].trim());
    if left.is_empty() {
        return None;
    }
    let right_raw = trimmed[idx + arrow.len()..]
        .split(':')
        .next()
        .map(str::trim)
        .unwrap_or_default();
    let right = normalize_seq_symbol(right_raw);
    if right.is_empty() {
        return None;
    }
    Some((left, right))
}

fn sequence_safe_id(raw: &str) -> String {
    let mut id: String = raw
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if id.is_empty() {
        id = "node".to_string();
    }
    if id
        .chars()
        .next()
        .map(|c| c.is_ascii_digit())
        .unwrap_or(false)
    {
        id = format!("n_{}", id);
    }
    id
}

fn sequence_participants_match_graph(participants: &[String], graph: &FirmwareGraph) -> bool {
    let mut allowed = HashSet::<String>::new();
    for node in &graph.nodes {
        allowed.insert(node.id.clone());
        allowed.insert(sequence_safe_id(&node.id));
    }
    participants.iter().all(|p| allowed.contains(p))
}

fn generate_sequence_mermaid_fallback(pir: &PirDocument, graph: &FirmwareGraph) -> Option<String> {
    if graph.nodes.len() < 2 {
        return None;
    }

    let mut ordered_nodes: Vec<&FirmwareNode> = graph.nodes.iter().collect();
    ordered_nodes.sort_by(|a, b| {
        let layer_a = a.visual.as_ref().and_then(|v| v.layer).unwrap_or(u32::MAX);
        let layer_b = b.visual.as_ref().and_then(|v| v.layer).unwrap_or(u32::MAX);
        let x_a = a.visual.as_ref().and_then(|v| v.x).unwrap_or(f64::MAX);
        let x_b = b.visual.as_ref().and_then(|v| v.x).unwrap_or(f64::MAX);
        layer_a
            .cmp(&layer_b)
            .then_with(|| x_a.partial_cmp(&x_b).unwrap_or(Ordering::Equal))
            .then_with(|| a.id.cmp(&b.id))
    });

    let node_rank: HashMap<String, usize> = ordered_nodes
        .iter()
        .enumerate()
        .map(|(idx, node)| (node.id.clone(), idx))
        .collect();

    let mut participant_ids = HashSet::<String>::new();
    let mut node_to_participant = HashMap::<String, String>::new();
    let mut node_to_label = HashMap::<String, String>::new();
    for node in &ordered_nodes {
        let mut participant = sequence_safe_id(&node.id);
        if !participant_ids.insert(participant.clone()) {
            let mut suffix = 1u32;
            loop {
                let candidate = format!("{}_{}", participant, suffix);
                if participant_ids.insert(candidate.clone()) {
                    participant = candidate;
                    break;
                }
                suffix += 1;
            }
        }
        node_to_participant.insert(node.id.clone(), participant.clone());
        let label = node
            .label
            .as_deref()
            .filter(|l| !l.trim().is_empty())
            .unwrap_or(&node.node_type)
            .replace('"', "'");
        node_to_label.insert(node.id.clone(), label);
    }

    let mut port_to_node = HashMap::<String, String>::new();
    for node in &graph.nodes {
        for port in &node.ports {
            port_to_node.insert(port.id.clone(), node.id.clone());
        }
    }

    let mut messages = Vec::<(String, String, String)>::new();
    let mut seen_pairs = HashSet::<(String, String)>::new();
    let mut edge_labels = HashMap::<(String, String), String>::new();
    for edge in &pir.edges {
        edge_labels
            .entry((edge.source_node_id.clone(), edge.target_node_id.clone()))
            .or_insert_with(|| edge_label(edge));
    }

    for (idx, pair) in graph.connections.iter().enumerate() {
        let src_node = port_to_node.get(&pair[0]);
        let dst_node = port_to_node.get(&pair[1]);
        let (Some(src_node), Some(dst_node)) = (src_node, dst_node) else {
            continue;
        };
        if src_node == dst_node {
            continue;
        }
        if !node_to_participant.contains_key(src_node)
            || !node_to_participant.contains_key(dst_node)
        {
            continue;
        }
        if !seen_pairs.insert((src_node.clone(), dst_node.clone())) {
            continue;
        }
        let label = edge_labels
            .get(&(src_node.clone(), dst_node.clone()))
            .cloned()
            .unwrap_or_else(|| format!("step_{}", idx + 1))
            .replace('"', "'");
        messages.push((src_node.clone(), dst_node.clone(), label));
    }

    if messages.is_empty() {
        for idx in 0..ordered_nodes.len().saturating_sub(1) {
            messages.push((
                ordered_nodes[idx].id.clone(),
                ordered_nodes[idx + 1].id.clone(),
                format!("step_{}", idx + 1),
            ));
        }
    }

    if messages.is_empty() {
        return None;
    }

    messages.sort_by(|a, b| {
        let a_src = node_rank.get(&a.0).copied().unwrap_or(usize::MAX);
        let b_src = node_rank.get(&b.0).copied().unwrap_or(usize::MAX);
        let a_dst = node_rank.get(&a.1).copied().unwrap_or(usize::MAX);
        let b_dst = node_rank.get(&b.1).copied().unwrap_or(usize::MAX);
        a_src
            .cmp(&b_src)
            .then(a_dst.cmp(&b_dst))
            .then(a.0.cmp(&b.0))
            .then(a.1.cmp(&b.1))
    });

    let mut active_nodes = HashSet::<String>::new();
    for (src_node, dst_node, _) in &messages {
        active_nodes.insert(src_node.clone());
        active_nodes.insert(dst_node.clone());
    }
    if active_nodes.len() < 2 {
        return None;
    }

    let mut lines = vec!["sequenceDiagram".to_string()];
    for node in &ordered_nodes {
        if !active_nodes.contains(&node.id) {
            continue;
        }
        let participant = node_to_participant.get(&node.id)?;
        let label = node_to_label.get(&node.id)?;
        lines.push(format!("    participant {} as {}", participant, label));
    }

    for (src_node, dst_node, label) in messages {
        let src = node_to_participant.get(&src_node)?;
        let dst = node_to_participant.get(&dst_node)?;
        lines.push(format!("    {}->>{}: {}", src, dst, label));
    }

    Some(lines.join("\n"))
}

/// Validate Mermaid sequence syntax at a structural level and return participant ids.
pub fn validate_sequence_mermaid(code: &str) -> Result<Vec<String>, String> {
    let trimmed = code.trim();
    let mut lines = trimmed.lines().map(str::trim).filter(|l| !l.is_empty());
    let first = lines.next().ok_or("empty Mermaid sequence string")?;
    if !first.starts_with("sequenceDiagram") {
        return Err("must start with `sequenceDiagram`".to_string());
    }

    let mut participants = Vec::<String>::new();
    let mut participant_set = HashSet::<String>::new();

    for line in trimmed.lines() {
        if let Some(p) = parse_sequence_participant(line) {
            if participant_set.insert(p.clone()) {
                participants.push(p);
            }
        }
    }

    if participants.len() < 2 {
        return Err("must declare at least 2 participants".to_string());
    }

    let mut participants_with_messages = HashSet::<String>::new();
    let mut message_count = 0usize;
    for line in trimmed.lines() {
        if let Some((src, dst)) = parse_sequence_message_endpoints(line) {
            if !participant_set.contains(&src) {
                return Err(format!(
                    "message source `{}` is not a declared participant",
                    src
                ));
            }
            if !participant_set.contains(&dst) {
                return Err(format!(
                    "message target `{}` is not a declared participant",
                    dst
                ));
            }
            participants_with_messages.insert(src);
            participants_with_messages.insert(dst);
            message_count += 1;
        }
    }

    if message_count == 0 {
        return Err("must include at least one participant interaction".to_string());
    }

    let disconnected: Vec<String> = participants
        .iter()
        .filter(|p| !participants_with_messages.contains(*p))
        .cloned()
        .collect();
    if !disconnected.is_empty() {
        return Err(format!(
            "disconnected participants without interactions: {}",
            disconnected.join(", ")
        ));
    }

    Ok(participants)
}

fn infer_sequence_generated_from(pir: &PirDocument) -> Vec<String> {
    let mut out = Vec::<String>::new();
    let mut seen = HashSet::<String>::new();

    for node in &pir.nodes {
        for file in &node.ownership.primary_files {
            if !file.trim().is_empty() && seen.insert(file.clone()) {
                out.push(file.clone());
            }
        }
    }
    for edge in &pir.edges {
        for s in &edge.source_refs {
            if !s.file.trim().is_empty() && seen.insert(s.file.clone()) {
                out.push(s.file.clone());
            }
        }
    }
    if out.is_empty() {
        for node in &pir.nodes {
            if seen.insert(node.id.clone()) {
                out.push(node.id.clone());
            }
        }
    }
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum HldNodeRole {
    User,
    Cloud,
    Controller,
    Peripheral,
    Storage,
    Connectivity,
    Runtime,
    Alert,
    Internet,
}

#[derive(Debug, Clone)]
struct HldNodeSpec {
    id: String,
    label: String,
    role: HldNodeRole,
    source_node_id: Option<String>,
    source_node_type: Option<String>,
    summary: Option<String>,
}

#[derive(Debug, Clone)]
struct HldEdgeSpec {
    source_id: String,
    target_id: String,
    label: String,
    bidirectional: bool,
}

#[derive(Debug, Clone, Default)]
struct HldDiagramSpec {
    nodes: Vec<HldNodeSpec>,
    edges: Vec<HldEdgeSpec>,
}

fn hld_role_layer(role: HldNodeRole) -> u32 {
    match role {
        HldNodeRole::User => 0,
        HldNodeRole::Cloud => 1,
        HldNodeRole::Controller => 2,
        HldNodeRole::Runtime => 3,
        HldNodeRole::Peripheral | HldNodeRole::Storage | HldNodeRole::Connectivity => 4,
        HldNodeRole::Alert => 5,
        HldNodeRole::Internet => 6,
    }
}

fn hld_role_name(role: HldNodeRole) -> &'static str {
    match role {
        HldNodeRole::User => "user",
        HldNodeRole::Cloud => "cloud",
        HldNodeRole::Controller => "controller",
        HldNodeRole::Peripheral => "peripheral",
        HldNodeRole::Storage => "storage",
        HldNodeRole::Connectivity => "connectivity",
        HldNodeRole::Runtime => "runtime",
        HldNodeRole::Alert => "alert",
        HldNodeRole::Internet => "internet",
    }
}

fn is_cloud_client_node_type(node_type: &str) -> bool {
    matches!(
        node_type,
        "mqtt_client" | "http_client" | "websocket_client" | "ota_update"
    )
}

fn is_wifi_like(node_type: &str, label: &str) -> bool {
    node_type == "wifi_manager" || label.to_ascii_lowercase().contains("wifi")
}

fn hld_module_role(node: &PirNode) -> Option<HldNodeRole> {
    let node_type = node.node_type.as_str();
    let lower_label = node_display_label(node).to_ascii_lowercase();

    if node_type == "system_init" || node.id == "boot" {
        return None;
    }
    if node_type == "storage_manager" {
        return Some(HldNodeRole::Storage);
    }
    if node_type == "wifi_manager" || node_type == "ble_manager" {
        return Some(HldNodeRole::Connectivity);
    }
    if is_cloud_client_node_type(node_type) {
        return None;
    }
    if lower_label.contains("alert")
        || lower_label.contains("notify")
        || lower_label.contains("notification")
        || lower_label.contains("alarm")
        || lower_label.contains("buzzer")
    {
        return Some(HldNodeRole::Alert);
    }
    if is_peripheral_like_node_type(node_type)
        || matches!(
            node_type,
            "gpio_input"
                | "gpio_output"
                | "sensor_input"
                | "pwm_output"
                | "adc_reader"
                | "i2c_device"
                | "spi_device"
                | "uart_device"
                | "camera_capture"
                | "display_output"
        )
    {
        return Some(HldNodeRole::Peripheral);
    }
    if matches!(
        node_type,
        "rtos_task"
            | "event_handler"
            | "event_queue"
            | "timer"
            | "timer_node"
            | "signal_processing"
            | "edge_ml_inference"
            | "diagnostics"
            | "logger"
    ) {
        return Some(HldNodeRole::Runtime);
    }
    if let Some(def) = get_node_type_def(node_type) {
        return Some(match def.category.as_str() {
            "gpio" | "sensors" | "analog" | "communication" | "media" => HldNodeRole::Peripheral,
            "network" => HldNodeRole::Connectivity,
            "system" | "rtos" | "pipeline" => HldNodeRole::Runtime,
            _ => HldNodeRole::Runtime,
        });
    }
    Some(HldNodeRole::Runtime)
}

fn hld_fallback_role(node: &PirNode) -> HldNodeRole {
    if is_cloud_client_node_type(&node.node_type) {
        return HldNodeRole::Connectivity;
    }
    hld_module_role(node).unwrap_or(HldNodeRole::Runtime)
}

fn hld_runtime_rank(node_type: &str) -> u8 {
    match node_type {
        "rtos_task" => 0,
        "signal_processing" | "edge_ml_inference" => 1,
        "event_handler" | "event_queue" => 2,
        "timer" | "timer_node" => 3,
        "diagnostics" | "logger" => 4,
        _ => 5,
    }
}

fn hld_peripheral_rank(node_type: &str) -> u8 {
    match node_type {
        "sensor_input" | "adc_reader" | "gpio_input" => 0,
        "gpio_output" | "pwm_output" => 1,
        "i2c_device" | "spi_device" | "uart_device" => 2,
        "display_output" | "camera_capture" => 3,
        _ => 4,
    }
}

fn infer_cloud_exchange_label(cloud_clients: &[&PirNode]) -> String {
    let mut has_mqtt = false;
    let mut has_http = false;
    let mut has_ws = false;
    let mut has_ota = false;
    for n in cloud_clients {
        match n.node_type.as_str() {
            "mqtt_client" => has_mqtt = true,
            "http_client" => has_http = true,
            "websocket_client" => has_ws = true,
            "ota_update" => has_ota = true,
            _ => {}
        }
    }
    if has_mqtt && (has_http || has_ws) {
        "MQTT / API Messages".to_string()
    } else if has_mqtt {
        "MQTT Messages".to_string()
    } else if has_ws {
        "WebSocket Messages".to_string()
    } else if has_http {
        "API Requests".to_string()
    } else if has_ota {
        "OTA / Cloud Updates".to_string()
    } else {
        "Cloud Messages".to_string()
    }
}

fn infer_controller_module_label(role: HldNodeRole, node: &PirNode) -> String {
    match role {
        HldNodeRole::Peripheral => match node.node_type.as_str() {
            "sensor_input" | "adc_reader" | "gpio_input" => "Sensor Data".to_string(),
            "gpio_output" | "pwm_output" => "Control Signals".to_string(),
            _ => "Peripheral I/O".to_string(),
        },
        HldNodeRole::Storage => "Read / Write Config".to_string(),
        HldNodeRole::Connectivity => "Network Management".to_string(),
        HldNodeRole::Runtime => "Firmware Logic".to_string(),
        HldNodeRole::Alert => {
            let label = node_display_label(node).to_ascii_lowercase();
            if label.contains("temperature") {
                "Temperature Events".to_string()
            } else if label.contains("motion") {
                "Motion Events".to_string()
            } else {
                "Alert Events".to_string()
            }
        }
        _ => "Data Flow".to_string(),
    }
}

fn make_unique_hld_id(base: &str, used: &mut HashSet<String>) -> String {
    let mut candidate = base.to_string();
    let mut idx = 2usize;
    while !used.insert(candidate.clone()) {
        candidate = format!("{base}_{idx}");
        idx += 1;
    }
    candidate
}

fn build_hld_diagram_spec(pir: &PirDocument) -> HldDiagramSpec {
    let mut nodes_sorted: Vec<&PirNode> = pir.nodes.iter().collect();
    nodes_sorted.sort_by(|a, b| a.id.cmp(&b.id));

    let mut peripherals: Vec<&PirNode> = Vec::new();
    let mut storage: Vec<&PirNode> = Vec::new();
    let mut connectivity: Vec<&PirNode> = Vec::new();
    let mut runtime: Vec<&PirNode> = Vec::new();
    let mut alerts: Vec<&PirNode> = Vec::new();
    let mut cloud_clients: Vec<&PirNode> = Vec::new();

    for node in &nodes_sorted {
        if is_cloud_client_node_type(&node.node_type) {
            cloud_clients.push(*node);
            continue;
        }
        match hld_module_role(node) {
            Some(HldNodeRole::Peripheral) => peripherals.push(*node),
            Some(HldNodeRole::Storage) => storage.push(*node),
            Some(HldNodeRole::Connectivity) => connectivity.push(*node),
            Some(HldNodeRole::Runtime) => runtime.push(*node),
            Some(HldNodeRole::Alert) => alerts.push(*node),
            _ => {}
        }
    }

    peripherals.sort_by(|a, b| {
        hld_peripheral_rank(&a.node_type)
            .cmp(&hld_peripheral_rank(&b.node_type))
            .then_with(|| a.id.cmp(&b.id))
    });
    runtime.sort_by(|a, b| {
        hld_runtime_rank(&a.node_type)
            .cmp(&hld_runtime_rank(&b.node_type))
            .then_with(|| a.id.cmp(&b.id))
    });

    let mut selected_modules: Vec<(&PirNode, HldNodeRole)> = Vec::new();
    let mut used_source_ids = HashSet::<String>::new();
    for node in peripherals.iter().take(4) {
        if used_source_ids.insert(node.id.clone()) {
            selected_modules.push((*node, HldNodeRole::Peripheral));
        }
    }
    for node in storage.iter().take(2) {
        if used_source_ids.insert(node.id.clone()) {
            selected_modules.push((*node, HldNodeRole::Storage));
        }
    }
    for node in connectivity.iter().take(2) {
        if used_source_ids.insert(node.id.clone()) {
            selected_modules.push((*node, HldNodeRole::Connectivity));
        }
    }
    for node in runtime.iter().take(2) {
        if used_source_ids.insert(node.id.clone()) {
            selected_modules.push((*node, HldNodeRole::Runtime));
        }
    }
    for node in alerts.iter().take(2) {
        if used_source_ids.insert(node.id.clone()) {
            selected_modules.push((*node, HldNodeRole::Alert));
        }
    }

    if selected_modules.is_empty() {
        for node in nodes_sorted
            .iter()
            .filter(|n| n.node_type != "system_init")
            .take(4)
        {
            if used_source_ids.insert(node.id.clone()) {
                selected_modules.push((*node, hld_fallback_role(node)));
            }
        }
    }

    let has_cloud = !cloud_clients.is_empty();
    let has_wifi = connectivity
        .iter()
        .any(|n| is_wifi_like(&n.node_type, &node_display_label(n)))
        || selected_modules.iter().any(|(n, role)| {
            *role == HldNodeRole::Connectivity && is_wifi_like(&n.node_type, &node_display_label(n))
        });
    let has_internet = has_cloud || has_wifi;

    let mut spec = HldDiagramSpec::default();
    let mut used_ids = HashSet::<String>::new();
    let mut module_id_by_source = HashMap::<String, String>::new();

    let controller_id = make_unique_hld_id("hld_esp32", &mut used_ids);
    let mut user_id: Option<String> = None;
    let mut cloud_id: Option<String> = None;
    let mut internet_id: Option<String> = None;

    if has_cloud {
        let uid = make_unique_hld_id("hld_user", &mut used_ids);
        spec.nodes.push(HldNodeSpec {
            id: uid.clone(),
            label: "Mobile App / Web Dashboard".to_string(),
            role: HldNodeRole::User,
            source_node_id: None,
            source_node_type: None,
            summary: Some(
                "External user interface for monitoring data and sending configuration."
                    .to_string(),
            ),
        });
        user_id = Some(uid);
    }

    if has_cloud {
        let cid = make_unique_hld_id("hld_cloud", &mut used_ids);
        spec.nodes.push(HldNodeSpec {
            id: cid.clone(),
            label: if cloud_clients
                .iter()
                .any(|n| n.node_type.as_str() == "mqtt_client")
            {
                "MQTT Broker / Cloud Service".to_string()
            } else {
                "Cloud Service / API".to_string()
            },
            role: HldNodeRole::Cloud,
            source_node_id: None,
            source_node_type: None,
            summary: Some(
                "External cloud endpoint for command/control and telemetry exchange.".to_string(),
            ),
        });
        cloud_id = Some(cid);
    }

    spec.nodes.push(HldNodeSpec {
        id: controller_id.clone(),
        label: "ESP32 Main Controller".to_string(),
        role: HldNodeRole::Controller,
        source_node_id: None,
        source_node_type: Some("system_init".to_string()),
        summary: Some(
            "Main firmware control node coordinating peripherals, storage, and connectivity."
                .to_string(),
        ),
    });

    for (idx, (node, role)) in selected_modules.iter().enumerate() {
        let base = {
            let safe = mermaid_safe_id(&node.id);
            if safe.is_empty() {
                format!("hld_module_{}", idx + 1)
            } else {
                format!("hld_{}", safe)
            }
        };
        let module_id = make_unique_hld_id(&base, &mut used_ids);
        module_id_by_source.insert(node.id.clone(), module_id.clone());
        spec.nodes.push(HldNodeSpec {
            id: module_id,
            label: node_display_label(node),
            role: *role,
            source_node_id: Some(node.id.clone()),
            source_node_type: Some(node.node_type.clone()),
            summary: node.ai_summary.clone(),
        });
    }

    if has_internet {
        let iid = make_unique_hld_id("hld_internet", &mut used_ids);
        spec.nodes.push(HldNodeSpec {
            id: iid.clone(),
            label: "Internet".to_string(),
            role: HldNodeRole::Internet,
            source_node_id: None,
            source_node_type: None,
            summary: Some("External network backbone used for cloud transport.".to_string()),
        });
        internet_id = Some(iid);
    }

    let mut seen_edges = HashSet::<(String, String, String, bool)>::new();
    let mut push_edge = |source_id: &str, target_id: &str, label: String, bidirectional: bool| {
        if source_id == target_id {
            return;
        }
        let key = (
            source_id.to_string(),
            target_id.to_string(),
            label.clone(),
            bidirectional,
        );
        if !seen_edges.insert(key) {
            return;
        }
        spec.edges.push(HldEdgeSpec {
            source_id: source_id.to_string(),
            target_id: target_id.to_string(),
            label,
            bidirectional,
        });
    };

    if let (Some(uid), Some(cid)) = (user_id.as_deref(), cloud_id.as_deref()) {
        push_edge(uid, cid, "View Data / Configure".to_string(), true);
    }
    if let Some(cid) = cloud_id.as_deref() {
        push_edge(
            cid,
            &controller_id,
            infer_cloud_exchange_label(&cloud_clients),
            true,
        );
    }

    for (node, role) in &selected_modules {
        if let Some(module_id) = module_id_by_source.get(&node.id) {
            push_edge(
                &controller_id,
                module_id,
                infer_controller_module_label(*role, node),
                false,
            );
        }
    }

    let mut edges_sorted: Vec<&PirEdge> = pir.edges.iter().collect();
    edges_sorted.sort_by(|a, b| a.id.cmp(&b.id));
    for edge in edges_sorted {
        let Some(src_module_id) = module_id_by_source.get(&edge.source_node_id) else {
            continue;
        };
        let Some(dst_module_id) = module_id_by_source.get(&edge.target_node_id) else {
            continue;
        };
        if src_module_id == dst_module_id {
            continue;
        }
        let src_type = selected_modules
            .iter()
            .find(|(n, _)| n.id == edge.source_node_id)
            .map(|(n, _)| n.node_type.as_str())
            .unwrap_or("");
        let dst_type = selected_modules
            .iter()
            .find(|(n, _)| n.id == edge.target_node_id)
            .map(|(n, _)| n.node_type.as_str())
            .unwrap_or("");
        let src_label = selected_modules
            .iter()
            .find(|(n, _)| n.id == edge.source_node_id)
            .map(|(n, _)| node_display_label(n))
            .unwrap_or_else(|| edge.source_node_id.clone());
        let dst_label = selected_modules
            .iter()
            .find(|(n, _)| n.id == edge.target_node_id)
            .map(|(n, _)| node_display_label(n))
            .unwrap_or_else(|| edge.target_node_id.clone());
        let label = diagram_view_builders::infer_hld_interaction_label(
            src_type,
            dst_type,
            &src_label,
            &dst_label,
            Some(edge),
        );
        push_edge(src_module_id, dst_module_id, label, false);
    }

    if let Some(iid) = internet_id.as_deref() {
        for (node, role) in &selected_modules {
            if *role != HldNodeRole::Connectivity {
                continue;
            }
            if !is_wifi_like(&node.node_type, &node_display_label(node)) {
                continue;
            }
            if let Some(module_id) = module_id_by_source.get(&node.id) {
                push_edge(module_id, iid, "Internet Access".to_string(), false);
            }
        }
        if let Some(cid) = cloud_id.as_deref() {
            push_edge(cid, iid, "Cloud Route".to_string(), false);
        } else if has_wifi {
            push_edge(&controller_id, iid, "Internet Access".to_string(), false);
        }
    }

    let ordered_spec_ids: Vec<String> = spec.nodes.iter().map(|n| n.id.clone()).collect();
    let mut directed_spec_edges = HashSet::<(String, String)>::new();
    for edge in &spec.edges {
        directed_spec_edges.insert((edge.source_id.clone(), edge.target_id.clone()));
        if edge.bidirectional {
            directed_spec_edges.insert((edge.target_id.clone(), edge.source_id.clone()));
        }
    }
    for (src, dst) in connectivity_bridge_pairs(&ordered_spec_ids, &directed_spec_edges) {
        spec.edges.push(HldEdgeSpec {
            source_id: src,
            target_id: dst,
            label: "workflow_link".to_string(),
            bidirectional: false,
        });
    }

    spec
}

/// Generate HLD as Mermaid `graph TD` architecture view.
pub fn generate_hld_mermaid(pir: &PirDocument) -> String {
    let spec = build_hld_diagram_spec(pir);
    let mut lines: Vec<String> = Vec::new();
    lines.push("graph TD".to_string());

    let mut used_mermaid_ids = HashSet::<String>::new();
    let mut mermaid_ids = HashMap::<String, String>::new();
    for node in &spec.nodes {
        let base = {
            let safe = mermaid_safe_id(&node.id);
            if safe.is_empty() {
                "node".to_string()
            } else {
                safe
            }
        };
        let id = make_unique_hld_id(&base, &mut used_mermaid_ids);
        mermaid_ids.insert(node.id.clone(), id);
    }

    for node in &spec.nodes {
        let Some(id) = mermaid_ids.get(&node.id) else {
            continue;
        };
        if node.role == HldNodeRole::Internet {
            lines.push(format!("    {id}[(Internet)]"));
        } else {
            let label = mermaid_escape_label(&node.label);
            lines.push(format!("    {id}[\"{label}\"]"));
        }
    }

    for edge in &spec.edges {
        let Some(source) = mermaid_ids.get(&edge.source_id) else {
            continue;
        };
        let Some(target) = mermaid_ids.get(&edge.target_id) else {
            continue;
        };
        let label = mermaid_escape_label(&edge.label);
        if edge.bidirectional {
            lines.push(format!("    {source} <-->|{label}| {target}"));
        } else if label.trim().is_empty() {
            lines.push(format!("    {source} --> {target}"));
        } else {
            lines.push(format!("    {source} -->|{label}| {target}"));
        }
    }

    lines.join("\n")
}

fn is_peripheral_like_node_type(node_type: &str) -> bool {
    if let Some(def) = get_node_type_def(node_type) {
        return matches!(
            def.category.as_str(),
            "gpio" | "sensors" | "analog" | "communication" | "media"
        );
    }
    matches!(
        node_type,
        "gpio_input"
            | "gpio_output"
            | "sensor_input"
            | "pwm_output"
            | "adc_reader"
            | "spi_device"
            | "i2c_device"
            | "uart_device"
            | "display_output"
            | "camera_capture"
    )
}

fn is_freertos_workflow_node_type(node_type: &str) -> bool {
    matches!(
        node_type,
        "rtos_task" | "event_queue" | "event_handler" | "timer" | "timer_node"
    )
}

fn is_lld_workflow_node_type(node_type: &str) -> bool {
    if is_freertos_workflow_node_type(node_type) {
        return true;
    }
    if is_peripheral_like_node_type(node_type) {
        return true;
    }
    if let Some(def) = get_node_type_def(node_type) {
        if def.category == "network" {
            return true;
        }
    }
    matches!(
        node_type,
        "wifi_manager"
            | "mqtt_client"
            | "http_client"
            | "websocket_client"
            | "ble_manager"
            | "storage_manager"
            | "ota_update"
            | "diagnostics"
            | "logger"
    )
}

fn lld_workflow_node_rank(node_type: &str) -> u8 {
    match node_type {
        "rtos_task" => 0,
        "event_queue" | "event_handler" => 1,
        "timer" | "timer_node" => 2,
        "sensor_input" | "gpio_input" | "adc_reader" => 3,
        "gpio_output" | "pwm_output" => 4,
        "i2c_device" | "spi_device" | "uart_device" | "display_output" | "camera_capture" => 5,
        "storage_manager" => 6,
        "wifi_manager" | "http_client" | "websocket_client" | "ble_manager" => 7,
        "mqtt_client" => 8,
        "ota_update" => 9,
        _ => 10,
    }
}

fn lld_workflow_node_ids(pir: &PirDocument) -> HashSet<String> {
    pir.nodes
        .iter()
        .filter(|n| is_lld_workflow_node_type(&n.node_type))
        .map(|n| n.id.clone())
        .collect()
}

fn connectivity_bridge_pairs(
    node_ids: &[String],
    directed_edges: &HashSet<(String, String)>,
) -> Vec<(String, String)> {
    if node_ids.len() < 2 {
        return Vec::new();
    }

    let node_set: HashSet<&str> = node_ids.iter().map(|id| id.as_str()).collect();
    let mut adjacency: HashMap<String, HashSet<String>> = node_ids
        .iter()
        .map(|id| (id.clone(), HashSet::new()))
        .collect();
    for (src, dst) in directed_edges {
        if src == dst || !node_set.contains(src.as_str()) || !node_set.contains(dst.as_str()) {
            continue;
        }
        adjacency.entry(src.clone()).or_default().insert(dst.clone());
        adjacency.entry(dst.clone()).or_default().insert(src.clone());
    }

    let order: HashMap<String, usize> = node_ids
        .iter()
        .enumerate()
        .map(|(idx, id)| (id.clone(), idx))
        .collect();
    let mut visited = HashSet::<String>::new();
    let mut components: Vec<Vec<String>> = Vec::new();
    for node_id in node_ids {
        if visited.contains(node_id) {
            continue;
        }
        let mut queue = VecDeque::<String>::new();
        let mut component = Vec::<String>::new();
        visited.insert(node_id.clone());
        queue.push_back(node_id.clone());
        while let Some(cur) = queue.pop_front() {
            component.push(cur.clone());
            if let Some(neighbors) = adjacency.get(&cur) {
                for next in neighbors {
                    if visited.insert(next.clone()) {
                        queue.push_back(next.clone());
                    }
                }
            }
        }
        components.push(component);
    }

    if components.len() <= 1 {
        return Vec::new();
    }
    components.sort_by_key(|component| {
        component
            .iter()
            .filter_map(|id| order.get(id).copied())
            .min()
            .unwrap_or(usize::MAX)
    });

    let mut bridges = Vec::<(String, String)>::new();
    for idx in 1..components.len() {
        let prev_rep = components[idx - 1]
            .iter()
            .min_by_key(|id| order.get(*id).copied().unwrap_or(usize::MAX))
            .cloned();
        let cur_rep = components[idx]
            .iter()
            .min_by_key(|id| order.get(*id).copied().unwrap_or(usize::MAX))
            .cloned();
        let (Some(src), Some(dst)) = (prev_rep, cur_rep) else {
            continue;
        };
        if src != dst {
            bridges.push((src, dst));
        }
    }
    bridges
}

fn ensure_node_port_graph_connected<FOut, FIn>(
    node_ids: &[String],
    connections: &mut Vec<[String; 2]>,
    out_port: FOut,
    in_port: FIn,
) -> usize
where
    FOut: Fn(&str) -> String,
    FIn: Fn(&str) -> String,
{
    if node_ids.len() < 2 {
        return 0;
    }

    let mut out_lookup = HashMap::<String, String>::new();
    let mut in_lookup = HashMap::<String, String>::new();
    for node_id in node_ids {
        out_lookup.insert(out_port(node_id), node_id.clone());
        in_lookup.insert(in_port(node_id), node_id.clone());
    }

    let mut directed_pairs = HashSet::<(String, String)>::new();
    for [src_port, dst_port] in connections.iter() {
        let Some(src_node) = out_lookup.get(src_port) else {
            continue;
        };
        let Some(dst_node) = in_lookup.get(dst_port) else {
            continue;
        };
        if src_node != dst_node {
            directed_pairs.insert((src_node.clone(), dst_node.clone()));
        }
    }

    let bridges = connectivity_bridge_pairs(node_ids, &directed_pairs);
    if bridges.is_empty() {
        return 0;
    }

    let mut existing_connections = HashSet::<(String, String)>::new();
    for [src_port, dst_port] in connections.iter() {
        existing_connections.insert((src_port.clone(), dst_port.clone()));
    }

    let mut added = 0usize;
    for (src_node, dst_node) in bridges {
        let src_port = out_port(&src_node);
        let dst_port = in_port(&dst_node);
        if existing_connections.insert((src_port.clone(), dst_port.clone())) {
            connections.push([src_port, dst_port]);
            added += 1;
        }
    }
    added
}

/// Generate LLD as Mermaid workflow (FreeRTOS tasks/queues + peripherals/services).
pub fn generate_lld_mermaid(pir: &PirDocument) -> String {
    let mut lines: Vec<String> = Vec::new();
    lines.push("flowchart TD".to_string());

    let mut nodes_sorted: Vec<&PirNode> = pir
        .nodes
        .iter()
        .filter(|n| is_lld_workflow_node_type(&n.node_type))
        .collect();
    nodes_sorted.sort_by(|a, b| {
        lld_workflow_node_rank(&a.node_type)
            .cmp(&lld_workflow_node_rank(&b.node_type))
            .then_with(|| a.id.cmp(&b.id))
    });
    let ordered_ids: Vec<String> = nodes_sorted.iter().map(|n| n.id.clone()).collect();
    let workflow_ids: HashSet<String> = ordered_ids.iter().cloned().collect();

    if ordered_ids.is_empty() {
        lines.push("  no_workflow[\"No LLD workflow detected\"]".to_string());
        return lines.join("\n");
    }

    let has_freertos_nodes = nodes_sorted
        .iter()
        .any(|n| is_freertos_workflow_node_type(&n.node_type));
    if has_freertos_nodes {
        lines.push("  subgraph freertos_runtime[\"FreeRTOS\"]".to_string());
        for n in nodes_sorted
            .iter()
            .filter(|n| is_freertos_workflow_node_type(&n.node_type))
        {
            let class_id = mermaid_safe_id(&n.id);
            let display = mermaid_escape_label(&node_display_label(n));
            lines.push(format!("    {class_id}[\"{display}\"]"));
        }
        lines.push("  end".to_string());
    }

    for n in nodes_sorted
        .iter()
        .filter(|n| !is_freertos_workflow_node_type(&n.node_type))
    {
        let class_id = mermaid_safe_id(&n.id);
        let display = mermaid_escape_label(&node_display_label(n));
        lines.push(format!("  {class_id}[\"{display}\"]"));
    }

    let mut rels_seen = std::collections::HashSet::<(String, String)>::new();
    let mut directed_pairs = HashSet::<(String, String)>::new();
    let mut edges_sorted: Vec<&PirEdge> = pir.edges.iter().collect();
    edges_sorted.sort_by(|a, b| a.id.cmp(&b.id));
    for e in edges_sorted {
        if !workflow_ids.contains(&e.source_node_id) || !workflow_ids.contains(&e.target_node_id) {
            continue;
        }
        let s = mermaid_safe_id(&e.source_node_id);
        let t = mermaid_safe_id(&e.target_node_id);
        if s.is_empty() || t.is_empty() || s == t {
            continue;
        }
        let key = (s.clone(), t.clone());
        if !rels_seen.insert(key) {
            continue;
        }
        directed_pairs.insert((e.source_node_id.clone(), e.target_node_id.clone()));
        lines.push(format!("  {s} --> {t}"));
    }

    for (src, dst) in connectivity_bridge_pairs(&ordered_ids, &directed_pairs) {
        let s = mermaid_safe_id(&src);
        let t = mermaid_safe_id(&dst);
        if s.is_empty() || t.is_empty() || s == t {
            continue;
        }
        if rels_seen.insert((s.clone(), t.clone())) {
            lines.push(format!("  {s} --> {t}"));
        }
    }

    lines.join("\n")
}

pub fn generate_all(pir: &PirDocument) -> PirDiagrams {
    let base_graph = crate::pir_maker::builder::graph_from_pir(
        pir,
        pir.provenance
            .project_path
            .split(std::path::MAIN_SEPARATOR)
            .last()
            .unwrap_or("esp32_project"),
    );

    let hld_graph = apply_layout(generate_hld_graph(pir, &base_graph));
    let lld_graph = apply_layout(generate_lld_graph(pir, &base_graph));
    let sequence_graph = generate_sequence_graph(pir, &base_graph);

    let mut diagrams = pir.diagrams.clone().unwrap_or_default();

    let mut hld = diagrams.hld.unwrap_or_default();
    // HLD is backend-owned: always regenerate deterministic architecture view.
    hld.mermaid = Some(generate_hld_mermaid(pir));
    if hld.title.is_none() {
        hld.title = Some("High-Level Design".to_string());
    }

    let mut lld = diagrams.lld.unwrap_or_default();
    // LLD is backend-owned: always regenerate as deterministic runtime workflow.
    lld.mermaid = Some(generate_lld_mermaid(pir));
    if lld.title.is_none() {
        lld.title = Some("Low-Level Design (Firmware Workflow)".to_string());
    }

    let mut sequence = diagrams.sequence.unwrap_or(PirSequenceDiagram {
        title: Some("Sequence Diagram".to_string()),
        ..Default::default()
    });
    if sequence.title.is_none() {
        sequence.title = Some("Sequence Diagram".to_string());
    }
    if sequence.generated_from.is_empty() {
        sequence.generated_from = infer_sequence_generated_from(pir);
    }

    let mut chosen_mermaid = sequence.mermaid.clone();
    if let Some(code) = chosen_mermaid.as_deref() {
        match validate_sequence_mermaid(code) {
            Ok(participants)
                if sequence_participants_match_graph(&participants, &sequence_graph) => {}
            Ok(_) => {
                chosen_mermaid = None;
            }
            Err(_) => {
                chosen_mermaid = None;
            }
        }
    }
    if chosen_mermaid.is_none() {
        chosen_mermaid = generate_sequence_mermaid_fallback(pir, &sequence_graph);
    }

    match chosen_mermaid.as_deref() {
        Some(code) => match validate_sequence_mermaid(code) {
            Ok(participants) => {
                sequence.mermaid = Some(code.to_string());
                sequence.participants = participants;
                sequence.generation_error = None;
            }
            Err(err) => {
                sequence.mermaid = None;
                sequence.generation_error =
                    Some(format!("Sequence Mermaid validation failed: {}", err));
            }
        },
        None => {
            sequence.mermaid = None;
            sequence.generation_error = Some(
                "Sequence Mermaid could not be generated from PIR graph; run regeneration."
                    .to_string(),
            );
        }
    }

    diagrams.hld = Some(hld);
    diagrams.lld = Some(lld);
    diagrams.sequence = Some(sequence);
    diagrams.hld_graph = Some(hld_graph);
    diagrams.lld_graph = Some(lld_graph);
    diagrams.sequence_graph = Some(sequence_graph);

    diagrams
}

fn assoc_port_id(node_id: &str, flavor: &str, dir: &str) -> String {
    diagram_view_builders::assoc_port_id(node_id, flavor, dir)
}

fn apply_layout(mut graph: FirmwareGraph) -> FirmwareGraph {
    let layout = compute_layout(&graph, graph.layout.as_ref());
    for pos in layout.positions {
        if let Some(node) = graph.nodes.iter_mut().find(|n| n.id == pos.node_id) {
            let visual = node.visual.get_or_insert_with(Default::default);
            visual.x = Some(pos.x);
            visual.y = Some(pos.y);
            visual.layer = Some(pos.layer);
        }
    }
    graph
}

fn map_pir_node_by_id<'a>(nodes: &'a [PirNode]) -> HashMap<&'a str, &'a PirNode> {
    nodes.iter().map(|n| (n.id.as_str(), n)).collect()
}

fn map_ports_to_node(graph: &FirmwareGraph) -> HashMap<&str, &str> {
    let mut out = HashMap::new();
    for n in &graph.nodes {
        for p in &n.ports {
            out.insert(p.id.as_str(), n.id.as_str());
        }
    }
    out
}

fn remap_edge_labels(
    base: &FirmwareGraph,
    pir: &PirDocument,
    flavor: &str,
    key_name: &str,
) -> HashMap<String, String> {
    // Build label map keyed by "oldSrcPort|oldDstPort" using PIR edges when possible.
    let port_to_node = map_ports_to_node(base);

    let mut pir_edge_by_pair: HashMap<String, &PirEdge> = HashMap::new();
    for e in &pir.edges {
        pir_edge_by_pair.insert(format!("{}->{}", e.source_node_id, e.target_node_id), e);
    }

    let mut remapped: HashMap<String, String> = HashMap::new();
    for [src_port, dst_port] in &base.connections {
        let Some(src_node) = port_to_node.get(src_port.as_str()) else {
            continue;
        };
        let Some(dst_node) = port_to_node.get(dst_port.as_str()) else {
            continue;
        };
        if src_node == dst_node {
            continue;
        }
        let e = pir_edge_by_pair.get(&format!("{}->{}", src_node, dst_node));
        let lbl = e
            .map(|x| edge_label(*x))
            .unwrap_or_else(|| "associates".to_string());
        let new_src = assoc_port_id(src_node, flavor, "out");
        let new_dst = assoc_port_id(dst_node, flavor, "in");
        remapped.insert(format!("{}|{}", new_src, new_dst), lbl);
    }

    remapped
}

pub fn generate_hld_graph(pir: &PirDocument, base: &FirmwareGraph) -> FirmwareGraph {
    let spec = build_hld_diagram_spec(pir);
    let base_by_id: HashMap<&str, &FirmwareNode> =
        base.nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    let pir_by_id = map_pir_node_by_id(&pir.nodes);

    let mut nodes: Vec<FirmwareNode> = Vec::new();
    for node in &spec.nodes {
        let source_base = node
            .source_node_id
            .as_deref()
            .and_then(|id| base_by_id.get(id))
            .copied();
        let source_pir = node
            .source_node_id
            .as_deref()
            .and_then(|id| pir_by_id.get(id))
            .copied();

        let mut properties = source_base
            .map(|n| n.properties.clone())
            .unwrap_or_else(|| json!({}));
        if !properties.is_object() {
            properties = json!({ "value": properties });
        }
        if let Some(obj) = properties.as_object_mut() {
            obj.remove("hld_component");
            obj.insert(
                "hld_role".to_string(),
                serde_json::Value::String(hld_role_name(node.role).to_string()),
            );
            if let Some(source_id) = node.source_node_id.as_deref() {
                obj.insert(
                    "source_node_id".to_string(),
                    serde_json::Value::String(source_id.to_string()),
                );
            }
        }

        let node_type = source_base
            .map(|n| n.node_type.clone())
            .or_else(|| node.source_node_type.clone())
            .unwrap_or_else(|| match node.role {
                HldNodeRole::Controller => "system_init".to_string(),
                HldNodeRole::Cloud => "cloud_service".to_string(),
                HldNodeRole::User => "user_actor".to_string(),
                HldNodeRole::Internet => "internet".to_string(),
                HldNodeRole::Peripheral => "peripheral".to_string(),
                HldNodeRole::Storage => "storage_manager".to_string(),
                HldNodeRole::Connectivity => "connectivity".to_string(),
                HldNodeRole::Runtime => "runtime".to_string(),
                HldNodeRole::Alert => "notification_service".to_string(),
            });

        let description = node
            .summary
            .clone()
            .or_else(|| source_base.and_then(|n| n.description.clone()))
            .or_else(|| match node.role {
                HldNodeRole::Controller => Some(
                    "Central firmware controller coordinating application services.".to_string(),
                ),
                HldNodeRole::Cloud => {
                    Some("External cloud endpoint for telemetry and remote commands.".to_string())
                }
                HldNodeRole::User => Some(
                    "External mobile/web client used for monitoring and configuration.".to_string(),
                ),
                HldNodeRole::Internet => {
                    Some("External internet route between edge and cloud systems.".to_string())
                }
                _ => None,
            });

        nodes.push(FirmwareNode {
            id: node.id.clone(),
            node_type,
            label: Some(node.label.clone()),
            description,
            ports: vec![
                Port {
                    id: assoc_port_id(&node.id, "hld", "in"),
                    name: "assoc_in".to_string(),
                    direction: PortDirection::Input,
                    datatype: Some("association".to_string()),
                    signal: None,
                    hardware: None,
                    required: false,
                    multiplicity: "one".to_string(),
                },
                Port {
                    id: assoc_port_id(&node.id, "hld", "out"),
                    name: "assoc_out".to_string(),
                    direction: PortDirection::Output,
                    datatype: Some("association".to_string()),
                    signal: None,
                    hardware: None,
                    required: false,
                    multiplicity: "one".to_string(),
                },
            ],
            properties,
            hardware: source_base.and_then(|n| n.hardware.clone()),
            execution: source_base.and_then(|n| n.execution.clone()),
            visual: Some(VisualMetadata {
                x: source_base.and_then(|n| n.visual.as_ref().and_then(|v| v.x)),
                y: source_base.and_then(|n| n.visual.as_ref().and_then(|v| v.y)),
                layer: Some(hld_role_layer(node.role)),
                collapsed: source_base.and_then(|n| n.visual.as_ref().and_then(|v| v.collapsed)),
            }),
            validation_state: source_base.and_then(|n| n.validation_state.clone()),
            runtime_state: source_base.and_then(|n| n.runtime_state.clone()),
        });
        let _ = source_pir;
    }

    let mut connections: Vec<[String; 2]> = Vec::new();
    let mut seen_connections = HashSet::<(String, String)>::new();
    let mut label_map = serde_json::Map::new();
    let mut add_connection = |src: &str, dst: &str, label: &str| {
        if src == dst {
            return;
        }
        let src_port = assoc_port_id(src, "hld", "out");
        let dst_port = assoc_port_id(dst, "hld", "in");
        if seen_connections.insert((src_port.clone(), dst_port.clone())) {
            connections.push([src_port.clone(), dst_port.clone()]);
            label_map.insert(
                format!("{src_port}|{dst_port}"),
                serde_json::Value::String(label.to_string()),
            );
        }
    };

    for edge in &spec.edges {
        add_connection(&edge.source_id, &edge.target_id, &edge.label);
        if edge.bidirectional {
            add_connection(&edge.target_id, &edge.source_id, &edge.label);
        }
    }
    let hld_node_ids: Vec<String> = nodes.iter().map(|n| n.id.clone()).collect();
    ensure_node_port_graph_connected(
        &hld_node_ids,
        &mut connections,
        |id| assoc_port_id(id, "hld", "out"),
        |id| assoc_port_id(id, "hld", "in"),
    );

    let mut overlays: HashMap<String, serde_json::Value> = HashMap::new();
    overlays.insert(
        "hld_edge_labels".to_string(),
        serde_json::Value::Object(label_map),
    );
    overlays.insert(
        "diagram_view".to_string(),
        serde_json::Value::String("hld".to_string()),
    );
    overlays.insert(
        "hld_projection".to_string(),
        serde_json::Value::String("architecture".to_string()),
    );

    FirmwareGraph {
        schema_version: SCHEMA_VERSION,
        id: Some(format!(
            "{}-hld",
            base.id.clone().unwrap_or_else(|| "pir".to_string())
        )),
        name: Some(format!(
            "{} — HLD",
            base.name.clone().unwrap_or_else(|| "Firmware".to_string())
        )),
        description: Some(
            "High-level architecture — ESP32 controller, firmware modules, and integrations"
                .to_string(),
        ),
        board_id: base.board_id.clone(),
        nodes,
        connections,
        layout: Some(LayoutConfig {
            orientation: "horizontal".to_string(),
            node_width: 280.0,
            layer_gap: 150.0,
            node_gap: 64.0,
        }),
        runtime_metadata: Some(RuntimeMetadata {
            telemetry_enabled: false,
            last_updated_ms: None,
            overlays,
        }),
    }
}

pub fn generate_lld_graph(pir: &PirDocument, base: &FirmwareGraph) -> FirmwareGraph {
    let workflow_ids = lld_workflow_node_ids(pir);
    let pir_by_id = map_pir_node_by_id(&pir.nodes);
    let mut filtered_base_nodes: Vec<FirmwareNode> = base
        .nodes
        .iter()
        .filter(|n| workflow_ids.contains(&n.id))
        .cloned()
        .collect();
    filtered_base_nodes.sort_by(|a, b| {
        let rank_a = lld_workflow_node_rank(&a.node_type);
        let rank_b = lld_workflow_node_rank(&b.node_type);
        let layer_a = a.visual.as_ref().and_then(|v| v.layer).unwrap_or(u32::MAX);
        let layer_b = b.visual.as_ref().and_then(|v| v.layer).unwrap_or(u32::MAX);
        let x_a = a.visual.as_ref().and_then(|v| v.x).unwrap_or(f64::MAX);
        let x_b = b.visual.as_ref().and_then(|v| v.x).unwrap_or(f64::MAX);
        rank_a.cmp(&rank_b).then(
            layer_a
                .cmp(&layer_b)
                .then_with(|| x_a.partial_cmp(&x_b).unwrap_or(Ordering::Equal))
                .then_with(|| a.id.cmp(&b.id)),
        )
    });

    let mut nodes: Vec<FirmwareNode> = Vec::new();
    for n in &filtered_base_nodes {
        let pn = pir_by_id.get(n.id.as_str()).copied();
        let label = pn
            .and_then(|x| x.label.clone())
            .or_else(|| n.label.clone())
            .unwrap_or_else(|| n.node_type.clone());
        let mut workflow_properties = n.properties.clone();
        if let Some(props) = workflow_properties.as_object_mut() {
            props.insert(
                "lld_workflow".to_string(),
                json!({
                        "role": if is_freertos_workflow_node_type(&n.node_type) { "freertos" } else if is_peripheral_like_node_type(&n.node_type) { "peripheral" } else { "service" },
                    "node_type": n.node_type
                }),
            );
        }

        nodes.push(FirmwareNode {
            id: n.id.clone(),
            node_type: n.node_type.clone(),
            label: Some(label),
            description: pn
                .and_then(|x| x.ai_summary.clone())
                .or_else(|| n.description.clone()),
            ports: vec![
                Port {
                    id: assoc_port_id(&n.id, "ldd", "in"),
                    name: "assoc_in".to_string(),
                    direction: PortDirection::Input,
                    datatype: Some("association".to_string()),
                    signal: None,
                    hardware: None,
                    required: false,
                    multiplicity: "one".to_string(),
                },
                Port {
                    id: assoc_port_id(&n.id, "ldd", "out"),
                    name: "assoc_out".to_string(),
                    direction: PortDirection::Output,
                    datatype: Some("association".to_string()),
                    signal: None,
                    hardware: None,
                    required: false,
                    multiplicity: "one".to_string(),
                },
            ],
            properties: workflow_properties,
            hardware: n.hardware.clone(),
            execution: n.execution.clone(),
            visual: n.visual.clone(),
            validation_state: n.validation_state.clone(),
            runtime_state: n.runtime_state.clone(),
        });
    }

    let port_to_node = map_ports_to_node(base);
    let mut seen_pair = std::collections::HashSet::<(String, String)>::new();
    let mut base_connections_filtered: Vec<[String; 2]> = Vec::new();
    for [src_port, dst_port] in &base.connections {
        let Some(src_node) = port_to_node.get(src_port.as_str()) else {
            continue;
        };
        let Some(dst_node) = port_to_node.get(dst_port.as_str()) else {
            continue;
        };
        if src_node == dst_node
            || !workflow_ids.contains(*src_node)
            || !workflow_ids.contains(*dst_node)
        {
            continue;
        }
        if !seen_pair.insert(((*src_node).to_string(), (*dst_node).to_string())) {
            continue;
        }
        base_connections_filtered.push([src_port.clone(), dst_port.clone()]);
    }

    let mut base_for_labels = base.clone();
    base_for_labels.nodes = filtered_base_nodes;
    base_for_labels.connections = base_connections_filtered.clone();

    let labels = build_ldd_edge_labels(&base_for_labels, pir, "ldd");
    let mut overlays: HashMap<String, serde_json::Value> = HashMap::new();
    let mut label_map = serde_json::Map::new();
    for [src_port, dst_port] in &base_for_labels.connections {
        let Some(src_node) = port_to_node.get(src_port.as_str()) else {
            continue;
        };
        let Some(dst_node) = port_to_node.get(dst_port.as_str()) else {
            continue;
        };
        let new_src = assoc_port_id(src_node, "ldd", "out");
        let new_dst = assoc_port_id(dst_node, "ldd", "in");
        if !labels.contains_key(&format!("{new_src}|{new_dst}")) {
            label_map.insert(
                format!("{new_src}|{new_dst}"),
                serde_json::Value::String("workflow".to_string()),
            );
        }
    }
    for (k, v) in labels {
        label_map.insert(k, serde_json::Value::String(v));
    }
    overlays.insert(
        "ldd_edge_labels".to_string(),
        serde_json::Value::Object(label_map),
    );
    overlays.insert(
        "diagram_view".to_string(),
        serde_json::Value::String("ldd".to_string()),
    );

    let mut connections: Vec<[String; 2]> = base_for_labels
        .connections
        .iter()
        .filter_map(|[src_port, dst_port]| {
            let src_node = port_to_node.get(src_port.as_str())?;
            let dst_node = port_to_node.get(dst_port.as_str())?;
            Some([
                assoc_port_id(src_node, "ldd", "out"),
                assoc_port_id(dst_node, "ldd", "in"),
            ])
        })
        .collect();
    let lld_node_ids: Vec<String> = nodes.iter().map(|n| n.id.clone()).collect();
    ensure_node_port_graph_connected(
        &lld_node_ids,
        &mut connections,
        |id| assoc_port_id(id, "ldd", "out"),
        |id| assoc_port_id(id, "ldd", "in"),
    );

    FirmwareGraph {
        schema_version: SCHEMA_VERSION,
        id: Some(format!(
            "{}-lld",
            base.id.clone().unwrap_or_else(|| "pir".to_string())
        )),
        name: Some(format!(
            "{} — LLD",
            base.name.clone().unwrap_or_else(|| "Firmware".to_string())
        )),
        description: Some(
            "Low-level design — FreeRTOS and peripheral/service workflow".to_string(),
        ),
        board_id: base.board_id.clone(),
        nodes,
        connections,
        layout: Some(LayoutConfig {
            orientation: "vertical".to_string(),
            node_width: 280.0,
            layer_gap: 120.0,
            node_gap: 48.0,
        }),
        runtime_metadata: Some(RuntimeMetadata {
            telemetry_enabled: false,
            last_updated_ms: None,
            overlays,
        }),
    }
}

pub fn generate_sequence_graph(pir: &PirDocument, base: &FirmwareGraph) -> FirmwareGraph {
    let order = sequence_node_order(pir);
    let node_by_id: HashMap<&str, &FirmwareNode> =
        base.nodes.iter().map(|n| (n.id.as_str(), n)).collect();

    let mut seq_nodes: Vec<FirmwareNode> = Vec::new();
    for (idx, node_id) in order.iter().enumerate() {
        let Some(n) = node_by_id.get(node_id.as_str()) else {
            continue;
        };
        let mut nn = (*n).clone();
        nn.label = Some(format!(
            "{}. {}",
            idx + 1,
            n.label.clone().unwrap_or_else(|| n.node_type.clone())
        ));
        nn.ports = vec![
            Port {
                id: format!("{}::in", n.id),
                name: "in".to_string(),
                direction: PortDirection::Input,
                datatype: Some("any".to_string()),
                signal: None,
                hardware: None,
                required: false,
                multiplicity: "one".to_string(),
            },
            Port {
                id: format!("{}::out", n.id),
                name: "out".to_string(),
                direction: PortDirection::Output,
                datatype: Some("any".to_string()),
                signal: None,
                hardware: None,
                required: false,
                multiplicity: "one".to_string(),
            },
        ];
        nn.visual = Some(VisualMetadata {
            x: Some(idx as f64 * 320.0),
            y: Some(80.0),
            layer: Some(idx as u32),
            collapsed: None,
        });
        seq_nodes.push(nn);
    }

    let node_pos: HashMap<String, usize> = seq_nodes
        .iter()
        .enumerate()
        .map(|(idx, n)| (n.id.clone(), idx))
        .collect();
    let node_type_by_id: HashMap<&str, &str> = pir
        .nodes
        .iter()
        .map(|n| (n.id.as_str(), n.node_type.as_str()))
        .collect();

    let mut edge_links: Vec<(usize, usize, String, String)> = Vec::new();
    for edge in &pir.edges {
        let relevant = matches!(
            edge.kind,
            PirEdgeKind::Execution
                | PirEdgeKind::Data
                | PirEdgeKind::Event
                | PirEdgeKind::Hardware
                | PirEdgeKind::Network
        );
        if !relevant {
            continue;
        }

        let src_type = node_type_by_id
            .get(edge.source_node_id.as_str())
            .copied()
            .unwrap_or("");
        let dst_type = node_type_by_id
            .get(edge.target_node_id.as_str())
            .copied()
            .unwrap_or("");
        if edge.kind == PirEdgeKind::Execution
            && src_type == "rtos_task"
            && matches!(dst_type, "sensor_input" | "gpio_input" | "adc_reader")
        {
            // Task->sensor exec edges are often init/poll wiring and tend to invert runtime flow.
            continue;
        }

        let Some(src_idx) = node_pos.get(&edge.source_node_id) else {
            continue;
        };
        let Some(dst_idx) = node_pos.get(&edge.target_node_id) else {
            continue;
        };
        if src_idx == dst_idx {
            continue;
        }
        edge_links.push((
            *src_idx,
            *dst_idx,
            edge.source_node_id.clone(),
            edge.target_node_id.clone(),
        ));
    }
    edge_links.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));

    let mut seen_pairs = HashSet::<(String, String)>::new();
    let mut connections: Vec<[String; 2]> = Vec::new();
    for (_src_idx, _dst_idx, src, dst) in edge_links {
        if !seen_pairs.insert((src.clone(), dst.clone())) {
            continue;
        }
        connections.push([format!("{}::out", src), format!("{}::in", dst)]);
    }

    let sequence_node_ids: Vec<String> = seq_nodes.iter().map(|n| n.id.clone()).collect();
    ensure_node_port_graph_connected(
        &sequence_node_ids,
        &mut connections,
        |id| format!("{id}::out"),
        |id| format!("{id}::in"),
    );

    for (idx, node) in seq_nodes.iter_mut().enumerate() {
        let base_label = node
            .label
            .as_deref()
            .map(|label| {
                label
                    .split_once(". ")
                    .map(|(_, tail)| tail.trim())
                    .filter(|tail| !tail.is_empty())
                    .unwrap_or(label)
                    .to_string()
            })
            .unwrap_or_else(|| node.node_type.clone());
        node.label = Some(format!("{}. {}", idx + 1, base_label));

        let visual = node.visual.get_or_insert_with(Default::default);
        visual.x = Some(idx as f64 * 320.0);
        visual.layer = Some(idx as u32);
    }

    FirmwareGraph {
        schema_version: SCHEMA_VERSION,
        id: Some(format!(
            "{}-sequence",
            base.id.clone().unwrap_or_else(|| "pir".to_string())
        )),
        name: Some(format!(
            "{} — Sequence",
            base.name.clone().unwrap_or_else(|| "Firmware".to_string())
        )),
        description: Some("Execution and initialization order".to_string()),
        board_id: base.board_id.clone(),
        nodes: seq_nodes,
        connections,
        layout: Some(LayoutConfig {
            orientation: "horizontal".to_string(),
            ..Default::default()
        }),
        runtime_metadata: base.runtime_metadata.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        generate_hld_graph, generate_hld_mermaid, generate_lld_graph, generate_lld_mermaid,
        generate_sequence_graph, generate_sequence_mermaid_fallback,
        sequence_participants_match_graph,
        validate_sequence_mermaid,
    };
    use crate::firmware_topology::types::{
        FirmwareGraph, FirmwareNode, Port, PortDirection, SCHEMA_VERSION,
    };
    use crate::pir_maker::schema::{
        FileOwnership, NodeAuthority, PirApproval, PirDocument, PirEdge, PirEdgeKind, PirLayers,
        PirNode, PirNodeSync, PirProvenance, PirSyncState,
    };
    use serde_json::json;
    use std::collections::HashSet;

    fn test_pir() -> PirDocument {
        PirDocument {
            schema_version: "2.0.0".to_string(),
            id: "pir_demo".to_string(),
            revision: "r1".to_string(),
            provenance: PirProvenance {
                project_path: "demo".to_string(),
                chat_id: None,
                revision: "r1".to_string(),
                generated_at_ms: 0,
                analyzer_version: "test".to_string(),
                analyzed_files: vec![],
                file_hashes: std::collections::HashMap::new(),
                board_id: None,
            },
            approval: PirApproval::default(),
            nodes: vec![],
            edges: vec![],
            layers: PirLayers::default(),
            partitions: vec![],
            components: vec![],
            summary: None,
            diagrams: None,
            change_log: vec![],
            unresolved: vec![],
            graph_version: 0,
            generation: Default::default(),
            sync_metadata: Default::default(),
            validation_state: Default::default(),
        }
    }

    fn test_pir_node(id: &str, node_type: &str, label: &str) -> PirNode {
        PirNode {
            id: id.to_string(),
            node_type: node_type.to_string(),
            label: Some(label.to_string()),
            properties: json!({}),
            source_refs: vec![],
            sync: PirNodeSync {
                state: PirSyncState::Synced,
                last_synced_revision: String::new(),
                last_error: None,
            },
            ownership: FileOwnership {
                primary_files: vec!["main/main.c".to_string()],
                component_id: None,
            },
            editable_fields: vec![],
            layer: None,
            ai_summary: Some(format!("{label} summary")),
            confidence: 1.0,
            authority: NodeAuthority::Agent,
            semantic_tags: vec![],
            dependencies: vec![],
            stale_reason: None,
        }
    }

    fn test_pir_edge(id: &str, src: &str, dst: &str, kind: PirEdgeKind) -> PirEdge {
        PirEdge {
            id: id.to_string(),
            source_node_id: src.to_string(),
            target_node_id: dst.to_string(),
            source_port_id: None,
            target_port_id: None,
            kind,
            confidence: 1.0,
            source_refs: vec![],
            semantic_label: Some("workflow".to_string()),
            validated: false,
        }
    }

    fn test_fw_node(id: &str, node_type: &str, label: &str) -> FirmwareNode {
        FirmwareNode {
            id: id.to_string(),
            node_type: node_type.to_string(),
            label: Some(label.to_string()),
            description: None,
            ports: vec![
                Port {
                    id: format!("{id}::in"),
                    name: "in".to_string(),
                    direction: PortDirection::Input,
                    datatype: Some("any".to_string()),
                    signal: None,
                    hardware: None,
                    required: false,
                    multiplicity: "one".to_string(),
                },
                Port {
                    id: format!("{id}::out"),
                    name: "out".to_string(),
                    direction: PortDirection::Output,
                    datatype: Some("any".to_string()),
                    signal: None,
                    hardware: None,
                    required: false,
                    multiplicity: "one".to_string(),
                },
            ],
            properties: json!({}),
            hardware: None,
            execution: None,
            visual: None,
            validation_state: None,
            runtime_state: None,
        }
    }

    #[test]
    fn validates_common_esp32_project_sequence_patterns() {
        let cases = [
            // Blink LED
            r#"sequenceDiagram
participant Main as Main Task
participant Led as GPIO LED
Main->>Led: Configure output pin
loop blink
    Main->>Led: Toggle level
end"#,
            // WiFi + MQTT
            r#"sequenceDiagram
participant Main as Main Controller
participant WiFi as WiFi Manager
participant MQTT as MQTT Client
Main->>WiFi: Start WiFi
WiFi-->>Main: Connected
Main->>MQTT: Connect broker
MQTT-->>Main: Session up"#,
            // Sensor + Display
            r#"sequenceDiagram
participant Task as Sensor Task
participant ADC as ADC Reader
participant Display as LCD Driver
Task->>ADC: Read sensor value
ADC-->>Task: Raw sample
Task->>Display: Render measurement"#,
            // OTA
            r#"sequenceDiagram
participant Main as Main
participant WiFi as WiFi
participant OTA as OTA Agent
Main->>WiFi: Connect
WiFi-->>Main: Ready
Main->>OTA: Check update
OTA-->>Main: Downloaded
Main->>OTA: Reboot to new partition"#,
            // Multi-task FreeRTOS
            r#"sequenceDiagram
participant Boot as app_main
participant SenseTask as Sensor Task
participant ControlTask as Control Task
Boot->>SenseTask: xTaskCreate
Boot->>ControlTask: xTaskCreate
loop runtime
    SenseTask-->>ControlTask: Queue sensor event
    ControlTask->>SenseTask: Ack processed
end"#,
        ];

        for case in cases {
            let participants = validate_sequence_mermaid(case).expect("expected valid sequence");
            assert!(participants.len() >= 2, "participants should be inferred");
        }
    }

    #[test]
    fn lld_workflow_includes_freertos_peripherals_and_services() {
        let mut pir = test_pir();
        pir.nodes = vec![
            test_pir_node("boot", "system_init", "Boot"),
            test_pir_node("app_task", "rtos_task", "Task"),
            test_pir_node("oled", "spi_device", "OLED"),
            test_pir_node("status_led", "gpio_output", "Status LED"),
            test_pir_node("wifi", "wifi_manager", "WiFi"),
        ];
        pir.edges = vec![
            test_pir_edge("e_boot_task", "boot", "app_task", PirEdgeKind::Execution),
            test_pir_edge("e_task_oled", "app_task", "oled", PirEdgeKind::Execution),
            test_pir_edge("e_oled_led", "oled", "status_led", PirEdgeKind::Hardware),
            test_pir_edge("e_task_wifi", "app_task", "wifi", PirEdgeKind::Network),
        ];

        let lld = generate_lld_mermaid(&pir);
        assert!(
            lld.starts_with("flowchart TD"),
            "LLD Mermaid must be workflow-style flowchart"
        );
        assert!(
            lld.contains("subgraph freertos_runtime[\"FreeRTOS\"]"),
            "LLD should contain FreeRTOS workflow lane"
        );
        assert!(lld.contains("app_task"), "runtime task should be present");
        assert!(lld.contains("oled"), "peripheral node should be present");
        assert!(
            lld.contains("status_led"),
            "peripheral gpio node should be present"
        );
        assert!(
            lld.contains("wifi"),
            "connectivity workflow node should be present"
        );
        assert!(
            !lld.contains("boot[\""),
            "boot node should not appear in LLD workflow"
        );

        let base = FirmwareGraph {
            schema_version: SCHEMA_VERSION,
            id: Some("pir_demo".to_string()),
            name: Some("Demo".to_string()),
            description: None,
            board_id: None,
            nodes: vec![
                test_fw_node("boot", "system_init", "Boot"),
                test_fw_node("app_task", "rtos_task", "Task"),
                test_fw_node("oled", "spi_device", "OLED"),
                test_fw_node("status_led", "gpio_output", "Status LED"),
                test_fw_node("wifi", "wifi_manager", "WiFi"),
            ],
            connections: vec![
                ["boot::out".to_string(), "app_task::in".to_string()],
                ["app_task::out".to_string(), "oled::in".to_string()],
                ["oled::out".to_string(), "status_led::in".to_string()],
                ["app_task::out".to_string(), "wifi::in".to_string()],
            ],
            layout: None,
            runtime_metadata: None,
        };
        let lld_graph = generate_lld_graph(&pir, &base);
        let lld_ids: HashSet<String> = lld_graph.nodes.iter().map(|n| n.id.clone()).collect();
        assert_eq!(
            lld_ids,
            HashSet::from([
                "app_task".to_string(),
                "oled".to_string(),
                "status_led".to_string(),
                "wifi".to_string()
            ]),
            "LLD graph should keep runtime/peripheral/service workflow nodes"
        );
        let is_workflow_port = |port: &str| {
            port.starts_with("app_task::ldd::")
                || port.starts_with("oled::ldd::")
                || port.starts_with("status_led::ldd::")
                || port.starts_with("wifi::ldd::")
        };
        assert!(
            lld_graph
                .connections
                .iter()
                .all(|[src, dst]| is_workflow_port(src) && is_workflow_port(dst)),
            "LLD connections must link workflow nodes only"
        );
    }

    #[test]
    fn lld_graph_bridges_disconnected_workflow_nodes() {
        let mut pir = test_pir();
        pir.nodes = vec![
            test_pir_node("task_a", "rtos_task", "Task A"),
            test_pir_node("queue_a", "event_queue", "Queue A"),
            test_pir_node("wifi", "wifi_manager", "WiFi"),
        ];
        pir.edges = vec![test_pir_edge(
            "e_task_queue",
            "task_a",
            "queue_a",
            PirEdgeKind::Data,
        )];

        let base = FirmwareGraph {
            schema_version: SCHEMA_VERSION,
            id: Some("pir_demo".to_string()),
            name: Some("Demo".to_string()),
            description: None,
            board_id: None,
            nodes: vec![
                test_fw_node("task_a", "rtos_task", "Task A"),
                test_fw_node("queue_a", "event_queue", "Queue A"),
                test_fw_node("wifi", "wifi_manager", "WiFi"),
            ],
            connections: vec![["task_a::out".to_string(), "queue_a::in".to_string()]],
            layout: None,
            runtime_metadata: None,
        };
        let lld_graph = generate_lld_graph(&pir, &base);
        let node_ids: HashSet<String> = lld_graph.nodes.iter().map(|n| n.id.clone()).collect();
        let touched_ids: HashSet<String> = lld_graph
            .connections
            .iter()
            .flat_map(|[src, dst]| [src, dst])
            .filter_map(|port| port.split_once("::ldd::").map(|(id, _)| id.to_string()))
            .collect();
        assert_eq!(
            touched_ids, node_ids,
            "every LLD workflow node should participate in at least one connection"
        );
    }

    #[test]
    fn hld_mermaid_renders_architecture_style_esp32_view() {
        let mut pir = test_pir();
        pir.nodes = vec![
            test_pir_node("boot", "system_init", "Boot"),
            test_pir_node("sensor_th", "sensor_input", "Temperature & Humidity Sensor"),
            test_pir_node(
                "storage_cfg",
                "storage_manager",
                "Local Configuration Storage",
            ),
            test_pir_node("wifi", "wifi_manager", "WiFi Manager"),
            test_pir_node("mqtt", "mqtt_client", "MQTT Client"),
            test_pir_node("alerts", "event_handler", "Notification Service"),
        ];
        pir.edges = vec![
            test_pir_edge("e_sensor_alert", "sensor_th", "alerts", PirEdgeKind::Event),
            test_pir_edge("e_boot_wifi", "boot", "wifi", PirEdgeKind::Execution),
        ];

        let hld = generate_hld_mermaid(&pir);
        assert!(
            hld.starts_with("graph TD"),
            "HLD Mermaid should use architecture graph TD"
        );
        assert!(hld.contains("Mobile App / Web Dashboard"));
        assert!(hld.contains("MQTT Broker / Cloud Service"));
        assert!(hld.contains("ESP32 Main Controller"));
        assert!(hld.contains("Temperature & Humidity Sensor"));
        assert!(hld.contains("Local Configuration Storage"));
        assert!(hld.contains("WiFi Manager"));
        assert!(hld.contains("Notification Service"));
        assert!(hld.contains("Internet"));
        assert!(hld.contains("<-->|View Data / Configure|"));
        assert!(hld.contains("<-->|MQTT Messages|"));
    }

    #[test]
    fn hld_graph_contains_architecture_overlay_labels() {
        let mut pir = test_pir();
        pir.nodes = vec![
            test_pir_node("boot", "system_init", "Boot"),
            test_pir_node("sensor_th", "sensor_input", "Temperature & Humidity Sensor"),
            test_pir_node(
                "storage_cfg",
                "storage_manager",
                "Local Configuration Storage",
            ),
            test_pir_node("wifi", "wifi_manager", "WiFi Manager"),
            test_pir_node("mqtt", "mqtt_client", "MQTT Client"),
        ];
        pir.edges = vec![test_pir_edge(
            "e_sensor_wifi",
            "sensor_th",
            "wifi",
            PirEdgeKind::Data,
        )];

        let base = FirmwareGraph {
            schema_version: SCHEMA_VERSION,
            id: Some("pir_demo".to_string()),
            name: Some("Demo".to_string()),
            description: None,
            board_id: None,
            nodes: vec![
                test_fw_node("boot", "system_init", "Boot"),
                test_fw_node("sensor_th", "sensor_input", "Temperature & Humidity Sensor"),
                test_fw_node(
                    "storage_cfg",
                    "storage_manager",
                    "Local Configuration Storage",
                ),
                test_fw_node("wifi", "wifi_manager", "WiFi Manager"),
                test_fw_node("mqtt", "mqtt_client", "MQTT Client"),
            ],
            connections: vec![
                ["boot::out".to_string(), "wifi::in".to_string()],
                ["sensor_th::out".to_string(), "wifi::in".to_string()],
            ],
            layout: None,
            runtime_metadata: None,
        };

        let hld_graph = generate_hld_graph(&pir, &base);
        let labels: HashSet<String> = hld_graph
            .nodes
            .iter()
            .filter_map(|n| n.label.clone())
            .collect();
        assert!(labels.contains("ESP32 Main Controller"));
        assert!(labels.contains("MQTT Broker / Cloud Service"));
        assert!(labels.contains("Mobile App / Web Dashboard"));
        assert!(labels.contains("Internet"));
        assert!(
            hld_graph
                .nodes
                .iter()
                .all(|n| n.properties.get("hld_component").is_none()),
            "HLD graph should be architecture blocks, not class-style nodes"
        );
        let overlays = hld_graph
            .runtime_metadata
            .as_ref()
            .expect("runtime metadata should exist")
            .overlays
            .get("hld_edge_labels")
            .and_then(|v| v.as_object())
            .expect("HLD edge labels overlay should exist");
        let overlay_values: Vec<&str> = overlays.values().filter_map(|v| v.as_str()).collect();
        assert!(overlay_values.contains(&"View Data / Configure"));
        assert!(overlay_values.contains(&"MQTT Messages"));
    }

    #[test]
    fn rejects_sequence_without_header() {
        let err = validate_sequence_mermaid("participant A\nparticipant B\nA->>B: ping")
            .expect_err("missing header must fail");
        assert!(err.contains("sequenceDiagram"));
    }

    #[test]
    fn rejects_messages_for_unknown_participant() {
        let err = validate_sequence_mermaid(
            r#"sequenceDiagram
participant A
participant B
A->>C: invalid receiver"#,
        )
        .expect_err("unknown participant must fail");
        assert!(err.contains("declared participant"));
    }

    #[test]
    fn rejects_sequence_without_messages() {
        let err = validate_sequence_mermaid(
            r#"sequenceDiagram
participant Boot
participant Worker"#,
        )
        .expect_err("sequence without message interactions must fail");
        assert!(err.contains("participant interaction"));
    }

    #[test]
    fn rejects_disconnected_participants() {
        let err = validate_sequence_mermaid(
            r#"sequenceDiagram
participant Boot
participant Worker
participant MQTT
Boot->>Worker: init
Worker-->>Boot: ready"#,
        )
        .expect_err("isolated participant must fail");
        assert!(err.contains("disconnected participants"));
    }

    #[test]
    fn generates_valid_fallback_sequence_from_graph() {
        let pir = test_pir();
        let graph = FirmwareGraph {
            schema_version: SCHEMA_VERSION,
            id: Some("pir_demo".to_string()),
            name: Some("Demo".to_string()),
            description: None,
            board_id: None,
            nodes: vec![
                FirmwareNode {
                    id: "boot".to_string(),
                    node_type: "system_init".to_string(),
                    label: Some("Boot / app_main".to_string()),
                    description: None,
                    ports: vec![Port {
                        id: "boot::out".to_string(),
                        name: "out".to_string(),
                        direction: PortDirection::Output,
                        datatype: Some("execution".to_string()),
                        signal: None,
                        hardware: None,
                        required: false,
                        multiplicity: "one".to_string(),
                    }],
                    properties: json!({}),
                    hardware: None,
                    execution: None,
                    visual: None,
                    validation_state: None,
                    runtime_state: None,
                },
                FirmwareNode {
                    id: "servo_task".to_string(),
                    node_type: "rtos_task".to_string(),
                    label: Some("Servo Task".to_string()),
                    description: None,
                    ports: vec![
                        Port {
                            id: "servo_task::in".to_string(),
                            name: "in".to_string(),
                            direction: PortDirection::Input,
                            datatype: Some("execution".to_string()),
                            signal: None,
                            hardware: None,
                            required: false,
                            multiplicity: "one".to_string(),
                        },
                        Port {
                            id: "servo_task::out".to_string(),
                            name: "out".to_string(),
                            direction: PortDirection::Output,
                            datatype: Some("execution".to_string()),
                            signal: None,
                            hardware: None,
                            required: false,
                            multiplicity: "one".to_string(),
                        },
                    ],
                    properties: json!({}),
                    hardware: None,
                    execution: None,
                    visual: None,
                    validation_state: None,
                    runtime_state: None,
                },
                FirmwareNode {
                    id: "oled".to_string(),
                    node_type: "spi_device".to_string(),
                    label: Some("OLED".to_string()),
                    description: None,
                    ports: vec![Port {
                        id: "oled::in".to_string(),
                        name: "in".to_string(),
                        direction: PortDirection::Input,
                        datatype: Some("execution".to_string()),
                        signal: None,
                        hardware: None,
                        required: false,
                        multiplicity: "one".to_string(),
                    }],
                    properties: json!({}),
                    hardware: None,
                    execution: None,
                    visual: None,
                    validation_state: None,
                    runtime_state: None,
                },
            ],
            connections: vec![
                ["boot::out".to_string(), "servo_task::in".to_string()],
                ["servo_task::out".to_string(), "oled::in".to_string()],
            ],
            layout: None,
            runtime_metadata: None,
        };

        let code =
            generate_sequence_mermaid_fallback(&pir, &graph).expect("fallback should be generated");
        let participants =
            validate_sequence_mermaid(&code).expect("fallback sequence must validate");
        assert_eq!(participants.len(), 3);
    }

    #[test]
    fn sequence_graph_connects_all_nodes() {
        let mut pir = test_pir();
        pir.nodes = vec![
            test_pir_node("task_a", "rtos_task", "Task A"),
            test_pir_node("queue_a", "event_queue", "Queue A"),
            test_pir_node("mqtt", "mqtt_client", "MQTT"),
        ];
        pir.edges = vec![test_pir_edge(
            "e_task_queue",
            "task_a",
            "queue_a",
            PirEdgeKind::Data,
        )];

        let base = FirmwareGraph {
            schema_version: SCHEMA_VERSION,
            id: Some("pir_demo".to_string()),
            name: Some("Demo".to_string()),
            description: None,
            board_id: None,
            nodes: vec![
                test_fw_node("task_a", "rtos_task", "Task A"),
                test_fw_node("queue_a", "event_queue", "Queue A"),
                test_fw_node("mqtt", "mqtt_client", "MQTT"),
            ],
            connections: vec![["task_a::out".to_string(), "queue_a::in".to_string()]],
            layout: None,
            runtime_metadata: None,
        };

        let sequence_graph = generate_sequence_graph(&pir, &base);
        let node_ids: HashSet<String> = sequence_graph.nodes.iter().map(|n| n.id.clone()).collect();
        let touched_ids: HashSet<String> = sequence_graph
            .connections
            .iter()
            .flat_map(|[src, dst]| [src, dst])
            .filter_map(|port| port.split_once("::").map(|(id, _)| id.to_string()))
            .collect();
        assert_eq!(
            touched_ids, node_ids,
            "every sequence node should be connected in the resulting graph"
        );
    }

    #[test]
    fn omits_disconnected_participants_in_fallback_sequence() {
        let pir = test_pir();
        let graph = FirmwareGraph {
            schema_version: SCHEMA_VERSION,
            id: Some("pir_demo".to_string()),
            name: Some("Demo".to_string()),
            description: None,
            board_id: None,
            nodes: vec![
                FirmwareNode {
                    id: "boot".to_string(),
                    node_type: "system_init".to_string(),
                    label: Some("Boot".to_string()),
                    description: None,
                    ports: vec![Port {
                        id: "boot::out".to_string(),
                        name: "out".to_string(),
                        direction: PortDirection::Output,
                        datatype: Some("execution".to_string()),
                        signal: None,
                        hardware: None,
                        required: false,
                        multiplicity: "one".to_string(),
                    }],
                    properties: json!({}),
                    hardware: None,
                    execution: None,
                    visual: None,
                    validation_state: None,
                    runtime_state: None,
                },
                FirmwareNode {
                    id: "task".to_string(),
                    node_type: "rtos_task".to_string(),
                    label: Some("Task".to_string()),
                    description: None,
                    ports: vec![Port {
                        id: "task::in".to_string(),
                        name: "in".to_string(),
                        direction: PortDirection::Input,
                        datatype: Some("execution".to_string()),
                        signal: None,
                        hardware: None,
                        required: false,
                        multiplicity: "one".to_string(),
                    }],
                    properties: json!({}),
                    hardware: None,
                    execution: None,
                    visual: None,
                    validation_state: None,
                    runtime_state: None,
                },
                FirmwareNode {
                    id: "orphan".to_string(),
                    node_type: "gpio_output".to_string(),
                    label: Some("Orphan LED".to_string()),
                    description: None,
                    ports: vec![],
                    properties: json!({}),
                    hardware: None,
                    execution: None,
                    visual: None,
                    validation_state: None,
                    runtime_state: None,
                },
            ],
            connections: vec![["boot::out".to_string(), "task::in".to_string()]],
            layout: None,
            runtime_metadata: None,
        };

        let code =
            generate_sequence_mermaid_fallback(&pir, &graph).expect("fallback should be generated");
        let participants =
            validate_sequence_mermaid(&code).expect("fallback sequence must validate");
        assert_eq!(participants, vec!["boot".to_string(), "task".to_string()]);
        assert!(!code.contains("participant orphan"));
    }

    #[test]
    fn rejects_sequence_participants_not_backed_by_graph_nodes() {
        let graph = FirmwareGraph {
            schema_version: SCHEMA_VERSION,
            id: Some("pir_demo".to_string()),
            name: Some("Demo".to_string()),
            description: None,
            board_id: None,
            nodes: vec![
                FirmwareNode {
                    id: "boot".to_string(),
                    node_type: "system_init".to_string(),
                    label: Some("Boot".to_string()),
                    description: None,
                    ports: vec![],
                    properties: json!({}),
                    hardware: None,
                    execution: None,
                    visual: None,
                    validation_state: None,
                    runtime_state: None,
                },
                FirmwareNode {
                    id: "app_task".to_string(),
                    node_type: "rtos_task".to_string(),
                    label: Some("Task".to_string()),
                    description: None,
                    ports: vec![],
                    properties: json!({}),
                    hardware: None,
                    execution: None,
                    visual: None,
                    validation_state: None,
                    runtime_state: None,
                },
            ],
            connections: vec![],
            layout: None,
            runtime_metadata: None,
        };

        assert!(sequence_participants_match_graph(
            &["boot".to_string(), "app_task".to_string()],
            &graph
        ));
        assert!(!sequence_participants_match_graph(
            &["boot".to_string(), "oled_spi_bus".to_string()],
            &graph
        ));
    }
}
