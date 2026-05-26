use std::path::PathBuf;

use axum::Extension;
use axum::response::Result;
use hyper::{Body, Response, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::json;
use url::Url;
use tracing::info;

use crate::custom_error::ScratchError;
use crate::global_context::SharedGlobalContext;
use crate::files_in_workspace;

#[derive(Serialize, Deserialize, Clone)]
pub struct LspLikeInit {
    pub project_roots: Vec<Url>,
}

#[derive(Serialize, Deserialize, Clone)]
struct LspLikeDidChange {
    pub uri: Url,
    pub text: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct LspLikeSetActiveDocument {
    pub uri: Url,
}

#[derive(Serialize, Deserialize, Clone)]
struct LspLikeAddFolder {
    pub uri: Url,
}

/// Separator used to pack multiple workspace roots into the single `cmdline.workspace_folder`
/// string.  On Windows every path already contains `:` after the drive letter (e.g. `C:\foo`),
/// so we switch to `|` on that platform to avoid collisions.
fn workspace_folder_sep() -> &'static str {
    if cfg!(windows) { "|" } else { ":" }
}

pub async fn handle_v1_lsp_initialize(
    Extension(global_context): Extension<SharedGlobalContext>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let post = serde_json::from_slice::<LspLikeInit>(&body_bytes).map_err(|e| {
        ScratchError::new(StatusCode::BAD_REQUEST, format!("JSON problem: {}", e))
    })?;

    let mut workspace_dirs: Vec<PathBuf> = vec![];
    for x in post.project_roots {
        let path = crate::files_correction::canonical_path(&x.to_file_path().unwrap_or_default().to_string_lossy().to_string());
        workspace_dirs.push(path);
    }
    
    // Convert workspace_dirs to strings for comparison (already canonicalized)
    // This ensures consistent hashing with Python CLI which also canonicalizes paths
    let workspace_dirs_str: Vec<String> = workspace_dirs.iter().map(|p| p.to_string_lossy().to_string()).collect();
    let new_workspace_folder = if workspace_dirs_str.is_empty() {
        String::new()
    } else {
        workspace_dirs_str.join(workspace_folder_sep())
    };
    
    // Check if workspace folder changed and VecDB needs reinitialization
    let needs_vecdb_reinit = {
        let gcx_locked = global_context.read().await;
        let old_workspace_folder_raw = gcx_locked.cmdline.workspace_folder.clone();
        let vecdb_enabled = gcx_locked.cmdline.vecdb;
        let vecdb_exists = gcx_locked.vec_db.lock().await.is_some();
        
        // Canonicalize old workspace folder for comparison (same as VecDB init does)
        // This ensures we compare canonicalized paths, matching how VecDB calculates the hash
        let old_workspace_folder = if old_workspace_folder_raw.is_empty() {
            String::new()
        } else {
            // Split by platform separator in case of multiple folders, canonicalize each, then rejoin
            let old_folders: Vec<String> = old_workspace_folder_raw.split(workspace_folder_sep())
                .map(|p| {
                    let canonical = crate::files_correction::canonical_path(p);
                    canonical.to_string_lossy().to_string()
                })
                .collect();
            old_folders.join(workspace_folder_sep())
        };
        
        // Check if workspace folder changed (comparing canonicalized paths)
        let workspace_changed = old_workspace_folder != new_workspace_folder;
        
        // If VecDB is enabled and workspace changed, we need to reinitialize
        vecdb_enabled && vecdb_exists && workspace_changed
    };
    
    // Update workspace folders in documents_state
    *global_context.write().await.documents_state.workspace_folders.lock().unwrap() = workspace_dirs.clone();
    
    // Update cmdline.workspace_folder to match new workspace folders
    // This is important for VecDB to use the correct workspace hash
    {
        let mut gcx_locked = global_context.write().await;
        gcx_locked.cmdline.workspace_folder = new_workspace_folder.clone();
    }
    
    // Reinitialize VecDB if workspace folder changed
    let vecdb_was_reinitialized = if needs_vecdb_reinit {
        info!("Workspace folder changed from command line, reinitializing VecDB with new workspace: {}", new_workspace_folder);
        
        // Get VecdbConstants for reinitialization
        let (need_reload, constants_opt) = crate::vecdb::vdb_highlev::do_i_need_to_reload_vecdb(global_context.clone()).await;
        
        let mut reinit_success = false;
        
        if need_reload {
            // Background tasks are managed by BackgroundTasksHolder, which will be replaced
            // when we reinitialize VecDB, so we don't need to explicitly abort them here
            
            if let Some(constants) = constants_opt {
                let init_config = crate::vecdb::vdb_init::VecDbInitConfig {
                    max_attempts: 5,
                    initial_delay_ms: 100,
                    max_delay_ms: 2000,
                    backoff_factor: 2.0,
                    test_search_after_init: false, // Skip test search for faster reinit
                };
                
                match crate::vecdb::vdb_init::initialize_vecdb_with_context(
                    global_context.clone(),
                    constants,
                    Some(init_config),
                ).await {
                    Ok(_) => {
                        global_context.write().await.vec_db_error = "".to_string();
                        info!("VecDB reinitialized successfully with new workspace folder");
                        reinit_success = true;
                    }
                    Err(err) => {
                        let err_msg = err.to_string();
                        global_context.write().await.vec_db_error = err_msg.clone();
                        tracing::error!("VecDB reinitialization failed: {}", err_msg);
                        // Continue anyway - VecDB will use old workspace until next successful reinit
                    }
                }
            } else {
                tracing::warn!("VecDB reinitialization needed but constants not available");
            }
        } else {
            // Even if do_i_need_to_reload_vecdb says no reload needed (same embedding model),
            // we still need to reinitialize because workspace folder changed
            // Force reinitialization by calling initialize_vecdb_with_context
            if let Ok(caps) = crate::global_context::try_load_caps_quickly_if_not_present(global_context.clone(), 0).await {
                let vecdb_max_files = global_context.read().await.cmdline.vecdb_max_files;
                let mut consts = crate::vecdb::vdb_structs::VecdbConstants {
                    embedding_model: caps.embedding_model.clone(),
                    tokenizer: None,
                    splitter_window_size: 1500,
                    vecdb_max_files: vecdb_max_files,
                };
                
                let tokenizer_result = crate::tokens::cached_tokenizer(
                    global_context.clone(), &consts.embedding_model.base,
                ).await;
                
                consts.tokenizer = match tokenizer_result {
                    Ok(tokenizer) => tokenizer,  // tokenizer is already Option<Arc<Tokenizer>>
                    Err(err) => {
                        tracing::error!("VecDB reinitialization failed: tokenizer not available: {}", err);
                        None
                    }
                };
                
                if consts.tokenizer.is_some() {
                    let init_config = crate::vecdb::vdb_init::VecDbInitConfig {
                        max_attempts: 5,
                        initial_delay_ms: 100,
                        max_delay_ms: 2000,
                        backoff_factor: 2.0,
                        test_search_after_init: false,
                    };
                    
                    match crate::vecdb::vdb_init::initialize_vecdb_with_context(
                        global_context.clone(),
                        consts,
                        Some(init_config),
                    ).await {
                        Ok(_) => {
                            global_context.write().await.vec_db_error = "".to_string();
                            info!("VecDB reinitialized successfully with new workspace folder");
                            reinit_success = true;
                        }
                        Err(err) => {
                            let err_msg = err.to_string();
                            global_context.write().await.vec_db_error = err_msg.clone();
                            tracing::error!("VecDB reinitialization failed: {}", err_msg);
                        }
                    }
                }
            } else {
                tracing::warn!("Cannot reinitialize VecDB: caps not available");
            }
        }
        
        reinit_success
    } else {
        false
    };
    
    // Only call on_workspaces_init if VecDB was NOT reinitialized
    // (because initialize_vecdb_with_context already enqueued files)
    let files_count = if vecdb_was_reinitialized {
        // VecDB reinitialization already enqueued files via initialize_vecdb_with_context
        // We still need to do watcher_init and other setup, but skip file enqueuing
        let folders = global_context.read().await.documents_state.workspace_folders.lock().unwrap().clone();
        let old_app_searchable_id = global_context.read().await.app_searchable_id.clone();
        let new_app_searchable_id = crate::global_context::get_app_searchable_id(&folders);
        if old_app_searchable_id != new_app_searchable_id {
            global_context.write().await.app_searchable_id = new_app_searchable_id;
            crate::cloud::threads_sub::trigger_threads_subscription_restart(global_context.clone()).await;
        }
        // crate::files_in_workspace::watcher_init(global_context.clone()).await;
        tokio::spawn(crate::files_in_workspace::watcher_init(global_context.clone()));
        crate::git::checkpoints::enqueue_init_shadow_repos(global_context.clone()).await;
        let _ = crate::integrations::running_integrations::load_integrations(global_context.clone(), &["**/mcp_*".to_string()]).await;
        
        // Get the file count that was already enqueued (without enqueuing again)
        let folders = global_context.read().await.documents_state.workspace_folders.lock().unwrap().clone();
        let mut indexing_everywhere = crate::files_blocklist::reload_global_indexing_only(global_context.clone()).await;
        let (all_files, _) = crate::files_in_workspace::retrieve_files_in_workspace_folders(
            folders,
            &mut indexing_everywhere,
            false,
            false
        ).await;
        all_files.len() as i32
    } else {
        // Normal case: call on_workspaces_init (which will enqueue files)
        files_in_workspace::on_workspaces_init(global_context).await
    };
    Ok(Response::builder()
        .status(StatusCode::OK)
        .body(Body::from(json!({"success": 1, "files_found": files_count}).to_string()))
        .unwrap())
}

pub async fn handle_v1_lsp_did_change(
    Extension(global_context): Extension<SharedGlobalContext>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let post = serde_json::from_slice::<LspLikeDidChange>(&body_bytes).map_err(|e| {
        ScratchError::new(StatusCode::BAD_REQUEST, format!("JSON problem: {}", e))
    })?;
    let cpath = crate::files_correction::canonical_path(&post.uri.to_file_path().unwrap_or_default().to_string_lossy().to_string());
    files_in_workspace::on_did_change(
        global_context.clone(),
        &cpath,
        &post.text,
    ).await;
    Ok(Response::builder()
        .status(StatusCode::OK)
        .body(Body::from(json!({"success": 1}).to_string()))
        .unwrap())
}

pub async fn handle_v1_set_active_document(
    Extension(global_context): Extension<SharedGlobalContext>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let post = serde_json::from_slice::<LspLikeSetActiveDocument>(&body_bytes).map_err(|e| {
        ScratchError::new(StatusCode::BAD_REQUEST, format!("JSON problem: {}", e))
    })?;
    let path = crate::files_correction::canonical_path(&post.uri.to_file_path().unwrap_or_default().display().to_string());
    tracing::info!("ACTIVE_DOC {:?}", crate::nicer_logs::last_n_chars(&path.to_string_lossy().to_string(), 30));
    global_context.write().await.documents_state.active_file_path = Some(path);
    Ok(Response::builder()
        .status(StatusCode::OK)
        .body(Body::from(json!({"success": true}).to_string()))
        .unwrap())
}

pub async fn handle_v1_lsp_add_folder(
    Extension(global_context): Extension<SharedGlobalContext>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let post = serde_json::from_slice::<LspLikeAddFolder>(&body_bytes).map_err(|e| {
        ScratchError::new(StatusCode::BAD_REQUEST, format!("JSON problem: {}", e))
    })?;
    let cpath = crate::files_correction::canonical_path(&post.uri.to_file_path().unwrap_or_default().to_string_lossy().to_string());
    files_in_workspace::add_folder(global_context.clone(), &cpath).await;
    Ok(Response::builder()
       .status(StatusCode::OK)
       .body(Body::from(json!({"success": 1}).to_string()))
       .unwrap())
}

pub async fn handle_v1_lsp_remove_folder(
    Extension(global_context): Extension<SharedGlobalContext>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let post = serde_json::from_slice::<LspLikeAddFolder>(&body_bytes).map_err(|e| {
        ScratchError::new(StatusCode::BAD_REQUEST, format!("JSON problem: {}", e))
    })?;
    let cpath = crate::files_correction::canonical_path(&post.uri.to_file_path().unwrap_or_default().to_string_lossy().to_string());
    files_in_workspace::remove_folder(global_context.clone(), &cpath).await;
    Ok(Response::builder()
        .status(StatusCode::OK)
        .body(Body::from(json!({"success": 1}).to_string()))
        .unwrap())
}
