use dialect_coach_shared::models::{LearningGoal, LearningItem, LearningItemType};
use uuid::Uuid;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct DynamicIslandProps {
    pub learning_goal: Option<LearningGoal>,
    pub items: Vec<LearningItem>,
}

#[derive(PartialEq, Clone)]
enum ViewState {
    Plan,
    Items(Vec<Uuid>), // UUIDs of items to show
}

#[function_component(DynamicIsland)]
pub fn dynamic_island(props: &DynamicIslandProps) -> Html {
    let state = use_state(|| ViewState::Plan);
    
    // --- Interaction Logic (Basic Reactive Wiring) ---
    // The handler does ONE thing: transition the state.
    // Logic is consolidated here. No splitting state updates.
    let on_click_container = {
        let state = state.clone();
        let items = props.items.clone();

        Callback::from(move |_: MouseEvent| {
            let new_state = match (*state).clone() {
                ViewState::Plan => {
                    let count = 3.min(items.len());
                    let uuids = pick_random_uuids(&items, count, &[]);
                    ViewState::Items(uuids)
                },
                ViewState::Items(_) => {
                    ViewState::Plan
                }
            };
            state.set(new_state);
        })
    };
    
    // --- Render Logic ---
    let content = match &*state {
        ViewState::Plan => {
            if let Some(goal) = &props.learning_goal {
                render_plan(goal)
            } else {
                // Fallback: If no plan, show items (calculate fresh random indices if needed for empty fallback)
                // But generally better to transition state. For now, simple fallback render.
                // We'll just render "Status" empty state if plan logic fails render.
                if !props.items.is_empty() {
                     // Since we don't have indices in Plan mode, we might need a default set?
                     // Or force state transition on mount?
                     // Simpler: Just render "Status" if no goal.
                     render_empty_status()
                } else {
                     render_empty_status()
                }
            }
        },
        ViewState::Items(uuids) => {
            if props.items.is_empty() {
                render_empty_status()
            } else {
                render_items(&props.items, uuids, state.clone())
            }
        }
    };

    html! {
        <div class="dynamic-island-container">
            <div 
                class="dynamic-island" 
                onclick={on_click_container}
                title="Click to toggle view"
            >
                {content}
            </div>
        </div>
    }
}

// --- Render Helpers ---

fn render_plan(goal: &LearningGoal) -> Html {
    html! {
        <div class="island-content">
            <span class="island-label">{"Current Goal"}</span>
            <div class="island-plan-step">
                <span class="check-icon">{"🎯"}</span>
                <span class="island-text large">{&goal.goal}</span>
            </div>
        </div>
    }
}

fn render_items(
    all_items: &[LearningItem],
    uuids: &[Uuid],
    state: UseStateHandle<ViewState>,
) -> Html {
    html! {
        <div class="island-list">
            <div class="island-header">
                <span class="island-label">{"Review"}</span>
            </div>
            {for uuids.iter().filter_map(|uuid| {
                find_item_by_uuid(all_items, *uuid)
                    .map(|item| render_single_item(item, state.clone(), all_items))
            })}
        </div>
    }
}

fn make_item_swap_callback(
    id: Uuid,
    state: UseStateHandle<ViewState>,
    all_items: Vec<LearningItem>,
) -> Callback<MouseEvent> {
    Callback::from(move |e: MouseEvent| {
        e.stop_propagation();
        let new_state = match (*state).clone() {
            ViewState::Items(current) => {
                ViewState::Items(swap_one_uuid(&current, id, &all_items))
            },
            other => other,
        };
        state.set(new_state);
    })
}

fn render_single_item(
    item: &LearningItem,
    state: UseStateHandle<ViewState>,
    all_items: &[LearningItem],
) -> Html {
    let id = get_item_id(item);
    let onclick = make_item_swap_callback(id, state, all_items.to_vec());

    html! {
        <div class="island-item" {onclick} title="Click to swap">
            <span class="island-text">{get_item_text(item)}</span>
        </div>
    }
}

fn render_empty_status() -> Html {
    html! {
        <div class="island-content">
            <span class="island-label">{"Status"}</span>
            <span class="island-text">{"Ready for conversation"}</span>
        </div>
    }
}

// --- Utilities ---

fn pick_random_uuids(
    items: &[LearningItem],
    count: usize,
    exclude: &[Uuid],
) -> Vec<Uuid> {
    let mut available: Vec<Uuid> = items
        .iter()
        .map(get_item_id)
        .filter(|uuid| !exclude.contains(uuid))
        .collect();
    let mut selected = Vec::new();
    for _ in 0..count {
        if available.is_empty() {
            break;
        }
        let r = (js_sys::Math::random() * available.len() as f64) as usize;
        selected.push(available.remove(r));
    }
    selected
}

fn find_item_by_uuid(items: &[LearningItem], uuid: Uuid) -> Option<&LearningItem> {
    items.iter().find(|item| get_item_id(item) == uuid)
}

fn swap_one_uuid(
    current: &[Uuid],
    to_replace: Uuid,
    all_items: &[LearningItem]
) -> Vec<Uuid> {
    let new_uuids = pick_random_uuids(all_items, 1, current);

    if new_uuids.is_empty() {
        return current.to_vec();
    }

    current.iter().map(|&id| {
        if id == to_replace {
            new_uuids[0]
        } else {
            id
        }
    }).collect()
}

fn get_item_text(item: &LearningItem) -> String {
    match &item.item {
        LearningItemType::Mistake(m) => m.specific_mistake.clone(),
        LearningItemType::Explanation(e) => e.new_phrase.clone(),
        LearningItemType::Translation(t) => format!("{} → {}", t.translated_word, t.translated_to),
        LearningItemType::Exploration(e) => e.point_to_try.clone(),
    }
}

fn get_item_id(item: &LearningItem) -> Uuid {
    match &item.item {
        LearningItemType::Mistake(m) => m.id,
        LearningItemType::Explanation(e) => e.id,
        LearningItemType::Translation(t) => t.id,
        LearningItemType::Exploration(e) => e.id,
    }
}
