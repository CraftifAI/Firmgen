use crate::at_commands::at_commands::{vec_context_file_to_context_tools, AtCommand, AtCommandsContext, AtParam};
use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex as AMutex;
use tracing::info;
use crate::nicer_logs::last_n_chars;

use crate::at_commands::execute_at::AtCommandMember;
use crate::call_validation::{ContextEnum, ContextFile};
use crate::vecdb;
use crate::vecdb::vdb_structs::VecdbSearch;


pub fn text_on_clip(query: &String, from_tool_call: bool) -> String {
    if !from_tool_call {
        return query.clone();
    }
    return format!("performed vecdb search, results below");
}


pub struct AtSearch {
    pub params: Vec<Box<dyn AtParam>>,
}

impl AtSearch {
    pub fn new() -> Self {
        AtSearch {
            params: vec![],
        }
    }
}

/// Maximum number of lines to include from a single VecDB chunk.
/// Large chunks (e.g., entire files) with gradient_type=4 assign usefulness=100.0
/// to every line in the range, which can flood the context with thousands of lines.
/// We clamp the range to this limit, centered on the chunk midpoint.
const MAX_CHUNK_LINES: usize = 200;

fn results2message(results: &Vec<vecdb::vdb_structs::VecdbRecord>, workspace_root: Option<PathBuf>) -> Vec<ContextFile> {
    let mut vector_of_context_file: Vec<ContextFile> = vec![];
    for r in results {
        // Resolve file path to a path likely valid on this machine before passing to tools/LLM
        let mut file_name = r.file_path.to_str().unwrap().to_string();
        if !r.file_path.exists() {
            if let Some(root) = workspace_root.as_ref() {
                let candidate = root.join(file_name.trim_start_matches('/'));
                if candidate.exists() {
                    file_name = candidate.to_string_lossy().to_string();
                } else {
                    let candidate2 = root.join(r.file_path.clone());
                    if candidate2.exists() {
                        file_name = candidate2.to_string_lossy().to_string();
                    }
                }
            }
        }
        let mut usefulness = r.usefulness;
        // diversifying results
        let same_file_again =  vector_of_context_file.iter().map(|x|&x.file_name).filter(|x|**x == file_name).count();
        let same_file_discount = 1. / (same_file_again as f32 * 0.1 + 1.);
        usefulness *= same_file_discount;
        info!("results {} usefulness {:.2} after same-file discount {:.2}",
            last_n_chars(&file_name, 30),
            usefulness,
            same_file_discount,
        );
        // Clamp line range to MAX_CHUNK_LINES centered on midpoint.
        // Without this, entire-file chunks (e.g. 1649 lines) cause gradient_type=4
        // to mark every line usefulness=100.0, flooding the context.
        let raw_line1 = r.start_line as usize + 1;
        let raw_line2 = r.end_line as usize + 1;
        let (line1, line2) = if raw_line2.saturating_sub(raw_line1) > MAX_CHUNK_LINES {
            let midpoint = (raw_line1 + raw_line2) / 2;
            let half = MAX_CHUNK_LINES / 2;
            let clamped1 = midpoint.saturating_sub(half).max(1);
            let clamped2 = clamped1 + MAX_CHUNK_LINES;
            info!("results {} clamping line range {}-{} -> {}-{} (exceeded MAX_CHUNK_LINES={})",
                last_n_chars(&file_name, 30), raw_line1, raw_line2, clamped1, clamped2, MAX_CHUNK_LINES);
            (clamped1, clamped2)
        } else {
            (raw_line1, raw_line2)
        };
        vector_of_context_file.push(ContextFile {
            file_name,
            file_content: "".to_string(),
            line1,
            line2,
            symbols: vec![],
            gradient_type: 4,
            usefulness,
        });
    }
    vector_of_context_file
}

pub async fn execute_at_search(
    ccx: Arc<AMutex<AtCommandsContext>>,
    query: &String,
    vecdb_scope_filter_mb: Option<String>,
) -> Result<Vec<ContextFile>, String> {
    let (gcx, top_n) = {
        let ccx_locked = ccx.lock().await;
        (ccx_locked.global_context.clone(), ccx_locked.top_n)
    };

    let top_n_twice_as_big = top_n * 2;  // top_n will be cut at postprocessing stage, and we really care about top_n files, not pieces
    
    // Determine workspace root if available (for resolving relative paths from static VecDB)
    let workspace_root = {
        let cx = gcx.read().await;
        let folders = cx.documents_state.workspace_folders.lock().unwrap().clone();
        if !folders.is_empty() {
            Some(folders[0].clone())
        } else if !cx.cmdline.workspace_folder.is_empty() {
            Some(PathBuf::from(cx.cmdline.workspace_folder.clone()))
        } else {
            None
        }
    };

    // Try static VecDB first (preferred - read-only, no corruption issues)
    let static_vec_db = gcx.read().await.static_vec_db.clone();
    {
        let static_db = static_vec_db.lock().await;
        if !static_db.is_empty() {
            info!("Using static VecDB for search");
            let search_result = static_db.vecdb_search(query.clone(), top_n_twice_as_big, vecdb_scope_filter_mb.clone()).await?;
            let results = search_result.results.clone();
            return Ok(results2message(&results, workspace_root));
        }
    }

    // Fall back to dynamic VecDB if no static DBs loaded
    let vec_db = gcx.read().await.vec_db.clone();
    let r = match *vec_db.lock().await {
        Some(ref db) => {
            info!("Using dynamic VecDB for search");
            let search_result = db.vecdb_search(query.clone(), top_n_twice_as_big, vecdb_scope_filter_mb).await?;
            let results = search_result.results.clone();
            return Ok(results2message(&results, workspace_root));
        }
        None => Err("VecDB is not active. No static VecDBs loaded and dynamic VecDB is not running. Use --static-vecdb to load pre-built databases.".to_string())
    };
    r
}

#[async_trait]
impl AtCommand for AtSearch {
    fn params(&self) -> &Vec<Box<dyn AtParam>> {
        &self.params
    }

    async fn at_execute(
        &self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        _cmd: &mut AtCommandMember,
        args: &mut Vec<AtCommandMember>,
    ) -> Result<(Vec<ContextEnum>, String), String> {
        let args1 = args.iter().map(|x|x.clone()).collect::<Vec<_>>();
        info!("execute @search {:?}", args1.iter().map(|x|x.text.clone()).collect::<Vec<_>>());

        let query = args.iter().map(|x|x.text.clone()).collect::<Vec<_>>().join(" ");
        if query.trim().is_empty() {
            if ccx.lock().await.is_preview {
                return Ok((vec![], "".to_string()));
            }
            return Err("Cannot execute search: query is empty.".to_string());
        }

        let vector_of_context_file = execute_at_search(ccx.clone(), &query, None).await?;
        let text = text_on_clip(&query, false);
        Ok((vec_context_file_to_context_tools(vector_of_context_file), text))
    }

    fn depends_on(&self) -> Vec<String> {
        vec!["vecdb".to_string()]
    }
}
