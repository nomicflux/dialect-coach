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
}

impl PartialEq for SettingsProps {
    fn eq(&self, other: &Self) -> bool {
        self.user == other.user
            && self.is_admin == other.is_admin
            && self.dispatch == other.dispatch
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

#[function_component(Settings)]
pub fn settings(props: &SettingsProps) -> Html {
    let SettingsProps {
        user,
        is_admin,
        dispatch,
    } = props;

    let us = user;

    html! {
        <div class="settings-content" style="padding: var(--s-5);">
            <div class="panel-section">
                <h4 class="panel-section-title">{"Language & Dialect"}</h4>
                <div class="panel-section-description">{"Choose your target language and regional variety"}</div>

                <div class="field-group">
                    <div class="panel-field">
                        <label for="language-select">{"Language"}</label>
                        <select id="language-select" onchange={on_language_change(dispatch.clone())}>
                            <option value="spanish" selected={us.selected_language == Language::Spanish}>{"Spanish"}</option>
                            <option value="arabic" selected={us.selected_language == Language::Arabic}>{"Arabic"}</option>
                            <option value="french" selected={us.selected_language == Language::French}>{"French"}</option>
                            <option value="english" selected={us.selected_language == Language::English}>{"English"}</option>
                            <option value="japanese" selected={us.selected_language == Language::Japanese}>{"Japanese"}</option>
                        </select>
                    </div>

                    <div class="panel-field">
                        <label for="dialect-select">{"Dialect"}</label>
                        <select id="dialect-select" onchange={on_dialect_change(user.clone(), dispatch.clone())}>
                            {{
                                let dialects = us.current_dialects();
                                let current = us.current_dialect();
                                dialects.iter().map(|dialect_features| {
                                    let is_selected = dialect_features.dialect == current;
                                    let tts_indicator = if dialect_features.has_tts() { "🔊" } else { "" };
                                    let corpus_indicator = if dialect_features.has_corpus { "📚" } else { "" };
                                    html! {
                                        <option value={dialect_features.dialect.id()} selected={is_selected}>
                                            {format!("{} {} {}", dialect_features.dialect.name(), tts_indicator, corpus_indicator)}
                                        </option>
                                    }
                                }).collect::<Html>()
                            }}
                        </select>
                        <div class="field-help field-help--info">
                            {"Regional variety affects accent, vocabulary, and expressions"}
                        </div>
                    </div>

                    {render_experimental_dialects_toggle(us.show_experimental_dialects, dispatch.clone())}
                </div>
            </div>

            <div class="panel-section">
                <h4 class="panel-section-title">{"Conversation Style"}</h4>

                <div class="field-group">
                    <div class="panel-field">
                        <label for="formality-select">{"Formality Level"}</label>
                        <select id="formality-select" onchange={on_formality_change(dispatch.clone())}>
                            <option value="formal" selected={us.formality == Formality::Formal}>{"Formal"}</option>
                            <option value="professional_casual" selected={us.formality == Formality::ProfessionalCasual}>{"Professional Casual"}</option>
                            <option value="informal" selected={us.formality == Formality::Informal}>{"Informal"}</option>
                            <option value="slang" selected={us.formality == Formality::Slang}>{"Slang"}</option>
                        </select>
                    </div>

                    <div class="panel-field">
                        <label for="teaching-mode-select">{"Teaching Mode"}</label>
                        <select id="teaching-mode-select" onchange={on_teaching_mode_change(dispatch.clone())}>
                            <option value="immersive" selected={us.teaching_mode == TeachingMode::Immersive}>{"Immersive"}</option>
                            <option value="corrective" selected={us.teaching_mode == TeachingMode::Corrective}>{"Corrective"}</option>
                            <option value="explanatory" selected={us.teaching_mode == TeachingMode::Explanatory}>{"Explanatory"}</option>
                            <option value="storyteller" selected={us.teaching_mode == TeachingMode::StoryTeller}>{"Story Teller"}</option>
                            <option value="error_finding" selected={us.teaching_mode == TeachingMode::ErrorFinding}>{"Find Agent Errors"}</option>
                            {if *is_admin {
                                html! { <option value="debug" selected={us.teaching_mode == TeachingMode::Debug}>{"Debug"}</option> }
                            } else {
                                html! {}
                            }}
                        </select>
                    </div>

                    <div class="panel-field">
                        <label for="user-gender-select">{"Your Gender"}</label>
                        <select id="user-gender-select" onchange={on_user_gender_change(dispatch.clone())}>
                            <option value="male" selected={us.user_gender == UserGender::Male}>{"Male"}</option>
                            <option value="female" selected={us.user_gender == UserGender::Female}>{"Female"}</option>
                            <option value="nonbinary" selected={us.user_gender == UserGender::NonBinary}>{"Non-binary"}</option>
                        </select>
                    </div>

                    <div class="panel-field">
                        <label for="language-level-select">{"Your Level"}</label>
                        <select id="language-level-select" onchange={on_language_level_change(user.clone(), dispatch.clone())}>
                            <option value="a1" selected={us.current_language_level() == LanguageLevel::A1}>{"A1 - Beginner"}</option>
                            <option value="a2" selected={us.current_language_level() == LanguageLevel::A2}>{"A2 - Elementary"}</option>
                            <option value="b1" selected={us.current_language_level() == LanguageLevel::B1}>{"B1 - Intermediate"}</option>
                            <option value="b2" selected={us.current_language_level() == LanguageLevel::B2}>{"B2 - Upper Intermediate"}</option>
                            <option value="c1" selected={us.current_language_level() == LanguageLevel::C1}>{"C1 - Advanced"}</option>
                            <option value="c2" selected={us.current_language_level() == LanguageLevel::C2}>{"C2 - Proficient"}</option>
                        </select>
                        <div class="field-help field-help--info">
                            {"Your proficiency level in the selected dialect (CEFR scale)"}
                        </div>
                    </div>
                </div>
            </div>

            {match us.selected_language {
                Language::Arabic => {
                    html! {
                        <div class="panel-section">
                            <h4 class="panel-section-title">{"Display Options"}</h4>
                            <div class="field-group">
                                <div class="panel-field">
                                    <label for="arabic-script-select">{"Script:"}</label>
                                    <select
                                        id="arabic-script-select"
                                        onchange={on_arabic_script_change(dispatch.clone())}
                                        value={us.language_options.arabic_script.to_string()}
                                    >
                                        <option value="naskh" selected={us.language_options.arabic_script == ArabicScript::Naskh}>{"Naskh"}</option>
                                        <option value="ruqa" selected={us.language_options.arabic_script == ArabicScript::Ruqa}>{"Ruq'a"}</option>
                                        <option value="latin" selected={us.language_options.arabic_script == ArabicScript::Latin}>{"Latin (Romanized)"}</option>
                                    </select>
                                </div>
                            </div>
                        </div>
                    }
                }
                Language::Japanese => {
                    html! {
                        <div class="panel-section">
                            <h4 class="panel-section-title">{"Display Options"}</h4>
                            <div class="field-group">
                                <div class="panel-field">
                                    <label for="japanese-script-select">{"Script:"}</label>
                                    <select
                                        id="japanese-script-select"
                                        onchange={on_japanese_script_change(dispatch.clone())}
                                        value={us.language_options.japanese_script.to_string()}
                                    >
                                        <option value="romaji" selected={us.language_options.japanese_script == JapaneseScript::Romaji}>{"Romaji"}</option>
                                        <option value="only_kana" selected={us.language_options.japanese_script == JapaneseScript::OnlyKana}>{"Kana Only"}</option>
                                        <option value="kanji_with_ruby" selected={us.language_options.japanese_script == JapaneseScript::KanjiWithRuby}>{"Kanji with Furigana"}</option>
                                        <option value="kanji" selected={us.language_options.japanese_script == JapaneseScript::Kanji}>{"Kanji"}</option>
                                    </select>
                                </div>
                            </div>
                        </div>
                    }
                }
                _ => {
                    html! {}
                }
            }}
        </div>
    }
}
