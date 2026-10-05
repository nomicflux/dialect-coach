use crate::{
    admin, admin_invites, agent_service, auth_service, embedding_service, enrichment_handler,
    grammar_handler, persistence, planning_handler, qdrant_service, rate_limiter, state::AppState,
    translation_handler, tts_handler, tts_service, websocket,
};
use anyhow::{Context, Result};
use axum::{
    Router,
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::{delete, get, post},
};
use persistence::UserPersistence;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use tts_service::eleven_labs_tts_provider::ElevenLabsTtsProvider;

pub fn init_logging() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
        .init();
}

pub async fn init_qdrant(url: &str, api_key: &str) -> Result<Arc<qdrant_service::QdrantService>> {
    let qdrant = qdrant_service::QdrantService::new(url, api_key)
        .await
        .context("Failed to initialize Qdrant service")?;
    qdrant.get_collection_info().await?;
    Ok(Arc::new(qdrant))
}

pub fn init_embeddings() -> Result<Arc<embedding_service::EmbeddingService>> {
    let embeddings = embedding_service::EmbeddingService::new()
        .context("Failed to initialize embedding service")?;
    Ok(Arc::new(embeddings))
}

pub fn init_agent(
    llm_config: &dialect_coach_shared::config::LlmConfig,
    qdrant: Arc<qdrant_service::QdrantService>,
    embeddings: Arc<embedding_service::EmbeddingService>,
) -> Result<Arc<agent_service::AgentService>> {
    let agent = agent_service::AgentService::from_config(llm_config, qdrant, embeddings)
        .context("Failed to initialize agent service")?;
    Ok(Arc::new(agent))
}

pub async fn init_persistence(
    database_url: Option<&str>,
    db_path: &str,
) -> Result<Arc<dyn UserPersistence>> {
    let user_persistence = persistence::create_persistence(database_url, db_path)
        .await
        .context("Failed to create persistence")?;
    user_persistence
        .initialize()
        .await
        .context("Failed to initialize user persistence")?;
    Ok(user_persistence)
}

pub fn init_auth(user_persistence: Arc<dyn UserPersistence>) -> Arc<dyn auth_service::AuthService> {
    Arc::new(auth_service::InviteCodeAuthService::new(user_persistence))
}

pub fn init_rate_limiter(
    rate_limits: &dialect_coach_shared::config::RateLimitsConfig,
) -> (
    Arc<rate_limiter::config::RateLimitConfig>,
    Arc<rate_limiter::org_quota::OrgQuotaChecker>,
    Arc<rate_limiter::service::RateLimiter>,
) {
    let config = Arc::new(rate_limiter::config::RateLimitConfig::from_yaml_config(
        rate_limits,
    ));
    let org_quota_checker = Arc::new(rate_limiter::org_quota::OrgQuotaChecker::new());
    let rate_limiter = Arc::new(rate_limiter::service::RateLimiter::new(
        org_quota_checker.clone(),
    ));
    (config, org_quota_checker, rate_limiter)
}

pub fn init_tts(
    tts_config: &dialect_coach_shared::config::TtsConfig,
    user_persistence: Arc<dyn UserPersistence>,
    rate_limiter: Arc<rate_limiter::service::RateLimiter>,
    rate_limit_config: Arc<rate_limiter::config::RateLimitConfig>,
    connections: crate::state::Connections,
) -> Option<tts_handler::TtsState> {
    match ElevenLabsTtsProvider::from_config(&tts_config.eleven_labs) {
        Ok(tts_provider) => {
            let tts_service = tts_service::TtsService::new(Arc::new(tts_provider));
            tracing::info!("TTS service is available");
            Some(tts_handler::TtsState {
                service: Arc::new(tts_service),
                user_persistence,
                rate_limiter,
                rate_limit_config,
                connections,
            })
        }
        Err(e) => {
            tracing::warn!(
                "TTS service initialization failed: {}. TTS endpoints will return errors.",
                e
            );
            None
        }
    }
}

pub fn build_router(state: AppState, tts_state: Option<tts_handler::TtsState>) -> Router {
    let mut app = Router::new()
        .route("/health", get(health_check))
        .route("/ws", get(websocket::websocket_handler))
        .route(
            "/api/translate",
            post(translation_handler::translate_handler),
        )
        .route("/api/grammar", post(grammar_handler::grammar_handler))
        .route(
            "/api/learning/enrich",
            post(enrichment_handler::enrich_handler),
        )
        .route(
            "/api/plans/generate",
            post(planning_handler::generate_plan_handler)
                .layer(axum::extract::DefaultBodyLimit::max(10 * 1024 * 1024)),
        )
        .route("/admin", get(serve_admin_html))
        .route("/admin/api/status", get(admin::status::get_admin_status))
        .route("/admin/api/invites", post(admin_invites::create_invite))
        .route("/admin/api/invites", get(admin_invites::list_invites))
        .route(
            "/admin/api/invites/{code}",
            delete(admin_invites::delete_invite),
        );

    // Add TTS routes only if TTS service is available
    if let Some(tts_state) = tts_state {
        let tts_router = Router::new()
            .route("/synthesize", post(tts_handler::synthesize_handler))
            .route("/status", get(tts_handler::status_handler))
            .route("/cache", delete(tts_handler::clear_cache_handler))
            .with_state(tts_state);
        app = app.nest("/api/tts", tts_router);
    } else {
        // Add fallback TTS endpoints that return service unavailable
        let fallback_router = Router::new()
            .route("/synthesize", post(tts_handler::tts_unavailable_handler))
            .route("/status", get(tts_handler::tts_unavailable_handler))
            .route("/cache", delete(tts_handler::tts_unavailable_handler));
        app = app.nest("/api/tts", fallback_router);
    }

    app.layer(TraceLayer::new_for_http())
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .with_state(state)
}

async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

async fn serve_admin_html() -> Result<Html<String>, StatusCode> {
    match std::fs::read_to_string("static/admin.html") {
        Ok(content) => Ok(Html(content)),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}
