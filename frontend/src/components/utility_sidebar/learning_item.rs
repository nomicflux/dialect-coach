use dialect_coach_shared::{
    LearningItem, LearningItemType,
};
use uuid::Uuid;
use yew::prelude::*;

pub fn get_learning_item_id(item: &LearningItem) -> Uuid {
    match &item.item {
        LearningItemType::Mistake(m) => m.id,
        LearningItemType::Explanation(e) => e.id,
        LearningItemType::Translation(t) => t.id,
        LearningItemType::Exploration(e) => e.id,
    }
}

fn get_tooltip(item: &LearningItem) -> Option<String> {
    match &item.item {
        LearningItemType::Mistake(m) => Some(m.mistake_category.to_string()),
        LearningItemType::Explanation(e) => Some(e.explanation.clone()),
        LearningItemType::Translation(t) => {
            if let Some(context) = &t.context {
                Some(format!(
                    "Translation: {}\nContext: {}",
                    t.translated_to, context
                ))
            } else {
                Some(format!("Translation: {}", t.translated_to))
            }
        }
        LearningItemType::Exploration(e) => Some(e.instructions_for_use.clone()),
    }
}

fn get_item_parts(item: &LearningItem) -> (String, String, &'static str) {
    match &item.item {
        LearningItemType::Mistake(m) => (
            m.correction.clone(),
            format!("Instead of: {}", m.specific_mistake),
            "🛠️",
        ),
        LearningItemType::Explanation(e) => (e.new_phrase.clone(), e.explanation.clone(), "💡"),
        LearningItemType::Translation(t) => {
            (t.translated_to.clone(), t.translated_word.clone(), "🌐")
        }
        LearningItemType::Exploration(e) => {
            (e.point_to_try.clone(), e.instructions_for_use.clone(), "🎯")
        }
    }
}

fn get_accent_color(item: &LearningItemType) -> &'static str {
    match item {
        LearningItemType::Mistake(_) => "var(--coral)",
        LearningItemType::Explanation(_) => "var(--teal)",
        LearningItemType::Translation(_) => "var(--yellow)",
        LearningItemType::Exploration(_) => "var(--green)",
    }
}

pub fn render_learning_item(item: &LearningItem, on_delete: Callback<Uuid>) -> Html {
    let (title, subtitle, icon) = get_item_parts(item);
    let tooltip = get_tooltip(item);
    let accent_color = get_accent_color(&item.item);
    let item_id = get_learning_item_id(item);
    let score_pct = item.score;

    // Subtle glass card style
    html! {
        <li class="learning-card" style={format!("--accent-color: {}", accent_color)} title={tooltip}>
            <div class="card-icon">{icon}</div>
            <div class="card-content">
                <div class="card-title">{title}</div>
                <div class="card-subtitle">{subtitle}</div>
            </div>
            <div class="card-meta">
                <div class="score-ring" style={format!("--score: {}%", score_pct)}>
                     // Visual ring or text handled by CSS/SVG, or just simple text for now
                    <span class="score-text">{format!("{}%", score_pct)}</span>
                </div>
                <button
                    class="card-delete-button"
                    onclick={on_delete.reform(move |_| item_id)}
                >
                    {"×"}
                </button>
            </div>
            <div class="card-progress-line">
                <div class="progress-fill" style={format!("width: {}%; background: {}", score_pct, accent_color)}></div>
            </div>
        </li>
    }
}
