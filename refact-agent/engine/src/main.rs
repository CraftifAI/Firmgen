use std::io::Write;
use std::env;
use std::panic;
use std::path::PathBuf;

use files_correction::canonical_path;
use integrations::running_integrations;
use tokio::task::JoinHandle;
use tracing::{info, Level};
use tracing_appender;
use backtrace;
use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::background_tasks::start_background_tasks;
use crate::lsp::spawn_lsp_task;
use crate::telemetry::{basic_transmit, snippets_transmit};
use crate::yaml_configs::create_configs::yaml_configs_try_create_all;
use crate::yaml_configs::customization_loader::load_customization;
use sqlite_vec::sqlite3_vec_init;
use rusqlite::ffi::sqlite3_auto_extension;

// mods roughly sorted by dependency ↓

mod version;
mod custom_error;
mod nicer_logs;
mod caps;
mod telemetry;
mod global_context;
mod indexing_utils;
mod background_tasks;
mod yaml_configs;
mod json_utils;

mod file_filter;
mod files_in_workspace;
mod files_in_jsonl;
mod files_blocklist;
mod fuzzy_search;
mod files_correction;
mod vecdb;
mod ast;
mod subchat;
mod at_commands;
mod tools;
mod postprocessing;
mod completion_cache;
mod tokens;
mod scratchpad_abstract;
mod scratchpads;

mod fetch_embedding;
mod forward_to_hf_endpoint;
mod forward_to_openai_endpoint;
mod restream;

mod call_validation;
mod dashboard;
mod lsp;
mod http;

mod integrations;
mod privacy;
mod git;
mod cloud;
mod agentic;
mod memories;
mod workflow;
mod progressbar;
// TODO: do we need this?
mod files_correction_cache;
pub mod constants;

fn default_cache_dir(home_dir: &PathBuf) -> PathBuf {
    #[cfg(windows)]
    {
        if let Ok(local_app_data) = env::var("LOCALAPPDATA") {
            return PathBuf::from(local_app_data).join("refact");
        }
        return home_dir.join("AppData").join("Local").join("refact");
    }
    #[cfg(not(windows))]
    {
        home_dir.join(".cache").join("refact")
    }
}

fn default_config_dir(home_dir: &PathBuf) -> PathBuf {
    #[cfg(windows)]
    {
        if let Ok(app_data) = env::var("APPDATA") {
            return PathBuf::from(app_data).join("refact");
        }
        return home_dir.join("AppData").join("Roaming").join("refact");
    }
    #[cfg(not(windows))]
    {
        home_dir.join(".config").join("refact")
    }
}

#[tokio::main]
async fn main() {
    unsafe {
        sqlite3_auto_extension(Some(std::mem::transmute(sqlite3_vec_init as *const ())));

        // Disabling owner validation in Git can theoretically allow code execution, but libgit2 doesn't run
        // executables, so the original risk doesn't apply. Repos in locations like CARGO_HOME would otherwise
        // be blocked, plus several more common cases in Windows. IDEs like VSCode and JetBrains already
        // prompt for trust when adding folders, so we disable the check.
        let _ = git2::opts::set_verify_owner_validation(false);
    }

    let cpu_num = std::thread::available_parallelism().unwrap().get();
    rayon::ThreadPoolBuilder::new().num_threads(cpu_num / 2).build_global().unwrap();
    let home_dir = canonical_path(home::home_dir().ok_or(()).expect("failed to find home dir").to_string_lossy().to_string());
    let cache_dir = default_cache_dir(&home_dir);
    let config_dir = default_config_dir(&home_dir);
    tokio::fs::create_dir_all(&cache_dir).await.expect("failed to create cache dir");
    tokio::fs::create_dir_all(&config_dir).await.expect("failed to create cache dir");
    let (gcx, ask_shutdown_receiver, cmdline) = global_context::create_global_context(cache_dir.clone(), config_dir.clone()).await;
    progressbar::init_progress_persistence(cache_dir.clone()).await;
    crate::tools::esp32_tools::device_port_store::init_device_port_persistence(cache_dir.clone()).await;
    let mut writer_is_stderr = false;
    let (logs_writer, _guard) = if cmdline.logs_stderr {
        writer_is_stderr = true;
        tracing_appender::non_blocking(std::io::stderr())
    } else if !cmdline.logs_to_file.is_empty() {
        tracing_appender::non_blocking(tracing_appender::rolling::RollingFileAppender::new(
            tracing_appender::rolling::Rotation::NEVER,
            std::path::Path::new(&cmdline.logs_to_file).parent().unwrap(),
            std::path::Path::new(&cmdline.logs_to_file).file_name().unwrap()
        ))
    } else {
        let _ = write!(std::io::stderr(), "This rust binary keeps logs as files, rotated daily. Try\ntail -f {}/logs/\nor use --logs-stderr for debugging. Any errors will duplicate here in stderr.\n\n", cache_dir.display());
        tracing_appender::non_blocking(tracing_appender::rolling::RollingFileAppender::builder()
            .rotation(tracing_appender::rolling::Rotation::DAILY)
            .filename_prefix("rustbinary")
            .max_log_files(30)
            .build(cache_dir.join("logs")).unwrap()
        )
    };
    let my_layer = nicer_logs::CustomLayer::new(
        logs_writer.clone(),
        writer_is_stderr,
        if cmdline.verbose { Level::DEBUG } else { Level::INFO },
        Level::ERROR,
        cmdline.lsp_stdin_stdout == 0
    );
    let _tracing = tracing_subscriber::registry()
        .with(my_layer)
        .init();

    panic::set_hook(Box::new(|panic_info| {
        let backtrace = backtrace::Backtrace::new();
        tracing::error!("Panic occurred: {:?}\n{:?}", panic_info, backtrace);
    }));

    match global_context::migrate_to_config_folder(&config_dir, &cache_dir).await {
        Ok(_) => {}
        Err(err) => {
            tracing::error!("failed to migrate config files from .cache to .config, exiting: {:?}", err);
        }
    }

    {
        let build_info = crate::http::routers::info::get_build_info();
        for (k, v) in build_info {
            info!("{:>20} {}", k, v);
        }
        info!("cache dir: {}", cache_dir.display());
        let mut api_key_at: usize = usize::MAX;
        for (arg_n, arg_v) in env::args().enumerate() {
            info!("cmdline[{}]: {:?}", arg_n, if arg_n != api_key_at { arg_v.as_str() } else { "***" } );
            if arg_v == "--api-key" { api_key_at = arg_n + 1; }
        }
    }

    let byok_config_path = yaml_configs_try_create_all(gcx.clone()).await;
    if cmdline.only_create_yaml_configs {
        println!("{}", byok_config_path);
        std::process::exit(0);
    }

    if cmdline.print_customization {  // used in JB
        let mut error_log = Vec::new();
        let cust = load_customization(gcx.clone(), false, &mut error_log).await;
        for e in error_log.iter() {
            eprintln!("{e}");
        }
        println!("{}", serde_json::to_string_pretty(&cust).unwrap());
        std::process::exit(0);
    }

    // Handle static VecDB build mode
    if !cmdline.build_static_vecdb.is_empty() {
        info!("Static VecDB build mode");
        
        if cmdline.output.is_empty() {
            eprintln!("Error: --output is required when using --build-static-vecdb");
            std::process::exit(1);
        }
        
        // Load caps to get embedding model
        let caps = match crate::global_context::try_load_caps_quickly_if_not_present(gcx.clone(), 0).await {
            Ok(caps) => caps,
            Err(e) => {
                eprintln!("Error: Failed to load caps (needed for embedding model): {}", e);
                std::process::exit(1);
            }
        };
        
        if caps.embedding_model.base.name.is_empty() {
            eprintln!("Error: No embedding model available. Check your server configuration.");
            std::process::exit(1);
        }
        
        let config = crate::vecdb::vdb_static_builder::StaticVecDbBuildConfig {
            source_directory: std::path::PathBuf::from(&cmdline.build_static_vecdb),
            output_path: std::path::PathBuf::from(&cmdline.output),
            embedding_model: caps.embedding_model.clone(),
            chunk_size: 1500,
            max_files: 50000,
        };
        
        match crate::vecdb::vdb_static_builder::build_static_vecdb(config, gcx.clone()).await {
            Ok(result) => {
                println!("Static VecDB built successfully!");
                println!("  Files processed: {}", result.files_processed);
                println!("  Chunks created: {}", result.chunks_created);
                println!("  Embeddings generated: {}", result.embeddings_generated);
                if !result.errors.is_empty() {
                    println!("  Errors: {}", result.errors.len());
                }
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("Error building static VecDB: {}", e);
                std::process::exit(1);
            }
        }
    }

    if cmdline.ast {
        let tmp = Some(crate::ast::ast_indexer_thread::ast_service_init(cmdline.ast_permanent.clone(), cmdline.ast_max_files).await);
        let mut gcx_locked = gcx.write().await;
        gcx_locked.ast_service = tmp;
    }

    // Load static VecDBs
    info!("Checking for static VecDBs: {:?}", cmdline.static_vecdb);
    if !cmdline.static_vecdb.is_empty() {
        info!("Loading {} static VecDB(s)", cmdline.static_vecdb.len());
        
        // Load caps to get embedding model for queries
        let caps = crate::global_context::try_load_caps_quickly_if_not_present(gcx.clone(), 0).await;
        
        let static_vec_db = gcx.read().await.static_vec_db.clone();
        let mut static_db = static_vec_db.lock().await;
        
        // Set embedding model if available
        if let Ok(ref caps) = caps {
            if !caps.embedding_model.base.name.is_empty() {
                info!("Setting embedding model for static VecDB: {}", caps.embedding_model.base.name);
                static_db.set_embedding_model(caps.embedding_model.clone());
            } else {
                tracing::error!("Embedding model name is empty!");
            }
        } else {
            tracing::error!("Failed to load caps for static VecDB: {:?}", caps.err());
        }
        
        // Load each static VecDB
        for vecdb_path in &cmdline.static_vecdb {
            info!("Attempting to load static VecDB from: {}", vecdb_path);
            let path = std::path::PathBuf::from(vecdb_path);
            if !path.exists() {
                tracing::error!("Static VecDB file does not exist: {:?}", path);
                continue;
            }
            match static_db.load(path.clone()).await {
                Ok(_) => {
                    info!("Successfully loaded static VecDB: {}", vecdb_path);
                }
                Err(e) => {
                    tracing::error!("Failed to load static VecDB '{}': {}", vecdb_path, e);
                }
            }
        }
        
        if static_db.is_empty() {
            tracing::error!("No static VecDBs were loaded!");
        } else {
            info!("Static VecDBs loaded successfully: {} total embeddings", static_db.total_embeddings());
        }
        drop(static_db);
    } else {
        info!("No --static-vecdb arguments provided");
    }

    // Pre-load board definition if specified
    if !cmdline.board_definition.is_empty() {
        info!("Pre-loading board definition: {}", cmdline.board_definition);
        use crate::tools::esp32_tools::{global_state, board_definition::BoardDefinition};
        
        let mut state = global_state::get_state_mut().await;
        state.session.set_board_id(cmdline.board_definition.clone());
        drop(state);
        
        // Pre-fetch board definition to cache
        let state = global_state::get_state().await;
        let cache = &state.cache;
        let board_url = global_state::board_definition_url(&cmdline.board_definition);
        
        match cache.get_board_definition(&cmdline.board_definition, async {
            let response = reqwest::get(&board_url)
                .await
                .map_err(|e| format!("Failed to fetch board definition: {}", e))?;
            
            if !response.status().is_success() {
                return Err(format!("Server returned error: {}", response.status()));
            }
            
            let board_def: BoardDefinition = response.json()
                .await
                .map_err(|e| format!("Failed to parse board definition: {}", e))?;
            
            Ok(board_def)
        }).await {
            Ok(_) => {
                info!("Successfully pre-loaded board definition: {}", cmdline.board_definition);
            }
            Err(e) => {
                tracing::warn!("Failed to pre-load board definition '{}': {}. It will be fetched on-demand.", cmdline.board_definition, e);
            }
        }
    } else {
        info!("No --board-definition argument provided");
    }

    // Privacy before we do anything else, the default is to block everything
    let _ = crate::privacy::load_privacy_if_needed(gcx.clone()).await;

    // Start or connect to mcp servers
    let _ = running_integrations::load_integrations(gcx.clone(), &["**/mcp_*".to_string()]).await;

    // not really needed, but it's nice to have an error message sooner if there's one
    let _caps = crate::global_context::try_load_caps_quickly_if_not_present(gcx.clone(), 0).await;

    let mut background_tasks = start_background_tasks(gcx.clone(), &config_dir).await;
    // vector db will spontaneously start if the downloaded caps and command line parameters are right

    let should_start_http = cmdline.http_port != 0;
    let should_start_lsp = (cmdline.lsp_port == 0 && cmdline.lsp_stdin_stdout == 1) ||
        (cmdline.lsp_port != 0 && cmdline.lsp_stdin_stdout == 0);

    let mut main_handle: Option<JoinHandle<()>> = None;
    if should_start_http {
        main_handle = http::start_server(gcx.clone(), ask_shutdown_receiver).await;
    }
    if should_start_lsp {
        if main_handle.is_none() {
            // FIXME: this ignores crate::global_context::block_until_signal , important because now we have a database to corrupt
            main_handle = spawn_lsp_task(gcx.clone(), cmdline.clone()).await;
        } else {
            background_tasks.push_back(spawn_lsp_task(gcx.clone(), cmdline.clone()).await.unwrap())
        }
    }
    if main_handle.is_some() {
        let _ = main_handle.unwrap().await;
    }

    background_tasks.abort().await;
    git::checkpoints::abort_init_shadow_repos(gcx.clone()).await;
    integrations::sessions::stop_sessions(gcx.clone()).await;
    info!("saving telemetry without sending, so should be quick");
    basic_transmit::basic_telemetry_compress(gcx.clone()).await;
    info!("bb\n");
}
