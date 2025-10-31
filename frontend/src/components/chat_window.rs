use dialect_coach_shared::models::{Formality, Message};
use uuid::Uuid;
use web_sys::HtmlElement;
use yew::prelude::*;

use super::MessageBubble;

#[derive(Properties, PartialEq)]
pub struct ChatWindowProps {
    pub messages: Vec<Message>,
    pub is_loading: bool,
    #[prop_or_default]
    pub on_replay_message: Option<Callback<Message>>,
    #[prop_or_default]
    pub on_prompt_click: Option<Callback<(String, String)>>, // (button_id, phrase)
    #[prop_or_default]
    pub translating_button: Option<String>, // Track which specific button is translating
    pub formality: Formality,
    #[prop_or_default]
    pub on_delete_message: Option<Callback<Uuid>>,
}

fn get_context_aware_prompt(prompt_type: &str, formality: Formality) -> &'static str {
    match (prompt_type, formality) {
        ("greeting", Formality::Formal) => "Good day, how are you doing?",
        ("greeting", Formality::Casual) => "Hello, how are you?",
        ("greeting", Formality::DialectRich) => "Hey there, what's up?",
        ("greeting", Formality::Slang) => "Yo, what's good?",

        ("weather", Formality::Formal) => "What is the weather forecast for today?",
        ("weather", Formality::Casual) => "What's the weather like today?",
        ("weather", Formality::DialectRich) => "How's it looking outside?",
        ("weather", Formality::Slang) => "What's the weather doing?",

        ("food", Formality::Formal) => "I would like to place an order, please",
        ("food", Formality::Casual) => "I'd like to order some food",
        ("food", Formality::DialectRich) => "Can I get something to eat?",
        ("food", Formality::Slang) => "What's good to eat here?",

        _ => "Hello, how are you?", // fallback
    }
}

#[function_component(ChatWindow)]
pub fn chat_window(props: &ChatWindowProps) -> Html {
    let chat_container_ref = use_node_ref();

    // Auto-scroll to bottom when new messages arrive
    {
        let chat_container_ref = chat_container_ref.clone();
        let message_count = props.messages.len();

        use_effect_with(message_count, move |_| {
            if let Some(container) = chat_container_ref.cast::<HtmlElement>() {
                container.set_scroll_top(container.scroll_height());
            }
            || ()
        });
    }

    html! {
        <div class="chat" ref={chat_container_ref} role="log" aria-live="polite" aria-relevant="additions">
            <div class="chat-scroll">
                {if props.messages.is_empty() {
                    html! {
                        <div class="empty">
                            <div class="empty-icon">{"💬"}</div>
                            <h3 class="empty-title">{"Ready to practice!"}</h3>
                            <p class="empty-body">{"Start a conversation to practice your dialect. Try one of these:"}</p>
                            <div class="prompt-list">
                                <button class="prompt-item"
                                        disabled={props.translating_button.is_some()}
                                        onclick={{
                                    let on_prompt_click = props.on_prompt_click.clone();
                                    let formality = props.formality;
                                    Callback::from(move |_| {
                                        if let Some(callback) = &on_prompt_click {
                                            let prompt = get_context_aware_prompt("greeting", formality);
                                            callback.emit(("greeting".to_string(), prompt.to_string()));
                                        }
                                    })
                                }}>
                                    {if props.translating_button.as_ref() == Some(&"greeting".to_string()) {
                                        "Translating..."
                                    } else {
                                        match props.formality {
                                            Formality::Formal => "Good day, how are you doing?",
                                            Formality::Casual => "Hello, how are you?",
                                            Formality::DialectRich => "Hey there, what's up?",
                                            Formality::Slang => "Yo, what's good?",
                                        }
                                    }}
                                </button>
                                <button class="prompt-item"
                                        disabled={props.translating_button.is_some()}
                                        onclick={{
                                    let on_prompt_click = props.on_prompt_click.clone();
                                    let formality = props.formality;
                                    Callback::from(move |_| {
                                        if let Some(callback) = &on_prompt_click {
                                            let prompt = get_context_aware_prompt("weather", formality);
                                            callback.emit(("weather".to_string(), prompt.to_string()));
                                        }
                                    })
                                }}>
                                    {if props.translating_button.as_ref() == Some(&"weather".to_string()) {
                                        "Translating..."
                                    } else {
                                        match props.formality {
                                            Formality::Formal => "What is the weather forecast for today?",
                                            Formality::Casual => "What's the weather like today?",
                                            Formality::DialectRich => "How's it looking outside?",
                                            Formality::Slang => "What's the weather doing?",
                                        }
                                    }}
                                </button>
                                <button class="prompt-item"
                                        disabled={props.translating_button.is_some()}
                                        onclick={{
                                    let on_prompt_click = props.on_prompt_click.clone();
                                    let formality = props.formality;
                                    Callback::from(move |_| {
                                        if let Some(callback) = &on_prompt_click {
                                            let prompt = get_context_aware_prompt("food", formality);
                                            callback.emit(("food".to_string(), prompt.to_string()));
                                        }
                                    })
                                }}>
                                    {if props.translating_button.as_ref() == Some(&"food".to_string()) {
                                        "Translating..."
                                    } else {
                                        match props.formality {
                                            Formality::Formal => "I would like to place an order, please",
                                            Formality::Casual => "I'd like to order some food",
                                            Formality::DialectRich => "Can I get something to eat?",
                                            Formality::Slang => "What's good to eat here?",
                                        }
                                    }}
                                </button>
                            </div>
                        </div>
                    }
                } else {
                    html! {
                        <div class="messages-list">
                            {for props.messages.iter().map(|msg| {
                                let is_own = msg.participant_id == "user";
                                html! {
                                    <MessageBubble
                                        message={msg.clone()}
                                        is_own_message={is_own}
                                        on_replay={props.on_replay_message.clone()}
                                        on_delete={props.on_delete_message.clone()}
                                    />
                                }
                            })}
                        </div>
                    }
                }}

                {if props.is_loading {
                    html! {
                        <div class="loading-indicator">
                            <div class="typing">
                                <div class="typing-dots">
                                    <div class="typing-dot"></div>
                                    <div class="typing-dot"></div>
                                    <div class="typing-dot"></div>
                                </div>
                                <span class="loading-text">{"Agent is typing..."}</span>
                            </div>
                        </div>
                    }
                } else {
                    html! {}
                }}
            </div>
        </div>
    }
}
