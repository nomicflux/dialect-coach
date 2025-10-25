use dialect_coach_shared::models::{Dialect, Formality, Language, Message, TeachingMode};
use dialect_coach_shared::{AgentAnalysis, Explained, Exploratory, Mistake, Translated};
use dialect_coach_shared::{UserState, LearningItem, LearningItemType};
use log::{error};
use std::cell::RefCell;
use std::rc::Rc;
use uuid::Uuid;
use yew::prelude::*;

use crate::services::save_queue::PendingSaveQueue;
use crate::services::speech::CloudTtsService;
use crate::services::translation::TranslationService;
use crate::services::websocket::{ConnectionState, WebSocketService};


pub enum AppStateAction {
    SetLoading,
    LoadingComplete,
    SetError(String),
    ClearError,
    SetConnectionState(ConnectionState),
    Speak(Message),
    QueuePendingSave(UserState),
    RetryPendingSaves,
}

#[derive(Clone)]
pub struct AppState {
    session_id: Uuid,
    pub connection_state: ConnectionState,
    pub is_loading: bool,
    pub error_message: Option<String>,
    pub ws_service: Rc<RefCell<WebSocketService>>,
    pub tts_service: Option<Rc<CloudTtsService>>,
    pub translation_service: Rc<TranslationService>,
    pub save_queue: Rc<PendingSaveQueue>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            session_id: Uuid::new_v4(),
            connection_state: ConnectionState::Disconnected,
            is_loading: false,
            error_message: None,
            ws_service: Rc::new(RefCell::new(WebSocketService::new(
                "ws://localhost:3000/ws",
            ))),
            tts_service: Some(Rc::new(CloudTtsService::new("http://localhost:3000"))),
            translation_service: Rc::new(TranslationService::new("http://localhost:3000")),
            save_queue: Rc::new(PendingSaveQueue::new()),
        }
    }
}

impl AppState {
    pub fn session_id(&self) -> Uuid {
        self.session_id
    }

    fn apply_action(&self, action: AppStateAction) -> Self {
        let mut next = self.clone();
        match action {
            AppStateAction::SetLoading => next.is_loading = true,
            AppStateAction::LoadingComplete => next.is_loading = false,
            AppStateAction::SetError(msg) => next.error_message = Some(msg),
            AppStateAction::ClearError => next.error_message = None,
            AppStateAction::SetConnectionState(conn_state) => next.connection_state = conn_state,
            AppStateAction::Speak(msg) => {
                let tts_service = next.tts_service.clone();
                let language_code = msg.language.clone();
                let text = msg.content.response.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    if let Some(tts) = tts_service
                        && let Err(e) = tts.speak(&text, &language_code).await
                    {
                        error!("Failed to replay message with TTS: {}", e);
                    }
                });
            }
            AppStateAction::QueuePendingSave(state) => {
                next.save_queue.enqueue(state);
            }
            AppStateAction::RetryPendingSaves => {
                if let Err(e) = next.save_queue.retry_all() {
                    error!("Failed to retry pending saves: {}", e);
                }
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

pub enum UIStateAction {
    OpenPanel,
    ClosePanel,
    EnterInputPrompt(String),
    ClearInputPrompt,
    PushTranslatingButton(String),
    ClearTranslatingButton,
    OpenLearningPanel,
    CloseLearningPanel,
}

#[derive(Clone)]
pub struct UIState {
    pub panel_open: bool,
    pub input_prompt_value: Option<String>,
    pub translating_button: Option<String>,
    pub learning_panel_open: bool,
}

impl Default for UIState {
    fn default() -> Self {
        Self {
            panel_open: false,
            input_prompt_value: None,
            translating_button: None,
            learning_panel_open: false,
        }
    }
}

impl UIState {
    fn apply_action(&self, action: UIStateAction) -> Self {
        let mut next = self.clone();
        match action {
            UIStateAction::OpenPanel => next.panel_open = true,
            UIStateAction::ClosePanel => next.panel_open = false,
            UIStateAction::EnterInputPrompt(input) => next.input_prompt_value = Some(input),
            UIStateAction::ClearInputPrompt => next.input_prompt_value = None,
            UIStateAction::PushTranslatingButton(msg) => next.translating_button = Some(msg),
            UIStateAction::ClearTranslatingButton => next.translating_button = None,
            UIStateAction::OpenLearningPanel => next.learning_panel_open = true,
            UIStateAction::CloseLearningPanel => next.learning_panel_open = false,
        }
        next
    }
}

impl Reducible for UIState {
    type Action = UIStateAction;

    fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
        self.clone().to_owned().apply_action(action).into()
    }
}

pub enum UserStateAction {
    AddMessage(Message),
    AddLearningItems(Vec<Mistake>, Vec<Explained>, Vec<Translated>, Vec<Exploratory>),
    UpdateScores(AgentAnalysis),
    ChangeDialect(Dialect),
    ChangeLanguage(Language),
    ChangeFormality(Formality),
    ChangeTeachingMode(TeachingMode),
    ToggleTTS,
}

fn add_learning_items_to_vec(
    mut items: Vec<LearningItem>,
    mistakes: Vec<Mistake>,
    explained: Vec<Explained>,
    translated: Vec<Translated>,
    exploratory: Vec<Exploratory>,
) -> Vec<LearningItem> {
    for mistake in mistakes {
        items.push(LearningItem::new(LearningItemType::Mistake(mistake)));
    }
    for expl in explained {
        items.push(LearningItem::new(LearningItemType::Explanation(expl)));
    }
    for trans in translated {
        items.push(LearningItem::new(LearningItemType::Translation(trans)));
    }
    for explor in exploratory {
        items.push(LearningItem::new(LearningItemType::Exploration(explor)));
    }
    items
}

fn update_item_score(mut item: LearningItem, analysis: &AgentAnalysis) -> LearningItem {
    match &item.item {
        LearningItemType::Mistake(m) => {
            if let Some(score_obj) = analysis.mistake_scores.get(&m.id) {
                item.score = score_obj.score.max(0) as u8;
            }
        }
        LearningItemType::Explanation(e) => {
            if let Some(score_obj) = analysis.explained_scores.get(&e.id) {
                item.score = score_obj.score.max(0) as u8;
            }
        }
        LearningItemType::Translation(t) => {
            if let Some(score_obj) = analysis.translated_scores.get(&t.id) {
                item.score = score_obj.score.max(0) as u8;
            }
        }
        LearningItemType::Exploration(e) => {
            if let Some(score_obj) = analysis.exploratory_scores.get(&e.id) {
                item.score = score_obj.score.max(0) as u8;
            }
        }
    }
    item
}

fn apply_score_updates(items: Vec<LearningItem>, analysis: &AgentAnalysis) -> Vec<LearningItem> {
    items.into_iter().map(|item| update_item_score(item, analysis)).collect()
}

fn default_dialect_for_language(lang: Language) -> Dialect {
    match lang {
        Language::Spanish => Dialect::SpanishCuban,
        Language::Arabic => Dialect::ArabicEgyptian,
        Language::French => Dialect::FrenchParisian,
    }
}

fn apply_user_state_action(state: &UserState, action: UserStateAction) -> UserState {
    let mut next = state.clone();
    match action {
        UserStateAction::AddMessage(msg) => {
            next.conversation_history.push(msg);
        }
        UserStateAction::AddLearningItems(mistakes, explained, translated, exploratory) => {
            next.learning_items = add_learning_items_to_vec(
                next.learning_items,
                mistakes,
                explained,
                translated,
                exploratory,
            );
        }
        UserStateAction::UpdateScores(analysis) => {
            next.learning_items = apply_score_updates(next.learning_items, &analysis);
        }
        UserStateAction::ChangeDialect(dialect) => {
            next.selected_dialect = dialect;
        }
        UserStateAction::ChangeLanguage(language) => {
            next.selected_language = language;
            next.selected_dialect = default_dialect_for_language(language);
        }
        UserStateAction::ChangeFormality(formality) => {
            next.formality = formality;
        }
        UserStateAction::ChangeTeachingMode(tm) => {
            next.teaching_mode = tm;
        }
        UserStateAction::ToggleTTS => {
            next.tts_enabled = !next.tts_enabled;
        }
    }
    next
}

// Newtype wrapper to implement Reducible (orphan rule workaround)
#[derive(Clone, PartialEq)]
pub struct UserStateWrapper(pub UserState);

impl std::ops::Deref for UserStateWrapper {
    type Target = UserState;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<UserState> for UserStateWrapper {
    fn from(state: UserState) -> Self {
        UserStateWrapper(state)
    }
}

impl Reducible for UserStateWrapper {
    type Action = UserStateAction;

    fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
        UserStateWrapper(apply_user_state_action(&self.0, action)).into()
    }
}
