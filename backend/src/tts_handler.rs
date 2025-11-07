use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use dialect_coach_shared::tts::TtsRequest;
use serde::Serialize;
use std::sync::Arc;
use tracing::{error, info};

use crate::persistence::UserPersistence;
use crate::rate_limiter::service::RateLimiterService;
use crate::tts_service::TtsService;

/// Axum state for TTS handlers
#[derive(Clone)]
pub struct TtsState {
    pub service: Arc<TtsService>,
    pub user_persistence: Arc<dyn UserPersistence>,
    pub rate_limiter: Arc<crate::rate_limiter::service::RateLimiter>,
    pub rate_limit_config: Arc<crate::rate_limiter::config::RateLimitConfig>,
}

async fn track_tts_usage(state: &TtsState, user_id: uuid::Uuid, characters: u64) {
    let now = chrono::Utc::now().timestamp();

    let mut user_state = match state.user_persistence.load(user_id).await {
        Ok(Some(s)) => s,
        Ok(None) | Err(_) => return,
    };

    crate::usage_tracker::add_tts_usage(&mut user_state.usage_stats, characters, now, 24);

    let _ = state.user_persistence.save(&user_state).await;
}

/// Response format for TTS API (frontend expects base64)
#[derive(Debug, Serialize)]
pub struct TtsSynthesizeApiResponse {
    pub audio_base64: String,
    pub duration_ms: u32,
}

async fn check_tts_rate_limits(
    state: &TtsState,
    user_id: uuid::Uuid,
) -> Result<(), TtsErrorResponse> {
    let user_state = state
        .user_persistence
        .load(user_id)
        .await
        .map_err(|_| TtsErrorResponse {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: "Failed to load user state".to_string(),
        })?
        .ok_or_else(|| TtsErrorResponse {
            status: StatusCode::NOT_FOUND,
            message: "User not found".to_string(),
        })?;

    if !state.rate_limiter.elevenlabs_has_quota().await {
        return Err(TtsErrorResponse {
            status: StatusCode::TOO_MANY_REQUESTS,
            message: "ElevenLabs quota exceeded".to_string(),
        });
    }

    if !state
        .rate_limiter
        .can_make_tts_call(&user_state.usage_stats, &state.rate_limit_config)
    {
        return Err(TtsErrorResponse {
            status: StatusCode::TOO_MANY_REQUESTS,
            message: "TTS rate limit exceeded".to_string(),
        });
    }

    Ok(())
}

/// POST /api/tts/synthesize
/// Synthesize speech from text
pub async fn synthesize_handler(
    State(state): State<TtsState>,
    Json(request): Json<TtsRequest>,
) -> Result<Json<TtsSynthesizeApiResponse>, TtsErrorResponse> {
    info!(
        "TTS synthesize request: {} chars in {}",
        request.text.len(),
        request.language_code
    );

    let characters = request.text.len() as u64;
    let user_id = request.user_id;

    check_tts_rate_limits(&state, user_id).await?;

    let response = state
        .service
        .synthesize(request)
        .await
        .map_err(TtsErrorResponse::from_tts_error)?;

    track_tts_usage(&state, user_id, characters).await;

    // Convert binary audio data to base64 for frontend
    let audio_base64 = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        &response.audio_data,
    );

    let api_response = TtsSynthesizeApiResponse {
        audio_base64,
        duration_ms: response.duration_ms,
    };

    Ok(Json(api_response))
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
