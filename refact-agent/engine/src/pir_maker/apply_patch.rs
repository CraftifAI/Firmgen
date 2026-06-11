//! Apply node property edits back to project source files.

use std::path::Path;

use regex::Regex;
use serde_json::{json, Value as JsonValue};

use serde::Deserialize;

use super::schema::{
    PirChangeEntry, PirDocument, PirEdge, PirNode, PirApprovalStatus, NodeAuthority, PirSyncState,
};

#[derive(Debug, Clone)]
pub struct PatchResult {
    pub files_patched: Vec<String>,
    pub change_entry: PirChangeEntry,
}

pub fn apply_node_property_patch(
    project_root: &Path,
    pir: &mut PirDocument,
    node_id: &str,
    property_updates: &serde_json::Map<String, JsonValue>,
    expected_revision: Option<&str>,
) -> Result<PatchResult, String> {
    if let Some(expected) = expected_revision {
        if expected != pir.revision {
            return Err(format!(
                "revision mismatch: expected {}, current {}",
                expected, pir.revision
            ));
        }
    }

    let node_idx = pir
        .nodes
        .iter()
        .position(|n| n.id == node_id)
        .ok_or_else(|| format!("unknown node_id '{}'", node_id))?;

    let mut files_patched = Vec::new();
    let mut old_values = serde_json::Map::new();
    let mut new_values = serde_json::Map::new();

    let analyzed_files = pir.provenance.analyzed_files.clone();
    let target_files = resolve_target_files(project_root, &pir.nodes[node_idx], &analyzed_files);

    let allowed_editable: std::collections::HashSet<String> =
        if pir.nodes[node_idx].editable_fields.is_empty() {
            crate::firmware_topology::registry::default_editable_for_type(
                &pir.nodes[node_idx].node_type,
            )
            .into_iter()
            .collect()
        } else {
            pir.nodes[node_idx]
                .editable_fields
                .iter()
                .cloned()
                .collect()
        };

    for (key, new_val) in property_updates {
        if !allowed_editable.contains(key) {
            return Err(format!(
                "property '{}' is locked or not editable on node '{}'",
                key, node_id
            ));
        }

        let old_val = property_get(&pir.nodes[node_idx].properties, key);
        old_values.insert(key.clone(), old_val.clone());
        new_values.insert(key.clone(), new_val.clone());

        match key.as_str() {
            "pin" => {
                let new_pin = new_val.as_u64().ok_or("pin must be an integer")? as u8;
                let old_pin = old_val.as_u64().map(|p| p as u8);
                if let Some(app_cfg) = find_app_config_path(project_root, &analyzed_files) {
                    let abs = project_root.join(&app_cfg);
                    if abs.is_file() && patch_app_config_gpio(&abs, node_id, old_pin, new_pin)? {
                        files_patched.push(app_cfg);
                    }
                }
                for file in &target_files {
                    let abs = project_root.join(file);
                    if abs.is_file() {
                        let changed = patch_gpio_pin_in_file(&abs, old_pin, new_pin)?;
                        if changed {
                            files_patched.push(file.clone());
                        }
                    }
                }
            }
            "period_ms" | "priority" | "stack_size" | "task_name" => {
                let task_name = property_get(&pir.nodes[node_idx].properties, "task_name")
                    .as_str()
                    .map(|s| s.to_string())
                    .or_else(|| {
                        new_values
                            .get("task_name")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string())
                    });
                for file in &target_files {
                    let abs = project_root.join(file);
                    if abs.is_file() {
                        if patch_rtos_property_in_file(&abs, task_name.as_deref(), key, new_val)? {
                            files_patched.push(file.clone());
                        }
                    }
                }
                if key == "period_ms" {
                    if let Some(app_cfg) = find_app_config_path(project_root, &analyzed_files) {
                        let abs = project_root.join(&app_cfg);
                        if abs.is_file() {
                            let macros = scan_app_config_timing_macros(&abs).unwrap_or_default();
                            let old_ms = json_as_u64(&old_val);
                            if let Some(macro_name) =
                                select_timing_macro_to_patch(&macros, old_ms, node_id)
                            {
                                if let Some(new_ms) = json_as_u64(new_val) {
                                    if patch_app_config_numeric_define(&abs, &macro_name, new_ms)? {
                                        files_patched.push(app_cfg);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            "password" if pir.nodes[node_idx].node_type == "mqtt_client" => {
                patch_mqtt_credentials(
                    project_root,
                    &analyzed_files,
                    &target_files,
                    key,
                    new_val,
                    &mut files_patched,
                )?;
            }
            "ssid" | "password" => {
                patch_wifi_credentials(
                    project_root,
                    &analyzed_files,
                    &target_files,
                    key,
                    new_val,
                    &mut files_patched,
                )?;
            }
            "broker_url" | "topic" | "username" => {
                patch_mqtt_credentials(
                    project_root,
                    &analyzed_files,
                    &target_files,
                    key,
                    new_val,
                    &mut files_patched,
                )?;
            }
            _ => {}
        }

        if let Some(obj) = pir.nodes[node_idx].properties.as_object_mut() {
            obj.insert(key.clone(), new_val.clone());
        }
    }

    files_patched.sort();
    files_patched.dedup();

    let change_id = format!("chg_{}", now_ms());
    let entry = PirChangeEntry {
        id: change_id,
        revision: pir.revision.clone(),
        timestamp_ms: now_ms(),
        node_id: node_id.to_string(),
        property: property_updates.keys().next().cloned().unwrap_or_default(),
        old_value: JsonValue::Object(old_values),
        new_value: JsonValue::Object(new_values),
        files_patched: files_patched.clone(),
        reversible: true,
    };
    pir.change_log.push(entry.clone());
    pir.approval.status = super::schema::PirApprovalStatus::Stale;

    Ok(PatchResult {
        files_patched,
        change_entry: entry,
    })
}

fn property_get(props: &JsonValue, key: &str) -> JsonValue {
    props
        .as_object()
        .and_then(|o| o.get(key))
        .cloned()
        .unwrap_or(JsonValue::Null)
}

fn resolve_target_files(
    project_root: &Path,
    node: &super::schema::PirNode,
    analyzed_files: &[String],
) -> Vec<String> {
    let mut files = if !node.ownership.primary_files.is_empty() {
        node.ownership.primary_files.clone()
    } else {
        analyzed_files
            .iter()
            .filter(|f| {
                let p = project_root.join(f);
                p.is_file() && (f.contains("main/") || f.ends_with(".c") || f.ends_with(".cpp"))
            })
            .cloned()
            .collect()
    };
    if let Some(app_cfg) = find_app_config_path(project_root, analyzed_files) {
        if !files.iter().any(|f| f == &app_cfg) {
            files.insert(0, app_cfg);
        }
    }
    files
}

fn find_app_config_path(project_root: &Path, analyzed_files: &[String]) -> Option<String> {
    analyzed_files
        .iter()
        .find(|f| {
            let norm = f.replace('\\', "/");
            norm.ends_with("app_config.h")
        })
        .cloned()
        .or_else(|| {
            let candidate = project_root.join("main/app_config.h");
            if candidate.is_file() {
                Some("main/app_config.h".to_string())
            } else {
                None
            }
        })
}

fn patch_wifi_credentials(
    project_root: &Path,
    analyzed_files: &[String],
    target_files: &[String],
    key: &str,
    new_val: &JsonValue,
    files_patched: &mut Vec<String>,
) -> Result<(), String> {
    let app_cfg = find_app_config_path(project_root, analyzed_files);
    if let Some(ref app_cfg) = app_cfg {
        let abs = project_root.join(app_cfg);
        if patch_app_config_wifi(&abs, key, new_val)? {
            files_patched.push(app_cfg.clone());
            return Ok(());
        }
        if insert_app_config_wifi_define(&abs, key, new_val)? {
            files_patched.push(app_cfg.clone());
            return Ok(());
        }
    }
    for file in target_files {
        if app_cfg.as_ref().is_some_and(|cfg| cfg == file) {
            continue;
        }
        let abs = project_root.join(file);
        if abs.is_file() {
            if patch_app_config_wifi(&abs, key, new_val)? {
                files_patched.push(file.clone());
            } else if patch_wifi_credential_file(&abs, key, new_val)? {
                files_patched.push(file.clone());
            }
        }
    }
    if app_cfg.is_none() && patch_wifi_in_sdkconfig_defaults(project_root, key, new_val)? {
        files_patched.push("sdkconfig.defaults".to_string());
    }
    Ok(())
}

fn patch_mqtt_credentials(
    project_root: &Path,
    analyzed_files: &[String],
    target_files: &[String],
    key: &str,
    new_val: &JsonValue,
    files_patched: &mut Vec<String>,
) -> Result<(), String> {
    let define = match key {
        "broker_url" => Some("APP_MQTT_URI"),
        "topic" => Some("APP_MQTT_BASE_TOPIC"),
        "username" => Some("APP_MQTT_USERNAME"),
        "password" => Some("APP_MQTT_PASSWORD"),
        _ => None,
    };
    if let Some(macro_name) = define {
        if let Some(app_cfg) = find_app_config_path(project_root, analyzed_files) {
            if patch_app_config_define(&project_root.join(&app_cfg), macro_name, new_val)? {
                files_patched.push(app_cfg);
            }
        }
    }
    for file in target_files {
        let abs = project_root.join(file);
        if abs.is_file() {
            if patch_mqtt_in_source(&abs, key, new_val)? {
                files_patched.push(file.clone());
            }
        }
    }
    Ok(())
}

fn patch_app_config_wifi(path: &Path, key: &str, new_val: &JsonValue) -> Result<bool, String> {
    let macro_names: &[&str] = match key {
        "ssid" => &[
            "APP_WIFI_SSID",
            "APP_WIFI_SSID_STR",
            "WIFI_SSID",
            "WIFI_AP_SSID",
        ],
        "password" => &[
            "APP_WIFI_PASSWORD",
            "APP_WIFI_PASS",
            "APP_WIFI_PASSWD",
            "WIFI_PASSWORD",
            "WIFI_AP_PASSWORD",
        ],
        _ => return Ok(false),
    };
    for macro_name in macro_names {
        if patch_app_config_define(path, macro_name, new_val)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn insert_app_config_wifi_define(
    path: &Path,
    key: &str,
    new_val: &JsonValue,
) -> Result<bool, String> {
    let Some(val_str) = new_val.as_str() else {
        return Ok(false);
    };
    let macro_name = match key {
        "ssid" => "APP_WIFI_SSID",
        "password" => "APP_WIFI_PASSWORD",
        _ => return Ok(false),
    };
    insert_app_config_define(path, macro_name, val_str)
}

fn insert_app_config_define(path: &Path, macro_name: &str, val_str: &str) -> Result<bool, String> {
    let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    if content
        .lines()
        .any(|line| line_has_define(line, macro_name))
    {
        return Ok(false);
    }
    let mut out = content;
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out.push_str(&format!(r#"#define {macro_name} "{val_str}""#));
    out.push('\n');
    std::fs::write(path, out).map_err(|e| e.to_string())?;
    Ok(true)
}

fn line_has_define(line: &str, macro_name: &str) -> bool {
    let trimmed = line.trim().trim_end_matches('\r');
    if !trimmed.starts_with("#define") {
        return false;
    }
    trimmed
        .split_whitespace()
        .nth(1)
        .is_some_and(|name| name.trim_end_matches('\r') == macro_name)
}

fn patch_app_config_define(
    path: &Path,
    macro_name: &str,
    new_val: &JsonValue,
) -> Result<bool, String> {
    let Some(val_str) = new_val.as_str() else {
        return Ok(false);
    };
    let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let pattern = format!(
        r"(?m)^(\s*#define\s+{}\b)([^\n]*)",
        regex::escape(macro_name)
    );
    let re = Regex::new(&pattern).map_err(|e| e.to_string())?;
    if !re.is_match(&content) {
        return Ok(false);
    }
    let updated = re
        .replace_all(&content, format!(r#"$1 "{val_str}""#))
        .to_string();
    if updated == content {
        return Ok(false);
    }
    let with_newline = if content.ends_with('\n') && !updated.ends_with('\n') {
        format!("{}\n", updated)
    } else {
        updated
    };
    std::fs::write(path, with_newline).map_err(|e| e.to_string())?;
    Ok(true)
}

fn gpio_define_candidates(node_id: &str) -> Vec<String> {
    let upper = node_id.to_uppercase().replace('-', "_");
    let base = upper.trim_end_matches("_GPIO").trim_end_matches("_PIN");
    let mut out = vec![
        format!("APP_{}", upper),
        format!("APP_{}_GPIO", base),
        format!("APP_{}_PIN", base),
        upper.clone(),
    ];
    out.sort();
    out.dedup();
    out
}

fn patch_app_config_gpio(
    path: &Path,
    node_id: &str,
    old_pin: Option<u8>,
    new_pin: u8,
) -> Result<bool, String> {
    let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let candidates = gpio_define_candidates(node_id);
    for macro_name in &candidates {
        let pattern = format!(
            r"(?m)^(\s*#define\s+{}\s+)(\d+)(\s*(?://.*)?)$",
            regex::escape(macro_name)
        );
        let re = Regex::new(&pattern).unwrap();
        if re.is_match(&content) {
            let updated = re
                .replace_all(&content, format!("${{1}}{new_pin}${{3}}"))
                .to_string();
            if updated != content {
                std::fs::write(path, updated).map_err(|e| e.to_string())?;
                return Ok(true);
            }
        }
    }
    if let Some(old) = old_pin {
        let pattern = format!(
            r"(?m)^(\s*#define\s+(APP_[A-Z0-9_]*GPIO[A-Z0-9_]*)\s+){}(\s*(?://.*)?)$",
            old
        );
        let re = Regex::new(&pattern).unwrap();
        if re.is_match(&content) {
            let updated = re
                .replace_all(&content, format!("${{1}}{new_pin}${{3}}"))
                .to_string();
            if updated != content {
                std::fs::write(path, updated).map_err(|e| e.to_string())?;
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn patch_gpio_pin_in_file(path: &Path, old_pin: Option<u8>, new_pin: u8) -> Result<bool, String> {
    let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut updated = content.clone();

    if let Some(old) = old_pin {
        let re_set = Regex::new(&format!(r"gpio_set_level\s*\(\s*{}\s*,", old)).unwrap();
        updated = re_set
            .replace_all(&updated, format!("gpio_set_level( {},", new_pin))
            .to_string();

        let re_num = Regex::new(&format!(r"GPIO_NUM_{}\b", old)).unwrap();
        updated = re_num
            .replace_all(&updated, format!("GPIO_NUM_{}", new_pin))
            .to_string();

        let re_shift = Regex::new(&format!(r"1\s*<<\s*{}\b", old)).unwrap();
        updated = re_shift
            .replace_all(&updated, format!("1 << {}", new_pin))
            .to_string();

        let re_pin_field = Regex::new(&format!(r"\bpin\s*=\s*{}\b", old)).unwrap();
        updated = re_pin_field
            .replace_all(&updated, format!("pin = {}", new_pin))
            .to_string();
    }

    let re_prop = Regex::new(r#""pin"\s*:\s*\d+"#).unwrap();
    updated = re_prop
        .replace_all(&updated, format!(r#""pin": {}"#, new_pin))
        .to_string();

    if updated != content {
        std::fs::write(path, updated).map_err(|e| e.to_string())?;
        return Ok(true);
    }
    Ok(false)
}

fn patch_rtos_property_in_file(
    path: &Path,
    task_name: Option<&str>,
    key: &str,
    new_val: &JsonValue,
) -> Result<bool, String> {
    let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let task = task_name.unwrap_or("");
    let mut lines_out: Vec<String> = Vec::new();
    let mut changed = false;

    for line in content.lines() {
        let mut new_line = line.to_string();
        if line.contains("xTaskCreate") && (task.is_empty() || line.contains(task)) {
            if key == "priority" {
                if let Some(p) = new_val.as_u64() {
                    let re_pri = Regex::new(r",\s*\d+\s*,").unwrap();
                    new_line = re_pri.replace(line, format!(", {},", p)).to_string();
                }
            } else if key == "stack_size" {
                if let Some(s) = new_val.as_u64() {
                    let re_stack =
                        Regex::new(r#"(xTaskCreate\s*\(\s*[^,]+,\s*"[^"]+",\s*)(\d+)"#).unwrap();
                    new_line = re_stack
                        .replace(line, |caps: &regex::Captures| format!("{}{}", &caps[1], s))
                        .to_string();
                }
            } else if key == "task_name" {
                if let Some(name) = new_val.as_str() {
                    let re_name = Regex::new(r#""([^"]+)""#).unwrap();
                    new_line = re_name.replace(line, format!("\"{}\"", name)).to_string();
                }
            }
        }
        if key == "period_ms" {
            if let Some(ms) = json_as_u64(new_val) {
                if line.contains("vTaskDelay") {
                    let re = Regex::new(r"vTaskDelay\s*\([^)]+\)").unwrap();
                    new_line = re
                        .replace(line, format!("vTaskDelay(pdMS_TO_TICKS({}))", ms))
                        .to_string();
                }
                let re_ticks = Regex::new(r"pdMS_TO_TICKS\s*\(\s*\d+\s*\)").unwrap();
                let replaced = re_ticks
                    .replace(&new_line, format!("pdMS_TO_TICKS({})", ms))
                    .to_string();
                if replaced != new_line {
                    new_line = replaced;
                }
            }
        }
        if new_line != line {
            changed = true;
        }
        lines_out.push(new_line);
    }

    if changed {
        let updated = lines_out.join("\n");
        let with_newline = if content.ends_with('\n') {
            format!("{}\n", updated)
        } else {
            updated
        };
        std::fs::write(path, with_newline).map_err(|e| e.to_string())?;
        return Ok(true);
    }
    Ok(false)
}

fn patch_wifi_in_sdkconfig_defaults(
    project_root: &Path,
    key: &str,
    new_val: &JsonValue,
) -> Result<bool, String> {
    let path = project_root.join("sdkconfig.defaults");
    let Some(val_str) = new_val.as_str() else {
        return Ok(false);
    };
    let symbol = match key {
        "ssid" => "CONFIG_ESP_WIFI_SSID",
        "password" => "CONFIG_ESP_WIFI_PASSWORD",
        _ => return Ok(false),
    };
    let line = format!(r#"{}="{}""#, symbol, val_str);
    upsert_config_line(&path, symbol, &line)
}

fn patch_wifi_credential_file(path: &Path, key: &str, new_val: &JsonValue) -> Result<bool, String> {
    let Some(val_str) = new_val.as_str() else {
        return Ok(false);
    };
    let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let pattern = match key {
        "ssid" => r#"\.sta\.ssid\s*=\s*"[^"]*""#,
        "password" => r#"\.sta\.password\s*=\s*"[^"]*""#,
        _ => return Ok(false),
    };
    let re = Regex::new(pattern).unwrap();
    let replacement = match key {
        "ssid" => format!(r#".sta.ssid = "{}""#, val_str),
        "password" => format!(r#".sta.password = "{}""#, val_str),
        _ => return Ok(false),
    };
    let updated = re.replace(&content, replacement.as_str()).to_string();
    if updated != content {
        std::fs::write(path, updated).map_err(|e| e.to_string())?;
        return Ok(true);
    }
    Ok(false)
}

fn patch_mqtt_in_source(path: &Path, key: &str, new_val: &JsonValue) -> Result<bool, String> {
    let Some(val_str) = new_val.as_str() else {
        return Ok(false);
    };
    let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let (pattern, replacement) = match key {
        "broker_url" => (
            r#"(?:uri|broker_url|broker)\s*=\s*"[^"]*""#,
            format!(r#"uri = "{}""#, val_str),
        ),
        "topic" => (
            r#"\.topic\s*=\s*"[^"]*""#,
            format!(r#".topic = "{}""#, val_str),
        ),
        _ => return Ok(false),
    };
    let re = Regex::new(pattern).unwrap();
    let updated = re.replace(&content, replacement.as_str()).to_string();
    if updated != content {
        std::fs::write(path, updated).map_err(|e| e.to_string())?;
        return Ok(true);
    }
    Ok(false)
}

fn upsert_config_line(path: &Path, symbol: &str, new_line: &str) -> Result<bool, String> {
    let mut lines: Vec<String> = if path.is_file() {
        std::fs::read_to_string(path)
            .map_err(|e| e.to_string())?
            .lines()
            .map(|s| s.to_string())
            .collect()
    } else {
        Vec::new()
    };
    let mut found = false;
    for line in &mut lines {
        if line.starts_with(symbol) {
            *line = new_line.to_string();
            found = true;
        }
    }
    if !found {
        lines.push(new_line.to_string());
    }
    std::fs::write(path, lines.join("\n") + "\n").map_err(|e| e.to_string())?;
    Ok(true)
}

fn json_as_u64(v: &JsonValue) -> Option<u64> {
    v.as_u64()
        .or_else(|| v.as_f64().map(|f| f.round() as u64))
        .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
}

fn patch_app_config_numeric_define(
    path: &Path,
    macro_name: &str,
    new_ms: u64,
) -> Result<bool, String> {
    let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let pattern = format!(
        r"(?m)^(\s*#define\s+{}\s+)(\d+)(u|U)?(\s*(?://.*)?)?$",
        regex::escape(macro_name)
    );
    let re = Regex::new(&pattern).map_err(|e| e.to_string())?;
    if !re.is_match(&content) {
        return Ok(false);
    }
    let updated = re
        .replace_all(&content, format!("${{1}}{new_ms}${{3}}${{4}}"))
        .to_string();
    if updated == content {
        return Ok(false);
    }
    let with_newline = if content.ends_with('\n') && !updated.ends_with('\n') {
        format!("{}\n", updated)
    } else {
        updated
    };
    std::fs::write(path, with_newline).map_err(|e| e.to_string())?;
    Ok(true)
}

fn select_timing_macro_to_patch(
    macros: &[JsonValue],
    old_ms: Option<u64>,
    node_id: &str,
) -> Option<String> {
    let timing: Vec<(String, u64)> = macros
        .iter()
        .filter_map(|m| {
            let name = m.get("macro")?.as_str()?.to_string();
            let val = m.get("value")?.as_u64()?;
            Some((name, val))
        })
        .collect();
    if timing.is_empty() {
        return None;
    }
    if let Some(old) = old_ms {
        let matches: Vec<&(String, u64)> = timing.iter().filter(|(_, v)| *v == old).collect();
        if matches.len() == 1 {
            return Some(matches[0].0.clone());
        }
        if matches.len() > 1 {
            return pick_preferred_timing_macro(&matches, node_id);
        }
    }
    if timing.len() == 1 {
        return Some(timing[0].0.clone());
    }
    let refs: Vec<&(String, u64)> = timing.iter().collect();
    pick_preferred_timing_macro(&refs, node_id)
}

fn pick_preferred_timing_macro(candidates: &[&(String, u64)], node_id: &str) -> Option<String> {
    let node_upper = node_id.to_uppercase().replace('-', "_");
    let mut scored: Vec<(String, i32)> = candidates
        .iter()
        .map(|(name, _)| {
            let upper = name.to_uppercase();
            let mut score = 0i32;
            if upper.contains("BLINK") {
                score += 10;
            }
            if upper.ends_with("_PERIOD_MS")
                || upper.ends_with("_INTERVAL_MS")
                || upper.ends_with("_DELAY_MS")
            {
                score += 5;
            }
            if node_upper.len() > 3 && upper.contains(&node_upper) {
                score += 3;
            }
            (name.clone(), score)
        })
        .collect();
    scored.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    scored.first().map(|(n, _)| n.clone())
}

/// Scan `app_config.h` for timing-related `APP_*` numeric defines.
fn scan_app_config_timing_macros(path: &Path) -> Result<Vec<serde_json::Value>, String> {
    let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let re = Regex::new(r"(?m)^\s*#define\s+(APP_[A-Z0-9_]+)\s+(\d+)(?:u|U)?(?:\s*(?://.*)?)?$")
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for cap in re.captures_iter(&content) {
        let name = cap.get(1).map(|m| m.as_str()).unwrap_or("");
        let upper = name.to_uppercase();
        if upper.ends_with("_INTERVAL_MS")
            || upper.ends_with("_PERIOD_MS")
            || upper.ends_with("_DELAY_MS")
            || upper.contains("BLINK")
        {
            out.push(json!({
                "macro": name,
                "value": cap.get(2).and_then(|m| m.as_str().parse::<u64>().ok()),
            }));
        }
    }
    Ok(out)
}

#[derive(Debug, Clone, Deserialize)]
pub struct StructuralPatchRequest {
    #[serde(default)]
    pub add_nodes: Vec<PirNode>,
    #[serde(default)]
    pub remove_node_ids: Vec<String>,
    #[serde(default)]
    pub add_edges: Vec<PirEdge>,
    #[serde(default)]
    pub remove_edge_ids: Vec<String>,
}

pub fn apply_structural_patch(
    _project_root: &Path,
    pir: &mut PirDocument,
    body: &StructuralPatchRequest,
    expected_revision: Option<&str>,
) -> Result<(), String> {
    if let Some(expected) = expected_revision {
        if expected != pir.revision {
            return Err(format!(
                "revision mismatch: expected {}, current {}",
                expected, pir.revision
            ));
        }
    }

    pir.nodes.retain(|n| !body.remove_node_ids.contains(&n.id));
    for id in &body.remove_node_ids {
        pir.edges
            .retain(|e| e.source_node_id != *id && e.target_node_id != *id);
    }

    for mut node in body.add_nodes.clone() {
        node.authority = NodeAuthority::User;
        node.sync.state = PirSyncState::Manual;
        if let Some(existing) = pir.nodes.iter_mut().find(|n| n.id == node.id) {
            *existing = node;
        } else {
            pir.nodes.push(node);
        }
    }

    pir.edges.retain(|e| !body.remove_edge_ids.contains(&e.id));
    for edge in &body.add_edges {
        if !pir.edges.iter().any(|e| e.id == edge.id) {
            pir.edges.push(edge.clone());
        }
    }

    pir.approval.status = PirApprovalStatus::Stale;
    Ok(())
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
