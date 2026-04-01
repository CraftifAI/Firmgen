use std::path::PathBuf;
use std::sync::Arc;
use rusqlite::{Connection, OpenFlags};
use tokio::sync::Mutex as AMutex;
use tracing::{info, warn};

use crate::ast::file_splitter::AstBasedFileSplitter;
use crate::caps::EmbeddingModelRecord;
use crate::fetch_embedding::get_embedding_with_retries;
use crate::file_filter::is_valid_file;
use crate::files_in_workspace::Document;
use crate::global_context::GlobalContext;

/// Configuration for building a static VecDB
pub struct StaticVecDbBuildConfig {
    pub source_directory: PathBuf,
    pub output_path: PathBuf,
    pub embedding_model: EmbeddingModelRecord,
    pub chunk_size: usize,
    pub max_files: usize,
}

/// Result of building a static VecDB
pub struct StaticVecDbBuildResult {
    pub files_processed: usize,
    pub chunks_created: usize,
    pub embeddings_generated: usize,
    pub errors: Vec<String>,
}

/// Build a static VecDB from a directory
pub async fn build_static_vecdb(
    config: StaticVecDbBuildConfig,
    gcx: Arc<tokio::sync::RwLock<GlobalContext>>,
) -> Result<StaticVecDbBuildResult, String> {
    info!("Building static VecDB from {:?} to {:?}", config.source_directory, config.output_path);
    
    // Create output directory if needed
    if let Some(parent) = config.output_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create output directory: {}", e))?;
    }
    
    // Remove existing file if it exists
    if config.output_path.exists() {
        std::fs::remove_file(&config.output_path).map_err(|e| format!("Failed to remove existing file: {}", e))?;
    }
    
    // Create SQLite database
    let conn = Connection::open_with_flags(
        &config.output_path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
    ).map_err(|e| format!("Failed to create database: {}", e))?;
    
    // Create schema
    create_static_vecdb_schema(&conn, config.embedding_model.embedding_size)?;
    
    // Store metadata
    store_metadata(&conn, &config)?;
    
    // Collect files
    let files = collect_files(&config.source_directory, config.max_files)?;
    info!("Found {} files to process", files.len());
    
    // Create HTTP client for embeddings
    let http_client = Arc::new(AMutex::new(reqwest::Client::new()));
    
    // Process files
    let mut result = StaticVecDbBuildResult {
        files_processed: 0,
        chunks_created: 0,
        embeddings_generated: 0,
        errors: vec![],
    };
    
    let file_splitter = AstBasedFileSplitter::new(config.chunk_size);
    let tokens_limit = (config.chunk_size as f64 * 1.5) as usize;
    
    let total_files = files.len();
    let mut all_chunks: Vec<(PathBuf, String, u64, u64)> = vec![]; // (file_path, text, start_line, end_line)
    
    for (idx, file_path) in files.iter().enumerate() {
        if idx % 100 == 0 {
            info!("Scanning files: {}/{}", idx, total_files);
        }
        
        let mut doc = Document {
            doc_path: file_path.clone(),
            doc_text: None,
        };
        
        if let Err(e) = doc.update_text_from_disk(gcx.clone()).await {
            result.errors.push(format!("Failed to read {:?}: {}", file_path, e));
            continue;
        }
        
        if let Err(e) = doc.does_text_look_good() {
            continue; // Skip binary/invalid files silently
        }
        
        // Split file into chunks
        let splits = match file_splitter.vectorization_split(&doc, None, gcx.clone(), tokens_limit).await {
            Ok(s) => s,
            Err(e) => {
                result.errors.push(format!("Failed to split {:?}: {}", file_path, e));
                continue;
            }
        };
        
        for split in splits {
            all_chunks.push((
                split.file_path,
                split.window_text,
                split.start_line,
                split.end_line,
            ));
        }
        
        // Also add filename as searchable
        if let Some(filename) = file_path.file_name() {
            let filename_str = filename.to_string_lossy().to_string();
            let end_line = doc.doc_text.as_ref().map(|t| t.lines().count() as u64).unwrap_or(0);
            all_chunks.push((file_path.clone(), filename_str, 0, end_line));
        }
        
        result.files_processed += 1;
    }
    
    info!("Collected {} chunks from {} files", all_chunks.len(), result.files_processed);
    result.chunks_created = all_chunks.len();
    
    // Generate embeddings in batches
    let batch_size = config.embedding_model.embedding_batch;
    let total_chunks = all_chunks.len();
    
    for (batch_idx, chunk_batch) in all_chunks.chunks(batch_size).enumerate() {
        if batch_idx % 10 == 0 {
            info!("Generating embeddings: batch {}/{}", batch_idx, (total_chunks + batch_size - 1) / batch_size);
        }
        
        let texts: Vec<String> = chunk_batch.iter().map(|(_, text, _, _)| text.clone()).collect();
        
        let embeddings = match get_embedding_with_retries(
            http_client.clone(),
            &config.embedding_model,
            texts,
            10,
        ).await {
            Ok(e) => e,
            Err(e) => {
                result.errors.push(format!("Embedding batch {} failed: {}", batch_idx, e));
                continue;
            }
        };
        
        // Insert into database
        for (i, embedding) in embeddings.iter().enumerate() {
            if embedding.is_empty() {
                continue;
            }
            
            let (file_path, _, start_line, end_line) = &chunk_batch[i];
            let file_path_str = file_path.to_string_lossy().to_string();
            let embedding_bytes: Vec<u8> = embedding.iter()
                .flat_map(|&f| f.to_ne_bytes())
                .collect();
            
            if let Err(e) = conn.execute(
                "INSERT INTO embeddings (embedding, file_path, start_line, end_line) VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![embedding_bytes, file_path_str, start_line, end_line],
            ) {
                result.errors.push(format!("Failed to insert embedding: {}", e));
                continue;
            }
            
            result.embeddings_generated += 1;
        }
        
        // Small delay to avoid overwhelming the embedding server
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
    
    // Store file hashes for rebuild detection
    store_file_hashes(&conn, &files)?;
    
    // Finalize
    conn.execute("PRAGMA optimize", []).ok();
    
    info!("Static VecDB build complete: {} files, {} chunks, {} embeddings", 
          result.files_processed, result.chunks_created, result.embeddings_generated);
    
    if !result.errors.is_empty() {
        warn!("Build had {} errors", result.errors.len());
        for err in result.errors.iter().take(10) {
            warn!("  {}", err);
        }
    }
    
    Ok(result)
}

fn create_static_vecdb_schema(conn: &Connection, embedding_size: i32) -> Result<(), String> {
    // Metadata table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS metadata (
            key TEXT PRIMARY KEY,
            value TEXT
        )",
        [],
    ).map_err(|e| format!("Failed to create metadata table: {}", e))?;
    
    // vec0 embeddings table
    conn.execute(
        &format!(
            "CREATE VIRTUAL TABLE IF NOT EXISTS embeddings USING vec0(
                embedding float[{}] distance_metric=cosine,
                file_path TEXT,
                +start_line INTEGER,
                +end_line INTEGER
            )", embedding_size
        ),
        [],
    ).map_err(|e| format!("Failed to create embeddings table: {}", e))?;
    
    // File hashes for rebuild detection
    conn.execute(
        "CREATE TABLE IF NOT EXISTS files (
            path TEXT PRIMARY KEY,
            hash TEXT,
            size INTEGER,
            modified INTEGER
        )",
        [],
    ).map_err(|e| format!("Failed to create files table: {}", e))?;
    
    Ok(())
}

fn store_metadata(conn: &Connection, config: &StaticVecDbBuildConfig) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    
    conn.execute(
        "INSERT OR REPLACE INTO metadata (key, value) VALUES ('source_directory', ?1)",
        [config.source_directory.to_string_lossy().to_string()],
    ).map_err(|e| e.to_string())?;
    
    conn.execute(
        "INSERT OR REPLACE INTO metadata (key, value) VALUES ('build_time', ?1)",
        [&now],
    ).map_err(|e| e.to_string())?;
    
    conn.execute(
        "INSERT OR REPLACE INTO metadata (key, value) VALUES ('embedding_model', ?1)",
        [&config.embedding_model.base.name],
    ).map_err(|e| e.to_string())?;
    
    conn.execute(
        "INSERT OR REPLACE INTO metadata (key, value) VALUES ('embedding_size', ?1)",
        [config.embedding_model.embedding_size.to_string()],
    ).map_err(|e| e.to_string())?;
    
    conn.execute(
        "INSERT OR REPLACE INTO metadata (key, value) VALUES ('chunk_size', ?1)",
        [config.chunk_size.to_string()],
    ).map_err(|e| e.to_string())?;
    
    Ok(())
}

fn store_file_hashes(conn: &Connection, files: &[PathBuf]) -> Result<(), String> {
    for file_path in files {
        let metadata = match std::fs::metadata(file_path) {
            Ok(m) => m,
            Err(_) => continue,
        };
        
        let size = metadata.len() as i64;
        let modified = metadata.modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        
        // Simple hash based on size and modified time (fast, good enough for change detection)
        let hash = format!("{}_{}", size, modified);
        
        conn.execute(
            "INSERT OR REPLACE INTO files (path, hash, size, modified) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![file_path.to_string_lossy().to_string(), hash, size, modified],
        ).ok();
    }
    
    Ok(())
}

fn collect_files(directory: &PathBuf, max_files: usize) -> Result<Vec<PathBuf>, String> {
    let mut files = vec![];
    collect_files_recursive(directory, &mut files, max_files)?;
    Ok(files)
}

fn collect_files_recursive(dir: &PathBuf, files: &mut Vec<PathBuf>, max_files: usize) -> Result<(), String> {
    if files.len() >= max_files {
        return Ok(());
    }
    
    let entries = std::fs::read_dir(dir).map_err(|e| format!("Failed to read dir {:?}: {}", dir, e))?;
    
    for entry in entries {
        if files.len() >= max_files {
            break;
        }
        
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        
        let path = entry.path();
        
        // Skip hidden files/directories
        if path.file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.starts_with('.'))
            .unwrap_or(false)
        {
            continue;
        }
        
        if path.is_dir() {
            // Skip common non-source directories
            let dir_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if matches!(dir_name, "node_modules" | "target" | "build" | "dist" | "__pycache__" | ".git") {
                continue;
            }
            collect_files_recursive(&path, files, max_files)?;
        } else if path.is_file() {
            // Check if it's a valid source file
            if is_valid_file(&path, false, false).is_ok() {
                files.push(path);
            }
        }
    }
    
    Ok(())
}

