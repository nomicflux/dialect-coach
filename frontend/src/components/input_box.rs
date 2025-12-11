use dialect_coach_shared::models::{ArabicScript, LanguageOption};
use web_sys::KeyboardEvent;
use yew::prelude::*;

fn font_class(lang_option: &Option<LanguageOption>) -> &'static str {
    match lang_option {
        Some(LanguageOption::Arabic(script)) => match script {
            ArabicScript::Naskh => "arabic-naskh",
            ArabicScript::Ruqa => "arabic-ruqa",
            ArabicScript::Latin => "arabic-latin",
        },
        _ => "",
    }
}

#[derive(Properties, PartialEq)]
pub struct InputBoxProps {
    pub on_send: Callback<String>,
    #[prop_or(false)]
    pub disabled: bool,
    // external_value removed
    #[prop_or_default]
    pub textarea_ref: Option<NodeRef>,
    #[prop_or_default]
    pub language_option: Option<LanguageOption>,
}

#[function_component(InputBox)]
pub fn input_box(props: &InputBoxProps) -> Html {
    let input_value = use_state(String::new);
    let default_ref = use_node_ref();
    let textarea_node_ref = props.textarea_ref.clone().unwrap_or(default_ref);
    // Removed external_value sync logic

    let on_input = {
        let input_value = input_value.clone();
        Callback::from(move |e: InputEvent| {
            if let Some(textarea) = e.target_dyn_into::<web_sys::HtmlTextAreaElement>() {
                input_value.set(textarea.value());
            }
        })
    };

    let on_submit = {
        let input_value = input_value.clone();
        let on_send = props.on_send.clone();
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            let value = (*input_value).clone();
            if !value.trim().is_empty() {
                on_send.emit(value);
                input_value.set(String::new());
            }
        })
    };

    let on_keydown = {
        let input_value = input_value.clone();
        let on_send = props.on_send.clone();
        Callback::from(move |e: KeyboardEvent| {
            if e.shift_key() && e.key() == "Enter" {
                e.prevent_default();
                let value = (*input_value).clone();
                if !value.trim().is_empty() {
                    on_send.emit(value);
                    input_value.set(String::new());
                }
            }
        })
    };

    let font_class_name = font_class(&props.language_option);

    html! {
        <form class="composer" onsubmit={on_submit}>
            <textarea
                ref={textarea_node_ref}
                class={classes!("composer-input", font_class_name)}
                placeholder={if props.disabled { "Connecting..." } else { "Type message… (Shift+Enter to send)" }}
                value={(*input_value).clone()}
                oninput={on_input}
                onkeydown={on_keydown}
                disabled={props.disabled}
                rows="3"
                aria-label="Type your message here. Press Shift+Enter to send."
            />
            <div class="composer-actions">
                <button type="submit" class="btn btn--primary" disabled={props.disabled}>{"Send"}</button>
            </div>
        </form>
    }
}
