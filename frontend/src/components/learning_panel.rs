use dialect_coach_shared::{LearningItem, LearningItemType};
use uuid::Uuid;
use yew::prelude::*;

fn get_learning_item_id(item: &LearningItem) -> Uuid {
    match &item.item {
        LearningItemType::Mistake(m) => m.id,
        LearningItemType::Explanation(e) => e.id,
        LearningItemType::Translation(t) => t.id,
        LearningItemType::Exploration(e) => e.id,
    }
}

#[derive(Properties, PartialEq)]
pub struct LearningPanelProps {
    pub items: Vec<LearningItem>,
    pub is_open: bool,
    pub on_close: Callback<()>,
    pub on_delete: Callback<Uuid>,
    pub on_undo: Callback<()>,
    pub deleted_count: usize,
}

fn calculate_color_from_score(score: u8, item_type: &LearningItemType) -> String {
    let intensity = 0.5 + (score as f32 / 100.0) * 0.5;
    match item_type {
        LearningItemType::Mistake(_) => format!("rgba(255, 107, 107, {})", intensity), // coral
        LearningItemType::Explanation(_) => format!("rgba(78, 205, 196, {})", intensity), // teal
        LearningItemType::Translation(_) => format!("rgba(255, 230, 109, {})", intensity), // yellow
        LearningItemType::Exploration(_) => format!("rgba(81, 207, 102, {})", intensity), // green
    }
}

fn calculate_bar_width(score: u8) -> String {
    format!("{}%", score)
}

fn get_item_content(item: &LearningItem) -> String {
    match &item.item {
        LearningItemType::Mistake(m) => m.get_content().to_string(),
        LearningItemType::Explanation(e) => e.get_content().to_string(),
        LearningItemType::Translation(t) => t.get_content().to_string(),
        LearningItemType::Exploration(e) => e.get_content().to_string(),
    }
}

fn get_tooltip(item: &LearningItem) -> Option<String> {
    match &item.item {
        LearningItemType::Mistake(m) => Some(m.mistake_category.to_string()),
        LearningItemType::Explanation(e) => Some(e.explanation.clone()),
        LearningItemType::Translation(t) => Some(format!("Translation: {}", t.translated_to)),
        LearningItemType::Exploration(e) => Some(e.instructions_for_use.clone()),
    }
}

fn get_item_class(item: &LearningItem) -> &'static str {
    match &item.item {
        LearningItemType::Mistake(_) => "learning-item mistake",
        LearningItemType::Explanation(_) => "learning-item explanation",
        LearningItemType::Translation(_) => "learning-item translation",
        LearningItemType::Exploration(_) => "learning-item exploration",
    }
}

fn render_learning_item(item: &LearningItem, on_delete: Callback<Uuid>) -> Html {
    let content = get_item_content(item);
    let tooltip = get_tooltip(item);
    let color = calculate_color_from_score(item.score, &item.item);
    let bar_width = calculate_bar_width(item.score);
    let item_class = get_item_class(item);
    let item_id = get_learning_item_id(item);

    html! {
        <li class={item_class}>
            <button
                class="delete-button"
                onclick={on_delete.reform(move |_| item_id)}
            >
                {"×"}
            </button>
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
            <div class="learning-panel-content">
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
                            {for accomplishments.iter().map(|item| render_learning_item(item, props.on_delete.clone()))}
                        </ul>
                    </div>
                }

                if !still_learning.is_empty() {
                    <div class="learning-section">
                        <h4 class="section-title">{"Still Learning"}</h4>
                        <ul class="learning-items">
                            {for still_learning.iter().map(|item| render_learning_item(item, props.on_delete.clone()))}
                        </ul>
                    </div>
                }

                if accomplishments.is_empty() && still_learning.is_empty() {
                    <p class="empty-message">{"No learning items yet. Start chatting to build your learning progress!"}</p>
                }
            </div>

            if props.deleted_count > 0 {
                <div class="undo-notification">
                    <span>{"Item deleted"}</span>
                    <button class="undo-button" onclick={{
                        let on_undo = props.on_undo.clone();
                        Callback::from(move |_| on_undo.emit(()))
                    }}>
                        {"Undo"}
                    </button>
                </div>
            }
        </div>
    }
}
