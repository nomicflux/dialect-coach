use super::actions::{DeletedMessage, DeletedMessageLocation};
use dialect_coach_shared::models::gamification::{GamificationStats, derive_gamification_stats};
use dialect_coach_shared::models::{ConversationBranch, Dialect, Formality, Message, TeachingMode};
use dialect_coach_shared::{
    AgentAnalysis, Explained, Exploratory, LearningGoal, LearningItem, LearningItemType, Mistake,
    Translated, UserState,
};
use std::sync::Arc;
use uuid::Uuid;

pub trait UserStateGamificationExt {
    fn gamification_stats(&self) -> GamificationStats;
}

impl UserStateGamificationExt for UserState {
    fn gamification_stats(&self) -> GamificationStats {
        derive_gamification_stats(self)
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

pub fn add_learning_items_to_vec(
    mut items: Vec<LearningItem>,
    mistakes: Vec<(Mistake, u8)>,
    explained: Vec<(Explained, u8)>,
    translated: Vec<(Translated, u8)>,
    exploratory: Vec<(Exploratory, u8)>,
    dialect: Dialect,
) -> Vec<LearningItem> {
    for (mistake, score) in mistakes {
        push_unique(
            &mut items,
            LearningItem::with_score(LearningItemType::Mistake(mistake), dialect, score),
        );
    }
    for (expl, score) in explained {
        push_unique(
            &mut items,
            LearningItem::with_score(LearningItemType::Explanation(expl), dialect, score),
        );
    }
    for (trans, score) in translated {
        push_unique(
            &mut items,
            LearningItem::with_score(LearningItemType::Translation(trans), dialect, score),
        );
    }
    for (explor, score) in exploratory {
        push_unique(
            &mut items,
            LearningItem::with_score(LearningItemType::Exploration(explor), dialect, score),
        );
    }
    items
}

pub fn merge_learning_items(
    mut existing: Vec<LearningItem>,
    new_items: Vec<LearningItem>,
) -> Vec<LearningItem> {
    for item in new_items {
        push_unique(&mut existing, item);
    }
    existing
}

fn push_unique(items: &mut Vec<LearningItem>, item: LearningItem) {
    let new_id = get_learning_item_id(&item);
    if !items.iter().any(|i| get_learning_item_id(i) == new_id) {
        items.push(item);
    }
}

pub fn update_item_score(mut item: LearningItem, analysis: &AgentAnalysis) -> LearningItem {
    match &item.item {
        LearningItemType::Mistake(m) => {
            if let Some(score_obj) = analysis.mistake_scores.get(&m.id) {
                item.score = add_score(item.score, score_obj.score);
            }
        }
        LearningItemType::Explanation(e) => {
            if let Some(score_obj) = analysis.explained_scores.get(&e.id) {
                item.score = add_score(item.score, score_obj.score);
            }
        }
        LearningItemType::Translation(t) => {
            if let Some(score_obj) = analysis.translated_scores.get(&t.id) {
                item.score = add_score(item.score, score_obj.score);
            }
        }
        LearningItemType::Exploration(e) => {
            if let Some(score_obj) = analysis.exploratory_scores.get(&e.id) {
                item.score = add_score(item.score, score_obj.score);
            }
        }
    }
    item
}

pub fn add_score(current: u8, delta: i8) -> u8 {
    (current as i32 + delta as i32).clamp(0, 100) as u8
}

pub fn cycle_dialect(state: &UserState) -> Dialect {
    let dialects = state.current_dialects();
    let current = state.current_dialect();
    match dialects.iter().position(|df| df.dialect == current) {
        Some(idx) => dialects[(idx + 1) % dialects.len()].dialect,
        None => current,
    }
}

pub fn cycle_formality(current: Formality) -> Formality {
    match current {
        Formality::Formal => Formality::ProfessionalCasual,
        Formality::ProfessionalCasual => Formality::Informal,
        Formality::Informal => Formality::Slang,
        Formality::Slang => Formality::Formal,
    }
}

pub fn cycle_teaching_mode(current: TeachingMode, is_admin: bool) -> TeachingMode {
    match current {
        TeachingMode::Immersive => TeachingMode::Corrective,
        TeachingMode::Corrective => TeachingMode::Explanatory,
        TeachingMode::Explanatory => TeachingMode::StoryTeller,
        TeachingMode::StoryTeller => TeachingMode::ErrorFinding,
        TeachingMode::ErrorFinding => {
            if is_admin {
                TeachingMode::Debug
            } else {
                TeachingMode::Immersive
            }
        }
        TeachingMode::Debug => TeachingMode::Immersive,
    }
}

pub fn apply_score_updates(
    items: Vec<LearningItem>,
    analysis: &AgentAnalysis,
) -> Vec<LearningItem> {
    items
        .into_iter()
        .map(|item| update_item_score(item, analysis))
        .collect()
}

pub fn delete_learning_item(mut items: Vec<LearningItem>, id: Uuid) -> Vec<LearningItem> {
    items.retain(|item| get_learning_item_id(item) != id);
    items
}

pub fn undo_delete_learning_item(
    mut items: Vec<LearningItem>,
    item: LearningItem,
) -> Vec<LearningItem> {
    items.push(item);
    items
}

pub fn delete_message(mut history: Vec<Message>, id: Uuid) -> Vec<Message> {
    history.retain(|msg| msg.id != id);
    history
}

pub fn remove_message_from_branches(
    mut branches: Vec<ConversationBranch>,
    msg_id: Uuid,
) -> Vec<ConversationBranch> {
    for branch in &mut branches {
        if let Some(pos) = branch.message_ids.iter().position(|&id| id == msg_id) {
            branch.message_ids.remove(pos);
            branch.leaf_message_id = branch.message_ids.last().copied();
        }
    }
    branches
}

pub fn capture_deleted_message(state: &UserState, id: Uuid) -> Option<DeletedMessage> {
    let history_index = state
        .conversation_history
        .iter()
        .position(|msg| msg.id == id)?;
    let branches = state
        .branches
        .iter()
        .filter_map(|branch| deleted_message_location(branch, id))
        .collect();
    Some(DeletedMessage {
        message: state.conversation_history[history_index].clone(),
        history_index,
        branches,
    })
}

fn deleted_message_location(
    branch: &ConversationBranch,
    id: Uuid,
) -> Option<DeletedMessageLocation> {
    let message_index = branch.message_ids.iter().position(|&msg_id| msg_id == id)?;
    Some(DeletedMessageLocation {
        branch_id: branch.id,
        message_index,
        leaf_message_id: branch.leaf_message_id,
    })
}

pub fn undo_delete_message(mut history: Vec<Message>, deleted: &DeletedMessage) -> Vec<Message> {
    history.insert(deleted.history_index, deleted.message.clone());
    history
}

pub fn restore_message_to_branch(
    mut branches: Vec<ConversationBranch>,
    deleted: &DeletedMessage,
) -> Vec<ConversationBranch> {
    for location in &deleted.branches {
        if let Some(branch) = branches
            .iter_mut()
            .find(|branch| branch.id == location.branch_id)
        {
            restore_message_location(branch, location, deleted.message.id);
        }
    }
    branches
}

fn restore_message_location(
    branch: &mut ConversationBranch,
    location: &DeletedMessageLocation,
    id: Uuid,
) {
    branch.message_ids.insert(location.message_index, id);
    if location.message_index == branch.message_ids.len() - 1 {
        branch.leaf_message_id = location.leaf_message_id;
    }
}

pub fn add_learning_goal(mut goals: Vec<LearningGoal>, goal: LearningGoal) -> Vec<LearningGoal> {
    goals.push(goal);
    goals
}

pub fn delete_learning_goal(mut goals: Vec<LearningGoal>, index: usize) -> Vec<LearningGoal> {
    goals.remove(index);
    goals
}

pub fn create_new_branch_for_language(state: &mut UserState, dialect: Dialect) {
    let plan_id = state.active_branch_plan_id();
    // Copy settings from current branch but convert level for new language
    let mut settings = state.active_branch_settings().clone();
    settings.language_level = settings
        .language_level
        .convert_for_language(dialect.language());
    let new_branch = ConversationBranch::new(None, None, None, dialect, vec![], plan_id, settings);
    let new_branch_id = new_branch.id;
    Arc::make_mut(&mut state.branches).push(new_branch);
    state.active_branch_id = new_branch_id;
}

pub fn find_or_create_branch_for_dialect(state: &mut UserState, dialect: Dialect) {
    let existing_branch_id = find_branch_with_most_recent_message(state, dialect);

    if let Some(branch_id) = existing_branch_id {
        state.active_branch_id = branch_id;
    } else {
        create_new_branch_for_language(state, dialect);
    }
}

fn find_branch_with_most_recent_message(state: &UserState, dialect: Dialect) -> Option<Uuid> {
    state
        .branches
        .iter()
        .filter(|b| b.dialect == dialect)
        .map(|branch| {
            let timestamp = branch
                .leaf_message_id
                .and_then(|msg_id| {
                    state
                        .conversation_history
                        .iter()
                        .find(|m| m.id == msg_id)
                        .map(|m| m.metadata.timestamp)
                })
                .unwrap_or(branch.created_at);
            (branch.id, timestamp)
        })
        .max_by_key(|(_, timestamp)| *timestamp)
        .map(|(id, _)| id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use dialect_coach_shared::models::MistakeCategory;

    #[test]
    fn test_add_learning_items_deduplication() {
        let dialect = Dialect::SpanishMexican;
        let mistake = Mistake::new(
            "mistake".to_string(),
            "correction".to_string(),
            MistakeCategory::SpellingError {
                context: "ctx".to_string(),
            },
        );
        let item = LearningItem::new(LearningItemType::Mistake(mistake.clone()), dialect);

        let items = vec![item.clone()];

        // Try adding the same mistake again
        let result =
            add_learning_items_to_vec(items, vec![(mistake, 0)], vec![], vec![], vec![], dialect);

        assert_eq!(result.len(), 1, "Should not add duplicate item");
        assert_eq!(
            get_learning_item_id(&result[0]),
            get_learning_item_id(&item)
        );
    }

    #[test]
    fn test_cycle_teaching_mode_admin() {
        // Admin should be able to access Debug mode
        let current = TeachingMode::ErrorFinding;
        let next = cycle_teaching_mode(current, true);
        assert_eq!(next, TeachingMode::Debug);

        // From Debug, should cycle back to Immersive
        let next = cycle_teaching_mode(TeachingMode::Debug, true);
        assert_eq!(next, TeachingMode::Immersive);
    }

    #[test]
    fn test_cycle_teaching_mode_non_admin() {
        // Non-admin should skip Debug mode (ErrorFinding -> Immersive)
        let current = TeachingMode::ErrorFinding;
        let next = cycle_teaching_mode(current, false);
        assert_eq!(next, TeachingMode::Immersive);

        // If somehow in Debug mode (e.g. legacy state), should recover to Immersive
        let next = cycle_teaching_mode(TeachingMode::Debug, false);
        assert_eq!(next, TeachingMode::Immersive);
    }
}
