//! Derive unresolved topology hints from main-chat user intent.

use std::collections::HashSet;

use serde_json::json;

use super::super::schema::AnalysisFacts;
use super::static_extract::is_app_config_path;

#[derive(Debug, Clone)]
struct ExpectedComponent {
    node_type: String,
    label: String,
    id_base: String,
    match_score: u32,
}

fn is_stop_word(word: &str) -> bool {
    matches!(
        word,
        "the"
            | "and"
            | "with"
            | "for"
            | "from"
            | "then"
            | "that"
            | "this"
            | "into"
            | "over"
            | "under"
            | "node"
            | "graph"
            | "project"
            | "firmware"
            | "esp32"
            | "manager"
            | "client"
            | "device"
            | "generic"
            | "input"
            | "output"
    )
}

fn tokenize(text: &str) -> HashSet<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .filter(|w| w.len() >= 3 || matches!(*w, "i2c" | "spi" | "adc" | "ota"))
        .filter(|w| !is_stop_word(w))
        .map(str::to_string)
        .collect()
}

fn tokens_for_node_type(label: &str, node_type: &str, description: &str) -> HashSet<String> {
    let mut merged = HashSet::new();
    for token in tokenize(label)
        .into_iter()
        .chain(tokenize(node_type))
        .chain(tokenize(description))
    {
        merged.insert(token);
    }
    merged
}

fn chat_hintable_node_type(node_type: &str) -> bool {
    matches!(
        node_type,
        "wifi_manager"
            | "mqtt_client"
            | "ble_manager"
            | "http_client"
            | "websocket_client"
            | "rtos_task"
            | "gpio_input"
            | "gpio_output"
            | "pwm_output"
            | "sensor_input"
            | "adc_reader"
    )
}

fn threshold_for_node_type(node_type: &str) -> u32 {
    match node_type {
        "wifi_manager" | "mqtt_client" | "ble_manager" | "http_client" | "websocket_client"
        | "rtos_task" => 1,
        _ => 2,
    }
}

/// Scan user chat text for firmware capabilities the topology should include.
pub fn expected_components_from_chat(text: &str) -> Vec<ExpectedComponent> {
    let prompt_tokens = tokenize(text);
    if prompt_tokens.is_empty() {
        return Vec::new();
    }

    let mut out: Vec<ExpectedComponent> = Vec::new();
    for def in crate::firmware_topology::registry::list_node_types() {
        if !chat_hintable_node_type(&def.node_type) {
            continue;
        }

        let def_tokens = tokens_for_node_type(&def.label, &def.node_type, &def.description);
        let overlap = prompt_tokens.intersection(&def_tokens).count() as u32;
        let exact = if text.to_lowercase().contains(&def.node_type) {
            2
        } else {
            0
        };
        let score = overlap + exact;
        if score < threshold_for_node_type(&def.node_type) {
            continue;
        }

        out.push(ExpectedComponent {
            node_type: def.node_type.clone(),
            label: def.label.clone(),
            id_base: def.node_type.clone(),
            match_score: score,
        });
    }

    // If prompt describes command-driven behavior over network, ensure a task exists.
    let has_command_plane_intent = prompt_tokens.contains("command")
        || prompt_tokens.contains("control")
        || prompt_tokens.contains("state")
        || prompt_tokens.contains("fsm")
        || prompt_tokens.contains("callback");
    let has_network_stack_intent = out.iter().any(|c| is_network_node_type(&c.node_type));
    let has_task_intent = out.iter().any(|c| c.node_type == "rtos_task");
    if has_command_plane_intent && has_network_stack_intent && !has_task_intent {
        out.push(ExpectedComponent {
            node_type: "rtos_task".to_string(),
            label: "RTOS Task".to_string(),
            id_base: "control_task".to_string(),
            match_score: 1,
        });
    }

    out.sort_by(|a, b| {
        b.match_score
            .cmp(&a.match_score)
            .then_with(|| a.node_type.cmp(&b.node_type))
    });
    out
}

fn has_node_type(facts: &AnalysisFacts, node_type: &str) -> bool {
    facts.gpio_facts.iter().any(|g| g.node_type == node_type)
        || facts.network_facts.iter().any(|n| n.node_type == node_type)
        || facts.task_facts.iter().any(|_| node_type == "rtos_task")
}

fn is_network_node_type(node_type: &str) -> bool {
    matches!(
        node_type,
        "wifi_manager" | "mqtt_client" | "ble_manager" | "http_client" | "websocket_client"
    )
}

/// Add unresolved hints from chat context when not present in static extraction.
///
/// Strict evidence policy: chat context must never mutate canonical extraction facts.
pub fn fill_gaps_from_chat(
    chat_context: Option<&str>,
    facts: &mut AnalysisFacts,
    app_config_rel: Option<&str>,
) {
    let Some(text) = chat_context.map(str::trim).filter(|t| !t.is_empty()) else {
        return;
    };

    let expected = expected_components_from_chat(text);
    if expected.is_empty() {
        return;
    }

    let source_file = app_config_rel
        .map(String::from)
        .or_else(|| find_app_config_in_manifest(facts))
        .unwrap_or_else(|| "main/app_config.h".to_string());

    for comp in expected {
        if has_node_type(facts, &comp.node_type) {
            continue;
        }
        facts.unresolved.push(json!({
            "kind": "chat_component_hint",
            "message": format!("Chat mentions {} but file-backed evidence is missing; add APP_* config or source usage before topology can include it", comp.label),
            "node_type": comp.node_type,
            "id_hint": comp.id_base,
            "confidence": comp.match_score,
            "file": source_file,
            "strict_evidence": true,
            "inferred_from": "chat_context",
        }));
    }
}

fn find_app_config_in_manifest(facts: &AnalysisFacts) -> Option<String> {
    facts
        .analyzed_files
        .iter()
        .find(|r| is_app_config_path(r))
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::fill_gaps_from_chat;
    use crate::pir_maker::schema::AnalysisFacts;

    #[test]
    fn chat_gap_only_adds_unresolved_hints() {
        let mut facts = AnalysisFacts {
            project_name: "demo".to_string(),
            analyzed_files: vec!["main/app_config.h".to_string()],
            ..Default::default()
        };

        fill_gaps_from_chat(
            Some("Use BLE manager for communication and control"),
            &mut facts,
            Some("main/app_config.h"),
        );

        assert!(
            facts.network_facts.is_empty(),
            "chat context must not create network facts"
        );
        assert!(
            facts.task_facts.is_empty(),
            "chat context must not create task facts"
        );
        assert!(
            facts.gpio_facts.is_empty(),
            "chat context must not create gpio facts"
        );
        assert!(
            facts
                .unresolved
                .iter()
                .any(|u| u.get("kind").and_then(|v| v.as_str()) == Some("chat_component_hint")),
            "chat hints should be recorded in unresolved[]"
        );
    }
}
