use yew::prelude::*;

#[derive(Clone, PartialEq)]
pub enum SelectionAction {
    Translate,
    ExplainGrammar,
}

#[derive(Properties, PartialEq)]
pub struct Props {
    pub selected_text: String,
    pub message_context: String,
    pub position: (f64, f64),
    pub on_action: Callback<(SelectionAction, String, String)>,
    #[prop_or(false)]
    pub is_loading: bool,
}

#[function_component(SelectionActionButtons)]
pub fn selection_action_buttons(props: &Props) -> Html {
    let on_translate_click = {
        let on_action = props.on_action.clone();
        let selected_text = props.selected_text.clone();
        let context = props.message_context.clone();
        let is_loading = props.is_loading;

        Callback::from(move |e: MouseEvent| {
            if !is_loading {
                e.stop_propagation();
                on_action.emit((SelectionAction::Translate, selected_text.clone(), context.clone()));
            }
        })
    };

    let on_grammar_click = {
        let on_action = props.on_action.clone();
        let selected_text = props.selected_text.clone();
        let context = props.message_context.clone();
        let is_loading = props.is_loading;

        Callback::from(move |e: MouseEvent| {
            if !is_loading {
                e.stop_propagation();
                on_action.emit((SelectionAction::ExplainGrammar, selected_text.clone(), context.clone()));
            }
        })
    };

    let style = format!(
        "position: fixed; left: {}px; top: {}px; z-index: 1000; display: flex; gap: 8px;",
        props.position.0, props.position.1
    );

    html! {
        <div class="selection-action-buttons" style={style}>
            <button
                class="translate-selection-btn"
                onclick={on_translate_click}
                disabled={props.is_loading}
            >
                {"Translate"}
            </button>
            <button
                class="translate-selection-btn"
                onclick={on_grammar_click}
                disabled={props.is_loading}
            >
                {"Grammar"}
            </button>
        </div>
    }
}

pub use SelectionActionButtons as TranslateSelectionButton;
