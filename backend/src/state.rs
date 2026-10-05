use crate::agent_service;
use crate::auth_service;
use crate::embedding_service;
use crate::persistence::UserPersistence;
use crate::qdrant_service;
use crate::rate_limiter;
use crate::selection_cache::SelectionCache;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

/// Every open WebSocket connection of each signed-in user, one sender per tab.
pub type Connections = Arc<Mutex<HashMap<Uuid, Vec<tokio::sync::mpsc::UnboundedSender<String>>>>>;

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub qdrant: Arc<qdrant_service::QdrantService>,
    pub agent: Arc<agent_service::AgentService>,
    pub embeddings: Arc<embedding_service::EmbeddingService>,
    pub user_persistence: Arc<dyn UserPersistence>,
    pub auth_service: Arc<dyn auth_service::AuthService>,
    pub rate_limiter: Arc<rate_limiter::service::RateLimiter>,
    pub rate_limit_config: Arc<rate_limiter::config::RateLimitConfig>,
    pub org_quota_checker: Arc<rate_limiter::org_quota::OrgQuotaChecker>,
    pub connections: Connections,
    pub planning_generator: Arc<agent_service::planning::PlanGenerator>,
    pub translation_cache: Arc<SelectionCache>,
    pub admin_token: Option<String>,
}

impl axum::extract::FromRef<AppState> for Arc<agent_service::planning::PlanGenerator> {
    fn from_ref(state: &AppState) -> Self {
        state.planning_generator.clone()
    }
}
