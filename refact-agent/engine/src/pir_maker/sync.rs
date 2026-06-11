//! Debounced live re-analysis scheduling.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Weak;
use std::time::Duration;

use lazy_static::lazy_static;
use tokio::sync::RwLock as ARwLock;

use crate::global_context::GlobalContext;

lazy_static! {
    static ref WATCH_TARGETS: Arc<ARwLock<HashMap<String, WatchTarget>>> =
        Arc::new(ARwLock::new(HashMap::new()));
    static ref DEBOUNCE_GEN: Arc<ARwLock<u64>> = Arc::new(ARwLock::new(0));
}

#[derive(Clone)]
struct WatchTarget {
    chat_id: String,
    project_path: PathBuf,
}

/// Register a chat for live PIR sync (debounced re-analyze triggered externally).
pub async fn enable_watch(chat_id: &str, project_path: PathBuf) {
    WATCH_TARGETS.write().await.insert(
        chat_id.to_string(),
        WatchTarget {
            chat_id: chat_id.to_string(),
            project_path,
        },
    );
}

pub async fn disable_watch(chat_id: &str) {
    WATCH_TARGETS.write().await.remove(chat_id);
}

/// Called when files under an ESP project may have changed.
pub async fn notify_project_changed(
    gcx_weak: Weak<ARwLock<GlobalContext>>,
    changed_path: &PathBuf,
) {
    let Some(gcx) = gcx_weak.upgrade() else {
        return;
    };

    let targets: Vec<WatchTarget> = WATCH_TARGETS
        .read()
        .await
        .values()
        .filter(|t| changed_path.starts_with(&t.project_path))
        .cloned()
        .collect();

    if targets.is_empty() {
        return;
    }

    let mut gen = DEBOUNCE_GEN.write().await;
    *gen += 1;
    let my_gen = *gen;
    drop(gen);

    for target in targets {
        let chat_id = target.chat_id.clone();
        let path = target.project_path.clone();
        let gcx = gcx.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(800)).await;
            let current = *DEBOUNCE_GEN.read().await;
            if current != my_gen {
                return;
            }
            super::service::spawn_analyze_for_chat(
                gcx,
                &chat_id,
                Some(path.to_string_lossy().as_ref()),
                true,
                "watcher",
                None,
            )
            .await;
        });
    }
}

/// Notify for any path under a watched project (checks all watch targets).
pub async fn notify_paths_changed(
    gcx_weak: Weak<ARwLock<GlobalContext>>,
    changed_paths: &[String],
) {
    for p in changed_paths {
        notify_project_changed(gcx_weak.clone(), &PathBuf::from(p)).await;
    }
}
