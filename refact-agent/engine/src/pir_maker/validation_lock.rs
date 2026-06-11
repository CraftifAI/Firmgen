//! Lock PIR node properties that fail graph validation or board constraints.

use crate::firmware_topology::ValidationReport;

use super::schema::PirDocument;

/// Remove property keys from `editable_fields` when editing them would keep the graph invalid.
pub fn apply_editable_field_locks(pir: &mut PirDocument, validation: &ValidationReport) {
    for issue in &validation.issues {
        if issue.severity != "error" {
            continue;
        }
        let keys = property_keys_for_issue(&issue.code);
        if keys.is_empty() {
            continue;
        }
        if let Some(node_id) = &issue.node_id {
            lock_keys_on_node(pir, node_id, &keys);
        }
    }

    // Board-restricted pins are already removed from editable_fields in board_validate.
    restore_editable_defaults(pir);
}

fn restore_editable_defaults(pir: &mut PirDocument) {
    use crate::firmware_topology::registry::default_editable_for_type;
    for node in &mut pir.nodes {
        if !node.editable_fields.is_empty() {
            continue;
        }
        let board_restricted = node
            .stale_reason
            .as_deref()
            .map(|r| r.to_lowercase().contains("restricted"))
            .unwrap_or(false);
        if board_restricted {
            continue;
        }
        node.editable_fields = default_editable_for_type(&node.node_type);
    }
}

fn lock_keys_on_node(pir: &mut PirDocument, node_id: &str, keys: &[&str]) {
    let Some(node) = pir.nodes.iter_mut().find(|n| n.id == node_id) else {
        return;
    };
    node.editable_fields
        .retain(|f| !keys.iter().any(|k| k == &f.as_str()));
}

fn property_keys_for_issue(code: &str) -> Vec<&'static str> {
    match code {
        // gpio_conflict is user-fixable by changing pin; do not lock it here.
        "gpio_conflict" => vec![],
        "invalid_source_direction" | "invalid_target_direction" | "datatype_mismatch" => {
            vec![]
        }
        "unknown_node_type" => vec![],
        "duplicate_node_id" | "duplicate_port_id" => vec![],
        "unknown_source_port" | "unknown_target_port" => vec![],
        "cyclic_execution" => vec![],
        "schema_version_mismatch" => vec![],
        "missing_required_port" => vec![],
        _ => vec![],
    }
}
