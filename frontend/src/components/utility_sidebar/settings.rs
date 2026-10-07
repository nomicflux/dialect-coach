use crate::app::app_state::SettingsAction;
use crate::app::app_state::user::UserDomainAction;
use crate::app::user_state_callbacks::{
    on_arabic_script_change, on_dialect_change, on_formality_change, on_japanese_script_change,
    on_language_change, on_language_level_change, on_teaching_mode_change, on_user_gender_change,
};
use dialect_coach_shared::UserState;
use dialect_coach_shared::models::{
    ArabicScript, Formality, JapaneseScript, Language, LanguageLevel, TeachingMode, UserGender,
};
use std::rc::Rc;
use yew::prelude::*;

#[derive(Properties)]
pub struct SettingsProps {
    pub user: Rc<UserState>,
    pub is_admin: bool,
    pub dispatch: Callback<UserDomainAction>,
    pub on_log_out: Callback<MouseEvent>,
}

impl PartialEq for SettingsProps {
    fn eq(&self, other: &Self) -> bool {
        self.user == other.user
            && self.is_admin == other.is_admin
            && self.dispatch == other.dispatch
            && self.on_log_out == other.on_log_out
    }
}

fn render_experimental_dialects_toggle(
    show_experimental: bool,
    dispatch: Callback<UserDomainAction>,
) -> Html {
    html! {
        <div class="panel-field">
            <label class="checkbox-label">
                <input
                    type="checkbox"
                    checked={show_experimental}
                    onchange={
                        Callback::from(move |_| {
                            dispatch.emit(UserDomainAction::Settings(SettingsAction::ToggleExperimentalDialects));
                        })
                    }
                />
                {" Show experimental dialects"}
            </label>
            <div class="field-help field-help--info">
                {"Experimental dialects may have limited features or incomplete voice support"}
            </div>
        </div>
    }
}

fn render_language_field(user: &UserState, dispatch: Callback<UserDomainAction>) -> Html {
    html! {
        <div class="panel-field">
            <label for="language-select">{"Language"}</label>
            <select id="language-select" onchange={on_language_change(dispatch)}>
                <option value="spanish" selected={user.selected_language == Language::Spanish}>{"Spanish"}</option>
                <option value="arabic" selected={user.selected_language == Language::Arabic}>{"Arabic"}</option>
                <option value="french" selected={user.selected_language == Language::French}>{"French"}</option>
                <option value="english" selected={user.selected_language == Language::English}>{"English"}</option>
                <option value="japanese" selected={user.selected_language == Language::Japanese}>{"Japanese"}</option>
            </select>
        </div>
    }
}

fn render_dialect_options(user: &UserState) -> Html {
    let current = user.current_dialect();
    user.current_dialects()
        .iter()
        .map(|features| {
            let tts = if features.has_tts() { "🔊" } else { "" };
            let corpus = if features.has_corpus { "📚" } else { "" };
            html! {
                <option value={features.dialect.id()} selected={features.dialect == current}>
                    {format!("{} {} {}", features.dialect.name(), tts, corpus)}
                </option>
            }
        })
        .collect()
}

fn render_dialect_field(user: &UserState, dispatch: Callback<UserDomainAction>) -> Html {
    html! {
        <div class="panel-field">
            <label for="dialect-select">{"Dialect"}</label>
            <select id="dialect-select" onchange={on_dialect_change(dispatch)}>
                {render_dialect_options(user)}
            </select>
            <div class="field-help field-help--info">{"Regional variety affects accent, vocabulary, and expressions"}</div>
        </div>
    }
}

fn render_language_section(props: &SettingsProps) -> Html {
    html! {
        <div class="panel-section">
            <h4 class="panel-section-title">{"Language & Dialect"}</h4>
            <div class="panel-section-description">{"Choose your target language and regional variety"}</div>
            <div class="field-group">
                {render_language_field(&props.user, props.dispatch.clone())}
                {render_dialect_field(&props.user, props.dispatch.clone())}
                {render_experimental_dialects_toggle(props.user.show_experimental_dialects, props.dispatch.clone())}
            </div>
        </div>
    }
}

fn render_formality_field(user: &UserState, dispatch: Callback<UserDomainAction>) -> Html {
    let formality = user.active_branch_settings().formality;
    html! {
        <div class="panel-field">
            <label for="formality-select">{"Formality Level"}</label>
            <select id="formality-select" onchange={on_formality_change(dispatch)}>
                <option value="formal" selected={formality == Formality::Formal}>{"Formal"}</option>
                <option value="professional_casual" selected={formality == Formality::ProfessionalCasual}>{"Professional Casual"}</option>
                <option value="informal" selected={formality == Formality::Informal}>{"Informal"}</option>
                <option value="slang" selected={formality == Formality::Slang}>{"Slang"}</option>
            </select>
        </div>
    }
}

fn render_teaching_mode_field(props: &SettingsProps) -> Html {
    let mode = props.user.active_branch_settings().teaching_mode;
    html! {
        <div class="panel-field">
            <label for="teaching-mode-select">{"Teaching Mode"}</label>
            <select id="teaching-mode-select" onchange={on_teaching_mode_change(props.dispatch.clone())}>
                <option value="immersive" selected={mode == TeachingMode::Immersive}>{"Immersive"}</option>
                <option value="corrective" selected={mode == TeachingMode::Corrective}>{"Corrective"}</option>
                <option value="explanatory" selected={mode == TeachingMode::Explanatory}>{"Explanatory"}</option>
                <option value="storyteller" selected={mode == TeachingMode::StoryTeller}>{"Story Teller"}</option>
                <option value="error_finding" selected={mode == TeachingMode::ErrorFinding}>{"Find Agent Errors"}</option>
                {if props.is_admin { html! { <option value="debug" selected={mode == TeachingMode::Debug}>{"Debug"}</option> } } else { html! {} }}
            </select>
        </div>
    }
}

fn render_gender_field(user: &UserState, dispatch: Callback<UserDomainAction>) -> Html {
    html! {
        <div class="panel-field">
            <label for="user-gender-select">{"Your Gender"}</label>
            <select id="user-gender-select" onchange={on_user_gender_change(dispatch)}>
                <option value="male" selected={user.user_gender == UserGender::Male}>{"Male"}</option>
                <option value="female" selected={user.user_gender == UserGender::Female}>{"Female"}</option>
                <option value="nonbinary" selected={user.user_gender == UserGender::NonBinary}>{"Non-binary"}</option>
            </select>
        </div>
    }
}

fn render_level_options(user: &UserState) -> Html {
    let current = user.active_branch_settings().language_level;
    LanguageLevel::for_language(user.selected_language).iter().map(|level| {
        html! { <option value={level.id()} selected={*level == current}>{level.name()}</option> }
    }).collect()
}

fn render_level_field(user: &UserState, dispatch: Callback<UserDomainAction>) -> Html {
    html! {
        <div class="panel-field">
            <label for="language-level-select">{"Your Level"}</label>
            <select id="language-level-select" key={user.selected_language.code()}
                value={user.active_branch_settings().language_level.id()} onchange={on_language_level_change(dispatch)}>
                {render_level_options(user)}
            </select>
            <div class="field-help field-help--info">
                {if user.selected_language == Language::Japanese {
                    "Your proficiency level in the selected dialect (JLPT scale)"
                } else { "Your proficiency level in the selected dialect (CEFR scale)" }}
            </div>
        </div>
    }
}

fn render_conversation_section(props: &SettingsProps) -> Html {
    html! {
        <div class="panel-section">
            <h4 class="panel-section-title">{"Conversation Settings"}</h4>
            <div class="panel-section-description">{"These settings apply to the current branch"}</div>
            <div class="field-group">
                {render_formality_field(&props.user, props.dispatch.clone())}
                {render_teaching_mode_field(props)}
                {render_gender_field(&props.user, props.dispatch.clone())}
                {render_level_field(&props.user, props.dispatch.clone())}
            </div>
        </div>
    }
}

fn render_arabic_script(user: &UserState, dispatch: Callback<UserDomainAction>) -> Html {
    let script = user.active_branch_settings().language_options.arabic_script;
    html! {
        <div class="panel-field">
            <label for="arabic-script-select">{"Script:"}</label>
            <select id="arabic-script-select" onchange={on_arabic_script_change(dispatch)} value={script.to_string()}>
                <option value="naskh" selected={script == ArabicScript::Naskh}>{"Naskh"}</option>
                <option value="ruqa" selected={script == ArabicScript::Ruqa}>{"Ruq'a"}</option>
                <option value="latin" selected={script == ArabicScript::Latin}>{"Latin (Romanized)"}</option>
                <option value="fully_voweled" selected={script == ArabicScript::FullyVoweled}>{"Fully Voweled (with Harakat)"}</option>
            </select>
        </div>
    }
}

fn render_japanese_script(user: &UserState, dispatch: Callback<UserDomainAction>) -> Html {
    let script = user
        .active_branch_settings()
        .language_options
        .japanese_script;
    html! {
        <div class="panel-field">
            <label for="japanese-script-select">{"Script:"}</label>
            <select id="japanese-script-select" onchange={on_japanese_script_change(dispatch)} value={script.to_string()}>
                <option value="romaji" selected={script == JapaneseScript::Romaji}>{"Romaji"}</option>
                <option value="only_kana" selected={script == JapaneseScript::OnlyKana}>{"Kana Only"}</option>
                <option value="kanji_with_ruby" selected={script == JapaneseScript::KanjiWithRuby}>{"Kanji with Furigana"}</option>
                <option value="kanji" selected={script == JapaneseScript::Kanji}>{"Kanji"}</option>
            </select>
        </div>
    }
}

fn render_display_options(props: &SettingsProps) -> Html {
    let field = match props.user.selected_language {
        Language::Arabic => render_arabic_script(&props.user, props.dispatch.clone()),
        Language::Japanese => render_japanese_script(&props.user, props.dispatch.clone()),
        _ => return html! {},
    };
    html! {
        <div class="panel-section">
            <h4 class="panel-section-title">{"Display Options"}</h4>
            <div class="field-group">{field}</div>
        </div>
    }
}

fn render_account_section(on_log_out: Callback<MouseEvent>) -> Html {
    html! {
        <div class="panel-section">
            <h4 class="panel-section-title">{"Account"}</h4>
            <button class="signout-button" onclick={on_log_out}>{"Log out"}</button>
        </div>
    }
}

#[function_component(Settings)]
pub fn settings(props: &SettingsProps) -> Html {
    html! {
        <div class="settings-content" style="padding: var(--s-5);">
            {render_language_section(props)}
            {render_conversation_section(props)}
            {render_display_options(props)}
            {render_account_section(props.on_log_out.clone())}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_props_compare_log_out_callback() {
        let user = Rc::new(UserState::new(uuid::Uuid::new_v4()));
        let dispatch = Callback::from(|_| ());
        let first = SettingsProps {
            user: user.clone(),
            is_admin: false,
            dispatch: dispatch.clone(),
            on_log_out: Callback::from(|_| ()),
        };
        let second = SettingsProps {
            user,
            is_admin: false,
            dispatch,
            on_log_out: Callback::from(|_| ()),
        };
        assert!(first != second);
    }
}
