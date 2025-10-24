mod app_state;
use app_state::{AppState, UIState, AppStateAction, UIStateAction};
pub use app_state::{LearningItem, LearningItemType};

use dialect_coach_shared::models::{Formality, Language, Message, TeachingMode};
use log::{error, info};
use yew::prelude::*;

use crate::components::{ChatWindow, InputBox, LearningPanel, SpeechControls};
use crate::services::websocket::{ConnectionState};

fn on_send_message(app_state: UseReducerHandle<AppState>) -> Callback<String> {
    let app_state = app_state.clone();

    Callback::from(move |content: String| {
        info!("Sending message: {}", content);

        let msg = (*app_state).create_msg(&content);
        app_state.dispatch(AppStateAction::AddMessage(msg.clone()));

        // Send through WebSocket
        match (*app_state).ws_service.borrow().send_message(&msg) {
            Ok(_) => {
                info!("Message sent successfully");
                app_state.dispatch(AppStateAction::SetLoading);
                app_state.dispatch(AppStateAction::ClearError);
            }
            Err(e) => {
                error!("Failed to send message: {}", e);
                app_state.dispatch(AppStateAction::SetError(format!("Failed to send: {}", e)));
            }
        }
    })
}

fn on_prompt_click(
    app_state: UseReducerHandle<AppState>,
    ui_state: UseReducerHandle<UIState>,
) -> Callback<(String, String)> {
    let app_state = app_state.clone();
    let ui_state = ui_state.clone();

    Callback::from(move |(button_id, english_phrase): (String, String)| {
        info!(
            "Translating prompt from button '{}': {}",
            button_id, english_phrase
        );

        // Set loading state for specific button
        ui_state.dispatch(UIStateAction::PushTranslatingButton(button_id));
        app_state.dispatch(AppStateAction::ClearError);

        let ui_state = ui_state.clone();
        let app_state = app_state.clone();

        // Start async translation
        wasm_bindgen_futures::spawn_local(async move {
            match app_state
                .translation_service
                .translate_phrase(
                    &english_phrase,
                    app_state.current_dialect(),
                    Some(app_state.current_formality()),
                )
                .await
            {
                Ok(translated) => {
                    info!(
                        "Translation success: '{}' -> '{}'",
                        english_phrase, translated
                    );
                    ui_state.dispatch(UIStateAction::EnterInputPrompt(translated));
                    ui_state.dispatch(UIStateAction::ClearTranslatingButton);
                }
                Err(e) => {
                    error!("Translation failed: {}", e);
                    // Fallback to English phrase on error
                    ui_state.dispatch(UIStateAction::EnterInputPrompt(english_phrase));
                    app_state.dispatch(AppStateAction::SetError(format!(
                        "Translation failed, using English phrase: {}",
                        e
                    )));
                    ui_state.dispatch(UIStateAction::ClearTranslatingButton);
                }
            }
        });
    })
}

fn on_language_change(app_state: UseReducerHandle<AppState>) -> Callback<Event> {
    let app_state = app_state.clone();
    Callback::from(move |e: Event| {
        if let Some(select) = e.target_dyn_into::<web_sys::HtmlSelectElement>() {
            let value = select.value();
            let lang = match value.as_str() {
                "spanish" => Language::Spanish,
                "arabic" => Language::Arabic,
                "french" => Language::French,
                _ => Language::Spanish,
            };
            app_state.dispatch(AppStateAction::ChangeLanguage(lang));
        }
    })
}

fn on_dialect_change(app_state: UseReducerHandle<AppState>) -> Callback<Event> {
    let app_state = app_state.clone();
    Callback::from(move |e: Event| {
        if let Some(select) = e.target_dyn_into::<web_sys::HtmlSelectElement>() {
            let value = select.value();

            // Get all dialects for current language and find matching one
            let dialects = app_state.current_dialects();
            if let Some(dialect) = dialects.iter().find(|d| d.id() == value) {
                app_state.dispatch(AppStateAction::ChangeDialect(dialect.clone()));
            }
        }
    })
}

fn on_formality_change(app_state: UseReducerHandle<AppState>) -> Callback<Event> {
    let app_state = app_state.clone();

    Callback::from(move |e: Event| {
        if let Some(select) = e.target_dyn_into::<web_sys::HtmlSelectElement>() {
            let value = select.value();
            let f = match value.as_str() {
                "formal" => Formality::Formal,
                "casual" => Formality::Casual,
                "dialect_rich" => Formality::DialectRich,
                "slang" => Formality::Slang,
                _ => Formality::Casual,
            };
            app_state.dispatch(AppStateAction::ChangeFormality(f));
        }
    })
}

fn on_teaching_mode_change(app_state: UseReducerHandle<AppState>) -> Callback<Event> {
    let app_state = app_state.clone();

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
            app_state.dispatch(AppStateAction::ChangeTeachingMode(tm));
        }
    })
}

fn on_replay_message(app_state: UseReducerHandle<AppState>) -> Callback<Message> {
    let app_state = app_state.clone();

    Callback::from(move |msg: Message| {
        info!("Replaying message with TTS: {}", msg.content.response);
        app_state.dispatch(AppStateAction::Speak(msg));
    })
}

fn on_dialect_cycle(app_state: UseReducerHandle<AppState>) -> Callback<()> {
    let app_state = app_state.clone();

    Callback::from(move |_| {
        app_state.dispatch(AppStateAction::RotateDialect);
    })
}

#[function_component(App)]
pub fn app() -> Html {
    let app_state = use_reducer(AppState::default);
    let ui_state = use_reducer(UIState::default);

    {
        let app_state = app_state.clone();
        let ui_state = ui_state.clone();
        use_effect_with((), move |_| {
            info!("Initializing WebSocket connection");

            {
                let mut ws = (*app_state).ws_service.borrow_mut();

                // Set up callbacks
                ws.set_on_open(Callback::from(move |_| {
                    info!("WebSocket opened");
                }));

                let asc = app_state.clone();
                ws.set_on_close(Callback::from(move |_| {
                    info!("WebSocket closed");
                    asc.dispatch(AppStateAction::SetError("Connection closed".to_string()));
                }));

                let asc = app_state.clone();
                ws.set_on_error(Callback::from(move |err| {
                    error!("WebSocket error: {}", err);
                    asc.dispatch(AppStateAction::SetError(err));
                }));

                let asc = app_state.clone();
                let usc = ui_state.clone();
                ws.set_on_message(Callback::from(move |msg: Message| {
                    info!("Received message from: {}", msg.participant_id);
                    asc.dispatch(AppStateAction::LoadingComplete);

                    if msg.participant_id != "user" {
                        asc.dispatch(AppStateAction::Speak(msg.clone()));
                    }

                    let mistakes = msg.content.mistakes.clone().unwrap_or_default();
                    let explained = msg.content.explained.clone().unwrap_or_default();
                    if !mistakes.is_empty() || !explained.is_empty() {
                        usc.dispatch(UIStateAction::AddLearningItems(mistakes, explained));
                    }

                    asc.dispatch(AppStateAction::AddMessage(msg));
                }));

                let asc = app_state.clone();
                ws.set_on_state_change(Callback::from(move |new_state| {
                    info!("Connection state changed to: {:?}", new_state);
                    asc.dispatch(AppStateAction::SetConnectionState(new_state));

                    // Handle reconnecting state - the WebSocketService will attempt
                    // automatic reconnection, but we need to trigger it from the app layer
                    // since we can't easily call methods from within the async task
                    if matches!(new_state, ConnectionState::Reconnecting) {
                        // Schedule a reconnect attempt
                        let ws_clone = (*asc).ws_service.clone();
                        gloo::timers::callback::Timeout::new(100, move || {
                            info!("Triggering reconnection from app layer");
                            ws_clone.borrow_mut().reconnect();
                        })
                        .forget();
                    }
                }));

                // Connect
                ws.connect();
            } // Drop the borrow here

            // Cleanup on unmount
            let ws_service_clone = (*app_state).ws_service.clone();
            move || {
                info!("Disconnecting WebSocket");
                ws_service_clone.borrow_mut().disconnect();
            }
        });
    }

    html! {
        <div class="app">
            <header class="app-header">
                <div class="container">
                    <div>
                        <h1 class="app-title">{"🎯 Dialect Coach"}</h1>
                        <p class="app-subtitle">{"Practice Spanish, Arabic, and French dialects with AI agents"}</p>
                    </div>

                    // Connection status
                    <div class="connection-status">
                        {match (*app_state).connection_state {
                            ConnectionState::Connected => html! { <span class="status-connected">{"● Ready to chat!"}</span> },
                            ConnectionState::Connecting => html! { <span class="status-connecting">{"⟳ Connecting..."}</span> },
                            ConnectionState::Reconnecting => html! { <span class="status-reconnecting">{"⟳ Reconnecting..."}</span> },
                            ConnectionState::Disconnected => html! { <span class="status-disconnected">{"○ Disconnected"}</span> },
                            ConnectionState::Failed => html! { <span class="status-failed">{"✖ Connection Failed"}</span> },
                        }}
                    </div>
                </div>
            </header>

            <main class="app-main">
                <div class="container">
                    // Main chat card
                    <div class="card card--chat" id="main-chat">
                        // Error display
                        {if let Some(err) = ((*app_state).error_message).as_ref() {
                            html! {
                                <div class="error-banner">
                                    {format!("⚠️ {}", err)}
                                </div>
                            }
                        } else {
                            html! {}
                        }}

                        // Chat interface
                        <ChatWindow
                            messages={app_state.current_messages()}
                            is_loading={(*app_state).is_loading}
                            on_replay_message={Some(on_replay_message(app_state.clone()))}
                            on_prompt_click={Some(on_prompt_click(app_state.clone(), ui_state.clone()))}
                            translating_button={((*ui_state).translating_button).clone()}
                            formality={app_state.current_formality()}
                        />
                        <SpeechControls
                            on_speech={on_send_message(app_state.clone())}
                            language_code={app_state.bcp47_tag()}
                            on_dialect_cycle={Some(on_dialect_cycle(app_state.clone()))}
                        />
                        <InputBox
                            on_send={{
                                let ui_state = ui_state.clone();
                                let send_message = on_send_message(app_state.clone());
                                Callback::from(move |content: String| {
                                    // Clear the prompt value after use
                                    ui_state.dispatch(UIStateAction::ClearInputPrompt);
                                    send_message.emit(content);
                                })
                            }}
                            disabled={!matches!((*app_state).connection_state, ConnectionState::Connected)}
                            external_value={((*ui_state).input_prompt_value).clone()}
                        />
                    </div>

                    // Floating panel toggle button
                    <button class="panel-toggle" onclick={{
                        let ui_state = ui_state.clone();
                        Callback::from(move |_| {
                            ui_state.dispatch(if (*ui_state).panel_open {
                                UIStateAction::ClosePanel
                            } else {
                                UIStateAction::OpenPanel
                            })
                        })
                    }}>
                        <span>{"⚙️"}</span>
                        <span>{"Practice Settings"}</span>
                    </button>

                    // Learning panel toggle button
                    <button class="learning-panel-toggle" onclick={{
                        let ui_state = ui_state.clone();
                        Callback::from(move |_| {
                            ui_state.dispatch(if (*ui_state).learning_panel_open {
                                UIStateAction::CloseLearningPanel
                            } else {
                                UIStateAction::OpenLearningPanel
                            })
                        })
                    }}>
                        <span>{"📚"}</span>
                        <span>{"Learning Progress"}</span>
                    </button>

                    // Learning panel
                    <LearningPanel
                        items={(*ui_state).learning_items.clone()}
                        is_open={(*ui_state).learning_panel_open}
                    />
                </div>

                // Configuration panel (collapsible)
                <div class="panel" data-open={if (*ui_state).panel_open { "true" } else { "false" }}>
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
                                    <select id="language-select" onchange={on_language_change(app_state.clone())}>
                                        <option value="spanish" selected=true>{"Spanish"}</option>
                                        <option value="arabic">{"Arabic"}</option>
                                        <option value="french">{"French"}</option>
                                    </select>
                                </div>

                                <div class="panel-field">
                                    <label for="dialect-select">{"Dialect"}</label>
                                    <select id="dialect-select" onchange={on_dialect_change(app_state.clone())}>
                                        {for app_state.current_dialects().iter().map(|dialect| {
                                            let is_selected = *dialect == app_state.current_dialect();
                                            html! {
                                                <option value={dialect.id()} selected={is_selected}>
                                                    {dialect.name()}
                                                </option>
                                            }
                                        })}
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
                                    <select id="formality-select" onchange={on_formality_change(app_state.clone())}>
                                        <option value="formal">{"Formal"}</option>
                                        <option value="casual" selected=true>{"Casual"}</option>
                                        <option value="dialect_rich">{"Dialect-Rich"}</option>
                                        <option value="slang">{"Slang"}</option>
                                    </select>
                                </div>

                                <div class="panel-field">
                                    <label for="teaching-mode-select">{"Teaching Mode"}</label>
                                    <select id="teaching-mode-select" onchange={on_teaching_mode_change(app_state.clone())}>
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
                <div class="panel-backdrop" data-open={if (*ui_state).panel_open { "true" } else { "false" }} onclick={{
                    let ui_state = ui_state.clone();
                    Callback::from(move |_| {
                        ui_state.dispatch(UIStateAction::ClosePanel);
                    })
                }}></div>
            </main>
        </div>
    }
}
