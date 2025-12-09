use super::step_editor::{EditableStep, StepEditor};
use crate::services::enrichment_service::EnrichmentService;
use dialect_coach_shared::models::{
    CompletionCriteria, Dialect, LanguagePlan, PlanContent, PlanStep, StepType,
};
use std::rc::Rc;
use uuid::Uuid;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct PlanCreateProps {
    pub dialect: Dialect,
    pub on_create: Callback<LanguagePlan>,
    pub on_cancel: Callback<()>,
    #[prop_or_default]
    pub plan_to_edit: Option<LanguagePlan>,
    pub enrichment_service: Rc<EnrichmentService>,
}

// EditableStep imported from step_editor.rs

#[function_component(PlanCreate)]
pub fn plan_create(props: &PlanCreateProps) -> Html {
    let title = use_state(|| {
        if let Some(plan) = &props.plan_to_edit {
            plan.title.clone()
        } else {
            String::new()
        }
    });
    let description = use_state(|| {
        if let Some(plan) = &props.plan_to_edit {
            plan.description.clone().unwrap_or_default()
        } else {
            String::new()
        }
    });

    let steps = use_state(|| {
        if let Some(plan) = &props.plan_to_edit {
            plan.steps
                .iter()
                .map(|s| EditableStep::new(s.clone()))
                .collect::<Vec<_>>()
        } else {
            vec![EditableStep::new(PlanStep::new(
                1,
                "".to_string(), // Empty title to start
                StepType::Learning,
                "".to_string(), // Empty instructions
                PlanContent::default(),
                CompletionCriteria::Manual,
            ))]
        }
    });

    let on_submit = {
        let title = title.clone();
        let description = description.clone();
        let steps = steps.clone();
        let dialect = props.dialect;
        let on_create = props.on_create.clone();
        let plan_to_edit = props.plan_to_edit.clone();

        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();

            // Renumber steps before submitting
            let final_steps: Vec<PlanStep> = steps
                .iter()
                .enumerate()
                .map(|(i, s)| {
                    let mut s = s.step.clone();
                    s.step_number = i + 1;
                    // Ensure ID is unique if not already
                    if s.id == Uuid::nil() {
                        s.id = Uuid::new_v4();
                    }
                    s
                })
                .collect();

            let mut plan = LanguagePlan::new(
                (*title).clone(),
                dialect,
                if description.is_empty() {
                    None
                } else {
                    Some((*description).clone())
                },
                final_steps,
            );

            // If editing, preserve the original ID and created status
            if let Some(original) = &plan_to_edit {
                plan.id = original.id;
                plan.created_at = original.created_at;
                plan.status = original.status;
                plan.current_step_index = original.current_step_index;
            }

            on_create.emit(plan);
        })
    };

    let add_step = {
        let steps = steps.clone();
        Callback::from(move |_| {
            let mut new_steps = (*steps).clone();
            new_steps.push(EditableStep::new(PlanStep::new(
                new_steps.len() + 1,
                "".to_string(),
                StepType::Learning,
                "Start your journey".to_string(),
                PlanContent::default(),
                CompletionCriteria::Manual,
            )));
            steps.set(new_steps);
        })
    };

    let is_editing = props.plan_to_edit.is_some();

    html! {
        <div class="plan-create-form">
            <h4 class="section-title">{if is_editing { "Edit Plan" } else { "Create New Plan" }}</h4>
            <form onsubmit={on_submit}>
                <input
                    type="text"
                    placeholder="Plan Title *"
                    value={(*title).clone()}
                    onchange={Callback::from(move |e: Event| {
                        let target: web_sys::HtmlInputElement = e.target_unchecked_into();
                        title.set(target.value());
                    })}
                    required=true
                />
                <textarea
                    placeholder="Description (optional)"
                    value={(*description).clone()}
                    onchange={Callback::from(move |e: Event| {
                        let target: web_sys::HtmlTextAreaElement = e.target_unchecked_into();
                        description.set(target.value());
                    })}
                />

                <div class="steps-section">
                    <label>{"Plan Steps"}</label>
                    {for steps.iter().enumerate().map(|(i, step)| {
                        let steps_handle = steps.clone();
                        // ID-based update: find step by ID to update correct item
                        let step_id = step.step.id;
                        let on_update = Callback::from(move |updated: EditableStep| {
                            let mut new_steps = (*steps_handle).clone();
                            // Find index by ID to ensure we update the correct step regardless of loop variable 'i'
                            // In this component 'i' is stable during render but finding by ID is safer for robustness
                            if let Some(idx) = new_steps.iter().position(|s| s.step.id == step_id) {
                                new_steps[idx] = updated;
                                steps_handle.set(new_steps);
                            }
                        });
                        let steps_handle = steps.clone();
                        let on_remove = Callback::from(move |_| {
                            let mut new_steps = (*steps_handle).clone();
                            new_steps.remove(i);
                            steps_handle.set(new_steps);
                        });

                        html! {
                            <StepEditor
                                key={step.step.id.to_string()}
                                index={i}
                                step={step.clone()}
                                on_update={on_update}
                                on_remove={on_remove}
                                enrichment_service={props.enrichment_service.clone()}
                                target_dialect={props.dialect}
                            />
                        }

                    })}

                    <button type="button" class="add-step-btn" onclick={add_step}>
                        {"+ Add Step"}
                    </button>
                </div>

                <div class="form-buttons">
                    <button type="submit" class="save-button">{if is_editing { "Save Changes" } else { "Create Plan" }}</button>
                    <button type="button" class="cancel-button" onclick={props.on_cancel.reform(|_| ())}>{"Cancel"}</button>
                </div>
            </form>
        </div>
    }
}
