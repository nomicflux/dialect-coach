use dialect_coach_shared::models::{GrammarExplanation, PhraseTranslation};
use yew::prelude::*;

#[derive(Clone, PartialEq)]
pub enum SelectionResult {
    Translation(Vec<PhraseTranslation>),
    Grammar(Vec<GrammarExplanation>),
}

#[derive(Properties, PartialEq)]
pub struct SelectionModalProps {
    pub original_text: String,
    pub result: Option<SelectionResult>,
    pub on_close: Callback<()>,
    pub on_save: Callback<(String, String, String)>,
}

#[function_component(SelectionModal)]
pub fn selection_modal(props: &SelectionModalProps) -> Html {
    let on_backdrop_click = {
        let on_close = props.on_close.clone();
        Callback::from(move |_| on_close.emit(()))
    };

    let on_modal_click = Callback::from(|e: MouseEvent| {
        e.stop_propagation();
    });

    html! {
        <div class="modal-overlay" onclick={on_backdrop_click}>
            <div class="modal-container" onclick={on_modal_click}>
                <div class="modal-header">
                    <h3>{get_modal_title(&props.result)}</h3>
                    <button class="modal-close" onclick={props.on_close.reform(|_| ())}>
                        {"×"}
                    </button>
                </div>
                <div class="modal-body">
                    <div class="original-sentence">
                        <strong>{"Selected: "}</strong>
                        <span>{&props.original_text}</span>
                    </div>
                    {render_content(&props.result, &props.on_save, &props.original_text)}
                </div>
            </div>
        </div>
    }
}

fn get_modal_title(result: &Option<SelectionResult>) -> &'static str {
    match result {
        Some(SelectionResult::Translation(_)) => "Translation",
        Some(SelectionResult::Grammar(_)) => "Grammar Explanation",
        None => "Loading...",
    }
}

fn render_content(
    result: &Option<SelectionResult>,
    on_save: &Callback<(String, String, String)>,
    context: &str,
) -> Html {
    match result {
        Some(SelectionResult::Translation(phrases)) => render_phrases(phrases, on_save, context),
        Some(SelectionResult::Grammar(explanations)) => {
            render_grammar_explanations(explanations, on_save, context)
        }
        None => render_loading(),
    }
}

fn render_loading() -> Html {
    html! {
        <div class="modal-loading">
            <div class="loading-spinner"></div>
            <p>{"Loading..."}</p>
        </div>
    }
}

fn render_phrases(
    phrases: &[PhraseTranslation],
    on_save: &Callback<(String, String, String)>,
    context: &str,
) -> Html {
    html! {
        <div class="phrases-list">
            {phrases.iter().map(|phrase| render_phrase_row(phrase, on_save, context)).collect::<Html>()}
        </div>
    }
}

fn render_grammar_explanations(
    explanations: &[GrammarExplanation],
    on_save: &Callback<(String, String, String)>,
    context: &str,
) -> Html {
    html! {
        <div class="phrases-list">
            {explanations.iter().map(|exp| render_grammar_row(exp, on_save, context)).collect::<Html>()}
        </div>
    }
}

fn render_phrase_row(
    phrase: &PhraseTranslation,
    on_save: &Callback<(String, String, String)>,
    context: &str,
) -> Html {
    let target = phrase.target_text.clone();
    let english = phrase.english.clone();
    let ctx = context.to_string();

    let on_save_click = {
        let on_save = on_save.clone();
        Callback::from(move |_: MouseEvent| {
            on_save.emit((target.clone(), english.clone(), ctx.clone()));
        })
    };

    html! {
        <div class="phrase-row">
            <span class="phrase-text">
                {&phrase.target_text}{" → "}{&phrase.english}
            </span>
            <button class="save-button" onclick={on_save_click}>
                {"💾 Save"}
            </button>
        </div>
    }
}

fn render_grammar_row(
    exp: &GrammarExplanation,
    on_save: &Callback<(String, String, String)>,
    context: &str,
) -> Html {
    let element = exp.element.clone();
    let explanation = exp.explanation.clone();
    let ctx = context.to_string();

    let on_save_click = {
        let on_save = on_save.clone();
        Callback::from(move |_: MouseEvent| {
            on_save.emit((element.clone(), explanation.clone(), ctx.clone()));
        })
    };

    html! {
        <div class="phrase-row">
            <span class="phrase-text">
                {&exp.element}{" → "}{&exp.explanation}
            </span>
            <button class="save-button" onclick={on_save_click}>
                {"💾 Save"}
            </button>
        </div>
    }
}

// Backwards compatibility alias
pub type TranslationModal = SelectionModal;
