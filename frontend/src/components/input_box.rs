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

    html! {
        <form class="composer" onsubmit={on_submit}>
            <textarea
                class="composer-input"
                placeholder={if props.disabled { "Connecting..." } else { "Practice a phrase… try '¿Cómo te llamas?'" }}
                value={(*input_value).clone()}
                oninput={on_input}
                disabled={props.disabled}
                rows="3"
                aria-label="Type your message here. Press Enter to send, Shift+Enter for new line."
            />
            <div class="composer-actions">
                <button type="submit" class="btn btn--primary" disabled={props.disabled}>{"Send"}</button>
            </div>
        </form>
    }
}
