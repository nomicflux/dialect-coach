use dialect_coach_shared::models::{Dialect, Formality, Language, Message, TeachingMode};
use log::{error, info};
use std::cell::RefCell;
use std::rc::Rc;
use uuid::Uuid;
use yew::prelude::*;

use crate::components::{ChatWindow, InputBox, SpeechControls};
use crate::services::speech::{CloudTtsService, SpeechSynthesisService};
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

impl Reducible for MessagesState {
    type Action = MessagesAction;

    fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
        match action {
            MessagesAction::Add(msg) => {
                let mut messages = self.messages.clone();
                messages.push(msg);
                Rc::new(Self { messages })
            }
            MessagesAction::Clear => Rc::new(Self::default()),
        }
    }
}

enum MessagesAction {
    Add(Message),
    Clear,
}

#[function_component(App)]
pub fn app() -> Html {
    // Language and dialect selection
    let selected_language = use_state(|| Language::Spanish);
    let selected_dialect = use_state(|| Dialect::SpanishMexican);

    // Formality and teaching mode
    let formality = use_state(|| Formality::Casual);
    let teaching_mode = use_state(|| TeachingMode::Immersive);

    // Session state
    let session_id = use_state(|| Uuid::new_v4());
    let messages = use_reducer(MessagesState::default);
    let connection_state = use_state(|| ConnectionState::Disconnected);
    let is_loading = use_state(|| false);
    let error_message = use_state(|| Option::<String>::None);
    
    // UI state
    let panel_open = use_state(|| false);
    let input_prompt_value = use_state(|| Option::<String>::None);

    // WebSocket service (wrapped in Rc<RefCell<>> for interior mutability)
    let ws_service = use_state(|| {
        Rc::new(RefCell::new(WebSocketService::new(
            "ws://localhost:3000/ws",
        )))
    });

    // Cloud TTS service
    let neural_tts_service =
        use_state(|| Some(Rc::new(CloudTtsService::new("http://localhost:3000"))));

    // Keep browser TTS as fallback (for speech controls)
    let tts_service = use_state(|| match SpeechSynthesisService::new() {
        Ok(service) => Some(Rc::new(RefCell::new(service))),
        Err(e) => {
            error!("Failed to initialize browser TTS service: {}", e);
            None
        }
    });

    // Initialize WebSocket on mount
    {
        let ws_service = ws_service.clone();
        let connection_state = connection_state.clone();
        let messages = messages.clone();
        let error_message = error_message.clone();
        let is_loading = is_loading.clone();
        let tts_service = tts_service.clone();
        let neural_tts_service = neural_tts_service.clone();
        let selected_dialect = selected_dialect.clone();

        use_effect_with((), move |_| {
            info!("Initializing WebSocket connection");

            {
                let mut ws = ws_service.borrow_mut();

                // Set up callbacks
                ws.set_on_open(Callback::from(move |_| {
                    info!("WebSocket opened");
                }));

                let error_message_clone = error_message.clone();
                ws.set_on_close(Callback::from(move |_| {
                    info!("WebSocket closed");
                    error_message_clone.set(Some("Connection closed".to_string()));
                }));

                let error_message_clone = error_message.clone();
                ws.set_on_error(Callback::from(move |err| {
                    error!("WebSocket error: {}", err);
                    error_message_clone.set(Some(err));
                }));

                let messages_dispatcher = messages.dispatcher();
                let is_loading_clone = is_loading.clone();
                let neural_tts_clone = neural_tts_service.clone();
                let selected_dialect_clone = selected_dialect.clone();
                ws.set_on_message(Callback::from(move |msg: Message| {
                    info!("Received message from: {}", msg.participant_id);
                    is_loading_clone.set(false);

                    // Speak agent messages automatically with TTS
                    if msg.participant_id != "user" {
                        if let Some(neural_tts) = neural_tts_clone.as_ref() {
                            let text = msg.content.clone();
                            let language_code = (*selected_dialect_clone).bcp47_tag().to_string();
                            let tts = neural_tts.clone();

                            // Spawn async task to call TTS
                            wasm_bindgen_futures::spawn_local(async move {
                                if let Err(e) = tts.speak(&text, "", &language_code).await {
                                    error!("Failed to speak message with TTS: {}", e);
                                }
                            });
                        }
                    }

                    messages_dispatcher.dispatch(MessagesAction::Add(msg));
                }));

                // Set up state change callback
                let connection_state_clone = connection_state.clone();
                let ws_service_clone = ws_service.clone();
                ws.set_on_state_change(Callback::from(move |new_state| {
                    info!("Connection state changed to: {:?}", new_state);
                    connection_state_clone.set(new_state);

                    // Handle reconnecting state - the WebSocketService will attempt
                    // automatic reconnection, but we need to trigger it from the app layer
                    // since we can't easily call methods from within the async task
                    if matches!(new_state, ConnectionState::Reconnecting) {
                        // Schedule a reconnect attempt
                        let ws_clone = ws_service_clone.clone();
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
            let ws_service_clone = ws_service.clone();
            move || {
                info!("Disconnecting WebSocket");
                ws_service_clone.borrow_mut().disconnect();
            }
        });
    }

    // Handle sending messages
    let on_send_message = {
        let ws_service = ws_service.clone();
        let session_id = session_id.clone();
        let selected_dialect = selected_dialect.clone();
        let formality = formality.clone();
        let teaching_mode = teaching_mode.clone();
        let messages = messages.clone();
        let is_loading = is_loading.clone();
        let error_message = error_message.clone();

        Callback::from(move |content: String| {
            info!("Sending message: {}", content);

            // Create message with metadata
            let mut msg = Message::new(
                *session_id,
                "user".to_string(),
                content,
                (*selected_dialect).bcp47_tag().to_string(),
            );

            // Set formality and teaching mode in metadata
            msg.metadata.formality = Some(*formality);
            msg.metadata.teaching_mode = Some(*teaching_mode);

            // Add to local messages
            messages.dispatch(MessagesAction::Add(msg.clone()));

            // Send through WebSocket
            match ws_service.borrow().send_message(&msg) {
                Ok(_) => {
                    info!("Message sent successfully");
                    is_loading.set(true);
                    error_message.set(None);
                }
                Err(e) => {
                    error!("Failed to send message: {}", e);
                    error_message.set(Some(format!("Failed to send: {}", e)));
                }
            }
        })
    };

    // Handle language change
    let on_language_change = {
        let selected_language = selected_language.clone();
        let selected_dialect = selected_dialect.clone();

        Callback::from(move |e: Event| {
            if let Some(select) = e.target_dyn_into::<web_sys::HtmlSelectElement>() {
                let value = select.value();
                let lang = match value.as_str() {
                    "spanish" => Language::Spanish,
                    "arabic" => Language::Arabic,
                    "french" => Language::French,
                    _ => Language::Spanish,
                };
                selected_language.set(lang);

                // Update dialect to match language
                let default_dialect = match lang {
                    Language::Spanish => Dialect::SpanishMexican,
                    Language::Arabic => Dialect::ArabicEgyptian,
                    Language::French => Dialect::FrenchParisian,
                };
                selected_dialect.set(default_dialect);
            }
        })
    };

    // Handle dialect change
    let on_dialect_change = {
        let selected_dialect = selected_dialect.clone();
        let selected_language = selected_language.clone();

        Callback::from(move |e: Event| {
            if let Some(select) = e.target_dyn_into::<web_sys::HtmlSelectElement>() {
                let value = select.value();

                // Get all dialects for current language and find matching one
                let dialects = Dialect::for_language(*selected_language);
                if let Some(dialect) = dialects.iter().find(|d| d.id() == value) {
                    selected_dialect.set(*dialect);
                }
            }
        })
    };

    // Handle formality change
    let on_formality_change = {
        let formality = formality.clone();

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
                formality.set(f);
            }
        })
    };

    // Handle teaching mode change
    let on_teaching_mode_change = {
        let teaching_mode = teaching_mode.clone();

        Callback::from(move |e: Event| {
            if let Some(select) = e.target_dyn_into::<web_sys::HtmlSelectElement>() {
                let value = select.value();
                let tm = match value.as_str() {
                    "immersive" => TeachingMode::Immersive,
                    "corrective" => TeachingMode::Corrective,
                    "explanatory" => TeachingMode::Explanatory,
                    _ => TeachingMode::Immersive,
                };
                teaching_mode.set(tm);
            }
        })
    };

    // Handle TTS replay for messages
    let on_replay_message = {
        let neural_tts = neural_tts_service.clone();
        let selected_dialect = selected_dialect.clone();

        Callback::from(move |msg: Message| {
            info!("Replaying message with TTS: {}", msg.content);
            if let Some(tts) = neural_tts.as_ref() {
                let text = msg.content.clone();
                let language_code = (*selected_dialect).bcp47_tag().to_string();
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
    
    // Handle prompt button clicks - populate input field
    let on_prompt_click = {
        let input_prompt_value = input_prompt_value.clone();
        
        Callback::from(move |prompt_text: String| {
            info!("Prompt clicked: {}", prompt_text);
            input_prompt_value.set(Some(prompt_text));
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
                        {match *connection_state {
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
                        {if let Some(err) = (*error_message).as_ref() {
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
                            messages={messages.messages.clone()}
                            is_loading={*is_loading}
                            on_replay_message={Some(on_replay_message.clone())}
                            on_prompt_click={Some(on_prompt_click.clone())}
                        />
                        <SpeechControls
                            on_speech={on_send_message.clone()}
                            language_code={(*selected_dialect).bcp47_tag().to_string()}
                        />
                        <InputBox 
                            on_send={{
                                let input_prompt_value = input_prompt_value.clone();
                                let on_send_message = on_send_message.clone();
                                Callback::from(move |content: String| {
                                    // Clear the prompt value after use
                                    input_prompt_value.set(None);
                                    on_send_message.emit(content);
                                })
                            }}
                            disabled={!matches!(*connection_state, ConnectionState::Connected)}
                            external_value={(*input_prompt_value).clone()}
                        />
                    </div>
                    
                    // Floating panel toggle button
                    <button class="panel-toggle" onclick={{
                        let panel_open = panel_open.clone();
                        Callback::from(move |_| {
                            panel_open.set(!*panel_open);
                        })
                    }}>
                        <span>{"⚙️"}</span>
                        <span>{"Practice Settings"}</span>
                    </button>
                </div>
                
                // Configuration panel (collapsible)
                <div class="panel" data-open={if *panel_open { "true" } else { "false" }}>
                    <div class="panel-header">
                        <h3 class="panel-title">{"Practice Settings"}</h3>
                        <button class="panel-close" onclick={{
                            let panel_open = panel_open.clone();
                            Callback::from(move |_| {
                                panel_open.set(false);
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
                                        {for Dialect::for_language(*selected_language).iter().map(|dialect| {
                                            let is_selected = *dialect == *selected_dialect;
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
                                    </select>
                                    <div class="field-help">
                                        {"Immersive keeps conversations flowing naturally"}
                                    </div>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>

                // Panel backdrop
                <div class="panel-backdrop" data-open={if *panel_open { "true" } else { "false" }} onclick={{
                    let panel_open = panel_open.clone();
                    Callback::from(move |_| {
                        panel_open.set(false);
                    })
                }}></div>
            </main>
        </div>
    }
}
