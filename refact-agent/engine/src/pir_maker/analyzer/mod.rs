//! Incremental static analysis of ESP-IDF projects.

pub mod ast_extract;
pub mod chat_gap;
pub mod manifest;
pub mod static_extract;

use std::path::{Path, PathBuf};

use super::schema::AnalysisFacts;

pub const ANALYZER_VERSION: &str = "pir_maker-static-0.2.0";

/// Analyze project; reuses cached hashes in `previous` for unchanged files.
pub fn analyze_project(
    project_root: &Path,
    previous: Option<&AnalysisFacts>,
    chat_context: Option<&str>,
) -> Result<AnalysisFacts, String> {
    if !project_root.is_dir() {
        return Err(format!(
            "project path is not a directory: {}",
            project_root.display()
        ));
    }

    let project_name = project_root
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("firmware")
        .to_string();

    let manifest = manifest::build_manifest(project_root)?;
    let mut facts = AnalysisFacts {
        project_name,
        target_chip: static_extract::detect_target_chip(project_root),
        board_id: None,
        has_app_main: false,
        app_main_file: None,
        gpio_facts: Vec::new(),
        task_facts: Vec::new(),
        network_facts: Vec::new(),
        partitions: Vec::new(),
        components: static_extract::parse_cmake_components(project_root),
        file_hashes: manifest.hashes.clone(),
        analyzed_files: manifest.rel_paths.clone(),
        unresolved: Vec::new(),
    };

    for rel in &manifest.rel_paths {
        let prev_hash = previous.and_then(|p| p.file_hashes.get(rel));
        let cur_hash = manifest.hashes.get(rel);
        if let (Some(ph), Some(ch)) = (prev_hash, cur_hash) {
            if ph == ch {
                if let Some(p) = previous {
                    merge_cached_file_facts(&mut facts, p, rel);
                }
                continue;
            }
        }
        let content = manifest
            .contents
            .get(rel)
            .cloned()
            .or_else(|| std::fs::read_to_string(project_root.join(rel)).ok());
        let Some(content) = content else {
            continue;
        };

        static_extract::extract_from_file(rel, &content, &mut facts);

        let mut ast_facts = AnalysisFacts {
            project_name: facts.project_name.clone(),
            ..Default::default()
        };
        ast_extract::extract_from_file(rel, &content, &mut ast_facts);
        ast_extract::merge_ast_into(&mut facts, ast_facts);
    }

    let app_config_rel = manifest
        .rel_paths
        .iter()
        .find(|r| static_extract::is_app_config_path(r))
        .cloned();

    // Bridge main-chat intent into unresolved hints (strict evidence policy).
    chat_gap::fill_gaps_from_chat(chat_context, &mut facts, app_config_rel.as_deref());

    if facts.has_app_main && facts.gpio_facts.is_empty() && facts.task_facts.is_empty() {
        facts.unresolved.push(serde_json::json!({
            "kind": "sparse_extraction",
            "message": "app_main found but no GPIO/tasks extracted; AI refine may help",
            "file": facts.app_main_file.clone().unwrap_or_default()
        }));
    }

    Ok(facts)
}

fn merge_cached_file_facts(facts: &mut AnalysisFacts, previous: &AnalysisFacts, rel: &str) {
    for g in &previous.gpio_facts {
        if g.file == rel {
            facts.gpio_facts.push(g.clone());
        }
    }
    for t in &previous.task_facts {
        if t.file == rel {
            facts.task_facts.push(t.clone());
        }
    }
    for n in &previous.network_facts {
        if n.file == rel {
            facts.network_facts.push(n.clone());
        }
    }
    if previous.app_main_file.as_deref() == Some(rel) {
        facts.has_app_main = true;
        facts.app_main_file = previous.app_main_file.clone();
    }
}
