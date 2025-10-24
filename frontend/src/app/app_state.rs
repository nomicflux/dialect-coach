use dialect_coach_shared::models::{Dialect, Formality, Language, Message, TeachingMode};
use dialect_coach_shared::{Explained, Mistake};
use log::{error};
use std::cell::RefCell;
use std::rc::Rc;
use uuid::Uuid;
use yew::prelude::*;

use crate::services::speech::CloudTtsService;
use crate::services::translation::TranslationService;
use crate::services::websocket::{ConnectionState, WebSocketService};

#[derive(Clone, PartialEq)]
pub enum LearningItemType {
    Mistake(Mistake),
    Explanation(Explained),
}

#[derive(Clone, PartialEq)]
pub struct LearningItem {
    pub item: LearningItemType,
    pub score: u8,
}

#[derive(Clone, PartialEq)]
struct MessagesState {
    messages: Vec<Message>,
}

impl Default for MessagesState {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
        }
    }
}

impl MessagesState {
    fn push_message(&self, msg: Message) -> Self {
        let mut msgs = self.messages.clone();
        msgs.push(msg);
        Self { messages: msgs }
    }
}

#[derive(Clone)]
struct LanguageChoices {
    selected_language: Language,
    selected_dialect: Dialect,
}

impl Default for LanguageChoices {
    fn default() -> Self {
        Self {
            selected_language: Language::Spanish,
            selected_dialect: Dialect::SpanishCuban,
        }
    }
}

impl LanguageChoices {
    fn default_dialect(lang: Language) -> Dialect {
        match lang {
            Language::Spanish => Dialect::SpanishCuban,
            Language::Arabic => Dialect::ArabicEgyptian,
            Language::French => Dialect::FrenchParisian,
        }
    }

    fn with_language(&self, language: Language) -> Self {
        if self.selected_language == language {
            self.clone()
        } else {
            Self {
                selected_language: language,
                selected_dialect: Self::default_dialect(language),
            }
        }
    }

    fn with_dialect(&self, dialect: Dialect) -> Self {
        Self {
            selected_dialect: dialect,
            ..self.clone()
        }
    }

    fn rotate_dialect(&self) -> Self {
        let current_dialects = Dialect::for_language(self.selected_language);
        let current_index = current_dialects
            .iter()
            .position(|d| *d == self.selected_dialect)
            .unwrap_or(0);
        let next_index = (current_index + 1) % current_dialects.len();
        let next_dialect = current_dialects[next_index];
        self.with_dialect(next_dialect)
    }
}

#[derive(Clone)]
struct LanguageManner {
    formality: Formality,
    teaching_mode: TeachingMode,
}

impl Default for LanguageManner {
    fn default() -> Self {
        Self {
            formality: Formality::Casual,
            teaching_mode: TeachingMode::Immersive,
        }
    }
}

impl LanguageManner {
    fn with_formality(&self, formality: Formality) -> Self {
        Self {
            formality,
            ..self.clone()
        }
    }

    fn with_teaching_mode(&self, teaching_mode: TeachingMode) -> Self {
        Self {
            teaching_mode,
            ..self.clone()
        }
    }
}

pub enum AppStateAction {
    AddMessage(Message),
    SetLoading,
    LoadingComplete,
    SetError(String),
    ClearError,
    SetConnectionState(ConnectionState),
    ChangeDialect(Dialect),
    RotateDialect,
    ChangeLanguage(Language),
    ChangeFormality(Formality),
    ChangeTeachingMode(TeachingMode),
    Speak(Message),
}

#[derive(Clone)]
pub struct AppState {
    session_id: Uuid,
    language_choices: LanguageChoices,
    language_manner: LanguageManner,
    messages: MessagesState,
    pub connection_state: ConnectionState,
    pub is_loading: bool,
    pub error_message: Option<String>,
    pub ws_service: Rc<RefCell<WebSocketService>>,
    pub tts_service: Option<Rc<CloudTtsService>>,
    pub translation_service: Rc<TranslationService>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            session_id: Uuid::new_v4(),
            messages: MessagesState::default(),
            connection_state: ConnectionState::Disconnected,
            language_choices: LanguageChoices::default(),
            language_manner: LanguageManner::default(),
            is_loading: false,
            error_message: None,
            ws_service: Rc::new(RefCell::new(WebSocketService::new(
                "ws://localhost:3000/ws",
            ))),
            tts_service: Some(Rc::new(CloudTtsService::new("http://localhost:3000"))),
            translation_service: Rc::new(TranslationService::new("http://localhost:3000")),
        }
    }
}

impl AppState {
    pub fn create_msg(&self, content: &String) -> Message {
        let agent_response = dialect_coach_shared::AgentResponse::from(content);
        Message::new(
            self.session_id,
            "user".to_string(),
            agent_response,
            self.language_choices
                .selected_dialect
                .bcp47_tag()
                .to_string(),
            self.language_manner.formality,
            self.language_manner.teaching_mode,
        )
    }

    pub fn current_dialects(&self) -> Vec<Dialect> {
        Dialect::for_language(self.language_choices.selected_language)
    }

    pub fn bcp47_tag(&self) -> String {
        self.language_choices.selected_dialect.bcp47_tag().to_string()
    }

    pub fn current_dialect(&self) -> Dialect {
        self.language_choices.selected_dialect
    }

    pub fn current_formality(&self) -> Formality {
        self.language_manner.formality
    }

    pub fn current_messages(&self) -> Vec<Message> {
        self.messages.messages.clone()
    }

    pub fn apply_action(&self, action: AppStateAction) -> Self {
        let mut next = self.clone();
        match action {
            AppStateAction::AddMessage(msg) => next.messages = self.messages.push_message(msg),
            AppStateAction::SetLoading => next.is_loading = true,
            AppStateAction::LoadingComplete => next.is_loading = false,
            AppStateAction::SetError(msg) => next.error_message = Some(msg),
            AppStateAction::ClearError => next.error_message = None,
            AppStateAction::SetConnectionState(conn_state) => next.connection_state = conn_state,
            AppStateAction::ChangeDialect(dialect) => {
                next.language_choices = next.language_choices.with_dialect(dialect)
            }
            AppStateAction::RotateDialect => {
                next.language_choices = next.language_choices.rotate_dialect()
            }
            AppStateAction::ChangeLanguage(language) => {
                next.language_choices = next.language_choices.with_language(language)
            }
            AppStateAction::ChangeFormality(formality) => {
                next.language_manner = next.language_manner.with_formality(formality)
            }
            AppStateAction::ChangeTeachingMode(tm) => {
                next.language_manner = next.language_manner.with_teaching_mode(tm)
            }
            AppStateAction::Speak(msg) => {
                let tts_service = next.tts_service.clone();
                let language_code = next
                    .language_choices
                    .selected_dialect
                    .bcp47_tag()
                    .to_string();
                let text = msg.content.response.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    if let Some(tts) = tts_service
                        && let Err(e) = tts.speak(&text, &language_code).await
                    {
                        error!("Failed to replay message with TTS: {}", e);
                    }
                });
            }
        };
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
    AddLearningItems(Vec<Mistake>, Vec<Explained>),
}

#[derive(Clone)]
pub struct UIState {
    pub panel_open: bool,
    pub input_prompt_value: Option<String>,
    pub translating_button: Option<String>,
    pub learning_items: Vec<LearningItem>,
    pub learning_panel_open: bool,
}

impl Default for UIState {
    fn default() -> Self {
        Self {
            panel_open: false,
            input_prompt_value: None,
            translating_button: None,
            learning_items: Vec::new(),
            learning_panel_open: false,
        }
    }
}

fn create_learning_item(item_type: LearningItemType) -> LearningItem {
    LearningItem {
        item: item_type,
        score: 0,
    }
}

fn merge_items(
    mut existing: Vec<LearningItem>,
    mistakes: Vec<Mistake>,
    explained: Vec<Explained>,
) -> Vec<LearningItem> {
    for mistake in mistakes {
        existing.push(create_learning_item(LearningItemType::Mistake(mistake)));
    }
    for expl in explained {
        existing.push(create_learning_item(LearningItemType::Explanation(expl)));
    }
    existing
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
            UIStateAction::AddLearningItems(mistakes, explained) => {
                next.learning_items = merge_items(self.learning_items.clone(), mistakes, explained)
            }
        };
        next
    }
}

impl Reducible for UIState {
    type Action = UIStateAction;

    fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
        self.clone().to_owned().apply_action(action).into()
    }
}
