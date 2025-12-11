use crate::app::app_state::{
    BranchAction, LearningAction, MessageAction, OptionalUserState, SettingsAction, UIState,
    UIStateAction, UserStateAction,
};
use dialect_coach_shared::models::{
    ArabicScript, Formality, JapaneseScript, Language, LanguageLevel, LearningGoal, TeachingMode,
    UserGender,
};
use log::info;
use uuid::Uuid;
use yew::prelude::*;

pub fn on_language_change(user_state: UseReducerHandle<OptionalUserState>) -> Callback<Event> {
    let user_state = user_state.clone();
    Callback::from(move |e: Event| {
        if user_state.state.is_none() {
            return;
        }
        if let Some(select) = e.target_dyn_into::<web_sys::HtmlSelectElement>() {
            let value = select.value();
            let lang = match value.as_str() {
                "spanish" => Language::Spanish,
                "arabic" => Language::Arabic,
                "french" => Language::French,
                "english" => Language::English,
                "japanese" => Language::Japanese,
                _ => Language::Spanish,
            };
            user_state.dispatch(UserStateAction::Settings(SettingsAction::ChangeLanguage(lang)));
        }
    })
}

pub fn on_dialect_change(user_state: UseReducerHandle<OptionalUserState>) -> Callback<Event> {
    let user_state = user_state.clone();
    Callback::from(move |e: Event| {
        let state = match user_state.state.as_ref() {
            Some(s) => s,
            None => return,
        };
        if let Some(select) = e.target_dyn_into::<web_sys::HtmlSelectElement>() {
            let value = select.value();

            // Get all dialects for current language and find matching one
            let dialects = state.current_dialects();
            if let Some(dialect_features) = dialects.iter().find(|df| df.dialect.id() == value) {
                user_state.dispatch(UserStateAction::Settings(SettingsAction::ChangeDialect(
                    dialect_features.dialect,
                )));
            }
        }
    })
}

pub fn on_formality_change(user_state: UseReducerHandle<OptionalUserState>) -> Callback<Event> {
    let user_state = user_state.clone();

    Callback::from(move |e: Event| {
        if user_state.state.is_none() {
            return;
        }
        if let Some(select) = e.target_dyn_into::<web_sys::HtmlSelectElement>() {
            let value = select.value();
            let f = match value.as_str() {
                "formal" => Formality::Formal,
                "professional_casual" => Formality::ProfessionalCasual,
                "informal" => Formality::Informal,
                "slang" => Formality::Slang,
                _ => Formality::Informal,
            };
            user_state.dispatch(UserStateAction::Settings(SettingsAction::ChangeFormality(f)));
        }
    })
}

pub fn on_teaching_mode_change(user_state: UseReducerHandle<OptionalUserState>) -> Callback<Event> {
    let user_state = user_state.clone();

    Callback::from(move |e: Event| {
        if user_state.state.is_none() {
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
            user_state.dispatch(UserStateAction::Settings(SettingsAction::ChangeTeachingMode(tm)));
        }
    })
}

pub fn on_user_gender_change(user_state: UseReducerHandle<OptionalUserState>) -> Callback<Event> {
    let user_state = user_state.clone();

    Callback::from(move |e: Event| {
        if user_state.state.is_none() {
            return;
        }
        if let Some(select) = e.target_dyn_into::<web_sys::HtmlSelectElement>() {
            let gender = match select.value().as_str() {
                "male" => UserGender::Male,
                "female" => UserGender::Female,
                "nonbinary" => UserGender::NonBinary,
                _ => UserGender::NonBinary,
            };
            user_state.dispatch(UserStateAction::Settings(SettingsAction::UpdateGender(gender)));
        }
    })
}

pub fn on_language_level_change(
    user_state: UseReducerHandle<OptionalUserState>,
) -> Callback<Event> {
    let user_state = user_state.clone();
    Callback::from(move |e: Event| {
        let state = match user_state.state.as_ref() {
            Some(s) => s,
            None => return,
        };
        if let Some(select) = e.target_dyn_into::<web_sys::HtmlSelectElement>() {
            let level = match select.value().as_str() {
                "a1" => LanguageLevel::A1,
                "a2" => LanguageLevel::A2,
                "b1" => LanguageLevel::B1,
                "b2" => LanguageLevel::B2,
                "c1" => LanguageLevel::C1,
                "c2" => LanguageLevel::C2,
                "a25" => LanguageLevel::A2, // Temporary fix for typo in selection
                _ => LanguageLevel::B1,
            };
            let dialect = state.selected_dialect;
            user_state.dispatch(UserStateAction::Settings(SettingsAction::UpdateLevel(
                dialect, level,
            )));
        }
    })
}

pub fn on_dialect_cycle(user_state: UseReducerHandle<OptionalUserState>) -> Callback<()> {
    let user_state = user_state.clone();
    Callback::from(move |_| {
        if user_state.state.is_some() {
            user_state.dispatch(UserStateAction::Settings(SettingsAction::CycleDialect));
        }
    })
}

pub fn on_formality_cycle(user_state: UseReducerHandle<OptionalUserState>) -> Callback<()> {
    let user_state = user_state.clone();
    Callback::from(move |_| {
        if user_state.state.is_some() {
            user_state.dispatch(UserStateAction::Settings(SettingsAction::CycleFormality));
        }
    })
}

pub fn on_teaching_mode_cycle(user_state: UseReducerHandle<OptionalUserState>) -> Callback<()> {
    let user_state = user_state.clone();
    Callback::from(move |_| {
        if user_state.state.is_some() {
            user_state.dispatch(UserStateAction::Settings(SettingsAction::CycleTeachingMode));
        }
    })
}

pub fn on_delete_message_callback(
    ui_state: UseReducerHandle<UIState>,
    user_state: UseReducerHandle<OptionalUserState>,
) -> Callback<Uuid> {
    Callback::from(move |msg_id: Uuid| {
        if let Some(state) = user_state.state.as_ref() {
            if let Some(msg) = state
                .conversation_history
                .iter()
                .find(|m| m.id == msg_id)
                .cloned()
            {
                ui_state.dispatch(UIStateAction::PushDeletedMessage(msg));
            }
            user_state.dispatch(UserStateAction::Message(MessageAction::Delete(msg_id)));
        }
    })
}

pub fn on_undo_message_callback(
    ui_state: UseReducerHandle<UIState>,
    user_state: UseReducerHandle<OptionalUserState>,
) -> Callback<()> {
    let deleted_messages = ui_state.deleted_messages.clone();
    Callback::from(move |_| {
        if user_state.state.is_none() {
            return;
        }
        if let Some(msg) = deleted_messages.back().cloned() {
            user_state.dispatch(UserStateAction::Message(MessageAction::UndoDelete(msg)));
            ui_state.dispatch(UIStateAction::PopDeletedMessage);
        }
    })
}

pub fn on_create_branch(user_state: UseReducerHandle<OptionalUserState>) -> Callback<Uuid> {
    Callback::from(move |message_id: Uuid| {
        if user_state.state.is_none() {
            return;
        }
        info!("Creating branch from message: {}", message_id);
        user_state.dispatch(UserStateAction::Branch(BranchAction::Create(message_id)));
    })
}

pub fn on_switch_branch(user_state: UseReducerHandle<OptionalUserState>) -> Callback<Uuid> {
    Callback::from(move |branch_id: Uuid| {
        if user_state.state.is_none() {
            return;
        }
        info!("Switching to branch: {}", branch_id);
        user_state.dispatch(UserStateAction::Branch(BranchAction::Switch(branch_id)));
    })
}

pub fn on_delete_branch(user_state: UseReducerHandle<OptionalUserState>) -> Callback<Uuid> {
    Callback::from(move |branch_id: Uuid| {
        if user_state.state.is_none() {
            return;
        }
        info!("Deleting branch: {}", branch_id);
        user_state.dispatch(UserStateAction::Branch(BranchAction::Delete(branch_id)));
    })
}

pub fn on_add_goal(user_state: UseReducerHandle<OptionalUserState>) -> Callback<String> {
    Callback::from(move |goal: String| {
        let state = match user_state.state.as_ref() {
            Some(s) => s,
            None => return,
        };
        let learning_goal = LearningGoal {
            goal,
            dialect: state.selected_dialect,
        };
        info!("Adding learning goal: {:?}", learning_goal);
        user_state.dispatch(UserStateAction::Learning(LearningAction::AddGoal(
            learning_goal,
        )));
    })
}

pub fn on_delete_goal(user_state: UseReducerHandle<OptionalUserState>) -> Callback<usize> {
    Callback::from(move |index: usize| {
        if user_state.state.is_none() {
            return;
        }
        info!("Deleting learning goal at index: {}", index);
        user_state.dispatch(UserStateAction::Learning(LearningAction::DeleteGoal(index)));
    })
}

pub fn on_delete_learning_item_callback(
    ui_state: UseReducerHandle<UIState>,
    user_state: UseReducerHandle<OptionalUserState>,
) -> Callback<Uuid> {
    Callback::from(move |id: Uuid| {
        if let Some(state) = user_state.state.as_ref() {
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
            user_state.dispatch(UserStateAction::Learning(LearningAction::DeleteItem(id)));
        }
    })
}

pub fn on_arabic_script_change(user_state: UseReducerHandle<OptionalUserState>) -> Callback<Event> {
    let user_state = user_state.clone();
    Callback::from(move |e: Event| {
        if user_state.state.is_none() {
            return;
        }
        if let Some(select) = e.target_dyn_into::<web_sys::HtmlSelectElement>() {
            let script = match select.value().as_str() {
                "naskh" => ArabicScript::Naskh,
                "ruqa" => ArabicScript::Ruqa,
                "latin" => ArabicScript::Latin,
                _ => ArabicScript::Naskh,
            };
            user_state.dispatch(UserStateAction::Settings(SettingsAction::SetArabicScript(script)));
        }
    })
}

pub fn on_japanese_script_change(
    user_state: UseReducerHandle<OptionalUserState>,
) -> Callback<Event> {
    let user_state = user_state.clone();
    Callback::from(move |e: Event| {
        if user_state.state.is_none() {
            return;
        }
        if let Some(select) = e.target_dyn_into::<web_sys::HtmlSelectElement>() {
            let script = match select.value().as_str() {
                "romaji" => JapaneseScript::Romaji,
                "only_kana" => JapaneseScript::OnlyKana,
                "kanji_with_ruby" => JapaneseScript::KanjiWithRuby,
                "kanji" => JapaneseScript::Kanji,
                _ => JapaneseScript::KanjiWithRuby,
            };
            user_state.dispatch(UserStateAction::Settings(SettingsAction::SetJapaneseScript(script)));
        }
    })
}

// Strict Callbacks

use crate::app::app_state::UserDomainAction;
use dialect_coach_shared::UserState;
use std::rc::Rc;

pub fn on_language_change_strict(dispatch: Callback<UserDomainAction>) -> Callback<Event> {
    Callback::from(move |e: Event| {
        if let Some(select) = e.target_dyn_into::<web_sys::HtmlSelectElement>() {
            let value = select.value();
            let lang = match value.as_str() {
                "spanish" => Language::Spanish,
                "arabic" => Language::Arabic,
                "french" => Language::French,
                "english" => Language::English,
                "japanese" => Language::Japanese,
                _ => Language::Spanish,
            };
            dispatch.emit(UserDomainAction::Settings(SettingsAction::ChangeLanguage(lang)));
        }
    })
}

pub fn on_dialect_change_strict(
    user: Rc<UserState>,
    dispatch: Callback<UserDomainAction>,
) -> Callback<Event> {
    Callback::from(move |e: Event| {
        if let Some(select) = e.target_dyn_into::<web_sys::HtmlSelectElement>() {
            let value = select.value();
            // Get all dialects for current language and find matching one
            let dialects = user.current_dialects();
            if let Some(dialect_features) = dialects.iter().find(|df| df.dialect.id() == value) {
                dispatch.emit(UserDomainAction::Settings(SettingsAction::ChangeDialect(
                    dialect_features.dialect,
                )));
            }
        }
    })
}

pub fn on_formality_change_strict(dispatch: Callback<UserDomainAction>) -> Callback<Event> {
    Callback::from(move |e: Event| {
        if let Some(select) = e.target_dyn_into::<web_sys::HtmlSelectElement>() {
            let value = select.value();
            let f = match value.as_str() {
                "formal" => Formality::Formal,
                "professional_casual" => Formality::ProfessionalCasual,
                "informal" => Formality::Informal,
                "slang" => Formality::Slang,
                _ => Formality::Informal,
            };
            dispatch.emit(UserDomainAction::Settings(SettingsAction::ChangeFormality(f)));
        }
    })
}

pub fn on_teaching_mode_change_strict(dispatch: Callback<UserDomainAction>) -> Callback<Event> {
    Callback::from(move |e: Event| {
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
            dispatch.emit(UserDomainAction::Settings(SettingsAction::ChangeTeachingMode(tm)));
        }
    })
}

pub fn on_user_gender_change_strict(dispatch: Callback<UserDomainAction>) -> Callback<Event> {
    Callback::from(move |e: Event| {
        if let Some(select) = e.target_dyn_into::<web_sys::HtmlSelectElement>() {
            let gender = match select.value().as_str() {
                "male" => UserGender::Male,
                "female" => UserGender::Female,
                "nonbinary" => UserGender::NonBinary,
                _ => UserGender::NonBinary,
            };
            dispatch.emit(UserDomainAction::Settings(SettingsAction::UpdateGender(gender)));
        }
    })
}

pub fn on_language_level_change_strict(
    user: Rc<UserState>,
    dispatch: Callback<UserDomainAction>,
) -> Callback<Event> {
    Callback::from(move |e: Event| {
        if let Some(select) = e.target_dyn_into::<web_sys::HtmlSelectElement>() {
            let level = match select.value().as_str() {
                "a1" => LanguageLevel::A1,
                "a2" => LanguageLevel::A2,
                "b1" => LanguageLevel::B1,
                "b2" => LanguageLevel::B2,
                "c1" => LanguageLevel::C1,
                "c2" => LanguageLevel::C2,
                "a25" => LanguageLevel::A2,
                _ => LanguageLevel::B1,
            };
            let dialect = user.selected_dialect;
            dispatch.emit(UserDomainAction::Settings(SettingsAction::UpdateLevel(
                dialect, level,
            )));
        }
    })
}

pub fn on_dialect_cycle_strict(
    _user: Rc<UserState>, // Kept for consistency if needed, but cycle logic is in reducer? 
    // Wait, old `on_dialect_cycle`: `if user_state.state.is_some()`.
    // It does not read state. It just dispatches.
    dispatch: Callback<UserDomainAction>,
) -> Callback<()> {
    Callback::from(move |_| {
        dispatch.emit(UserDomainAction::Settings(SettingsAction::CycleDialect));
    })
}

pub fn on_formality_cycle_strict(dispatch: Callback<UserDomainAction>) -> Callback<()> {
    Callback::from(move |_| {
        dispatch.emit(UserDomainAction::Settings(SettingsAction::CycleFormality));
    })
}

pub fn on_teaching_mode_cycle_strict(dispatch: Callback<UserDomainAction>) -> Callback<()> {
    Callback::from(move |_| {
        dispatch.emit(UserDomainAction::Settings(SettingsAction::CycleTeachingMode));
    })
}

pub fn on_add_goal_strict(
    user: Rc<UserState>,
    dispatch: Callback<UserDomainAction>,
) -> Callback<String> {
    Callback::from(move |goal: String| {
        let learning_goal = LearningGoal {
            goal,
            dialect: user.selected_dialect,
        };
        info!("Adding learning goal: {:?}", learning_goal);
        dispatch.emit(UserDomainAction::Learning(LearningAction::AddGoal(
            learning_goal,
        )));
    })
}

pub fn on_delete_goal_strict(dispatch: Callback<UserDomainAction>) -> Callback<usize> {
    Callback::from(move |index: usize| {
        info!("Deleting learning goal at index: {}", index);
        dispatch.emit(UserDomainAction::Learning(LearningAction::DeleteGoal(index)));
    })
}

pub fn on_delete_learning_item_callback_strict(
    ui_state: UseReducerHandle<UIState>,
    user: Rc<UserState>,
    dispatch: Callback<UserDomainAction>,
) -> Callback<Uuid> {
    Callback::from(move |id: Uuid| {
        if let Some(item) = user.learning_items.iter().find(|i| {
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
        dispatch.emit(UserDomainAction::Learning(LearningAction::DeleteItem(id)));
    })
}

pub fn on_undo_message_callback_strict(
    ui_state: UseReducerHandle<UIState>,
    dispatch: Callback<UserDomainAction>,
) -> Callback<()> {
    let deleted_messages = ui_state.deleted_messages.clone();
    Callback::from(move |_| {
        if let Some(msg) = deleted_messages.back().cloned() {
            dispatch.emit(UserDomainAction::Message(MessageAction::UndoDelete(msg)));
            ui_state.dispatch(UIStateAction::PopDeletedMessage);
        }
    })
}

pub fn on_arabic_script_change_strict(dispatch: Callback<UserDomainAction>) -> Callback<Event> {
    Callback::from(move |e: Event| {
        if let Some(select) = e.target_dyn_into::<web_sys::HtmlSelectElement>() {
            let script = match select.value().as_str() {
                "naskh" => ArabicScript::Naskh,
                "ruqa" => ArabicScript::Ruqa,
                "latin" => ArabicScript::Latin,
                _ => ArabicScript::Naskh,
            };
            dispatch.emit(UserDomainAction::Settings(SettingsAction::SetArabicScript(script)));
        }
    })
}

pub fn on_japanese_script_change_strict(dispatch: Callback<UserDomainAction>) -> Callback<Event> {
    Callback::from(move |e: Event| {
        if let Some(select) = e.target_dyn_into::<web_sys::HtmlSelectElement>() {
            let script = match select.value().as_str() {
                "romaji" => JapaneseScript::Romaji,
                "only_kana" => JapaneseScript::OnlyKana,
                "kanji_with_ruby" => JapaneseScript::KanjiWithRuby,
                "kanji" => JapaneseScript::Kanji,
                _ => JapaneseScript::KanjiWithRuby,
            };
            dispatch.emit(UserDomainAction::Settings(SettingsAction::SetJapaneseScript(script)));
        }
    })
}
