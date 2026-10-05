use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use dialect_coach_shared::tts::{TtsError, TtsRequest, TtsResponse};
use dialect_coach_shared::{SpeechAudio, UsageStats};
use serde::Serialize;
use std::sync::Arc;
use tracing::{error, info};

use crate::persistence::UserPersistence;
use crate::rate_limiter::service::RateLimiterService;
use crate::state::Connections;
use crate::tts_service::TtsService;

/// Axum state for TTS handlers
#[derive(Clone)]
pub struct TtsState {
    pub service: Arc<TtsService>,
    pub user_persistence: Arc<dyn UserPersistence>,
    pub rate_limiter: Arc<crate::rate_limiter::service::RateLimiter>,
    pub rate_limit_config: Arc<crate::rate_limiter::config::RateLimitConfig>,
    pub connections: Connections,
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
            crate::websocket::registry::push_usage(&state.connections, user_id, updated_usage)
                .await;
        }
        Err(e) => tracing::error!(
            user_id = %user_id,
            "Failed to save TTS usage stats: {}",
            e
        ),
    }
}

fn speech_audio(response: TtsResponse) -> SpeechAudio {
    let audio_base64 = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        &response.audio_data,
    );
    SpeechAudio {
        audio_base64,
        duration_ms: response.duration_ms,
    }
}

async fn check_tts_rate_limits(
    state: &TtsState,
    user_id: uuid::Uuid,
) -> Result<UsageStats, String> {
    let usage_stats = state
        .user_persistence
        .load_usage_stats(user_id)
        .await
        .map_err(|_| "Failed to load usage stats".to_string())?
        .unwrap_or_default();
    if !state.rate_limiter.elevenlabs_has_quota().await {
        return Err("ElevenLabs quota exceeded".to_string());
    }
    let limits = &state.rate_limit_config;
    if !state.rate_limiter.can_make_tts_call(&usage_stats, limits) {
        return Err("TTS rate limit exceeded".to_string());
    }
    Ok(usage_stats)
}

/// Synthesize speech for a user; every attempt that passes the rate limits counts toward usage.
pub async fn synthesize(state: &TtsState, request: TtsRequest) -> Result<SpeechAudio, String> {
    info!(
        "TTS synthesize request: {} chars for dialect {}",
        request.text.len(),
        request.dialect.name()
    );
    let characters = request.text.len() as u64;
    let user_id = request.user_id;
    let usage_stats = check_tts_rate_limits(state, user_id).await?;
    let result = state.service.synthesize(request).await;
    track_tts_usage(state, user_id, usage_stats, characters).await;
    result.map(speech_audio).map_err(tts_error_message)
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

fn tts_error_message(error: TtsError) -> String {
    let message = match error {
        TtsError::NetworkError(msg) => format!("Network error: {}", msg),
        TtsError::InvalidRequest(msg) => format!("Invalid request: {}", msg),
        TtsError::UnsupportedFormat(msg) => format!("Unsupported format: {}", msg),
        TtsError::VoiceNotFound(msg) => format!("Voice not found: {}", msg),
        TtsError::QuotaExceeded => "API quota exceeded".to_string(),
        TtsError::AuthenticationFailed => "Authentication failed".to_string(),
        TtsError::Unknown(msg) => format!("Unknown error: {}", msg),
    };
    error!("TTS synthesis failed: {}", message);
    message
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
    use dialect_coach_shared::ServerMessage;
    use dialect_coach_shared::models::TTSProviderType;
    use dialect_coach_shared::tts::TextToSpeechProvider;
    use dialect_coach_shared::{Dialect, InviteCode, User, UserState};
    use std::collections::HashMap;
    use tokio::sync::{Mutex, mpsc};
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

        async fn create_user(&self, _user: &User, _password_hash: String) -> Result<()> {
            Ok(())
        }

        async fn load_user_by_username(&self, _username: &str) -> Result<Option<(User, String)>> {
            Ok(None)
        }

        async fn load_user_by_id(&self, _user_id: Uuid) -> Result<Option<User>> {
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

    fn tts_state(persistence: Arc<TestPersistence>, connections: Connections) -> TtsState {
        let app_config = dialect_coach_shared::config::load_config(&format!(
            "{}/../config.yaml",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();
        TtsState {
            service: Arc::new(TtsService::new(Arc::new(NoopProvider))),
            user_persistence: persistence,
            rate_limiter: Arc::new(RateLimiter::new(Arc::new(OrgQuotaChecker::new()))),
            rate_limit_config: Arc::new(RateLimitConfig::from_yaml_config(&app_config.rate_limits)),
            connections,
        }
    }

    #[tokio::test]
    async fn test_synthesize_reports_provider_error_and_counts_usage() {
        let persistence = Arc::new(TestPersistence::default());
        let state = tts_state(persistence.clone(), Arc::new(Mutex::new(HashMap::new())));
        let user_id = Uuid::new_v4();
        let request = TtsRequest::new(user_id, "Hola".to_string(), Dialect::SpanishArgentinian);

        let result = synthesize(&state, request).await;

        assert_eq!(result.unwrap_err(), "Unknown error: noop");
        let persisted = persistence.usage_for(user_id).await.expect("usage saved");
        assert_eq!(persisted.tts_characters(), 4);
    }

    #[test]
    fn test_speech_audio_encodes_audio_as_base64() {
        let audio = speech_audio(TtsResponse {
            audio_data: b"ID3".to_vec(),
            audio_format: dialect_coach_shared::tts::AudioFormat::Mp3,
            duration_ms: 1200,
            cache_key: "k".to_string(),
        });
        assert_eq!(audio.audio_base64, "SUQz");
        assert_eq!(audio.duration_ms, 1200);
    }

    #[test]
    fn test_tts_error_message_names_the_failure() {
        assert_eq!(
            tts_error_message(TtsError::QuotaExceeded),
            "API quota exceeded"
        );
        assert_eq!(
            tts_error_message(TtsError::VoiceNotFound("v1".to_string())),
            "Voice not found: v1"
        );
    }

    #[tokio::test]
    async fn test_track_tts_usage_sends_usage_stats_update() {
        let user_id = Uuid::new_v4();
        let (tx, mut rx) = mpsc::unbounded_channel();

        let connections: Connections = Arc::new(Mutex::new(HashMap::new()));
        connections.lock().await.insert(user_id, vec![tx]);

        let persistence = Arc::new(TestPersistence::default());
        let tts_state = tts_state(persistence.clone(), connections.clone());

        track_tts_usage(&tts_state, user_id, UsageStats::default(), 120).await;

        let message = rx.recv().await.expect("expected usage stats update");
        let parsed: ServerMessage = serde_json::from_str(&message).expect("valid JSON");

        match parsed {
            ServerMessage::UsageStats(stats) => {
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
