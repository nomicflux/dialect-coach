use yew::prelude::*;
use dialect_coach_shared::models::{LanguagePlan, Dialect, PlanStep, StepType, CompletionCriteria};

#[derive(Properties, PartialEq)]
pub struct PlanCreateProps {
    pub dialect: Dialect,
    pub on_create: Callback<LanguagePlan>,
    pub on_cancel: Callback<()>,
}

#[function_component(PlanCreate)]
pub fn plan_create(props: &PlanCreateProps) -> Html {
    let title = use_state(String::new);
    let description = use_state(String::new);

    let on_submit = {
        let title = title.clone();
        let description = description.clone();
        let dialect = props.dialect;
        let on_create = props.on_create.clone();
        
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            // Create a basic plan with one placeholder step for now
            let steps = vec![
                PlanStep::new(
                    1,
                    "Getting Started".to_string(),
                    StepType::Learning { focus: "Basics".to_string() },
                    "Start your journey".to_string(),
                    CompletionCriteria::Manual
                )
            ];
            
            let plan = LanguagePlan::new(
                (*title).clone(),
                dialect,
                if description.is_empty() { None } else { Some((*description).clone()) },
                steps
            );
            on_create.emit(plan);
        })
    };

    html! {
        <div class="plan-create-form">
            <h4 class="section-title">{"Create New Plan"}</h4>
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
                <div class="form-buttons">
                    <button type="submit" class="save-button">{"Create Plan"}</button>
                    <button type="button" class="cancel-button" onclick={props.on_cancel.reform(|_| ())}>{"Cancel"}</button>
                </div>
            </form>
        </div>
    }
}
