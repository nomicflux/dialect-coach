use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct Props {
    pub selected_text: String,
    pub message_context: String,
    pub position: (f64, f64),
    pub on_translate: Callback<(String, String)>,
}

#[function_component(TranslateSelectionButton)]
pub fn translate_selection_button(props: &Props) -> Html {
    let on_click = {
        let on_translate = props.on_translate.clone();
        let selected_text = props.selected_text.clone();
        let context = props.message_context.clone();

        Callback::from(move |e: MouseEvent| {
            e.stop_propagation();
            on_translate.emit((selected_text.clone(), context.clone()));
        })
    };

    let style = format!(
        "position: fixed; left: {}px; top: {}px; z-index: 1000;",
        props.position.0, props.position.1
    );

    html! {
        <button
            class="translate-selection-btn"
            style={style}
            onclick={on_click}
        >
            {"Translate"}
        </button>
    }
}
