use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct InputBoxProps {
    pub on_send: Callback<String>,
}

#[function_component(InputBox)]
pub fn input_box(props: &InputBoxProps) -> Html {
    let input_value = use_state(String::new);

    let on_input = {
        let input_value = input_value.clone();
        Callback::from(move |e: InputEvent| {
            if let Some(input) = e.target_dyn_into::<web_sys::HtmlInputElement>() {
                input_value.set(input.value());
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
            <input
                type="text"
                placeholder="Type a message..."
                value={(*input_value).clone()}
                oninput={on_input}
            />
            <button type="submit">{"Send"}</button>
        </form>
    }
}
