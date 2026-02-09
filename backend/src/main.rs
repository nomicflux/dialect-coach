use dialect_coach_backend::AppState;
use dialect_coach_backend::agent_service;
use dialect_coach_backend::selection_cache::SelectionCache;
use dialect_coach_backend::startup;

use anyhow::Result;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    let config = dialect_coach_shared::config::load_config("config.yaml")
        .expect("Failed to load config.yaml");

    startup::init_logging();

    let qdrant = startup::init_qdrant(&config.qdrant.url, &std::env::var("QDRANT_API_KEY")?).await?;
    let embeddings = startup::init_embeddings()?;
    let agent = startup::init_agent(qdrant.clone(), embeddings.clone())?;

    tracing::info!("Initializing user persistence...");
    let user_persistence = startup::init_persistence(
        std::env::var("DATABASE_URL").ok().as_deref(),
        &config.persistence.db_path,
    )
    .await?;

    let auth_service = startup::init_auth(user_persistence.clone());

    let (rate_limit_config, org_quota_checker, rate_limiter) =
        startup::init_rate_limiter(&config.rate_limits);

    let user_state_connections: Arc<
        Mutex<HashMap<Uuid, tokio::sync::mpsc::UnboundedSender<String>>>,
    > = Arc::new(Mutex::new(HashMap::new()));

    let tts_state = startup::init_tts(
        user_persistence.clone(),
        rate_limiter.clone(),
        rate_limit_config.clone(),
        user_state_connections.clone(),
    );

    let anthropic_admin_key = std::env::var("ANTHROPIC_ADMIN_API_KEY").ok();
    let elevenlabs_api_key = std::env::var("ELEVENLABS_API_KEY").ok();
    org_quota_checker
        .clone()
        .spawn_background_task(anthropic_admin_key, elevenlabs_api_key);

    let admin_token = std::env::var("ADMIN_TOKEN").ok();

    let planning_generator = Arc::new(agent_service::planning::PlanGenerator::new(
        agent.planning_agent.clone(),
        agent.planning_config.clone(),
    ));

    let translation_cache = Arc::new(SelectionCache::new(100));

    let state = AppState {
        qdrant,
        agent,
        embeddings,
        user_persistence,
        auth_service,
        rate_limiter,
        rate_limit_config,
        org_quota_checker,
        user_state_connections,
        planning_generator,
        translation_cache,
        admin_token,
    };

    let app = startup::build_router(state, tts_state);

    // Start server
    let addr: SocketAddr = config.server.bind_address.parse()?;
    tracing::info!("Backend server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
