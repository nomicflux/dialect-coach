use crate::app::app_state::{OptionalUserState, UIState, UIStateAction};
use crate::app::user_state_callbacks::{
    on_dialect_change, on_formality_change, on_language_change, on_teaching_mode_change,
};
use dialect_coach_shared::models::Language;
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
                        </div>
                    </div>
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
