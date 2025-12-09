use dialect_coach_shared::models::{
    LanguagePlan, PlanContent, StepStatus, StepType, learning_item::LearningItemType,
};
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct ActivePlanProps {
    pub plan: LanguagePlan,
    pub on_advance: Callback<uuid::Uuid>,
}

#[function_component(ActivePlan)]
pub fn active_plan(props: &ActivePlanProps) -> Html {
    let current_step = props.plan.current_step();
    let total_steps = props.plan.steps.len();
    let current_step_num = props.plan.current_step_index + 1;
    let progress_pct = (props.plan.current_step_index as f64 / total_steps as f64) * 100.0;

    let on_advance = {
        let on_advance = props.on_advance.clone();
        let plan_id = props.plan.id;
        Callback::from(move |_| on_advance.emit(plan_id))
    };

    html! {
        <div class="active-plan-card">
            <div class="active-plan-header">
                <h4 class="active-plan-title">{&props.plan.title}</h4>
                <span class="step-counter">{format!("Step {}/{}", current_step_num, total_steps)}</span>
            </div>

            <div class="plan-progress-bar">
                <div class="plan-progress-fill" style={format!("width: {}%", progress_pct)}></div>
            </div>

            if let Some(step) = current_step {
                <div class="current-step-content">
                    <h5 class="step-title">{&step.title}</h5>
                    <div class="step-instructions">
                        {render_step_icon(&step.step_type)}
                        <p>{&step.instructions}</p>
                    </div>

                    {render_plan_content(&step.content)}

                    if step.status == StepStatus::InProgress {
                        <button class="advance-step-btn" onclick={on_advance}>
                            {"Mark Step Complete"}
                        </button>
                    } else if step.status == StepStatus::Completed {
                         <div class="step-completed-badge">{"✓ Completed"}</div>
                    }
                </div>
            } else {
                <div class="plan-completed-message">
                    <span>{"🎉 Plan Completed!"}</span>
                </div>
            }
        </div>
    }
}

fn render_step_icon(step_type: &StepType) -> Html {
    match step_type {
        StepType::Learning => html! { <span class="step-icon">{"📚"}</span> },
        StepType::Review { .. } => html! { <span class="step-icon">{"↺"}</span> },
    }
}

fn render_plan_content(content: &PlanContent) -> Html {
    if content.items.is_empty() && content.agent_instructions.is_empty() {
        return html! {};
    }

    html! {
        <div class="active-step-content-blocks">
            {if !content.agent_instructions.is_empty() {
                 html! { <div class="agent-instructions-note"><strong>{"Note explicitly for Agent:"}</strong> {&content.agent_instructions}</div> }
            } else { html!{} }}

            {for content.items.iter().map(|item| {
                match &item.item {
                     LearningItemType::Translation(trans) => html! {
                        <div class="content-block-view vocab">
                            <h6 class="content-title">{"Translation"}</h6>
                            <div class="translation-pair">
                                <span class="word">{&trans.translated_word}</span>
                                <span class="arrow">{" → "}</span>
                                <span class="trans">{&trans.translated_to}</span>
                            </div>
                            if let Some(ctx) = &trans.context {
                                <div class="context-note">{ctx}</div>
                            }
                        </div>
                    },
                    LearningItemType::Explanation(expl) => html! {
                        <div class="content-block-view grammar">
                            <h6 class="content-title">{format!("Explanation: {}", expl.new_phrase)}</h6>
                            <p class="content-desc">{&expl.explanation}</p>
                        </div>
                    },
                     _ => html! {}
                }
            })}
        </div>
    }
}
