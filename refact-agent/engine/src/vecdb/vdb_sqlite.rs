use rusqlite::{OpenFlags, Result};
use std::any::Any;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::path::PathBuf;
use tokio::fs;
use tokio_rusqlite::Connection;
use tracing::info;
use zerocopy::IntoBytes;

use crate::vecdb::vdb_structs::{SimpleTextHashVector, SplitResult, VecdbRecord};


impl Debug for VecDBSqlite {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "VecDBSqlite: {:?}", self.conn.type_id())
    }
}

pub struct VecDBSqlite {
    conn: Connection,
    emb_table_name: String,
    db_path: PathBuf,
}


#[derive(Debug, PartialEq)]
struct DataColumn {
    name: String,
    type_: String,
}

pub async fn get_db_path(cache_dir: &PathBuf, model_name: &String, embedding_size: i32, workspace_hash: &String) -> Result<String, String> {
    let old_path = cache_dir
        .join("refact_vecdb_cache")
        .join(format!("model_{}_esize_{}.sqlite",
                      model_name.replace("/", "_"),
                      embedding_size
        ));
    let new_path = cache_dir
        .join(format!("vecdb_model_{}_esize_{}_ws_{}.sqlite",
                      model_name.replace("/", "_"),
                      embedding_size,
                      workspace_hash
        ));
    if old_path.exists() && !new_path.exists() {
        match fs::rename(&old_path, &new_path).await {
            Ok(_) => {
                Ok(new_path.to_string_lossy().to_string())
            }
            Err(e) => Err(format!("{:?}", e))
        }
    } else {
        Ok(new_path.to_string_lossy().to_string())
    }
}

async fn migrate_202406(conn: &Connection) -> tokio_rusqlite::Result<()> {
    let expected_schema = vec![
        DataColumn { name: "vector".to_string(), type_: "BLOB".to_string() },
        DataColumn { name: "window_text".to_string(), type_: "TEXT".to_string() },
        DataColumn { name: "window_text_hash".to_string(), type_: "TEXT".to_string() },
    ];
    conn.call(move |conn| {
        match conn.execute(&format!("ALTER TABLE data RENAME TO embeddings;"), []) {
            _ => {}
        };
        let mut stmt = conn.prepare(&format!("PRAGMA table_info(embeddings);"))?;
        let schema_iter = stmt.query_map([], |row| {
            Ok(DataColumn {
                name: row.get(1)?,
                type_: row.get(2)?,
            })
        })?;
        let mut schema = Vec::new();
        for column in schema_iter {
            schema.push(column?);
        }
        if schema != expected_schema {
            if schema.len() > 0 {
                info!("vector cache database has invalid schema, recreating the database");
            }
            conn.execute(&format!("DROP TABLE IF EXISTS embeddings"), [])?;
            conn.execute(&format!(
                "CREATE TABLE embeddings (
                    vector BLOB,
                    window_text TEXT NOT NULL,
                    window_text_hash TEXT NOT NULL
                )"), [])?;
            conn.execute(&format!(
                "CREATE INDEX IF NOT EXISTS idx_window_text_hash \
                ON embeddings (window_text_hash)"),
                         [],
            )?;
        }
        Ok(())
    }).await
}


async fn migrate_202501(conn: &Connection, embedding_size: i32, emb_table_name: String) -> tokio_rusqlite::Result<()> {
    conn.call(move |conn| {
        match conn.execute(&format!("ALTER TABLE embeddings RENAME TO embeddings_cache;"), []) {
            _ => {}
        };
        conn.execute(&format!(
            "CREATE VIRTUAL TABLE IF NOT EXISTS {emb_table_name} using vec0(
              embedding float[{embedding_size}] distance_metric=cosine,
              scope TEXT,
              +start_line INTEGER,
              +end_line INTEGER
            );"), [])?;
        Ok(())
    }).await
}

/// Migration to add UNIQUE constraint on window_text_hash to prevent duplicate embeddings
/// This fixes the unbounded cache growth issue where the same text could be cached multiple times
async fn migrate_202512_unique_cache(conn: &Connection) -> tokio_rusqlite::Result<()> {
    conn.call(move |conn| {
        // Check if unique index already exists
        let index_exists: bool = conn.query_row(
            "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='index' AND name='idx_window_text_hash_unique'",
            [],
            |row| row.get(0)
        ).unwrap_or(false);
        
        if index_exists {
            return Ok(());
        }
        
        info!("Running migration: Adding UNIQUE constraint to embeddings_cache");
        
        // First, remove duplicates keeping only one row per window_text_hash
        // We keep the row with the smallest rowid (oldest entry)
        let duplicates_removed: usize = conn.execute(
            "DELETE FROM embeddings_cache WHERE rowid NOT IN (
                SELECT MIN(rowid) FROM embeddings_cache GROUP BY window_text_hash
            )",
            []
        ).unwrap_or(0);
        
        if duplicates_removed > 0 {
            info!("Removed {} duplicate entries from embeddings_cache", duplicates_removed);
        }
        
        // Drop the old non-unique index if it exists
        match conn.execute("DROP INDEX IF EXISTS idx_window_text_hash", []) {
            _ => {}
        };
        
        // Create UNIQUE index to prevent future duplicates
        conn.execute(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_window_text_hash_unique ON embeddings_cache (window_text_hash)",
            []
        )?;
        
        info!("Created UNIQUE index on embeddings_cache.window_text_hash");
        
        Ok(())
    }).await
}

impl VecDBSqlite {
    pub async fn init(cache_dir: &PathBuf, model_name: &String, embedding_size: i32, emb_table_name: &String, workspace_hash: &String) -> Result<VecDBSqlite, String> {
        let db_path_str = get_db_path(cache_dir, model_name, embedding_size, workspace_hash).await?;
        let db_path_buf = PathBuf::from(&db_path_str);
        let conn = match Connection::open_with_flags(
            db_path_str, OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_CREATE
                | OpenFlags::SQLITE_OPEN_NO_MUTEX
                | OpenFlags::SQLITE_OPEN_URI).await {
            Ok(db) => db,
            Err(err) => return Err(format!("{:?}", err))
        };
        conn.call(move |conn| {
            let _: String = conn.query_row("PRAGMA journal_mode=WAL", [], |row| row.get(0))?;
            Ok(())
        }).await.map_err(|e| e.to_string())?;
        migrate_202406(&conn).await.map_err(|e| e.to_string())?;
        migrate_202501(&conn, embedding_size, emb_table_name.clone()).await.map_err(|e| e.to_string())?;
        migrate_202512_unique_cache(&conn).await.map_err(|e| e.to_string())?;
        crate::vecdb::vdb_emb_aux::migrate_to_persistent_table(&conn, &emb_table_name).await?;
        crate::vecdb::vdb_emb_aux::cleanup_old_emb_tables(&conn, 7, 10).await?;

        // Checkpoint WAL after initialization to merge any initial writes
        conn.call(move |conn| {
            // TRUNCATE mode checkpoints and truncates WAL file
            // PRAGMA wal_checkpoint returns results (busy, log, checkpointed), so we need to use query_row
            let _: (i32, i32, i32) = conn.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })?;
            Ok(())
        }).await.map_err(|e| e.to_string())?;

        info!("vecdb initialized");
        Ok(VecDBSqlite { conn, emb_table_name: emb_table_name.clone(), db_path: db_path_buf })
    }

    pub async fn fetch_vectors_from_cache(&mut self, splits: &Vec<SplitResult>) -> Result<Vec<Option<Vec<f32>>>, String> {
        let placeholders: String = splits.iter().map(|_| "?").collect::<Vec<&str>>().join(",");
        let query = format!("SELECT * FROM embeddings_cache WHERE window_text_hash IN ({placeholders})");
        let splits_clone = splits.clone();
        let found_hashes = match self.conn.call(move |connection| {
            let mut statement = connection.prepare(&query)?;
            let params = rusqlite::params_from_iter(splits_clone.iter().map(|x| &x.window_text_hash));
            let x = match statement.query_map(params, |row| {
                let vector_blob: Vec<u8> = row.get(0)?;
                let vector: Vec<f32> = vector_blob
                    .chunks_exact(4)
                    .map(|b| f32::from_ne_bytes(b.try_into().unwrap()))
                    .collect();
                let window_text: String = row.get(1)?;
                let window_text_hash: String = row.get(2)?;
                Ok((window_text_hash, (vector, window_text)))
            }) {
                Ok(mapped_rows) => {
                    Ok(mapped_rows.filter_map(|r| r.ok()).collect::<HashMap<_, _>>())
                }
                Err(e) => {
                    Err(tokio_rusqlite::Error::Rusqlite(e))
                }
            };
            x
        }).await {
            Ok(records) => records,
            Err(err) => return Err(format!("{:?}", err))
        };
        let mut records: Vec<Option<Vec<f32>>> = vec![];
        for split in splits.iter() {
            if let Some(query_data) = found_hashes.get(&split.window_text_hash) {
                records.push(Some(query_data.0.clone()));
            } else {
                records.push(None);
            }
        }
        Ok(records)
    }

    pub async fn cache_add_new_records(&mut self, records: Vec<SimpleTextHashVector>) -> Result<(), String> {
        self.conn.call(|connection| {
            let transaction = connection.transaction()?;
            for record in records {
                let vector_as_bytes: Vec<u8> = match record.vector {
                    Some(vector) => vector.iter()
                        .flat_map(|&num| num.to_ne_bytes())
                        .collect(),
                    None => {
                        tracing::error!("Skipping record with no vector: {:?}", record.window_text_hash);
                        continue;
                    }
                };

                // Use INSERT OR REPLACE to prevent duplicates
                // If window_text_hash already exists (due to UNIQUE constraint), it will be replaced
                match transaction.execute(
                    "INSERT OR REPLACE INTO embeddings_cache (vector, window_text, window_text_hash) VALUES (?1, ?2, ?3)",
                                          rusqlite::params![
                        vector_as_bytes,
                        record.window_text,
                        record.window_text_hash,
                    ],
                ) {
                    Ok(_) => {}
                    Err(err) => {
                        tracing::error!("Error while inserting record to cache: {:?}", err);
                        continue;
                    }
                }
            }
            match transaction.commit() {
                Ok(_) => Ok(()),
                Err(err) => Err(err.into())
            }
        }).await.map_err(|e| e.to_string())
    }

    pub async fn cache_size(&self) -> Result<usize, String> {
        self.conn.call(move |connection| {
            let mut stmt = connection.prepare(
                &format!("SELECT COUNT(1) FROM embeddings_cache")
            )?;
            let count: usize = stmt.query_row([], |row| row.get(0))?;
            Ok(count)
        }).await.map_err(|e| e.to_string())
    }

    pub async fn size(&self) -> Result<usize, String> {
        let emb_table_name = self.emb_table_name.clone();
        self.conn.call(move |connection| {
            let mut stmt = connection.prepare(
                &format!("SELECT COUNT(1) FROM {}", emb_table_name)
            )?;
            let count: usize = stmt.query_row([], |row| row.get(0))?;
            Ok(count)
        }).await.map_err(|e| e.to_string())
    }

    pub async fn vecdb_records_add(&mut self, records: &Vec<VecdbRecord>) -> Result<(), String> {
        use crate::vecdb::vdb_error::with_retry;
        use tokio::time::Duration;
        
        let records_owned = records.clone();
        let emb_table_name = self.emb_table_name.clone();
        
        with_retry(
            || {
                let records_owned = records_owned.clone();
                let emb_table_name = emb_table_name.clone();
                
                self.conn.call(move |connection| {
                    // Use a transaction for better reliability
                    let tx = connection.transaction()?;
                    
                    {
                        let mut stmt = tx.prepare(&format!(
                            "INSERT INTO {}(embedding, scope, start_line, end_line) VALUES (?, ?, ?, ?)", emb_table_name
                        ))?;
                        
                        for item in records_owned.iter() {
                            stmt.execute(rusqlite::params![
                                item.vector.clone().expect("No embedding is provided").as_bytes(),
                                item.file_path.to_string_lossy().to_string(),
                                item.start_line,
                                item.end_line
                            ])?;
                        }
                    }
                    
                    // Commit the transaction
                    tx.commit()?;
                    Ok(())
                })
            },
            3, // Max retries
            Duration::from_millis(100), // Retry delay
            "add vector records"
        ).await
    }

    pub async fn vecdb_search(
        &mut self,
        embedding: &Vec<f32>,
        top_n: usize,
        vecdb_scope_filter_mb: Option<String>,
    ) -> Result<Vec<VecdbRecord>, String> {
        use crate::vecdb::vdb_error::with_retry;
        use tokio::time::Duration;

        let scope_condition = vecdb_scope_filter_mb
            .clone()
            .map(|_| format!("AND scope = ?"))
            .unwrap_or_else(String::new);
        let embedding_owned = embedding.clone();
        let emb_table_name = self.emb_table_name.clone();
        
        // Wrap the database call in retry logic
        with_retry(
            || {
                let embedding_owned = embedding_owned.clone();
                let emb_table_name = emb_table_name.clone();
                let scope_condition = scope_condition.clone();
                let vecdb_scope_filter_mb = vecdb_scope_filter_mb.clone();
                
                self.conn.call(move |connection| {
                    let mut stmt = connection.prepare(&format!(
                        r#"
                        SELECT
                            scope,
                            start_line,
                            end_line,
                            embedding,
                            distance
                        FROM {}
                        WHERE embedding MATCH ?
                            AND k = ?
                            {}
                        ORDER BY distance
                        "#,
                        emb_table_name, scope_condition
                    ))?;

                    let embedding_bytes = embedding_owned.as_bytes();
                    let params = match &vecdb_scope_filter_mb {
                        Some(scope) => rusqlite::params![&embedding_bytes, top_n, scope.clone()],
                        None => rusqlite::params![&embedding_bytes, top_n],
                    };

                    let rows = stmt.query_map(
                        params,
                        |row| {
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
                        },
                    )?;

                    let mut results = Vec::new();
                    for row in rows {
                        results.push(row?);
                    }

                    Ok(results)
                })
            },
            3, // Max retries
            Duration::from_millis(100), // Retry delay
            "vector search"
        ).await
    }

    pub async fn vecdb_records_remove(
        &mut self,
        scopes_to_remove: Vec<String>,
    ) -> Result<(), String> {
        use crate::vecdb::vdb_error::with_retry;
        use tokio::time::Duration;
        
        if scopes_to_remove.is_empty() {
            return Ok(());
        }

        let emb_table_name = self.emb_table_name.clone();
        
        with_retry(
            || {
                let scopes_to_remove = scopes_to_remove.clone();
                let emb_table_name = emb_table_name.clone();
                
                self.conn.call(move |connection| {
                    // Use a transaction for better reliability
                    let tx = connection.transaction()?;
                    
                    // vec0 virtual tables may have issues with DELETE WHERE scope IN (...)
                    // So we delete one scope at a time to ensure it works correctly
                    {
                        let mut stmt = tx.prepare(
                            &format!("DELETE FROM {} WHERE scope = ?", emb_table_name)
                        )?;

                        for scope in scopes_to_remove.iter() {
                            stmt.execute(rusqlite::params![scope])?;
                        }
                    }
                    
                    // Commit the transaction
                    tx.commit()?;
                    Ok(())
                })
            },
            3, // Max retries
            Duration::from_millis(100), // Retry delay
            "remove vector records"
        ).await
    }

    /// Checkpoint the WAL file to merge changes into the main database
    /// This should be called periodically during idle periods to prevent WAL growth
    pub async fn checkpoint_wal(&self) -> Result<(), String> {
        self.conn.call(move |conn| {
            // TRUNCATE mode checkpoints and truncates WAL file
            // PRAGMA wal_checkpoint returns results (busy, log, checkpointed), so we need to use query_row
            // This merges WAL changes into the main database and frees disk space
            let _: (i32, i32, i32) = conn.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })?;
            Ok(())
        }).await.map_err(|e| e.to_string())
    }

    /// Get the database file path
    pub fn get_db_path(&self) -> &PathBuf {
        &self.db_path
    }

    /// Get the actual database file size including WAL and SHM files
    pub async fn file_size(&self) -> Result<u64, String> {
        use std::fs;
        let mut total_size = 0u64;
        
        // Main database file
        if let Ok(metadata) = fs::metadata(&self.db_path) {
            total_size += metadata.len();
        }
        
        // WAL file
        let wal_path = self.db_path.with_extension("sqlite-wal");
        if let Ok(metadata) = fs::metadata(&wal_path) {
            total_size += metadata.len();
        }
        
        // SHM file
        let shm_path = self.db_path.with_extension("sqlite-shm");
        if let Ok(metadata) = fs::metadata(&shm_path) {
            total_size += metadata.len();
        }
        
        Ok(total_size)
    }

    /// Atomically remove old records and add new ones for a set of files
    /// This prevents race conditions where concurrent operations could create duplicates
    pub async fn vecdb_records_replace(&mut self, records: &Vec<VecdbRecord>) -> Result<(), String> {
        use crate::vecdb::vdb_error::with_retry;
        use tokio::time::Duration;
        use std::collections::HashSet;
        
        if records.is_empty() {
            return Ok(());
        }
        
        // Collect unique file paths to remove
        let scopes_to_remove: HashSet<String> = records.iter()
            .map(|r| r.file_path.to_string_lossy().to_string())
            .collect();
        let scopes_vec: Vec<String> = scopes_to_remove.into_iter().collect();
        let num_scopes = scopes_vec.len();
        let num_records = records.len();
        
        let records_owned = records.clone();
        let emb_table_name = self.emb_table_name.clone();
        
        with_retry(
            || {
                let records_owned = records_owned.clone();
                let emb_table_name = emb_table_name.clone();
                let scopes_vec = scopes_vec.clone();
                
                self.conn.call(move |connection| {
                    // Get count before deletion for logging
                    let count_before: i64 = connection.query_row(
                        &format!("SELECT COUNT(*) FROM {}", emb_table_name),
                        [],
                        |row| row.get(0)
                    ).unwrap_or(0);
                    
                    // Use a single transaction for both delete and insert
                    // This ensures atomicity - either both succeed or both fail
                    let tx = connection.transaction()?;
                    
                    let mut deleted_count = 0usize;
                    // First, delete all existing records for these files
                    // Note: vec0 virtual tables may have issues with DELETE WHERE scope IN (...)
                    // So we delete one scope at a time to ensure it works correctly
                    if !scopes_vec.is_empty() {
                        let mut del_stmt = tx.prepare(
                            &format!("DELETE FROM {} WHERE scope = ?", emb_table_name)
                        )?;
                        for scope in scopes_vec.iter() {
                            deleted_count += del_stmt.execute(rusqlite::params![scope])?;
                        }
                    }
                    
                    // Then, insert all new records
                    {
                        let mut ins_stmt = tx.prepare(&format!(
                            "INSERT INTO {}(embedding, scope, start_line, end_line) VALUES (?, ?, ?, ?)", emb_table_name
                        ))?;
                        
                        for item in records_owned.iter() {
                            ins_stmt.execute(rusqlite::params![
                                item.vector.clone().expect("No embedding is provided").as_bytes(),
                                item.file_path.to_string_lossy().to_string(),
                                item.start_line,
                                item.end_line
                            ])?;
                        }
                    }
                    
                    // Commit the transaction - this makes the operation atomic
                    tx.commit()?;
                    
                    // Get count after for logging
                    let count_after: i64 = connection.query_row(
                        &format!("SELECT COUNT(*) FROM {}", emb_table_name),
                        [],
                        |row| row.get(0)
                    ).unwrap_or(0);
                    
                    info!("vecdb_records_replace: deleted {} rows, inserted {} rows, total: {} -> {}", 
                          deleted_count, records_owned.len(), count_before, count_after);
                    
                    Ok(())
                })
            },
            3, // Max retries
            Duration::from_millis(100), // Retry delay
            "replace vector records"
        ).await
    }

    /// Clean up the embeddings cache by removing any duplicate entries
    /// Returns the number of duplicates removed
    pub async fn cleanup_cache_duplicates(&mut self) -> Result<usize, String> {
        self.conn.call(|connection| {
            let removed: usize = connection.execute(
                "DELETE FROM embeddings_cache WHERE rowid NOT IN (
                    SELECT MIN(rowid) FROM embeddings_cache GROUP BY window_text_hash
                )",
                []
            ).map_err(|e| tokio_rusqlite::Error::Rusqlite(e))?;
            
            if removed > 0 {
                info!("Cleaned up {} duplicate entries from embeddings_cache", removed);
            }
            
            Ok(removed)
        }).await.map_err(|e| e.to_string())
    }

    /// Vacuum the database to reclaim disk space after deletions
    /// This is a heavy operation and should only be called during idle periods
    pub async fn vacuum(&self) -> Result<(), String> {
        self.conn.call(|connection| {
            connection.execute("VACUUM", [])?;
            Ok(())
        }).await.map_err(|e| e.to_string())
    }

    /// Get statistics about the database
    pub async fn get_stats(&self) -> Result<VecDbStats, String> {
        let emb_table_name = self.emb_table_name.clone();
        self.conn.call(move |connection| {
            let cache_count: usize = connection.query_row(
                "SELECT COUNT(*) FROM embeddings_cache",
                [],
                |row| row.get(0)
            ).unwrap_or(0);
            
            let cache_unique_count: usize = connection.query_row(
                "SELECT COUNT(DISTINCT window_text_hash) FROM embeddings_cache",
                [],
                |row| row.get(0)
            ).unwrap_or(0);
            
            let vectors_count: usize = connection.query_row(
                &format!("SELECT COUNT(*) FROM {}", emb_table_name),
                [],
                |row| row.get(0)
            ).unwrap_or(0);
            
            let unique_scopes: usize = connection.query_row(
                &format!("SELECT COUNT(DISTINCT scope) FROM {}", emb_table_name),
                [],
                |row| row.get(0)
            ).unwrap_or(0);
            
            // Check for duplicate (scope, start_line, end_line) combinations
            let duplicate_vectors: usize = connection.query_row(
                &format!("SELECT COUNT(*) - COUNT(DISTINCT scope || '|' || start_line || '|' || end_line) FROM {}", emb_table_name),
                [],
                |row| row.get(0)
            ).unwrap_or(0);
            
            Ok(VecDbStats {
                cache_entries: cache_count,
                cache_unique_hashes: cache_unique_count,
                cache_duplicates: cache_count.saturating_sub(cache_unique_count),
                vector_entries: vectors_count,
                unique_files: unique_scopes,
                duplicate_vectors,
            })
        }).await.map_err(|e| e.to_string())
    }

    /// Diagnose and log detailed database state for debugging
    pub async fn diagnose(&self) -> Result<String, String> {
        let emb_table_name = self.emb_table_name.clone();
        self.conn.call(move |connection| {
            let mut report = String::new();
            
            // Get all tables
            let mut stmt = connection.prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")?;
            let tables: Vec<String> = stmt.query_map([], |row| row.get(0))?
                .filter_map(|r| r.ok())
                .collect();
            report.push_str(&format!("Tables: {:?}\n", tables));
            
            // Cache stats
            let cache_count: i64 = connection.query_row(
                "SELECT COUNT(*) FROM embeddings_cache", [], |row| row.get(0)
            ).unwrap_or(0);
            let cache_unique: i64 = connection.query_row(
                "SELECT COUNT(DISTINCT window_text_hash) FROM embeddings_cache", [], |row| row.get(0)
            ).unwrap_or(0);
            report.push_str(&format!("Cache: {} total, {} unique hashes, {} duplicates\n", 
                cache_count, cache_unique, cache_count - cache_unique));
            
            // Vector table stats
            let vec_count: i64 = connection.query_row(
                &format!("SELECT COUNT(*) FROM {}", emb_table_name), [], |row| row.get(0)
            ).unwrap_or(0);
            let vec_unique_scopes: i64 = connection.query_row(
                &format!("SELECT COUNT(DISTINCT scope) FROM {}", emb_table_name), [], |row| row.get(0)
            ).unwrap_or(0);
            let vec_unique_combos: i64 = connection.query_row(
                &format!("SELECT COUNT(DISTINCT scope || '|' || start_line || '|' || end_line) FROM {}", emb_table_name), 
                [], |row| row.get(0)
            ).unwrap_or(0);
            report.push_str(&format!("Vectors ({}): {} total, {} unique scopes, {} unique (scope,line) combos, {} potential duplicates\n", 
                emb_table_name, vec_count, vec_unique_scopes, vec_unique_combos, vec_count - vec_unique_combos));
            
            // Show top files by record count
            let mut stmt = connection.prepare(&format!(
                "SELECT scope, COUNT(*) as cnt FROM {} GROUP BY scope ORDER BY cnt DESC LIMIT 10", 
                emb_table_name
            ))?;
            let top_files: Vec<(String, i64)> = stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })?.filter_map(|r| r.ok()).collect();
            report.push_str("Top 10 files by record count:\n");
            for (scope, count) in top_files {
                let short_scope = if scope.len() > 60 { &scope[scope.len()-60..] } else { &scope };
                report.push_str(&format!("  {} records: ...{}\n", count, short_scope));
            }
            
            // Check for exact duplicates (same scope, start_line, end_line)
            let mut stmt = connection.prepare(&format!(
                "SELECT scope, start_line, end_line, COUNT(*) as cnt FROM {} GROUP BY scope, start_line, end_line HAVING cnt > 1 LIMIT 10",
                emb_table_name
            ))?;
            let duplicates: Vec<(String, i64, i64, i64)> = stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?, row.get::<_, i64>(2)?, row.get::<_, i64>(3)?))
            })?.filter_map(|r| r.ok()).collect();
            if !duplicates.is_empty() {
                report.push_str("Duplicate records found (same scope, start_line, end_line):\n");
                for (scope, start, end, count) in duplicates {
                    let short_scope = if scope.len() > 50 { &scope[scope.len()-50..] } else { &scope };
                    report.push_str(&format!("  {} copies: ...{} lines {}-{}\n", count, short_scope, start, end));
                }
            } else {
                report.push_str("No exact duplicate records found.\n");
            }
            
            Ok(report)
        }).await.map_err(|e| e.to_string())
    }

    /// Remove duplicate vector records (keeping only one per scope+start_line+end_line)
    pub async fn cleanup_vector_duplicates(&mut self) -> Result<usize, String> {
        let emb_table_name = self.emb_table_name.clone();
        self.conn.call(move |connection| {
            // For vec0 tables, we need to delete by rowid
            // First find all rowids that are duplicates (not the first occurrence)
            let delete_sql = format!(
                "DELETE FROM {} WHERE rowid NOT IN (
                    SELECT MIN(rowid) FROM {} GROUP BY scope, start_line, end_line
                )", emb_table_name, emb_table_name
            );
            
            let removed = connection.execute(&delete_sql, [])
                .map_err(|e| tokio_rusqlite::Error::Rusqlite(e))?;
            
            if removed > 0 {
                info!("Cleaned up {} duplicate vector entries", removed);
            }
            
            Ok(removed)
        }).await.map_err(|e| e.to_string())
    }
}

/// Statistics about the vector database
#[derive(Debug, Clone)]
pub struct VecDbStats {
    pub cache_entries: usize,
    pub cache_unique_hashes: usize,
    pub cache_duplicates: usize,
    pub vector_entries: usize,
    pub unique_files: usize,
    pub duplicate_vectors: usize,
}
