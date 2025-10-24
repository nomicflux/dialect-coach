use yew::prelude::*;
use crate::app::{LearningItem, LearningItemType};

#[derive(Properties, PartialEq)]
pub struct LearningPanelProps {
    pub items: Vec<LearningItem>,
    pub is_open: bool,
}

fn calculate_color_from_score(score: u8, is_mistake: bool) -> String {
    let intensity = 0.5 + (score as f32 / 100.0) * 0.5;
    if is_mistake {
        format!("rgba(255, 107, 107, {})", intensity)
    } else {
        format!("rgba(78, 205, 196, {})", intensity)
    }
}

fn calculate_bar_width(score: u8) -> String {
    format!("{}%", score)
}

fn get_item_content(item: &LearningItem) -> String {
    match &item.item {
        LearningItemType::Mistake(m) => m.get_content().to_string(),
        LearningItemType::Explanation(e) => e.get_content().to_string(),
    }
}

fn is_mistake(item: &LearningItem) -> bool {
    matches!(&item.item, LearningItemType::Mistake(_))
}

#[function_component(LearningPanel)]
pub fn learning_panel(props: &LearningPanelProps) -> Html {
    if !props.is_open {
        return html! {};
    }

    html! {
        <div class="learning-panel">
            <h3>{"Learning Progress"}</h3>
            <ul class="learning-items">
                {for props.items.iter().map(|item| {
                    let content = get_item_content(item);
                    let is_mistake_item = is_mistake(item);
                    let color = calculate_color_from_score(item.score, is_mistake_item);
                    let bar_width = calculate_bar_width(item.score);
                    let item_class = if is_mistake_item { "learning-item mistake" } else { "learning-item explanation" };

                    html! {
                        <li class={item_class}>
                            <span class="item-content" style={format!("color: {}", color)}>
                                {content}
                            </span>
                            <div class="progress-bar">
                                <div class="progress-fill" style={format!("width: {}", bar_width)}></div>
                            </div>
                        </li>
                    }
                })}
            </ul>
        </div>
    }
}
