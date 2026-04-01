use chrono::{DateTime, NaiveDateTime, Utc};
use log::warn;
use tracing::info;
use std::hash::{DefaultHasher, Hasher};
use tokio_rusqlite::Connection;

struct TableInfo {
    name: String,
    creation_time: DateTime<Utc>,
}

pub fn create_emb_table_name(workspace_folders: &Vec<String>) -> String {
    fn _make_hash(msg: String) -> String {
        let mut hasher = DefaultHasher::new();
        hasher.write(msg.as_bytes());
        format!("{:x}", hasher.finish())
    }

    let workspace_folder_list = workspace_folders.join(":");
    let hash = _make_hash(workspace_folder_list);
    format!("emb_{}_persistent", hash)
}

pub fn create_workspace_hash(workspace_folders: &Vec<String>) -> String {
    fn _make_hash(msg: String) -> String {
        let mut hasher = DefaultHasher::new();
        hasher.write(msg.as_bytes());
        format!("{:x}", hasher.finish())
    }

    let workspace_folder_list = workspace_folders.join(":");
    _make_hash(workspace_folder_list)
}

fn parse_table_timestamp(table_name: &str) -> Option<DateTime<Utc>> {
    let parts: Vec<&str> = table_name.split('_').collect();
    if parts.len() >= 3 {
        let date = parts[parts.len() - 2];
        let time = parts[parts.len() - 1];

        if date.len() == 8 && time.len() == 6 {
            let datetime_str = format!(
                "{} {}",
                format!("{}-{}-{}", &date[0..4], &date[4..6], &date[6..8]),
                format!("{}:{}:{}", &time[0..2], &time[2..4], &time[4..6])
            );
            if let Ok(naive_dt) = NaiveDateTime::parse_from_str(&datetime_str, "%Y-%m-%d %H:%M:%S")
            {
                return Some(DateTime::from_naive_utc_and_offset(naive_dt, Utc));
            }
        }
    }
    None
}

pub async fn cleanup_old_emb_tables(conn: &Connection, days: usize, max_count: usize) -> Result<(), String> {
    async fn get_all_emb_tables(
        conn: &Connection,
    ) -> rusqlite::Result<Vec<TableInfo>, String> {
        Ok(conn.call(move |conn| {
            let mut stmt = conn.prepare(
                "SELECT name FROM sqlite_master WHERE type='table' AND name LIKE 'emb_%'",
            )?;
            let tables = stmt.query_map([], |row| {
                let name: String = row.get(0)?;
                Ok(name)
            })?;
            let mut table_infos = Vec::new();
            for table_result in tables {
                let table_name = table_result?;
                if let Some(creation_time) = parse_table_timestamp(&table_name) {
                    table_infos.push(TableInfo {
                        name: table_name,
                        creation_time,
                    });
                }
            }
            Ok(table_infos)
        })
        .await
        .map_err(|e| e.to_string())?)
    }

    let mut tables = get_all_emb_tables(conn).await?;
    tables.sort_by_key(|t| t.creation_time);
    let cutoff = Utc::now() - chrono::Duration::days(days as i64);
    if !tables.is_empty() {
        conn.call(move |conn| {
            for table in tables.iter().take(tables.len().saturating_sub(max_count)) {
                warn!(
                    "dropping emb table (1): {} (created at {})",
                    table.name, table.creation_time
                );
                conn.execute(&format!("DROP TABLE {}", table.name), [])?;
            }
            for table in tables.iter().skip(tables.len().saturating_sub(max_count)) {
                if table.creation_time < cutoff {
                    warn!(
                        "dropping emb table (2): {} (created at {})",
                        table.name, table.creation_time
                    );
                    conn.execute(&format!("DROP TABLE {}", table.name), [])?;
                }
            }
            Ok(())
        })
        .await
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub async fn migrate_to_persistent_table(conn: &Connection, persistent_table_name: &String) -> Result<(), String> {
    let table_name = persistent_table_name.clone();
    
    // First, check if the persistent table already exists and has data
    let persistent_exists = conn.call(move |conn| {
        let mut stmt = conn.prepare("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name = ?")?;
        let count: i32 = stmt.query_row([&table_name], |row| row.get(0))?;
        Ok(count > 0)
    }).await.map_err(|e| e.to_string())?;

    if persistent_exists {
        let table_name = persistent_table_name.clone();
        // Check if persistent table has data
        let persistent_count = conn.call(move |conn| {
            let mut stmt = conn.prepare(&format!("SELECT COUNT(*) FROM {}", table_name))?;
            let count: i32 = stmt.query_row([], |row| row.get(0))?;
            Ok(count)
        }).await.map_err(|e| e.to_string())?;

        if persistent_count > 0 {
            info!("Persistent table {} already exists with {} records, skipping migration", persistent_table_name, persistent_count);
            return Ok(());
        }
    }

    // Find the most recent timestamp-based table
    let latest_table = conn.call(move |conn| {
        let mut stmt = conn.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name LIKE 'emb_%' AND name NOT LIKE '%_persistent'")?;
        let tables: Vec<String> = stmt.query_map([], |row| {
            let name: String = row.get(0)?;
            Ok(name)
        })?.collect::<Result<Vec<_>, _>>()?;
        
        // Find the table with the latest timestamp
        let mut latest_table: Option<String> = None;
        let mut latest_time: Option<DateTime<Utc>> = None;
        
        for table_name in tables {
            if let Some(time) = parse_table_timestamp(&table_name) {
                if latest_time.is_none() || time > latest_time.unwrap() {
                    latest_time = Some(time);
                    latest_table = Some(table_name);
                }
            }
        }
        
        Ok(latest_table)
    }).await.map_err(|e| e.to_string())?;

    if let Some(source_table) = latest_table {
        let table_name = persistent_table_name.clone();
        info!("Migrating data from {} to {}", source_table, persistent_table_name);
        
        // Migrate data from the latest timestamp-based table to the persistent table
        conn.call(move |conn| {
            // Create the persistent table if it doesn't exist (it should be created by the migration)
            conn.execute(&format!(
                "CREATE TABLE IF NOT EXISTS {} AS SELECT * FROM {}",
                table_name, source_table
            ), [])?;
            
            info!("Successfully migrated data from {} to {}", source_table, table_name);
            Ok(())
        }).await.map_err(|e| e.to_string())?;
    } else {
        info!("No timestamp-based tables found to migrate");
    }

    Ok(())
}
