use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use dialect_coach_shared::{UsageStats, UserStateMessage, tts::TtsRequest};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};
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
    pub user_state_connections: Arc<Mutex<HashMap<uuid::Uuid, mpsc::UnboundedSender<String>>>>,
}

async fn track_tts_usage(
    state: &TtsState,
    user_id: uuid::Uuid,
    mut usage_stats: UsageStats,
    characters: u64,
) {
    let now = chrono::Utc::now().timestamp();

    tracing::info!(
        user_id = %user_id,
        "Tracking TTS usage: {} characters at timestamp {}",
        characters,
        now
    );

    crate::usage_tracker::add_tts_usage(&mut usage_stats, characters, now, 24);
    let updated_usage = usage_stats.clone();

    match state
        .user_persistence
        .save_usage_stats(user_id, &usage_stats)
        .await
    {
        Ok(_) => {
            tracing::info!(
                user_id = %user_id,
                "Successfully saved TTS usage stats"
            );
            notify_usage_stats_update(state, user_id, updated_usage).await;
        }
        Err(e) => tracing::error!(
            user_id = %user_id,
            "Failed to save TTS usage stats: {}",
            e
        ),
    }
}

async fn handle_tts_success(
    state: &TtsState,
    user_id: uuid::Uuid,
    usage_stats: UsageStats,
    response: dialect_coach_shared::tts::TtsResponse,
    characters: u64,
) -> Json<TtsSynthesizeApiResponse> {
    track_tts_usage(state, user_id, usage_stats, characters).await;
    let audio_base64 = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        &response.audio_data,
    );
    Json(TtsSynthesizeApiResponse {
        audio_base64,
        duration_ms: response.duration_ms,
    })
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
) -> Result<dialect_coach_shared::UsageStats, TtsErrorResponse> {
    let usage_stats = state
        .user_persistence
        .load_usage_stats(user_id)
        .await
        .map_err(|_| TtsErrorResponse {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: "Failed to load usage stats".to_string(),
        })?
        .unwrap_or_default();

    if !state.rate_limiter.elevenlabs_has_quota().await {
        return Err(TtsErrorResponse {
            status: StatusCode::TOO_MANY_REQUESTS,
            message: "ElevenLabs quota exceeded".to_string(),
        });
    }

    if !state
        .rate_limiter
        .can_make_tts_call(&usage_stats, &state.rate_limit_config)
    {
        return Err(TtsErrorResponse {
            status: StatusCode::TOO_MANY_REQUESTS,
            message: "TTS rate limit exceeded".to_string(),
        });
    }

    Ok(usage_stats)
}

/// POST /api/tts/synthesize
/// Synthesize speech from text
pub async fn synthesize_handler(
    State(state): State<TtsState>,
    Json(request): Json<TtsRequest>,
) -> Result<Json<TtsSynthesizeApiResponse>, TtsErrorResponse> {
    info!(
        "TTS synthesize request: {} chars for dialect {}",
        request.text.len(),
        request.dialect.name()
    );

    let characters = request.text.len() as u64;
    let user_id = request.user_id;
    let usage_stats = check_tts_rate_limits(&state, user_id).await?;

    match state.service.synthesize(request).await {
        Ok(response) => {
            Ok(
                handle_tts_success(&state, user_id, usage_stats.clone(), response, characters)
                    .await,
            )
        }
        Err(e) => {
            track_tts_usage(&state, user_id, usage_stats, characters).await;
            Err(TtsErrorResponse::from_tts_error(e))
        }
    }
}

async fn notify_usage_stats_update(state: &TtsState, user_id: uuid::Uuid, usage_stats: UsageStats) {
    let tx = {
        let connections = state.user_state_connections.lock().await;
        connections.get(&user_id).cloned()
    };

    if let Some(tx) = tx {
        let message = UserStateMessage::UsageStatsUpdate(usage_stats);
        match serde_json::to_string(&message) {
            Ok(json) => {
                if let Err(e) = tx.send(json) {
                    tracing::warn!(user_id = %user_id, "Failed to send TTS usage stats update: {:?}", e);
                } else {
                    tracing::info!(user_id = %user_id, "Sent TTS usage stats update to frontend");
                }
            }
            Err(e) => tracing::error!(
                user_id = %user_id,
                "Failed to serialize TTS usage stats update: {}",
                e
            ),
        }
    } else {
        tracing::debug!(
            user_id = %user_id,
            "No connected user state WebSocket to receive TTS usage stats update"
        );
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        persistence::UserPersistence,
        rate_limiter::{config::RateLimitConfig, org_quota::OrgQuotaChecker, service::RateLimiter},
    };
    use anyhow::Result;
    use dialect_coach_shared::models::TTSProviderType;
    use dialect_coach_shared::tts::{TextToSpeechProvider, TtsError, TtsRequest, TtsResponse};
    use dialect_coach_shared::{InviteCode, User, UserState};
    use std::collections::HashMap;
    use tokio::sync::mpsc;
    use uuid::Uuid;

    #[derive(Default)]
    struct TestPersistence {
        saved_usage: Mutex<HashMap<Uuid, UsageStats>>,
    }

    impl TestPersistence {
        async fn usage_for(&self, user_id: Uuid) -> Option<UsageStats> {
            let guard = self.saved_usage.lock().await;
            guard.get(&user_id).cloned()
        }
    }

    #[async_trait::async_trait]
    impl UserPersistence for TestPersistence {
        async fn initialize(&self) -> Result<()> {
            Ok(())
        }

        async fn save(&self, _user_state: &UserState) -> Result<()> {
            Ok(())
        }

        async fn load(&self, _user_id: Uuid) -> Result<Option<UserState>> {
            Ok(None)
        }

        async fn create_user(&self, _user: &User) -> Result<()> {
            Ok(())
        }

        async fn load_user_by_username(&self, _username: &str) -> Result<Option<User>> {
            Ok(None)
        }

        async fn save_usage_stats(&self, user_id: Uuid, usage_stats: &UsageStats) -> Result<()> {
            let mut guard = self.saved_usage.lock().await;
            guard.insert(user_id, usage_stats.clone());
            Ok(())
        }

        async fn load_usage_stats(&self, user_id: Uuid) -> Result<Option<UsageStats>> {
            let guard = self.saved_usage.lock().await;
            Ok(guard.get(&user_id).cloned())
        }

        async fn create_invite_code(&self, _invite_code: &InviteCode) -> Result<()> {
            Ok(())
        }

        async fn load_invite_code(&self, _code: &str) -> Result<Option<InviteCode>> {
            Ok(None)
        }

        async fn save_invite_code(&self, _invite_code: &InviteCode) -> Result<()> {
            Ok(())
        }

        async fn list_invite_codes(&self) -> Result<Vec<InviteCode>> {
            Ok(Vec::new())
        }

        async fn delete_invite_code(&self, _code: &str) -> Result<()> {
            Ok(())
        }
    }

    struct NoopProvider;

    #[async_trait::async_trait]
    impl TextToSpeechProvider for NoopProvider {
        async fn synthesize(
            &self,
            _request: TtsRequest,
        ) -> std::result::Result<TtsResponse, TtsError> {
            Err(TtsError::Unknown("noop".to_string()))
        }

        fn provider_name(&self) -> &'static str {
            "noop"
        }

        fn provider_type(&self) -> TTSProviderType {
            TTSProviderType::Azure
        }
    }

    #[tokio::test]
    async fn test_track_tts_usage_sends_usage_stats_update() {
        let user_id = Uuid::new_v4();
        let (tx, mut rx) = mpsc::unbounded_channel();

        let connections = Arc::new(Mutex::new(HashMap::new()));
        connections.lock().await.insert(user_id, tx);

        let persistence = Arc::new(TestPersistence::default());
        let tts_state = TtsState {
            service: Arc::new(TtsService::new(Arc::new(NoopProvider))),
            user_persistence: persistence.clone(),
            rate_limiter: Arc::new(RateLimiter::new(Arc::new(OrgQuotaChecker::new()))),
            rate_limit_config: Arc::new(RateLimitConfig::default()),
            user_state_connections: connections.clone(),
        };

        track_tts_usage(&tts_state, user_id, UsageStats::default(), 120).await;

        let message = rx.recv().await.expect("expected usage stats update");
        let parsed: UserStateMessage = serde_json::from_str(&message).expect("valid JSON");

        match parsed {
            UserStateMessage::UsageStatsUpdate(stats) => {
                assert_eq!(stats.tts_count(), 1);
                assert_eq!(stats.tts_characters(), 120);
            }
            other => panic!("unexpected message variant: {:?}", other),
        }

        let persisted = persistence
            .usage_for(user_id)
            .await
            .expect("usage stats persisted");
        assert_eq!(persisted.tts_count(), 1);
        assert_eq!(persisted.tts_characters(), 120);
    }
}
