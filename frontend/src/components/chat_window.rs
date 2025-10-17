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
        <div class="chat-window" ref={chat_container_ref}>
            {if props.messages.is_empty() {
                html! {
                    <div class="empty-state">
                        <p>{"No messages yet. Start a conversation!"}</p>
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
                        <span class="loading-dots">{"●●●"}</span>
                        <span class="loading-text">{"Agent is typing..."}</span>
                    </div>
                }
            } else {
                html! {}
            }}
        </div>
    }
}
