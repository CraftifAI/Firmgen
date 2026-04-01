use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use super::board_definition::BoardDefinition;
use serde_json;

/// Intelligent caching layer for ESP32 tools
pub struct ESP32Cache {
    // Hardware detection cache (expensive, cache aggressively)
    hardware_cache: RwLock<TimedCache<Vec<super::session_state::DetectedDevice>>>,
    hardware_cache_ttl: Duration,
    
    // Config cache (load once per session)
    config: RwLock<Option<super::config::ESP32Config>>,
    
    // Board definition cache (per session + file cache)
    board_definitions: RwLock<HashMap<String, BoardDefinition>>,
    board_cache_dir: PathBuf,
    
    // File hashes (for incremental operations)
    file_hashes: RwLock<HashMap<PathBuf, String>>,
    
    // Build artifacts (know what's already built)
    build_cache: RwLock<HashMap<BuildKey, BuildArtifact>>,
}

#[derive(Clone)]
struct TimedCache<T> {
    value: Option<T>,
    cached_at: Option<Instant>,
    ttl: Duration,
}

impl<T: Clone> TimedCache<T> {
    fn new(ttl: Duration) -> Self {
        Self {
            value: None,
            cached_at: None,
            ttl,
        }
    }

    fn get(&self) -> Option<&T> {
        match (&self.value, self.cached_at) {
            (Some(v), Some(t)) if t.elapsed() < self.ttl => Some(v),
            _ => None,
        }
    }

    fn set(&mut self, value: T) {
        self.value = Some(value);
        self.cached_at = Some(Instant::now());
    }

    fn invalidate(&mut self) {
        self.value = None;
        self.cached_at = None;
    }
}

#[derive(Clone, Hash, Eq, PartialEq)]
struct BuildKey {
    project_path: PathBuf,
    config: String,
    source_hash: String,
}

#[derive(Clone)]
struct BuildArtifact {
    output_path: PathBuf,
    built_at: Instant,
    build_config: String,
    binary_size: usize,
}

impl ESP32Cache {
    pub fn new() -> Self {
        let board_cache_dir = Self::default_board_cache_dir();
        // Best-effort create
        let _ = std::fs::create_dir_all(&board_cache_dir);

        Self {
            hardware_cache: RwLock::new(TimedCache::new(Duration::from_secs(30))),
            hardware_cache_ttl: Duration::from_secs(30),
            config: RwLock::new(None),
            board_definitions: RwLock::new(HashMap::new()),
            board_cache_dir,
            file_hashes: RwLock::new(HashMap::new()),
            build_cache: RwLock::new(HashMap::new()),
        }
    }

    /// Get config (cached, loaded once)
    pub async fn get_config(&self, api_url: &str) -> Result<super::config::ESP32Config, String> {
        // Check cache first
        {
            let cache = self.config.read().await;
            if let Some(config) = &*cache {
                return Ok(config.clone());
            }
        }
        
        // Load and cache
        let config = super::config::ESP32Config::load_from_api(api_url).await?;
        {
            let mut cache = self.config.write().await;
            *cache = Some(config.clone());
        }
        Ok(config)
    }

    /// Get board definition with in-memory + file cache and provided fetcher
    /// Priority: 1) Memory cache, 2) File cache, 3) Local folder, 4) Server fetch, 5) Stale cache
    pub async fn get_board_definition<F>(&self, board_id: &str, fetch_fn: F) -> Result<BoardDefinition, String>
    where
        F: std::future::Future<Output = Result<BoardDefinition, String>>,
    {
        // 1) Check in-memory cache
        {
            let cache = self.board_definitions.read().await;
            if let Some(def) = cache.get(board_id) {
                return Ok(def.clone());
            }
        }

        // 2) Check file cache (~/.cache/refact/board_definitions/)
        let file_path = self.board_cache_dir.join(format!("{}.json", board_id));
        if let Ok(content) = tokio::fs::read_to_string(&file_path).await {
            if let Ok(def) = serde_json::from_str::<BoardDefinition>(&content) {
                let mut cache = self.board_definitions.write().await;
                cache.insert(board_id.to_string(), def.clone());
                return Ok(def);
            }
        }

        // 3) Check local board_definitions folder (workspace or env var)
        if let Some(local_path) = Self::local_board_definitions_path(board_id) {
            if let Ok(content) = tokio::fs::read_to_string(&local_path).await {
                if let Ok(def) = serde_json::from_str::<BoardDefinition>(&content) {
                    // Save to file cache for faster future access
                    if let Ok(json) = serde_json::to_string_pretty(&def) {
                        let _ = tokio::fs::write(&file_path, json).await;
                    }
                    // Save to memory
                    {
                        let mut cache = self.board_definitions.write().await;
                        cache.insert(board_id.to_string(), def.clone());
                    }
                    return Ok(def);
                }
            }
        }

        // 4) Fetch from server (via fetch_fn)
        match fetch_fn.await {
            Ok(def) => {
                // Save to file cache (best-effort)
                if let Ok(json) = serde_json::to_string_pretty(&def) {
                    let _ = tokio::fs::write(&file_path, json).await;
                }
                // Save to memory
                {
                    let mut cache = self.board_definitions.write().await;
                    cache.insert(board_id.to_string(), def.clone());
                }
                Ok(def)
            }
            Err(e) => {
                // 5) Try stale file cache as last resort
                if let Ok(content) = tokio::fs::read_to_string(&file_path).await {
                    if let Ok(def) = serde_json::from_str::<BoardDefinition>(&content) {
                        tracing::warn!("Using stale board definition from cache (failed to fetch {}): {}", board_id, e);
                        return Ok(def);
                    }
                }
                Err(format!("Failed to fetch board definition and no cache available: {}", e))
            }
        }
    }

    /// Get path to local board_definitions folder
    fn local_board_definitions_path(board_id: &str) -> Option<PathBuf> {
        // Check env var first
        if let Ok(dir) = std::env::var("REFACT_BOARD_DEFINITIONS_DIR") {
            let path = PathBuf::from(dir).join(format!("{}.json", board_id));
            if path.exists() {
                return Some(path);
            }
        }
        
        // Check workspace-relative path
        if let Ok(cwd) = std::env::current_dir() {
            let path = cwd.join("board_definitions").join(format!("{}.json", board_id));
            if path.exists() {
                return Some(path);
            }
        }
        
        None
    }

    fn default_board_cache_dir() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        PathBuf::from(home).join(".cache").join("refact").join("board_definitions")
    }

    /// Get hardware (cached with TTL)
    pub async fn get_hardware<F>(&self, force_refresh: bool, detect_fn: F) -> Result<Vec<super::session_state::DetectedDevice>, String>
    where
        F: std::future::Future<Output = Result<Vec<super::session_state::DetectedDevice>, String>>,
    {
        if !force_refresh {
            let cache = self.hardware_cache.read().await;
            if let Some(devices) = cache.get() {
                return Ok(devices.clone());
            }
        }
        
        // Expensive detection
        let devices = detect_fn.await?;
        {
            let mut cache = self.hardware_cache.write().await;
            cache.set(devices.clone());
        }
        Ok(devices)
    }

    /// Invalidate on specific events
    pub async fn invalidate_on(&self, event: CacheInvalidationEvent) {
        match event {
            CacheInvalidationEvent::FileModified(path) => {
                let mut hashes = self.file_hashes.write().await;
                hashes.remove(&path);
                
                let mut builds = self.build_cache.write().await;
                builds.retain(|k, _| k.project_path != path);
            },
            CacheInvalidationEvent::HardwareChange => {
                let mut cache = self.hardware_cache.write().await;
                cache.invalidate();
            },
            CacheInvalidationEvent::ConfigChange => {
                let mut cache = self.config.write().await;
                *cache = None;
            },
            CacheInvalidationEvent::SessionEnd => {
                *self.config.write().await = None;
                self.hardware_cache.write().await.invalidate();
                self.file_hashes.write().await.clear();
                self.build_cache.write().await.clear();
            },
        }
    }
}

pub enum CacheInvalidationEvent {
    FileModified(PathBuf),
    HardwareChange,
    ConfigChange,
    SessionEnd,
}

impl Default for ESP32Cache {
    fn default() -> Self {
        Self::new()
    }
}

