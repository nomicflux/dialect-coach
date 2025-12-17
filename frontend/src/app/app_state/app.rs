use dialect_coach_shared::models::dialect::dialect_features;
use dialect_coach_shared::models::{Message, User, UserState};
use log::error;
use std::cell::RefCell;
use std::rc::Rc;
use uuid::Uuid;
use yew::prelude::*;

use crate::services::enrichment_service::EnrichmentService;
use crate::services::plan_service::PlanService;
use crate::services::save_queue::PendingSaveQueue;
use crate::services::speech::CloudTtsService;
use crate::services::translation::TranslationService;
use crate::services::user_state_websocket::UserStateWebSocketService;
use crate::services::user_websocket::UserWebSocketService;
use crate::services::websocket::{ConnectionState, WebSocketService};

pub enum AppStateAction {
    SetLoading,
    LoadingComplete,
    SetError(String),
    ClearError,
    SetConnectionState(ConnectionState),
    Speak(Message),
    QueuePendingSave(Box<UserState>),
    RetryPendingSaves,
    SetUser(User),
    ClearUser,
    LoadUserState(Uuid),
    CreateSession(Uuid),
    DestroySession,
    NotifyTTSEnabled(bool),
    ProcessAgentMessage(Message),
    SetResponseRateLimited(bool),
    SetAnalysisRateLimited(bool),
    SetTtsRateLimited(bool),
}

#[derive(Clone, Default)]
pub struct RateLimitState {
    pub response_limited: bool,
    pub analysis_limited: bool,
    pub tts_limited: bool,
}

#[derive(Clone)]
pub struct AppState {
    pub session_id: Option<Uuid>,
    pub connection_state: ConnectionState,
    pub is_loading: bool,
    pub error_message: Option<String>,
    pub current_user: Option<User>,
    pub ws_service: Rc<RefCell<WebSocketService>>,
    pub user_state_ws_service: Rc<RefCell<UserStateWebSocketService>>,
    pub user_ws_service: Rc<RefCell<UserWebSocketService>>,
    pub tts_service: Option<Rc<CloudTtsService>>,
    pub translation_service: Rc<TranslationService>,
    pub enrichment_service: Rc<EnrichmentService>,
    pub plan_service: Rc<PlanService>,
    pub save_queue: Rc<PendingSaveQueue>,
    pub autoplay_enabled: bool,
    pub rate_limit_state: RateLimitState,
}

impl Default for AppState {
    fn default() -> Self {
        let base_url = get_base_url();
        let ws_url = get_ws_url("/ws");
        let user_state_ws_url = get_ws_url("/ws/user_state");
        let user_ws_url = get_ws_url("/ws/user");

        Self {
            session_id: None,
            connection_state: ConnectionState::Disconnected,
            is_loading: false,
            error_message: None,
            current_user: None,
            ws_service: Rc::new(RefCell::new(WebSocketService::new(&ws_url))),
            user_state_ws_service: Rc::new(RefCell::new(UserStateWebSocketService::new(
                &user_state_ws_url,
            ))),
            user_ws_service: Rc::new(RefCell::new(UserWebSocketService::new(&user_ws_url))),
            tts_service: Some(Rc::new(CloudTtsService::new(&base_url))),
            translation_service: Rc::new(TranslationService::new(&base_url)),
            enrichment_service: Rc::new(EnrichmentService::new(&base_url)),
            plan_service: Rc::new(PlanService::new(&base_url)),
            save_queue: Rc::new(PendingSaveQueue::new()),
            autoplay_enabled: false,
            rate_limit_state: RateLimitState::default(),
        }
    }
}

fn load_user_state(ws_service: &Rc<RefCell<UserStateWebSocketService>>, user_id: Uuid) {
    let ws = ws_service.borrow();
    if let Err(e) = ws.load_user_state(user_id) {
        error!("Failed to load user state: {}", e);
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
                let text = msg.get_content();
                wasm_bindgen_futures::spawn_local(async move {
                    if let Some(tts) = tts_service
                        && let Some(uid) = user_id
                        && let Err(e) = tts.speak(uid, &text, dialect).await
                    {
                        error!("Failed to replay message with TTS: {}", e);
                    }
                });
            }
            AppStateAction::QueuePendingSave(state) => {
                next.save_queue.enqueue(*state);
            }
            AppStateAction::RetryPendingSaves => {
                let ws_service = next.user_state_ws_service.borrow();
                if let Err(e) = next.save_queue.retry_all(&ws_service) {
                    error!("Failed to retry pending saves: {}", e);
                }
            }
            AppStateAction::SetUser(user) => {
                load_user_state(&next.user_state_ws_service, user.id);
                next.current_user = Some(user);
            }
            AppStateAction::ClearUser => {
                next.current_user = None;
            }
            AppStateAction::LoadUserState(user_id) => {
                load_user_state(&next.user_state_ws_service, user_id);
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
                    let text = msg.get_content();
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
