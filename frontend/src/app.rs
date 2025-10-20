use dialect_coach_shared::models::{Dialect, Formality, Language, Message, TeachingMode};
use log::{error, info};
use std::cell::RefCell;
use std::rc::Rc;
use uuid::Uuid;
use yew::prelude::*;

use crate::components::{ChatWindow, InputBox, SpeechControls};
use crate::services::speech::CloudTtsService;
use crate::services::translation::TranslationService;
use crate::services::websocket::{ConnectionState, WebSocketService};

// Reducer for messages to handle state updates properly
#[derive(Clone, PartialEq)]
struct MessagesState {
    messages: Vec<Message>,
}

impl Default for MessagesState {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
        }
    }
}

impl MessagesState {
    fn push_message(&self, msg: Message) -> Self {
        let mut msgs = self.messages.clone();
        msgs.push(msg);
        Self {
            messages: msgs,
        }
    }
}

#[derive(Clone)]
struct LanguageChoices {
    selected_language: Language,
    selected_dialect: Dialect,
}

impl Default for LanguageChoices {
    fn default() -> Self {
        Self {
            selected_language: Language::Spanish,
            selected_dialect: Dialect::SpanishCuban,
        }
    }
}

impl LanguageChoices {
    fn with_language(&self, language: Language) -> Self {
        Self { selected_language: language, ..self.clone() }
    }

    fn with_dialect(&self, dialect: Dialect) -> Self {
        Self { selected_dialect: dialect, ..self.clone() }
    }
}

#[derive(Clone)]
struct LanguageManner {
    formality: Formality,
    teaching_mode: TeachingMode,
}

impl Default for LanguageManner {
    fn default() -> Self {
        Self {
            formality: Formality::Casual,
            teaching_mode: TeachingMode::Immersive,
        }
    }
}

impl LanguageManner {
    fn with_formality(&self, formality: Formality) -> Self {
        Self { formality, ..self.clone() }
    }

    fn with_teaching_mode(&self, teaching_mode: TeachingMode) -> Self {
        Self { teaching_mode, ..self.clone() }
    }
}

enum AppStateAction {
    AddMessage(Message),
    SetLoading,
    LoadingComplete,
    SetError(String),
    ClearError,
    SetConnectionState(ConnectionState),
}

#[derive(Clone)]
struct AppState {
    session_id: Uuid,
    messages: MessagesState,
    connection_state: ConnectionState,
    is_loading: bool,
    error_message: Option<String>,
    ws_service: Rc<RefCell<WebSocketService>>,
    tts_service: Option<Rc<CloudTtsService>>,
    translation_service: Rc<TranslationService>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            session_id: Uuid::new_v4(),
            messages: MessagesState::default(),
            connection_state: ConnectionState::Disconnected,
            is_loading: false,
            error_message: None,
            ws_service: Rc::new(RefCell::new(WebSocketService::new(
                "ws://localhost:3000/ws",
            ))),
            tts_service: Some(Rc::new(CloudTtsService::new("http://localhost:3000"))),
            translation_service: Rc::new(TranslationService::new("http://localhost:3000")),
        }
    }
}

impl AppState {
    pub fn apply_action(&self, action: AppStateAction) -> Self {
        let mut next = self.clone();
        match action {
            AppStateAction::AddMessage(msg) => next.messages = self.messages.push_message(msg),
            AppStateAction::SetLoading => next.is_loading = true,
            AppStateAction::LoadingComplete => next. is_loading = false,
            AppStateAction::SetError(msg) => next.error_message = Some(msg),
            AppStateAction::ClearError => next.error_message = None,
            AppStateAction::SetConnectionState(conn_state) => next.connection_state = conn_state,
        };
        next
    }
}

impl Reducible for AppState {
    type Action = AppStateAction;

    fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
        self.apply_action(action).into()
    }
}

enum UIStateAction {
    OpenPanel,
    ClosePanel,
    EnterInputPrompt(String),
    ClearInputPrompt,
    PushTranslatingButton(String),
    ClearTranslatingButton,
}

#[derive(Clone)]
struct UIState {
    panel_open: bool,
    input_prompt_value: Option<String>,
    translating_button: Option<String>,
}

impl Default for UIState {
    fn default() -> Self {
        Self {
            panel_open: false,
            input_prompt_value: None,
            translating_button: None,
        }
    }
}

impl UIState {
    fn apply_action(&self, action: UIStateAction) -> Self {
        let mut next = self.clone();
        match action {
            UIStateAction::OpenPanel => next.panel_open = true,
            UIStateAction::ClosePanel => next.panel_open = false,
            UIStateAction::EnterInputPrompt(input) => next.input_prompt_value = Some(input),
            UIStateAction::ClearInputPrompt => next.input_prompt_value = None,
            UIStateAction::PushTranslatingButton(msg) => next.translating_button = Some(msg),
            UIStateAction::ClearTranslatingButton => next.translating_button = None,
        };
        next
    }
}

impl Reducible for UIState {
    type Action = UIStateAction;

    fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
        self.clone().to_owned().apply_action(action).into()
    }
}

#[function_component(App)]
pub fn app() -> Html {
    let language_dialect = use_state(LanguageChoices::default);
    let language_manner = use_state(LanguageManner::default);
    let app_state = use_reducer(AppState::default);
    let ui_state = use_reducer(UIState::default);

    {
        let app_state = app_state.clone();
        let language_dialect = language_dialect.clone();

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
                ws.set_on_message(Callback::from(move |msg: Message| {
                    info!("Received message from: {}", msg.participant_id);
                    asc.dispatch(AppStateAction::LoadingComplete);

                    // Speak agent messages automatically with TTS
                    if msg.participant_id != "user" {
                        if let Some(neural_tts) = (*asc).tts_service.as_ref() {
                            info!(
                                "Synthesizing from dialect {}",
                                (*language_dialect).selected_dialect
                            );
                            let text = msg.content.clone();
                            let language_code = (*language_dialect).selected_dialect.bcp47_tag().to_string();
                            let tts = neural_tts.clone();

                            // Spawn async task to call TTS
                            wasm_bindgen_futures::spawn_local(async move {
                                if let Err(e) = tts.speak(&text, "", &language_code).await {
                                    error!("Failed to speak message with TTS: {}", e);
                                }
                            });
                        }
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

    // Handle sending messages
    let on_send_message = {
        let app_state = app_state.clone();
        let language_dialect = language_dialect.clone();
        let language_manner = language_manner.clone();

        Callback::from(move |content: String| {
            info!("Sending message: {}", content);

            // Create message with metadata
            let mut msg = Message::new(
                (*app_state).session_id,
                "user".to_string(),
                content,
                (*language_dialect).selected_dialect.bcp47_tag().to_string(),
            );

            // Set formality and teaching mode in metadata
            msg.metadata.formality = Some((*language_manner).formality);
            msg.metadata.teaching_mode = Some((*language_manner).teaching_mode);

            // Add to local messages
            app_state
                .dispatch(AppStateAction::AddMessage(msg.clone()));

            // Send through WebSocket
            match app_state.ws_service.borrow().send_message(&msg) {
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
    };

    // Handle language change
    let on_language_change = {
        let language_dialect = language_dialect.clone();

        Callback::from(move |e: Event| {
            if let Some(select) = e.target_dyn_into::<web_sys::HtmlSelectElement>() {
                let value = select.value();
                let lang = match value.as_str() {
                    "spanish" => Language::Spanish,
                    "arabic" => Language::Arabic,
                    "french" => Language::French,
                    _ => Language::Spanish,
                };
                language_dialect.set(language_dialect.with_language(lang));

                // Update dialect to match language
                let default_dialect = match lang {
                    Language::Spanish => Dialect::SpanishCuban,
                    Language::Arabic => Dialect::ArabicEgyptian,
                    Language::French => Dialect::FrenchParisian,
                };
                language_dialect.set(language_dialect.with_dialect(default_dialect));
            }
        })
    };

    // Handle dialect change
    let on_dialect_change = {
        let language_dialect = language_dialect.clone();

        Callback::from(move |e: Event| {
            if let Some(select) = e.target_dyn_into::<web_sys::HtmlSelectElement>() {
                let value = select.value();

                // Get all dialects for current language and find matching one
                let dialects = Dialect::for_language((*language_dialect).selected_language);
                if let Some(dialect) = dialects.iter().find(|d| d.id() == value) {
                    language_dialect.set(language_dialect.with_dialect(dialect.clone()));
                }
            }
        })
    };

    // Handle formality change
    let on_formality_change = {
        let language_manner = language_manner.clone();

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
                language_manner.set(language_manner.with_formality(f));
            }
        })
    };

    // Handle teaching mode change
    let on_teaching_mode_change = {
        let language_manner = language_manner.clone();

        Callback::from(move |e: Event| {
            if let Some(select) = e.target_dyn_into::<web_sys::HtmlSelectElement>() {
                let value = select.value();
                let tm = match value.as_str() {
                    "immersive" => TeachingMode::Immersive,
                    "corrective" => TeachingMode::Corrective,
                    "explanatory" => TeachingMode::Explanatory,
                    "interleaved" => TeachingMode::Interleaved,
                    "debug" => TeachingMode::Debug,
                    _ => TeachingMode::Immersive,
                };
                language_manner.set(language_manner.with_teaching_mode(tm));
            }
        })
    };

    // Handle TTS replay for messages
    let on_replay_message = {
        let app_state = app_state.clone();
        let language_dialect = language_dialect.clone();

        Callback::from(move |msg: Message| {
            info!("Replaying message with TTS: {}", msg.content);
            if let Some(tts) = (*app_state).tts_service.as_ref() {
                let text = msg.content.clone();
                let language_code = ((*language_dialect).selected_dialect)
                    .bcp47_tag()
                    .to_string();
                let tts_clone = tts.clone();

                // Spawn async task to call TTS
                wasm_bindgen_futures::spawn_local(async move {
                    if let Err(e) = tts_clone.speak(&text, "", &language_code).await {
                        error!("Failed to replay message with TTS: {}", e);
                    }
                });
            }
        })
    };

    // Handle dialect cycling - quick switch between dialects within current language
    let on_dialect_cycle = {
        let language_dialect = language_dialect.clone();

        Callback::from(move |_| {
            let current_dialects = Dialect::for_language((*language_dialect).selected_language);
            let current_index = current_dialects
                .iter()
                .position(|d| d == &(*language_dialect).selected_dialect)
                .unwrap_or(0);
            let next_index = (current_index + 1) % current_dialects.len();
            let next_dialect = current_dialects[next_index];

            info!(
                "Cycling dialect: {} -> {}",
                language_dialect.selected_dialect.name(),
                next_dialect.name()
            );
            language_dialect.set(language_dialect.with_dialect(next_dialect));
        })
    };

    // Handle prompt button clicks - AI translate then populate input field
    let on_prompt_click = {
        let language_dialect = language_dialect.clone();
        let language_manner = language_manner.clone();
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
            let language_dialect = language_dialect.clone();
            let language_manner = language_manner.clone();

            // Start async translation
            wasm_bindgen_futures::spawn_local(async move {
                match app_state
                    .translation_service
                    .translate_phrase(
                        &english_phrase,
                        (*language_dialect).selected_dialect,
                        Some((*language_manner).formality),
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
    };

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
                            messages={(*app_state).messages.messages.clone()}
                            is_loading={(*app_state).is_loading}
                            on_replay_message={Some(on_replay_message.clone())}
                            on_prompt_click={Some(on_prompt_click.clone())}
                            translating_button={((*ui_state).translating_button).clone()}
                            formality={(*language_manner).formality}
                        />
                        <SpeechControls
                            on_speech={on_send_message.clone()}
                            language_code={(*language_dialect).selected_dialect.bcp47_tag().to_string()}
                            on_dialect_cycle={Some(on_dialect_cycle.clone())}
                        />
                        <InputBox
                            on_send={{
                                let ui_state = ui_state.clone();
                                let on_send_message = on_send_message.clone();
                                Callback::from(move |content: String| {
                                    // Clear the prompt value after use
                                    ui_state.dispatch(UIStateAction::ClearInputPrompt);
                                    on_send_message.emit(content);
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
                                    <select id="language-select" onchange={on_language_change}>
                                        <option value="spanish" selected=true>{"Spanish"}</option>
                                        <option value="arabic">{"Arabic"}</option>
                                        <option value="french">{"French"}</option>
                                    </select>
                                </div>

                                <div class="panel-field">
                                    <label for="dialect-select">{"Dialect"}</label>
                                    <select id="dialect-select" onchange={on_dialect_change}>
                                        {for Dialect::for_language((*language_dialect).selected_language).iter().map(|dialect| {
                                            let is_selected = *dialect == (*language_dialect).selected_dialect;
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
                                    <select id="formality-select" onchange={on_formality_change}>
                                        <option value="formal">{"Formal"}</option>
                                        <option value="casual" selected=true>{"Casual"}</option>
                                        <option value="dialect_rich">{"Dialect-Rich"}</option>
                                        <option value="slang">{"Slang"}</option>
                                    </select>
                                </div>

                                <div class="panel-field">
                                    <label for="teaching-mode-select">{"Teaching Mode"}</label>
                                    <select id="teaching-mode-select" onchange={on_teaching_mode_change}>
                                        <option value="immersive" selected=true>{"Immersive"}</option>
                                        <option value="corrective">{"Corrective"}</option>
                                        <option value="explanatory">{"Explanatory"}</option>
                                        <option value="interleaved">{"Interleaved"}</option>
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
