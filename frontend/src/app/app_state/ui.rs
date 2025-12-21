use dialect_coach_shared::{LearningItem, Message};
use std::collections::{HashSet, VecDeque};
use std::rc::Rc;
use uuid::Uuid;
use yew::prelude::*;

pub enum UIStateAction {
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
    PushDeletedMessage(Message),
    PopDeletedMessage,
    ShowUserCreationPage,
    HideUserCreationPage,
    SetExplainLoading { message_id: Uuid },
    ClearExplainLoading { message_id: Uuid },
    SetTranslateLoading { message_id: Uuid },
    ClearTranslateLoading { message_id: Uuid },
    SetSignInLoading(bool),
}

#[derive(Clone, PartialEq, Default)]
pub struct UIState {
    pub panel_open: bool,
    pub translating_button: Option<String>,
    pub learning_panel_open: bool,
    pub drawer_open: bool,
    pub learning_panel_collapsed: bool,
    pub deleted_learning_items: VecDeque<LearningItem>,
    pub deleted_messages: VecDeque<Message>,
    pub show_user_creation_page: bool,
    pub explain_loading: HashSet<Uuid>,
    pub translate_loading: HashSet<Uuid>,
    pub is_signing_in: bool,
}


impl UIState {
    pub fn apply_action(&self, action: UIStateAction) -> Self {
        let mut next = self.clone();
        match action {
            UIStateAction::OpenPanel => next.panel_open = true,
            UIStateAction::ClosePanel => next.panel_open = false,
            UIStateAction::PushTranslatingButton(msg) => next.translating_button = Some(msg),
            UIStateAction::ClearTranslatingButton => next.translating_button = None,
            UIStateAction::OpenLearningPanel => next.learning_panel_open = true,
            UIStateAction::CloseLearningPanel => next.learning_panel_open = false,
            UIStateAction::ToggleDrawer => next.drawer_open = !next.drawer_open,
            UIStateAction::SetDrawerOpen(open) => next.drawer_open = open,
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
            UIStateAction::SetTranslateLoading { message_id } => {
                next.translate_loading.insert(message_id);
            }
            UIStateAction::ClearTranslateLoading { message_id } => {
                next.translate_loading.remove(&message_id);
            }
            UIStateAction::SetSignInLoading(loading) => {
                next.is_signing_in = loading;
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
