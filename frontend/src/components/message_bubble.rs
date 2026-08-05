use super::ruby_text::render_text_with_ruby;
use crate::components::{TranslateSelectionButton, translate_selection_button::SelectionAction};

use dialect_coach_shared::models::dialect::dialect_features;
use dialect_coach_shared::models::{Language, LanguageOption, Message};
use uuid::Uuid;
use wasm_bindgen::prelude::*;
use web_sys::{MouseEvent, window};
use yew::prelude::*;

#[derive(Clone, PartialEq)]
struct TextSelection {
    text: String,
    position: (f64, f64),
}

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
    #[prop_or(false)]
    pub is_translate_loading: bool,
    #[prop_or_default]
    pub on_selection_translate: Option<Callback<(Uuid, SelectionAction, String, String)>>,
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

fn get_text_selection() -> Option<TextSelection> {
    let window = window()?;
    let document = window.document()?;
    let selection = document.get_selection().ok()??;

    if selection.is_collapsed() {
        return None;
    }

    let text = selection.to_string().as_string()?;
    if text.trim().is_empty() {
        return None;
    }

    let range = selection.get_range_at(0).ok()?;
    let rect = range.get_bounding_client_rect();

    Some(TextSelection {
        text,
        position: (rect.right(), rect.bottom()),
    })
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

#[function_component(MessageBubble)]
pub fn message_bubble(props: &MessageBubbleProps) -> Html {
    let selection_state = use_state(|| None::<TextSelection>);

    let on_mouseup = {
        let selection_state = selection_state.clone();
        Callback::from(move |_: MouseEvent| {
            if let Some(selection) = get_text_selection() {
                selection_state.set(Some(selection));
            }
        })
    };

    // Clear selection when selection changes and becomes empty
    use_effect_with((), {
        let selection_state = selection_state.clone();
        move |_| {
            let document = web_sys::window()
                .and_then(|w| w.document())
                .expect("should have document");

            let callback = {
                let selection_state = selection_state.clone();
                Closure::wrap(Box::new(move |_: web_sys::Event| {
                    if let Some(window) = web_sys::window()
                        && let Ok(Some(selection)) = window.get_selection()
                        && selection.is_collapsed()
                    {
                        selection_state.set(None);
                    }
                }) as Box<dyn Fn(_)>)
            };

            let _ = document.add_event_listener_with_callback(
                "selectionchange",
                callback.as_ref().unchecked_ref(),
            );

            // Keep callback alive and remove on cleanup
            move || {
                let _ = document.remove_event_listener_with_callback(
                    "selectionchange",
                    callback.as_ref().unchecked_ref(),
                );
            }
        }
    });

    let on_selection_action = {
        let message_id = props.message.id;
        let callback = props.on_selection_translate.clone();
        let selection_state = selection_state.clone();

        Callback::from(
            move |(action, selected_text, context): (SelectionAction, String, String)| {
                if let Some(cb) = &callback {
                    cb.emit((message_id, action, selected_text, context));
                }
                selection_state.set(None);
            },
        )
    };

    let (msg_class, avatar_class, bubble_class, avatar_text) =
        get_css_classes(props.is_own_message);
    let lang = language_code(props.message.metadata.language);
    let font_class_name = font_class(&props.language_option);
    let wrapper_class = if props.is_own_message {
        "message-wrapper message-wrapper--user"
    } else {
        "message-wrapper message-wrapper--agent"
    };

    html! {
        <div class={wrapper_class} style="position: relative; width: 100%;">
            <div class={msg_class}>
                <div class={avatar_class}>{avatar_text}</div>
                <div class={bubble_class}>
                    {render_delete_button(&props.on_delete, props.message.id)}
                    // Avatar and alignment already say who is speaking, so the
                    // header carries only the replay control.
                    {if !props.is_own_message {
                        html! {
                            <div class="message-header">
                                {render_replay_button(&props.on_replay, &props.message)}
                            </div>
                        }
                    } else {
                        html! {}
                    }}
                    <div class={classes!("message-content", font_class_name)} {lang} onmouseup={on_mouseup}>
                        {render_text_with_ruby(&props.message.get_content())}
                    </div>
                    {if props.message.is_agent() {
                        render_action_buttons(&props.on_explain, props.message.id, props.is_explain_loading)
                    } else {
                        html! {}
                    }}
                    <div class="message-time">{props.message.metadata.timestamp.format("%H:%M").to_string()}</div>
                </div>
            </div>

            {
                if let Some(selection) = &*selection_state {
                    html! {
                        <TranslateSelectionButton
                            selected_text={selection.text.clone()}
                            message_context={props.message.get_content()}
                            position={selection.position}
                            on_action={on_selection_action}
                            is_loading={props.is_translate_loading}
                        />
                    }
                } else {
                    html! {}
                }
            }
        </div>
    }
}
