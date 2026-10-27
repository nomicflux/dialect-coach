use super::user::DeletedMessage;
use dialect_coach_shared::LearningItem;
use std::collections::{HashMap, HashSet, VecDeque};
use std::rc::Rc;
use uuid::Uuid;
use yew::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ActivityOperation {
    Generate,
    Open,
    Check,
    Chat,
}

#[derive(Clone, PartialEq, Default)]
pub struct ActivityUi {
    pub busy: Option<ActivityOperation>,
    pub error: Option<ActivityOperation>,
    pub expanded: bool,
    pub initial_preview: bool,
    pub description: String,
    pub force_check: bool,
    pub pending_open: Option<Uuid>,
    pub attempted: Option<(Uuid, Option<Uuid>)>,
}

pub enum UIStateAction {
    SelectBranch(Uuid),
    ActivityBusy(Uuid, ActivityOperation),
    ActivityFinished(Uuid, Option<ActivityOperation>),
    ActivityDescription(Uuid, String),
    ActivityCreate(Uuid),
    ActivityExpanded(Uuid, bool),
    ActivityInstalled(Uuid, Uuid),
    ActivityOpeningTaken(Uuid),
    ActivityAttempted(Uuid, Uuid, Option<Uuid>),
    ActivityRetryCheck(Uuid),
    ActivitySent(Uuid),
    ActivityEnded(Uuid),
    ActivityEditor(Uuid, Uuid),
    ReplyEffects(Uuid, dialect_coach_shared::Message, Vec<LearningItem>),
    ClearReplyEffect,
    ClearActivityFocus,
    OpenPanel,
    ClosePanel,
    PushTranslatingButton(String),
    ClearTranslatingButton,
    OpenLearningPanel,
    CloseLearningPanel,
    ToggleDrawer,
    SetDrawerOpen(bool),
    ToggleLearningPanel,
    PushDeletedLearningItem(LearningItem),
    PopDeletedLearningItem,
    PushDeletedMessage(DeletedMessage),
    PopDeletedMessage,
    ShowUserCreationPage,
    HideUserCreationPage,
    SetExplainLoading { message_id: Uuid },
    ClearExplainLoading { message_id: Uuid },
    SetTranslateLoading { source_id: Uuid },
    ClearTranslateLoading { source_id: Uuid },
    SetSignInLoading(bool),
    QueueFlashItems(Vec<LearningItem>),
    DismissFlashItem(Uuid),
}

#[derive(Clone, PartialEq, Default)]
pub struct UIState {
    pub selected_branch: Option<Uuid>,
    pub activities: HashMap<Uuid, ActivityUi>,
    pub activity_editor: Option<(Uuid, Uuid)>,
    pub activity_creation: Option<Uuid>,
    pub reply_effect: Option<dialect_coach_shared::Message>,
    pub focus_activity_start: bool,
    pub panel_open: bool,
    pub translating_button: Option<String>,
    pub learning_panel_open: bool,
    pub drawer_open: bool,
    pub learning_panel_collapsed: bool,
    pub deleted_learning_items: VecDeque<LearningItem>,
    pub deleted_messages: VecDeque<DeletedMessage>,
    pub show_user_creation_page: bool,
    pub explain_loading: HashSet<Uuid>,
    pub translate_loading: HashSet<Uuid>,
    pub is_signing_in: bool,
    pub flashed_items: Vec<LearningItem>,
}

impl UIState {
    pub fn apply_action(&self, action: UIStateAction) -> Self {
        let mut next = self.clone();
        match action {
            UIStateAction::SelectBranch(id) => {
                if next.selected_branch != Some(id) {
                    next.selected_branch = Some(id);
                    next.flashed_items.clear();
                    next.reply_effect = None;
                    if next.activity_editor.is_some() || next.activity_creation.is_some() {
                        next.activity_editor = None;
                        next.activity_creation = None;
                        next.drawer_open = false;
                    }
                    for activity in next.activities.values_mut() {
                        activity.expanded = false;
                        activity.initial_preview = false;
                    }
                }
            }
            UIStateAction::ActivityBusy(id, operation) => {
                let state = next.activities.entry(id).or_default();
                state.busy = Some(operation);
                state.error = None;
            }
            UIStateAction::ActivityFinished(id, error) => {
                let state = next.activities.entry(id).or_default();
                state.busy = None;
                state.error = error;
            }
            UIStateAction::ActivityDescription(id, text) => {
                next.activities.entry(id).or_default().description = text
            }
            UIStateAction::ActivityCreate(id) => {
                next.activity_creation = Some(id);
                next.activity_editor = None;
                next.drawer_open = true;
            }
            UIStateAction::ActivityExpanded(id, open) => {
                let state = next.activities.entry(id).or_default();
                state.expanded = open;
                state.initial_preview = false;
            }
            UIStateAction::ActivityInstalled(id, activity_id) => {
                if next.activity_creation == Some(id) {
                    next.activity_creation = None;
                    next.drawer_open = false;
                }
                let state = next.activities.entry(id).or_default();
                state.pending_open = Some(activity_id);
                if next.selected_branch == Some(id) {
                    state.expanded = true;
                    state.initial_preview = true;
                }
            }
            UIStateAction::ActivityOpeningTaken(id) => {
                next.activities.entry(id).or_default().pending_open = None
            }
            UIStateAction::ActivityAttempted(id, snapshot, user) => {
                let state = next.activities.entry(id).or_default();
                state.attempted = Some((snapshot, user));
                state.force_check = false;
            }
            UIStateAction::ActivityRetryCheck(id) => {
                let state = next.activities.entry(id).or_default();
                state.attempted = None;
                state.error = None;
                state.force_check = true;
            }
            UIStateAction::ActivitySent(id) => {
                let state = next.activities.entry(id).or_default();
                if state.initial_preview {
                    state.expanded = false;
                    state.initial_preview = false;
                }
            }
            UIStateAction::ActivityEnded(id) => {
                next.focus_activity_start = next.selected_branch == Some(id);
                let state = next.activities.entry(id).or_default();
                state.expanded = false;
                state.error = None;
                state.pending_open = None;
                if next.activity_editor.is_some_and(|(branch, _)| branch == id) {
                    next.activity_editor = None;
                    next.drawer_open = false;
                }
            }
            UIStateAction::ActivityEditor(branch, activity) => {
                next.activity_creation = None;
                next.activity_editor = Some((branch, activity));
                next.drawer_open = true;
            }
            UIStateAction::ReplyEffects(branch, message, items) => {
                if next.selected_branch == Some(branch) {
                    next.reply_effect = Some(message);
                    next.flashed_items.extend(items);
                }
            }
            UIStateAction::ClearReplyEffect => next.reply_effect = None,
            UIStateAction::ClearActivityFocus => next.focus_activity_start = false,
            UIStateAction::OpenPanel => next.panel_open = true,
            UIStateAction::ClosePanel => next.panel_open = false,
            UIStateAction::PushTranslatingButton(msg) => next.translating_button = Some(msg),
            UIStateAction::ClearTranslatingButton => next.translating_button = None,
            UIStateAction::OpenLearningPanel => next.learning_panel_open = true,
            UIStateAction::CloseLearningPanel => next.learning_panel_open = false,
            UIStateAction::ToggleDrawer => {
                next.drawer_open = !next.drawer_open;
                next.activity_editor = None;
                next.activity_creation = None;
            }
            UIStateAction::SetDrawerOpen(open) => {
                next.drawer_open = open;
                next.activity_editor = None;
                next.activity_creation = None;
            }
            UIStateAction::ToggleLearningPanel => {
                next.learning_panel_collapsed = !next.learning_panel_collapsed
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
            UIStateAction::SetExplainLoading { message_id } => {
                next.explain_loading.insert(message_id);
            }
            UIStateAction::ClearExplainLoading { message_id } => {
                next.explain_loading.remove(&message_id);
            }
            UIStateAction::SetTranslateLoading { source_id } => {
                next.translate_loading.insert(source_id);
            }
            UIStateAction::ClearTranslateLoading { source_id } => {
                next.translate_loading.remove(&source_id);
            }
            UIStateAction::SetSignInLoading(loading) => {
                next.is_signing_in = loading;
            }
            UIStateAction::QueueFlashItems(items) => {
                next.flashed_items.extend(items);
            }
            UIStateAction::DismissFlashItem(id) => {
                next.flashed_items.retain(|item| item.id() != id);
            }
        }
        next
    }
}

impl Reducible for UIState {
    type Action = UIStateAction;

    fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
        self.apply_action(action).into()
    }
}

#[cfg(test)]
mod activity_tests {
    use super::*;
    #[test]
    fn reply_effects_use_current_selection_and_opening_is_consumed_once() {
        let first = Uuid::new_v4();
        let second = Uuid::new_v4();
        let activity = Uuid::new_v4();
        let state = UIState::default().apply_action(UIStateAction::SelectBranch(first));
        let state = state.apply_action(UIStateAction::ActivityInstalled(first, activity));
        assert_eq!(state.activities[&first].pending_open, Some(activity));
        let state = state.apply_action(UIStateAction::ActivityOpeningTaken(first));
        assert!(state.activities[&first].pending_open.is_none());
        let state = state.apply_action(UIStateAction::SelectBranch(second));
        let user = dialect_coach_shared::UserState::new(Uuid::new_v4());
        let message = user.create_user_msg(Uuid::new_v4(), "late");
        let state = state.apply_action(UIStateAction::ReplyEffects(first, message, vec![]));
        assert!(state.reply_effect.is_none());
        assert!(!state.activities[&first].expanded);
    }
    #[test]
    fn generation_failure_keeps_form_and_description_for_retry() {
        let id = Uuid::new_v4();
        let state = UIState::default()
            .apply_action(UIStateAction::ActivityDescription(id, "Dinner".into()))
            .apply_action(UIStateAction::ActivityCreate(id))
            .apply_action(UIStateAction::ActivityBusy(id, ActivityOperation::Generate))
            .apply_action(UIStateAction::ActivityFinished(
                id,
                Some(ActivityOperation::Generate),
            ));
        assert_eq!(state.activities[&id].description, "Dinner");
        assert_eq!(state.activity_creation, Some(id));
        assert!(state.drawer_open);
        let state =
            state.apply_action(UIStateAction::ActivityBusy(id, ActivityOperation::Generate));
        assert_eq!(state.activities[&id].description, "Dinner");
        let state = state.apply_action(UIStateAction::ActivityInstalled(id, Uuid::new_v4()));
        assert!(state.activity_creation.is_none());
        assert!(!state.drawer_open);
    }

    #[test]
    fn late_generation_leaves_another_branch_creation_form_open() {
        let first = Uuid::new_v4();
        let second = Uuid::new_v4();
        let state = UIState::default()
            .apply_action(UIStateAction::SelectBranch(first))
            .apply_action(UIStateAction::ActivityCreate(first))
            .apply_action(UIStateAction::ActivityDescription(first, "Dinner".into()))
            .apply_action(UIStateAction::ActivityBusy(
                first,
                ActivityOperation::Generate,
            ))
            .apply_action(UIStateAction::SelectBranch(second));
        assert!(state.activity_creation.is_none());
        assert!(!state.drawer_open);
        let state = state
            .apply_action(UIStateAction::ActivityCreate(second))
            .apply_action(UIStateAction::ActivityInstalled(first, Uuid::new_v4()));
        assert_eq!(state.activity_creation, Some(second));
        assert!(state.drawer_open);
        assert_eq!(state.activities[&first].description, "Dinner");
    }
}
