use dialect_coach_shared::models::Message;
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
    pub on_prompt_click: Option<Callback<String>>,
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
                                <button class="prompt-item" onclick={{
                                    let on_prompt_click = props.on_prompt_click.clone();
                                    Callback::from(move |_| {
                                        if let Some(callback) = &on_prompt_click {
                                            callback.emit("Greet me in Mexican Spanish".to_string());
                                        }
                                    })
                                }}>{"Greet me in Mexican Spanish"}</button>
                                <button class="prompt-item" onclick={{
                                    let on_prompt_click = props.on_prompt_click.clone();
                                    Callback::from(move |_| {
                                        if let Some(callback) = &on_prompt_click {
                                            callback.emit("Ask about the weather".to_string());
                                        }
                                    })
                                }}>{"Ask about the weather"}</button>
                                <button class="prompt-item" onclick={{
                                    let on_prompt_click = props.on_prompt_click.clone();
                                    Callback::from(move |_| {
                                        if let Some(callback) = &on_prompt_click {
                                            callback.emit("Order food at a restaurant".to_string());
                                        }
                                    })
                                }}>{"Order food at a restaurant"}</button>
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
