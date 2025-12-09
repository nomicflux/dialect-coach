use yew::prelude::*;
use dialect_coach_shared::models::{
    PlanStep, StepType,
    Translated, Explained,
    learning_item::{LearningItem, LearningItemType},
    EnrichRequest, PartialLearningItem, PartialTranslated, Dialect
};
use web_sys::{HtmlInputElement, HtmlTextAreaElement};
use crate::services::enrichment_service::EnrichmentService;
use std::rc::Rc;

#[derive(Properties, PartialEq, Clone)]
pub struct StepEditorProps {
    pub step: PlanStep,
    pub index: usize,
    pub on_update: Callback<PlanStep>,
    pub on_remove: Callback<()>,
    pub enrichment_service: Rc<EnrichmentService>,
}

#[function_component(StepEditor)]
pub fn step_editor(props: &StepEditorProps) -> Html {
    let on_title_change = {
        let props = props.clone();
        Callback::from(move |e: Event| {
            let target: web_sys::HtmlInputElement = e.target_unchecked_into();
            let mut new_step = props.step.clone();
            new_step.title = target.value();
            props.on_update.emit(new_step);
        })
    };

    let on_instructions_change = {
        let props = props.clone();
        Callback::from(move |e: Event| {
            let target: web_sys::HtmlTextAreaElement = e.target_unchecked_into();
            let mut new_step = props.step.clone();
            new_step.instructions = target.value();
            props.on_update.emit(new_step);
        })
    };

    let on_step_type_change = {
        let props = props.clone();
        Callback::from(move |e: Event| {
            let target: web_sys::HtmlSelectElement = e.target_unchecked_into();
            let mut new_step = props.step.clone();
            let value = target.value();
            match value.as_str() {
                "Learning" => new_step.step_type = StepType::Learning { focus: "".to_string() },
                "Review" => new_step.step_type = StepType::Review { review_step_ids: vec![] },
                _ => {}
            }
            props.on_update.emit(new_step);
        })
    };

    let on_focus_change = {
        let props = props.clone();
        Callback::from(move |e: Event| {
            let target: web_sys::HtmlInputElement = e.target_unchecked_into();
            let mut new_step = props.step.clone();
            if let StepType::Learning { .. } = new_step.step_type {
                new_step.step_type = StepType::Learning { focus: target.value() };
                props.on_update.emit(new_step);
            }
        })
    };

    let add_vocab = {
        let props = props.clone();
        Callback::from(move |_| {
            let mut new_step = props.step.clone();
            // Start with an empty Translation item
            let item = LearningItem::new(
                LearningItemType::Translation(Translated::new(
                    "".to_string(),
                    "".to_string(),
                    None
                )),
                dialect_coach_shared::models::Dialect::SpanishMexican // TODO: Get from plan?
            );
            new_step.content.items.push(item);
            props.on_update.emit(new_step);
        })
    };

    let add_grammar = {
        let props = props.clone();
        Callback::from(move |_| {
            let mut new_step = props.step.clone();
            // Start with an empty Explanation item
            let item = LearningItem::new(
                LearningItemType::Explanation(Explained::new(
                    "".to_string(),
                    "".to_string()
                )),
                dialect_coach_shared::models::Dialect::SpanishMexican
            );
            new_step.content.items.push(item);
            props.on_update.emit(new_step);
        })
    };

    let add_examples = {
        let props = props.clone();
        Callback::from(move |_| {
            let mut new_step = props.step.clone();
            // Examples are also translations, maybe with context
            let item = LearningItem::new(
                LearningItemType::Translation(Translated::new(
                    "".to_string(),
                    "".to_string(),
                    Some("Reference Example".to_string())
                )),
                dialect_coach_shared::models::Dialect::SpanishMexican
            );
            new_step.content.items.push(item);
            props.on_update.emit(new_step);
        })
    };

    let remove_content = {
        let props = props.clone();
        Callback::from(move |index: usize| {
            let mut new_step = props.step.clone();
            if index < new_step.content.items.len() {
                new_step.content.items.remove(index);
                props.on_update.emit(new_step);
            }
        })
    };

    // Helper for updating items
    let update_item_at = {
        let props = props.clone();
        Callback::from(move |(index, item): (usize, LearningItem)| {
             let mut new_step = props.step.clone();
             if index < new_step.content.items.len() {
                 new_step.content.items[index] = item;
                 props.on_update.emit(new_step);
             }
        })
    };

    let bulk_text = use_state(String::new);
    let show_bulk = use_state(|| false);
    let is_enriching_bulk = use_state(|| false);
    let bulk_delimiter = use_state(|| "auto".to_string());

    let add_bulk_items = {
        let props = props.clone();
        let bulk_text = bulk_text.clone();
        let is_enriching_bulk = is_enriching_bulk.clone();
        let show_bulk = show_bulk.clone();
        let bulk_delimiter = bulk_delimiter.clone();
        let enrichment_service = props.enrichment_service.clone();

        Callback::from(move |_| {
            let text = (*bulk_text).clone();
            let delimiter = (*bulk_delimiter).clone();
            let props = props.clone();
            let is_enriching_bulk = is_enriching_bulk.clone();
            let show_bulk = show_bulk.clone();
            let enrichment_service = enrichment_service.clone();
            
            // 1. Split text
            let raw_items: Vec<String> = match delimiter.as_str() {
                "newline" => text.split('\n').map(|s| s.trim().to_string()).collect(),
                "comma" => text.split(',').map(|s| s.trim().to_string()).collect(),
                "semicolon" => text.split(';').map(|s| s.trim().to_string()).collect(),
                _ => { // Auto
                    if text.contains('\n') {
                         text.split('\n').map(|s| s.trim().to_string()).collect()
                    } else if text.contains(';') {
                         text.split(';').map(|s| s.trim().to_string()).collect()
                    } else {
                         text.split(',').map(|s| s.trim().to_string()).collect() // Fallback to comma if single line
                    }
                }
            };
            
            let items_to_process: Vec<String> = raw_items.into_iter()
                .filter(|s| !s.is_empty())
                .collect();

            if items_to_process.is_empty() {
                return;
            }

            let props_for_async = props.clone();
            let is_enriching_bulk_for_async = is_enriching_bulk.clone();
            let show_bulk_for_async = show_bulk.clone();
            let bulk_text_for_async = bulk_text.clone();

            is_enriching_bulk.set(true);

            wasm_bindgen_futures::spawn_local(async move {
                let mut new_items: Vec<LearningItem> = Vec::new();
                
                // Process in parallel (batches of 5 to avoid overwhelming?)
                // For now, let's just spawn all. Browser limit is like 6.
                // We'll trust the browser scheduler.
                
                let futures = items_to_process.iter().map(|phrase| {
                     let service = enrichment_service.clone();
                     let phrase = phrase.clone();
                     async move {
                         // Create partial item
                         let partial = PartialLearningItem::Translated(PartialTranslated {
                             translated_word: Some(phrase),
                             translated_to: None,
                             context: None
                         });
                         
                         let request = EnrichRequest {
                             dialect: Dialect::SpanishMexican, // TODO: Get from plan? Props don't have dialect, need to add if critical.
                                                              // For now default to Mexican Spanish as it's the main detailed dialect.
                             partial_data: partial,
                         };
                         
                         service.enrich_learning_item(request).await
                     }
                });

                let results = futures_util::future::join_all(futures).await;

                for res in results {
                    if let Ok(response) = res {
                         // Convert JSON value to LearningItem
                         // Note: The response.enriched_item is a serde_json::Value.
                         // We need to parse it back to a LearningItemType or manually construct it.
                         // Since LearningItemType::Translated(Translated) matches the struct
                         // Let's assume enrichment service returns valid fields.
                         
                         if let Some(obj) = response.enriched_item.as_object() {
                             // Manual extraction because we don't have direct deserialization helper here
                             let word = obj.get("translated_word").and_then(|v| v.as_str()).unwrap_or("").to_string();
                             let trans = obj.get("translated_to").and_then(|v| v.as_str()).unwrap_or("").to_string();
                             let ctx = obj.get("context").and_then(|v| v.as_str()).map(|s| s.to_string());
                             
                             let item = LearningItem::new(
                                 LearningItemType::Translation(Translated::new(word, trans, ctx)),
                                 Dialect::SpanishMexican
                             );
                             new_items.push(item);
                         }
                    }
                }

                // Add all new items to the step
                let mut new_step = props_for_async.step.clone();
                new_step.content.items.extend(new_items);
                props_for_async.on_update.emit(new_step);
                
                // Reset state
                is_enriching_bulk_for_async.set(false);
                show_bulk_for_async.set(false);
                bulk_text_for_async.set(String::new());
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
                    value={props.step.title.clone()}
                    onchange={on_title_change}
                />

                <div class="step-meta-controls">
                     <select class="step-type-select" onchange={on_step_type_change}>
                        <option value="Learning" selected={matches!(props.step.step_type, StepType::Learning{..})}>{"Learning"}</option>
                        <option value="Review" selected={matches!(props.step.step_type, StepType::Review{..})}>{"Review"}</option>
                    </select>

                    if let StepType::Learning { focus } = &props.step.step_type {
                        <input
                            type="text"
                            class="step-focus-input"
                            placeholder="Focus (e.g. 'Grammar')"
                            value={focus.clone()}
                            onchange={on_focus_change}
                        />
                    }
                </div>

                <textarea
                    class="step-instructions-input"
                    placeholder="Instructions for this step..."
                    value={props.step.instructions.clone()}
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
                        class={classes!("add-content-btn", "bulk-btn", (*show_bulk).then_some("active"))}
                        onclick={
                            let show_bulk = show_bulk.clone();
                            Callback::from(move |_| show_bulk.set(!*show_bulk))
                        }
                    >
                         <span class="btn-icon">{"📥"}</span>
                         {"Bulk Add"}
                    </button>
                </div>

                if *show_bulk {
                    <div class="bulk-add-panel">
                        <div class="bulk-controls">
                             <label>{"Delimiter:"}</label>
                             <select 
                                value={(*bulk_delimiter).clone()} 
                                onchange={
                                    let bulk_delimiter = bulk_delimiter.clone();
                                    Callback::from(move |e: Event| {
                                        let target: web_sys::HtmlSelectElement = e.target_unchecked_into();
                                        bulk_delimiter.set(target.value());
                                    })
                                }
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
                            value={(*bulk_text).clone()}
                            onchange={
                                let bulk_text = bulk_text.clone();
                                Callback::from(move |e: Event| {
                                    let target: web_sys::HtmlTextAreaElement = e.target_unchecked_into();
                                    bulk_text.set(target.value());
                                })
                            }
                        />
                         <button 
                             type="button" 
                             class="bulk-process-btn" 
                             onclick={add_bulk_items}
                             disabled={*is_enriching_bulk || (*bulk_text).trim().is_empty()}
                         >
                            {if *is_enriching_bulk { "Enriching Items..." } else { "Add & Enrich Items" }}
                         </button>
                    </div>
                }

                <div class="content-blocks-list">
                    {for props.step.content.items.iter().enumerate().map(|(idx, item)| {
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
    on_remove: Callback<usize>
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
        },
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
        },
        _ => html! { <div>{"Unsupported Item Type"}</div> }
    }
}


