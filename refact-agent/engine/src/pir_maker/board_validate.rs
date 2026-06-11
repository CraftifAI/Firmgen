//! Validate PIR GPIO assignments against board context profile.

use serde_json::Value as JsonValue;

use super::schema::{PirDocument, PirSyncState};

pub fn apply_board_validation(
    pir: &mut PirDocument,
    board_profile: Option<&str>,
    board_id: Option<&str>,
) {
    let Some(readme) = board_profile else {
        if let Some(bid) = board_id {
            pir.provenance.board_id = Some(bid.to_string());
        }
        return;
    };
    let Ok(profile) = serde_json::from_str::<JsonValue>(readme) else {
        return;
    };

    let restricted = extract_restricted_pins(&profile);
    if restricted.is_empty() {
        if let Some(bid) = board_id {
            pir.provenance.board_id = Some(bid.to_string());
        }
        return;
    }

    let mut warnings = pir
        .summary
        .as_ref()
        .map(|s| s.warnings.clone())
        .unwrap_or_default();

    for node in &mut pir.nodes {
        if let Some(pin) = node.properties.get("pin").and_then(|v| v.as_u64()) {
            let pin = pin as u8;
            if restricted.contains(&pin) {
                node.stale_reason = Some(format!("GPIO {} restricted by board profile", pin));
                node.sync.state = PirSyncState::Conflict;
                node.editable_fields.retain(|f| f != "pin");
                warnings.push(format!(
                    "Node {} uses restricted GPIO {} (pin locked)",
                    node.id, pin
                ));
            }
        }
    }

    if let Some(summary) = pir.summary.as_mut() {
        summary.warnings = warnings;
    }

    if let Some(bid) = board_id {
        pir.provenance.board_id = Some(bid.to_string());
    }
}

fn extract_restricted_pins(profile: &JsonValue) -> Vec<u8> {
    let mut pins = Vec::new();
    if let Some(arr) = profile
        .get("gpio")
        .and_then(|g| g.get("restricted_pins"))
        .and_then(|v| v.as_array())
    {
        for v in arr {
            if let Some(n) = v.as_u64() {
                pins.push(n as u8);
            }
        }
    }
    pins
}
