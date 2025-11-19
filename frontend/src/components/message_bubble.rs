use dialect_coach_shared::models::dialect::dialect_features;
use dialect_coach_shared::models::{Language, Message};
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
    #[prop_or_default]
    pub on_create_branch: Option<Callback<Uuid>>,
    #[prop_or(false)]
    pub has_child_branches: bool,
    #[prop_or_default]
    pub on_explain: Option<Callback<Uuid>>,
    #[prop_or_default]
    pub on_translate: Option<Callback<Uuid>>,
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
        let dialect = msg.metadata.dialect;
        if !dialect_features(dialect).has_tts() {
            return html! {};
        }
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
        (
            "message message--user",
            "avatar avatar--user",
            "bubble bubble--user",
            "U",
        )
    } else {
        (
            "message message--bot",
            "avatar avatar--bot",
            "bubble bubble--bot",
            "🤖",
        )
    }
}

fn language_code(lang: Language) -> &'static str {
    match lang {
        Language::Spanish => "es",
        Language::Arabic => "ar",
        Language::French => "fr",
        Language::English => "en",
        Language::Japanese => "ja",
    }
}

fn render_branch_button(
    on_create_branch: &Option<Callback<Uuid>>,
    msg_id: Uuid,
    has_children: bool,
) -> Html {
    if let Some(callback) = on_create_branch {
        let cb = callback.clone();
        let onclick = Callback::from(move |_| cb.emit(msg_id));
        let title = if has_children {
            "Branch from here (has existing branches)"
        } else {
            "Branch from here"
        };
        let class = if has_children {
            "branch-button branch-button--has-children"
        } else {
            "branch-button"
        };
        html! {
            <button {class} {onclick} {title}>{"🌿"}</button>
        }
    } else {
        html! {}
    }
}

fn render_action_buttons(
    on_explain: &Option<Callback<Uuid>>,
    on_translate: &Option<Callback<Uuid>>,
    msg_id: Uuid,
) -> Html {
    if on_explain.is_none() && on_translate.is_none() {
        return html! {};
    }

    html! {
        <div class="message-actions">
            {render_explain_button(on_explain, msg_id)}
            {render_translate_button(on_translate, msg_id)}
        </div>
    }
}

fn render_explain_button(on_explain: &Option<Callback<Uuid>>, msg_id: Uuid) -> Html {
    if let Some(callback) = on_explain {
        let cb = callback.clone();
        let onclick = Callback::from(move |_| cb.emit(msg_id));
        html! {
            <button class="action-button explain-button" {onclick}>{"Explain"}</button>
        }
    } else {
        html! {}
    }
}

fn render_translate_button(on_translate: &Option<Callback<Uuid>>, msg_id: Uuid) -> Html {
    if let Some(callback) = on_translate {
        let cb = callback.clone();
        let onclick = Callback::from(move |_| cb.emit(msg_id));
        html! {
            <button class="action-button translate-button" {onclick}>{"Translate"}</button>
        }
    } else {
        html! {}
    }
}

#[function_component(MessageBubble)]
pub fn message_bubble(props: &MessageBubbleProps) -> Html {
    let (msg_class, avatar_class, bubble_class, avatar_text) =
        get_css_classes(props.is_own_message);
    let lang = language_code(props.message.metadata.language);

    html! {
        <>
            <div class={msg_class}>
                <div class={avatar_class}>{avatar_text}</div>
                <div class={bubble_class}>
                    {render_delete_button(&props.on_delete, props.message.id)}
                    <div class="message-header">
                        <div class="message-author">{if props.message.is_agent() { "agent" } else { "user" }}</div>
                        {if !props.is_own_message { render_replay_button(&props.on_replay, &props.message) } else { html! {} }}
                    </div>
                    <div class="message-content" {lang}>{&props.message.get_content()}</div>
                    {if props.message.is_agent() {
                        render_action_buttons(&props.on_explain, &props.on_translate, props.message.id)
                    } else {
                        html! {}
                    }}
                    <div class="message-time">{props.message.metadata.timestamp.to_rfc3339()}</div>
                </div>
            </div>
            {render_branch_button(&props.on_create_branch, props.message.id, props.has_child_branches)}
        </>
    }
}
