use dialect_coach_shared::models::Message;
use uuid::Uuid;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct MessageBubbleProps {
    pub message: Message,
    pub is_own_message: bool,
    #[prop_or_default]
    pub on_replay: Option<Callback<Message>>,
    #[prop_or_default]
    pub on_delete: Option<Callback<Uuid>>,
}

fn render_delete_button(on_delete: &Option<Callback<Uuid>>, msg_id: Uuid) -> Html {
    if let Some(callback) = on_delete {
        let cb = callback.clone();
        let onclick = Callback::from(move |_| cb.emit(msg_id));
        html! {
            <button class="message-delete-button" {onclick} title="Delete message">{"×"}</button>
        }
    } else {
        html! {}
    }
}

fn render_replay_button(on_replay: &Option<Callback<Message>>, msg: &Message) -> Html {
    if let Some(callback) = on_replay {
        let cb = callback.clone();
        let m = msg.clone();
        let onclick = Callback::from(move |_| cb.emit(m.clone()));
        html! {
            <button class="replay-button" {onclick} title="Replay audio">{"🔊"}</button>
        }
    } else {
        html! {}
    }
}

fn get_css_classes(is_own: bool) -> (&'static str, &'static str, &'static str, &'static str) {
    if is_own {
        ("message message--user", "avatar avatar--user", "bubble bubble--user", "U")
    } else {
        ("message message--bot", "avatar avatar--bot", "bubble bubble--bot", "🤖")
    }
}

#[function_component(MessageBubble)]
pub fn message_bubble(props: &MessageBubbleProps) -> Html {
    let (msg_class, avatar_class, bubble_class, avatar_text) = get_css_classes(props.is_own_message);

    html! {
        <div class={msg_class}>
            <div class={avatar_class}>{avatar_text}</div>
            <div class={bubble_class}>
                {render_delete_button(&props.on_delete, props.message.id)}
                <div class="message-header">
                    <div class="message-author">{&props.message.participant_id}</div>
                    {if !props.is_own_message { render_replay_button(&props.on_replay, &props.message) } else { html! {} }}
                </div>
                <div class="message-content">{&props.message.content.response}</div>
                <div class="message-time">{props.message.timestamp.to_rfc3339()}</div>
            </div>
        </div>
    }
}
