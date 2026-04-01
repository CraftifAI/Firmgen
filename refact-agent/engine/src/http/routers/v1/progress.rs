//! GET /v1/progress - Returns ESP32 tool progress for a chat (for GUI progress bar).

use axum::extract::Query;
use axum::Json;
use serde::Deserialize;

use crate::progressbar;

#[derive(Deserialize)]
pub struct ProgressQuery {
    pub chat_id: String,
}

#[derive(serde::Serialize)]
pub struct ApiResponse<T: serde::Serialize> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
}

/// GET /v1/progress?chat_id=... - Get progress for a chat.
pub async fn handle_v1_progress(
    Query(query): Query<ProgressQuery>,
) -> Json<ApiResponse<progressbar::ProgressSessionDto>> {
    if query.chat_id.is_empty() {
        return Json(ApiResponse {
            success: false,
            data: None,
            error: Some("chat_id is required".to_string()),
        });
    }

    match progressbar::get_progress(&query.chat_id).await {
        Some(session) => Json(ApiResponse {
            success: true,
            data: Some(session),
            error: None,
        }),
        None => Json(ApiResponse {
            success: true,
            data: None,
            error: None,
        }),
    }
}
