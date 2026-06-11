//! File discovery and prioritized context assembly for the PIR sub-agent.

use std::collections::HashMap;
use std::path::Path;

use serde_json::Value as JsonValue;

use super::super::analyzer::manifest;
use super::super::analyzer::static_extract;

const MAX_CHARS_PER_FILE: usize = 12_000;
const MAX_TOTAL_CHARS: usize = 80_000;
const MAX_REFINE_CHARS: usize = 15_000;
const MAX_SNIPPET_LINES: usize = 80;

#[derive(Debug, Clone)]
pub struct ContextFile {
    pub rel_path: String,
    pub priority: u8,
    pub content: String,
    pub hash: String,
}

/// Files whose hash changed or are new compared to a previous revision.
pub fn diff_changed_files(
    previous_hashes: &HashMap<String, String>,
    current_hashes: &HashMap<String, String>,
) -> HashMap<String, String> {
    let mut changed = HashMap::new();
    for (rel, hash) in current_hashes {
        match previous_hashes.get(rel) {
            Some(prev) if prev == hash => {}
            _ => {
                changed.insert(rel.clone(), hash.clone());
            }
        }
    }
    changed
}

pub fn build_project_context(
    project_root: &Path,
    changed_only: Option<&HashMap<String, String>>,
) -> Result<(Vec<ContextFile>, HashMap<String, String>), String> {
    let manifest = manifest::build_manifest(project_root)?;
    let mut files: Vec<(String, u8)> = Vec::new();

    for rel in &manifest.rel_paths {
        if let Some(changed) = changed_only {
            if !changed.contains_key(rel) {
                continue;
            }
        }
        files.push((rel.clone(), manifest::priority_for_path(rel)));
    }

    files.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    // Always include app_config.h first — primary topology manifest from main agent.
    if let Some(app_cfg) = manifest
        .rel_paths
        .iter()
        .find(|r| static_extract::is_app_config_path(r))
    {
        if !files.iter().any(|(r, _)| r == app_cfg) {
            files.insert(0, (app_cfg.clone(), manifest::priority_for_path(app_cfg)));
        }
    }

    let mut out = Vec::new();
    let mut total = 0usize;
    for (rel, priority) in files {
        if total >= MAX_TOTAL_CHARS {
            break;
        }
        let content = manifest
            .contents
            .get(&rel)
            .cloned()
            .or_else(|| std::fs::read_to_string(project_root.join(&rel)).ok())
            .map(|s| truncate_content(&s, MAX_CHARS_PER_FILE))
            .unwrap_or_default();
        if content.is_empty() {
            continue;
        }
        total += content.len();
        let hash = manifest.hashes.get(&rel).cloned().unwrap_or_default();
        out.push(ContextFile {
            rel_path: rel,
            priority,
            content,
            hash,
        });
    }

    Ok((out, manifest.hashes))
}

/// Build targeted snippets for unresolved items (hybrid AI refine pass).
pub fn build_unresolved_snippets(
    project_root: &Path,
    unresolved: &[JsonValue],
    manifest_contents: &HashMap<String, String>,
) -> String {
    let mut target_files: Vec<(String, Option<u32>)> = Vec::new();
    for item in unresolved {
        if let Some(file) = item.get("file").and_then(|v| v.as_str()) {
            let line = item.get("line").and_then(|v| v.as_u64()).map(|n| n as u32);
            if !target_files.iter().any(|(f, _)| f == file) {
                target_files.push((file.to_string(), line));
            }
        }
    }

    if target_files.is_empty() {
        if let Some(app_main) = unresolved
            .iter()
            .find_map(|u| u.get("file").and_then(|v| v.as_str()))
        {
            target_files.push((app_main.to_string(), None));
        } else if let Some(candidate) = manifest::first_main_source_file(project_root) {
            target_files.push((candidate, None));
        }
    }

    let mut blocks = Vec::new();
    let mut total = 0usize;
    for (rel, center_line) in target_files {
        if total >= MAX_REFINE_CHARS {
            break;
        }
        let content = manifest_contents
            .get(&rel)
            .cloned()
            .or_else(|| std::fs::read_to_string(project_root.join(&rel)).ok());
        let Some(content) = content else {
            continue;
        };
        let snippet = if let Some(line) = center_line {
            extract_lines_around(&content, line, MAX_SNIPPET_LINES / 2)
        } else {
            truncate_content(&content, MAX_CHARS_PER_FILE)
        };
        total += snippet.len();
        blocks.push(format!("=== FILE: {} ===\n{}", rel, snippet));
    }
    blocks.join("\n\n")
}

fn extract_lines_around(content: &str, center_line: u32, radius: usize) -> String {
    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        return String::new();
    }
    let idx = center_line.saturating_sub(1) as usize;
    let start = idx.saturating_sub(radius);
    let end = (idx + radius + 1).min(lines.len());
    lines[start..end].join("\n")
}

fn truncate_content(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    format!("{}\n... [truncated {} chars] ...", &s[..max], s.len() - max)
}

pub fn format_context_block(files: &[ContextFile]) -> String {
    let mut blocks = Vec::new();
    for f in files {
        blocks.push(format!(
            "=== FILE: {} (priority {}) ===\n{}",
            f.rel_path, f.priority, f.content
        ));
    }
    blocks.join("\n\n")
}

pub fn compact_pir_summary(pir: &super::super::schema::PirDocument) -> String {
    let mut lines = vec![format!(
        "Revision {} — {} nodes, {} edges",
        pir.revision,
        pir.nodes.len(),
        pir.edges.len()
    )];
    for node in pir.nodes.iter().take(15) {
        lines.push(format!(
            "- {} type={} layer={:?} files={}",
            node.id,
            node.node_type,
            node.layer,
            node.ownership.primary_files.join(",")
        ));
    }
    lines.join("\n")
}

pub fn project_name_from_path(project_root: &Path) -> String {
    project_root
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("esp32_project")
        .to_string()
}
