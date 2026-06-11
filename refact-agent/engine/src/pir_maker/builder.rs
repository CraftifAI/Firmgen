//! Build PIR document and FirmwareGraph from AnalysisFacts.

use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::path::Path;

use serde_json::json;

use crate::firmware_topology::{
    compute_layout,
    registry::{
        are_datatypes_compatible, default_ports_for_type, default_properties_for_type,
        get_node_type_def,
    },
    FirmwareGraph, FirmwareNode, HardwareMetadata, SCHEMA_VERSION, ValidationReport,
};
use crate::firmware_topology::types::{ExecutionMetadata, Port, PortDirection};

use super::analyzer::ANALYZER_VERSION;
use super::diagrams;
use super::schema::{
    AnalysisFacts, NodeAuthority, PirApproval, PirApprovalStatus, PirDocument, PirEdge,
    PirEdgeKind, PirLayers, PirNode, PirNodeSync, PirProvenance, PirSummary, PirSyncState,
    PirValidationState, SourceRef, FileOwnership, TopologyDiff,
};

/// Minimum node confidence (0–1) for inclusion in the topology graph.
/// Static-extracted nodes use 1.0; user/hybrid nodes are always kept.
pub const PIR_GRAPH_MIN_NODE_CONFIDENCE: f32 = 0.90;
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

pub fn build_pir_and_graph(
    facts: &AnalysisFacts,
    project_path: &Path,
    chat_id: Option<&str>,
    revision: &str,
    previous: Option<&PirDocument>,
) -> (
    PirDocument,
    FirmwareGraph,
    ValidationReport,
    Option<TopologyDiff>,
) {
    let mut pir_nodes: Vec<PirNode> = Vec::new();
    let mut firmware_nodes: Vec<FirmwareNode> = Vec::new();
    let mut port_pairs: Vec<[String; 2]> = Vec::new();
    let mut pir_edges: Vec<PirEdge> = Vec::new();

    let mut layer_physical = Vec::new();
    let mut layer_runtime = Vec::new();
    let mut layer_network = Vec::new();
    let mut layer_system = Vec::new();

    if facts.has_app_main {
        let boot = make_node(
            "boot",
            "system_init",
            "Boot / app_main",
            json!({"target": facts.target_chip.clone().unwrap_or_else(|| "esp32".to_string())}),
            None,
            facts.app_main_file.clone(),
            vec!["target".to_string()],
            "system",
        );
        push_pair(&mut pir_nodes, &mut firmware_nodes, &mut layer_system, boot);
    }

    for g in &facts.gpio_facts {
        let props = json!({"pin": g.pin});
        let hw = HardwareMetadata {
            gpio: Some(g.pin),
            bus: Some("gpio".to_string()),
            peripheral: None,
            pin_label: Some(g.label.clone()),
            i2c_address: None,
            spi_host: None,
            uart_port: None,
        };
        let n = make_node(
            &g.node_id,
            &g.node_type,
            &g.label,
            props,
            Some(hw),
            Some(g.file.clone()),
            vec!["pin".to_string()],
            "physical",
        );
        push_pair(&mut pir_nodes, &mut firmware_nodes, &mut layer_physical, n);
    }

    for t in &facts.task_facts {
        let props = json!({
            "task_name": t.task_name,
            "priority": t.priority.unwrap_or(5),
            "stack_size": t.stack_size.unwrap_or(4096),
            "period_ms": t.period_ms.unwrap_or(0.0),
        });
        let n = make_node(
            &t.node_id,
            "rtos_task",
            &format!("Task {}", t.task_name),
            props,
            None,
            Some(t.file.clone()),
            vec![
                "task_name".to_string(),
                "priority".to_string(),
                "stack_size".to_string(),
                "period_ms".to_string(),
            ],
            "runtime",
        );
        push_pair(&mut pir_nodes, &mut firmware_nodes, &mut layer_runtime, n);
    }

    for net in &facts.network_facts {
        let layer_name = infer_layer_for_node_type(&net.node_type);
        let mut props = default_properties_for_type(&net.node_type);
        merge_properties(&mut props, &net.properties);
        let editable = network_editable_fields(&net.node_type);
        let n = make_node(
            &net.node_id,
            &net.node_type,
            &net.label,
            props,
            None,
            Some(net.file.clone()),
            editable,
            layer_name,
        );
        push_pair_with_layer(
            &mut pir_nodes,
            &mut firmware_nodes,
            &mut layer_physical,
            &mut layer_runtime,
            &mut layer_network,
            &mut layer_system,
            n,
        );
    }

    wire_default_connections(&firmware_nodes, &mut port_pairs, &mut pir_edges);

    let project_name = facts.project_name.clone();
    let mut graph = FirmwareGraph {
        schema_version: SCHEMA_VERSION,
        id: Some(format!("pir_{}", project_name)),
        name: Some(project_name.clone()),
        description: Some(format!(
            "PIR_maker generated topology for {}",
            project_path.display()
        )),
        board_id: facts.board_id.clone(),
        nodes: firmware_nodes,
        connections: port_pairs,
        layout: Some(Default::default()),
        runtime_metadata: None,
    };
    let connectivity_edges_added = ensure_graph_nodes_connected(&mut graph);
    if connectivity_edges_added > 0 {
        debug_mode_log(
            "pir-graph-quality",
            "H10",
            "builder.rs:build_pir_and_graph",
            "connected disconnected graph components",
            serde_json::json!({
                "connectivity_edges_added": connectivity_edges_added,
                "nodes": graph.nodes.len(),
                "connections": graph.connections.len(),
            }),
        );
    }

    let layout = compute_layout(&graph, graph.layout.as_ref());
    for pos in layout.positions {
        if let Some(node) = graph.nodes.iter_mut().find(|n| n.id == pos.node_id) {
            let visual = node.visual.get_or_insert_with(Default::default);
            visual.x = Some(pos.x);
            visual.y = Some(pos.y);
            visual.layer = Some(pos.layer);
        }
    }

    let validation = crate::firmware_topology::validate_graph(&graph);

    let warnings: Vec<String> = validation
        .issues
        .iter()
        .filter(|i| i.severity == "warning")
        .map(|i| i.message.clone())
        .collect();

    let summary = PirSummary {
        headline: format!(
            "{} nodes, {} connections — {}",
            pir_nodes.len(),
            pir_edges.len(),
            if validation.valid {
                "valid"
            } else {
                "has validation issues"
            }
        ),
        node_count: pir_nodes.len() as u32,
        edge_count: pir_edges.len() as u32,
        warnings,
    };

    let approval = if let Some(prev) = previous {
        if prev.revision != revision {
            PirApproval {
                status: PirApprovalStatus::Stale,
                ..Default::default()
            }
        } else {
            prev.approval.clone()
        }
    } else {
        PirApproval::default()
    };

    let mut pir = PirDocument {
        schema_version: super::schema::PIR_SCHEMA_VERSION.to_string(),
        id: format!("pir_{}", project_name),
        revision: revision.to_string(),
        provenance: PirProvenance {
            project_path: project_path.to_string_lossy().to_string(),
            chat_id: chat_id.map(String::from),
            revision: revision.to_string(),
            generated_at_ms: now_ms(),
            analyzer_version: ANALYZER_VERSION.to_string(),
            analyzed_files: facts.analyzed_files.clone(),
            board_id: facts.board_id.clone(),
            file_hashes: facts.file_hashes.clone(),
        },
        approval,
        nodes: pir_nodes,
        edges: pir_edges,
        layers: PirLayers {
            physical: layer_physical,
            runtime: layer_runtime,
            network: layer_network,
            system: layer_system,
        },
        partitions: facts.partitions.clone(),
        components: facts.components.clone(),
        summary: Some(summary),
        diagrams: None,
        change_log: previous.map(|p| p.change_log.clone()).unwrap_or_default(),
        unresolved: facts.unresolved.clone(),
        graph_version: previous.map(|p| p.graph_version + 1).unwrap_or(1),
        generation: Default::default(),
        sync_metadata: Default::default(),
        validation_state: PirValidationState {
            valid: validation.valid,
            error_count: validation
                .issues
                .iter()
                .filter(|i| i.severity == "error")
                .count() as u32,
            warning_count: validation
                .issues
                .iter()
                .filter(|i| i.severity == "warning")
                .count() as u32,
            validated_at_ms: now_ms(),
        },
    };

    super::validation_lock::apply_editable_field_locks(&mut pir, &validation);

    // Backend owns diagram artifacts — always regenerate from current PIR graph.
    pir.diagrams = Some(diagrams::generate_all(&pir));

    let diff = previous.map(|prev| diff_documents(prev, &pir));
    (pir, graph, validation, diff)
}

struct BuiltNode {
    pir: PirNode,
    firmware: FirmwareNode,
    layer: String,
}

fn infer_layer_for_node_type(node_type: &str) -> &'static str {
    let category = get_node_type_def(node_type)
        .map(|d| d.category)
        .unwrap_or_default();
    match category.as_str() {
        "system" => "system",
        "rtos" => "runtime",
        "network" => "network",
        "gpio" | "sensors" | "analog" | "communication" | "media" => "physical",
        "pipeline" => "runtime",
        _ => "runtime",
    }
}

fn property_str<'a>(properties: &'a serde_json::Value, key: &str) -> Option<&'a str> {
    properties
        .get(key)
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
}

fn property_u64(properties: &serde_json::Value, key: &str) -> Option<u64> {
    properties.get(key).and_then(|v| v.as_u64())
}

fn collapse_spaces(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn shorten_summary(summary: &str) -> String {
    let normalized = collapse_spaces(summary).trim().to_string();
    if normalized.len() <= 120 {
        return normalized;
    }
    let mut truncated = normalized.chars().take(117).collect::<String>();
    truncated.push_str("...");
    truncated
}

fn fallback_node_ai_summary(
    node_type: &str,
    label: &str,
    properties: &serde_json::Value,
) -> String {
    let summary = match node_type {
        "system_init" => "Bootstraps firmware startup and initializes core subsystems.".to_string(),
        "rtos_task" => {
            let task_name = property_str(properties, "task_name").unwrap_or(label);
            format!("Executes RTOS loop logic for {task_name}.")
        }
        "gpio_output" => property_u64(properties, "pin")
            .map(|pin| format!("Drives GPIO {pin} as an output signal."))
            .unwrap_or_else(|| "Drives a digital output signal.".to_string()),
        "gpio_input" => property_u64(properties, "pin")
            .map(|pin| format!("Reads GPIO {pin} as an input signal."))
            .unwrap_or_else(|| "Reads a digital input signal.".to_string()),
        "sensor_input" => property_u64(properties, "pin")
            .map(|pin| format!("Samples sensor state from GPIO {pin}."))
            .unwrap_or_else(|| "Samples external sensor input events.".to_string()),
        "pwm_output" => property_u64(properties, "pin")
            .map(|pin| format!("Generates PWM output on GPIO {pin}."))
            .unwrap_or_else(|| "Generates PWM output for actuator control.".to_string()),
        "wifi_manager" => property_str(properties, "ssid")
            .map(|ssid| format!("Manages Wi-Fi connectivity for SSID `{ssid}`."))
            .unwrap_or_else(|| "Manages Wi-Fi connectivity and network state.".to_string()),
        "mqtt_client" => {
            let topic = property_str(properties, "topic");
            let broker = property_str(properties, "broker_url");
            match (topic, broker) {
                (Some(topic), Some(_)) => {
                    format!("Publishes and receives MQTT messages on `{topic}`.")
                }
                (Some(topic), None) => format!("Handles MQTT messaging on `{topic}`."),
                (None, Some(_)) => {
                    "Handles MQTT broker communication and message routing.".to_string()
                }
                (None, None) => "Handles MQTT publish/subscribe messaging.".to_string(),
            }
        }
        "i2c_device" => "Represents an I2C peripheral with mapped bus pins.".to_string(),
        "spi_device" => "Represents an SPI peripheral with mapped bus pins.".to_string(),
        "uart_device" => "Represents a UART peripheral with mapped serial pins.".to_string(),
        "adc_reader" => "Reads analog values from configured ADC input channels.".to_string(),
        "timer" | "timer_node" => {
            "Schedules periodic callbacks for time-based workflow.".to_string()
        }
        "event_queue" | "event_handler" => {
            "Queues and dispatches asynchronous firmware events.".to_string()
        }
        "ota_update" => "Performs over-the-air firmware update operations.".to_string(),
        "display_output" => "Drives display output through configured interface pins.".to_string(),
        "camera_capture" => {
            "Captures image frames from the configured camera interface.".to_string()
        }
        "ble_manager" => "Manages BLE advertising, connections, and data flow.".to_string(),
        "http_client" => "Performs outbound HTTP requests for remote services.".to_string(),
        "websocket_client" => "Maintains WebSocket communication for real-time data.".to_string(),
        "storage_manager" => "Persists runtime state and configuration data.".to_string(),
        "logger" => "Emits structured runtime logs for diagnostics.".to_string(),
        "diagnostics" => "Collects firmware health and diagnostic telemetry.".to_string(),
        _ => format!(
            "Represents {} in the firmware topology.",
            collapse_spaces(label)
        ),
    };
    shorten_summary(&summary)
}

fn ensure_node_ai_summaries(pir: &mut PirDocument) {
    for node in &mut pir.nodes {
        let current = node
            .ai_summary
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(shorten_summary);
        node.ai_summary = Some(current.unwrap_or_else(|| {
            fallback_node_ai_summary(
                &node.node_type,
                node.label.as_deref().unwrap_or(&node.id),
                &node.properties,
            )
        }));
    }
}

fn make_node(
    id: &str,
    node_type: &str,
    label: &str,
    properties: serde_json::Value,
    hardware: Option<HardwareMetadata>,
    primary_file: Option<String>,
    editable: Vec<String>,
    layer: &str,
) -> BuiltNode {
    let auto_summary = fallback_node_ai_summary(node_type, label, &properties);
    let refs: Vec<SourceRef> = primary_file
        .as_ref()
        .map(|f| SourceRef {
            file: f.clone(),
            line: None,
            symbol: None,
            confidence: 1.0,
            inferred_by: "static".to_string(),
        })
        .into_iter()
        .collect();

    let ownership = FileOwnership {
        primary_files: primary_file.into_iter().collect(),
        component_id: None,
    };

    let pir = PirNode {
        id: id.to_string(),
        node_type: node_type.to_string(),
        label: Some(label.to_string()),
        properties: properties.clone(),
        source_refs: refs,
        sync: PirNodeSync {
            state: PirSyncState::Synced,
            last_synced_revision: String::new(),
            last_error: None,
        },
        ownership,
        editable_fields: editable,
        layer: Some(layer.to_string()),
        ai_summary: Some(auto_summary.clone()),
        confidence: 1.0,
        authority: NodeAuthority::Agent,
        semantic_tags: Vec::new(),
        dependencies: Vec::new(),
        stale_reason: None,
    };

    let firmware = FirmwareNode {
        id: id.to_string(),
        node_type: node_type.to_string(),
        label: Some(label.to_string()),
        description: Some(auto_summary),
        ports: default_ports_for_type(node_type, id),
        properties,
        hardware,
        execution: Some(ExecutionMetadata {
            phase: None,
            priority: None,
            stack_size: None,
            core_affinity: None,
            period_ms: None,
            trigger: None,
        }),
        visual: None,
        validation_state: None,
        runtime_state: None,
    };

    BuiltNode {
        pir,
        firmware,
        layer: layer.to_string(),
    }
}

fn push_pair(
    pir_nodes: &mut Vec<PirNode>,
    firmware_nodes: &mut Vec<FirmwareNode>,
    layer: &mut Vec<String>,
    built: BuiltNode,
) {
    layer.push(built.pir.id.clone());
    pir_nodes.push(built.pir);
    firmware_nodes.push(built.firmware);
}

fn push_pair_with_layer(
    pir_nodes: &mut Vec<PirNode>,
    firmware_nodes: &mut Vec<FirmwareNode>,
    layer_physical: &mut Vec<String>,
    layer_runtime: &mut Vec<String>,
    layer_network: &mut Vec<String>,
    layer_system: &mut Vec<String>,
    built: BuiltNode,
) {
    match built.layer.as_str() {
        "system" => push_pair(pir_nodes, firmware_nodes, layer_system, built),
        "runtime" => push_pair(pir_nodes, firmware_nodes, layer_runtime, built),
        "network" => push_pair(pir_nodes, firmware_nodes, layer_network, built),
        _ => push_pair(pir_nodes, firmware_nodes, layer_physical, built),
    }
}

fn port_id(node_id: &str, port_name: &str, index: usize) -> String {
    format!("{}_{}_{}", node_id, port_name, index)
}

fn find_port_id(node: &FirmwareNode, port_name: &str) -> Option<String> {
    node.ports
        .iter()
        .enumerate()
        .find(|(_, p)| p.name == port_name)
        .map(|(i, _)| port_id(&node.id, port_name, i))
}

fn add_connection_with_edge(
    connections: &mut Vec<[String; 2]>,
    pir_edges: &mut Vec<PirEdge>,
    edge: PirEdge,
) {
    let (Some(src), Some(dst)) = (edge.source_port_id.as_ref(), edge.target_port_id.as_ref())
    else {
        return;
    };
    if connections
        .iter()
        .any(|pair| pair[0] == *src && pair[1] == *dst)
    {
        return;
    }
    connections.push([src.clone(), dst.clone()]);
    pir_edges.push(edge);
}

fn port_datatype(port: &Port) -> &str {
    port.datatype.as_deref().unwrap_or("any")
}

fn should_autowire_input_port(port: &Port) -> bool {
    matches!(
        port.name.as_str(),
        "exec_in" | "trigger_in" | "network_in" | "data_in" | "event_in"
    )
}

fn ports_connectable(source: &Port, target: &Port) -> bool {
    let source_dt = port_datatype(source);
    let target_dt = port_datatype(target);
    if are_datatypes_compatible(source_dt, target_dt) {
        return true;
    }
    // Soft bridge: signal/data-like outputs can drive trigger ports.
    if target.name == "trigger_in" && matches!(source_dt, "signal" | "payload" | "mqtt_message") {
        return true;
    }
    matches!(
        (source_dt, target_dt),
        ("signal", "event") | ("payload", "event")
    )
}

fn infer_edge_kind_for_ports(source: &Port, target: &Port) -> PirEdgeKind {
    let source_dt = port_datatype(source);
    let target_dt = port_datatype(target);
    if source_dt == "network"
        || target_dt == "network"
        || source.name.contains("network")
        || target.name.contains("network")
    {
        PirEdgeKind::Network
    } else if source_dt == "event" || target_dt == "event" || target.name.contains("trigger") {
        PirEdgeKind::Event
    } else if source_dt == "execution" || target_dt == "execution" {
        PirEdgeKind::Execution
    } else if source_dt == "gpio_level" || target_dt == "gpio_level" {
        PirEdgeKind::Hardware
    } else {
        PirEdgeKind::Data
    }
}

fn infer_edge_label(
    source_node: &FirmwareNode,
    target_node: &FirmwareNode,
    source_port: &Port,
    target_port: &Port,
    kind: &PirEdgeKind,
) -> String {
    match kind {
        PirEdgeKind::Network => "connects".to_string(),
        PirEdgeKind::Event => {
            if target_port.name.contains("trigger") {
                "triggers".to_string()
            } else {
                "signals".to_string()
            }
        }
        PirEdgeKind::Execution => {
            if source_node.id == "boot" {
                if target_node.node_type == "rtos_task" {
                    "spawns".to_string()
                } else {
                    "initializes".to_string()
                }
            } else {
                "controls".to_string()
            }
        }
        PirEdgeKind::Hardware => "drives".to_string(),
        PirEdgeKind::Dependency => "depends_on".to_string(),
        _ => {
            if source_port.name.contains("data") || target_port.name.contains("data") {
                "data_flow".to_string()
            } else {
                "feeds".to_string()
            }
        }
    }
}

fn node_category(node_type: &str) -> String {
    crate::firmware_topology::registry::get_node_type_def(node_type)
        .map(|d| d.category)
        .unwrap_or_default()
}

fn should_bootstrap_node(node: &FirmwareNode) -> bool {
    if node.id == "boot" {
        return false;
    }
    !matches!(
        node_category(&node.node_type).as_str(),
        "gpio" | "sensors" | "analog"
    )
}

fn is_gpio_interface_node(node: &FirmwareNode) -> bool {
    matches!(node.node_type.as_str(), "gpio_input" | "gpio_output")
}

fn node_pin_number(node: &FirmwareNode) -> Option<u8> {
    node.properties
        .get("pin")
        .and_then(|v| v.as_u64())
        .and_then(|n| u8::try_from(n).ok())
}

fn node_pin_bindings(node: &FirmwareNode) -> Vec<(String, u8)> {
    let mut out: Vec<(String, u8)> = Vec::new();
    if let Some(bindings) = node
        .properties
        .get("pin_bindings")
        .and_then(|v| v.as_object())
    {
        for (name, value) in bindings {
            if let Some(pin) = value.as_u64().and_then(|n| u8::try_from(n).ok()) {
                out.push((name.to_string(), pin));
            }
        }
    }

    if out.is_empty() {
        if let Some(props) = node.properties.as_object() {
            for (key, value) in props {
                if !key.ends_with("_pin") {
                    continue;
                }
                if let Some(pin) = value.as_u64().and_then(|n| u8::try_from(n).ok()) {
                    let binding = key.trim_end_matches("_pin").to_string();
                    out.push((binding, pin));
                }
            }
        }
    }

    if out.is_empty() {
        if let Some(pin) = node_pin_number(node) {
            out.push(("signal".to_string(), pin));
        }
    }

    out.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    out.dedup();
    out
}

fn has_edge_between(
    pir_edges: &[PirEdge],
    source_node_id: &str,
    target_node_id: &str,
    kinds: &[PirEdgeKind],
) -> bool {
    pir_edges.iter().any(|e| {
        e.source_node_id == source_node_id
            && e.target_node_id == target_node_id
            && kinds.iter().any(|k| k == &e.kind)
    })
}

fn score_source_for_input(
    source_node: &FirmwareNode,
    source_port: &Port,
    target_node: &FirmwareNode,
    target_port: &Port,
) -> i32 {
    if source_node.id == target_node.id || !ports_connectable(source_port, target_port) {
        return -1000;
    }

    let source_dt = port_datatype(source_port);
    let target_dt = port_datatype(target_port);
    let mut score = if source_dt == target_dt { 40 } else { 20 };

    if source_node.id == "boot" {
        if target_port.name == "exec_in" {
            score += 60;
        } else {
            score -= 120;
        }
        // Preserve startup hierarchy: prefer boot -> task/service, then task/service -> hardware.
        let dst_cat = node_category(&target_node.node_type);
        if matches!(dst_cat.as_str(), "gpio" | "analog" | "sensors") {
            score -= 90;
        }
    }

    if target_port.name == "trigger_in" {
        if source_port.name.contains("event") || source_dt == "event" {
            score += 35;
        }
        if source_port.name.contains("data")
            || matches!(source_dt, "signal" | "payload" | "mqtt_message")
        {
            score += 18;
        }
    }

    if target_port.name == "network_in" && source_port.name == "network_out" {
        score += 40;
    }

    if target_port.name == "exec_in" && source_node.node_type == "rtos_task" {
        score += 30;
    }

    let src_cat = node_category(&source_node.node_type);
    let dst_cat = node_category(&target_node.node_type);
    if target_port.name == "exec_in" && dst_cat == "sensors" {
        score -= 90;
    }
    if src_cat == "network" && dst_cat == "rtos" {
        score += 18;
    }
    if src_cat == "rtos" && matches!(dst_cat.as_str(), "gpio" | "analog") {
        score += 22;
    }
    if src_cat == "sensors" && dst_cat == "rtos" {
        score += 20;
    }

    score
}

fn infer_compatible_connections(
    nodes: &[FirmwareNode],
    connections: &mut Vec<[String; 2]>,
    pir_edges: &mut Vec<PirEdge>,
) {
    let mut connected_inputs: HashSet<String> = connections.iter().map(|c| c[1].clone()).collect();
    let mut total_autowired = 0usize;
    let mut boot_to_gpio_like = 0usize;
    let mut task_to_gpio_like = 0usize;
    let mut task_to_task = 0usize;
    let mut autowire_samples: Vec<serde_json::Value> = Vec::new();

    for target_node in nodes {
        for target_port in target_node
            .ports
            .iter()
            .filter(|p| matches!(p.direction, PortDirection::Input))
        {
            if !should_autowire_input_port(target_port)
                || connected_inputs.contains(&target_port.id)
            {
                continue;
            }

            let mut best_source: Option<(i32, &FirmwareNode, &Port)> = None;
            for source_node in nodes {
                for source_port in source_node
                    .ports
                    .iter()
                    .filter(|p| matches!(p.direction, PortDirection::Output))
                {
                    let score =
                        score_source_for_input(source_node, source_port, target_node, target_port);
                    if score < 20 {
                        continue;
                    }
                    let replace = best_source
                        .as_ref()
                        .map(|(best_score, _, _)| score > *best_score)
                        .unwrap_or(true);
                    if replace {
                        best_source = Some((score, source_node, source_port));
                    }
                }
            }

            let Some((_, source_node, source_port)) = best_source else {
                continue;
            };
            let kind = infer_edge_kind_for_ports(source_port, target_port);
            let label = infer_edge_label(source_node, target_node, source_port, target_port, &kind);
            let edge = PirEdge {
                id: format!(
                    "e_{}_{}_{}",
                    source_node.id, target_node.id, target_port.name
                ),
                source_node_id: source_node.id.clone(),
                target_node_id: target_node.id.clone(),
                source_port_id: Some(source_port.id.clone()),
                target_port_id: Some(target_port.id.clone()),
                kind,
                confidence: 0.78,
                source_refs: Vec::new(),
                semantic_label: Some(label),
                validated: false,
            };
            let dst_cat = node_category(&target_node.node_type);
            if source_node.id == "boot" && matches!(dst_cat.as_str(), "gpio" | "analog" | "sensors")
            {
                boot_to_gpio_like += 1;
            }
            if source_node.node_type == "rtos_task"
                && matches!(dst_cat.as_str(), "gpio" | "analog" | "sensors")
            {
                task_to_gpio_like += 1;
            }
            if source_node.node_type == "rtos_task" && target_node.node_type == "rtos_task" {
                task_to_task += 1;
            }
            if autowire_samples.len() < 8 {
                autowire_samples.push(serde_json::json!({
                    "source": source_node.id,
                    "source_type": source_node.node_type,
                    "target": target_node.id,
                    "target_type": target_node.node_type,
                    "target_port": target_port.name,
                    "edge_kind": format!("{:?}", edge.kind),
                    "label": edge.semantic_label,
                }));
            }
            add_connection_with_edge(connections, pir_edges, edge);
            total_autowired += 1;
            if connections.iter().any(|pair| pair[1] == target_port.id) {
                connected_inputs.insert(target_port.id.clone());
            }
        }
    }
    // #region agent log
    debug_mode_log(
        "pir-graph-quality",
        "H7",
        "builder.rs:infer_compatible_connections",
        "autowire relationship summary",
        serde_json::json!({
            "total_autowired": total_autowired,
            "boot_to_gpio_like": boot_to_gpio_like,
            "task_to_gpio_like": task_to_gpio_like,
            "task_to_task": task_to_task,
            "samples": autowire_samples,
        }),
    );
    // #endregion
}

fn add_task_control_edges(
    nodes: &[FirmwareNode],
    connections: &mut Vec<[String; 2]>,
    pir_edges: &mut Vec<PirEdge>,
) -> usize {
    let mut task_nodes: Vec<&FirmwareNode> = nodes
        .iter()
        .filter(|n| n.node_type == "rtos_task")
        .collect();
    if task_nodes.is_empty() {
        return 0;
    }
    task_nodes.sort_by(|a, b| a.id.cmp(&b.id));
    let primary_task = task_nodes[0];

    let mut existing_edge_ids: HashSet<String> = pir_edges.iter().map(|e| e.id.clone()).collect();
    let mut added = 0usize;
    for node in nodes {
        if node.id == primary_task.id
            || node.id == "boot"
            || node.node_type == "system_init"
            || is_gpio_interface_node(node)
            || node.node_type == "rtos_task"
        {
            continue;
        }
        if has_edge_between(
            pir_edges,
            &primary_task.id,
            &node.id,
            &[
                PirEdgeKind::Execution,
                PirEdgeKind::Data,
                PirEdgeKind::Event,
            ],
        ) {
            continue;
        }

        let source_port = resolve_default_output_port_id(
            &primary_task.node_type,
            &primary_task.id,
            &["exec_out", "data_out", "event_out"],
        );
        let target_port =
            resolve_default_input_port_id(&node.node_type, &node.id, &["exec_in", "data_in"]);
        let (Some(source_port_id), Some(target_port_id)) = (source_port, target_port) else {
            continue;
        };

        let edge_id = next_unique_edge_id(
            &mut existing_edge_ids,
            &format!("e_{}_{}_control", primary_task.id, node.id),
        );
        add_connection_with_edge(
            connections,
            pir_edges,
            PirEdge {
                id: edge_id,
                source_node_id: primary_task.id.clone(),
                target_node_id: node.id.clone(),
                source_port_id: Some(source_port_id),
                target_port_id: Some(target_port_id),
                kind: PirEdgeKind::Execution,
                confidence: 0.86,
                source_refs: Vec::new(),
                semantic_label: Some("controls".to_string()),
                validated: false,
            },
        );
        added += 1;
    }
    added
}

fn add_driver_dependency_edges(
    nodes: &[FirmwareNode],
    connections: &mut Vec<[String; 2]>,
    pir_edges: &mut Vec<PirEdge>,
) -> usize {
    let drivers_by_type: HashMap<&str, Vec<&FirmwareNode>> = {
        let mut map: HashMap<&str, Vec<&FirmwareNode>> = HashMap::new();
        for node in nodes {
            if matches!(
                node.node_type.as_str(),
                "spi_device" | "i2c_device" | "uart_device"
            ) {
                map.entry(node.node_type.as_str()).or_default().push(node);
            }
        }
        map
    };
    let mut existing_edge_ids: HashSet<String> = pir_edges.iter().map(|e| e.id.clone()).collect();
    let mut added = 0usize;

    for node in nodes {
        let interface = node
            .properties
            .get("interface")
            .and_then(|v| v.as_str())
            .map(|s| s.to_ascii_lowercase());
        let Some(interface) = interface else {
            continue;
        };

        let driver_type = match interface.as_str() {
            "spi" | "spi2" | "spi3" => "spi_device",
            "i2c" => "i2c_device",
            "uart" => "uart_device",
            _ => continue,
        };

        let Some(drivers) = drivers_by_type.get(driver_type) else {
            continue;
        };
        for driver in drivers {
            if node.id == driver.id {
                continue;
            }
            let edge_shape = if let (Some(source_port_id), Some(target_port_id)) = (
                resolve_default_output_port_id(
                    &node.node_type,
                    &node.id,
                    &["data_out", "exec_out", "event_out"],
                ),
                resolve_default_input_port_id(
                    &driver.node_type,
                    &driver.id,
                    &["data_in", "exec_in", "network_in"],
                ),
            ) {
                Some((
                    node.id.clone(),
                    driver.id.clone(),
                    source_port_id,
                    target_port_id,
                    "depends_on".to_string(),
                ))
            } else if let (Some(source_port_id), Some(target_port_id)) = (
                resolve_default_output_port_id(
                    &driver.node_type,
                    &driver.id,
                    &["data_out", "exec_out", "event_out"],
                ),
                resolve_default_input_port_id(
                    &node.node_type,
                    &node.id,
                    &["data_in", "exec_in", "network_in"],
                ),
            ) {
                Some((
                    driver.id.clone(),
                    node.id.clone(),
                    source_port_id,
                    target_port_id,
                    "provides_driver".to_string(),
                ))
            } else {
                None
            };
            let Some((source_id, target_id, source_port_id, target_port_id, semantic_label)) =
                edge_shape
            else {
                continue;
            };
            if has_edge_between(
                pir_edges,
                &source_id,
                &target_id,
                &[PirEdgeKind::Dependency],
            ) {
                continue;
            }

            let edge_id = next_unique_edge_id(
                &mut existing_edge_ids,
                &format!("e_{}_{}_depends", source_id, target_id),
            );
            add_connection_with_edge(
                connections,
                pir_edges,
                PirEdge {
                    id: edge_id,
                    source_node_id: source_id,
                    target_node_id: target_id,
                    source_port_id: Some(source_port_id),
                    target_port_id: Some(target_port_id),
                    kind: PirEdgeKind::Dependency,
                    confidence: 0.84,
                    source_refs: Vec::new(),
                    semantic_label: Some(semantic_label),
                    validated: false,
                },
            );
            added += 1;
        }
    }

    let driver_nodes: Vec<&FirmwareNode> = nodes
        .iter()
        .filter(|n| {
            matches!(
                n.node_type.as_str(),
                "spi_device" | "i2c_device" | "uart_device"
            )
        })
        .collect();
    for node in nodes {
        if node.id == "boot"
            || node.node_type == "system_init"
            || node.node_type == "rtos_task"
            || matches!(
                node.node_type.as_str(),
                "spi_device" | "i2c_device" | "uart_device"
            )
        {
            continue;
        }
        let node_pins: HashSet<u8> = node_pin_bindings(node)
            .into_iter()
            .map(|(_, p)| p)
            .collect();
        if node_pins.len() < 2 {
            continue;
        }
        for driver in &driver_nodes {
            if node.id == driver.id {
                continue;
            }
            let driver_pins: HashSet<u8> = node_pin_bindings(driver)
                .into_iter()
                .map(|(_, p)| p)
                .collect();
            if driver_pins.len() < 2 {
                continue;
            }
            let overlap_count = node_pins.intersection(&driver_pins).count();
            if overlap_count < 2 {
                continue;
            }
            let edge_shape = if let (Some(source_port_id), Some(target_port_id)) = (
                resolve_default_output_port_id(
                    &node.node_type,
                    &node.id,
                    &["data_out", "exec_out", "event_out"],
                ),
                resolve_default_input_port_id(
                    &driver.node_type,
                    &driver.id,
                    &["data_in", "exec_in", "network_in"],
                ),
            ) {
                Some((
                    node.id.clone(),
                    driver.id.clone(),
                    source_port_id,
                    target_port_id,
                    "pin_bus_binding".to_string(),
                ))
            } else if let (Some(source_port_id), Some(target_port_id)) = (
                resolve_default_output_port_id(
                    &driver.node_type,
                    &driver.id,
                    &["data_out", "exec_out", "event_out"],
                ),
                resolve_default_input_port_id(
                    &node.node_type,
                    &node.id,
                    &["data_in", "exec_in", "network_in"],
                ),
            ) {
                Some((
                    driver.id.clone(),
                    node.id.clone(),
                    source_port_id,
                    target_port_id,
                    "pin_bus_binding".to_string(),
                ))
            } else {
                None
            };
            let Some((source_id, target_id, source_port_id, target_port_id, semantic_label)) =
                edge_shape
            else {
                continue;
            };
            if has_edge_between(
                pir_edges,
                &source_id,
                &target_id,
                &[PirEdgeKind::Dependency],
            ) {
                continue;
            }

            let edge_id = next_unique_edge_id(
                &mut existing_edge_ids,
                &format!("e_{}_{}_pinbus", source_id, target_id),
            );
            add_connection_with_edge(
                connections,
                pir_edges,
                PirEdge {
                    id: edge_id,
                    source_node_id: source_id,
                    target_node_id: target_id,
                    source_port_id: Some(source_port_id),
                    target_port_id: Some(target_port_id),
                    kind: PirEdgeKind::Dependency,
                    confidence: 0.82,
                    source_refs: Vec::new(),
                    semantic_label: Some(semantic_label),
                    validated: false,
                },
            );
            added += 1;
        }
    }

    added
}

fn add_pin_binding_edges(
    nodes: &[FirmwareNode],
    connections: &mut Vec<[String; 2]>,
    pir_edges: &mut Vec<PirEdge>,
) -> usize {
    let mut gpio_nodes_by_pin: HashMap<u8, Vec<&FirmwareNode>> = HashMap::new();
    for node in nodes {
        if !is_gpio_interface_node(node) {
            continue;
        }
        if let Some(pin) = node_pin_number(node) {
            gpio_nodes_by_pin.entry(pin).or_default().push(node);
        }
    }

    let mut existing_edge_ids: HashSet<String> = pir_edges.iter().map(|e| e.id.clone()).collect();
    let mut added = 0usize;
    for node in nodes {
        if is_gpio_interface_node(node) {
            continue;
        }
        for (binding_name, pin) in node_pin_bindings(node) {
            let Some(gpio_nodes) = gpio_nodes_by_pin.get(&pin) else {
                continue;
            };

            let inbound = {
                let binding = binding_name.to_ascii_lowercase();
                binding.contains("rx")
                    || binding.contains("miso")
                    || binding.contains("sda")
                    || binding.contains("in")
                    || binding.contains("irq")
                    || binding.contains("int")
            };
            for gpio in gpio_nodes {
                if node.id == gpio.id {
                    continue;
                }
                let (source_node, target_node) = if inbound {
                    (*gpio, node)
                } else {
                    (node, *gpio)
                };

                if has_edge_between(
                    pir_edges,
                    &source_node.id,
                    &target_node.id,
                    &[
                        PirEdgeKind::Hardware,
                        PirEdgeKind::Dependency,
                        PirEdgeKind::Data,
                    ],
                ) {
                    continue;
                }

                let source_port = resolve_default_output_port_id(
                    &source_node.node_type,
                    &source_node.id,
                    &["gpio_out", "data_out", "exec_out", "event_out"],
                );
                let target_port = resolve_default_input_port_id(
                    &target_node.node_type,
                    &target_node.id,
                    &["gpio_in", "data_in", "exec_in", "trigger_in", "event_in"],
                );
                let (Some(source_port_id), Some(target_port_id)) = (source_port, target_port)
                else {
                    continue;
                };

                let edge_id = next_unique_edge_id(
                    &mut existing_edge_ids,
                    &format!(
                        "e_{}_{}_pin_{}",
                        source_node.id, target_node.id, binding_name
                    ),
                );
                add_connection_with_edge(
                    connections,
                    pir_edges,
                    PirEdge {
                        id: edge_id,
                        source_node_id: source_node.id.clone(),
                        target_node_id: target_node.id.clone(),
                        source_port_id: Some(source_port_id),
                        target_port_id: Some(target_port_id),
                        kind: PirEdgeKind::Hardware,
                        confidence: 0.88,
                        source_refs: Vec::new(),
                        semantic_label: Some(format!("pin_binding:{}", binding_name)),
                        validated: false,
                    },
                );
                added += 1;
            }
        }
    }
    added
}

fn add_structural_relationship_edges(
    nodes: &[FirmwareNode],
    connections: &mut Vec<[String; 2]>,
    pir_edges: &mut Vec<PirEdge>,
) {
    let added_task_controls = add_task_control_edges(nodes, connections, pir_edges);
    let added_driver_dependencies = add_driver_dependency_edges(nodes, connections, pir_edges);
    let added_pin_bindings = add_pin_binding_edges(nodes, connections, pir_edges);
    debug_mode_log(
        "pir-graph-quality",
        "H9",
        "builder.rs:add_structural_relationship_edges",
        "added explicit structural relationship edges",
        serde_json::json!({
            "task_control_edges_added": added_task_controls,
            "driver_dependency_edges_added": added_driver_dependencies,
            "pin_binding_edges_added": added_pin_bindings,
        }),
    );
}

fn wire_default_connections(
    nodes: &[FirmwareNode],
    connections: &mut Vec<[String; 2]>,
    pir_edges: &mut Vec<PirEdge>,
) {
    let task_nodes: Vec<&FirmwareNode> = nodes
        .iter()
        .filter(|n| n.node_type == "rtos_task")
        .collect();
    let task_missing_exec_in: Vec<String> = task_nodes
        .iter()
        .filter(|n| !n.ports.iter().any(|p| p.name == "exec_in"))
        .map(|n| n.id.clone())
        .collect();
    let task_with_trigger_in: Vec<String> = task_nodes
        .iter()
        .filter(|n| n.ports.iter().any(|p| p.name == "trigger_in"))
        .map(|n| n.id.clone())
        .collect();
    // #region agent log
    debug_mode_log(
        "pir-graph-quality",
        "H6",
        "builder.rs:wire_default_connections:task_ports",
        "task port capabilities before bootstrapping",
        serde_json::json!({
            "task_count": task_nodes.len(),
            "task_missing_exec_in_count": task_missing_exec_in.len(),
            "task_missing_exec_in_ids": task_missing_exec_in,
            "task_with_trigger_in_count": task_with_trigger_in.len(),
            "task_with_trigger_in_ids": task_with_trigger_in,
        }),
    );
    // #endregion

    let boot = nodes.iter().find(|n| n.id == "boot");
    let boot_out = boot.and_then(|n| find_port_id(n, "boot_out"));

    if let Some(boot_port) = boot_out {
        for node in nodes {
            if !should_bootstrap_node(node) {
                continue;
            }
            if let Some(dst) = find_port_id(node, "exec_in") {
                let boot_label = if node.node_type == "rtos_task" {
                    "spawns"
                } else {
                    "initializes"
                };
                add_connection_with_edge(
                    connections,
                    pir_edges,
                    PirEdge {
                        id: format!("e_boot_{}", node.id),
                        source_node_id: "boot".to_string(),
                        target_node_id: node.id.clone(),
                        source_port_id: Some(boot_port.clone()),
                        target_port_id: Some(dst),
                        kind: PirEdgeKind::Execution,
                        confidence: 0.9,
                        source_refs: Vec::new(),
                        semantic_label: Some(boot_label.to_string()),
                        validated: false,
                    },
                );
            }
        }
    }

    infer_compatible_connections(nodes, connections, pir_edges);
    add_structural_relationship_edges(nodes, connections, pir_edges);
}

pub fn compute_revision(facts: &AnalysisFacts) -> String {
    let mut keys: Vec<_> = facts.file_hashes.keys().collect();
    keys.sort();
    let mut payload = String::new();
    for k in keys {
        payload.push_str(k);
        payload.push(':');
        payload.push_str(facts.file_hashes.get(k).map(|s| s.as_str()).unwrap_or(""));
        payload.push('|');
    }
    format!("{:x}", md5::compute(payload.as_bytes()))
}

fn merge_properties(base: &mut serde_json::Value, overrides: &serde_json::Value) {
    if let (Some(base_map), Some(override_map)) = (base.as_object_mut(), overrides.as_object()) {
        for (k, v) in override_map {
            if !v.is_null() {
                base_map.insert(k.clone(), v.clone());
            }
        }
    }
}

fn network_editable_fields(node_type: &str) -> Vec<String> {
    match node_type {
        "wifi_manager" => vec![
            "ssid".to_string(),
            "password".to_string(),
            "mode".to_string(),
        ],
        "mqtt_client" => vec![
            "broker_url".to_string(),
            "topic".to_string(),
            "client_id".to_string(),
            "qos".to_string(),
        ],
        "ble_manager" => vec!["device_name".to_string(), "role".to_string()],
        "http_client" => vec![
            "url".to_string(),
            "method".to_string(),
            "timeout_ms".to_string(),
        ],
        "websocket_client" => vec!["url".to_string(), "reconnect_ms".to_string()],
        "i2c_device" => vec![
            "sda_pin".to_string(),
            "scl_pin".to_string(),
            "address".to_string(),
            "clock_hz".to_string(),
        ],
        "uart_device" => vec![
            "port".to_string(),
            "baud_rate".to_string(),
            "tx_pin".to_string(),
            "rx_pin".to_string(),
        ],
        "spi_device" => vec![
            "host".to_string(),
            "sclk_pin".to_string(),
            "mosi_pin".to_string(),
            "miso_pin".to_string(),
            "cs_pin".to_string(),
            "dc_pin".to_string(),
            "rst_pin".to_string(),
        ],
        "adc_reader" => vec![
            "pin".to_string(),
            "attenuation".to_string(),
            "sample_rate_hz".to_string(),
        ],
        "pwm_output" => vec![
            "pin".to_string(),
            "frequency_hz".to_string(),
            "resolution_bits".to_string(),
        ],
        "display_output" => vec![
            "interface".to_string(),
            "width".to_string(),
            "height".to_string(),
        ],
        "camera_capture" => vec![
            "interface".to_string(),
            "frame_width".to_string(),
            "frame_height".to_string(),
            "fps".to_string(),
        ],
        "storage_manager" => vec!["backend".to_string(), "namespace".to_string()],
        "ota_update" => vec!["partition_label".to_string(), "url".to_string()],
        "event_handler" => vec!["event_base".to_string(), "event_id".to_string()],
        "timer_node" => vec![
            "timer_name".to_string(),
            "period_ms".to_string(),
            "auto_reload".to_string(),
        ],
        "logger" => vec!["tag".to_string(), "level".to_string()],
        "diagnostics" => vec!["check_heap".to_string(), "check_stack".to_string()],
        _ => Vec::new(),
    }
}

pub fn diff_documents(prev: &PirDocument, next: &PirDocument) -> TopologyDiff {
    let prev_nodes: std::collections::HashSet<_> = prev.nodes.iter().map(|n| &n.id).collect();
    let next_nodes: std::collections::HashSet<_> = next.nodes.iter().map(|n| &n.id).collect();
    let nodes_added: Vec<String> = next_nodes
        .difference(&prev_nodes)
        .map(|s| (*s).clone())
        .collect();
    let nodes_removed: Vec<String> = prev_nodes
        .difference(&next_nodes)
        .map(|s| (*s).clone())
        .collect();
    let nodes_changed: Vec<String> = next
        .nodes
        .iter()
        .filter(|n| {
            prev.nodes
                .iter()
                .find(|p| p.id == n.id)
                .map(|p| p.properties != n.properties)
                .unwrap_or(false)
        })
        .map(|n| n.id.clone())
        .collect();

    let prev_edges: std::collections::HashSet<_> = prev.edges.iter().map(|e| &e.id).collect();
    let next_edges: std::collections::HashSet<_> = next.edges.iter().map(|e| &e.id).collect();

    TopologyDiff {
        from_revision: prev.revision.clone(),
        to_revision: next.revision.clone(),
        nodes_added,
        nodes_removed,
        nodes_changed,
        edges_added: next_edges
            .difference(&prev_edges)
            .map(|s| (*s).clone())
            .collect(),
        edges_removed: prev_edges
            .difference(&next_edges)
            .map(|s| (*s).clone())
            .collect(),
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Build FirmwareGraph from a PIR document (AI or static path).
pub fn graph_from_pir(pir: &PirDocument, project_name: &str) -> FirmwareGraph {
    let mut firmware_nodes: Vec<FirmwareNode> = Vec::new();
    for pn in &pir.nodes {
        let pin = pn
            .properties
            .get("pin")
            .and_then(|v| v.as_u64())
            .map(|p| p as u8);
        let hw = pin.map(|gpio| HardwareMetadata {
            gpio: Some(gpio),
            bus: Some("gpio".to_string()),
            peripheral: None,
            pin_label: pn.label.clone(),
            i2c_address: None,
            spi_host: None,
            uart_port: None,
        });
        let node_summary = pn
            .ai_summary
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(shorten_summary)
            .unwrap_or_else(|| {
                fallback_node_ai_summary(
                    &pn.node_type,
                    pn.label.as_deref().unwrap_or(&pn.id),
                    &pn.properties,
                )
            });
        firmware_nodes.push(FirmwareNode {
            id: pn.id.clone(),
            node_type: pn.node_type.clone(),
            label: pn.label.clone(),
            description: Some(node_summary),
            ports: default_ports_for_type(&pn.node_type, &pn.id),
            properties: pn.properties.clone(),
            hardware: hw,
            execution: Some(ExecutionMetadata {
                phase: None,
                priority: pn
                    .properties
                    .get("priority")
                    .and_then(|v| v.as_u64())
                    .map(|p| p as u8),
                stack_size: pn
                    .properties
                    .get("stack_size")
                    .and_then(|v| v.as_u64())
                    .map(|p| p as u32),
                core_affinity: None,
                period_ms: pn.properties.get("period_ms").and_then(|v| v.as_f64()),
                trigger: None,
            }),
            visual: None,
            validation_state: None,
            runtime_state: None,
        });
    }

    let mut connections: Vec<[String; 2]> = Vec::new();
    let mut port_to_node: HashMap<String, String> = HashMap::new();
    for node in &firmware_nodes {
        for port in &node.ports {
            port_to_node.insert(port.id.clone(), node.id.clone());
        }
    }
    let mut edges_used_ai_ports = 0u32;
    let mut edges_fallback_resolve = 0u32;
    let mut edges_dropped = 0u32;
    let mut edges_mismatched_port_owner = 0u32;
    let mut fallback_samples: Vec<serde_json::Value> = Vec::new();
    let mut dropped_samples: Vec<serde_json::Value> = Vec::new();
    for edge in &pir.edges {
        let before = connections.len();
        if let (Some(sp), Some(tp)) = (&edge.source_port_id, &edge.target_port_id) {
            let src_owner = port_to_node.get(sp);
            let dst_owner = port_to_node.get(tp);
            let ownership_matches = src_owner.map(|s| s.as_str())
                == Some(edge.source_node_id.as_str())
                && dst_owner.map(|s| s.as_str()) == Some(edge.target_node_id.as_str());
            if ownership_matches {
                push_unique_connection(&mut connections, sp, tp);
                if connections.len() > before {
                    edges_used_ai_ports += 1;
                }
            } else {
                if src_owner.is_some() || dst_owner.is_some() {
                    edges_mismatched_port_owner += 1;
                }
                resolve_edge_ports(edge, &firmware_nodes, &mut connections);
                if connections.len() > before {
                    edges_fallback_resolve += 1;
                    if fallback_samples.len() < 5 {
                        fallback_samples.push(serde_json::json!({
                            "edge_id": edge.id,
                            "source_node_id": edge.source_node_id,
                            "target_node_id": edge.target_node_id,
                            "source_port_id": edge.source_port_id,
                            "target_port_id": edge.target_port_id,
                            "source_port_exists": edge.source_port_id.as_ref().map(|id| port_id_exists_on_nodes(&firmware_nodes, id)).unwrap_or(false),
                            "target_port_exists": edge.target_port_id.as_ref().map(|id| port_id_exists_on_nodes(&firmware_nodes, id)).unwrap_or(false),
                            "source_port_owner": src_owner,
                            "target_port_owner": dst_owner,
                        }));
                    }
                } else {
                    edges_dropped += 1;
                    if dropped_samples.len() < 5 {
                        let src = firmware_nodes.iter().find(|n| n.id == edge.source_node_id);
                        let dst = firmware_nodes.iter().find(|n| n.id == edge.target_node_id);
                        dropped_samples.push(serde_json::json!({
                            "edge_id": edge.id,
                            "source_node_id": edge.source_node_id,
                            "target_node_id": edge.target_node_id,
                            "source_port_id": edge.source_port_id,
                            "target_port_id": edge.target_port_id,
                            "source_node_exists": src.is_some(),
                            "target_node_exists": dst.is_some(),
                            "source_output_ports": src.map(|n| n.ports.iter().filter(|p| matches!(p.direction, crate::firmware_topology::types::PortDirection::Output)).count()).unwrap_or(0),
                            "target_input_ports": dst.map(|n| n.ports.iter().filter(|p| matches!(p.direction, crate::firmware_topology::types::PortDirection::Input)).count()).unwrap_or(0),
                        }));
                    }
                }
            }
        } else {
            resolve_edge_ports(edge, &firmware_nodes, &mut connections);
            if connections.len() > before {
                edges_fallback_resolve += 1;
                if fallback_samples.len() < 5 {
                    fallback_samples.push(serde_json::json!({
                        "edge_id": edge.id,
                        "source_node_id": edge.source_node_id,
                        "target_node_id": edge.target_node_id,
                        "source_port_id": edge.source_port_id,
                        "target_port_id": edge.target_port_id,
                        "reason": "missing_explicit_ports",
                    }));
                }
            } else {
                edges_dropped += 1;
                if dropped_samples.len() < 5 {
                    let src = firmware_nodes.iter().find(|n| n.id == edge.source_node_id);
                    let dst = firmware_nodes.iter().find(|n| n.id == edge.target_node_id);
                    dropped_samples.push(serde_json::json!({
                        "edge_id": edge.id,
                        "source_node_id": edge.source_node_id,
                        "target_node_id": edge.target_node_id,
                        "source_port_id": edge.source_port_id,
                        "target_port_id": edge.target_port_id,
                        "source_node_exists": src.is_some(),
                        "target_node_exists": dst.is_some(),
                        "source_output_ports": src.map(|n| n.ports.iter().filter(|p| matches!(p.direction, crate::firmware_topology::types::PortDirection::Output)).count()).unwrap_or(0),
                        "target_input_ports": dst.map(|n| n.ports.iter().filter(|p| matches!(p.direction, crate::firmware_topology::types::PortDirection::Input)).count()).unwrap_or(0),
                    }));
                }
            }
        }
    }
    // #region agent log
    debug_mode_log(
        "pir-graph-quality",
        "H2",
        "builder.rs:graph_from_pir",
        "resolved PIR edges into graph connections",
        serde_json::json!({
            "pir_nodes": pir.nodes.len(),
            "pir_edges": pir.edges.len(),
            "edges_used_ai_ports": edges_used_ai_ports,
            "edges_fallback_resolve": edges_fallback_resolve,
            "edges_dropped": edges_dropped,
            "edges_mismatched_port_owner": edges_mismatched_port_owner,
            "fallback_samples": fallback_samples,
            "dropped_samples": dropped_samples,
        }),
    );
    // #endregion

    let mut graph = FirmwareGraph {
        schema_version: SCHEMA_VERSION,
        id: Some(pir.id.clone()),
        name: Some(format!("{} topology", project_name)),
        description: pir.summary.as_ref().map(|s| s.headline.clone()),
        board_id: pir.provenance.board_id.clone(),
        nodes: firmware_nodes,
        connections,
        layout: Some(Default::default()),
        runtime_metadata: None,
    };
    let connectivity_edges_added = ensure_graph_nodes_connected(&mut graph);
    if connectivity_edges_added > 0 {
        debug_mode_log(
            "pir-graph-quality",
            "H11",
            "builder.rs:graph_from_pir",
            "added synthetic connections to keep all nodes connected",
            serde_json::json!({
                "connectivity_edges_added": connectivity_edges_added,
                "nodes": graph.nodes.len(),
                "connections": graph.connections.len(),
            }),
        );
    }

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

fn port_id_exists_on_nodes(nodes: &[FirmwareNode], port_id: &str) -> bool {
    nodes
        .iter()
        .any(|n| n.ports.iter().any(|p| p.id == port_id))
}

fn push_unique_connection(
    connections: &mut Vec<[String; 2]>,
    source_port: &str,
    target_port: &str,
) {
    if connections
        .iter()
        .any(|pair| pair[0] == source_port && pair[1] == target_port)
    {
        return;
    }
    connections.push([source_port.to_string(), target_port.to_string()]);
}

fn connected_node_components(
    node_ids: &[String],
    edges: &[(String, String)],
) -> Vec<Vec<String>> {
    if node_ids.is_empty() {
        return Vec::new();
    }

    let valid_nodes: HashSet<&str> = node_ids.iter().map(|id| id.as_str()).collect();
    let mut adjacency: HashMap<String, HashSet<String>> = node_ids
        .iter()
        .map(|id| (id.clone(), HashSet::new()))
        .collect();
    for (a, b) in edges {
        if a == b || !valid_nodes.contains(a.as_str()) || !valid_nodes.contains(b.as_str()) {
            continue;
        }
        adjacency.entry(a.clone()).or_default().insert(b.clone());
        adjacency.entry(b.clone()).or_default().insert(a.clone());
    }

    let mut visited = HashSet::<String>::new();
    let mut components = Vec::<Vec<String>>::new();
    for id in node_ids {
        if visited.contains(id) {
            continue;
        }
        let mut queue = std::collections::VecDeque::new();
        let mut component = Vec::<String>::new();
        queue.push_back(id.clone());
        visited.insert(id.clone());
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
    components
}

fn sorted_component_ids(
    component: &[String],
    node_order: &HashMap<String, usize>,
) -> Vec<String> {
    let mut out = component.to_vec();
    out.sort_by_key(|id| node_order.get(id).copied().unwrap_or(usize::MAX));
    out
}

fn select_connectable_bridge(
    component_a: &[String],
    component_b: &[String],
    node_by_id: &HashMap<String, &FirmwareNode>,
    node_order: &HashMap<String, usize>,
) -> Option<(String, String)> {
    let comp_a = sorted_component_ids(component_a, node_order);
    let comp_b = sorted_component_ids(component_b, node_order);

    for src_id in &comp_a {
        let Some(src_node) = node_by_id.get(src_id) else {
            continue;
        };
        let Some(src_port) = preferred_output_port_id(
            src_node,
            &["exec_out", "data_out", "event_out", "network_out", "gpio_out", "boot_out"],
        ) else {
            continue;
        };
        for dst_id in &comp_b {
            let Some(dst_node) = node_by_id.get(dst_id) else {
                continue;
            };
            let Some(dst_port) = preferred_input_port_id(
                dst_node,
                &["exec_in", "data_in", "trigger_in", "event_in", "network_in", "gpio_in"],
            ) else {
                continue;
            };
            return Some((src_port, dst_port));
        }
    }

    for src_id in &comp_b {
        let Some(src_node) = node_by_id.get(src_id) else {
            continue;
        };
        let Some(src_port) = preferred_output_port_id(
            src_node,
            &["exec_out", "data_out", "event_out", "network_out", "gpio_out", "boot_out"],
        ) else {
            continue;
        };
        for dst_id in &comp_a {
            let Some(dst_node) = node_by_id.get(dst_id) else {
                continue;
            };
            let Some(dst_port) = preferred_input_port_id(
                dst_node,
                &["exec_in", "data_in", "trigger_in", "event_in", "network_in", "gpio_in"],
            ) else {
                continue;
            };
            return Some((src_port, dst_port));
        }
    }

    None
}

fn ensure_graph_nodes_connected(graph: &mut FirmwareGraph) -> usize {
    if graph.nodes.len() < 2 {
        return 0;
    }

    let node_ids: Vec<String> = graph.nodes.iter().map(|n| n.id.clone()).collect();
    let node_order: HashMap<String, usize> = node_ids
        .iter()
        .enumerate()
        .map(|(idx, id)| (id.clone(), idx))
        .collect();
    let node_by_id: HashMap<String, &FirmwareNode> =
        graph.nodes.iter().map(|n| (n.id.clone(), n)).collect();

    let port_to_node: HashMap<String, String> = graph
        .nodes
        .iter()
        .flat_map(|n| n.ports.iter().map(|p| (p.id.clone(), n.id.clone())))
        .collect();

    let mut node_edges: Vec<(String, String)> = Vec::new();
    for pair in &graph.connections {
        let Some(src_node) = port_to_node.get(&pair[0]) else {
            continue;
        };
        let Some(dst_node) = port_to_node.get(&pair[1]) else {
            continue;
        };
        if src_node == dst_node {
            continue;
        }
        node_edges.push((src_node.clone(), dst_node.clone()));
    }

    let mut components = connected_node_components(&node_ids, &node_edges);
    if components.len() <= 1 {
        return 0;
    }
    components.sort_by_key(|comp| {
        comp.iter()
            .filter_map(|id| node_order.get(id).copied())
            .min()
            .unwrap_or(usize::MAX)
    });

    let mut existing_pairs: HashSet<(String, String)> = graph
        .connections
        .iter()
        .map(|pair| (pair[0].clone(), pair[1].clone()))
        .collect();
    let mut added = 0usize;
    for idx in 1..components.len() {
        let prev_component = &components[idx - 1];
        let cur_component = &components[idx];
        let Some((src_port, dst_port)) =
            select_connectable_bridge(prev_component, cur_component, &node_by_id, &node_order)
        else {
            continue;
        };
        if existing_pairs.insert((src_port.clone(), dst_port.clone())) {
            graph.connections.push([src_port, dst_port]);
            added += 1;
        }
    }
    added
}

fn preferred_output_port_id(node: &FirmwareNode, preferred_names: &[&str]) -> Option<String> {
    for preferred in preferred_names {
        if let Some(port) = node
            .ports
            .iter()
            .find(|p| p.name == *preferred && matches!(p.direction, PortDirection::Output))
        {
            return Some(port.id.clone());
        }
    }
    node.ports
        .iter()
        .find(|p| matches!(p.direction, PortDirection::Output))
        .map(|p| p.id.clone())
}

fn preferred_input_port_id(node: &FirmwareNode, preferred_names: &[&str]) -> Option<String> {
    for preferred in preferred_names {
        if let Some(port) = node
            .ports
            .iter()
            .find(|p| p.name == *preferred && matches!(p.direction, PortDirection::Input))
        {
            return Some(port.id.clone());
        }
    }
    node.ports
        .iter()
        .find(|p| matches!(p.direction, PortDirection::Input))
        .map(|p| p.id.clone())
}

fn resolve_edge_ports(edge: &PirEdge, nodes: &[FirmwareNode], connections: &mut Vec<[String; 2]>) {
    let src = nodes.iter().find(|n| n.id == edge.source_node_id);
    let dst = nodes.iter().find(|n| n.id == edge.target_node_id);
    if let (Some(s), Some(d)) = (src, dst) {
        let source_preferred: &[&str] = if s.id == "boot" {
            &[
                "boot_out",
                "exec_out",
                "event_out",
                "data_out",
                "gpio_out",
                "network_out",
            ]
        } else {
            match edge.kind {
                PirEdgeKind::Network => &["network_out", "data_out", "event_out", "exec_out"],
                PirEdgeKind::Event => &["event_out", "data_out", "exec_out"],
                PirEdgeKind::Hardware => &["gpio_out", "data_out", "exec_out", "event_out"],
                PirEdgeKind::Data => &["data_out", "event_out", "exec_out"],
                PirEdgeKind::Execution
                | PirEdgeKind::Dependency
                | PirEdgeKind::Ota
                | PirEdgeKind::Fsm => &[
                    "exec_out",
                    "event_out",
                    "data_out",
                    "gpio_out",
                    "network_out",
                ],
            }
        };
        let target_preferred: &[&str] = match edge.kind {
            PirEdgeKind::Network => &["network_in", "data_in", "exec_in"],
            PirEdgeKind::Event => &["trigger_in", "event_in", "exec_in", "data_in"],
            PirEdgeKind::Hardware => &["gpio_in", "exec_in", "data_in"],
            PirEdgeKind::Data => &["data_in", "trigger_in", "event_in", "exec_in"],
            PirEdgeKind::Execution
            | PirEdgeKind::Dependency
            | PirEdgeKind::Ota
            | PirEdgeKind::Fsm => &[
                "exec_in",
                "trigger_in",
                "data_in",
                "network_in",
                "event_in",
                "gpio_in",
            ],
        };

        let out_port = preferred_output_port_id(s, source_preferred);
        let in_port = preferred_input_port_id(d, target_preferred);
        if let (Some(sp), Some(tp)) = (out_port, in_port) {
            push_unique_connection(connections, &sp, &tp);
        }
    }
}

fn node_is_static_extracted(node: &PirNode) -> bool {
    !node.source_refs.is_empty() && node.source_refs.iter().all(|r| r.inferred_by == "static")
}

fn node_meets_graph_confidence(node: &PirNode, min_confidence: f32) -> bool {
    if matches!(node.authority, NodeAuthority::User | NodeAuthority::Hybrid) {
        return true;
    }
    if node_is_static_extracted(node) {
        return true;
    }
    node.confidence >= min_confidence
}

/// Drop speculative agent nodes below the confidence threshold; prune dependent edges and layers.
pub fn filter_pir_by_confidence(pir: &mut PirDocument, min_confidence: f32) -> u32 {
    let kept_ids: HashSet<String> = pir
        .nodes
        .iter()
        .filter(|n| node_meets_graph_confidence(n, min_confidence))
        .map(|n| n.id.clone())
        .collect();

    let before = pir.nodes.len();
    let removed_node_ids: Vec<String> = pir
        .nodes
        .iter()
        .filter(|n| !kept_ids.contains(&n.id))
        .take(8)
        .map(|n| n.id.clone())
        .collect();
    let edges_before = pir.edges.len();
    pir.nodes.retain(|n| kept_ids.contains(&n.id));
    pir.edges
        .retain(|e| kept_ids.contains(&e.source_node_id) && kept_ids.contains(&e.target_node_id));

    let prune_layer = |ids: &mut Vec<String>| ids.retain(|id| kept_ids.contains(id));
    prune_layer(&mut pir.layers.physical);
    prune_layer(&mut pir.layers.runtime);
    prune_layer(&mut pir.layers.network);
    prune_layer(&mut pir.layers.system);

    let dropped = before.saturating_sub(pir.nodes.len());
    let edges_pruned = edges_before.saturating_sub(pir.edges.len());
    // #region agent log
    debug_mode_log(
        "pir-graph-quality",
        "H4",
        "builder.rs:filter_pir_by_confidence",
        "applied confidence filter to PIR nodes",
        serde_json::json!({
            "min_confidence": min_confidence,
            "nodes_before": before,
            "nodes_after": pir.nodes.len(),
            "nodes_dropped": dropped,
            "edges_before": edges_before,
            "edges_after": pir.edges.len(),
            "edges_pruned": edges_pruned,
            "removed_node_ids": removed_node_ids,
        }),
    );
    // #endregion
    if dropped > 0 {
        let msg = format!(
            "Omitted {dropped} low-confidence node(s) (below {:.0}% threshold).",
            min_confidence * 100.0
        );
        if let Some(summary) = pir.summary.as_mut() {
            if !summary.warnings.iter().any(|w| w == &msg) {
                summary.warnings.push(msg);
            }
            summary.node_count = pir.nodes.len() as u32;
            summary.edge_count = pir.edges.len() as u32;
        }
    }
    dropped as u32
}

fn normalize_edge_endpoints_from_ports(pir: &mut PirDocument) {
    let known_node_ids: HashSet<String> = pir.nodes.iter().map(|n| n.id.clone()).collect();
    let mut port_to_node: HashMap<String, String> = HashMap::new();
    for node in &pir.nodes {
        for port in default_ports_for_type(&node.node_type, &node.id) {
            port_to_node.insert(port.id, node.id.clone());
        }
    }

    let mut fixed_source_node = 0usize;
    let mut fixed_target_node = 0usize;
    let mut fixed_source_port = 0usize;
    let mut fixed_target_port = 0usize;
    let mut dropped_unknown_source_port = 0usize;
    let mut dropped_unknown_target_port = 0usize;
    let mut cleared_mismatched_source_port = 0usize;
    let mut cleared_mismatched_target_port = 0usize;
    let mut unresolved_samples: Vec<serde_json::Value> = Vec::new();

    for edge in &mut pir.edges {
        if let Some(source_port) = edge.source_port_id.clone() {
            if !port_to_node.contains_key(&source_port) {
                edge.source_port_id = None;
                dropped_unknown_source_port += 1;
            }
        }
        if let Some(target_port) = edge.target_port_id.clone() {
            if !port_to_node.contains_key(&target_port) {
                edge.target_port_id = None;
                dropped_unknown_target_port += 1;
            }
        }

        if edge.source_port_id.is_none() {
            let candidate = edge.source_node_id.trim();
            if !candidate.is_empty() && port_to_node.contains_key(candidate) {
                edge.source_port_id = Some(candidate.to_string());
                fixed_source_port += 1;
            }
        }
        if edge.target_port_id.is_none() {
            let candidate = edge.target_node_id.trim();
            if !candidate.is_empty() && port_to_node.contains_key(candidate) {
                edge.target_port_id = Some(candidate.to_string());
                fixed_target_port += 1;
            }
        }

        let src_invalid = edge.source_node_id.trim().is_empty()
            || !known_node_ids.contains(edge.source_node_id.trim());
        if src_invalid {
            if let Some(mapped) = edge
                .source_port_id
                .as_ref()
                .and_then(|p| port_to_node.get(p))
                .cloned()
            {
                edge.source_node_id = mapped;
                fixed_source_node += 1;
            }
        }

        let dst_invalid = edge.target_node_id.trim().is_empty()
            || !known_node_ids.contains(edge.target_node_id.trim());
        if dst_invalid {
            if let Some(mapped) = edge
                .target_port_id
                .as_ref()
                .and_then(|p| port_to_node.get(p))
                .cloned()
            {
                edge.target_node_id = mapped;
                fixed_target_node += 1;
            }
        }

        if let Some(source_port) = edge.source_port_id.clone() {
            if let Some(owner) = port_to_node.get(&source_port) {
                if known_node_ids.contains(edge.source_node_id.trim())
                    && owner != edge.source_node_id.trim()
                {
                    edge.source_port_id = None;
                    cleared_mismatched_source_port += 1;
                }
            }
        }
        if let Some(target_port) = edge.target_port_id.clone() {
            if let Some(owner) = port_to_node.get(&target_port) {
                if known_node_ids.contains(edge.target_node_id.trim())
                    && owner != edge.target_node_id.trim()
                {
                    edge.target_port_id = None;
                    cleared_mismatched_target_port += 1;
                }
            }
        }

        if unresolved_samples.len() < 6
            && (edge.source_node_id.trim().is_empty()
                || edge.target_node_id.trim().is_empty()
                || !known_node_ids.contains(edge.source_node_id.trim())
                || !known_node_ids.contains(edge.target_node_id.trim()))
        {
            unresolved_samples.push(serde_json::json!({
                "edge_id": edge.id,
                "source_node_id": edge.source_node_id,
                "target_node_id": edge.target_node_id,
                "source_port_id": edge.source_port_id,
                "target_port_id": edge.target_port_id,
            }));
        }
    }

    // #region agent log
    debug_mode_log(
        "pir-graph-quality",
        "H5",
        "builder.rs:normalize_edge_endpoints_from_ports",
        "normalized edge endpoints using registry ports",
        serde_json::json!({
            "edges_count": pir.edges.len(),
            "fixed_source_node": fixed_source_node,
            "fixed_target_node": fixed_target_node,
            "fixed_source_port": fixed_source_port,
            "fixed_target_port": fixed_target_port,
            "dropped_unknown_source_port": dropped_unknown_source_port,
            "dropped_unknown_target_port": dropped_unknown_target_port,
            "cleared_mismatched_source_port": cleared_mismatched_source_port,
            "cleared_mismatched_target_port": cleared_mismatched_target_port,
            "unresolved_samples": unresolved_samples,
        }),
    );
    // #endregion
}

fn next_unique_edge_id(existing_ids: &mut HashSet<String>, base_id: &str) -> String {
    if existing_ids.insert(base_id.to_string()) {
        return base_id.to_string();
    }
    let mut suffix = 1usize;
    loop {
        let candidate = format!("{}_{}", base_id, suffix);
        if existing_ids.insert(candidate.clone()) {
            return candidate;
        }
        suffix += 1;
    }
}

fn resolve_default_output_port_id(
    node_type: &str,
    node_id: &str,
    preferred: &[&str],
) -> Option<String> {
    let ports = default_ports_for_type(node_type, node_id);
    for name in preferred {
        if let Some(port) = ports
            .iter()
            .find(|p| p.name == *name && matches!(p.direction, PortDirection::Output))
        {
            return Some(port.id.clone());
        }
    }
    ports
        .iter()
        .find(|p| matches!(p.direction, PortDirection::Output))
        .map(|p| p.id.clone())
}

fn resolve_default_input_port_id(
    node_type: &str,
    node_id: &str,
    preferred: &[&str],
) -> Option<String> {
    let ports = default_ports_for_type(node_type, node_id);
    for name in preferred {
        if let Some(port) = ports
            .iter()
            .find(|p| p.name == *name && matches!(p.direction, PortDirection::Input))
        {
            return Some(port.id.clone());
        }
    }
    ports
        .iter()
        .find(|p| matches!(p.direction, PortDirection::Input))
        .map(|p| p.id.clone())
}

fn push_execution_edge(
    pir: &mut PirDocument,
    existing_edge_ids: &mut HashSet<String>,
    node_type_by_id: &HashMap<String, String>,
    source_id: &str,
    target_id: &str,
    semantic_label: &str,
    confidence: f32,
    source_preferred: &[&str],
    target_preferred: &[&str],
    edge_suffix: &str,
) -> bool {
    let source_type = node_type_by_id
        .get(source_id)
        .map(String::as_str)
        .unwrap_or("system_init");
    let target_type = node_type_by_id
        .get(target_id)
        .map(String::as_str)
        .unwrap_or("gpio_output");
    let source_port = resolve_default_output_port_id(source_type, source_id, source_preferred);
    let target_port = resolve_default_input_port_id(target_type, target_id, target_preferred);
    if let (Some(source_port_id), Some(target_port_id)) = (source_port, target_port) {
        let edge_id = next_unique_edge_id(
            existing_edge_ids,
            &format!("e_{}_{}_{}", source_id, target_id, edge_suffix),
        );
        pir.edges.push(PirEdge {
            id: edge_id,
            source_node_id: source_id.to_string(),
            target_node_id: target_id.to_string(),
            source_port_id: Some(source_port_id),
            target_port_id: Some(target_port_id),
            kind: PirEdgeKind::Execution,
            confidence,
            source_refs: Vec::new(),
            semantic_label: Some(semantic_label.to_string()),
            validated: false,
        });
        return true;
    }
    false
}

fn enforce_boot_task_gpio_chain(pir: &mut PirDocument) {
    let boot_id = pir
        .nodes
        .iter()
        .find(|n| n.id == "boot")
        .or_else(|| pir.nodes.iter().find(|n| n.node_type == "system_init"))
        .map(|n| n.id.clone());
    let Some(boot_id) = boot_id else {
        return;
    };

    let output_ids: Vec<String> = pir
        .nodes
        .iter()
        .filter(|n| matches!(n.node_type.as_str(), "gpio_output" | "pwm_output"))
        .map(|n| n.id.clone())
        .collect();
    if output_ids.is_empty() {
        return;
    }

    let mut task_ids: Vec<String> = pir
        .nodes
        .iter()
        .filter(|n| n.node_type == "rtos_task")
        .map(|n| n.id.clone())
        .collect();
    task_ids.sort();

    let node_type_by_id: HashMap<String, String> = pir
        .nodes
        .iter()
        .map(|n| (n.id.clone(), n.node_type.clone()))
        .collect();
    let task_id_set: HashSet<String> = task_ids.iter().cloned().collect();

    let mut existing_edge_ids: HashSet<String> = pir.edges.iter().map(|e| e.id.clone()).collect();
    let mut added_boot_to_task = 0usize;
    let mut added_task_to_output = 0usize;
    let mut added_boot_to_output = 0usize;
    let mut primary_task_used: Option<String> = None;

    if let Some(primary_task) = task_ids
        .iter()
        .find(|task_id| {
            pir.edges.iter().any(|e| {
                e.source_node_id == boot_id
                    && e.target_node_id == **task_id
                    && matches!(e.kind, PirEdgeKind::Execution)
            })
        })
        .cloned()
        .or_else(|| task_ids.first().cloned())
    {
        primary_task_used = Some(primary_task.clone());
        let has_boot_to_primary = pir.edges.iter().any(|e| {
            e.source_node_id == boot_id
                && e.target_node_id == primary_task
                && matches!(e.kind, PirEdgeKind::Execution)
        });
        if !has_boot_to_primary
            && push_execution_edge(
                pir,
                &mut existing_edge_ids,
                &node_type_by_id,
                &boot_id,
                &primary_task,
                "spawns",
                0.90,
                &["boot_out", "exec_out", "event_out"],
                &["exec_in", "trigger_in", "data_in"],
                "spawns",
            )
        {
            added_boot_to_task += 1;
        }

        for output_id in &output_ids {
            let has_task_to_output = pir.edges.iter().any(|e| {
                task_id_set.contains(&e.source_node_id)
                    && e.target_node_id == *output_id
                    && matches!(e.kind, PirEdgeKind::Execution | PirEdgeKind::Hardware)
            });
            if has_task_to_output {
                continue;
            }
            if push_execution_edge(
                pir,
                &mut existing_edge_ids,
                &node_type_by_id,
                &primary_task,
                output_id,
                "controls",
                0.86,
                &["exec_out", "event_out", "data_out"],
                &["exec_in", "gpio_in", "data_in"],
                "controls",
            ) {
                added_task_to_output += 1;
            }
        }
    }

    for output_id in &output_ids {
        let has_driver_edge = pir.edges.iter().any(|e| {
            e.target_node_id == *output_id
                && matches!(e.kind, PirEdgeKind::Execution | PirEdgeKind::Hardware)
                && (e.source_node_id == boot_id || task_id_set.contains(&e.source_node_id))
        });
        if has_driver_edge {
            continue;
        }
        if push_execution_edge(
            pir,
            &mut existing_edge_ids,
            &node_type_by_id,
            &boot_id,
            output_id,
            "drives",
            0.84,
            &["boot_out", "exec_out", "event_out", "data_out"],
            &["exec_in", "gpio_in", "data_in", "trigger_in"],
            "drives",
        ) {
            added_boot_to_output += 1;
        }
    }
    let unconnected_outputs: Vec<String> = output_ids
        .iter()
        .filter(|output_id| {
            !pir.edges.iter().any(|e| {
                e.target_node_id == output_id.as_str()
                    && matches!(e.kind, PirEdgeKind::Execution | PirEdgeKind::Hardware)
                    && (e.source_node_id == boot_id || task_id_set.contains(&e.source_node_id))
            })
        })
        .cloned()
        .collect();

    // #region agent log
    debug_mode_log(
        "pir-graph-quality",
        "H8",
        "builder.rs:enforce_boot_task_gpio_chain",
        "enforced boot/task -> output execution chain",
        serde_json::json!({
            "boot_id": boot_id,
            "primary_task": primary_task_used,
            "output_targets": output_ids,
            "added_boot_to_task": added_boot_to_task,
            "added_task_to_output": added_task_to_output,
            "added_boot_to_output": added_boot_to_output,
            "unconnected_outputs_after_enforcement": unconnected_outputs,
        }),
    );
    // #endregion
}

/// Validate, layout, and attach validation_state after PIR is assembled.
pub fn finalize_pir_result(
    mut pir: PirDocument,
    project_path: &Path,
    previous: Option<&PirDocument>,
    diff: Option<TopologyDiff>,
) -> (
    PirDocument,
    FirmwareGraph,
    ValidationReport,
    Option<TopologyDiff>,
) {
    normalize_edge_endpoints_from_ports(&mut pir);
    filter_pir_by_confidence(&mut pir, PIR_GRAPH_MIN_NODE_CONFIDENCE);
    enforce_boot_task_gpio_chain(&mut pir);
    ensure_node_ai_summaries(&mut pir);

    let project_name = project_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("esp32_project");
    let graph = graph_from_pir(&pir, project_name);
    let validation = crate::firmware_topology::validate_graph(&graph);

    pir.validation_state = PirValidationState {
        valid: validation.valid,
        error_count: validation
            .issues
            .iter()
            .filter(|i| i.severity == "error")
            .count() as u32,
        warning_count: validation
            .issues
            .iter()
            .filter(|i| i.severity == "warning")
            .count() as u32,
        validated_at_ms: now_ms(),
    };

    if let Some(summary) = pir.summary.as_mut() {
        summary.warnings = validation
            .issues
            .iter()
            .filter(|i| i.severity == "warning")
            .map(|i| i.message.clone())
            .collect();
    }

    pir.diagrams = Some(diagrams::generate_all(&pir));

    super::validation_lock::apply_editable_field_locks(&mut pir, &validation);

    let computed_diff = diff.or_else(|| previous.map(|p| diff_documents(p, &pir)));
    if let Some(ref d) = computed_diff {
        pir.sync_metadata.last_diff = Some(d.clone());
    }

    (pir, graph, validation, computed_diff)
}

#[cfg(test)]
mod tests {
    use super::{
        build_pir_and_graph, ensure_graph_nodes_connected, finalize_pir_result, make_node,
        wire_default_connections, PirEdgeKind,
    };
    use crate::firmware_topology::registry::default_ports_for_type;
    use crate::firmware_topology::types::{FirmwareGraph, FirmwareNode, SCHEMA_VERSION};
    use crate::pir_maker::schema::{AnalysisFacts, GpioFact};
    use serde_json::json;
    use std::path::Path;

    fn test_node(id: &str, node_type: &str, properties: serde_json::Value) -> FirmwareNode {
        FirmwareNode {
            id: id.to_string(),
            node_type: node_type.to_string(),
            label: Some(id.to_string()),
            description: None,
            ports: default_ports_for_type(node_type, id),
            properties,
            hardware: None,
            execution: None,
            visual: None,
            validation_state: None,
            runtime_state: None,
        }
    }

    #[test]
    fn preserves_component_identity_and_adds_structural_edges() {
        let nodes = vec![
            test_node("boot", "system_init", json!({"target": "esp32s3"})),
            test_node("app_task", "rtos_task", json!({"task_name": "app_task"})),
            test_node(
                "oled_spi",
                "spi_device",
                json!({
                    "host": "SPI2_HOST",
                    "sclk_pin": 11,
                    "mosi_pin": 10,
                    "pin_bindings": {
                        "sclk": 11,
                        "mosi": 10
                    }
                }),
            ),
            test_node(
                "oled_display",
                "display_output",
                json!({
                    "interface": "spi",
                    "pin_bindings": {
                        "sclk": 11,
                        "mosi": 10
                    }
                }),
            ),
            test_node("gpio_11", "gpio_output", json!({"pin": 11})),
            test_node("gpio_10", "gpio_output", json!({"pin": 10})),
        ];

        let mut connections: Vec<[String; 2]> = Vec::new();
        let mut pir_edges = Vec::new();
        wire_default_connections(&nodes, &mut connections, &mut pir_edges);

        assert!(
            pir_edges
                .iter()
                .any(|e| { e.source_node_id == "app_task" && e.target_node_id == "oled_spi" }),
            "task should control peripheral components"
        );
        assert!(
            pir_edges.iter().any(|e| {
                ((e.source_node_id == "oled_display" && e.target_node_id == "oled_spi")
                    || (e.source_node_id == "oled_spi" && e.target_node_id == "oled_display"))
                    && e.kind == PirEdgeKind::Dependency
            }),
            "display component should depend on SPI driver component"
        );
        assert!(
            pir_edges.iter().any(|e| {
                e.source_node_id == "oled_spi"
                    && e.target_node_id == "gpio_11"
                    && e.kind == PirEdgeKind::Hardware
                    && e.semantic_label.as_deref() == Some("pin_binding:sclk")
            }),
            "pin binding edge should connect component to standalone GPIO pin node"
        );
    }

    #[test]
    fn make_node_adds_short_ai_summary() {
        let built = make_node(
            "status_led",
            "gpio_output",
            "Status LED",
            json!({"pin": 2}),
            None,
            Some("main/app_config.h".to_string()),
            vec!["pin".to_string()],
            "physical",
        );
        let summary = built.pir.ai_summary.clone().unwrap_or_default();
        assert!(
            summary.contains("GPIO 2"),
            "generated summary should mention the concrete pin"
        );
        assert_eq!(built.firmware.description, Some(summary));
    }

    #[test]
    fn auto_connects_disconnected_graph_components() {
        let mut graph = FirmwareGraph {
            schema_version: SCHEMA_VERSION,
            id: Some("pir_demo".to_string()),
            name: Some("demo".to_string()),
            description: None,
            board_id: None,
            nodes: vec![
                test_node("task_a", "rtos_task", json!({"task_name": "task_a"})),
                test_node("task_b", "rtos_task", json!({"task_name": "task_b"})),
            ],
            connections: Vec::new(),
            layout: None,
            runtime_metadata: None,
        };
        let added = ensure_graph_nodes_connected(&mut graph);
        assert!(
            added > 0,
            "connectivity pass should add a bridge between disconnected components"
        );
        assert!(
            !graph.connections.is_empty(),
            "graph should contain at least one connection after connectivity pass"
        );
    }

    #[test]
    fn adds_boot_to_output_edge_when_no_task_nodes_exist() {
        let mut facts = AnalysisFacts {
            project_name: "demo".to_string(),
            has_app_main: true,
            app_main_file: Some("main/main.c".to_string()),
            ..Default::default()
        };
        facts.gpio_facts.push(GpioFact {
            node_id: "status_led".to_string(),
            node_type: "gpio_output".to_string(),
            label: "Status LED".to_string(),
            pin: 2,
            file: "main/main.c".to_string(),
            line: Some(42),
        });
        facts.analyzed_files.push("main/main.c".to_string());
        facts
            .file_hashes
            .insert("main/main.c".to_string(), "hash1".to_string());

        let root = Path::new("C:/tmp/demo");
        let (pir, _, _, _) = build_pir_and_graph(&facts, root, Some("chat-1"), "rev-1", None);
        let (pir, _, _, _) = finalize_pir_result(pir, root, None, None);

        assert!(
            pir.edges.iter().any(|e| {
                e.source_node_id == "boot"
                    && e.target_node_id == "status_led"
                    && matches!(e.kind, PirEdgeKind::Execution | PirEdgeKind::Hardware)
            }),
            "boot should directly drive output nodes when no task nodes exist"
        );
    }
}
