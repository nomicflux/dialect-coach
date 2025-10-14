use yew::prelude::*;
use dialect_coach_shared::models::{Language, Dialect, Message, Formality, TeachingMode};
use uuid::Uuid;
use std::rc::Rc;
use std::cell::RefCell;
use log::{info, error};

use crate::services::websocket::{WebSocketService, ConnectionState};
use crate::services::speech::SpeechSynthesisService;
use crate::components::{ChatWindow, InputBox, SpeechControls};

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

    // WebSocket service (wrapped in Rc<RefCell<>> for interior mutability)
    let ws_service = use_state(|| Rc::new(RefCell::new(WebSocketService::new("ws://localhost:3000/ws"))));

    // Speech synthesis service for TTS
    let tts_service = use_state(|| {
        match SpeechSynthesisService::new() {
            Ok(service) => Some(Rc::new(RefCell::new(service))),
            Err(e) => {
                error!("Failed to initialize TTS service: {}", e);
                None
            }
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
                let tts_service_clone = tts_service.clone();
                ws.set_on_message(Callback::from(move |msg: Message| {
                    info!("Received message from: {}", msg.participant_id);
                    is_loading_clone.set(false);

                    // Speak agent messages automatically
                    if msg.participant_id != "user" {
                        if let Some(tts) = tts_service_clone.as_ref() {
                            let language_code = msg.language.clone();
                            let text = msg.content.clone();
                            if let Err(e) = tts.borrow().speak(&text, &language_code) {
                                error!("Failed to speak message: {}", e);
                            }
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
                        }).forget();
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
        let tts_service = tts_service.clone();

        Callback::from(move |msg: Message| {
            info!("Replaying message: {}", msg.content);
            if let Some(tts) = tts_service.as_ref() {
                let language_code = msg.language.clone();
                let text = msg.content.clone();
                if let Err(e) = tts.borrow().speak(&text, &language_code) {
                    error!("Failed to replay message: {}", e);
                }
            }
        })
    };

    html! {
        <div class="app-container">
            <header class="app-header">
                <h1>{"Dialect Coach"}</h1>
                <p>{"Practice Spanish, Arabic, and French dialects with AI agents"}</p>

                // Connection status
                <div class="connection-status">
                    {match *connection_state {
                        ConnectionState::Connected => html! { <span class="status-connected">{"● Connected"}</span> },
                        ConnectionState::Connecting => html! { <span class="status-connecting">{"⟳ Connecting..."}</span> },
                        ConnectionState::Reconnecting => html! { <span class="status-reconnecting">{"⟳ Reconnecting..."}</span> },
                        ConnectionState::Disconnected => html! { <span class="status-disconnected">{"○ Disconnected"}</span> },
                        ConnectionState::Failed => html! { <span class="status-failed">{"✖ Connection Failed"}</span> },
                    }}
                </div>
            </header>

            <main class="app-main">
                <div class="configuration-panel">
                    <div class="config-row">
                        <div class="language-selection">
                            <label>{"Language: "}</label>
                            <select onchange={on_language_change}>
                                <option value="spanish" selected=true>{"Spanish"}</option>
                                <option value="arabic">{"Arabic"}</option>
                                <option value="french">{"French"}</option>
                            </select>
                        </div>

                        <div class="dialect-selection">
                            <label>{"Dialect: "}</label>
                            <select onchange={on_dialect_change}>
                                {for Dialect::for_language(*selected_language).iter().map(|dialect| {
                                    let is_selected = *dialect == *selected_dialect;
                                    html! {
                                        <option value={dialect.id()} selected={is_selected}>
                                            {dialect.name()}
                                        </option>
                                    }
                                })}
                            </select>
                        </div>
                    </div>

                    <div class="config-row">
                        <div class="formality-selection">
                            <label>{"Formality: "}</label>
                            <select onchange={on_formality_change}>
                                <option value="formal">{"Formal"}</option>
                                <option value="casual" selected=true>{"Casual"}</option>
                                <option value="slang">{"Slang"}</option>
                            </select>
                        </div>

                        <div class="teaching-mode-selection">
                            <label>{"Teaching Mode: "}</label>
                            <select onchange={on_teaching_mode_change}>
                                <option value="immersive" selected=true>{"Immersive"}</option>
                                <option value="corrective">{"Corrective"}</option>
                                <option value="explanatory">{"Explanatory"}</option>
                            </select>
                        </div>
                    </div>
                </div>

                // Error display
                {if let Some(err) = (*error_message).as_ref() {
                    html! {
                        <div class="error-banner">
                            {format!("Error: {}", err)}
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
                />
                <SpeechControls
                    on_speech={on_send_message.clone()}
                    language_code={(*selected_dialect).bcp47_tag().to_string()}
                />
                <InputBox on_send={on_send_message} disabled={!matches!(*connection_state, ConnectionState::Connected)} />
            </main>
        </div>
    }
}
