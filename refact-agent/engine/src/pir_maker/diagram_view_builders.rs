//! Rich HLD/LLD/Sequence view builders (ported from GUI heuristics).

use std::collections::{HashMap, HashSet, VecDeque};

use serde_json::{json, Value as JsonValue};

use super::schema::{PirDocument, PirEdge, PirEdgeKind, PirNode};
use crate::firmware_topology::types::FirmwareNode;

pub fn assoc_port_id(node_id: &str, flavor: &str, dir: &str) -> String {
    format!("{node_id}::{flavor}::{dir}")
}

fn capitalized_word(word: &str) -> String {
    let mut it = word.chars();
    match it.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + it.as_str(),
    }
}

fn prop_str(props: &JsonValue, key: &str) -> Option<String> {
    props.get(key).and_then(|v| {
        if v.is_string() {
            v.as_str().map(String::from)
        } else if v.is_number() {
            Some(v.to_string())
        } else {
            None
        }
    })
}

fn to_service_name(label: &str, node_type: &str) -> String {
    let trimmed = label.trim();
    if !trimmed.is_empty() {
        return trimmed.to_string();
    }
    node_type
        .split('_')
        .map(capitalized_word)
        .collect::<Vec<_>>()
        .join(" ")
}

fn resolve_tier(node_type: &str, node_id: &str) -> &'static str {
    if node_type == "system_init" || node_id == "boot" {
        return "entry";
    }
    if node_type.contains("wifi") || node_type.contains("mqtt") || node_type.contains("network") {
        return "connectivity";
    }
    if node_type.contains("task") || node_type.contains("rtos") || node_type.contains("timer") {
        return "control";
    }
    if node_type.contains("gpio")
        || node_type.contains("sensor")
        || node_type.contains("i2c")
        || node_type.contains("spi")
        || node_type.contains("uart")
        || node_type.contains("adc")
    {
        return "io";
    }
    "control"
}

pub fn tier_to_layer(tier: &str) -> u32 {
    match tier {
        "entry" => 0,
        "control" => 1,
        "io" => 2,
        "connectivity" => 3,
        "storage" => 4,
        _ => 1,
    }
}

pub fn build_hld_component(pn: &PirNode, label: &str) -> JsonValue {
    let node_type = pn.node_type.as_str();
    let mut methods: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    fn push_method(methods: &mut Vec<String>, seen: &mut HashSet<String>, sig: &str) {
        let key = sig.to_ascii_lowercase();
        if seen.insert(key) {
            methods.push(sig.to_string());
        }
    }

    if node_type == "system_init" || pn.id == "boot" {
        push_method(&mut methods, &mut seen, "startSystem()");
        push_method(&mut methods, &mut seen, "initializeHardware()");
        push_method(&mut methods, &mut seen, "registerTasks()");
    } else if node_type == "rtos_task" {
        let task_name =
            prop_str(&pn.properties, "task_name").unwrap_or_else(|| "controlLoop".to_string());
        push_method(&mut methods, &mut seen, &format!("{task_name}()"));
        push_method(&mut methods, &mut seen, "processEvents()");
        push_method(&mut methods, &mut seen, "handleTriggers()");
    } else if node_type == "gpio_output" {
        push_method(&mut methods, &mut seen, "setOutput()");
        push_method(&mut methods, &mut seen, "toggle()");
        push_method(&mut methods, &mut seen, "configurePin()");
    } else if node_type == "gpio_input" || node_type == "sensor_input" {
        push_method(&mut methods, &mut seen, "readSample()");
        if label.to_ascii_lowercase().contains("ir")
            || label.to_ascii_lowercase().contains("pir")
            || label.to_ascii_lowercase().contains("motion")
        {
            push_method(&mut methods, &mut seen, "onMotionDetected()");
        } else {
            push_method(&mut methods, &mut seen, "onStateChange()");
        }
        push_method(&mut methods, &mut seen, "calibrate()");
    } else if node_type == "wifi_manager" {
        push_method(&mut methods, &mut seen, "connectToNetwork()");
        push_method(&mut methods, &mut seen, "disconnect()");
        push_method(&mut methods, &mut seen, "getConnectionStatus()");
    } else if node_type == "mqtt_client" {
        push_method(&mut methods, &mut seen, "publishTelemetry()");
        push_method(&mut methods, &mut seen, "subscribeCommands()");
        push_method(&mut methods, &mut seen, "handleBrokerMessage()");
    } else if node_type.starts_with("i2c") {
        push_method(&mut methods, &mut seen, "readRegister()");
        push_method(&mut methods, &mut seen, "writeRegister()");
        push_method(&mut methods, &mut seen, "probeDevice()");
    } else {
        push_method(&mut methods, &mut seen, "initialize()");
        push_method(&mut methods, &mut seen, "process()");
    }

    if let Some(sym) = pn.source_refs.iter().find_map(|r| r.symbol.as_deref()) {
        let sig = format!("{sym}()");
        let key = sig.to_ascii_lowercase();
        if seen.insert(key) {
            methods.push(sig);
        }
    }

    let component_name = to_service_name(label, node_type);
    let tier = resolve_tier(node_type, &pn.id);
    json!({
        "componentName": component_name,
        "methods": methods,
        "tier": tier
    })
}

pub fn to_pascal_class_name(raw: &str) -> String {
    let cleaned: String = raw
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { ' ' })
        .collect();
    let parts: Vec<&str> = cleaned
        .split_whitespace()
        .filter(|p| !p.is_empty())
        .collect();
    if parts.is_empty() {
        return "Component".to_string();
    }
    parts
        .iter()
        .map(|p| capitalized_word(p))
        .collect::<Vec<_>>()
        .join("")
}

pub fn build_ldd_uml(pn: &PirNode, n: &FirmwareNode, label: &str) -> JsonValue {
    let class_name = to_pascal_class_name(label);
    let mut attrs: Vec<JsonValue> = Vec::new();
    let mut methods: Vec<JsonValue> = Vec::new();
    let mut seen_attr: HashSet<String> = HashSet::new();

    fn push_attr(attrs: &mut Vec<JsonValue>, seen: &mut HashSet<String>, name: &str, ty: &str) {
        let key = name.to_ascii_lowercase();
        if seen.insert(key) {
            attrs.push(json!({ "name": name, "type": ty }));
        }
    }

    if pn.properties.get("pin").is_some() || n.hardware.as_ref().and_then(|h| h.gpio).is_some() {
        push_attr(&mut attrs, &mut seen_attr, "pin", "gpio_num_t");
    }
    if let Some(hw) = &n.hardware {
        if hw.bus.is_some() {
            push_attr(&mut attrs, &mut seen_attr, "bus", "String");
        }
        if hw.i2c_address.is_some() {
            push_attr(&mut attrs, &mut seen_attr, "i2cAddress", "uint8_t");
        }
    }

    for (prop_key, attr_name, attr_type) in [
        ("task_name", "taskName", "String"),
        ("priority", "priority", "uint8_t"),
        ("stack_size", "stackSize", "uint32_t"),
        ("period_ms", "periodMs", "uint32_t"),
        ("ssid", "ssid", "String"),
        ("password", "password", "String"),
        ("broker_url", "brokerUrl", "String"),
        ("topic", "topic", "String"),
        ("target", "target", "String"),
        ("mode", "mode", "String"),
    ] {
        if let Some(v) = pn.properties.get(prop_key) {
            if !v.is_null() && v.as_str() != Some("") {
                push_attr(&mut attrs, &mut seen_attr, attr_name, attr_type);
            }
        }
    }

    if let Some(exec) = &n.execution {
        if exec.phase.is_some() {
            push_attr(&mut attrs, &mut seen_attr, "phase", "String");
        }
        if exec.period_ms.is_some() {
            push_attr(&mut attrs, &mut seen_attr, "periodMs", "uint32_t");
        }
    }

    if pn.ownership.primary_files.first().is_some() || pn.source_refs.first().is_some() {
        push_attr(&mut attrs, &mut seen_attr, "sourceFile", "String");
    }

    let node_type = pn.node_type.as_str();
    if node_type == "system_init" || pn.id == "boot" {
        methods.push(json!({ "name": "app_main", "signature": "app_main(): void" }));
        methods.push(json!({ "name": "initialize", "signature": "initialize(): void" }));
    } else if node_type == "rtos_task" {
        let task_name = prop_str(&pn.properties, "task_name").unwrap_or_else(|| "task".to_string());
        methods.push(json!({
            "name": task_name,
            "signature": format!("{task_name}(void* arg): void")
        }));
        methods.push(json!({ "name": "run", "signature": "run(): void" }));
    } else if node_type == "gpio_output" {
        methods.push(json!({ "name": "setLevel", "signature": "setLevel(level: int): void" }));
        methods.push(json!({ "name": "configure", "signature": "configure(): esp_err_t" }));
    } else if node_type == "gpio_input" || node_type == "sensor_input" {
        methods.push(json!({ "name": "read", "signature": "read(): int" }));
        let ll = label.to_ascii_lowercase();
        if ll.contains("ir") || ll.contains("pir") || ll.contains("motion") || ll.contains("sensor")
        {
            methods.push(json!({
                "name": "onMotionDetected",
                "signature": "onMotionDetected(): void"
            }));
        }
    } else if node_type == "wifi_manager" {
        methods.push(json!({ "name": "connect", "signature": "connect(): bool" }));
        methods.push(json!({ "name": "disconnect", "signature": "disconnect(): void" }));
    } else if node_type == "mqtt_client" {
        methods.push(json!({
            "name": "publish",
            "signature": "publish(topic: String, payload: String): void"
        }));
        methods.push(json!({
            "name": "subscribe",
            "signature": "subscribe(topic: String): void"
        }));
    } else if let Some(sym) = pn.source_refs.iter().find_map(|r| r.symbol.as_deref()) {
        methods.push(json!({
            "name": sym,
            "signature": format!("{sym}(): void")
        }));
    } else {
        methods.push(json!({ "name": "init", "signature": "init(): esp_err_t" }));
    }

    let stereotype = if node_type == "wifi_manager" || node_type == "mqtt_client" {
        Some("service")
    } else if node_type.starts_with("gpio_") || node_type == "sensor_input" {
        Some("peripheral")
    } else {
        None
    };

    if seen_attr.is_empty() {
        push_attr(&mut attrs, &mut seen_attr, "id", "String");
    }

    json!({
        "className": class_name,
        "stereotype": stereotype,
        "attributes": attrs,
        "methods": methods
    })
}

pub fn infer_association_label(
    source_type: &str,
    target_type: &str,
    edge_kind: Option<&PirEdgeKind>,
    semantic_label: Option<&str>,
) -> String {
    if let Some(l) = semantic_label {
        let t = l.trim();
        if !t.is_empty() {
            return t.to_string();
        }
    }

    if let Some(kind) = edge_kind {
        match kind {
            PirEdgeKind::Execution | PirEdgeKind::Fsm => {
                if source_type == "system_init" {
                    return "initializes".to_string();
                }
                if source_type == "rtos_task" && target_type == "gpio_output" {
                    return "controls".to_string();
                }
                if source_type == "rtos_task" {
                    return "invokes".to_string();
                }
                if source_type == "sensor_input" {
                    return "triggers".to_string();
                }
                return "executes".to_string();
            }
            PirEdgeKind::Data => {
                if source_type == "sensor_input" {
                    return "reads".to_string();
                }
                return "dataFlow".to_string();
            }
            PirEdgeKind::Hardware => return "uses".to_string(),
            PirEdgeKind::Network => return "connects".to_string(),
            PirEdgeKind::Dependency => return "dependsOn".to_string(),
            PirEdgeKind::Event => return "triggers".to_string(),
            PirEdgeKind::Ota => return "ota".to_string(),
        }
    }

    if source_type == "sensor_input" && target_type == "rtos_task" {
        return "triggers".to_string();
    }
    if source_type == "rtos_task" && target_type == "gpio_output" {
        return "controls".to_string();
    }
    if source_type == "system_init" {
        return "bootstraps".to_string();
    }
    if target_type == "wifi_manager" || target_type == "mqtt_client" {
        return "uses".to_string();
    }
    "associates".to_string()
}

pub fn format_hld_interaction_label(
    technical: &str,
    source_type: &str,
    target_type: &str,
    target_label: &str,
) -> String {
    let key = technical.trim().to_ascii_lowercase().replace(' ', "_");
    let contextual: [(&str, &str); 12] = [
        ("triggers", "Forward Sensor Event"),
        ("controls", "Control"),
        ("initializes", "Initialize"),
        ("spawns", "Start"),
        ("invokes", "Invoke"),
        ("reads", "Read Sensor Data"),
        ("dataflow", "Transfer Data"),
        ("uses", "Use"),
        ("connects", "Connect to"),
        ("dependson", "Depends on"),
        ("executes", "Execute Flow"),
        ("bootstraps", "Bootstrap System"),
    ];
    for (k, prefix) in contextual {
        if key == k {
            if prefix.contains("Control")
                || prefix.contains("Initialize")
                || prefix.contains("Start")
                || prefix.contains("Invoke")
                || prefix.contains("Use")
                || prefix.contains("Connect")
                || prefix.contains("Depends")
            {
                return format!("{prefix} {target_label}");
            }
            return prefix.to_string();
        }
    }

    if source_type == "sensor_input" && target_type == "rtos_task" {
        return "Forward Sensor Event".to_string();
    }
    if source_type == "rtos_task" && target_type == "gpio_output" {
        return format!("Drive {target_label}");
    }
    if source_type == "system_init" && target_type == "rtos_task" {
        return format!("Start {target_label}");
    }
    if target_type == "wifi_manager" {
        return "Connect Network".to_string();
    }
    if target_type == "mqtt_client" {
        return "Publish / Subscribe".to_string();
    }
    if source_type == "wifi_manager" && target_type == "mqtt_client" {
        return "Open MQTT Session".to_string();
    }

    technical
        .replace('_', " ")
        .split_whitespace()
        .map(capitalized_word)
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn infer_hld_interaction_label(
    source_type: &str,
    target_type: &str,
    _source_label: &str,
    target_label: &str,
    edge: Option<&PirEdge>,
) -> String {
    let technical = infer_association_label(
        source_type,
        target_type,
        edge.map(|e| &e.kind),
        edge.and_then(|e| e.semantic_label.as_deref()),
    );
    format_hld_interaction_label(&technical, source_type, target_type, target_label)
}

pub fn build_hld_edge_labels(
    base: &crate::firmware_topology::types::FirmwareGraph,
    pir: &PirDocument,
    flavor: &str,
) -> HashMap<String, String> {
    let port_to_node = map_ports_to_node(base);
    let node_by_id: HashMap<&str, &FirmwareNode> =
        base.nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    let pir_by_id: HashMap<&str, &PirNode> = pir.nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    let mut pir_edge_by_pair: HashMap<String, &PirEdge> = HashMap::new();
    for e in &pir.edges {
        pir_edge_by_pair
            .entry(format!("{}->{}", e.source_node_id, e.target_node_id))
            .or_insert(e);
    }

    let mut labels = HashMap::new();
    for [src_port, dst_port] in &base.connections {
        let Some(src_node_id) = port_to_node.get(src_port.as_str()) else {
            continue;
        };
        let Some(dst_node_id) = port_to_node.get(dst_port.as_str()) else {
            continue;
        };
        if src_node_id == dst_node_id {
            continue;
        }
        let src_fw = node_by_id.get(src_node_id);
        let dst_fw = node_by_id.get(dst_node_id);
        let (Some(src_fw), Some(dst_fw)) = (src_fw, dst_fw) else {
            continue;
        };
        let src_type = src_fw.node_type.as_str();
        let dst_type = dst_fw.node_type.as_str();
        let src_label = src_fw.label.as_deref().unwrap_or(&src_fw.node_type);
        let dst_label = dst_fw.label.as_deref().unwrap_or(&dst_fw.node_type);
        let pir_edge = pir_edge_by_pair.get(&format!("{src_node_id}->{dst_node_id}"));
        let lbl = infer_hld_interaction_label(
            src_type,
            dst_type,
            src_label,
            dst_label,
            pir_edge.copied(),
        );
        let new_src = assoc_port_id(src_node_id, flavor, "out");
        let new_dst = assoc_port_id(dst_node_id, flavor, "in");
        labels.insert(format!("{new_src}|{new_dst}"), lbl);
    }
    labels
}

pub fn build_ldd_edge_labels(
    base: &crate::firmware_topology::types::FirmwareGraph,
    pir: &PirDocument,
    flavor: &str,
) -> HashMap<String, String> {
    let port_to_node = map_ports_to_node(base);
    let node_type_by_id: HashMap<&str, &str> = base
        .nodes
        .iter()
        .map(|n| (n.id.as_str(), n.node_type.as_str()))
        .collect();
    let mut pir_edge_by_pair: HashMap<String, &PirEdge> = HashMap::new();
    for e in &pir.edges {
        pir_edge_by_pair
            .entry(format!("{}->{}", e.source_node_id, e.target_node_id))
            .or_insert(e);
    }

    let mut labels = HashMap::new();
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
        let src_type = node_type_by_id.get(src_node).copied().unwrap_or("");
        let dst_type = node_type_by_id.get(dst_node).copied().unwrap_or("");
        let pir_edge = pir_edge_by_pair.get(&format!("{src_node}->{dst_node}"));
        let lbl = infer_association_label(
            src_type,
            dst_type,
            pir_edge.map(|e| &e.kind),
            pir_edge.and_then(|e| e.semantic_label.as_deref()),
        );
        let new_src = assoc_port_id(src_node, flavor, "out");
        let new_dst = assoc_port_id(dst_node, flavor, "in");
        labels.insert(format!("{new_src}|{new_dst}"), lbl);
    }
    labels
}

fn map_ports_to_node(
    graph: &crate::firmware_topology::types::FirmwareGraph,
) -> HashMap<&str, &str> {
    let mut out = HashMap::new();
    for n in &graph.nodes {
        for p in &n.ports {
            out.insert(p.id.as_str(), n.id.as_str());
        }
    }
    out
}

pub fn sequence_node_order(pir: &PirDocument) -> Vec<String> {
    let mut order = Vec::new();
    let mut seen = HashSet::new();
    let node_type_by_id: HashMap<&str, &str> = pir
        .nodes
        .iter()
        .map(|n| (n.id.as_str(), n.node_type.as_str()))
        .collect();

    let start = pir
        .nodes
        .iter()
        .find(|n| n.id == "boot" || n.node_type == "system_init")
        .map(|n| n.id.clone())
        .or_else(|| pir.layers.system.first().cloned());

    if let Some(start_id) = start {
        let mut queue = VecDeque::new();
        queue.push_back(start_id);
        while let Some(id) = queue.pop_front() {
            if !seen.insert(id.clone()) {
                continue;
            }
            let id_for_edges = id.clone();
            order.push(id);
            for e in &pir.edges {
                if e.source_node_id != id_for_edges {
                    continue;
                }
                if matches!(
                    e.kind,
                    PirEdgeKind::Execution | PirEdgeKind::Data | PirEdgeKind::Event
                ) {
                    let src_type = node_type_by_id
                        .get(e.source_node_id.as_str())
                        .copied()
                        .unwrap_or("");
                    let dst_type = node_type_by_id
                        .get(e.target_node_id.as_str())
                        .copied()
                        .unwrap_or("");
                    if e.kind == PirEdgeKind::Execution
                        && src_type == "rtos_task"
                        && matches!(dst_type, "sensor_input" | "gpio_input" | "adc_reader")
                    {
                        // Skip task->sensor init wiring when deriving sequence runtime order.
                        continue;
                    }
                    queue.push_back(e.target_node_id.clone());
                }
            }
        }
    }

    for layer_ids in [
        &pir.layers.system,
        &pir.layers.runtime,
        &pir.layers.physical,
        &pir.layers.network,
    ] {
        for id in layer_ids {
            if seen.insert(id.clone()) {
                order.push(id.clone());
            }
        }
    }

    let mut rest: Vec<String> = pir
        .nodes
        .iter()
        .filter(|n| !seen.contains(&n.id))
        .map(|n| n.id.clone())
        .collect();
    rest.sort();
    for id in rest {
        if seen.insert(id.clone()) {
            order.push(id);
        }
    }

    order
}
