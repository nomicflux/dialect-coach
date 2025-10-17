use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use dialect_coach_shared::tts::{TtsRequest, TtsResponse, VoiceInfo};
use serde::Serialize;
use std::sync::Arc;
use tracing::{error, info};

use crate::tts_service::TtsService;

/// Axum state for TTS handlers
#[derive(Clone)]
pub struct TtsState {
    pub service: Arc<TtsService>,
}

/// POST /api/tts/synthesize
/// Synthesize speech from text
pub async fn synthesize_handler(
    State(state): State<TtsState>,
    Json(request): Json<TtsRequest>,
) -> Result<Json<TtsResponse>, TtsErrorResponse> {
    info!(
        "TTS synthesize request: {} chars in {}",
        request.text.len(),
        request.language_code
    );

    let response = state
        .service
        .synthesize(request)
        .await
        .map_err(TtsErrorResponse::from_tts_error)?;

    Ok(Json(response))
}

/// GET /api/tts/voices?language_code=es-MX
/// Get available voices for a language
pub async fn voices_handler(
    State(state): State<TtsState>,
    axum::extract::Query(params): axum::extract::Query<VoicesQuery>,
) -> Result<Json<VoicesResponse>, TtsErrorResponse> {
    info!("TTS voices request for: {}", params.language_code);

    let voices = state
        .service
        .get_voices(&params.language_code)
        .await
        .map_err(TtsErrorResponse::from_tts_error)?;

    Ok(Json(VoicesResponse { voices }))
}

/// GET /api/tts/status
/// Get TTS service status and cache statistics
pub async fn status_handler(State(state): State<TtsState>) -> Json<TtsStatusResponse> {
    let (cache_size, cache_capacity) = state.service.cache_stats().await;

    Json(TtsStatusResponse {
        provider: state.service.provider_name().to_string(),
        cache_size,
        cache_capacity,
        status: "operational".to_string(),
    })
}

/// DELETE /api/tts/cache
/// Clear the TTS cache
pub async fn clear_cache_handler(State(state): State<TtsState>) -> Json<ClearCacheResponse> {
    info!("Clearing TTS cache");
    state.service.clear_cache().await;

    Json(ClearCacheResponse {
        message: "Cache cleared successfully".to_string(),
    })
}

// Request/Response types

#[derive(Debug, serde::Deserialize)]
pub struct VoicesQuery {
    pub language_code: String,
}

#[derive(Debug, Serialize)]
pub struct VoicesResponse {
    pub voices: Vec<VoiceInfo>,
}

#[derive(Debug, Serialize)]
pub struct TtsStatusResponse {
    pub provider: String,
    pub cache_size: usize,
    pub cache_capacity: usize,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct ClearCacheResponse {
    pub message: String,
}

// Error handling

#[derive(Debug)]
pub struct TtsErrorResponse {
    status: StatusCode,
    message: String,
}

impl TtsErrorResponse {
    fn from_tts_error(error: dialect_coach_shared::tts::TtsError) -> Self {
        use dialect_coach_shared::tts::TtsError;

        let (status, message) = match error {
            TtsError::NetworkError(msg) => {
                error!("TTS network error: {}", msg);
                (StatusCode::BAD_GATEWAY, format!("Network error: {}", msg))
            }
            TtsError::InvalidRequest(msg) => {
                error!("TTS invalid request: {}", msg);
                (StatusCode::BAD_REQUEST, format!("Invalid request: {}", msg))
            }
            TtsError::UnsupportedFormat(msg) => {
                error!("TTS unsupported format: {}", msg);
                (
                    StatusCode::BAD_REQUEST,
                    format!("Unsupported format: {}", msg),
                )
            }
            TtsError::VoiceNotFound(msg) => {
                error!("TTS voice not found: {}", msg);
                (StatusCode::NOT_FOUND, format!("Voice not found: {}", msg))
            }
            TtsError::QuotaExceeded => {
                error!("TTS quota exceeded");
                (
                    StatusCode::TOO_MANY_REQUESTS,
                    "API quota exceeded".to_string(),
                )
            }
            TtsError::AuthenticationFailed => {
                error!("TTS authentication failed");
                (
                    StatusCode::UNAUTHORIZED,
                    "Authentication failed".to_string(),
                )
            }
            TtsError::Unknown(msg) => {
                error!("TTS unknown error: {}", msg);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Unknown error: {}", msg),
                )
            }
        };

        Self { status, message }
    }
}

impl IntoResponse for TtsErrorResponse {
    fn into_response(self) -> Response {
        #[derive(Serialize)]
        struct ErrorBody {
            error: String,
        }

        let body = Json(ErrorBody {
            error: self.message,
        });

        (self.status, body).into_response()
    }
}

/// Fallback handler for when TTS service is not available
pub async fn tts_unavailable_handler() -> impl IntoResponse {
    #[derive(Serialize)]
    struct ErrorBody {
        error: String,
    }

    let body = Json(ErrorBody {
        error: "TTS service is not available. Please check server logs.".to_string(),
    });

    (StatusCode::SERVICE_UNAVAILABLE, body)
}
