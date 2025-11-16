use web_sys::KeyboardEvent;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct InputBoxProps {
    pub on_send: Callback<String>,
    #[prop_or(false)]
    pub disabled: bool,
    #[prop_or_default]
    pub external_value: Option<String>,
}

#[function_component(InputBox)]
pub fn input_box(props: &InputBoxProps) -> Html {
    let input_value = use_state(String::new);

    // Update input value if external value is provided
    {
        let input_value = input_value.clone();
        let external_value = props.external_value.clone();
        use_effect_with(external_value, move |external_val| {
            if let Some(value) = external_val {
                input_value.set(value.clone());
            }
            || ()
        });
    }

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

    html! {
        <form class="composer" onsubmit={on_submit}>
            <textarea
                class="composer-input"
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
