use yew::prelude::*;
use dialect_coach_shared::models::{Language, Dialect, Message, Formality, TeachingMode};
use uuid::Uuid;
use std::rc::Rc;
use std::cell::RefCell;
use log::{info, error};

use crate::services::websocket::WebSocketService;
use crate::components::{ChatWindow, InputBox};

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
    let is_connected = use_state(|| false);
    let is_loading = use_state(|| false);
    let error_message = use_state(|| Option::<String>::None);

    // WebSocket service (wrapped in Rc<RefCell<>> for interior mutability)
    let ws_service = use_state(|| Rc::new(RefCell::new(WebSocketService::new("ws://localhost:3000/ws"))));

    // Initialize WebSocket on mount
    {
        let ws_service = ws_service.clone();
        let is_connected = is_connected.clone();
        let messages = messages.clone();
        let error_message = error_message.clone();
        let is_loading = is_loading.clone();

        use_effect_with((), move |_| {
            info!("Initializing WebSocket connection");

            {
                let mut ws = ws_service.borrow_mut();

                // Set up callbacks
                let is_connected_clone = is_connected.clone();
                ws.set_on_open(Callback::from(move |_| {
                    info!("WebSocket opened");
                    is_connected_clone.set(true);
                }));

                let is_connected_clone = is_connected.clone();
                let error_message_clone = error_message.clone();
                ws.set_on_close(Callback::from(move |_| {
                    info!("WebSocket closed");
                    is_connected_clone.set(false);
                    error_message_clone.set(Some("Connection closed".to_string()));
                }));

                let error_message_clone = error_message.clone();
                ws.set_on_error(Callback::from(move |err| {
                    error!("WebSocket error: {}", err);
                    error_message_clone.set(Some(err));
                }));

                let messages_dispatcher = messages.dispatcher();
                let is_loading_clone = is_loading.clone();
                ws.set_on_message(Callback::from(move |msg: Message| {
                    info!("Received message from: {}", msg.participant_id);
                    is_loading_clone.set(false);
                    messages_dispatcher.dispatch(MessagesAction::Add(msg));
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

    html! {
        <div class="app-container">
            <header class="app-header">
                <h1>{"Dialect Coach"}</h1>
                <p>{"Practice Spanish, Arabic, and French dialects with AI agents"}</p>

                // Connection status
                <div class="connection-status">
                    {if *is_connected {
                        html! { <span class="status-connected">{"● Connected"}</span> }
                    } else {
                        html! { <span class="status-disconnected">{"○ Disconnected"}</span> }
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
                <ChatWindow messages={messages.messages.clone()} is_loading={*is_loading} />
                <InputBox on_send={on_send_message} disabled={!*is_connected} />
            </main>
        </div>
    }
}
