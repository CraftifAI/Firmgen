use std::path::PathBuf;
use std::sync::Arc;
use async_trait::async_trait;
use tokio_rusqlite::Connection;
use rusqlite::OpenFlags;
use tokio::sync::Mutex as AMutex;
use tracing::{info, warn};

use crate::caps::EmbeddingModelRecord;
use crate::fetch_embedding::get_embedding_with_retries;
use crate::vecdb::vdb_structs::{SearchResult, VecdbRecord, VecdbSearch};

/// A read-only static VecDB loaded from a pre-built SQLite file
pub struct StaticVecDb {
    conn: Connection,
    pub name: String,
    pub source_directory: String,
    pub embedding_model_name: String,
    pub embedding_size: i32,
    pub file_count: usize,
    pub embedding_count: usize,
}

/// Metadata about a static VecDB
#[derive(Debug, Clone)]
pub struct StaticVecDbInfo {
    pub name: String,
    pub path: PathBuf,
    pub source_directory: String,
    pub build_time: String,
    pub embedding_model: String,
    pub embedding_size: i32,
    pub file_count: usize,
    pub embedding_count: usize,
}

impl StaticVecDb {
    /// Load a static VecDB from a file (read-only)
    pub async fn load(path: &PathBuf) -> Result<Self, String> {
        let name = path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();
        
        info!("Loading static VecDB '{}' from {:?}", name, path);
        
        if !path.exists() {
            return Err(format!("Static VecDB file not found: {:?}", path));
        }
        
        // Open read-only with tokio_rusqlite
        let conn = Connection::open_with_flags(
            path.clone(),
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        ).await.map_err(|e| format!("Failed to open static VecDB: {}", e))?;
        
        // Read metadata
        let source_directory = conn.call(|conn| {
            Ok(conn.query_row(
                "SELECT value FROM metadata WHERE key = 'source_directory'",
                [],
                |row| row.get::<_, String>(0),
            ).unwrap_or_default())
        }).await.unwrap_or_default();
        
        let embedding_model_name = conn.call(|conn| {
            Ok(conn.query_row(
                "SELECT value FROM metadata WHERE key = 'embedding_model'",
                [],
                |row| row.get::<_, String>(0),
            ).unwrap_or_default())
        }).await.unwrap_or_default();
        
        let embedding_size: i32 = conn.call(|conn| {
            Ok(conn.query_row(
                "SELECT value FROM metadata WHERE key = 'embedding_size'",
                [],
                |row| row.get::<_, String>(0),
            ).unwrap_or_default().parse().unwrap_or(1536))
        }).await.unwrap_or(1536);
        
        let file_count: usize = conn.call(|conn| {
            Ok(conn.query_row(
                "SELECT COUNT(*) FROM files",
                [],
                |row| row.get(0),
            ).unwrap_or(0))
        }).await.unwrap_or(0);
        
        let embedding_count: usize = conn.call(|conn| {
            Ok(conn.query_row(
                "SELECT COUNT(*) FROM embeddings",
                [],
                |row| row.get(0),
            ).unwrap_or(0))
        }).await.unwrap_or(0);
        
        info!("Loaded static VecDB '{}': {} files, {} embeddings, model={}", 
              name, file_count, embedding_count, embedding_model_name);
        
        Ok(StaticVecDb {
            conn,
            name,
            source_directory,
            embedding_model_name,
            embedding_size,
            file_count,
            embedding_count,
        })
    }
    
    /// Get info about this static VecDB
    pub fn get_info(&self, path: &PathBuf) -> StaticVecDbInfo {
        StaticVecDbInfo {
            name: self.name.clone(),
            path: path.clone(),
            source_directory: self.source_directory.clone(),
            build_time: String::new(), // Would need async to get this
            embedding_model: self.embedding_model_name.clone(),
            embedding_size: self.embedding_size,
            file_count: self.file_count,
            embedding_count: self.embedding_count,
        }
    }
    
    /// Search this static VecDB
    pub async fn search(
        &self,
        embedding: &Vec<f32>,
        top_n: usize,
        scope_filter: Option<String>,
    ) -> Result<Vec<VecdbRecord>, String> {
        let embedding_owned = embedding.clone();
        let scope_filter_owned = scope_filter.clone();
        
        self.conn.call(move |conn| {
            let embedding_bytes: Vec<u8> = embedding_owned.iter()
                .flat_map(|&f| f.to_ne_bytes())
                .collect();
            let mut results = Vec::new();
            
            if let Some(scope) = scope_filter_owned {
                let sql = r#"
                    SELECT file_path, start_line, end_line, embedding, distance
                    FROM embeddings
                    WHERE embedding MATCH ?1 AND k = ?2 AND file_path = ?3
                    ORDER BY distance
                "#;
                
                let mut stmt = conn.prepare(sql)?;
                
                let rows = stmt.query_map(
                    rusqlite::params![&embedding_bytes, top_n, scope],
                    |row| Self::row_to_record_static(row),
                )?;
                
                for row in rows {
                    if let Ok(record) = row {
                        results.push(record);
                    }
                }
            } else {
                let sql = r#"
                    SELECT file_path, start_line, end_line, embedding, distance
                    FROM embeddings
                    WHERE embedding MATCH ?1 AND k = ?2
                    ORDER BY distance
                "#;
                
                let mut stmt = conn.prepare(sql)?;
                
                let rows = stmt.query_map(
                    rusqlite::params![&embedding_bytes, top_n],
                    |row| Self::row_to_record_static(row),
                )?;
                
                for row in rows {
                    if let Ok(record) = row {
                        results.push(record);
                    }
                }
            }
            
            Ok(results)
        }).await.map_err(|e| format!("Search query failed: {}", e))
    }
    
    fn row_to_record_static(row: &rusqlite::Row) -> rusqlite::Result<VecdbRecord> {
        let vector_blob: Vec<u8> = row.get(3)?;
        let vector: Vec<f32> = vector_blob
            .chunks_exact(4)
            .map(|b| f32::from_ne_bytes(b.try_into().unwrap()))
            .collect();
        
        Ok(VecdbRecord {
            vector: Some(vector),
            file_path: PathBuf::from(row.get::<_, String>(0)?),
            start_line: row.get(1)?,
            end_line: row.get(2)?,
            distance: row.get(4)?,
            usefulness: 0.0,
        })
    }
}

/// Manager for multiple static VecDBs
pub struct MultiStaticVecDb {
    dbs: Vec<(PathBuf, StaticVecDb)>,
    http_client: Arc<AMutex<reqwest::Client>>,
    embedding_model: Option<EmbeddingModelRecord>,
}

impl MultiStaticVecDb {
    pub fn new() -> Self {
        MultiStaticVecDb {
            dbs: vec![],
            http_client: Arc::new(AMutex::new(reqwest::Client::new())),
            embedding_model: None,
        }
    }
    
    /// Set the embedding model to use for queries
    pub fn set_embedding_model(&mut self, model: EmbeddingModelRecord) {
        self.embedding_model = Some(model);
    }
    
    /// Load a static VecDB from a file
    pub async fn load(&mut self, path: PathBuf) -> Result<(), String> {
        let db = StaticVecDb::load(&path).await?;
        self.dbs.push((path, db));
        Ok(())
    }
    
    /// Get info about all loaded static VecDBs
    pub fn get_all_info(&self) -> Vec<StaticVecDbInfo> {
        self.dbs.iter()
            .map(|(path, db)| db.get_info(path))
            .collect()
    }
    
    /// Check if any DBs are loaded
    pub fn is_empty(&self) -> bool {
        self.dbs.is_empty()
    }
    
    /// Get total embedding count across all DBs
    pub fn total_embeddings(&self) -> usize {
        self.dbs.iter().map(|(_, db)| db.embedding_count).sum()
    }
}

#[async_trait]
impl VecdbSearch for MultiStaticVecDb {
    async fn vecdb_search(
        &self,
        query: String,
        top_n: usize,
        scope_filter: Option<String>,
    ) -> Result<SearchResult, String> {
        if self.dbs.is_empty() {
            return Err("No static VecDBs loaded".to_string());
        }
        
        let embedding_model = self.embedding_model.as_ref()
            .ok_or("Embedding model not set")?;
        
        // Get embedding for query
        let t0 = std::time::Instant::now();
        let embeddings = get_embedding_with_retries(
            self.http_client.clone(),
            embedding_model,
            vec![query.clone()],
            5,
        ).await.map_err(|e| format!("Failed to get query embedding: {}", e))?;
        
        if embeddings.is_empty() || embeddings[0].is_empty() {
            return Err("Empty embedding returned for query".to_string());
        }
        
        let query_embedding = &embeddings[0];
        info!("Query embedding took {:.3}s", t0.elapsed().as_secs_f64());
        
        // Search all DBs
        let t1 = std::time::Instant::now();
        let mut all_results: Vec<VecdbRecord> = vec![];
        
        for (path, db) in &self.dbs {
            match db.search(query_embedding, top_n, scope_filter.clone()).await {
                Ok(results) => {
                    all_results.extend(results);
                }
                Err(e) => {
                    warn!("Search in {:?} failed: {}", path, e);
                }
            }
        }
        
        // Sort by distance and take top_n
        all_results.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap_or(std::cmp::Ordering::Equal));
        all_results.truncate(top_n);
        
        // Calculate usefulness scores
        let rejection_threshold = embedding_model.rejection_threshold;
        let mut dist0 = 0.0;
        let mut filtered_results = Vec::new();
        
        for rec in all_results.iter_mut() {
            if dist0 == 0.0 {
                dist0 = rec.distance.abs();
            }
            rec.usefulness = 100.0 - 75.0 * ((rec.distance.abs() - dist0) / (dist0 + 0.01)).max(0.0).min(1.0);
            
            if rec.distance.abs() < rejection_threshold {
                filtered_results.push(rec.clone());
            }
        }
        
        info!("Static VecDB search took {:.3}s, found {} results", t1.elapsed().as_secs_f64(), filtered_results.len());
        
        Ok(SearchResult {
            query_text: query,
            results: filtered_results,
        })
    }
}

