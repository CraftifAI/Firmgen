use axum::response::Result;
use axum::Extension;
use hyper::{Body, Response, StatusCode};
use serde::{Deserialize, Serialize};

use crate::custom_error::ScratchError;
use crate::global_context::SharedGlobalContext;
use crate::vecdb::vdb_structs::VecdbSearch;


#[derive(Serialize, Deserialize, Clone)]
struct VecDBPost {
    query: String,
    top_n: usize,
}

const NO_VECDB: &str = "Vector db is not running. Use --static-vecdb to load pre-built databases, or --vecdb for dynamic indexing.";


pub async fn handle_v1_vecdb_search(
    Extension(gcx): Extension<SharedGlobalContext>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let post = serde_json::from_slice::<VecDBPost>(&body_bytes).map_err(|e| {
        ScratchError::new(StatusCode::BAD_REQUEST, format!("JSON problem: {}", e))
    })?;

    // Try static VecDB first
    let static_vec_db = gcx.read().await.static_vec_db.clone();
    {
        let static_db = static_vec_db.lock().await;
        if !static_db.is_empty() {
            let search_res = static_db.vecdb_search(post.query.to_string(), post.top_n, None).await;
            return match search_res {
                Ok(search_res) => {
                    let json_string = serde_json::to_string_pretty(&search_res).map_err(|e| {
                        ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("JSON serialization problem: {}", e))
                    })?;
                    Ok(Response::builder()
                        .status(StatusCode::OK)
                        .body(Body::from(json_string))
                        .unwrap())
                }
                Err(e) => Err(ScratchError::new(StatusCode::BAD_REQUEST, e))
            };
        }
    }

    // Fall back to dynamic VecDB
    let vec_db = gcx.read().await.vec_db.clone();
    let search_res = match *vec_db.lock().await {
        Some(ref db) => db.vecdb_search(post.query.to_string(), post.top_n, None).await,
        None => {
            return Err(ScratchError::new(
                StatusCode::INTERNAL_SERVER_ERROR, NO_VECDB.to_string(),
            ));
        }
    };

    match search_res {
        Ok(search_res) => {
            let json_string = serde_json::to_string_pretty(&search_res).map_err(|e| {
                ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("JSON serialization problem: {}", e))
            })?;
            Ok(Response::builder()
                .status(StatusCode::OK)
                .body(Body::from(json_string))
                .unwrap())
        }
        Err(e) => {
            Err(ScratchError::new(StatusCode::BAD_REQUEST, e))
        }
    }
}


#[derive(Serialize, Deserialize, Clone)]
struct StaticVecDbStatus {
    pub state: String,
    pub db_count: usize,
    pub total_embeddings: usize,
    pub db_names: Vec<String>,
}

pub async fn handle_v1_vecdb_status(
    Extension(gcx): Extension<SharedGlobalContext>,
    _: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    // Check static VecDB first
    let static_vec_db = gcx.read().await.static_vec_db.clone();
    {
        let static_db = static_vec_db.lock().await;
        let is_empty = static_db.is_empty();
        tracing::info!("VecDB status check: static_db.is_empty() = {}", is_empty);
        if !is_empty {
            let info = static_db.get_all_info();
            let status = StaticVecDbStatus {
                state: "static".to_string(),
                db_count: info.len(),
                total_embeddings: static_db.total_embeddings(),
                db_names: info.iter().map(|i| i.name.clone()).collect(),
            };
            let status_str = serde_json::to_string_pretty(&status).unwrap();
            tracing::info!("Returning static VecDB status: {} DBs, {} embeddings", status.db_count, status.total_embeddings);
            return Ok(Response::builder()
                .status(StatusCode::OK)
                .body(Body::from(status_str))
                .unwrap());
        }
    }

    // Fall back to dynamic VecDB status
    tracing::info!("Checking dynamic VecDB status...");
    let vec_db = gcx.read().await.vec_db.clone();
    let status_str = match crate::vecdb::vdb_highlev::get_status(vec_db).await {
        Ok(Some(status)) => serde_json::to_string_pretty(&status).unwrap(),
        Ok(None) => "{\"success\": 0, \"detail\": \"turned_off\"}".to_string(),
        Err(err) => {
            return Err(ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, err));
        }
    };
    Ok(Response::builder()
        .status(StatusCode::OK)
        .body(Body::from(status_str))
        .unwrap())
}
