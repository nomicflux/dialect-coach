use web_sys::{HtmlInputElement, MouseEvent};
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct LearningGoalsPanelProps {
    pub goals: Vec<String>,
    pub on_add: Callback<String>,
    pub on_delete: Callback<usize>,
}

fn render_goal_item(goal: &str, index: usize, on_delete: Callback<usize>) -> Html {
    html! {
        <li class="goal-item">
            <span class="goal-text">{goal}</span>
            <button class="delete-button" onclick={Callback::from(move |_| on_delete.emit(index))}>{"×"}</button>
        </li>
    }
}

fn render_goals_list(goals: &[String], on_delete: Callback<usize>) -> Html {
    html! {
        <ul class="goals-list">
            {for goals.iter().enumerate().map(|(i, goal)| render_goal_item(goal, i, on_delete.clone()))}
        </ul>
    }
}

fn render_input_section(input_ref: NodeRef, on_add: Callback<MouseEvent>) -> Html {
    html! {
        <div class="goal-input-container">
            <input ref={input_ref} type="text" class="goal-input" placeholder="Add goal..." />
            <button class="add-button" onclick={on_add}>{"+"}</button>
        </div>
    }
}

#[function_component(LearningGoalsPanel)]
pub fn learning_goals_panel(props: &LearningGoalsPanelProps) -> Html {
    let input_ref = use_node_ref();
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
                <h4>{"Learning Goals"}</h4>
            </div>
            {render_input_section(input_ref, handle_add)}
            {render_goals_list(&props.goals, props.on_delete.clone())}
        </div>
    }
}
