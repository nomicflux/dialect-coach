use yew::prelude::*;
use dialect_coach_shared::models::Message;

#[derive(Properties, PartialEq)]
pub struct MessageBubbleProps {
    pub message: Message,
    pub is_own_message: bool,
}

#[function_component(MessageBubble)]
pub fn message_bubble(props: &MessageBubbleProps) -> Html {
    let msg = &props.message;

    let bubble_class = if props.is_own_message {
        "message-bubble message-own"
    } else {
        "message-bubble message-other"
    };

    html! {
        <div class={bubble_class}>
            <div class="message-author">{&msg.participant_id}</div>
            <div class="message-content">{&msg.content}</div>
            <div class="message-time">{msg.timestamp.to_rfc3339()}</div>
        </div>
    }
}
