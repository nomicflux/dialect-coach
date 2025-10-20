use dialect_coach_shared::models::Message;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct MessageBubbleProps {
    pub message: Message,
    pub is_own_message: bool,
    #[prop_or_default]
    pub on_replay: Option<Callback<Message>>,
}

#[function_component(MessageBubble)]
pub fn message_bubble(props: &MessageBubbleProps) -> Html {
    let msg = &props.message;

    let on_replay_click = {
        let on_replay = props.on_replay.clone();
        let msg = props.message.clone();
        Callback::from(move |_| {
            if let Some(callback) = &on_replay {
                callback.emit(msg.clone());
            }
        })
    };

    html! {
        <div class={if props.is_own_message { "message message--user" } else { "message message--bot" }}>
            <div class={if props.is_own_message { "avatar avatar--user" } else { "avatar avatar--bot" }}>
                {if props.is_own_message { "U" } else { "🤖" }}
            </div>
            <div class={if props.is_own_message { "bubble bubble--user" } else { "bubble bubble--bot" }}>
                <div class="message-header">
                    <div class="message-author">{&msg.participant_id}</div>
                    {if !props.is_own_message && props.on_replay.is_some() {
                        html! {
                            <button class="replay-button" onclick={on_replay_click} title="Replay audio">
                                {"🔊"}
                            </button>
                        }
                    } else {
                        html! {}
                    }}
                </div>
                <div class="message-content">{&msg.content}</div>
                <div class="message-time">{msg.timestamp.to_rfc3339()}</div>
            </div>
        </div>
    }
}
