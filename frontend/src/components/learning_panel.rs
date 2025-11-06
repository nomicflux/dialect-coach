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
    pub is_collapsed: bool,
    pub on_close: Callback<()>,
    pub on_toggle: Callback<()>,
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

fn count_by_type(items: &[LearningItem]) -> (usize, usize, usize, usize) {
    let mut mistakes = 0;
    let mut explanations = 0;
    let mut translations = 0;
    let mut explorations = 0;

    for item in items {
        match &item.item {
            LearningItemType::Mistake(_) => mistakes += 1,
            LearningItemType::Explanation(_) => explanations += 1,
            LearningItemType::Translation(_) => translations += 1,
            LearningItemType::Exploration(_) => explorations += 1,
        }
    }

    (mistakes, explanations, translations, explorations)
}

fn render_collapsed_type_indicator(
    type_name: &str,
    class: String,
    count: usize,
    items: &[LearningItem],
) -> Html {
    let items_of_type: Vec<String> = items
        .iter()
        .filter(|item| match (type_name, &item.item) {
            ("mistake", LearningItemType::Mistake(_)) => true,
            ("explanation", LearningItemType::Explanation(_)) => true,
            ("translation", LearningItemType::Translation(_)) => true,
            ("exploration", LearningItemType::Exploration(_)) => true,
            _ => false,
        })
        .map(|item| get_item_content(item))
        .collect();

    let tooltip_text = if items_of_type.is_empty() {
        format!("{} items", count)
    } else {
        items_of_type.join("\n")
    };

    // Use abbreviated labels that fit in the collapsed panel
    let label = match type_name {
        "mistake" => "MST",
        "explanation" => "EXP",
        "translation" => "TRN",
        "exploration" => "EXR", // Use EXR to distinguish from EXP (explanations)
        _ => "",
    };

    html! {
        <div
            class={format!("learning-type-count-box {}", class.clone())}
            title={tooltip_text.clone()}
        >
            <div class="learning-type-indicator-tooltip">
                {tooltip_text}
            </div>
            <span class="learning-type-label">{label}</span>
            <span class="learning-type-number">{count}</span>
        </div>
    }
}

fn render_collapsed_view(props: &LearningPanelProps) -> Html {
    let (mistakes, explanations, translations, explorations) = count_by_type(&props.items);
    let total = props.items.len();

    html! {
        <>
            <div class="learning-panel-header">
            </div>
            <button
                class="learning-panel-toggle-button"
                onclick={Callback::from({
                    let on_toggle = props.on_toggle.clone();
                    move |_| on_toggle.emit(())
                })}
                title="Expand learning panel"
            >
                {"◄"}
            </button>
            <div class="learning-type-indicators">
                {if mistakes > 0 {
                    html! {
                        {render_collapsed_type_indicator(
                            "mistake",
                            "learning-type-count-box--mistake".to_string(),
                            mistakes,
                            &props.items
                        )}
                    }
                } else {
                    html! {}
                }}
                {if explanations > 0 {
                    html! {
                        {render_collapsed_type_indicator(
                            "explanation",
                            "learning-type-count-box--explanation".to_string(),
                            explanations,
                            &props.items
                        )}
                    }
                } else {
                    html! {}
                }}
                {if translations > 0 {
                    html! {
                        {render_collapsed_type_indicator(
                            "translation",
                            "learning-type-count-box--translation".to_string(),
                            translations,
                            &props.items
                        )}
                    }
                } else {
                    html! {}
                }}
                {if explorations > 0 {
                    html! {
                        {render_collapsed_type_indicator(
                            "exploration",
                            "learning-type-count-box--exploration".to_string(),
                            explorations,
                            &props.items
                        )}
                    }
                } else {
                    html! {}
                }}
            </div>
            <div class="learning-total-count">
                <span class="learning-total-label">{"Total"}</span>
                <span class="learning-total-number">{total}</span>
            </div>
        </>
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

fn render_expanded_view(props: &LearningPanelProps) -> Html {
    let (accomplishments, still_learning): (Vec<_>, Vec<_>) =
        props.items.iter().partition(|item| item.score == 100);

    html! {
        <>
            <div class="learning-panel-header">
                <h3>{"Learning Progress"}</h3>
            </div>
            <button
                class="learning-panel-toggle-button"
                onclick={Callback::from({
                    let on_toggle = props.on_toggle.clone();
                    move |_| on_toggle.emit(())
                })}
                title="Collapse learning panel"
            >
                {"►"}
            </button>
            <div class="learning-panel-content">
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
        </>
    }
}

#[function_component(LearningPanel)]
pub fn learning_panel(props: &LearningPanelProps) -> Html {
    let panel_class = if props.is_collapsed {
        "learning-panel learning-panel--collapsed"
    } else {
        "learning-panel"
    };

    html! {
        <div class={panel_class}>
            {if props.is_collapsed {
                render_collapsed_view(props)
            } else {
                render_expanded_view(props)
            }}
        </div>
    }
}
