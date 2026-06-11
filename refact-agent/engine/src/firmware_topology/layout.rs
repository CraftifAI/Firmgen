use std::collections::{HashMap, VecDeque};

use super::types::{FirmwareGraph, LayoutConfig, LayoutPosition, LayoutResultResponse};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutOrientation {
    Horizontal,
    Vertical,
}

impl LayoutOrientation {
    pub fn from_str(s: &str) -> Self {
        match s {
            "vertical" => Self::Vertical,
            _ => Self::Horizontal,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Horizontal => "horizontal",
            Self::Vertical => "vertical",
        }
    }
}

pub struct LayoutResult {
    pub positions: Vec<LayoutPosition>,
    pub has_cycles: bool,
    pub orientation: LayoutOrientation,
}

const DEFAULT_NODE_HEIGHT: f64 = 200.0;

pub fn compute_layout(graph: &FirmwareGraph, config: Option<&LayoutConfig>) -> LayoutResult {
    let cfg = config.cloned().unwrap_or_default();
    let orientation = LayoutOrientation::from_str(&cfg.orientation);
    let node_width = cfg.node_width;
    let layer_gap = cfg.layer_gap;
    let node_gap = cfg.node_gap;

    let port_to_node: HashMap<&str, &str> = graph
        .nodes
        .iter()
        .flat_map(|n| n.ports.iter().map(|p| (p.id.as_str(), n.id.as_str())))
        .collect();

    let mut children: HashMap<String, Vec<String>> = HashMap::new();
    let mut in_degree: HashMap<String, u32> = HashMap::new();

    for node in &graph.nodes {
        in_degree.entry(node.id.clone()).or_insert(0);
    }

    for conn in &graph.connections {
        if let (Some(src), Some(dst)) = (
            port_to_node.get(conn[0].as_str()),
            port_to_node.get(conn[1].as_str()),
        ) {
            if src != dst {
                children
                    .entry(src.to_string())
                    .or_default()
                    .push(dst.to_string());
                *in_degree.entry(dst.to_string()).or_insert(0) += 1;
            }
        }
    }

    let mut layer: HashMap<String, u32> = HashMap::new();
    let mut placed: HashMap<String, ()> = HashMap::new();
    let mut queue: VecDeque<String> = VecDeque::new();
    let mut has_cycles = false;

    for (id, deg) in &in_degree {
        if *deg == 0 {
            queue.push_back(id.clone());
            layer.insert(id.clone(), 0);
        }
    }

    let mut processed = 0u32;
    while let Some(node_id) = queue.pop_front() {
        processed += 1;
        placed.insert(node_id.clone(), ());
        let current_layer = *layer.get(&node_id).unwrap_or(&0);
        if let Some(kids) = children.get(&node_id) {
            for child in kids {
                let next_layer = current_layer + 1;
                let entry = layer.entry(child.clone()).or_insert(0);
                *entry = (*entry).max(next_layer);
                let deg = in_degree.get_mut(child).unwrap();
                *deg = deg.saturating_sub(1);
                if *deg == 0 {
                    queue.push_back(child.clone());
                }
            }
        }
    }

    if processed < graph.nodes.len() as u32 {
        has_cycles = true;
        let per_row: u32 = 4;
        let mut extra: u32 = 0;
        for node in &graph.nodes {
            if placed.contains_key(&node.id) {
                continue;
            }
            layer.insert(node.id.clone(), extra / per_row);
            extra += 1;
        }
    }

    let max_layer = layer.values().copied().max().unwrap_or(0);
    let mut layers: Vec<Vec<String>> = vec![Vec::new(); (max_layer + 1) as usize];
    for node in &graph.nodes {
        let l = *layer.get(&node.id).unwrap_or(&0) as usize;
        layers[l].push(node.id.clone());
    }

    let mut positions = Vec::new();
    for (layer_idx, nodes_in_layer) in layers.iter().enumerate() {
        let count = nodes_in_layer.len() as f64;
        let total_h = count * DEFAULT_NODE_HEIGHT + (count - 1.0).max(0.0) * node_gap;

        for (i, node_id) in nodes_in_layer.iter().enumerate() {
            let (x, y) = match orientation {
                LayoutOrientation::Horizontal => {
                    let x = layer_idx as f64 * (node_width + layer_gap);
                    let y = i as f64 * (DEFAULT_NODE_HEIGHT + node_gap) - total_h / 2.0;
                    (x, y)
                }
                LayoutOrientation::Vertical => {
                    let x = i as f64 * (node_width + node_gap) - (count * node_width) / 2.0;
                    let y = layer_idx as f64 * (DEFAULT_NODE_HEIGHT + layer_gap);
                    (x, y)
                }
            };
            positions.push(LayoutPosition {
                node_id: node_id.clone(),
                x,
                y,
                layer: layer_idx as u32,
            });
        }
    }

    LayoutResult {
        positions,
        has_cycles,
        orientation,
    }
}

impl From<LayoutResult> for LayoutResultResponse {
    fn from(r: LayoutResult) -> Self {
        LayoutResultResponse {
            positions: r.positions,
            has_cycles: r.has_cycles,
            orientation: r.orientation.as_str().to_string(),
        }
    }
}
