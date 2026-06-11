//! Persist PIR documents to `.craftif/pir.json` on disk.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::schema::{PirAnalyzeResult, PirDocument};
use crate::firmware_topology::FirmwareGraph;

pub const PIR_DIR: &str = ".craftif";
pub const PIR_FILE: &str = "pir.json";
pub const PIR_HISTORY_DIR: &str = "pir.history";
pub const PIR_VIEWS_DIR: &str = "pir.views";
pub const PIR_VIEWS_SCHEMA_VERSION: &str = "1.0.0";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PirGraphView {
    Wiring,
    Hld,
    Lld,
    Sequence,
}

impl PirGraphView {
    pub fn as_str(self) -> &'static str {
        match self {
            PirGraphView::Wiring => "wiring",
            PirGraphView::Hld => "hld",
            PirGraphView::Lld => "lld",
            PirGraphView::Sequence => "sequence",
        }
    }

    pub fn file_name(self) -> &'static str {
        match self {
            PirGraphView::Wiring => "wiring.json",
            PirGraphView::Hld => "hld.json",
            PirGraphView::Lld => "lld.json",
            PirGraphView::Sequence => "sequence.json",
        }
    }

    pub fn from_query(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "wiring" | "topology" => Some(PirGraphView::Wiring),
            "hld" => Some(PirGraphView::Hld),
            "lld" | "ldd" => Some(PirGraphView::Lld),
            "sequence" => Some(PirGraphView::Sequence),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PirGraphViewDocument {
    pub schema_version: String,
    pub view: String,
    pub revision: String,
    #[serde(default)]
    pub graph_version: u32,
    pub generated_at_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph: Option<FirmwareGraph>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mermaid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_error: Option<String>,
}

pub fn pir_file_path(project_root: &Path) -> PathBuf {
    project_root.join(PIR_DIR).join(PIR_FILE)
}

pub fn pir_history_dir(project_root: &Path) -> PathBuf {
    project_root.join(PIR_DIR).join(PIR_HISTORY_DIR)
}

pub fn pir_views_dir(project_root: &Path) -> PathBuf {
    project_root.join(PIR_DIR).join(PIR_VIEWS_DIR)
}

pub fn pir_view_file_path(project_root: &Path, view: PirGraphView) -> PathBuf {
    pir_views_dir(project_root).join(view.file_name())
}

pub fn load_pir(project_root: &Path) -> Option<PirDocument> {
    let path = pir_file_path(project_root);
    if !path.is_file() {
        return None;
    }
    let text = fs::read_to_string(&path).ok()?;
    serde_json::from_str(&text).ok()
}

pub fn save_pir(project_root: &Path, pir: &PirDocument) -> Result<(), String> {
    let dir = project_root.join(PIR_DIR);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = pir_file_path(project_root);
    let json = serde_json::to_string_pretty(pir).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())
}

pub fn save_history_snapshot(project_root: &Path, pir: &PirDocument) -> Result<(), String> {
    let dir = pir_history_dir(project_root);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let name = format!("pir_{}.json", pir.revision);
    let path = dir.join(name);
    let json = serde_json::to_string_pretty(pir).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub fn build_graph_view_document(
    result: &PirAnalyzeResult,
    view: PirGraphView,
) -> PirGraphViewDocument {
    let mut diagrams = result.pir.diagrams.clone().unwrap_or_default();
    if diagrams.hld_graph.is_none()
        || diagrams.lld_graph.is_none()
        || diagrams.sequence_graph.is_none()
        || diagrams.hld.is_none()
        || diagrams.lld.is_none()
        || diagrams.sequence.is_none()
    {
        let regenerated = super::diagrams::generate_all(&result.pir);
        if diagrams.hld.is_none() {
            diagrams.hld = regenerated.hld;
        }
        if diagrams.lld.is_none() {
            diagrams.lld = regenerated.lld;
        }
        if diagrams.sequence.is_none() {
            diagrams.sequence = regenerated.sequence;
        }
        if diagrams.hld_graph.is_none() {
            diagrams.hld_graph = regenerated.hld_graph;
        }
        if diagrams.lld_graph.is_none() {
            diagrams.lld_graph = regenerated.lld_graph;
        }
        if diagrams.sequence_graph.is_none() {
            diagrams.sequence_graph = regenerated.sequence_graph;
        }
    }

    match view {
        PirGraphView::Wiring => PirGraphViewDocument {
            schema_version: PIR_VIEWS_SCHEMA_VERSION.to_string(),
            view: view.as_str().to_string(),
            revision: result.pir.revision.clone(),
            graph_version: result.pir.graph_version,
            generated_at_ms: now_ms(),
            graph: Some(result.graph.clone()),
            mermaid: None,
            title: Some("Wiring Topology".to_string()),
            generation_error: None,
        },
        PirGraphView::Hld => PirGraphViewDocument {
            schema_version: PIR_VIEWS_SCHEMA_VERSION.to_string(),
            view: view.as_str().to_string(),
            revision: result.pir.revision.clone(),
            graph_version: result.pir.graph_version,
            generated_at_ms: now_ms(),
            graph: diagrams.hld_graph.clone(),
            mermaid: diagrams.hld.as_ref().and_then(|d| d.mermaid.clone()),
            title: diagrams
                .hld
                .as_ref()
                .and_then(|d| d.title.clone())
                .or_else(|| Some("High-Level Design".to_string())),
            generation_error: None,
        },
        PirGraphView::Lld => PirGraphViewDocument {
            schema_version: PIR_VIEWS_SCHEMA_VERSION.to_string(),
            view: view.as_str().to_string(),
            revision: result.pir.revision.clone(),
            graph_version: result.pir.graph_version,
            generated_at_ms: now_ms(),
            graph: diagrams.lld_graph.clone(),
            mermaid: diagrams.lld.as_ref().and_then(|d| d.mermaid.clone()),
            title: diagrams
                .lld
                .as_ref()
                .and_then(|d| d.title.clone())
                .or_else(|| Some("Low-Level Design".to_string())),
            generation_error: None,
        },
        PirGraphView::Sequence => PirGraphViewDocument {
            schema_version: PIR_VIEWS_SCHEMA_VERSION.to_string(),
            view: view.as_str().to_string(),
            revision: result.pir.revision.clone(),
            graph_version: result.pir.graph_version,
            generated_at_ms: now_ms(),
            graph: diagrams.sequence_graph.clone(),
            mermaid: diagrams.sequence.as_ref().and_then(|d| d.mermaid.clone()),
            title: diagrams
                .sequence
                .as_ref()
                .and_then(|d| d.title.clone())
                .or_else(|| Some("Sequence Diagram".to_string())),
            generation_error: diagrams
                .sequence
                .as_ref()
                .and_then(|d| d.generation_error.clone()),
        },
    }
}

pub fn save_graph_views(project_root: &Path, result: &PirAnalyzeResult) -> Result<(), String> {
    let dir = pir_views_dir(project_root);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    for view in [
        PirGraphView::Wiring,
        PirGraphView::Hld,
        PirGraphView::Lld,
        PirGraphView::Sequence,
    ] {
        let doc = build_graph_view_document(result, view);
        let path = pir_view_file_path(project_root, view);
        let json = serde_json::to_string_pretty(&doc).map_err(|e| e.to_string())?;
        fs::write(path, json).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn load_graph_view(project_root: &Path, view: PirGraphView) -> Option<PirGraphViewDocument> {
    let path = pir_view_file_path(project_root, view);
    if !path.is_file() {
        return None;
    }
    let text = fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

pub fn compute_manifest_revision(hashes: &std::collections::HashMap<String, String>) -> String {
    let mut keys: Vec<_> = hashes.keys().collect();
    keys.sort();
    let mut payload = String::new();
    for k in keys {
        payload.push_str(k);
        payload.push(':');
        payload.push_str(hashes.get(k).map(|s| s.as_str()).unwrap_or(""));
        payload.push('|');
    }
    format!("{:x}", md5::compute(payload.as_bytes()))
}
