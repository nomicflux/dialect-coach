use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct InputBoxProps {
    pub on_send: Callback<String>,
    #[prop_or(false)]
    pub disabled: bool,
}

#[function_component(InputBox)]
pub fn input_box(props: &InputBoxProps) -> Html {
    let input_value = use_state(String::new);

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
        <form class="input-box" onsubmit={on_submit}>
            <textarea
                placeholder={if props.disabled { "Connecting..." } else { "Type a message..." }}
                value={(*input_value).clone()}
                oninput={on_input}
                disabled={props.disabled}
                rows="3"
            />
            <button type="submit" disabled={props.disabled}>{"Send"}</button>
        </form>
    }
}
