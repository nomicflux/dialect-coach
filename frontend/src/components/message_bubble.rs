use super::ruby_text::render_text_with_ruby;
use dialect_coach_shared::models::dialect::dialect_features;
use dialect_coach_shared::models::{Language, LanguageOption, Message};
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
    pub language_option: Option<LanguageOption>,
    #[prop_or(false)]
    pub is_explain_loading: bool,
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

fn font_class(lang_option: &Option<LanguageOption>) -> &'static str {
    match lang_option {
        Some(LanguageOption::Arabic(script)) => {
            use dialect_coach_shared::models::ArabicScript;
            match script {
                ArabicScript::Naskh => "arabic-naskh",
                ArabicScript::Ruqa => "arabic-ruqa",
                ArabicScript::Latin => "arabic-latin",
                ArabicScript::FullyVoweled => "arabic-naskh",
            }
        }
        _ => "",
    }
}

fn render_action_buttons(
    on_explain: &Option<Callback<Uuid>>,
    msg_id: Uuid,
    is_explain_loading: bool,
) -> Html {
    if on_explain.is_none() {
        return html! {};
    }

    html! {
        <div class="message-actions">
            {render_explain_button(on_explain, msg_id, is_explain_loading)}
        </div>
    }
}

fn render_explain_button(
    on_explain: &Option<Callback<Uuid>>,
    msg_id: Uuid,
    is_loading: bool,
) -> Html {
    if let Some(callback) = on_explain {
        let cb = callback.clone();
        let onclick = Callback::from(move |_| cb.emit(msg_id));
        let button_text = if is_loading {
            "Explaining..."
        } else {
            "Explain"
        };
        html! {
            <button class="action-button explain-button" {onclick} disabled={is_loading}>{button_text}</button>
        }
    } else {
        html! {}
    }
}

fn render_bot_header(props: &MessageBubbleProps) -> Html {
    if props.is_own_message {
        return html! {};
    }
    html! { <div class="message-header">{render_replay_button(&props.on_replay, &props.message)}</div> }
}

fn render_bubble(props: &MessageBubbleProps, bubble_class: &str) -> Html {
    let lang = language_code(props.message.metadata.language);
    html! {
        <div class={bubble_class.to_string()}>
            {render_delete_button(&props.on_delete, props.message.id)}
            {render_bot_header(props)}
            <div class={classes!("message-content", font_class(&props.language_option))} {lang} data-message-id={props.message.id.to_string()}>
                {render_text_with_ruby(&props.message.get_content())}
            </div>
            {if props.message.is_agent() { render_action_buttons(&props.on_explain, props.message.id, props.is_explain_loading) } else { html! {} }}
            <div class="message-time">{props.message.metadata.timestamp.format("%H:%M").to_string()}</div>
        </div>
    }
}

#[function_component(MessageBubble)]
pub fn message_bubble(props: &MessageBubbleProps) -> Html {
    let (msg_class, avatar_class, bubble_class, avatar_text) =
        get_css_classes(props.is_own_message);
    let wrapper_class = if props.is_own_message {
        "message-wrapper message-wrapper--user"
    } else {
        "message-wrapper message-wrapper--agent"
    };
    html! {
        <div class={wrapper_class} style="position: relative; width: 100%;">
            <div class={msg_class}>
                <div class={avatar_class}>{avatar_text}</div>
                {render_bubble(props, bubble_class)}
            </div>
        </div>
    }
}
