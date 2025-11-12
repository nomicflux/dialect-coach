use crate::app::app_state::{OptionalUserState, UIState, UIStateAction, UserStateAction};
use dialect_coach_shared::models::{Formality, Language, TeachingMode};
use log::info;
use uuid::Uuid;
use yew::prelude::*;

pub fn on_language_change(user_state: UseReducerHandle<OptionalUserState>) -> Callback<Event> {
    let user_state = user_state.clone();
    Callback::from(move |e: Event| {
        if user_state.0.is_none() {
            return;
        }
        if let Some(select) = e.target_dyn_into::<web_sys::HtmlSelectElement>() {
            let value = select.value();
            let lang = match value.as_str() {
                "spanish" => Language::Spanish,
                "arabic" => Language::Arabic,
                "french" => Language::French,
                _ => Language::Spanish,
            };
            user_state.dispatch(UserStateAction::ChangeLanguage(lang));
        }
    })
}

pub fn on_dialect_change(user_state: UseReducerHandle<OptionalUserState>) -> Callback<Event> {
    let user_state = user_state.clone();
    Callback::from(move |e: Event| {
        let state = match user_state.0.as_ref() {
            Some(s) => s,
            None => return,
        };
        if let Some(select) = e.target_dyn_into::<web_sys::HtmlSelectElement>() {
            let value = select.value();

            // Get all dialects for current language and find matching one
            let dialects = state.current_dialects();
            if let Some(dialect_features) = dialects.iter().find(|df| df.dialect.id() == value) {
                user_state.dispatch(UserStateAction::ChangeDialect(dialect_features.dialect));
            }
        }
    })
}

pub fn on_formality_change(user_state: UseReducerHandle<OptionalUserState>) -> Callback<Event> {
    let user_state = user_state.clone();

    Callback::from(move |e: Event| {
        if user_state.0.is_none() {
            return;
        }
        if let Some(select) = e.target_dyn_into::<web_sys::HtmlSelectElement>() {
            let value = select.value();
            let f = match value.as_str() {
                "formal" => Formality::Formal,
                "casual" => Formality::Casual,
                "dialect_rich" => Formality::DialectRich,
                "slang" => Formality::Slang,
                _ => Formality::Casual,
            };
            user_state.dispatch(UserStateAction::ChangeFormality(f));
        }
    })
}

pub fn on_teaching_mode_change(user_state: UseReducerHandle<OptionalUserState>) -> Callback<Event> {
    let user_state = user_state.clone();

    Callback::from(move |e: Event| {
        if user_state.0.is_none() {
            return;
        }
        if let Some(select) = e.target_dyn_into::<web_sys::HtmlSelectElement>() {
            let value = select.value();
            let tm = match value.as_str() {
                "immersive" => TeachingMode::Immersive,
                "corrective" => TeachingMode::Corrective,
                "explanatory" => TeachingMode::Explanatory,
                "interleaved" => TeachingMode::Interleaved,
                "storyteller" => TeachingMode::StoryTeller,
                "debug" => TeachingMode::Debug,
                _ => TeachingMode::Immersive,
            };
            user_state.dispatch(UserStateAction::ChangeTeachingMode(tm));
        }
    })
}

pub fn on_dialect_cycle(user_state: UseReducerHandle<OptionalUserState>) -> Callback<()> {
    let user_state = user_state.clone();
    Callback::from(move |_| {
        let state = match user_state.0.as_ref() {
            Some(s) => s,
            None => return,
        };
        let dialects = state.current_dialects();
        let current = state.current_dialect();
        if let Some(idx) = dialects.iter().position(|df| df.dialect == current) {
            let next_idx = (idx + 1) % dialects.len();
            user_state.dispatch(UserStateAction::ChangeDialect(dialects[next_idx].dialect));
        }
    })
}

pub fn on_formality_cycle(user_state: UseReducerHandle<OptionalUserState>) -> Callback<()> {
    let user_state = user_state.clone();
    Callback::from(move |_| {
        let state = match user_state.0.as_ref() {
            Some(s) => s,
            None => return,
        };
        let formalities = [
            Formality::Formal,
            Formality::Casual,
            Formality::DialectRich,
            Formality::Slang,
        ];
        let current = state.formality;
        if let Some(idx) = formalities.iter().position(|f| f == &current) {
            let next_idx = (idx + 1) % formalities.len();
            user_state.dispatch(UserStateAction::ChangeFormality(formalities[next_idx]));
        }
    })
}

pub fn on_teaching_mode_cycle(user_state: UseReducerHandle<OptionalUserState>) -> Callback<()> {
    let user_state = user_state.clone();
    Callback::from(move |_| {
        let state = match user_state.0.as_ref() {
            Some(s) => s,
            None => return,
        };
        let modes = [
            TeachingMode::Immersive,
            TeachingMode::Corrective,
            TeachingMode::Explanatory,
            TeachingMode::Interleaved,
            TeachingMode::StoryTeller,
            TeachingMode::Debug,
        ];
        let current = state.teaching_mode;
        if let Some(idx) = modes.iter().position(|m| m == &current) {
            let next_idx = (idx + 1) % modes.len();
            user_state.dispatch(UserStateAction::ChangeTeachingMode(modes[next_idx]));
        }
    })
}

pub fn on_delete_message_callback(
    ui_state: UseReducerHandle<UIState>,
    user_state: UseReducerHandle<OptionalUserState>,
) -> Callback<Uuid> {
    Callback::from(move |msg_id: Uuid| {
        if let Some(state) = user_state.0.as_ref() {
            if let Some(msg) = state
                .conversation_history
                .iter()
                .find(|m| m.id == msg_id)
                .cloned()
            {
                ui_state.dispatch(UIStateAction::PushDeletedMessage(msg));
            }
            user_state.dispatch(UserStateAction::DeleteMessage(msg_id));
        }
    })
}

pub fn on_undo_message_callback(
    ui_state: UseReducerHandle<UIState>,
    user_state: UseReducerHandle<OptionalUserState>,
) -> Callback<()> {
    let deleted_messages = ui_state.deleted_messages.clone();
    Callback::from(move |_| {
        if user_state.0.is_none() {
            return;
        }
        if let Some(msg) = deleted_messages.back().cloned() {
            user_state.dispatch(UserStateAction::UndoDeleteMessage(msg));
            ui_state.dispatch(UIStateAction::PopDeletedMessage);
        }
    })
}

pub fn on_create_branch(user_state: UseReducerHandle<OptionalUserState>) -> Callback<Uuid> {
    Callback::from(move |message_id: Uuid| {
        if user_state.0.is_none() {
            return;
        }
        info!("Creating branch from message: {}", message_id);
        user_state.dispatch(UserStateAction::CreateBranch(message_id));
    })
}

pub fn on_switch_branch(user_state: UseReducerHandle<OptionalUserState>) -> Callback<Uuid> {
    Callback::from(move |branch_id: Uuid| {
        if user_state.0.is_none() {
            return;
        }
        info!("Switching to branch: {}", branch_id);
        user_state.dispatch(UserStateAction::SwitchBranch(branch_id));
    })
}

pub fn on_delete_branch(user_state: UseReducerHandle<OptionalUserState>) -> Callback<Uuid> {
    Callback::from(move |branch_id: Uuid| {
        if user_state.0.is_none() {
            return;
        }
        info!("Deleting branch: {}", branch_id);
        user_state.dispatch(UserStateAction::DeleteBranch(branch_id));
    })
}

pub fn on_add_goal(user_state: UseReducerHandle<OptionalUserState>) -> Callback<String> {
    Callback::from(move |goal: String| {
        if user_state.0.is_none() {
            return;
        }
        info!("Adding learning goal: {}", goal);
        user_state.dispatch(UserStateAction::AddLearningGoal(goal));
    })
}

pub fn on_delete_goal(user_state: UseReducerHandle<OptionalUserState>) -> Callback<usize> {
    Callback::from(move |index: usize| {
        if user_state.0.is_none() {
            return;
        }
        info!("Deleting learning goal at index: {}", index);
        user_state.dispatch(UserStateAction::DeleteLearningGoal(index));
    })
}

pub fn on_delete_learning_item_callback(
    ui_state: UseReducerHandle<UIState>,
    user_state: UseReducerHandle<OptionalUserState>,
) -> Callback<Uuid> {
    Callback::from(move |id: Uuid| {
        if let Some(state) = user_state.0.as_ref() {
            if let Some(item) = state.learning_items.iter().find(|i| {
                let item_id = match &i.item {
                    dialect_coach_shared::LearningItemType::Mistake(m) => m.id,
                    dialect_coach_shared::LearningItemType::Explanation(e) => e.id,
                    dialect_coach_shared::LearningItemType::Translation(t) => t.id,
                    dialect_coach_shared::LearningItemType::Exploration(e) => e.id,
                };
                item_id == id
            }) {
                ui_state.dispatch(UIStateAction::PushDeletedLearningItem(item.clone()));
            }
            user_state.dispatch(UserStateAction::DeleteLearningItem(id));
        }
    })
}
