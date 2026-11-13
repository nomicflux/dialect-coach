use crate::components::ruby_text::render_text_with_ruby;
use dialect_coach_shared::models::LearningGoal;
use uuid::Uuid;
use web_sys::{HtmlInputElement, MouseEvent};
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct LearningGoalsPanelProps {
    #[prop_or_default]
    pub activity_id: Option<Uuid>,
    #[prop_or_default]
    pub on_edit: Option<Callback<(usize, String)>>,
    #[prop_or_default]
    pub heading: Option<String>,
    #[prop_or_default]
    pub show_completion: bool,
    pub goals: Vec<LearningGoal>,
    pub on_add: Callback<String>,
    pub on_delete: Callback<usize>,
    #[prop_or_default]
    pub input_ref: Option<NodeRef>,
}

fn render_goal_item(
    goal: &LearningGoal,
    index: usize,
    on_delete: Option<Callback<usize>>,
    on_edit: Option<Callback<(usize, String)>>,
    completion: bool,
    activity_id: Option<Uuid>,
) -> Html {
    let text = goal.goal.clone();
    let onchange = on_edit.clone().map(|edit| {
        Callback::from(move |event: Event| {
            if let Some(input) = event.target_dyn_into::<web_sys::HtmlTextAreaElement>() {
                let value = input.value();
                if value != text {
                    edit.emit((index, value));
                }
            }
        })
    });
    html! {
        <li class="goal-item" dir="auto">
            if completion { <span aria-label={if goal.completed == Some(true) { "Achieved" } else { "Not achieved" }}>{if goal.completed == Some(true) { "✓" } else { "○" }}</span> }
            if let Some(onchange) = onchange { <textarea data-activity-goal="true" data-activity-id={activity_id.map(|id| id.to_string())} data-goal-index={index.to_string()} aria-label={format!("Activity goal {}", index + 1)} class="goal-input" rows="2" value={goal.goal.clone()} {onchange} /> }
            else { <span class="goal-text" data-activity-id={activity_id.map(|id| id.to_string())}>{render_text_with_ruby(&goal.goal)}</span> }
            if let Some(on_delete) = on_delete {
                <button type="button" aria-label="Delete goal" class="delete-button" onclick={Callback::from(move |_| on_delete.emit(index))}>{"×"}</button>
            }
        </li>
    }
}

pub(super) fn render_goals_list(
    goals: &[LearningGoal],
    on_delete: Option<Callback<usize>>,
    on_edit: Option<Callback<(usize, String)>>,
    completion: bool,
    activity_id: Option<Uuid>,
) -> Html {
    html! {
        <ul class="goals-list">
            {for goals.iter().enumerate().map(|(i, goal)| render_goal_item(goal, i, on_delete.clone(), on_edit.clone(), completion, activity_id))}
        </ul>
    }
}

fn render_input_section(input_ref: NodeRef, on_add: Callback<MouseEvent>) -> Html {
    html! {
        <div class="goal-input-container">
            <input ref={input_ref} type="text" class="goal-input" placeholder="Add goal..." />
            <button type="button" aria-label="Add goal" class="add-button" onclick={on_add}>{"+"}</button>
        </div>
    }
}

#[function_component(LearningGoalsPanel)]
pub fn learning_goals_panel(props: &LearningGoalsPanelProps) -> Html {
    let default_ref = use_node_ref();
    let input_ref = props.input_ref.clone().unwrap_or(default_ref);
    let handle_add = {
        let input_ref = input_ref.clone();
        let on_add = props.on_add.clone();
        Callback::from(move |_| {
            if let Some(input) = input_ref.cast::<HtmlInputElement>() {
                on_add.emit(input.value());
                input.set_value("");
            }
        })
    };
    html! {
        <div class="learning-goals-section">
            <div class="goals-header">
                <h4>{props.heading.as_deref().unwrap_or("Learning Goals")}</h4>
            </div>
            {render_input_section(input_ref, handle_add)}
            {render_goals_list(&props.goals, Some(props.on_delete.clone()), props.on_edit.clone(), props.show_completion, props.activity_id)}
        </div>
    }
}
