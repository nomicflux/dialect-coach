use yew::prelude::*;
use crate::app::{LearningItem, LearningItemType};

#[derive(Properties, PartialEq)]
pub struct LearningPanelProps {
    pub items: Vec<LearningItem>,
    pub is_open: bool,
    pub on_close: Callback<()>,
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

fn get_tooltip(item: &LearningItem) -> Option<String> {
    match &item.item {
        LearningItemType::Mistake(m) => Some(m.mistake_category.to_string()),
        LearningItemType::Explanation(e) => Some(e.explanation.clone()),
    }
}

fn is_mistake(item: &LearningItem) -> bool {
    matches!(&item.item, LearningItemType::Mistake(_))
}

fn render_learning_item(item: &LearningItem) -> Html {
    let content = get_item_content(item);
    let tooltip = get_tooltip(item);
    let is_mistake_item = is_mistake(item);
    let color = calculate_color_from_score(item.score, is_mistake_item);
    let bar_width = calculate_bar_width(item.score);
    let item_class = if is_mistake_item {
        "learning-item mistake"
    } else {
        "learning-item explanation"
    };

    html! {
        <li class={item_class}>
            <div class="item-with-tooltip">
                <span class="item-content" style={format!("color: {}", color)}>
                    {content}
                </span>
                if let Some(tip) = tooltip {
                    <span class="tooltip-text">{tip}</span>
                }
            </div>
            <div class="progress-bar">
                <div class="progress-fill" style={format!("width: {}", bar_width)}></div>
            </div>
        </li>
    }
}

#[function_component(LearningPanel)]
pub fn learning_panel(props: &LearningPanelProps) -> Html {
    if !props.is_open {
        return html! {};
    }

    let (accomplishments, still_learning): (Vec<_>, Vec<_>) =
        props.items.iter().partition(|item| item.score == 100);

    html! {
        <div class="learning-panel">
            <div class="learning-panel-header">
                <h3>{"Learning Progress"}</h3>
                <button class="learning-panel-close" onclick={{
                    let on_close = props.on_close.clone();
                    Callback::from(move |_| on_close.emit(()))
                }}>
                    {"×"}
                </button>
            </div>

            if !accomplishments.is_empty() {
                <div class="learning-section">
                    <h4 class="section-title">{"Accomplishments"}</h4>
                    <ul class="learning-items">
                        {for accomplishments.iter().map(|item| render_learning_item(item))}
                    </ul>
                </div>
            }

            if !still_learning.is_empty() {
                <div class="learning-section">
                    <h4 class="section-title">{"Still Learning"}</h4>
                    <ul class="learning-items">
                        {for still_learning.iter().map(|item| render_learning_item(item))}
                    </ul>
                </div>
            }

            if accomplishments.is_empty() && still_learning.is_empty() {
                <p class="empty-message">{"No learning items yet. Start chatting to build your learning progress!"}</p>
            }
        </div>
    }
}
