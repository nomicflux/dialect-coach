use crate::services::enrichment_service::EnrichmentService;
use dialect_coach_shared::models::{
    Dialect, EnrichRequest, Explained, PartialLearningItem, PartialTranslated, PlanStep, StepType,
    Translated,
    learning_item::{LearningItem, LearningItemType},
};
use std::rc::Rc;
use web_sys::{HtmlInputElement, HtmlTextAreaElement};
use yew::prelude::*;

#[derive(Clone, PartialEq)]
pub struct EditableStep {
    pub step: PlanStep,
    pub is_bulk_open: bool,
    pub bulk_text: String,
    pub bulk_delimiter: String,
    pub is_enriching_bulk: bool,
}

impl EditableStep {
    pub fn new(step: PlanStep) -> Self {
        Self {
            step,
            is_bulk_open: false,
            bulk_text: String::new(),
            bulk_delimiter: "auto".to_string(),
            is_enriching_bulk: false,
        }
    }
}

#[derive(Properties, PartialEq, Clone)]
pub struct StepEditorProps {
    pub step: EditableStep,
    pub index: usize,
    pub on_update: Callback<EditableStep>,
    pub on_remove: Callback<()>,
    pub enrichment_service: Rc<EnrichmentService>,
    pub target_dialect: Dialect,
}

#[function_component(StepEditor)]
pub fn step_editor(props: &StepEditorProps) -> Html {
    let on_title_change = {
        let props = props.clone();
        Callback::from(move |e: Event| {
            let target: web_sys::HtmlInputElement = e.target_unchecked_into();
            let mut new_state = props.step.clone();
            new_state.step.title = target.value();
            props.on_update.emit(new_state);
        })
    };

    let on_instructions_change = {
        let props = props.clone();
        Callback::from(move |e: Event| {
            let target: web_sys::HtmlTextAreaElement = e.target_unchecked_into();
            let mut new_state = props.step.clone();
            new_state.step.instructions = target.value();
            props.on_update.emit(new_state);
        })
    };

    let on_step_type_change = {
        let props = props.clone();
        Callback::from(move |e: Event| {
            let target: web_sys::HtmlSelectElement = e.target_unchecked_into();
            let mut new_state = props.step.clone();
            let value = target.value();
            match value.as_str() {
                "Learning" => new_state.step.step_type = StepType::Learning,
                "Review" => {
                    new_state.step.step_type = StepType::Review {
                        review_step_ids: vec![],
                    }
                }
                _ => {}
            }
            props.on_update.emit(new_state);
        })
    };

    let add_vocab = {
        let props = props.clone();
        Callback::from(move |_| {
            let mut new_state = props.step.clone();
            let item = LearningItem::new(
                LearningItemType::Translation(Translated::new(
                    "".to_string(),
                    "".to_string(),
                    None,
                )),
                props.target_dialect,
            );
            new_state.step.content.items.push(item);
            props.on_update.emit(new_state);
        })
    };

    let add_grammar = {
        let props = props.clone();
        Callback::from(move |_| {
            let mut new_state = props.step.clone();
            let item = LearningItem::new(
                LearningItemType::Explanation(Explained::new("".to_string(), "".to_string())),
                props.target_dialect,
            );
            new_state.step.content.items.push(item);
            props.on_update.emit(new_state);
        })
    };

    let add_examples = {
        let props = props.clone();
        Callback::from(move |_| {
            let mut new_state = props.step.clone();
            let item = LearningItem::new(
                LearningItemType::Translation(Translated::new(
                    "".to_string(),
                    "".to_string(),
                    Some("Reference Example".to_string()),
                )),
                props.target_dialect,
            );
            new_state.step.content.items.push(item);
            props.on_update.emit(new_state);
        })
    };

    let remove_content = {
        let props = props.clone();
        Callback::from(move |index: usize| {
            let mut new_state = props.step.clone();
            if index < new_state.step.content.items.len() {
                new_state.step.content.items.remove(index);
                props.on_update.emit(new_state);
            }
        })
    };

    let update_item_at = {
        let props = props.clone();
        Callback::from(move |(index, item): (usize, LearningItem)| {
            let mut new_state = props.step.clone();
            if index < new_state.step.content.items.len() {
                new_state.step.content.items[index] = item;
                props.on_update.emit(new_state);
            }
        })
    };

    let toggle_bulk = {
        let props = props.clone();
        Callback::from(move |_| {
            let mut new_state = props.step.clone();
            new_state.is_bulk_open = !new_state.is_bulk_open;
            props.on_update.emit(new_state);
        })
    };

    let update_bulk_text = {
        let props = props.clone();
        Callback::from(move |e: Event| {
            let target: web_sys::HtmlTextAreaElement = e.target_unchecked_into();
            let mut new_state = props.step.clone();
            new_state.bulk_text = target.value();
            props.on_update.emit(new_state);
        })
    };

    let update_bulk_delimiter = {
        let props = props.clone();
        Callback::from(move |e: Event| {
            let target: web_sys::HtmlSelectElement = e.target_unchecked_into();
            let mut new_state = props.step.clone();
            new_state.bulk_delimiter = target.value();
            props.on_update.emit(new_state);
        })
    };

    let add_bulk_items = {
        let props = props.clone();
        let enrichment_service = props.enrichment_service.clone();

        Callback::from(move |_| {
            let text = props.step.bulk_text.clone();
            let delimiter = props.step.bulk_delimiter.clone();
            let props = props.clone();
            let enrichment_service = enrichment_service.clone();

            // 1. Split text
            let raw_items: Vec<String> = match delimiter.as_str() {
                "newline" => text.split('\n').map(|s| s.trim().to_string()).collect(),
                "comma" => text.split(',').map(|s| s.trim().to_string()).collect(),
                "semicolon" => text.split(';').map(|s| s.trim().to_string()).collect(),
                _ => {
                    // Auto
                    if text.contains('\n') {
                        text.split('\n').map(|s| s.trim().to_string()).collect()
                    } else if text.contains(';') {
                        text.split(';').map(|s| s.trim().to_string()).collect()
                    } else {
                        text.split(',').map(|s| s.trim().to_string()).collect()
                    }
                }
            };

            let items_to_process: Vec<String> =
                raw_items.into_iter().filter(|s| !s.is_empty()).collect();

            if items_to_process.is_empty() {
                return;
            }

            // Set enriching state
            let mut enriching_state = props.step.clone();
            enriching_state.is_enriching_bulk = true;
            props.on_update.emit(enriching_state);

            let props_for_async = props.clone();

            wasm_bindgen_futures::spawn_local(async move {
                let mut new_items: Vec<LearningItem> = Vec::new();

                let futures = items_to_process.iter().map(|phrase| {
                    let service = enrichment_service.clone();
                    let phrase = phrase.clone();
                    async move {
                        let partial = PartialLearningItem::Translated(PartialTranslated {
                            translated_word: Some(phrase),
                            translated_to: None,
                            context: None,
                        });

                        let request = EnrichRequest {
                            dialect: props.target_dialect,
                            partial_data: partial,
                        };

                        service.enrich_learning_item(request).await
                    }
                });

                let results = futures_util::future::join_all(futures).await;

                for response in results.into_iter().flatten() {
                    if let Some(obj) = response.enriched_item.as_object() {
                        let word = obj
                            .get("translated_word")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let trans = obj
                            .get("translated_to")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let ctx = obj
                            .get("context")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());

                        let item = LearningItem::new(
                            LearningItemType::Translation(Translated::new(word, trans, ctx)),
                            props.target_dialect,
                        );
                        new_items.push(item);
                    }
                }

                // Update state with new items and reset UI
                let mut final_state = props_for_async.step.clone();
                final_state.step.content.items.extend(new_items);
                final_state.is_enriching_bulk = false;
                final_state.is_bulk_open = false;
                final_state.bulk_text = String::new();

                props_for_async.on_update.emit(final_state);
            });
        })
    };

    html! {
        <div class="step-editor">
            <div class="step-header">
                <h5>{format!("Step {}", props.index + 1)}</h5>
                <button type="button" class="remove-step-btn" onclick={props.on_remove.reform(|_| ())}>{"Remove"}</button>
            </div>

            <div class="step-fields">
                <input
                    type="text"
                    class="step-title-input"
                    placeholder="Step Title (e.g. 'Verbs with con')"
                    value={props.step.step.title.clone()}
                    onchange={on_title_change}
                />

                <div class="step-meta-controls">
                     <select class="step-type-select" onchange={on_step_type_change}>
                        <option value="Learning" selected={matches!(props.step.step.step_type, StepType::Learning)}>{"Learning"}</option>
                        <option value="Review" selected={matches!(props.step.step.step_type, StepType::Review{..})}>{"Review"}</option>
                    </select>
                </div>

                <textarea
                    class="step-instructions-input"
                    placeholder="Instructions for this step..."
                    value={props.step.step.instructions.clone()}
                    onchange={on_instructions_change}
                />
            </div>

            <div class="step-content-section">
                <h5>{"Content"}</h5>
                <div class="content-actions">
                    <button type="button" onclick={add_vocab} class="add-content-btn">
                        <span class="btn-icon">{"+"}</span>
                        {"Vocab"}
                    </button>
                    <button type="button" onclick={add_grammar} class="add-content-btn">
                        <span class="btn-icon">{"+"}</span>
                        {"Grammar"}
                    </button>
                    <button type="button" onclick={add_examples} class="add-content-btn">
                        <span class="btn-icon">{"+"}</span>
                        {"Example"}
                    </button>

                    <button
                        type="button"
                        class={classes!("add-content-btn", "bulk-btn", props.step.is_bulk_open.then_some("active"))}
                        onclick={toggle_bulk}
                    >
                         <span class="btn-icon">{"📥"}</span>
                         {"Bulk Add"}
                    </button>
                </div>

                if props.step.is_bulk_open {
                    <div class="bulk-add-panel">
                        <div class="bulk-controls">
                             <label>{"Delimiter:"}</label>
                             <select
                                value={props.step.bulk_delimiter.clone()}
                                onchange={update_bulk_delimiter}
                             >
                                 <option value="auto">{"Auto"}</option>
                                 <option value="newline">{"Newline"}</option>
                                 <option value="comma">{"Comma"}</option>
                                 <option value="semicolon">{"Semicolon"}</option>
                             </select>
                        </div>
                        <textarea
                            class="bulk-textarea"
                            placeholder="Paste multiple items here..."
                            value={props.step.bulk_text.clone()}
                            onchange={update_bulk_text}
                        />
                         <button
                             type="button"
                             class="bulk-process-btn"
                             onclick={add_bulk_items}
                             disabled={props.step.is_enriching_bulk || props.step.bulk_text.trim().is_empty()}
                         >
                            {if props.step.is_enriching_bulk { "Enriching Items..." } else { "Add & Enrich Items" }}
                         </button>
                    </div>
                }

                <div class="content-blocks-list">
                    {for props.step.step.content.items.iter().enumerate().map(|(idx, item)| {
                         render_content_editor(idx, item, update_item_at.clone(), remove_content.clone())
                    })}
                </div>
            </div>
        </div>
    }
}

fn render_content_editor(
    index: usize,
    item: &LearningItem,
    on_update: Callback<(usize, LearningItem)>,
    on_remove: Callback<usize>,
) -> Html {
    let on_delete = {
        let on_remove = on_remove.clone();
        Callback::from(move |_| on_remove.emit(index))
    };

    match &item.item {
        LearningItemType::Translation(trans) => {
            let item_clone = item.clone();
            let on_word_change = {
                let on_update = on_update.clone();
                let item_base = item_clone.clone();
                Callback::from(move |e: Event| {
                    let mut new_item = item_base.clone();
                    let input: HtmlInputElement = e.target_unchecked_into();
                    if let LearningItemType::Translation(t) = &mut new_item.item {
                        t.translated_word = input.value();
                        on_update.emit((index, new_item));
                    }
                })
            };
            let item_clone2 = item.clone();
            let on_trans_change = {
                let on_update = on_update.clone();
                let item_base = item_clone2.clone();
                Callback::from(move |e: Event| {
                    let mut new_item = item_base.clone();
                    let input: HtmlInputElement = e.target_unchecked_into();
                    if let LearningItemType::Translation(t) = &mut new_item.item {
                        t.translated_to = input.value();
                        on_update.emit((index, new_item));
                    }
                })
            };

            html! {
                <div class="content-block-editor vocab-block">
                    <div class="block-header">
                        <span class="block-type">{"Translation"}</span>
                        <button type="button" class="remove-block-btn" onclick={on_delete}>{"×"}</button>
                    </div>
                    <div class="vocab-inputs">
                        <input type="text" placeholder="Word/Phrase" value={trans.translated_word.clone()} onchange={on_word_change} />
                        <span class="arrow">{"→"}</span>
                        <input type="text" placeholder="Translation" value={trans.translated_to.clone()} onchange={on_trans_change} />
                    </div>
                </div>
            }
        }
        LearningItemType::Explanation(expl) => {
            let item_clone = item.clone();
            let on_phrase_change = {
                let on_update = on_update.clone();
                let item_base = item_clone.clone();
                Callback::from(move |e: Event| {
                    let mut new_item = item_base.clone();
                    let input: HtmlInputElement = e.target_unchecked_into();
                    if let LearningItemType::Explanation(ex) = &mut new_item.item {
                        ex.new_phrase = input.value();
                        on_update.emit((index, new_item));
                    }
                })
            };
            let item_clone2 = item.clone();
            let on_expl_change = {
                let on_update = on_update.clone();
                let item_base = item_clone2.clone();
                Callback::from(move |e: Event| {
                    let mut new_item = item_base.clone();
                    let input: HtmlTextAreaElement = e.target_unchecked_into();
                    if let LearningItemType::Explanation(ex) = &mut new_item.item {
                        ex.explanation = input.value();
                        on_update.emit((index, new_item));
                    }
                })
            };

            html! {
                <div class="content-block-editor grammar-block">
                    <div class="block-header">
                        <span class="block-type">{"Explanation"}</span>
                        <button type="button" class="remove-block-btn" onclick={on_delete}>{"×"}</button>
                    </div>
                    <input class="rule-title-input" type="text" placeholder="Concept (e.g. 'Future Tense')" value={expl.new_phrase.clone()} onchange={on_phrase_change} />
                    <textarea class="rule-desc-input" placeholder="Explanation..." value={expl.explanation.clone()} onchange={on_expl_change}></textarea>
                </div>
            }
        }
        _ => html! { <div>{"Unsupported Item Type"}</div> },
    }
}
