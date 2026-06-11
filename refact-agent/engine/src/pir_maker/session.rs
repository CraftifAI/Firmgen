//! Per-chat PIR_maker session state.

use std::collections::HashMap;
use std::sync::Arc;

use lazy_static::lazy_static;
use tokio::sync::RwLock as ARwLock;

use super::schema::{AnalysisFacts, PirAnalyzeResult, PirApprovalStatus, PirDocument};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PirSessionStatus {
    Idle,
    Analyzing,
    Ready,
    Error,
}

impl PirSessionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            PirSessionStatus::Idle => "idle",
            PirSessionStatus::Analyzing => "analyzing",
            PirSessionStatus::Ready => "ready",
            PirSessionStatus::Error => "error",
        }
    }
}

#[derive(Clone, Debug)]
pub struct PirMakerSession {
    pub chat_id: String,
    pub project_path: Option<String>,
    pub status: PirSessionStatus,
    pub watch_enabled: bool,
    pub last_result: Option<PirAnalyzeResult>,
    pub last_facts: Option<AnalysisFacts>,
    pub error: Option<String>,
    pub updated_at_ms: u64,
}

lazy_static! {
    static ref SESSIONS: Arc<ARwLock<HashMap<String, PirMakerSession>>> =
        Arc::new(ARwLock::new(HashMap::new()));
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub async fn get(chat_id: &str) -> Option<PirMakerSession> {
    SESSIONS.read().await.get(chat_id).cloned()
}

pub async fn set_analyzing(chat_id: &str, project_path: &str) {
    let mut map = SESSIONS.write().await;
    let entry = map.entry(chat_id.to_string()).or_insert(PirMakerSession {
        chat_id: chat_id.to_string(),
        project_path: None,
        status: PirSessionStatus::Idle,
        watch_enabled: false,
        last_result: None,
        last_facts: None,
        error: None,
        updated_at_ms: now_ms(),
    });
    entry.status = PirSessionStatus::Analyzing;
    entry.project_path = Some(project_path.to_string());
    entry.error = None;
    entry.updated_at_ms = now_ms();
}

pub async fn set_ready(chat_id: &str, result: PirAnalyzeResult, facts: Option<AnalysisFacts>) {
    let mut map = SESSIONS.write().await;
    if let Some(entry) = map.get_mut(chat_id) {
        entry.status = PirSessionStatus::Ready;
        entry.last_result = Some(result);
        entry.last_facts = facts;
        entry.error = None;
        entry.updated_at_ms = now_ms();
    }
}

pub async fn set_error(chat_id: &str, err: String) {
    let mut map = SESSIONS.write().await;
    if let Some(entry) = map.get_mut(chat_id) {
        entry.status = PirSessionStatus::Error;
        entry.error = Some(err);
        entry.updated_at_ms = now_ms();
    }
}

pub async fn set_watch(chat_id: &str, enabled: bool) {
    let mut map = SESSIONS.write().await;
    if let Some(entry) = map.get_mut(chat_id) {
        entry.watch_enabled = enabled;
        entry.updated_at_ms = now_ms();
    }
}

pub async fn approve(chat_id: &str, comment: Option<String>) -> Result<PirDocument, String> {
    let mut map = SESSIONS.write().await;
    let entry = map.get_mut(chat_id).ok_or("no PIR session for chat")?;
    let result = entry
        .last_result
        .as_mut()
        .ok_or("no analyzed PIR document")?;
    let revision = result.pir.revision.clone();
    result.pir.approval.status = PirApprovalStatus::Approved;
    result.pir.approval.approved_at_ms = Some(now_ms());
    result.pir.approval.approved_revision = Some(revision);
    result.pir.approval.comment = comment;
    Ok(result.pir.clone())
}
