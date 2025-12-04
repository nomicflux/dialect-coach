use dialect_coach_shared::models::PhraseTranslation;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct TranslationModalProps {
    pub original_sentence: String,
    pub phrases: Option<Vec<PhraseTranslation>>,
    pub on_close: Callback<()>,
    pub on_save_phrase: Callback<(String, String, String)>,
}

#[function_component(TranslationModal)]
pub fn translation_modal(props: &TranslationModalProps) -> Html {
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
                    <h3>{"Translation"}</h3>
                    <button class="modal-close" onclick={props.on_close.reform(|_| ())}>
                        {"×"}
                    </button>
                </div>
                <div class="modal-body">
                    <div class="original-sentence">
                        <strong>{"Original: "}</strong>
                        <span>{&props.original_sentence}</span>
                    </div>
                    {render_content(&props.phrases, &props.on_save_phrase, &props.original_sentence)}
                </div>
            </div>
        </div>
    }
}

fn render_content(
    phrases: &Option<Vec<PhraseTranslation>>,
    on_save: &Callback<(String, String, String)>,
    context: &str,
) -> Html {
    match phrases {
        Some(phrases) => html! {
            <div class="phrases-list">
                {render_phrases(phrases, on_save, context)}
            </div>
        },
        None => render_loading(),
    }
}

fn render_loading() -> Html {
    html! {
        <div class="modal-loading">
            <div class="loading-spinner"></div>
            <p>{"Translating..."}</p>
        </div>
    }
}

fn render_phrases(
    phrases: &[PhraseTranslation],
    on_save: &Callback<(String, String, String)>,
    context: &str,
) -> Html {
    phrases
        .iter()
        .map(|phrase| render_phrase_row(phrase, on_save, context))
        .collect()
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
