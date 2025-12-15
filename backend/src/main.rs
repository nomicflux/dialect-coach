use dialect_coach_backend::AppState;
use dialect_coach_backend::agent_service;
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

    startup::init_logging();

    let qdrant = startup::init_qdrant().await?;
    let embeddings = startup::init_embeddings()?;
    let agent = startup::init_agent(qdrant.clone(), embeddings.clone())?;

    tracing::info!("Initializing user persistence...");
    let user_persistence = startup::init_persistence().await?;

    let auth_service = startup::init_auth(user_persistence.clone());

    let (rate_limit_config, org_quota_checker, rate_limiter) = startup::init_rate_limiter();

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

    let state = AppState {
        qdrant,
        agent,
        embeddings,
        session_histories: Arc::new(Mutex::new(HashMap::new())),
        user_persistence,
        auth_service,
        rate_limiter,
        rate_limit_config,
        org_quota_checker,
        user_state_connections,
        planning_generator,
        admin_token,
    };

    let app = startup::build_router(state, tts_state);

    // Start server
    let bind_addr = std::env::var("BIND_ADDRESS")
        .unwrap_or_else(|_| "0.0.0.0:3000".to_string());
    let addr: SocketAddr = bind_addr.parse()?;
    tracing::info!("Backend server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
