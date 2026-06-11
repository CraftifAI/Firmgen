use std::collections::{HashMap, HashSet};

use super::registry::{are_datatypes_compatible, get_node_type_def};
use super::types::{FirmwareGraph, PortDirection, ValidationIssue, ValidationReport};

pub fn validate_graph(graph: &FirmwareGraph) -> ValidationReport {
    let mut issues = Vec::new();

    if graph.schema_version != super::types::SCHEMA_VERSION {
        issues.push(issue(
            "schema_version_mismatch",
            format!(
                "Expected schema_version {}, got {}",
                super::types::SCHEMA_VERSION,
                graph.schema_version
            ),
            "error",
            None,
            None,
            None,
        ));
    }

    let mut node_ids = HashSet::new();
    for node in &graph.nodes {
        if !node_ids.insert(node.id.clone()) {
            issues.push(issue(
                "duplicate_node_id",
                format!("Duplicate node id '{}'", node.id),
                "error",
                Some(node.id.clone()),
                None,
                None,
            ));
        }

        if get_node_type_def(&node.node_type).is_none() {
            issues.push(issue(
                "unknown_node_type",
                format!("Unknown node type '{}'", node.node_type),
                "warning",
                Some(node.id.clone()),
                None,
                None,
            ));
        }

        let mut port_ids = HashSet::new();
        for port in &node.ports {
            if !port_ids.insert(port.id.clone()) {
                issues.push(issue(
                    "duplicate_port_id",
                    format!("Duplicate port id '{}' on node '{}'", port.id, node.id),
                    "error",
                    Some(node.id.clone()),
                    Some(port.id.clone()),
                    None,
                ));
            }
        }
    }

    let port_index = graph.port_index();
    let mut gpio_usage: HashMap<u8, Vec<String>> = HashMap::new();

    for node in &graph.nodes {
        let mut pins_for_node: Vec<u8> = Vec::new();
        if let Some(pin) = node.properties.get("pin").and_then(|v| v.as_u64()) {
            if pin <= 48 {
                pins_for_node.push(pin as u8);
            }
        }
        if let Some(hw) = &node.hardware {
            if let Some(gpio) = hw.gpio {
                if !pins_for_node.contains(&gpio) {
                    pins_for_node.push(gpio);
                }
            }
        }
        for pin in pins_for_node {
            gpio_usage.entry(pin).or_default().push(node.id.clone());
        }
    }

    for (pin, nodes) in &gpio_usage {
        if nodes.len() > 1 {
            for node_id in nodes {
                issues.push(issue(
                    "gpio_conflict",
                    format!(
                        "GPIO {} also used by: {}",
                        pin,
                        nodes
                            .iter()
                            .filter(|id| *id != node_id)
                            .cloned()
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                    "error",
                    Some(node_id.clone()),
                    None,
                    None,
                ));
            }
        }
    }

    for conn in &graph.connections {
        let src_id = &conn[0];
        let dst_id = &conn[1];

        let Some((src_node_id, src_port)) = port_index.get(src_id) else {
            issues.push(issue(
                "unknown_source_port",
                format!("Unknown source port '{}'", src_id),
                "error",
                None,
                Some(src_id.clone()),
                Some(conn.clone()),
            ));
            continue;
        };

        let Some((dst_node_id, dst_port)) = port_index.get(dst_id) else {
            issues.push(issue(
                "unknown_target_port",
                format!("Unknown target port '{}'", dst_id),
                "error",
                None,
                Some(dst_id.clone()),
                Some(conn.clone()),
            ));
            continue;
        };

        if src_port.direction != PortDirection::Output {
            issues.push(issue(
                "invalid_source_direction",
                format!("Port '{}' must be an output port", src_id),
                "error",
                Some(src_node_id.clone()),
                Some(src_id.clone()),
                Some(conn.clone()),
            ));
        }

        if dst_port.direction != PortDirection::Input {
            issues.push(issue(
                "invalid_target_direction",
                format!("Port '{}' must be an input port", dst_id),
                "error",
                Some(dst_node_id.clone()),
                Some(dst_id.clone()),
                Some(conn.clone()),
            ));
        }

        let src_dtype = src_port.datatype.as_deref().unwrap_or("any");
        let dst_dtype = dst_port.datatype.as_deref().unwrap_or("any");
        if !are_datatypes_compatible(src_dtype, dst_dtype) {
            issues.push(issue(
                "datatype_mismatch",
                format!(
                    "Incompatible datatypes: {} ({}) -> {} ({})",
                    src_id, src_dtype, dst_id, dst_dtype
                ),
                "error",
                Some(dst_node_id.clone()),
                Some(dst_id.clone()),
                Some(conn.clone()),
            ));
        }
    }

    if let Some(cycle_nodes) = detect_execution_cycles(graph) {
        issues.push(issue(
            "cyclic_execution",
            format!(
                "Cyclic execution path detected involving: {}",
                cycle_nodes.join(" -> ")
            ),
            "error",
            None,
            None,
            None,
        ));
    }

    for node in &graph.nodes {
        if let Some(def) = get_node_type_def(&node.node_type) {
            for req_port in def.ports.iter().filter(|p| p.required.unwrap_or(false)) {
                let has_port = node.ports.iter().any(|p| p.name == req_port.name);
                if !has_port {
                    issues.push(issue(
                        "missing_required_port",
                        format!(
                            "Node '{}' missing required port '{}'",
                            node.id, req_port.name
                        ),
                        "warning",
                        Some(node.id.clone()),
                        None,
                        None,
                    ));
                }
            }
        }
    }

    let has_errors = issues.iter().any(|i| i.severity == "error");
    ValidationReport {
        valid: !has_errors,
        issues,
    }
}

fn issue(
    code: &str,
    message: String,
    severity: &str,
    node_id: Option<String>,
    port_id: Option<String>,
    connection: Option<[String; 2]>,
) -> ValidationIssue {
    ValidationIssue {
        code: code.to_string(),
        message,
        severity: severity.to_string(),
        node_id,
        port_id,
        connection,
    }
}

/// Detect cycles in the execution/dataflow graph using port-level adjacency.
fn detect_execution_cycles(graph: &FirmwareGraph) -> Option<Vec<String>> {
    let port_to_node: HashMap<&str, &str> = graph
        .nodes
        .iter()
        .flat_map(|n| n.ports.iter().map(|p| (p.id.as_str(), n.id.as_str())))
        .collect();

    let mut adj: HashMap<String, Vec<String>> = HashMap::new();
    for conn in &graph.connections {
        if let (Some(src_node), Some(dst_node)) = (
            port_to_node.get(conn[0].as_str()),
            port_to_node.get(conn[1].as_str()),
        ) {
            adj.entry(src_node.to_string())
                .or_default()
                .push(dst_node.to_string());
        }
    }

    let mut visited = HashSet::new();
    let mut stack = HashSet::new();
    let mut path = Vec::new();

    for node in &graph.nodes {
        if dfs_cycle(&node.id, &adj, &mut visited, &mut stack, &mut path) {
            return Some(path);
        }
    }
    None
}

fn dfs_cycle(
    node: &str,
    adj: &HashMap<String, Vec<String>>,
    visited: &mut HashSet<String>,
    stack: &mut HashSet<String>,
    path: &mut Vec<String>,
) -> bool {
    if stack.contains(node) {
        path.push(node.to_string());
        return true;
    }
    if visited.contains(node) {
        return false;
    }
    visited.insert(node.to_string());
    stack.insert(node.to_string());
    path.push(node.to_string());

    if let Some(neighbors) = adj.get(node) {
        for next in neighbors {
            if dfs_cycle(next, adj, visited, stack, path) {
                return true;
            }
        }
    }

    stack.remove(node);
    path.pop();
    false
}
