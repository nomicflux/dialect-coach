mod agent_service;
mod embedding_service;
mod qdrant_service;
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
use uuid::Uuid;

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub qdrant: Arc<qdrant_service::QdrantService>,
    pub agent: Arc<agent_service::AgentService>,
    pub embeddings: Arc<embedding_service::EmbeddingService>,
    pub session_histories: Arc<Mutex<HashMap<Uuid, Vec<String>>>>,
}

#[tokio::main]
async fn main() -> Result<()> {
    eprintln!("[MAIN] Starting backend...");

    // Load environment variables from .env file
    dotenvy::dotenv().ok();
    eprintln!("[MAIN] Environment variables loaded");

    // Initialize tracing with explicit stderr output
    eprintln!("[MAIN] Initializing tracing...");
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                eprintln!("[MAIN] Using default log level: debug");
                "debug".into()
            }),
        )
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
        .init();
    eprintln!("[MAIN] Tracing initialized");

    // Initialize Qdrant service
    eprintln!("[MAIN] Initializing Qdrant...");
    let qdrant = qdrant_service::QdrantService::from_env()
        .await
        .context("Failed to initialize Qdrant service")?;
    eprintln!("[MAIN] Qdrant initialized");

    // Verify connection
    eprintln!("[MAIN] Verifying Qdrant connection...");
    qdrant.get_collection_info().await?;
    eprintln!("[MAIN] Qdrant connection verified");

    let qdrant = Arc::new(qdrant);

    // Initialize embedding service
    eprintln!("[MAIN] Initializing Fastembed (this may take 30-60 seconds)...");
    tracing::info!("Initializing embedding service...");
    let embeddings = embedding_service::EmbeddingService::new()
        .context("Failed to initialize embedding service")?;
    eprintln!("[MAIN] Fastembed initialized successfully");
    let embeddings = Arc::new(embeddings);

    // Initialize agent service
    eprintln!("[MAIN] Initializing agent service...");
    let agent = agent_service::AgentService::from_env(qdrant.clone(), embeddings.clone())
        .context("Failed to initialize agent service")?;
    eprintln!("[MAIN] Agent service initialized");

    // Initialize TTS service (optional - continue if it fails)
    eprintln!("[MAIN] Initializing TTS service...");
    let tts_state = match tts_service::AzureTtsProvider::from_env() {
        Ok(tts_provider) => {
            let tts_service = tts_service::TtsService::new(Arc::new(tts_provider));
            eprintln!("[MAIN] TTS service initialized successfully");
            tracing::info!("TTS service is available");
            Some(tts_handler::TtsState {
                service: Arc::new(tts_service),
            })
        }
        Err(e) => {
            eprintln!("[MAIN] WARNING: TTS service initialization failed: {}", e);
            tracing::warn!("TTS service initialization failed: {}. TTS endpoints will return errors.", e);
            None
        }
    };

    let state = AppState {
        qdrant,
        agent: Arc::new(agent),
        embeddings,
        session_histories: Arc::new(Mutex::new(HashMap::new())),
    };
    eprintln!("[MAIN] Application state created");

    // Build main application with routes
    let mut app = Router::new()
        .route("/health", get(health_check))
        .route("/ws", get(websocket::websocket_handler));

    // Add TTS routes only if TTS service is available
    if let Some(tts_state) = tts_state {
        let tts_router = Router::new()
            .route("/synthesize", post(tts_handler::synthesize_handler))
            .route("/voices", get(tts_handler::voices_handler))
            .route("/status", get(tts_handler::status_handler))
            .route("/cache", delete(tts_handler::clear_cache_handler))
            .with_state(tts_state);
        app = app.nest("/api/tts", tts_router);
        eprintln!("[MAIN] TTS endpoints added to router");
    } else {
        // Add fallback TTS endpoints that return service unavailable
        let fallback_router = Router::new()
            .route("/synthesize", post(tts_handler::tts_unavailable_handler))
            .route("/voices", get(tts_handler::tts_unavailable_handler))
            .route("/status", get(tts_handler::tts_unavailable_handler))
            .route("/cache", delete(tts_handler::tts_unavailable_handler));
        app = app.nest("/api/tts", fallback_router);
        eprintln!("[MAIN] TTS endpoints added with 'service unavailable' fallback");
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

    eprintln!("[MAIN] Router built with health, ws, and TTS routes");

    // Start server
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    eprintln!("[MAIN] Starting server on {}...", addr);
    tracing::info!("Backend server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    eprintln!("[MAIN] Server is now listening and ready for connections");
    eprintln!("[MAIN] About to start serving requests");
    axum::serve(listener, app).await?;

    Ok(())
}

/// Health check endpoint
async fn health_check() -> impl IntoResponse {
    eprintln!("[HEALTH] Handler called");
    // Note: We could enhance this to include TTS status in the future
    (StatusCode::OK, "OK")
}
