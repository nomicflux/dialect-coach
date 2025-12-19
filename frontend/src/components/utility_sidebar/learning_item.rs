use dialect_coach_shared::{
    LearningItem, LearningItemType,
};
use uuid::Uuid;
use yew::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LearningItemState {
    New,        // 0
    Started,    // >0
    Activated,  // >20
    Competent,  // >50
    Mastered,   // >80
    Perfected,  // 100
}

impl LearningItemState {
    pub fn from_score(score: u8) -> Self {
        match score {
            100..=u8::MAX => Self::Perfected,
            81..=99 => Self::Mastered,
            51..=80 => Self::Competent,
            21..=50 => Self::Activated,
            1..=20 => Self::Started,
            0 => Self::New,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::New => "New",
            Self::Started => "Started",
            Self::Activated => "Activated",
            Self::Competent => "Competent",
            Self::Mastered => "Mastered",
            Self::Perfected => "Perfected",
        }
    }

    pub fn css_class(&self) -> &'static str {
        match self {
            Self::New => "state-new",
            Self::Started => "state-started",
            Self::Activated => "state-activated",
            Self::Competent => "state-competent",
            Self::Mastered => "state-mastered",
            Self::Perfected => "state-perfected",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::New => "◯",
            Self::Started => "◔",
            Self::Activated => "◑",
            Self::Competent => "◕",
            Self::Mastered => "⬤",
            Self::Perfected => "★",
        }
    }
}

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
    let state = LearningItemState::from_score(item.score);

    // Subtle glass card style
    html! {
        <li class={classes!("learning-card", state.css_class())} style={format!("--accent-color: {}", accent_color)} title={tooltip}>
            <div class="card-icon">{icon}</div>
            <div class="card-content">
                <div class="card-title">{title}</div>
                <div class="card-subtitle">{subtitle}</div>
            </div>
            <div class="card-meta">
                <div class="score-ring" style={format!("--score: {}%", score_pct)}>
                    <span class="score-text">{format!("{}%", score_pct)}</span>
                </div>
                <div class="state-badge" title={format!("{} ({}%)", state.label(), score_pct)}>
                    <span class="state-icon">{state.icon()}</span>
                    <span class="state-label">{state.label()}</span>
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
