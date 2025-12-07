use dialect_coach_shared::LearningItem;
use dialect_coach_shared::LearningItemType;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct VocabHudProps {
    pub items: Vec<LearningItem>,
}

fn get_item_label(item: &LearningItem) -> String {
    match &item.item {
        LearningItemType::Mistake(m) => m.specific_mistake.clone(),
        LearningItemType::Explanation(e) => e.new_phrase.clone(),
        LearningItemType::Translation(t) => t.translated_word.clone(),
        LearningItemType::Exploration(e) => e.point_to_try.clone(),
    }
}

fn get_color_class(item: &LearningItem) -> &'static str {
    match &item.item {
        LearningItemType::Mistake(_) => "vocab-chip--mistake",
        LearningItemType::Explanation(_) => "vocab-chip--explanation",
        LearningItemType::Translation(_) => "vocab-chip--translation",
        LearningItemType::Exploration(_) => "vocab-chip--exploration",
    }
}

#[function_component(VocabHud)]
pub fn vocab_hud(props: &VocabHudProps) -> Html {
    if props.items.is_empty() {
        return html! {};
    }

    html! {
        <div class="vocab-hud">
            <div class="vocab-hud-scroll">
                {for props.items.iter().take(10).map(|item| {
                    let label = get_item_label(item);
                    let color_class = get_color_class(item);
                    html! {
                        <div class={classes!("vocab-chip", color_class)} title={label.clone()}>
                            <span class="vocab-chip-text">{label}</span>
                            <div class="vocab-chip-progress" style={format!("width: {}%", item.score)}></div>
                        </div>
                    }
                })}
            </div>
        </div>
    }
}
