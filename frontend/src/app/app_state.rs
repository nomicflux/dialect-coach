use dialect_coach_shared::models::{
    ConversationBranch, Dialect, Formality, Language, Message, TeachingMode,
};
use dialect_coach_shared::{AgentAnalysis, Explained, Exploratory, Mistake, Translated};
use dialect_coach_shared::{LearningItem, LearningItemType, User, UserState};
use log::error;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use uuid::Uuid;
use yew::prelude::*;

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
    QueuePendingSave(UserState),
    RetryPendingSaves,
    SetUser(User),
    ClearUser,
    LoadUserState(Uuid),
    CreateSession(Uuid),
    DestroySession,
}

#[derive(Clone)]
pub struct AppState {
    session_id: Option<Uuid>,
    pub connection_state: ConnectionState,
    pub is_loading: bool,
    pub error_message: Option<String>,
    pub current_user: Option<User>,
    pub ws_service: Rc<RefCell<WebSocketService>>,
    pub user_state_ws_service: Rc<RefCell<UserStateWebSocketService>>,
    pub user_ws_service: Rc<RefCell<UserWebSocketService>>,
    pub tts_service: Option<Rc<CloudTtsService>>,
    pub translation_service: Rc<TranslationService>,
    pub save_queue: Rc<PendingSaveQueue>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            session_id: None,
            connection_state: ConnectionState::Disconnected,
            is_loading: false,
            error_message: None,
            current_user: None,
            ws_service: Rc::new(RefCell::new(WebSocketService::new(
                "ws://localhost:3000/ws",
            ))),
            user_state_ws_service: Rc::new(RefCell::new(UserStateWebSocketService::new(
                "ws://localhost:3000/ws/user_state",
            ))),
            user_ws_service: Rc::new(RefCell::new(UserWebSocketService::new(
                "ws://localhost:3000/ws/user",
            ))),
            tts_service: Some(Rc::new(CloudTtsService::new("http://localhost:3000"))),
            translation_service: Rc::new(TranslationService::new("http://localhost:3000")),
            save_queue: Rc::new(PendingSaveQueue::new()),
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
                let language_code = msg.metadata.dialect.bcp47_tag();
                let text = msg.get_content();
                wasm_bindgen_futures::spawn_local(async move {
                    if let Some(tts) = tts_service
                        && let Err(e) = tts.speak(&text, language_code).await
                    {
                        error!("Failed to replay message with TTS: {}", e);
                    }
                });
            }
            AppStateAction::QueuePendingSave(state) => {
                next.save_queue.enqueue(state);
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
    SetCreateUsernameInput(String),
    SetSigninUsernameInput(String),
    ClearCreateUsernameInput,
    ClearSigninUsernameInput,
    PushDeletedLearningItem(LearningItem),
    PopDeletedLearningItem,
    PushDeletedMessage(Message),
    PopDeletedMessage,
}

#[derive(Clone, Default)]
pub struct UIState {
    pub panel_open: bool,
    pub input_prompt_value: Option<String>,
    pub translating_button: Option<String>,
    pub learning_panel_open: bool,
    pub create_username_input: String,
    pub signin_username_input: String,
    pub deleted_learning_items: VecDeque<LearningItem>,
    pub deleted_messages: VecDeque<Message>,
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
            UIStateAction::SetCreateUsernameInput(input) => next.create_username_input = input,
            UIStateAction::SetSigninUsernameInput(input) => next.signin_username_input = input,
            UIStateAction::ClearCreateUsernameInput => next.create_username_input = String::new(),
            UIStateAction::ClearSigninUsernameInput => next.signin_username_input = String::new(),
            UIStateAction::PushDeletedLearningItem(item) => {
                next.deleted_learning_items.push_back(item);
                if next.deleted_learning_items.len() > 10 {
                    next.deleted_learning_items.pop_front();
                }
            }
            UIStateAction::PopDeletedLearningItem => {
                next.deleted_learning_items.pop_back();
            }
            UIStateAction::PushDeletedMessage(msg) => {
                next.deleted_messages.push_back(msg);
                if next.deleted_messages.len() > 10 {
                    next.deleted_messages.pop_front();
                }
            }
            UIStateAction::PopDeletedMessage => {
                next.deleted_messages.pop_back();
            }
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
    AddLearningItems(
        Vec<Mistake>,
        Vec<Explained>,
        Vec<Translated>,
        Vec<Exploratory>,
    ),
    UpdateScores(AgentAnalysis),
    ChangeDialect(Dialect),
    ChangeLanguage(Language),
    ChangeFormality(Formality),
    ChangeTeachingMode(TeachingMode),
    ToggleTTS,
    ReplaceUserState(UserState),
    ClearUserState,
    DeleteLearningItem(Uuid),
    UndoDeleteLearningItem(LearningItem),
    DeleteMessage(Uuid),
    UndoDeleteMessage(Message),
    CreateBranch(Uuid),
    SwitchBranch(Uuid),
    DeleteBranch(Uuid),
    RenameBranch(Uuid, String),
}

fn get_learning_item_id(item: &LearningItem) -> Uuid {
    match &item.item {
        LearningItemType::Mistake(m) => m.id,
        LearningItemType::Explanation(e) => e.id,
        LearningItemType::Translation(t) => t.id,
        LearningItemType::Exploration(e) => e.id,
    }
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
    items
        .into_iter()
        .map(|item| update_item_score(item, analysis))
        .collect()
}

fn delete_learning_item(mut items: Vec<LearningItem>, id: Uuid) -> Vec<LearningItem> {
    items.retain(|item| get_learning_item_id(item) != id);
    items
}

fn undo_delete_learning_item(
    mut items: Vec<LearningItem>,
    item: LearningItem,
) -> Vec<LearningItem> {
    items.push(item);
    items
}

fn delete_message(mut history: Vec<Message>, id: Uuid) -> Vec<Message> {
    history.retain(|msg| msg.id != id);
    history
}

fn undo_delete_message(mut history: Vec<Message>, msg: Message) -> Vec<Message> {
    history.push(msg);
    history
}

fn default_dialect_for_language(lang: Language) -> Dialect {
    match lang {
        Language::Spanish => Dialect::SpanishCuban,
        Language::Arabic => Dialect::ArabicEgyptian,
        Language::French => Dialect::FrenchParisian,
    }
}

fn remove_messages_on_path(mut messages: Vec<Message>, leaf_id: Uuid) -> Vec<Message> {
    let mut to_remove = Vec::new();
    let mut current_id = Some(leaf_id);

    while let Some(msg_id) = current_id {
        to_remove.push(msg_id);
        current_id = messages
            .iter()
            .find(|m| m.id == msg_id)
            .and_then(|m| m.parent_id);
    }

    messages.retain(|m| !to_remove.contains(&m.id));
    messages
}

fn apply_user_state_action(state: &UserState, action: UserStateAction) -> UserState {
    let mut next = state.clone();
    match action {
        UserStateAction::AddMessage(mut msg) => {
            let current_leaf = next
                .branches
                .iter()
                .find(|b| b.id == next.active_branch_id)
                .and_then(|b| b.leaf_message_id);

            msg.parent_id = current_leaf;
            let new_msg_id = msg.id;
            next.conversation_history.push(msg);

            if let Some(branch) = next
                .branches
                .iter_mut()
                .find(|b| b.id == next.active_branch_id)
            {
                branch.leaf_message_id = Some(new_msg_id);
            }
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
        UserStateAction::ReplaceUserState(new_state) => {
            next = new_state;
        }
        UserStateAction::DeleteLearningItem(id) => {
            next.learning_items = delete_learning_item(next.learning_items, id);
        }
        UserStateAction::UndoDeleteLearningItem(item) => {
            next.learning_items = undo_delete_learning_item(next.learning_items, item);
        }
        UserStateAction::DeleteMessage(id) => {
            next.conversation_history = delete_message(next.conversation_history, id);
        }
        UserStateAction::UndoDeleteMessage(msg) => {
            next.conversation_history = undo_delete_message(next.conversation_history, msg);
        }
        UserStateAction::CreateBranch(message_id) => {
            // Update the current branch's parent_message_id if it's None
            // This ensures both branches know where they diverged
            if let Some(current_branch) = next
                .branches
                .iter_mut()
                .find(|b| b.id == next.active_branch_id)
                && current_branch.parent_message_id.is_none()
            {
                current_branch.parent_message_id = Some(message_id);
            }

            // Create new branch
            let new_branch = ConversationBranch::new(Some(message_id), None, Some(message_id));
            let new_branch_id = new_branch.id;
            next.branches.push(new_branch);
            next.active_branch_id = new_branch_id;
        }
        UserStateAction::SwitchBranch(branch_id) => {
            next.active_branch_id = branch_id;
        }
        UserStateAction::DeleteBranch(branch_id) => {
            let leaf_id = next
                .branches
                .iter()
                .find(|b| b.id == branch_id)
                .and_then(|b| b.leaf_message_id);

            if let Some(leaf) = leaf_id {
                next.conversation_history =
                    remove_messages_on_path(next.conversation_history, leaf);
            }

            next.branches.retain(|b| b.id != branch_id);

            if next.active_branch_id == branch_id {
                next.active_branch_id = next
                    .branches
                    .first()
                    .map(|b| b.id)
                    .unwrap_or(next.active_branch_id);
            }
        }
        UserStateAction::RenameBranch(branch_id, name) => {
            if let Some(branch) = next.branches.iter_mut().find(|b| b.id == branch_id) {
                branch.name = Some(name);
            }
        }
        UserStateAction::ClearUserState => {
            // This should never be called - ClearUserState is handled at OptionalUserState level
            // But we need this case for exhaustiveness
            panic!("ClearUserState should not reach apply_user_state_action");
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

// Wrapper for optional user state - None until authenticated
#[derive(Clone, PartialEq)]
pub struct OptionalUserState(pub Option<UserState>);

impl Reducible for OptionalUserState {
    type Action = UserStateAction;

    fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
        match action {
            UserStateAction::ReplaceUserState(new_state) => {
                OptionalUserState(Some(new_state)).into()
            }
            UserStateAction::ClearUserState => OptionalUserState(None).into(),
            _ => match &self.0 {
                Some(state) => {
                    OptionalUserState(Some(apply_user_state_action(state, action))).into()
                }
                None => self,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dialect_coach_shared::models::MessageContent;
    use dialect_coach_shared::models::MessageMetadata;
    use dialect_coach_shared::models::{Dialect, Formality, Language, TeachingMode};

    fn create_test_message(session_id: Uuid, parent_id: Option<Uuid>) -> Message {
        Message::new(
            MessageContent::UserMessage {
                content: "test".to_string(),
            },
            MessageMetadata::at_now(
                Formality::Casual,
                TeachingMode::Immersive,
                Language::Spanish,
                Dialect::SpanishMexican,
                session_id,
            ),
            parent_id,
        )
    }

    #[test]
    fn test_remove_messages_on_path() {
        let session_id = Uuid::new_v4();

        let msg1 = create_test_message(session_id, None);
        let msg2 = create_test_message(session_id, Some(msg1.id));
        let msg3 = create_test_message(session_id, Some(msg2.id));

        let messages = vec![msg1.clone(), msg2.clone(), msg3.clone()];
        let result = remove_messages_on_path(messages, msg3.id);

        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_create_branch_reducer() {
        let mut state = UserState::new(Uuid::new_v4());
        let message_id = Uuid::new_v4();
        let initial_branch_count = state.branches.len();

        let action = UserStateAction::CreateBranch(message_id);
        state = apply_user_state_action(&state, action);

        assert_eq!(state.branches.len(), initial_branch_count + 1);
        let new_branch = state.branches.last().unwrap();
        assert_eq!(new_branch.parent_message_id, Some(message_id));
        assert_eq!(new_branch.leaf_message_id, Some(message_id));
        assert_eq!(state.active_branch_id, new_branch.id);
    }

    #[test]
    fn test_switch_branch_reducer() {
        let mut state = UserState::new(Uuid::new_v4());
        let new_branch_id = Uuid::new_v4();

        let action = UserStateAction::SwitchBranch(new_branch_id);
        state = apply_user_state_action(&state, action);

        assert_eq!(state.active_branch_id, new_branch_id);
    }

    #[test]
    fn test_delete_branch_reducer() {
        let mut state = UserState::new(Uuid::new_v4());

        let msg = create_test_message(Uuid::new_v4(), None);
        state.conversation_history.push(msg.clone());

        state.branches.push(ConversationBranch::new(
            None,
            Some("ToDelete".to_string()),
            Some(msg.id),
        ));
        let branch = state.branches.last().unwrap();
        let branch_id = branch.id;

        let initial_message_count = state.conversation_history.len();
        let action = UserStateAction::DeleteBranch(branch_id);
        state = apply_user_state_action(&state, action);

        assert!(!state.branches.iter().any(|b| b.id == branch_id));
        assert_eq!(state.conversation_history.len(), initial_message_count - 1);
    }

    #[test]
    fn test_delete_active_branch_switches_to_first() {
        let mut state = UserState::new(Uuid::new_v4());
        let first_branch_id = state.branches.first().unwrap().id;

        let new_branch = ConversationBranch::new(None, Some("NewBranch".to_string()), None);
        let new_branch_id = new_branch.id;
        state.branches.push(new_branch);
        state.active_branch_id = new_branch_id;

        let action = UserStateAction::DeleteBranch(new_branch_id);
        state = apply_user_state_action(&state, action);

        assert_eq!(state.active_branch_id, first_branch_id);
    }

    #[test]
    fn test_rename_branch_reducer() {
        let mut state = UserState::new(Uuid::new_v4());
        let branch_id = state.branches.first().unwrap().id;
        let new_name = "Renamed Branch".to_string();

        let action = UserStateAction::RenameBranch(branch_id, new_name.clone());
        state = apply_user_state_action(&state, action);

        let branch = state.branches.iter().find(|b| b.id == branch_id).unwrap();
        assert_eq!(branch.name, Some(new_name));
    }

    #[test]
    fn test_add_message_updates_leaf() {
        let mut state = UserState::new(Uuid::new_v4());
        let active_branch_id = state.active_branch_id;

        let msg1 = create_test_message(Uuid::new_v4(), None);
        let action1 = UserStateAction::AddMessage(msg1.clone());
        state = apply_user_state_action(&state, action1);

        let added_msg1 = state.conversation_history.last().unwrap();
        assert_eq!(added_msg1.parent_id, None);
        let msg1_id = added_msg1.id;

        let branch = state
            .branches
            .iter()
            .find(|b| b.id == active_branch_id)
            .unwrap();
        assert_eq!(branch.leaf_message_id, Some(msg1_id));

        let msg2 = create_test_message(Uuid::new_v4(), None);
        let action2 = UserStateAction::AddMessage(msg2);
        state = apply_user_state_action(&state, action2);

        let added_msg2 = state.conversation_history.last().unwrap();
        assert_eq!(added_msg2.parent_id, Some(msg1_id));

        let branch = state
            .branches
            .iter()
            .find(|b| b.id == active_branch_id)
            .unwrap();
        assert_eq!(branch.leaf_message_id, Some(added_msg2.id));
    }
}
