use crate::app::app_state::{OptionalUserState, UIState, UIStateAction};
use crate::app::user_state_callbacks::{
    on_arabic_script_change, on_dialect_change, on_formality_change, on_japanese_script_change,
    on_language_change, on_teaching_mode_change, on_user_gender_change,
};
use dialect_coach_shared::models::{ArabicScript, JapaneseScript, Language, UserGender};
use yew::prelude::*;

#[derive(Properties)]
pub struct SettingsPanelProps {
    pub user_state: UseReducerHandle<OptionalUserState>,
    pub ui_state: UseReducerHandle<UIState>,
}

impl PartialEq for SettingsPanelProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[function_component(SettingsPanel)]
pub fn settings_panel(props: &SettingsPanelProps) -> Html {
    let SettingsPanelProps {
        user_state,
        ui_state,
    } = props;

    let us = match user_state.0.as_ref() {
        Some(s) => s,
        None => return html! {},
    };

    html! {
        <>
            // Configuration panel (collapsible)
            <div class="panel" data-open={if ui_state.panel_open { "true" } else { "false" }}>
                <div class="panel-header">
                    <h3 class="panel-title">{"Practice Settings"}</h3>
                    <button class="panel-close" onclick={{
                        let ui_state = ui_state.clone();
                        Callback::from(move |_| {
                            ui_state.dispatch(UIStateAction::ClosePanel);
                        })
                    }}>
                        {"×"}
                    </button>
                </div>

                <div class="panel-content">
                    <div class="panel-section">
                        <h4 class="panel-section-title">{"Language & Dialect"}</h4>
                        <div class="panel-section-description">{"Choose your target language and regional variety"}</div>

                        <div class="field-group">
                            <div class="panel-field">
                                <label for="language-select">{"Language"}</label>
                                <select id="language-select" onchange={on_language_change(user_state.clone())}>
                                    <option value="spanish" selected={us.selected_language == Language::Spanish}>{"Spanish"}</option>
                                    <option value="arabic" selected={us.selected_language == Language::Arabic}>{"Arabic"}</option>
                                    <option value="french" selected={us.selected_language == Language::French}>{"French"}</option>
                                    <option value="english" selected={us.selected_language == Language::English}>{"English"}</option>
                                    <option value="japanese" selected={us.selected_language == Language::Japanese}>{"Japanese"}</option>
                                </select>
                            </div>

                            <div class="panel-field">
                                <label for="dialect-select">{"Dialect"}</label>
                                <select id="dialect-select" onchange={on_dialect_change(user_state.clone())}>
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
                        </div>
                    </div>

                    <div class="panel-section">
                        <h4 class="panel-section-title">{"Conversation Style"}</h4>

                        <div class="field-group">
                            <div class="panel-field">
                                <label for="formality-select">{"Formality Level"}</label>
                                <select id="formality-select" onchange={on_formality_change(user_state.clone())}>
                                    <option value="formal">{"Formal"}</option>
                                    <option value="casual" selected=true>{"Casual"}</option>
                                    <option value="dialect_rich">{"Dialect-Rich"}</option>
                                    <option value="slang">{"Slang"}</option>
                                </select>
                            </div>

                            <div class="panel-field">
                                <label for="teaching-mode-select">{"Teaching Mode"}</label>
                                <select id="teaching-mode-select" onchange={on_teaching_mode_change(user_state.clone())}>
                                    <option value="immersive" selected=true>{"Immersive"}</option>
                                    <option value="corrective">{"Corrective"}</option>
                                    <option value="explanatory">{"Explanatory"}</option>
                                    <option value="interleaved">{"Interleaved"}</option>
                                    <option value="storyteller">{"Story Teller"}</option>
                                    <option value="debug">{"Debug"}</option>
                                </select>
                            </div>

                            <div class="panel-field">
                                <label for="user-gender-select">{"Your Gender"}</label>
                                <select id="user-gender-select" onchange={on_user_gender_change(user_state.clone())}>
                                    <option value="male" selected={us.user_gender == UserGender::Male}>{"Male"}</option>
                                    <option value="female" selected={us.user_gender == UserGender::Female}>{"Female"}</option>
                                    <option value="nonbinary" selected={us.user_gender == UserGender::NonBinary}>{"Non-binary"}</option>
                                </select>
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
                                                onchange={on_arabic_script_change(user_state.clone())}
                                                value={us.language_options.arabic_script.unwrap_or_default().to_string()}
                                            >
                                                <option value="naskh" selected={matches!(us.language_options.arabic_script, Some(ArabicScript::Naskh)) || us.language_options.arabic_script.is_none()}>{"Naskh"}</option>
                                                <option value="ruqa" selected={matches!(us.language_options.arabic_script, Some(ArabicScript::Ruqa))}>{"Ruq'a"}</option>
                                                <option value="latin" selected={matches!(us.language_options.arabic_script, Some(ArabicScript::Latin))}>{"Latin (Romanized)"}</option>
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
                                                onchange={on_japanese_script_change(user_state.clone())}
                                                value={us.language_options.japanese_script.unwrap_or_default().to_string()}
                                            >
                                                <option value="romaji" selected={matches!(us.language_options.japanese_script, Some(JapaneseScript::Romaji))}>{"Romaji"}</option>
                                                <option value="only_kana" selected={matches!(us.language_options.japanese_script, Some(JapaneseScript::OnlyKana))}>{"Kana Only"}</option>
                                                <option value="kanji_with_ruby" selected={matches!(us.language_options.japanese_script, Some(JapaneseScript::KanjiWithRuby)) || us.language_options.japanese_script.is_none()}>{"Kanji with Furigana"}</option>
                                                <option value="kanji" selected={matches!(us.language_options.japanese_script, Some(JapaneseScript::Kanji))}>{"Kanji"}</option>
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
            </div>

            // Panel backdrop
            <div class="panel-backdrop" data-open={if ui_state.panel_open { "true" } else { "false" }} onclick={{
                let ui_state = ui_state.clone();
                Callback::from(move |_| {
                    ui_state.dispatch(UIStateAction::ClosePanel);
                })
            }}></div>
        </>
    }
}
