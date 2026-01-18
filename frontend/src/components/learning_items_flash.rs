use dialect_coach_shared::{LearningItem, LearningItemType};
use uuid::Uuid;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct LearningItemsFlashProps {
    pub items: Vec<LearningItem>,
    pub on_dismiss: Callback<Uuid>,
}

#[function_component(LearningItemsFlash)]
pub fn learning_items_flash(props: &LearningItemsFlashProps) -> Html {
    if props.items.is_empty() {
        return html! {};
    }

    html! {
        <div class="learning-items-flash">
            {for props.items.iter().map(|item| {
                render_flash_item(item, props.on_dismiss.clone())
            })}
        </div>
    }
}

fn render_flash_item(item: &LearningItem, on_dismiss: Callback<Uuid>) -> Html {
    let id = item.id();
    let onclick = Callback::from(move |_| on_dismiss.emit(id));
    let (icon, text) = get_item_display(item);

    html! {
        <div class="flash-item" {onclick} title="Click to dismiss">
            <span class="flash-item-icon">{icon}</span>
            <span class="flash-item-text">{text}</span>
        </div>
    }
}

fn get_item_display(item: &LearningItem) -> (&'static str, String) {
    match &item.item {
        LearningItemType::Mistake(m) => ("🛠️", m.specific_mistake.clone()),
        LearningItemType::Explanation(e) => ("💡", e.new_phrase.clone()),
        LearningItemType::Translation(t) => {
            ("🌐", format!("{} → {}", t.translated_word, t.translated_to))
        }
        LearningItemType::Exploration(x) => ("🎯", x.point_to_try.clone()),
    }
}
