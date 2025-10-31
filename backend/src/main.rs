mod admin;
mod agent_service;
mod embedding_service;
mod persistence;
mod qdrant_service;
mod translation_handler;
mod tts_handler;
mod tts_service;
mod websocket;

use anyhow::{Context, Result};
use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::{delete, get, post},
    Json,
    Router,
};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
//use tts_service::azure_tts_provider::{AzureTtsProvider};
use tts_service::eleven_labs_tts_provider::{ElevenLabsTtsProvider};
use uuid::Uuid;

use persistence::{SledPersistence, UserPersistence};

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub qdrant: Arc<qdrant_service::QdrantService>,
    pub agent: Arc<agent_service::AgentService>,
    pub embeddings: Arc<embedding_service::EmbeddingService>,
    pub session_histories: Arc<Mutex<HashMap<Uuid, Vec<String>>>>,
    pub user_persistence: Arc<dyn UserPersistence>,
}

/// Serve admin HTML page
async fn serve_admin_html() -> Result<Html<String>, StatusCode> {
    match std::fs::read_to_string("static/admin.html") {
        Ok(content) => Ok(Html(content)),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

/// Get admin status from all monitors
async fn get_admin_status(State(state): State<AppState>) -> Json<admin::types::AdminStatusResponse> {
    use chrono::Utc;

    let timestamp = Utc::now().to_rfc3339();
    let anthropic = fetch_anthropic_stats().await;

    let elevenlabs = match std::env::var("ELEVEN_LABS_API_KEY") {
        Ok(api_key) => admin::elevenlabs_monitor::get_elevenlabs_usage(&api_key).await.ok(),
        Err(_) => None,
    };

    let qdrant = admin::qdrant_monitor::get_qdrant_stats(&state.qdrant).await.ok();

    Json(admin::types::AdminStatusResponse {
        timestamp,
        anthropic,
        elevenlabs,
        qdrant,
    })
}

async fn fetch_anthropic_stats() -> admin::types::AnthropicStats {
    match std::env::var("ANTHROPIC_ADMIN_API_KEY") {
        Ok(api_key) => fetch_with_key(&api_key).await,
        Err(_) => anthropic_stats_error("ANTHROPIC_ADMIN_API_KEY not set"),
    }
}

async fn fetch_with_key(api_key: &str) -> admin::types::AnthropicStats {
    match admin::anthropic_monitor::get_anthropic_stats(api_key).await {
        Ok(stats) => stats,
        Err(e) => anthropic_stats_error(&format!("API error: {}", e)),
    }
}

fn anthropic_stats_error(error: &str) -> admin::types::AnthropicStats {
    admin::types::AnthropicStats {
        uncached_input_tokens: None,
        cached_input_tokens: None,
        cache_creation_tokens: None,
        output_tokens: None,
        total_cost_usd: None,
        error: Some(error.to_string()),
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                "debug".into()
            }),
        )
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
        .init();
    let qdrant = qdrant_service::QdrantService::from_env()
        .await
        .context("Failed to initialize Qdrant service")?;
    qdrant.get_collection_info().await?;
    let qdrant = Arc::new(qdrant);
    tracing::info!("Initializing embedding service...");
    let embeddings = embedding_service::EmbeddingService::new()
        .context("Failed to initialize embedding service")?;
    let embeddings = Arc::new(embeddings);
    let agent = agent_service::AgentService::from_env(qdrant.clone(), embeddings.clone())
        .context("Failed to initialize agent service")?;

    tracing::info!("Initializing user persistence...");
    let db_path = std::env::var("DB_PATH")
        .unwrap_or_else(|_| "./data/dialect-coach.db".to_string());
    let user_persistence: Arc<dyn UserPersistence> = Arc::new(
        SledPersistence::new(&db_path)
            .context("Failed to create SledPersistence")?
    );
    user_persistence
        .initialize()
        .await
        .context("Failed to initialize user persistence")?;

    let tts_state = match ElevenLabsTtsProvider::from_env() {
        Ok(tts_provider) => {
            let tts_service = tts_service::TtsService::new(Arc::new(tts_provider));
            tracing::info!("TTS service is available");
            Some(tts_handler::TtsState {
                service: Arc::new(tts_service),
            })
        }
        Err(e) => {
            tracing::warn!(
                "TTS service initialization failed: {}. TTS endpoints will return errors.",
                e
            );
            None
        }
    };

    let state = AppState {
        qdrant,
        agent: Arc::new(agent),
        embeddings,
        session_histories: Arc::new(Mutex::new(HashMap::new())),
        user_persistence,
    };

    // Build main application with routes
    let mut app = Router::new()
        .route("/health", get(health_check))
        .route("/ws", get(websocket::websocket_handler))
        .route("/ws/user_state", get(websocket::user_state_websocket_handler))
        .route("/ws/user", get(websocket::user_websocket_handler))
        .route(
            "/api/translate",
            post(translation_handler::translate_handler),
        )
        .route("/admin", get(serve_admin_html))
        .route("/admin/api/status", get(get_admin_status));

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

    let app = app
        .layer(TraceLayer::new_for_http())
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .with_state(state);

    // Start server
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::info!("Backend server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// Health check endpoint
async fn health_check() -> impl IntoResponse {
    // Note: We could enhance this to include TTS status in the future
    (StatusCode::OK, "OK")
}
