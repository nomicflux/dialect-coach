use dialect_coach_shared::models::dialect::dialect_features;
use dialect_coach_shared::models::{Message, User};
use log::error;
use std::rc::Rc;
use uuid::Uuid;
use yew::prelude::*;

use crate::services::connection::{Connection, ConnectionStatus};
use crate::services::enrichment_service::EnrichmentService;
use crate::services::grammar::GrammarService;
use crate::services::plan_service::PlanService;
use crate::services::speech::CloudTtsService;
use crate::services::translation::TranslationService;

pub enum AppStateAction {
    SetLoading,
    LoadingComplete,
    SetError(String),
    ClearError,
    SetConnectionState(ConnectionStatus),
    Speak(Message),
    /// The signed-in user and their session token.
    SetUser(User, String),
    ClearUser,
    CreateSession(Uuid),
    DestroySession,
    NotifyTTSEnabled(bool),
    ProcessAgentMessage(Message),
    SetResponseRateLimited(bool),
    SetAnalysisRateLimited(bool),
    SetTtsRateLimited(bool),
    StorePendingInitialSettings(Option<dialect_coach_shared::InitialUserSettings>),
}

#[derive(Clone, Default, PartialEq)]
pub struct RateLimitState {
    pub response_limited: bool,
    pub analysis_limited: bool,
    pub tts_limited: bool,
}

#[derive(Clone)]
pub struct AppState {
    pub session_id: Option<Uuid>,
    pub connection_state: ConnectionStatus,
    pub is_loading: bool,
    pub error_message: Option<String>,
    pub current_user: Option<User>,
    /// Kept in memory so a tab can renew its sign-in after the cookie expires.
    pub session_token: Option<String>,
    pub connection: Connection,
    pub tts_service: Option<Rc<CloudTtsService>>,
    pub translation_service: Rc<TranslationService>,
    pub grammar_service: Rc<GrammarService>,
    pub enrichment_service: Rc<EnrichmentService>,
    pub plan_service: Rc<PlanService>,
    pub autoplay_enabled: bool,
    pub rate_limit_state: RateLimitState,
    pub pending_initial_settings: Option<dialect_coach_shared::InitialUserSettings>,
}

impl PartialEq for AppState {
    fn eq(&self, other: &Self) -> bool {
        self.session_id == other.session_id
            && self.connection_state == other.connection_state
            && self.is_loading == other.is_loading
            && self.error_message == other.error_message
            && self.current_user == other.current_user
            && self.session_token == other.session_token
            && self.connection == other.connection
            && match (&self.tts_service, &other.tts_service) {
                (Some(a), Some(b)) => Rc::ptr_eq(a, b),
                (None, None) => true,
                _ => false,
            }
            && Rc::ptr_eq(&self.translation_service, &other.translation_service)
            && Rc::ptr_eq(&self.grammar_service, &other.grammar_service)
            && Rc::ptr_eq(&self.enrichment_service, &other.enrichment_service)
            && Rc::ptr_eq(&self.plan_service, &other.plan_service)
            && self.autoplay_enabled == other.autoplay_enabled
            && self.rate_limit_state == other.rate_limit_state
            && self.pending_initial_settings == other.pending_initial_settings
    }
}

impl Default for AppState {
    fn default() -> Self {
        let connection = Connection::new(&get_ws_url("/ws"));

        Self {
            session_id: None,
            connection_state: ConnectionStatus::Connecting,
            is_loading: false,
            error_message: None,
            current_user: None,
            session_token: None,
            tts_service: Some(Rc::new(CloudTtsService::new(connection.clone()))),
            translation_service: Rc::new(TranslationService::new(connection.clone())),
            grammar_service: Rc::new(GrammarService::new(connection.clone())),
            enrichment_service: Rc::new(EnrichmentService::new(connection.clone())),
            plan_service: Rc::new(PlanService::new(connection.clone())),
            connection,
            autoplay_enabled: false,
            rate_limit_state: RateLimitState::default(),
            pending_initial_settings: None,
        }
    }
}

impl AppState {
    pub fn session_id(&self) -> Option<Uuid> {
        self.session_id
    }

    pub fn apply_action(&self, action: AppStateAction) -> Self {
        let mut next = self.clone();
        match action {
            AppStateAction::SetLoading => next.is_loading = true,
            AppStateAction::LoadingComplete => next.is_loading = false,
            AppStateAction::SetError(msg) => next.error_message = Some(msg),
            AppStateAction::ClearError => next.error_message = None,
            AppStateAction::SetConnectionState(conn_state) => next.connection_state = conn_state,
            AppStateAction::Speak(msg) => {
                let dialect = msg.metadata.dialect;
                if !dialect_features(dialect).has_tts() {
                    return next;
                }
                let tts_service = next.tts_service.clone();
                let user_id = next.current_user.as_ref().map(|u| u.id);
                let text = msg.get_tts_content();
                wasm_bindgen_futures::spawn_local(async move {
                    if let Some(tts) = tts_service
                        && let Some(uid) = user_id
                        && let Err(e) = tts.speak(uid, &text, dialect).await
                    {
                        error!("Failed to replay message with TTS: {}", e);
                    }
                });
            }
            AppStateAction::SetUser(user, token) => {
                next.current_user = Some(user);
                next.session_token = Some(token);
            }
            AppStateAction::ClearUser => {
                next.current_user = None;
                next.session_token = None;
            }
            AppStateAction::CreateSession(uuid) => {
                next.session_id = Some(uuid);
            }
            AppStateAction::DestroySession => {
                next.session_id = None;
            }
            AppStateAction::NotifyTTSEnabled(enabled) => {
                next.autoplay_enabled = enabled;
            }
            AppStateAction::ProcessAgentMessage(msg) => {
                // Read autoplay_enabled from AppState's own state (not from UserState handle)
                if next.autoplay_enabled {
                    let dialect = msg.metadata.dialect;
                    if !dialect_features(dialect).has_tts() {
                        return next;
                    }
                    let tts_service = next.tts_service.clone();
                    let user_id = next.current_user.as_ref().map(|u| u.id);
                    let text = msg.get_tts_content();
                    wasm_bindgen_futures::spawn_local(async move {
                        if let Some(tts) = tts_service
                            && let Some(uid) = user_id
                            && let Err(e) = tts.speak(uid, &text, dialect).await
                        {
                            error!("Failed to replay message with TTS: {}", e);
                        }
                    });
                }
            }
            AppStateAction::SetResponseRateLimited(limited) => {
                next.rate_limit_state.response_limited = limited;
            }
            AppStateAction::SetAnalysisRateLimited(limited) => {
                next.rate_limit_state.analysis_limited = limited;
            }
            AppStateAction::SetTtsRateLimited(limited) => {
                next.rate_limit_state.tts_limited = limited;
            }
            AppStateAction::StorePendingInitialSettings(settings) => {
                next.pending_initial_settings = settings;
            }
        }
        next
    }
}

impl Reducible for AppState {
    type Action = AppStateAction;

    fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
        self.apply_action(action).into()
    }
}

fn get_base_url() -> String {
    web_sys::window()
        .and_then(|w| w.location().origin().ok())
        .unwrap_or_else(|| "http://localhost:3000".to_string())
}

fn get_ws_url(path: &str) -> String {
    let base = get_base_url();
    let ws_protocol = if base.starts_with("https") {
        "wss"
    } else {
        "ws"
    };
    let host = base
        .trim_start_matches("http://")
        .trim_start_matches("https://");
    format!("{}://{}{}", ws_protocol, host, path)
}
