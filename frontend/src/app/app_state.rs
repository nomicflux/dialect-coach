pub mod callbacks;

use dialect_coach_shared::models::dialect::dialect_features;
use dialect_coach_shared::models::{
    ConversationBranch, Dialect, Formality, Language, Message, TeachingMode, UserGender,
    ArabicScript, JapaneseScript,
};
use dialect_coach_shared::{AgentAnalysis, Explained, Exploratory, Mistake, Translated};
use dialect_coach_shared::{LearningItem, LearningItemType, UsageStats, User, UserState};
use log::error;
use std::cell::RefCell;
use std::collections::{HashSet, VecDeque};
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
    pub autoplay_enabled: bool,
    pub rate_limit_state: RateLimitState,
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

    fn apply_action(&self, action: AppStateAction) -> Self {
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

pub enum UIStateAction {
    OpenPanel,
    ClosePanel,
    EnterInputPrompt(String),
    ClearInputPrompt,
    PushTranslatingButton(String),
    ClearTranslatingButton,
    OpenLearningPanel,
    CloseLearningPanel,
    ToggleSidebar,
    ToggleLearningPanel,
    ToggleUsageFooter,
    SetCreateUsernameInput(String),
    SetSigninUsernameInput(String),
    SetCreateEmailInput(String),
    SetCreateInviteCodeInput(String),
    SetSigninInviteCodeInput(String),
    ClearCreateUsernameInput,
    ClearSigninUsernameInput,
    ClearCreateEmailInput,
    ClearCreateInviteCodeInput,
    ClearSigninInviteCodeInput,
    PushDeletedLearningItem(LearningItem),
    PopDeletedLearningItem,
    PushDeletedMessage(Message),
    PopDeletedMessage,
    ShowUserCreationPage,
    HideUserCreationPage,
}

#[derive(Clone)]
pub struct UIState {
    pub panel_open: bool,
    pub input_prompt_value: Option<String>,
    pub translating_button: Option<String>,
    pub learning_panel_open: bool,
    pub sidebar_collapsed: bool,
    pub learning_panel_collapsed: bool,
    pub usage_footer_collapsed: bool,
    pub create_username_input: String,
    pub signin_username_input: String,
    pub create_email_input: String,
    pub create_invite_code_input: String,
    pub signin_invite_code_input: String,
    pub deleted_learning_items: VecDeque<LearningItem>,
    pub deleted_messages: VecDeque<Message>,
    pub show_user_creation_page: bool,
}

impl Default for UIState {
    fn default() -> Self {
        Self {
            panel_open: false,
            input_prompt_value: None,
            translating_button: None,
            learning_panel_open: false,
            sidebar_collapsed: false,
            learning_panel_collapsed: false,
            usage_footer_collapsed: true,
            create_username_input: String::new(),
            signin_username_input: String::new(),
            create_email_input: String::new(),
            create_invite_code_input: String::new(),
            signin_invite_code_input: String::new(),
            deleted_learning_items: VecDeque::new(),
            deleted_messages: VecDeque::new(),
            show_user_creation_page: false,
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
            UIStateAction::ToggleSidebar => next.sidebar_collapsed = !next.sidebar_collapsed,
            UIStateAction::ToggleLearningPanel => {
                next.learning_panel_collapsed = !next.learning_panel_collapsed
            }
            UIStateAction::ToggleUsageFooter => {
                next.usage_footer_collapsed = !next.usage_footer_collapsed
            }
            UIStateAction::SetCreateUsernameInput(input) => next.create_username_input = input,
            UIStateAction::SetSigninUsernameInput(input) => next.signin_username_input = input,
            UIStateAction::SetCreateEmailInput(input) => next.create_email_input = input,
            UIStateAction::SetCreateInviteCodeInput(input) => next.create_invite_code_input = input,
            UIStateAction::SetSigninInviteCodeInput(input) => next.signin_invite_code_input = input,
            UIStateAction::ClearCreateUsernameInput => next.create_username_input = String::new(),
            UIStateAction::ClearSigninUsernameInput => next.signin_username_input = String::new(),
            UIStateAction::ClearCreateEmailInput => next.create_email_input = String::new(),
            UIStateAction::ClearCreateInviteCodeInput => {
                next.create_invite_code_input = String::new()
            }
            UIStateAction::ClearSigninInviteCodeInput => {
                next.signin_invite_code_input = String::new()
            }
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
            UIStateAction::ShowUserCreationPage => {
                next.show_user_creation_page = true;
            }
            UIStateAction::HideUserCreationPage => {
                next.show_user_creation_page = false;
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
    UpdateUserGender(UserGender),
    ToggleTTS,
    ReplaceUserState(UserState),
    UpdateUsageStats(UsageStats),
    ClearUserState,
    DeleteLearningItem(Uuid),
    UndoDeleteLearningItem(LearningItem),
    DeleteMessage(Uuid),
    UndoDeleteMessage(Message),
    CreateBranch(Uuid),
    SwitchBranch(Uuid),
    DeleteBranch(Uuid),
    RenameBranch(Uuid, String),
    AddLearningGoal(String),
    DeleteLearningGoal(usize),
    CycleDialect,
    CycleFormality,
    CycleTeachingMode,
    SetArabicScript(ArabicScript),
    SetJapaneseScript(JapaneseScript),
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
    dialect: Dialect,
) -> Vec<LearningItem> {
    for mistake in mistakes {
        items.push(LearningItem::new(LearningItemType::Mistake(mistake), dialect));
    }
    for expl in explained {
        items.push(LearningItem::new(LearningItemType::Explanation(expl), dialect));
    }
    for trans in translated {
        items.push(LearningItem::new(LearningItemType::Translation(trans), dialect));
    }
    for explor in exploratory {
        items.push(LearningItem::new(LearningItemType::Exploration(explor), dialect));
    }
    items
}

fn update_item_score(mut item: LearningItem, analysis: &AgentAnalysis) -> LearningItem {
    match &item.item {
        LearningItemType::Mistake(m) => {
            if let Some(score_obj) = analysis.mistake_scores.get(&m.id) {
                item.score = add_score(item.score, score_obj.score);
            }
        }
        LearningItemType::Explanation(e) => {
            if let Some(score_obj) = analysis.explained_scores.get(&e.id) {
                item.score = add_score(item.score, score_obj.score);
            }
        }
        LearningItemType::Translation(t) => {
            if let Some(score_obj) = analysis.translated_scores.get(&t.id) {
                item.score = add_score(item.score, score_obj.score);
            }
        }
        LearningItemType::Exploration(e) => {
            if let Some(score_obj) = analysis.exploratory_scores.get(&e.id) {
                item.score = add_score(item.score, score_obj.score);
            }
        }
    }
    item
}

fn add_score(current: u8, delta: i8) -> u8 {
    (current as i32 + delta as i32).clamp(0, 100) as u8
}

fn cycle_dialect(state: &UserState) -> Dialect {
    let dialects = state.current_dialects();
    let current = state.current_dialect();
    match dialects.iter().position(|df| df.dialect == current) {
        Some(idx) => dialects[(idx + 1) % dialects.len()].dialect,
        None => current,
    }
}

fn cycle_formality(current: Formality) -> Formality {
    match current {
        Formality::Formal => Formality::ProfessionalCasual,
        Formality::ProfessionalCasual => Formality::Informal,
        Formality::Informal => Formality::Slang,
        Formality::Slang => Formality::Formal,
    }
}

fn cycle_teaching_mode(current: TeachingMode) -> TeachingMode {
    match current {
        TeachingMode::Immersive => TeachingMode::Corrective,
        TeachingMode::Corrective => TeachingMode::Explanatory,
        TeachingMode::Explanatory => TeachingMode::Interleaved,
        TeachingMode::Interleaved => TeachingMode::StoryTeller,
        TeachingMode::StoryTeller => TeachingMode::Debug,
        TeachingMode::Debug => TeachingMode::Immersive,
    }
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

fn add_learning_goal(mut goals: Vec<String>, goal: String) -> Vec<String> {
    goals.push(goal);
    goals
}

fn delete_learning_goal(mut goals: Vec<String>, index: usize) -> Vec<String> {
    goals.remove(index);
    goals
}

fn default_dialect_for_language(lang: Language) -> Dialect {
    match lang {
        Language::Spanish => Dialect::SpanishCuban,
        Language::Arabic => Dialect::ArabicEgyptian,
        Language::French => Dialect::FrenchParisian,
        Language::English => Dialect::EnglishGeneralAmerican,
        Language::Japanese => Dialect::JapaneseTokyo,
    }
}

fn get_messages_in_branch_path(
    messages: &[Message],
    branch_point_id: Option<Uuid>,
    leaf_id: Uuid,
) -> HashSet<Uuid> {
    let mut ids = HashSet::new();
    let mut current = Some(leaf_id);

    while let Some(msg_id) = current {
        if Some(msg_id) == branch_point_id {
            break;
        }
        ids.insert(msg_id);
        current = messages
            .iter()
            .find(|m| m.id == msg_id)
            .and_then(|m| m.parent_id);
    }

    ids
}

fn get_all_descendants(messages: &[Message], parent_ids: &HashSet<Uuid>) -> HashSet<Uuid> {
    let mut descendants = HashSet::new();
    let mut to_check: Vec<Uuid> = parent_ids.iter().copied().collect();

    while let Some(parent_id) = to_check.pop() {
        for msg in messages {
            if msg.parent_id == Some(parent_id) {
                descendants.insert(msg.id);
                to_check.push(msg.id);
            }
        }
    }

    descendants
}

fn remove_branch_messages(
    mut messages: Vec<Message>,
    branch_point_id: Option<Uuid>,
    leaf_id: Uuid,
) -> Vec<Message> {
    let branch_ids = get_messages_in_branch_path(&messages, branch_point_id, leaf_id);
    let descendant_ids = get_all_descendants(&messages, &branch_ids);
    let mut all_ids = branch_ids;
    all_ids.extend(descendant_ids);
    messages.retain(|m| !all_ids.contains(&m.id));
    messages
}

fn prepare_state_for_action(state: &UserState) -> UserState {
    let mut prepared = state.clone();
    prepared.rebuild_branches_from_history();
    prepared
}

fn create_new_branch_for_language(state: &mut UserState) {
    let new_branch = ConversationBranch::new(None, None, None, None);
    let new_branch_id = new_branch.id;
    state.branches.push(new_branch);
    state.active_branch_id = new_branch_id;
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
            let msg_dialect = msg.metadata.dialect;
            next.conversation_history.push(msg);

            if let Some(branch) = next
                .branches
                .iter_mut()
                .find(|b| b.id == next.active_branch_id)
            {
                branch.leaf_message_id = Some(new_msg_id);
                // Set branch dialect from first message if dialect is None
                if branch.dialect.is_none() {
                    branch.dialect = Some(msg_dialect);
                }
            }
        }
        UserStateAction::AddLearningItems(mistakes, explained, translated, exploratory) => {
            next.learning_items = add_learning_items_to_vec(
                next.learning_items,
                mistakes,
                explained,
                translated,
                exploratory,
                next.selected_dialect,
            );
        }
        UserStateAction::UpdateScores(analysis) => {
            next.learning_items = apply_score_updates(next.learning_items, &analysis);
        }
        UserStateAction::ChangeDialect(dialect) => {
            let old_language = next.selected_dialect.language();
            next.selected_dialect = dialect;
            let new_language = dialect.language();

            if old_language != new_language {
                create_new_branch_for_language(&mut next);
            }
        }
        UserStateAction::ChangeLanguage(language) => {
            let old_language = next.selected_language;
            next.selected_language = language;
            next.selected_dialect = default_dialect_for_language(language);

            if old_language != language {
                create_new_branch_for_language(&mut next);
            }
        }
        UserStateAction::ChangeFormality(formality) => {
            next.formality = formality;
        }
        UserStateAction::ChangeTeachingMode(tm) => {
            next.teaching_mode = tm;
        }
        UserStateAction::UpdateUserGender(gender) => {
            next.user_gender = gender;
        }
        UserStateAction::ToggleTTS => {
            next.tts_enabled = !next.tts_enabled;
        }
        UserStateAction::ReplaceUserState(new_state) => {
            next = new_state;
        }
        UserStateAction::UpdateUsageStats(new_stats) => {
            next.usage_stats = new_stats;
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

            // Create new branch with dialect from the branching message
            let dialect = next.conversation_history
                .iter()
                .find(|m| m.id == message_id)
                .map(|m| m.metadata.dialect);
            let new_branch = ConversationBranch::new(Some(message_id), None, Some(message_id), dialect);
            let new_branch_id = new_branch.id;
            next.branches.push(new_branch);
            next.active_branch_id = new_branch_id;
        }
        UserStateAction::SwitchBranch(branch_id) => {
            next.active_branch_id = branch_id;

            // Update selected_dialect to match the branch's dialect
            if let Some(branch) = next.branches.iter().find(|b| b.id == branch_id)
                && let Some(dialect) = branch.dialect
            {
                next.selected_dialect = dialect;
                next.selected_language = dialect.language();
            }
        }
        UserStateAction::DeleteBranch(branch_id) => {
            let branch_data = next
                .branches
                .iter()
                .find(|b| b.id == branch_id)
                .map(|b| (b.parent_message_id, b.leaf_message_id));

            if let Some((parent_id, Some(leaf))) = branch_data {
                next.conversation_history =
                    remove_branch_messages(next.conversation_history, parent_id, leaf);
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
        UserStateAction::AddLearningGoal(goal) => {
            next.learning_goals = add_learning_goal(next.learning_goals, goal);
        }
        UserStateAction::DeleteLearningGoal(index) => {
            next.learning_goals = delete_learning_goal(next.learning_goals, index);
        }
        UserStateAction::CycleDialect => {
            let old_language = next.selected_dialect.language();
            next.selected_dialect = cycle_dialect(&next);
            let new_language = next.selected_dialect.language();

            if old_language != new_language {
                create_new_branch_for_language(&mut next);
            }
        }
        UserStateAction::CycleFormality => {
            next.formality = cycle_formality(next.formality);
        }
        UserStateAction::CycleTeachingMode => {
            next.teaching_mode = cycle_teaching_mode(next.teaching_mode);
        }
        UserStateAction::SetArabicScript(script) => {
            next.language_options.arabic_script = Some(script);
        }
        UserStateAction::SetJapaneseScript(script) => {
            next.language_options.japanese_script = Some(script);
        }
        UserStateAction::ClearUserState => {
            // This should never be called - ClearUserState is handled at OptionalUserState level
            // But we need this case for exhaustiveness
            panic!("ClearUserState should not reach apply_user_state_action");
        }
    }
    next
}

#[derive(Clone, PartialEq)]
pub struct OptionalUserState(pub Option<UserState>);

impl Reducible for OptionalUserState {
    type Action = UserStateAction;

    fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
        match action {
            UserStateAction::ReplaceUserState(mut new_state) => {
                new_state.rebuild_branches_from_history();
                OptionalUserState(Some(new_state)).into()
            }
            UserStateAction::ClearUserState => OptionalUserState(None).into(),
            _ => match &self.0 {
                Some(state) => {
                    let prepared = prepare_state_for_action(state);
                    OptionalUserState(Some(apply_user_state_action(&prepared, action))).into()
                }
                None => self,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dialect_coach_shared::models::MessageMetadata;
    use dialect_coach_shared::models::{Dialect, Formality, Language, TeachingMode};
    use std::rc::Rc;

    fn create_test_message(session_id: Uuid, parent_id: Option<Uuid>) -> Message {
        Message::user_message(
            "test".to_string(),
            MessageMetadata::at_now(
                Formality::Informal,
                TeachingMode::Immersive,
                Language::Spanish,
                Dialect::SpanishMexican,
                session_id,
            ),
            parent_id,
        )
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
            Some(Dialect::SpanishMexican),
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

        let new_branch = ConversationBranch::new(None, Some("NewBranch".to_string()), None, Some(Dialect::SpanishMexican));
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

    #[test]
    fn test_simple_branch_deletion() {
        let session_id = Uuid::new_v4();
        let mut state = UserState::new(session_id);

        // Create A→B
        let msg_a = create_test_message(session_id, None);
        state.conversation_history.push(msg_a.clone());
        let msg_b = create_test_message(session_id, Some(msg_a.id));
        state.conversation_history.push(msg_b.clone());

        // Update initial branch to point to B
        state.branches[0].leaf_message_id = Some(msg_b.id);

        // Create branch C→D from B
        let branch_cd = ConversationBranch::new(Some(msg_b.id), Some("C+D".to_string()), None, Some(Dialect::SpanishMexican));
        let branch_cd_id = branch_cd.id;
        state.branches.push(branch_cd);

        let msg_c = create_test_message(session_id, Some(msg_b.id));
        state.conversation_history.push(msg_c.clone());
        let msg_d = create_test_message(session_id, Some(msg_c.id));
        state.conversation_history.push(msg_d.clone());

        state
            .branches
            .iter_mut()
            .find(|b| b.id == branch_cd_id)
            .unwrap()
            .leaf_message_id = Some(msg_d.id);

        // Create branch X→Y from B (make active)
        let branch_xy = ConversationBranch::new(Some(msg_b.id), Some("X+Y".to_string()), None, Some(Dialect::SpanishMexican));
        let branch_xy_id = branch_xy.id;
        state.branches.push(branch_xy);
        state.active_branch_id = branch_xy_id;

        let msg_x = create_test_message(session_id, Some(msg_b.id));
        state.conversation_history.push(msg_x.clone());
        let msg_y = create_test_message(session_id, Some(msg_x.id));
        state.conversation_history.push(msg_y.clone());

        state
            .branches
            .iter_mut()
            .find(|b| b.id == branch_xy_id)
            .unwrap()
            .leaf_message_id = Some(msg_y.id);

        // Delete branch C+D
        let action = UserStateAction::DeleteBranch(branch_cd_id);
        state = apply_user_state_action(&state, action);

        // Assert: A, B, X, Y remain; C, D are deleted
        assert_eq!(state.conversation_history.len(), 4);
        assert!(state.conversation_history.iter().any(|m| m.id == msg_a.id));
        assert!(state.conversation_history.iter().any(|m| m.id == msg_b.id));
        assert!(state.conversation_history.iter().any(|m| m.id == msg_x.id));
        assert!(state.conversation_history.iter().any(|m| m.id == msg_y.id));
        assert!(!state.conversation_history.iter().any(|m| m.id == msg_c.id));
        assert!(!state.conversation_history.iter().any(|m| m.id == msg_d.id));
        assert!(!state.branches.iter().any(|b| b.id == branch_cd_id));
    }

    #[test]
    fn test_complex_nested_branch_deletion() {
        let session_id = Uuid::new_v4();
        let mut state = UserState::new(session_id);

        // Create A→B→X→W
        let msg_a = create_test_message(session_id, None);
        state.conversation_history.push(msg_a.clone());
        let msg_b = create_test_message(session_id, Some(msg_a.id));
        state.conversation_history.push(msg_b.clone());
        let msg_x = create_test_message(session_id, Some(msg_b.id));
        state.conversation_history.push(msg_x.clone());
        let msg_w = create_test_message(session_id, Some(msg_x.id));
        state.conversation_history.push(msg_w.clone());

        state.branches[0].leaf_message_id = Some(msg_w.id);

        // Create branch C→D from B
        let branch_cd = ConversationBranch::new(Some(msg_b.id), Some("C+D".to_string()), None, Some(Dialect::SpanishMexican));
        let branch_cd_id = branch_cd.id;
        state.branches.push(branch_cd);

        let msg_c = create_test_message(session_id, Some(msg_b.id));
        state.conversation_history.push(msg_c.clone());
        let msg_d = create_test_message(session_id, Some(msg_c.id));
        state.conversation_history.push(msg_d.clone());

        state
            .branches
            .iter_mut()
            .find(|b| b.id == branch_cd_id)
            .unwrap()
            .leaf_message_id = Some(msg_d.id);

        // Create branch Y from X
        let branch_y = ConversationBranch::new(Some(msg_x.id), Some("Y".to_string()), None, Some(Dialect::SpanishMexican));
        let branch_y_id = branch_y.id;
        state.branches.push(branch_y);

        let msg_y = create_test_message(session_id, Some(msg_x.id));
        state.conversation_history.push(msg_y.clone());

        state
            .branches
            .iter_mut()
            .find(|b| b.id == branch_y_id)
            .unwrap()
            .leaf_message_id = Some(msg_y.id);

        // Delete branch Y
        let action = UserStateAction::DeleteBranch(branch_y_id);
        state = apply_user_state_action(&state, action);

        // Assert: A, B, C, D, X, W remain; Y deleted
        assert_eq!(state.conversation_history.len(), 6);
        assert!(state.conversation_history.iter().any(|m| m.id == msg_a.id));
        assert!(state.conversation_history.iter().any(|m| m.id == msg_b.id));
        assert!(state.conversation_history.iter().any(|m| m.id == msg_c.id));
        assert!(state.conversation_history.iter().any(|m| m.id == msg_d.id));
        assert!(state.conversation_history.iter().any(|m| m.id == msg_x.id));
        assert!(state.conversation_history.iter().any(|m| m.id == msg_w.id));
        assert!(!state.conversation_history.iter().any(|m| m.id == msg_y.id));
        assert!(!state.branches.iter().any(|b| b.id == branch_y_id));

        // Delete branch C+D
        let action2 = UserStateAction::DeleteBranch(branch_cd_id);
        state = apply_user_state_action(&state, action2);

        // Assert: A, B, X, W remain; C, D deleted
        assert_eq!(state.conversation_history.len(), 4);
        assert!(state.conversation_history.iter().any(|m| m.id == msg_a.id));
        assert!(state.conversation_history.iter().any(|m| m.id == msg_b.id));
        assert!(state.conversation_history.iter().any(|m| m.id == msg_x.id));
        assert!(state.conversation_history.iter().any(|m| m.id == msg_w.id));
        assert!(!state.conversation_history.iter().any(|m| m.id == msg_c.id));
        assert!(!state.conversation_history.iter().any(|m| m.id == msg_d.id));
        assert!(!state.branches.iter().any(|b| b.id == branch_cd_id));
    }

    #[test]
    fn test_delete_branch_with_sub_branches() {
        let session_id = Uuid::new_v4();
        let mut state = UserState::new(session_id);

        // Create A→B→C
        let msg_a = create_test_message(session_id, None);
        state.conversation_history.push(msg_a.clone());
        let msg_b = create_test_message(session_id, Some(msg_a.id));
        state.conversation_history.push(msg_b.clone());
        let msg_c = create_test_message(session_id, Some(msg_b.id));
        state.conversation_history.push(msg_c.clone());

        state.branches[0].leaf_message_id = Some(msg_c.id);

        // Create branch D→E→F from C
        let branch_def = ConversationBranch::new(Some(msg_c.id), Some("D+E+F".to_string()), None, Some(Dialect::SpanishMexican));
        let branch_def_id = branch_def.id;
        state.branches.push(branch_def);

        let msg_d = create_test_message(session_id, Some(msg_c.id));
        state.conversation_history.push(msg_d.clone());
        let msg_e = create_test_message(session_id, Some(msg_d.id));
        state.conversation_history.push(msg_e.clone());
        let msg_f = create_test_message(session_id, Some(msg_e.id));
        state.conversation_history.push(msg_f.clone());

        state
            .branches
            .iter_mut()
            .find(|b| b.id == branch_def_id)
            .unwrap()
            .leaf_message_id = Some(msg_f.id);

        // Delete branch D→E→F
        let action = UserStateAction::DeleteBranch(branch_def_id);
        state = apply_user_state_action(&state, action);

        // Assert: A, B, C remain; D, E, F deleted
        assert_eq!(state.conversation_history.len(), 3);
        assert!(state.conversation_history.iter().any(|m| m.id == msg_a.id));
        assert!(state.conversation_history.iter().any(|m| m.id == msg_b.id));
        assert!(state.conversation_history.iter().any(|m| m.id == msg_c.id));
        assert!(!state.conversation_history.iter().any(|m| m.id == msg_d.id));
        assert!(!state.conversation_history.iter().any(|m| m.id == msg_e.id));
        assert!(!state.conversation_history.iter().any(|m| m.id == msg_f.id));
        assert!(!state.branches.iter().any(|b| b.id == branch_def_id));
    }

    #[test]
    fn test_delete_inactive_branch_preserves_active() {
        let session_id = Uuid::new_v4();
        let mut state = UserState::new(session_id);

        // Create A→B
        let msg_a = create_test_message(session_id, None);
        state.conversation_history.push(msg_a.clone());
        let msg_b = create_test_message(session_id, Some(msg_a.id));
        state.conversation_history.push(msg_b.clone());

        state.branches[0].leaf_message_id = Some(msg_b.id);

        // Create branch C→D from B (inactive)
        let branch_cd = ConversationBranch::new(Some(msg_b.id), Some("Inactive".to_string()), None, Some(Dialect::SpanishMexican));
        let branch_cd_id = branch_cd.id;
        state.branches.push(branch_cd);

        let msg_c = create_test_message(session_id, Some(msg_b.id));
        state.conversation_history.push(msg_c.clone());
        let msg_d = create_test_message(session_id, Some(msg_c.id));
        state.conversation_history.push(msg_d.clone());

        state
            .branches
            .iter_mut()
            .find(|b| b.id == branch_cd_id)
            .unwrap()
            .leaf_message_id = Some(msg_d.id);

        // Create branch X→Y from B (make active)
        let branch_xy = ConversationBranch::new(Some(msg_b.id), Some("Active".to_string()), None, Some(Dialect::SpanishMexican));
        let branch_xy_id = branch_xy.id;
        state.branches.push(branch_xy);
        state.active_branch_id = branch_xy_id;

        let msg_x = create_test_message(session_id, Some(msg_b.id));
        state.conversation_history.push(msg_x.clone());
        let msg_y = create_test_message(session_id, Some(msg_x.id));
        state.conversation_history.push(msg_y.clone());

        state
            .branches
            .iter_mut()
            .find(|b| b.id == branch_xy_id)
            .unwrap()
            .leaf_message_id = Some(msg_y.id);

        // Get active branch messages before deletion
        let active_before: Vec<_> = state
            .get_active_branch_messages()
            .iter()
            .map(|m| m.id)
            .collect();

        // Delete the inactive C→D branch
        let action = UserStateAction::DeleteBranch(branch_cd_id);
        state = apply_user_state_action(&state, action);

        // Get active branch messages after deletion
        let active_after: Vec<_> = state
            .get_active_branch_messages()
            .iter()
            .map(|m| m.id)
            .collect();

        // Assert: Active branch messages unchanged
        assert_eq!(active_before, active_after);
        assert_eq!(state.active_branch_id, branch_xy_id);
        assert!(state.conversation_history.iter().any(|m| m.id == msg_a.id));
        assert!(state.conversation_history.iter().any(|m| m.id == msg_b.id));
        assert!(state.conversation_history.iter().any(|m| m.id == msg_x.id));
        assert!(state.conversation_history.iter().any(|m| m.id == msg_y.id));
        assert!(!state.conversation_history.iter().any(|m| m.id == msg_c.id));
        assert!(!state.conversation_history.iter().any(|m| m.id == msg_d.id));
    }

    #[test]
    fn test_update_usage_stats_preserves_conversation_branches() {
        let mut state = UserState::new(Uuid::new_v4());
        let msg = create_test_message(Uuid::new_v4(), None);
        state = apply_user_state_action(&state, UserStateAction::AddMessage(msg.clone()));
        let history_len = state.conversation_history.len();
        let branch_ids: Vec<Uuid> = state.branches.iter().map(|b| b.id).collect();

        let updated = apply_user_state_action(
            &state,
            UserStateAction::UpdateUsageStats(UsageStats::default()),
        );

        assert_eq!(updated.conversation_history.len(), history_len);
        let updated_ids: Vec<Uuid> = updated.branches.iter().map(|b| b.id).collect();
        assert_eq!(updated_ids, branch_ids);
    }

    #[test]
    fn test_optional_user_state_rebuilds_missing_branches_before_update() {
        let mut base = UserState::new(Uuid::new_v4());
        let msg = create_test_message(Uuid::new_v4(), None);
        base.conversation_history.push(msg.clone());
        base.branches.clear();
        base.active_branch_id = Uuid::new_v4();

        let optional = Rc::new(OptionalUserState(Some(base)));
        let updated = OptionalUserState::reduce(
            optional,
            UserStateAction::UpdateUsageStats(UsageStats::default()),
        );
        let updated_state = updated.0.as_ref().unwrap();

        assert!(!updated_state.branches.is_empty());
        assert!(
            updated_state
                .branches
                .iter()
                .any(|branch| branch.leaf_message_id == Some(msg.id))
        );
    }

    #[test]
    fn test_add_score_accumulates() {
        assert_eq!(add_score(10, 8), 18);
        assert_eq!(add_score(50, 10), 60);
    }

    #[test]
    fn test_add_score_caps_at_100() {
        assert_eq!(add_score(95, 10), 100);
        assert_eq!(add_score(100, 10), 100);
    }

    #[test]
    fn test_add_score_handles_negative() {
        assert_eq!(add_score(20, -10), 10);
        assert_eq!(add_score(15, -10), 5);
    }

    #[test]
    fn test_add_score_floors_at_zero() {
        assert_eq!(add_score(0, -5), 0);
        assert_eq!(add_score(3, -10), 0);
    }

    #[test]
    fn test_cycle_formality() {
        assert_eq!(
            cycle_formality(Formality::Formal),
            Formality::ProfessionalCasual
        );
        assert_eq!(
            cycle_formality(Formality::ProfessionalCasual),
            Formality::Informal
        );
        assert_eq!(cycle_formality(Formality::Informal), Formality::Slang);
        assert_eq!(cycle_formality(Formality::Slang), Formality::Formal);
    }

    #[test]
    fn test_cycle_teaching_mode() {
        assert_eq!(
            cycle_teaching_mode(TeachingMode::Immersive),
            TeachingMode::Corrective
        );
        assert_eq!(
            cycle_teaching_mode(TeachingMode::Corrective),
            TeachingMode::Explanatory
        );
        assert_eq!(
            cycle_teaching_mode(TeachingMode::Explanatory),
            TeachingMode::Interleaved
        );
        assert_eq!(
            cycle_teaching_mode(TeachingMode::Interleaved),
            TeachingMode::StoryTeller
        );
        assert_eq!(
            cycle_teaching_mode(TeachingMode::StoryTeller),
            TeachingMode::Debug
        );
        assert_eq!(
            cycle_teaching_mode(TeachingMode::Debug),
            TeachingMode::Immersive
        );
    }

    #[test]
    fn test_cycle_dialect_action() {
        let mut state = UserState::new(Uuid::new_v4());
        state.selected_language = Language::Spanish;
        state.selected_dialect = Dialect::SpanishMexican;

        let action = UserStateAction::CycleDialect;
        state = apply_user_state_action(&state, action);

        assert_ne!(state.selected_dialect, Dialect::SpanishMexican);
    }

    #[test]
    fn test_cycle_formality_action() {
        let mut state = UserState::new(Uuid::new_v4());
        state.formality = Formality::Formal;

        let action = UserStateAction::CycleFormality;
        state = apply_user_state_action(&state, action);

        assert_eq!(state.formality, Formality::ProfessionalCasual);
    }

    #[test]
    fn test_cycle_teaching_mode_action() {
        let mut state = UserState::new(Uuid::new_v4());
        state.teaching_mode = TeachingMode::Immersive;

        let action = UserStateAction::CycleTeachingMode;
        state = apply_user_state_action(&state, action);

        assert_eq!(state.teaching_mode, TeachingMode::Corrective);
    }
}
