use crate::components::ruby_text::render_text_with_ruby;
use crate::components::utility_sidebar::learning_item::{LearningItemState, get_accent_color};
use dialect_coach_shared::models::{LearningGoal, LearningItem, LearningItemType, Quest};
use std::rc::Rc;
use std::sync::Arc;
use uuid::Uuid;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct DynamicIslandProps {
    pub current_step_title: Option<String>,
    pub items: Rc<Vec<LearningItem>>,
    #[prop_or_default]
    pub learning_goals: Arc<Vec<LearningGoal>>,
    #[prop_or_default]
    pub quests: Vec<Quest>,
    pub on_open_review_list: Callback<()>,
}

#[derive(PartialEq, Clone)]
enum ViewState {
    Plan,
    Goal,
    Items(Vec<Uuid>), // UUIDs of items to show
}

fn quest_item_ids(quests: &[Quest]) -> Vec<Uuid> {
    quests
        .iter()
        .filter(|q| !q.completed)
        .filter_map(|q| {
            q.id.strip_prefix("quest_")
                .and_then(|id| Uuid::parse_str(id).ok())
        })
        .collect()
}

fn available_items(items: &[LearningItem], quest_ids: &[Uuid]) -> Vec<LearningItem> {
    items
        .iter()
        .filter(|item| !quest_ids.contains(&item.id()))
        .cloned()
        .collect()
}

fn effective_mode(state: &ViewState, available: &[Mode]) -> Option<Mode> {
    let selected = mode_from_view_state(state);
    if available.contains(&selected) {
        Some(selected)
    } else {
        available.first().copied()
    }
}

fn view_state_for_mode(mode: Mode, items: &[LearningItem]) -> ViewState {
    match mode {
        Mode::Plan => ViewState::Plan,
        Mode::Goal => ViewState::Goal,
        Mode::Items => ViewState::Items(pick_random_uuids(items, 3.min(items.len()), &[])),
    }
}

#[hook]
fn use_cycle_click(
    state: UseStateHandle<ViewState>,
    modes: Vec<Mode>,
    items: Vec<LearningItem>,
) -> Callback<MouseEvent> {
    Callback::from(move |_| {
        if let Some(current) = effective_mode(&state, &modes)
            && let Some(next) = get_next_mode(current, &modes)
        {
            state.set(view_state_for_mode(next, &items));
        }
    })
}

fn render_summary(
    count: usize,
    on_open: Callback<()>,
    expanded: bool,
    on_toggle: Callback<MouseEvent>,
) -> Html {
    html! {
        <div class="island-summary">
            <button type="button" class="island-summary-toggle" onclick={on_toggle}
                aria-expanded={expanded.to_string()} aria-controls="todays-focus-content">
                <span aria-hidden="true">{if expanded { "▾" } else { "▸" }}</span>
                <span class="island-label">{"Today's focus"}</span>
            </button>
            if count > 0 {
                <button type="button" class="island-summary-count" onclick={on_open.reform(|_| ())}>
                    {format!("{count} to review")}
                </button>
            }
        </div>
    }
}

fn render_review_items(items: &[LearningItem], state: &UseStateHandle<ViewState>) -> Html {
    let uuids = match &**state {
        ViewState::Items(ids) => ids.clone(),
        _ => pick_random_uuids(items, 3.min(items.len()), &[]),
    };
    render_items(items, &uuids, state.clone())
}

fn render_mode_content(
    props: &DynamicIslandProps,
    state: &UseStateHandle<ViewState>,
    items: &[LearningItem],
    mode: Option<Mode>,
) -> Html {
    match mode {
        None => render_guidance_message(),
        Some(Mode::Plan) => render_plan(props.current_step_title.as_ref().unwrap()),
        Some(Mode::Goal) => render_goal(&props.learning_goals),
        Some(Mode::Items) => render_review_items(items, state),
    }
}

fn render_island_body(
    props: &DynamicIslandProps,
    state: &UseStateHandle<ViewState>,
    items: &[LearningItem],
    mode: Option<Mode>,
    onclick: Callback<MouseEvent>,
) -> Html {
    html! {
        <div id="todays-focus-content" class="dynamic-island" {onclick} title="Click to toggle view">
            {render_quests(&props.quests)}
            {render_mode_content(props, state, items, mode)}
        </div>
    }
}

#[function_component(DynamicIsland)]
pub fn dynamic_island(props: &DynamicIslandProps) -> Html {
    let state = use_state(|| ViewState::Plan);
    let expanded = use_state(|| false);
    let on_toggle = {
        let expanded = expanded.clone();
        Callback::from(move |_| expanded.set(!*expanded))
    };
    let items = available_items(&props.items, &quest_item_ids(&props.quests));
    let modes = get_available_modes(
        props.current_step_title.is_some(),
        !props.learning_goals.is_empty(),
        !items.is_empty(),
    );
    let onclick = use_cycle_click(state.clone(), modes.clone(), items.clone());
    // Mobile reserves only the summary row until details are requested.
    html! {
        <div class={classes!("dynamic-island-container", (*expanded).then_some("is-expanded"))}>
            {render_summary(items.len(), props.on_open_review_list.clone(), *expanded, on_toggle)}
            {render_island_body(props, &state, &items, effective_mode(&state, &modes), onclick)}
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
                <span class="island-text large">{render_text_with_ruby(step_title)}</span>
            </div>
        </div>
    }
}

/// Review rows kept visible at <=900px. Matches the `.island-item` cutoff in
/// dynamic_island.css; the rest are hidden and counted by `.island-more`.
const MOBILE_REVIEW_LIMIT: usize = 1;
const MOBILE_QUEST_LIMIT: usize = 2;

fn review_rows(
    all_items: &[LearningItem],
    uuids: &[Uuid],
    state: UseStateHandle<ViewState>,
) -> Vec<Html> {
    uuids
        .iter()
        .filter_map(|uuid| {
            find_item_by_uuid(all_items, *uuid)
                .map(|item| render_single_item(item, state.clone(), all_items))
        })
        .collect()
}

fn render_items(
    all_items: &[LearningItem],
    uuids: &[Uuid],
    state: UseStateHandle<ViewState>,
) -> Html {
    let shown = review_rows(all_items, uuids, state);
    // Phones show one review row; the remaining rendered rows are counted below.
    let hidden = shown.len().saturating_sub(MOBILE_REVIEW_LIMIT);
    html! {
        <div class="island-list">
            <div class="island-header">
                <span class="island-label">{"Review"}</span>
                if hidden > 0 { <span class="island-more">{format!("+{hidden} more")}</span> }
            </div>
            {shown}
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
            <span class="island-text">{render_text_with_ruby(&get_item_text(item))}</span>
            <span class="island-score">{format!("{}%", item.score)}</span>
        </div>
    }
}

fn render_quest(quest: &Quest) -> Html {
    html! {
        <div class="island-quest-item">
            <span class="quest-icon">{"🎯"}</span>
            <span class="island-text">{render_text_with_ruby(&quest.description)}</span>
        </div>
    }
}

fn render_quests(quests: &[Quest]) -> Html {
    let undone: Vec<_> = quests.iter().filter(|q| !q.completed).collect();
    if undone.is_empty() {
        return html! {};
    }
    let hidden = undone.len().saturating_sub(MOBILE_QUEST_LIMIT);
    html! {
        <div class="island-quests">
            <div class="island-header">
                <span class="island-label">{"Daily Focus"}</span>
                if hidden > 0 { <span class="island-more">{format!("+{hidden} more")}</span> }
            </div>
            {for undone.iter().map(|quest| render_quest(quest))}
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
                    <span class="island-text large">{render_text_with_ruby(&goal.goal)}</span>
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_fixtures::{learning_item, quest};
    use dialect_coach_shared::Dialect;

    #[test]
    fn incomplete_quests_supply_valid_item_ids_only() {
        let id = Uuid::new_v4();
        let pending = quest(id, Dialect::ArabicEgyptian, "Practice a greeting");
        let mut complete = quest(Uuid::new_v4(), pending.dialect, "Completed");
        complete.completed = true;
        let mut invalid = pending.clone();
        invalid.id = "quest_invalid".into();
        let mut other = pending.clone();
        other.id = format!("other_{id}");
        assert_eq!(
            quest_item_ids(&[pending, complete, invalid, other]),
            vec![id]
        );
    }

    #[test]
    fn review_items_exclude_daily_targets_and_keep_full_content_in_order() {
        let first = learning_item("أهلا", Dialect::ArabicEgyptian, 37);
        let focus = learning_item("بالراحة", Dialect::ArabicEgyptian, 12);
        let last = learning_item("واحدة واحدة", Dialect::ArabicEgyptian, 89);
        let items = vec![first.clone(), focus.clone(), last.clone()];
        assert_eq!(available_items(&items, &[focus.id()]), vec![first, last]);
        assert_eq!(available_items(&items, &[]), items);
    }

    #[test]
    fn no_review_items_remain_when_all_are_daily_targets() {
        let items = [learning_item("أهلا", Dialect::ArabicEgyptian, 0)];
        assert!(available_items(&items, &[items[0].id()]).is_empty());
        assert!(available_items(&[], &[]).is_empty());
    }

    #[test]
    fn selected_view_stays_selected_while_available() {
        let modes = vec![Mode::Plan, Mode::Goal, Mode::Items];
        assert_eq!(effective_mode(&ViewState::Goal, &modes), Some(Mode::Goal));
        assert_eq!(
            effective_mode(&ViewState::Items(vec![]), &modes),
            Some(Mode::Items)
        );
    }

    #[test]
    fn unavailable_view_falls_back_in_plan_goal_items_order() {
        let modes = get_available_modes(false, true, true);
        assert_eq!(effective_mode(&ViewState::Plan, &modes), Some(Mode::Goal));
        let modes = get_available_modes(false, false, true);
        assert_eq!(effective_mode(&ViewState::Goal, &modes), Some(Mode::Items));
        assert_eq!(
            effective_mode(&ViewState::Items(vec![]), &[Mode::Plan]),
            Some(Mode::Plan)
        );
        assert_eq!(effective_mode(&ViewState::Plan, &[]), None);
    }

    #[test]
    fn clicking_cycles_available_views_and_wraps() {
        let modes = get_available_modes(true, true, true);
        assert_eq!(get_next_mode(Mode::Plan, &modes), Some(Mode::Goal));
        assert_eq!(get_next_mode(Mode::Goal, &modes), Some(Mode::Items));
        assert_eq!(get_next_mode(Mode::Items, &modes), Some(Mode::Plan));
        assert_eq!(
            get_next_mode(Mode::Items, &[Mode::Items]),
            Some(Mode::Items)
        );
        assert_eq!(get_next_mode(Mode::Plan, &[]), None);
    }
}
