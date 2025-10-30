mod app_state;
use app_state::{AppState, UIState, AppStateAction, UIStateAction, UserStateAction, UserStateWrapper};
pub use dialect_coach_shared::{LearningItem, LearningItemType, UserState};

use dialect_coach_shared::models::{Formality, Language, Message, TeachingMode};
use dialect_coach_shared::{Explained, Exploratory, Mistake, Translated, UserMessageWithContext};
use log::{error, info};
use uuid::Uuid;
use yew::prelude::*;

use crate::components::{ChatWindow, InputBox, LearningPanel, SpeechControls};
use crate::hooks::use_debounced_save;
use crate::services::websocket::ConnectionState;

fn extract_learning_items(user_state: &UserStateWrapper) -> (Vec<Mistake>, Vec<Explained>, Vec<Translated>, Vec<Exploratory>) {
    let mut mistakes = Vec::new();
    let mut explained = Vec::new();
    let mut translated = Vec::new();
    let mut exploratory = Vec::new();

    for item in &user_state.learning_items {
        match &item.item {
            LearningItemType::Mistake(m) => mistakes.push(m.clone()),
            LearningItemType::Explanation(e) => explained.push(e.clone()),
            LearningItemType::Translation(t) => translated.push(t.clone()),
            LearningItemType::Exploration(e) => exploratory.push(e.clone()),
        }
    }

    (mistakes, explained, translated, exploratory)
}

fn on_send_message(
    app_state: UseReducerHandle<AppState>,
    user_state: UseReducerHandle<UserStateWrapper>
) -> Callback<String> {
    let app_state = app_state.clone();
    let user_state = user_state.clone();

    Callback::from(move |content: String| {
        info!("Sending message: {}", content);

        let msg = (*user_state).create_msg((*app_state).session_id(), &content);
        user_state.dispatch(UserStateAction::AddMessage(msg.clone()));

        // Extract learning items from user state
        let (past_mistakes, past_explained, past_translated, past_exploratory) = extract_learning_items(&user_state);

        // Build UserMessageWithContext
        let msg_with_context = UserMessageWithContext::new(msg, past_mistakes, past_explained, past_translated, past_exploratory);

        // Send through WebSocket
        match (*app_state).ws_service.borrow().send_message(&msg_with_context) {
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
    user_state: UseReducerHandle<UserStateWrapper>,
    ui_state: UseReducerHandle<UIState>,
) -> Callback<(String, String)> {
    let app_state = app_state.clone();
    let user_state = user_state.clone();
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
        let user_state = user_state.clone();
        let app_state = app_state.clone();

        // Start async translation
        wasm_bindgen_futures::spawn_local(async move {
            match app_state
                .translation_service
                .translate_phrase(
                    &english_phrase,
                    user_state.current_dialect(),
                    Some(user_state.formality),
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

fn on_language_change(user_state: UseReducerHandle<UserStateWrapper>) -> Callback<Event> {
    let user_state = user_state.clone();
    Callback::from(move |e: Event| {
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

fn on_dialect_change(user_state: UseReducerHandle<UserStateWrapper>) -> Callback<Event> {
    let user_state = user_state.clone();
    Callback::from(move |e: Event| {
        if let Some(select) = e.target_dyn_into::<web_sys::HtmlSelectElement>() {
            let value = select.value();

            // Get all dialects for current language and find matching one
            let dialects = user_state.current_dialects();
            if let Some(dialect) = dialects.iter().find(|d| d.id() == value) {
                user_state.dispatch(UserStateAction::ChangeDialect(dialect.clone()));
            }
        }
    })
}

fn on_formality_change(user_state: UseReducerHandle<UserStateWrapper>) -> Callback<Event> {
    let user_state = user_state.clone();

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
            user_state.dispatch(UserStateAction::ChangeFormality(f));
        }
    })
}

fn on_teaching_mode_change(user_state: UseReducerHandle<UserStateWrapper>) -> Callback<Event> {
    let user_state = user_state.clone();

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
            user_state.dispatch(UserStateAction::ChangeTeachingMode(tm));
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

fn on_tts_toggle(user_state: UseReducerHandle<UserStateWrapper>) -> Callback<()> {
    let user_state = user_state.clone();
    Callback::from(move |_| {
        user_state.dispatch(UserStateAction::ToggleTTS);
    })
}

fn on_dialect_cycle(user_state: UseReducerHandle<UserStateWrapper>) -> Callback<()> {
    let user_state = user_state.clone();
    Callback::from(move |_| {
        let dialects = user_state.current_dialects();
        let current = user_state.current_dialect();
        if let Some(idx) = dialects.iter().position(|d| d == &current) {
            let next_idx = (idx + 1) % dialects.len();
            user_state.dispatch(UserStateAction::ChangeDialect(dialects[next_idx].clone()));
        }
    })
}

fn on_formality_cycle(user_state: UseReducerHandle<UserStateWrapper>) -> Callback<()> {
    use dialect_coach_shared::models::Formality;
    let user_state = user_state.clone();
    Callback::from(move |_| {
        let formalities = [
            Formality::Formal,
            Formality::Casual,
            Formality::DialectRich,
            Formality::Slang,
        ];
        let current = user_state.formality;
        if let Some(idx) = formalities.iter().position(|f| f == &current) {
            let next_idx = (idx + 1) % formalities.len();
            user_state.dispatch(UserStateAction::ChangeFormality(formalities[next_idx]));
        }
    })
}

fn on_teaching_mode_cycle(user_state: UseReducerHandle<UserStateWrapper>) -> Callback<()> {
    use dialect_coach_shared::models::TeachingMode;
    let user_state = user_state.clone();
    Callback::from(move |_| {
        let modes = [
            TeachingMode::Immersive,
            TeachingMode::Corrective,
            TeachingMode::Explanatory,
            TeachingMode::Interleaved,
            TeachingMode::StoryTeller,
            TeachingMode::Debug,
        ];
        let current = user_state.teaching_mode;
        if let Some(idx) = modes.iter().position(|m| m == &current) {
            let next_idx = (idx + 1) % modes.len();
            user_state.dispatch(UserStateAction::ChangeTeachingMode(modes[next_idx]));
        }
    })
}

fn on_user_state_ws_open(
    app_state: UseReducerHandle<AppState>,
    user_id: uuid::Uuid,
) -> Callback<()> {
    Callback::from(move |_| {
        info!("User state WebSocket opened, loading state for user: {}", user_id);
        if let Err(e) = app_state.user_state_ws_service.borrow().load_user_state(user_id) {
            error!("Failed to request user state load: {}", e);
        }
        app_state.dispatch(AppStateAction::RetryPendingSaves);
    })
}

fn on_user_state_load_response(
    user_state: UseReducerHandle<UserStateWrapper>,
) -> Callback<Option<UserState>> {
    Callback::from(move |loaded_state: Option<UserState>| {
        if let Some(state) = loaded_state {
            info!("Received user state from backend");
            user_state.dispatch(UserStateAction::ReplaceUserState(state));
        } else {
            info!("No existing user state on backend, using current state");
        }
    })
}

fn on_user_state_save_response() -> Callback<Result<(), String>> {
    Callback::from(move |result: Result<(), String>| {
        match result {
            Ok(()) => info!("User state saved successfully to backend"),
            Err(e) => error!("Failed to save user state to backend: {}", e),
        }
    })
}

fn on_create_user_click(
    app_state: UseReducerHandle<AppState>,
    ui_state: UseReducerHandle<UIState>,
    user_state: UseReducerHandle<UserStateWrapper>,
) -> Callback<MouseEvent> {
    Callback::from(move |_: MouseEvent| {
        let username = (*ui_state).create_username_input.clone();
        let user_id = (*user_state).user_id;
        if let Err(e) = app_state.user_ws_service.borrow().create_user(user_id, username) {
            error!("Failed to create user: {}", e);
            app_state.dispatch(AppStateAction::SetError(format!("Failed to create user: {}", e)));
        }
    })
}

fn on_signin_click(
    app_state: UseReducerHandle<AppState>,
    ui_state: UseReducerHandle<UIState>,
) -> Callback<MouseEvent> {
    Callback::from(move |_: MouseEvent| {
        let username = (*ui_state).signin_username_input.clone();
        if let Err(e) = app_state.user_ws_service.borrow().sign_in(username) {
            error!("Failed to sign in: {}", e);
            app_state.dispatch(AppStateAction::SetError(format!("Failed to sign in: {}", e)));
        }
    })
}

fn on_user_create_response(
    app_state: UseReducerHandle<AppState>,
    ui_state: UseReducerHandle<UIState>,
) -> Callback<Result<dialect_coach_shared::User, String>> {
    Callback::from(move |result: Result<dialect_coach_shared::User, String>| {
        match result {
            Ok(user) => {
                info!("User created successfully: {}", user.username);
                ui_state.dispatch(UIStateAction::ClearCreateUsernameInput);
                app_state.dispatch(AppStateAction::SetUser(user));
            }
            Err(e) => {
                error!("Failed to create user: {}", e);
                app_state.dispatch(AppStateAction::SetError(format!("Create failed: {}", e)));
            }
        }
    })
}

fn on_user_signin_response(
    app_state: UseReducerHandle<AppState>,
    ui_state: UseReducerHandle<UIState>,
) -> Callback<Result<dialect_coach_shared::User, String>> {
    Callback::from(move |result: Result<dialect_coach_shared::User, String>| {
        match result {
            Ok(user) => {
                info!("Signed in successfully as: {}", user.username);
                ui_state.dispatch(UIStateAction::ClearSigninUsernameInput);
                app_state.dispatch(AppStateAction::SetUser(user));
            }
            Err(e) => {
                error!("Sign in failed: {}", e);
                app_state.dispatch(AppStateAction::SetError(format!("Sign in failed: {}", e)));
            }
        }
    })
}

#[function_component(App)]
pub fn app() -> Html {
    let app_state = use_reducer(AppState::default);
    let ui_state = use_reducer(UIState::default);

    // Create initial user state - will be loaded from backend via WebSocket
    let user_state = use_reducer(|| {
        info!("Creating initial UserState");
        UserStateWrapper(UserState::new(Uuid::new_v4()))
    });

    // Set up debounced auto-save via WebSocket with retry queue
    let app_state_for_save = app_state.clone();
    let _force_save = use_debounced_save(&user_state, move |state| {
        let ws_service = app_state_for_save.user_state_ws_service.borrow();
        match ws_service.save_user_state(state) {
            Ok(()) => {
                info!("UserState save request sent via WebSocket");
            }
            Err(e) => {
                error!("Failed to send UserState save: {}", e);
                app_state_for_save.dispatch(AppStateAction::QueuePendingSave(state.clone()));
            }
        }
    });

    {
        let app_state = app_state.clone();
        let user_state = user_state.clone();
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
                let usc = user_state.clone();
                ws.set_on_message(Callback::from(move |msg: Message| {
                    info!("Received message from: {}", msg.participant_id);
                    asc.dispatch(AppStateAction::LoadingComplete);

                    if msg.participant_id != "user" && (*usc).tts_enabled {
                        asc.dispatch(AppStateAction::Speak(msg.clone()));
                    }

                    let mistakes = msg.content.mistakes.clone().unwrap_or_default();
                    let explained = msg.content.explained.clone().unwrap_or_default();
                    let translated = msg.content.translated.clone().unwrap_or_default();
                    let exploratory = msg.content.exploratory.clone().unwrap_or_default();
                    if !mistakes.is_empty() || !explained.is_empty() || !translated.is_empty() || !exploratory.is_empty() {
                        usc.dispatch(UserStateAction::AddLearningItems(mistakes, explained, translated, exploratory));
                    }

                    if let Some(analysis) = msg.content.analysis.clone() {
                        info!("Received analysis with {} mistake scores, {} explained scores, {} translated scores, {} exploratory scores",
                            analysis.mistake_scores.len(),
                            analysis.explained_scores.len(),
                            analysis.translated_scores.len(),
                            analysis.exploratory_scores.len()
                        );
                        usc.dispatch(UserStateAction::UpdateScores(analysis));
                    }

                    usc.dispatch(UserStateAction::AddMessage(msg));
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

                    // Retry pending saves when connected
                    if matches!(new_state, ConnectionState::Connected) {
                        asc.dispatch(AppStateAction::RetryPendingSaves);
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

    // Initialize user state WebSocket for persistence
    {
        let app_state = app_state.clone();
        let user_state = user_state.clone();
        use_effect_with((), move |_| {
            info!("Initializing user state WebSocket connection");

            let user_id = (*user_state).user_id;
            let mut ws = (*app_state).user_state_ws_service.borrow_mut();

            ws.set_on_open(on_user_state_ws_open(app_state.clone(), user_id));
            ws.set_on_load_response(on_user_state_load_response(user_state.clone()));
            ws.set_on_save_response(on_user_state_save_response());
            ws.connect();

            move || {
                info!("User state WebSocket cleanup");
            }
        });
    }

    // Initialize user WebSocket for user management
    {
        let app_state = app_state.clone();
        let ui_state = ui_state.clone();
        use_effect_with((), move |_| {
            info!("Initializing user WebSocket connection");

            let mut ws = (*app_state).user_ws_service.borrow_mut();

            ws.set_on_create_response(on_user_create_response(app_state.clone(), ui_state.clone()));
            ws.set_on_signin_response(on_user_signin_response(app_state.clone(), ui_state.clone()));
            ws.set_on_open(Callback::from(|_| info!("User WebSocket opened")));
            ws.connect();

            move || {
                info!("User WebSocket cleanup");
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

                    // User management section
                    <div class="user-section">
                        {if let Some(user) = (*app_state).current_user.as_ref() {
                            html! {
                                <div class="user-signed-in">
                                    {format!("Signed in as: {}", user.username)}
                                </div>
                            }
                        } else {
                            html! {
                                <div class="user-forms">
                                    <div class="user-form">
                                        <label>{"Create: "}</label>
                                        <input
                                            type="text"
                                            value={(*ui_state).create_username_input.clone()}
                                            oninput={{
                                                let ui_state = ui_state.clone();
                                                Callback::from(move |e: InputEvent| {
                                                    if let Some(input) = e.target_dyn_into::<web_sys::HtmlInputElement>() {
                                                        ui_state.dispatch(UIStateAction::SetCreateUsernameInput(input.value()));
                                                    }
                                                })
                                            }}
                                        />
                                        <button onclick={on_create_user_click(app_state.clone(), ui_state.clone(), user_state.clone())}>
                                            {"Create Account"}
                                        </button>
                                    </div>
                                    <div class="user-form">
                                        <label>{"Sign In: "}</label>
                                        <input
                                            type="text"
                                            value={(*ui_state).signin_username_input.clone()}
                                            oninput={{
                                                let ui_state = ui_state.clone();
                                                Callback::from(move |e: InputEvent| {
                                                    if let Some(input) = e.target_dyn_into::<web_sys::HtmlInputElement>() {
                                                        ui_state.dispatch(UIStateAction::SetSigninUsernameInput(input.value()));
                                                    }
                                                })
                                            }}
                                        />
                                        <button onclick={on_signin_click(app_state.clone(), ui_state.clone())}>
                                            {"Sign In"}
                                        </button>
                                    </div>
                                </div>
                            }
                        }}
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
                            messages={(*user_state).conversation_history.clone()}
                            is_loading={(*app_state).is_loading}
                            on_replay_message={Some(on_replay_message(app_state.clone()))}
                            on_prompt_click={Some(on_prompt_click(app_state.clone(), user_state.clone(), ui_state.clone()))}
                            translating_button={((*ui_state).translating_button).clone()}
                            formality={(*user_state).formality}
                        />
                        <SpeechControls
                            on_speech={on_send_message(app_state.clone(), user_state.clone())}
                            language_code={user_state.bcp47_tag()}
                            teaching_mode={user_state.teaching_mode_display().to_string()}
                            formality={user_state.formality_display().to_string()}
                            tts_enabled={(*user_state).tts_enabled}
                            on_dialect_cycle={Some(on_dialect_cycle(user_state.clone()))}
                            on_teaching_mode_cycle={Some(on_teaching_mode_cycle(user_state.clone()))}
                            on_formality_cycle={Some(on_formality_cycle(user_state.clone()))}
                            on_tts_toggle={Some(on_tts_toggle(user_state.clone()))}
                        />
                        <InputBox
                            on_send={{
                                let ui_state = ui_state.clone();
                                let send_message = on_send_message(app_state.clone(), user_state.clone());
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
                        items={(*user_state).learning_items.clone()}
                        is_open={(*ui_state).learning_panel_open}
                        on_close={{
                            let ui_state = ui_state.clone();
                            Callback::from(move |_| {
                                ui_state.dispatch(UIStateAction::CloseLearningPanel);
                            })
                        }}
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
                                    <select id="language-select" onchange={on_language_change(user_state.clone())}>
                                        <option value="spanish" selected=true>{"Spanish"}</option>
                                        <option value="arabic">{"Arabic"}</option>
                                        <option value="french">{"French"}</option>
                                    </select>
                                </div>

                                <div class="panel-field">
                                    <label for="dialect-select">{"Dialect"}</label>
                                    <select id="dialect-select" onchange={on_dialect_change(user_state.clone())}>
                                        {for user_state.current_dialects().iter().map(|dialect| {
                                            let is_selected = *dialect == user_state.current_dialect();
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
