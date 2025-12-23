use crate::components::utility_sidebar::learning_item::{LearningItemState, get_accent_color};
use dialect_coach_shared::models::{LearningGoal, LearningItem, LearningItemType, Quest};
use uuid::Uuid;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct DynamicIslandProps {
    pub current_step_title: Option<String>,
    pub items: Vec<LearningItem>,
    #[prop_or_default]
    pub learning_goals: Vec<LearningGoal>,
    #[prop_or_default]
    pub quests: Vec<Quest>,
}

#[derive(PartialEq, Clone)]
enum ViewState {
    Plan,
    Goal,
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
        let has_plan = props.current_step_title.is_some();
        let has_goals = !props.learning_goals.is_empty();
        let has_items = !props.items.is_empty();

        Callback::from(move |_: MouseEvent| {
            let available = get_available_modes(has_plan, has_goals, has_items);

            if available.is_empty() {
                // No modes available, stay in current state
                return;
            }

            let current = (*state).clone();
            let current_mode = mode_from_view_state(&current);

            if let Some(next_mode) = get_next_mode(current_mode, &available) {
                let new_state = match next_mode {
                    Mode::Plan => ViewState::Plan,
                    Mode::Goal => ViewState::Goal,
                    Mode::Items => {
                        let count = 3.min(items.len());
                        let uuids = pick_random_uuids(&items, count, &[]);
                        ViewState::Items(uuids)
                    }
                };
                state.set(new_state);
            }
        })
    };

    // --- Render Logic ---
    // Collect IDs of items shown as uncompleted quests (to avoid showing duplicates)
    let quest_item_ids: Vec<_> = props
        .quests
        .iter()
        .filter(|q| !q.completed)
        .filter_map(|q| {
            q.id.strip_prefix("quest_")
                .and_then(|s| uuid::Uuid::parse_str(s).ok())
        })
        .collect();

    let has_plan = props.current_step_title.is_some();
    let has_goals = !props.learning_goals.is_empty();
    // Items available for display = items not in uncompleted quests
    let available_items: Vec<_> = props
        .items
        .iter()
        .filter(|item| !quest_item_ids.contains(&item.id()))
        .cloned()
        .collect();
    let has_items = !available_items.is_empty();
    let available_modes = get_available_modes(has_plan, has_goals, has_items);

    let content = if available_modes.is_empty() {
        // No content available - show guidance
        render_guidance_message()
    } else {
        match &*state {
            ViewState::Plan => {
                if let Some(step_title) = &props.current_step_title {
                    render_plan(step_title)
                } else {
                    render_guidance_message()
                }
            }
            ViewState::Goal => {
                if !props.learning_goals.is_empty() {
                    render_goal(&props.learning_goals)
                } else {
                    render_guidance_message()
                }
            }
            ViewState::Items(uuids) => {
                if available_items.is_empty() {
                    render_guidance_message()
                } else {
                    render_items(&available_items, uuids, state.clone())
                }
            }
        }
    };

    // Render quests above the main content if any undone quests exist
    let quests_section = render_quests(&props.quests);

    html! {
        <div class="dynamic-island-container">
            <div
                class="dynamic-island"
                onclick={on_click_container}
                title="Click to toggle view"
            >
                {quests_section}
                {content}
            </div>
        </div>
    }
}

// --- Render Helpers ---

fn render_plan(step_title: &str) -> Html {
    html! {
        <div class="island-content">
            <span class="island-label">{"Current Learning Plan Step"}</span>
            <div class="island-plan-step">
                <span class="check-icon">{"🎯"}</span>
                <span class="island-text large">{step_title}</span>
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
            ViewState::Items(current) => ViewState::Items(swap_one_uuid(&current, id, &all_items)),
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
    let item_state = LearningItemState::from_score(item.score);
    let accent_color = get_accent_color(&item.item);

    html! {
        <div class={classes!("island-item", item_state.css_class())} style={format!("--accent-color: {}", accent_color)} {onclick} title={format!("Click to swap • {} ({}%)", item_state.label(), item.score)}>
            <span class="state-indicator">{item_state.icon()}</span>
            <span class="island-text">{get_item_text(item)}</span>
            <span class="island-score">{format!("{}%", item.score)}</span>
        </div>
    }
}

fn render_quests(quests: &[Quest]) -> Html {
    let undone: Vec<_> = quests.iter().filter(|q| !q.completed).collect();
    if undone.is_empty() {
        return html! {};
    }

    html! {
        <div class="island-quests">
            <span class="island-label">{"Daily Focus"}</span>
            {for undone.iter().map(|quest| {
                html! {
                    <div class="island-quest-item">
                        <span class="quest-icon">{"🎯"}</span>
                        <span class="island-text">{&quest.description}</span>
                    </div>
                }
            })}
        </div>
    }
}

fn render_goal(goals: &[LearningGoal]) -> Html {
    if let Some(goal) = goals.first() {
        html! {
            <div class="island-content">
                <span class="island-label">{"Learning Goal"}</span>
                <div class="island-goal">
                    <span class="goal-icon">{"🎯"}</span>
                    <span class="island-text large">{&goal.goal}</span>
                </div>
            </div>
        }
    } else {
        html! {}
    }
}

fn render_guidance_message() -> Html {
    html! {
        <div class="island-content island-guidance">
            <span class="island-label">{"Get Started"}</span>
            <span class="island-text">{"Set goals and create plans to track progress. Start chatting to build learning items."}</span>
        </div>
    }
}

// --- Mode Cycling Utilities ---

#[derive(Clone, Copy, PartialEq, Debug)]
enum Mode {
    Plan,
    Goal,
    Items,
}

fn get_available_modes(has_plan: bool, has_goals: bool, has_items: bool) -> Vec<Mode> {
    let mut modes = Vec::new();
    if has_plan {
        modes.push(Mode::Plan);
    }
    if has_goals {
        modes.push(Mode::Goal);
    }
    if has_items {
        modes.push(Mode::Items);
    }
    modes
}

fn get_next_mode(current: Mode, available: &[Mode]) -> Option<Mode> {
    if available.is_empty() {
        return None;
    }

    // Find current mode's position in available modes
    let current_idx = available.iter().position(|m| *m == current);

    match current_idx {
        Some(idx) => {
            // Cycle to next available mode
            let next_idx = (idx + 1) % available.len();
            Some(available[next_idx])
        }
        None => {
            // Current mode not in available modes, return first available
            available.first().copied()
        }
    }
}

fn mode_from_view_state(state: &ViewState) -> Mode {
    match state {
        ViewState::Plan => Mode::Plan,
        ViewState::Goal => Mode::Goal,
        ViewState::Items(_) => Mode::Items,
    }
}

// --- Other Utilities ---

fn pick_random_uuids(items: &[LearningItem], count: usize, exclude: &[Uuid]) -> Vec<Uuid> {
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

fn swap_one_uuid(current: &[Uuid], to_replace: Uuid, all_items: &[LearningItem]) -> Vec<Uuid> {
    let new_uuids = pick_random_uuids(all_items, 1, current);

    if new_uuids.is_empty() {
        return current.to_vec();
    }

    current
        .iter()
        .map(|&id| if id == to_replace { new_uuids[0] } else { id })
        .collect()
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
