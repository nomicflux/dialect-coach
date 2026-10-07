use crate::app::app_callbacks::{
    on_auto_start, on_continue_branch, on_explain_message, on_send_message,
};
use crate::app::app_helpers::render_message_undo_notification;
use crate::app::app_state::callbacks::on_replay_message;
use crate::app::app_state::user::{BranchAction, UserDomainAction};
use crate::app::app_state::{
    AppState, LearningAction, SessionAction, SessionState, SettingsAction, UIState, UIStateAction,
    UserStateGamificationExt,
};
use crate::app::user_state_callbacks::{
    on_create_branch, on_delete_message_callback, on_undo_message_callback,
};
use crate::components::study_drawer_content::DrawerTab;
use crate::components::{
    ChatWindow, Drawer, DynamicIsland, InputBox, LearningItemsFlash, SelectionModal,
    SelectionResult, StudyDrawerContent, TranslateSelectionButton,
    translate_selection_button::SelectionAction,
};
use crate::keyboard_shortcuts::{ShortcutAction, default_shortcuts, matches_binding};
use crate::utils::perf::PerfGuard;
use dialect_coach_shared::{
    UserState,
    models::{
        ConversationBranch, Explained, LearningGoal, LearningItem, Message, Quest, Translated,
    },
};
use gloo::events::EventListener;
use std::{rc::Rc, sync::Arc};
use uuid::Uuid;
use wasm_bindgen::JsCast;
use yew::prelude::*;

#[derive(Clone, PartialEq)]
pub enum SelectionModalState {
    Loading {
        original_text: String,
    },
    Loaded {
        original_text: String,
        result: SelectionResult,
    },
}

#[derive(Properties, Clone, PartialEq)]
pub struct MainContentProps {
    pub app_state: UseReducerHandle<AppState>,
    pub ui_state: UseReducerHandle<UIState>,
    pub session: UseReducerHandle<SessionState>,
}

#[derive(Clone, PartialEq)]
struct MainMemos {
    items: Rc<Vec<LearningItem>>,
    goals: Arc<Vec<LearningGoal>>,
}

#[derive(Clone, PartialEq)]
struct HistoryMemos {
    branches: Arc<Vec<ConversationBranch>>,
    messages: Arc<Vec<Message>>,
}

#[derive(Clone, PartialEq)]
struct MainContext {
    props: MainContentProps,
    user: Rc<UserState>,
    dispatch: Callback<UserDomainAction>,
    chat_input_ref: NodeRef,
    goal_input_ref: NodeRef,
    data: MainMemos,
    history: HistoryMemos,
}

type ModalState = UseStateHandle<Option<SelectionModalState>>;
type SavePhrase = (String, String, String);
type SelectionRequest = (Uuid, SelectionAction, String, String);
type TabState = UseStateHandle<DrawerTab>;

struct ModalCallbacks {
    state: ModalState,
    close: Callback<()>,
    save: Callback<SavePhrase>,
    action: Callback<SelectionRequest>,
}

struct DrawerCallbacks {
    active_tab: UseStateHandle<DrawerTab>,
    close: Callback<()>,
    change: Callback<DrawerTab>,
}

struct LearningCallbacks {
    delete: Callback<Uuid>,
    dismiss: Callback<Uuid>,
    undo: Callback<()>,
}

struct MainCallbacks {
    modal: ModalCallbacks,
    goals: (Callback<String>, Callback<usize>),
    branches: (Callback<Uuid>, Callback<Uuid>),
    learning: LearningCallbacks,
    drawer: DrawerCallbacks,
}

#[function_component(MainContent)]
pub fn main_content(props: &MainContentProps) -> Html {
    let _render_guard = PerfGuard::new("MainContent::render");
    let user = use_memo(props.session.clone(), |session| {
        session.user.as_ref().map(|u| Rc::new(u.clone()))
    });
    let Some(user) = user.as_ref() else {
        return html! {};
    };
    let ctx = use_main_context(props.clone(), user.clone());
    let callbacks = use_main_callbacks(&ctx);
    render_main_content(&ctx, &callbacks)
}

fn render_main_content(ctx: &MainContext, callbacks: &MainCallbacks) -> Html {
    html! {
        <>
            {render_chat_canvas(ctx, callbacks)}
            {render_drawer(ctx, callbacks)}
            {render_modal(&callbacks.modal.state, &callbacks.modal.close, &callbacks.modal.save)}
            <LearningItemsFlash items={ctx.props.ui_state.flashed_items.clone()} on_dismiss={callbacks.learning.dismiss.clone()} />
        </>
    }
}

#[hook]
fn use_main_context(props: MainContentProps, user: Rc<UserState>) -> MainContext {
    let dispatch = use_callback(props.session.clone(), |action, session| {
        session.dispatch(SessionAction::Domain(action))
    });
    let chat_input_ref = use_node_ref();
    let goal_input_ref = use_node_ref();
    let data = use_main_memos(user.clone());
    let history = use_history_memos(user.clone());
    MainContext {
        props,
        user,
        dispatch,
        chat_input_ref,
        goal_input_ref,
        data,
        history,
    }
}

#[hook]
fn use_main_memos(user: Rc<UserState>) -> MainMemos {
    let items = use_filtered_items(user.clone());
    let goals = use_memo(user, |user| {
        Arc::new(
            user.get_learning_goals_for_dialect(&user.active_branch_dialect())
                .into_iter()
                .cloned()
                .collect::<Vec<LearningGoal>>(),
        )
    });
    MainMemos {
        items,
        goals: (*goals).clone(),
    }
}

#[hook]
fn use_filtered_items(user: Rc<UserState>) -> Rc<Vec<LearningItem>> {
    use_memo(user, |user| {
        let dialect = user.active_branch_dialect();
        let items = user
            .get_learning_items_for_dialect(&dialect)
            .into_iter()
            .cloned()
            .collect::<Vec<_>>();
        gloo::console::log!(
            "get_filtered_items count:",
            items.len(),
            "for dialect:",
            dialect.to_string()
        );
        items
    })
}

#[hook]
fn use_history_memos(user: Rc<UserState>) -> HistoryMemos {
    let branches = use_memo(user.clone(), |user| user.branches.clone());
    let messages = use_memo(user, |user| user.conversation_history.clone());
    HistoryMemos {
        branches: (*branches).clone(),
        messages: (*messages).clone(),
    }
}

fn current_step_title(user: &UserState) -> Option<String> {
    user.active_branch_plan_id().and_then(|id| {
        user.language_plans
            .iter()
            .find(|plan| plan.id == id)
            .and_then(|plan| plan.steps.get(plan.current_step_index))
            .map(|step| step.title.clone())
    })
}

fn dialect_quests(user: &UserState) -> Vec<Quest> {
    user.gamification_stats()
        .quests
        .into_iter()
        .filter(|quest| quest.dialect == user.active_branch_dialect())
        .collect()
}

#[hook]
fn use_main_callbacks(ctx: &MainContext) -> MainCallbacks {
    let modal = use_selection_modal(&ctx.props);
    let goals = use_goal_callbacks(ctx.dispatch.clone());
    let branches = use_branch_callbacks(ctx.dispatch.clone());
    let learning = use_learning_item_callbacks(ctx);
    let drawer = use_drawer_callbacks(ctx.props.ui_state.clone());
    use_keyboard_shortcuts(ctx.clone(), drawer.active_tab.clone());
    MainCallbacks {
        modal,
        goals,
        branches,
        learning,
        drawer,
    }
}

#[hook]
fn use_selection_modal(props: &MainContentProps) -> ModalCallbacks {
    let state = use_state(|| None);
    let close = use_callback(state.clone(), |_, state| state.set(None));
    let save = use_save_phrase(props.session.clone(), state.clone());
    let action = use_callback((props.clone(), state.clone()), |request, (props, state)| {
        start_selection_action(props.clone(), state.clone(), request);
    });
    ModalCallbacks {
        state,
        close,
        save,
        action,
    }
}

#[hook]
fn use_save_phrase(
    session: UseReducerHandle<SessionState>,
    state: ModalState,
) -> Callback<SavePhrase> {
    use_callback(
        (session, state),
        |(element, meaning, context), (session, state)| {
            if let Some(SelectionModalState::Loaded { result, .. }) = state.as_ref() {
                let action = learning_action_for(result, element, meaning, context);
                session.dispatch(SessionAction::Domain(UserDomainAction::Learning(action)));
            }
            state.set(None);
        },
    )
}

fn learning_action_for(
    result: &SelectionResult,
    element: String,
    meaning: String,
    context: String,
) -> LearningAction {
    let (explained, translated) = match result {
        SelectionResult::Translation(_) => (
            vec![],
            vec![(Translated::new(meaning, element, Some(context)), 0)],
        ),
        SelectionResult::Grammar(_) => (vec![(Explained::new(element, meaning), 0)], vec![]),
    };
    LearningAction::AddItems(vec![], explained, translated, vec![])
}

fn start_selection_action(props: MainContentProps, state: ModalState, request: SelectionRequest) {
    state.set(Some(SelectionModalState::Loading {
        original_text: request.2.clone(),
    }));
    props.ui_state.dispatch(UIStateAction::SetTranslateLoading {
        message_id: request.0,
    });
    wasm_bindgen_futures::spawn_local(perform_selection_action(props, state, request));
}

async fn perform_selection_action(
    props: MainContentProps,
    state: ModalState,
    request: SelectionRequest,
) {
    if let Some(user) = props.session.user.as_ref() {
        let result = run_selection_action(&props.app_state, user, &request).await;
        apply_selection_result(state, request.2, result);
    }
    props
        .ui_state
        .dispatch(UIStateAction::ClearTranslateLoading {
            message_id: request.0,
        });
}

async fn run_selection_action(
    app: &AppState,
    user: &UserState,
    request: &SelectionRequest,
) -> Result<SelectionResult, String> {
    match request.1 {
        SelectionAction::Translate => translate_selection(app, user, request).await,
        SelectionAction::ExplainGrammar => app
            .grammar_service
            .explain_grammar(&request.2, request.3.clone(), user.current_dialect())
            .await
            .map(SelectionResult::Grammar)
            .map_err(|error| format!("Grammar explanation failed: {error}")),
    }
}

async fn translate_selection(
    app: &AppState,
    user: &UserState,
    request: &SelectionRequest,
) -> Result<SelectionResult, String> {
    let dialect = user.current_dialect();
    let formality = Some(user.active_branch_settings().formality);
    app.translation_service
        .translate_phrase(&request.2, request.3.clone(), dialect, formality)
        .await
        .map(|response| SelectionResult::Translation(response.segmented_phrases))
        .map_err(|error| format!("Translation failed: {error}"))
}

fn apply_selection_result(
    state: ModalState,
    original_text: String,
    result: Result<SelectionResult, String>,
) {
    match result {
        Ok(result) => state.set(Some(SelectionModalState::Loaded {
            original_text,
            result,
        })),
        Err(error) => {
            web_sys::console::error_1(&error.into());
            state.set(None);
        }
    }
}

#[hook]
fn use_goal_callbacks(dispatch: Callback<UserDomainAction>) -> (Callback<String>, Callback<usize>) {
    let add = use_callback(dispatch.clone(), |goal: String, dispatch| {
        if !goal.trim().is_empty() {
            dispatch.emit(UserDomainAction::Learning(LearningAction::AddGoal(goal)));
        }
    });
    let delete = use_callback(dispatch, |index, dispatch| {
        dispatch.emit(UserDomainAction::Learning(LearningAction::DeleteGoal(
            index,
        )))
    });
    (add, delete)
}

#[hook]
fn use_branch_callbacks(dispatch: Callback<UserDomainAction>) -> (Callback<Uuid>, Callback<Uuid>) {
    let switch = use_callback(dispatch.clone(), |id, dispatch| {
        dispatch.emit(UserDomainAction::Branch(BranchAction::Switch(id)))
    });
    let delete = use_callback(dispatch, |id, dispatch| {
        dispatch.emit(UserDomainAction::Branch(BranchAction::Delete(id)))
    });
    (switch, delete)
}

#[hook]
fn use_drawer_callbacks(ui: UseReducerHandle<UIState>) -> DrawerCallbacks {
    let active_tab = use_state(|| DrawerTab::Branches);
    let close = use_callback(ui, |_, ui| ui.dispatch(UIStateAction::ToggleDrawer));
    let change = use_callback(active_tab.clone(), |tab, active| active.set(tab));
    DrawerCallbacks {
        active_tab,
        close,
        change,
    }
}

#[hook]
fn use_learning_item_callbacks(ctx: &MainContext) -> LearningCallbacks {
    let delete = use_callback(ctx.clone(), |id, ctx| delete_learning_item(ctx, id));
    let dismiss = use_callback(ctx.props.ui_state.clone(), |id, ui| {
        ui.dispatch(UIStateAction::DismissFlashItem(id))
    });
    let undo = use_callback(ctx.props.clone(), |_, props| undo_learning_item(props));
    LearningCallbacks {
        delete,
        dismiss,
        undo,
    }
}

fn delete_learning_item(ctx: &MainContext, id: Uuid) {
    if let Some(item) = ctx.user.learning_items.iter().find(|item| item.id() == id) {
        ctx.dispatch
            .emit(UserDomainAction::Learning(LearningAction::DeleteItem(
                item.id(),
            )));
        ctx.props
            .ui_state
            .dispatch(UIStateAction::PushDeletedLearningItem(item.clone()));
    }
}

fn undo_learning_item(props: &MainContentProps) {
    if let Some(item) = props.ui_state.deleted_learning_items.back() {
        props
            .session
            .dispatch(SessionAction::Domain(UserDomainAction::Learning(
                LearningAction::UndoDeleteItem(item.clone()),
            )));
        props
            .ui_state
            .dispatch(UIStateAction::PopDeletedLearningItem);
    }
}

#[hook]
fn use_keyboard_shortcuts(ctx: MainContext, active_tab: UseStateHandle<DrawerTab>) {
    use_effect_with((), move |_| {
        let shortcuts = default_shortcuts();
        let listener = EventListener::new(&gloo::utils::window(), "keydown", move |event| {
            let event = event.dyn_ref::<web_sys::KeyboardEvent>().unwrap();
            for (action, binding) in &shortcuts {
                if matches_binding(event, binding) {
                    event.prevent_default();
                    handle_shortcut(action, &ctx, &active_tab);
                    break;
                }
            }
        });
        move || drop(listener)
    });
}

fn handle_shortcut(action: &ShortcutAction, ctx: &MainContext, active_tab: &TabState) {
    match action {
        ShortcutAction::FocusGoalInput => {
            // Disabled in this phase
            // if let Some(input) =
            //     goal_input_ref.cast::<web_sys::HtmlInputElement>()
            // {
            //     let _ = input.focus();
            // }
        }
        _ => handle_active_shortcut(action, ctx, active_tab),
    }
}

fn handle_active_shortcut(action: &ShortcutAction, ctx: &MainContext, active_tab: &TabState) {
    let ui = &ctx.props.ui_state;
    match action {
        ShortcutAction::ToggleDrawer => ui.dispatch(UIStateAction::ToggleDrawer),
        ShortcutAction::ToggleLearningPanel => ui.dispatch(UIStateAction::ToggleLearningPanel),
        ShortcutAction::TogglePracticeSettings => toggle_practice_settings(ui),
        ShortcutAction::ToggleAutoSpeak => dispatch_setting(ctx, SettingsAction::ToggleTTS),
        ShortcutAction::ReplayLastMessage => replay_last_message(&ctx.props),
        ShortcutAction::CycleDialect => dispatch_setting(ctx, SettingsAction::CycleDialect),
        ShortcutAction::CycleTeachingMode => cycle_teaching_mode(ctx),
        ShortcutAction::CycleFormality => dispatch_setting(ctx, SettingsAction::CycleFormality),
        ShortcutAction::FocusGoalInput => (),
        ShortcutAction::FocusChatInput => focus_chat_input(&ctx.chat_input_ref),
        ShortcutAction::OpenUsage => open_usage(&ctx.props.ui_state, active_tab),
    }
}

fn toggle_practice_settings(ui: &UseReducerHandle<UIState>) {
    ui.dispatch(if ui.panel_open {
        UIStateAction::ClosePanel
    } else {
        UIStateAction::OpenPanel
    });
}

fn dispatch_setting(ctx: &MainContext, action: SettingsAction) {
    ctx.props
        .session
        .dispatch(SessionAction::Domain(UserDomainAction::Settings(action)));
}

fn replay_last_message(props: &MainContentProps) {
    if let Some(user) = props.session.user.as_ref()
        && let Some(message) = user.get_active_branch_messages().last()
    {
        on_replay_message(props.app_state.clone()).emit((*message).clone());
    }
}

fn cycle_teaching_mode(ctx: &MainContext) {
    let is_admin = ctx
        .props
        .app_state
        .current_user
        .as_ref()
        .map(|user| user.is_admin)
        .unwrap_or(false);
    dispatch_setting(ctx, SettingsAction::CycleTeachingMode(is_admin));
}

fn focus_chat_input(input: &NodeRef) {
    if let Some(textarea) = input.cast::<web_sys::HtmlTextAreaElement>() {
        let _ = textarea.focus();
    }
}

fn open_usage(ui: &UseReducerHandle<UIState>, active_tab: &UseStateHandle<DrawerTab>) {
    active_tab.set(DrawerTab::Usage);
    ui.dispatch(UIStateAction::SetDrawerOpen(true));
}

fn render_chat_canvas(ctx: &MainContext, callbacks: &MainCallbacks) -> Html {
    html! {
        <div class="chat-canvas">
            {render_chat(ctx, &callbacks.modal.action)}
            {render_island(ctx)}
            {render_input(ctx)}
            {render_message_undo_notification(ctx.props.ui_state.deleted_messages.len(), on_undo_message_callback(ctx.props.ui_state.clone(), ctx.dispatch.clone()))}
        </div>
    }
}

fn render_selection_bar(ctx: &MainContext, action: &Callback<SelectionRequest>) -> Html {
    html! { <TranslateSelectionButton messages={ctx.history.messages.clone()} translate_loading={ctx.props.ui_state.translate_loading.clone()} on_action={action.clone()} /> }
}

fn render_chat(ctx: &MainContext, action: &Callback<SelectionRequest>) -> Html {
    let app_state = &ctx.props.app_state;
    let ui_state = &ctx.props.ui_state;
    let session = &ctx.props.session;
    html! {
        <ChatWindow user={ctx.user.clone()} is_loading={app_state.is_loading}
            on_replay_message={Some(on_replay_message(app_state.clone()))}
            on_delete_message={Some(on_delete_message_callback(ui_state.clone(), ctx.user.clone(), ctx.dispatch.clone()))}
            on_create_branch={Some(on_create_branch(ctx.dispatch.clone()))}
            on_auto_start={Some(on_auto_start(app_state.clone(), session.clone(), ui_state.clone()))}
            on_continue_branch={Some(on_continue_branch(app_state.clone(), session.clone(), ui_state.clone()))}
            on_explain={Some(on_explain_message(app_state.clone(), session.clone(), ui_state.clone()))}
            explain_loading={ui_state.explain_loading.clone()}>
            {render_selection_bar(ctx, action)}
        </ChatWindow>
    }
}

fn render_island(ctx: &MainContext) -> Html {
    html! {
        <DynamicIsland items={ctx.data.items.clone()} current_step_title={current_step_title(&ctx.user)}
            learning_goals={ctx.data.goals.clone()} quests={dialect_quests(&ctx.user)} />
    }
}

fn render_input(ctx: &MainContext) -> Html {
    html! {
        <InputBox on_send={on_send_message(ctx.props.app_state.clone(), ctx.props.session.clone(), ctx.props.ui_state.clone())}
            textarea_ref={Some(ctx.chat_input_ref.clone())} language_option={ctx.user.current_language_option()} />
    }
}

fn render_drawer(ctx: &MainContext, callbacks: &MainCallbacks) -> Html {
    let app_state = &ctx.props.app_state;
    let ui_state = &ctx.props.ui_state;
    html! {
        <Drawer is_open={ui_state.drawer_open} on_close={callbacks.drawer.close.clone()} title="Study Tools">
            <StudyDrawerContent active_tab={*callbacks.drawer.active_tab} on_tab_change={callbacks.drawer.change.clone()}
                user={ctx.user.clone()} is_admin={app_state.current_user.as_ref().map(|u| u.is_admin).unwrap_or(false)} dispatch={ctx.dispatch.clone()}
                branches={ctx.history.branches.clone()} active_branch_id={ctx.user.active_branch_id} messages={ctx.history.messages.clone()}
                learning_goals={ctx.data.goals.clone()} on_add_goal={callbacks.goals.0.clone()} on_delete_goal={callbacks.goals.1.clone()}
                on_switch_branch={Some(callbacks.branches.0.clone())} on_delete_branch={Some(callbacks.branches.1.clone())}
                goal_input_ref={Some(ctx.goal_input_ref.clone())} learning_items={ctx.data.items.clone()} active_branch_dialect={Some(ctx.user.active_branch_dialect())}
                enrichment_service={app_state.enrichment_service.clone()} plan_service={app_state.plan_service.clone()}
                on_delete_learning_item={callbacks.learning.delete.clone()} on_undo_delete_learning_item={callbacks.learning.undo.clone()}
                deleted_learning_items_count={ui_state.deleted_learning_items.len()} />
        </Drawer>
    }
}
fn render_modal(
    modal_state: &UseStateHandle<Option<SelectionModalState>>,
    on_close: &Callback<()>,
    on_save: &Callback<(String, String, String)>,
) -> Html {
    match modal_state.as_ref() {
        Some(SelectionModalState::Loading { original_text }) => html! {
            <SelectionModal
                original_text={original_text.clone()}
                result={None}
                on_close={on_close.clone()}
                on_save={on_save.clone()}
            />
        },
        Some(SelectionModalState::Loaded {
            original_text,
            result,
        }) => html! {
            <SelectionModal
                original_text={original_text.clone()}
                result={Some(result.clone())}
                on_close={on_close.clone()}
                on_save={on_save.clone()}
            />
        },
        None => html! {},
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_fixtures::{quest, user_with_plan_step};
    use dialect_coach_shared::models::{Dialect, LearningItemType};

    #[test]
    fn finds_the_active_branch_plan_step() {
        let mut user = user_with_plan_step("Current step");
        assert_eq!(current_step_title(&user), Some("Current step".into()));
        Arc::make_mut(&mut user.language_plans)[0].current_step_index = 1;
        assert_eq!(current_step_title(&user), None);
    }

    #[test]
    fn does_not_use_an_inactive_missing_or_other_dialect_plan() {
        let mut user = user_with_plan_step("Inactive step");
        Arc::make_mut(&mut user.branches)[0].active_plan_id = None;
        assert_eq!(current_step_title(&user), None);
        Arc::make_mut(&mut user.branches)[0].active_plan_id = Some(Uuid::new_v4());
        assert_eq!(current_step_title(&user), None);
        let mut user = user_with_plan_step("Other dialect");
        Arc::make_mut(&mut user.language_plans)[0].dialect = Dialect::ArabicEgyptian;
        assert_eq!(current_step_title(&user), None);
        assert_eq!(current_step_title(&UserState::new(Uuid::new_v4())), None);
    }

    #[test]
    fn quests_only_include_the_active_branch_dialect() {
        let mut user = UserState::new(Uuid::new_v4());
        let dialect = user.active_branch_dialect();
        let local = LearningItem::new(
            LearningItemType::Explanation(Explained::new("local".into(), "meaning".into())),
            dialect,
        );
        let other = LearningItem::new(
            LearningItemType::Explanation(Explained::new("other".into(), "meaning".into())),
            Dialect::ArabicEgyptian,
        );
        user.learning_items = Arc::new(vec![other, local.clone()]);
        assert_eq!(
            dialect_quests(&user),
            vec![quest(local.id(), dialect, "Practice: local")]
        );
        assert!(dialect_quests(&UserState::new(Uuid::new_v4())).is_empty());
    }

    #[test]
    fn translation_save_keeps_english_target_order_and_selected_context() {
        let result = SelectionResult::Translation(vec![]);
        let (element, meaning, context) = ("أهلا".into(), "hello".into(), "selected phrase".into());
        let action = learning_action_for(&result, element, meaning, context);
        let LearningAction::AddItems(mistakes, explained, translated, exploratory) = action else {
            panic!("expected added items")
        };
        assert!(mistakes.is_empty() && explained.is_empty() && exploratory.is_empty());
        assert_eq!(translated.len(), 1);
        assert_eq!(translated[0].0.translated_word, "hello");
        assert_eq!(translated[0].0.translated_to, "أهلا");
        assert_eq!(translated[0].0.context.as_deref(), Some("selected phrase"));
        assert_eq!(translated[0].1, 0);
    }

    #[test]
    fn grammar_save_keeps_element_explanation_and_initial_score() {
        let result = SelectionResult::Grammar(vec![]);
        let (element, meaning, context) = (
            "element".into(),
            "explanation".into(),
            "selected phrase".into(),
        );
        let action = learning_action_for(&result, element, meaning, context);
        let LearningAction::AddItems(mistakes, explained, translated, exploratory) = action else {
            panic!("expected added items")
        };
        assert!(mistakes.is_empty() && translated.is_empty() && exploratory.is_empty());
        assert_eq!(explained.len(), 1);
        assert_eq!(explained[0].0.new_phrase, "element");
        assert_eq!(explained[0].0.explanation, "explanation");
        assert_eq!(explained[0].1, 0);
    }
}
