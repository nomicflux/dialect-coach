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
    Router,
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
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

use persistence::{InMemoryPersistence, UserPersistence};

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub qdrant: Arc<qdrant_service::QdrantService>,
    pub agent: Arc<agent_service::AgentService>,
    pub embeddings: Arc<embedding_service::EmbeddingService>,
    pub session_histories: Arc<Mutex<HashMap<Uuid, Vec<String>>>>,
    pub user_persistence: Arc<dyn UserPersistence>,
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
    let user_persistence: Arc<dyn UserPersistence> = Arc::new(InMemoryPersistence::new());
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
