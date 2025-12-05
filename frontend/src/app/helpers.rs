use super::app_state::OptionalUserState;
use dialect_coach_shared::{Dialect, Explained, Exploratory, LearningItemType, Mistake, Translated};
use yew::prelude::*;

pub fn extract_learning_items(
    user_state: &OptionalUserState,
    dialect: &Dialect,
) -> (
    Vec<Mistake>,
    Vec<Explained>,
    Vec<Translated>,
    Vec<Exploratory>,
) {
    let mut mistakes = Vec::new();
    let mut explained = Vec::new();
    let mut translated = Vec::new();
    let mut exploratory = Vec::new();

    if let Some(state) = &user_state.state {
        // Filter learning items by dialect to only send relevant items to agents
        for item in state.get_learning_items_for_dialect(dialect) {
            match &item.item {
                LearningItemType::Mistake(m) => mistakes.push(m.clone()),
                LearningItemType::Explanation(e) => explained.push(e.clone()),
                LearningItemType::Translation(t) => translated.push(t.clone()),
                LearningItemType::Exploration(e) => exploratory.push(e.clone()),
            }
        }
    }

    (mistakes, explained, translated, exploratory)
}

pub fn render_message_undo_notification(deleted_count: usize, on_undo: Callback<()>) -> Html {
    if deleted_count > 0 {
        let onclick = Callback::from(move |_| on_undo.emit(()));
        html! {
            <div class="message-undo-notification">
                <span>{"Message deleted."}</span>
                <button class="undo-button" {onclick}>{"Undo"}</button>
            </div>
        }
    } else {
        html! {}
    }
}
