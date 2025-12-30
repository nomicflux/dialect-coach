use crate::app::app_state::user::UserDomainAction;
use crate::app::app_state::{
    BranchAction, LearningAction, MessageAction, SettingsAction, UIState, UIStateAction,
};
use dialect_coach_shared::UserState;
use dialect_coach_shared::models::{
    ArabicScript, Dialect, Formality, JapaneseScript, Language, LanguageLevel, TeachingMode,
    UserGender,
};
use log::info;
use std::rc::Rc;
use uuid::Uuid;
use yew::prelude::*;

pub fn on_language_change(dispatch: Callback<UserDomainAction>) -> Callback<Event> {
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
            dispatch.emit(UserDomainAction::Settings(SettingsAction::ChangeLanguage(
                lang,
            )));
        }
    })
}

pub fn on_dialect_change(dispatch: Callback<UserDomainAction>) -> Callback<Event> {
    Callback::from(move |e: Event| {
        if let Some(select) = e.target_dyn_into::<web_sys::HtmlSelectElement>()
            && let Some(dialect) = Dialect::from_id(&select.value())
        {
            dispatch.emit(UserDomainAction::Settings(SettingsAction::ChangeDialect(
                dialect,
            )));
        }
    })
}

pub fn on_formality_change(dispatch: Callback<UserDomainAction>) -> Callback<Event> {
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
            dispatch.emit(UserDomainAction::Settings(SettingsAction::ChangeFormality(
                f,
            )));
        }
    })
}

pub fn on_teaching_mode_change(dispatch: Callback<UserDomainAction>) -> Callback<Event> {
    Callback::from(move |e: Event| {
        if let Some(select) = e.target_dyn_into::<web_sys::HtmlSelectElement>() {
            let value = select.value();
            let tm = match value.as_str() {
                "immersive" => TeachingMode::Immersive,
                "corrective" => TeachingMode::Corrective,
                "explanatory" => TeachingMode::Explanatory,
                "storyteller" => TeachingMode::StoryTeller,
                "error_finding" => TeachingMode::ErrorFinding,
                "debug" => TeachingMode::Debug,
                _ => TeachingMode::Immersive,
            };
            dispatch.emit(UserDomainAction::Settings(
                SettingsAction::ChangeTeachingMode(tm),
            ));
        }
    })
}

pub fn on_user_gender_change(dispatch: Callback<UserDomainAction>) -> Callback<Event> {
    Callback::from(move |e: Event| {
        if let Some(select) = e.target_dyn_into::<web_sys::HtmlSelectElement>() {
            let gender = match select.value().as_str() {
                "male" => UserGender::Male,
                "female" => UserGender::Female,
                "nonbinary" => UserGender::NonBinary,
                _ => UserGender::NonBinary,
            };
            dispatch.emit(UserDomainAction::Settings(SettingsAction::UpdateGender(
                gender,
            )));
        }
    })
}

pub fn on_language_level_change(
    dispatch: Callback<UserDomainAction>,
) -> Callback<Event> {
    Callback::from(move |e: Event| {
        if let Some(select) = e.target_dyn_into::<web_sys::HtmlSelectElement>()
            && let Some(level) = LanguageLevel::from_id(&select.value())
        {
            dispatch.emit(UserDomainAction::Settings(SettingsAction::UpdateLevel(level)));
        }
    })
}

pub fn on_dialect_cycle(dispatch: Callback<UserDomainAction>) -> Callback<()> {
    Callback::from(move |_| {
        dispatch.emit(UserDomainAction::Settings(SettingsAction::CycleDialect));
    })
}

pub fn on_formality_cycle(dispatch: Callback<UserDomainAction>) -> Callback<()> {
    Callback::from(move |_| {
        dispatch.emit(UserDomainAction::Settings(SettingsAction::CycleFormality));
    })
}

pub fn on_teaching_mode_cycle(
    is_admin: bool,
    dispatch: Callback<UserDomainAction>,
) -> Callback<()> {
    Callback::from(move |_| {
        dispatch.emit(UserDomainAction::Settings(
            SettingsAction::CycleTeachingMode(is_admin),
        ));
    })
}

pub fn on_add_goal(dispatch: Callback<UserDomainAction>) -> Callback<String> {
    Callback::from(move |goal: String| {
        info!("Adding learning goal: {}", goal);
        dispatch.emit(UserDomainAction::Learning(LearningAction::AddGoal(goal)));
    })
}

pub fn on_delete_goal(dispatch: Callback<UserDomainAction>) -> Callback<usize> {
    Callback::from(move |index: usize| {
        info!("Deleting learning goal at index: {}", index);
        dispatch.emit(UserDomainAction::Learning(LearningAction::DeleteGoal(
            index,
        )));
    })
}

pub fn on_delete_learning_item_callback(
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

pub fn on_undo_message_callback(
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

pub fn on_arabic_script_change(dispatch: Callback<UserDomainAction>) -> Callback<Event> {
    Callback::from(move |e: Event| {
        if let Some(select) = e.target_dyn_into::<web_sys::HtmlSelectElement>() {
            let script = match select.value().as_str() {
                "naskh" => ArabicScript::Naskh,
                "ruqa" => ArabicScript::Ruqa,
                "latin" => ArabicScript::Latin,
                _ => ArabicScript::Naskh,
            };
            dispatch.emit(UserDomainAction::Settings(SettingsAction::SetArabicScript(
                script,
            )));
        }
    })
}

pub fn on_japanese_script_change(dispatch: Callback<UserDomainAction>) -> Callback<Event> {
    Callback::from(move |e: Event| {
        if let Some(select) = e.target_dyn_into::<web_sys::HtmlSelectElement>() {
            let script = match select.value().as_str() {
                "romaji" => JapaneseScript::Romaji,
                "only_kana" => JapaneseScript::OnlyKana,
                "kanji_with_ruby" => JapaneseScript::KanjiWithRuby,
                "kanji" => JapaneseScript::Kanji,
                _ => JapaneseScript::KanjiWithRuby,
            };
            dispatch.emit(UserDomainAction::Settings(
                SettingsAction::SetJapaneseScript(script),
            ));
        }
    })
}

pub fn on_create_branch(dispatch: Callback<UserDomainAction>) -> Callback<Uuid> {
    Callback::from(move |message_id: Uuid| {
        info!("Creating branch from message: {}", message_id);
        dispatch.emit(UserDomainAction::Branch(BranchAction::Create(message_id)));
    })
}

pub fn on_switch_branch(dispatch: Callback<UserDomainAction>) -> Callback<Uuid> {
    Callback::from(move |branch_id: Uuid| {
        info!("Switching to branch: {}", branch_id);
        dispatch.emit(UserDomainAction::Branch(BranchAction::Switch(branch_id)));
    })
}

pub fn on_delete_branch(dispatch: Callback<UserDomainAction>) -> Callback<Uuid> {
    Callback::from(move |branch_id: Uuid| {
        info!("Deleting branch: {}", branch_id);
        dispatch.emit(UserDomainAction::Branch(BranchAction::Delete(branch_id)));
    })
}

pub fn on_delete_message_callback(
    ui_state: UseReducerHandle<UIState>,
    user: Rc<UserState>,
    dispatch: Callback<UserDomainAction>,
) -> Callback<Uuid> {
    Callback::from(move |msg_id: Uuid| {
        if let Some(msg) = user
            .conversation_history
            .iter()
            .find(|m| m.id == msg_id)
            .cloned()
        {
            ui_state.dispatch(UIStateAction::PushDeletedMessage(msg));
        }
        dispatch.emit(UserDomainAction::Message(MessageAction::Delete(msg_id)));
    })
}

pub fn on_activate_step(dispatch: Callback<UserDomainAction>) -> Callback<Uuid> {
    use crate::app::app_state::user::PlanAction;
    Callback::from(move |plan_id: Uuid| {
        info!("Activating step for plan: {}", plan_id);
        dispatch.emit(UserDomainAction::Plan(PlanAction::ActivateStep(plan_id)));
    })
}
