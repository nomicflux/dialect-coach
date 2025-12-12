use dialect_coach_shared::models::gamification::{GamificationStats, derive_gamification_stats};
use dialect_coach_shared::models::{ConversationBranch, Dialect, Formality, Message, TeachingMode};
use dialect_coach_shared::{
    AgentAnalysis, Explained, Exploratory, LearningGoal, LearningItem, LearningItemType, Mistake,
    Translated, UserState,
};
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
    mistakes: Vec<Mistake>,
    explained: Vec<Explained>,
    translated: Vec<Translated>,
    exploratory: Vec<Exploratory>,
    dialect: Dialect,
) -> Vec<LearningItem> {
    for mistake in mistakes {
        items.push(LearningItem::new(
            LearningItemType::Mistake(mistake),
            dialect,
        ));
    }
    for expl in explained {
        items.push(LearningItem::new(
            LearningItemType::Explanation(expl),
            dialect,
        ));
    }
    for trans in translated {
        items.push(LearningItem::new(
            LearningItemType::Translation(trans),
            dialect,
        ));
    }
    for explor in exploratory {
        items.push(LearningItem::new(
            LearningItemType::Exploration(explor),
            dialect,
        ));
    }
    items
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

pub fn cycle_teaching_mode(current: TeachingMode) -> TeachingMode {
    match current {
        TeachingMode::Immersive => TeachingMode::Corrective,
        TeachingMode::Corrective => TeachingMode::Explanatory,
        TeachingMode::Explanatory => TeachingMode::Interleaved,
        TeachingMode::Interleaved => TeachingMode::StoryTeller,
        TeachingMode::StoryTeller => TeachingMode::Debug,
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

pub fn undo_delete_message(mut history: Vec<Message>, msg: Message) -> Vec<Message> {
    history.push(msg);
    history
}

pub fn restore_message_to_branch(
    mut branches: Vec<ConversationBranch>,
    msg: &Message,
) -> Vec<ConversationBranch> {
    // Find the branch that should contain this message based on parent chain
    for branch in &mut branches {
        // Check if this message belongs in this branch by verifying parent chain
        if let Some(parent_id) = msg.parent_id {
            if branch.message_ids.contains(&parent_id) {
                // Insert after parent, maintaining order
                if let Some(pos) = branch.message_ids.iter().position(|&id| id == parent_id) {
                    branch.message_ids.insert(pos + 1, msg.id);
                    // Update leaf if this was the last message
                    if pos + 1 == branch.message_ids.len() - 1 {
                        branch.leaf_message_id = Some(msg.id);
                    }
                }
                break;
            }
        } else {
            // Root message - add to branch with no parent
            if branch.parent_message_id.is_none() && !branch.message_ids.contains(&msg.id) {
                branch.message_ids.insert(0, msg.id);
                if branch.message_ids.len() == 1 {
                    branch.leaf_message_id = Some(msg.id);
                }
                break;
            }
        }
    }
    branches
}

pub fn add_learning_goal(mut goals: Vec<LearningGoal>, goal: LearningGoal) -> Vec<LearningGoal> {
    goals.push(goal);
    goals
}

pub fn delete_learning_goal(mut goals: Vec<LearningGoal>, index: usize) -> Vec<LearningGoal> {
    goals.remove(index);
    goals
}

pub fn create_new_branch_for_language(state: &mut UserState) {
    let new_branch = ConversationBranch::new(None, None, None, None, vec![]);
    let new_branch_id = new_branch.id;
    state.branches.push(new_branch);
    state.active_branch_id = new_branch_id;
}

pub fn sync_to_active_branch(state: &mut UserState) {
    if let Some(branch) = state
        .branches
        .iter()
        .find(|b| b.id == state.active_branch_id)
        && let Some(dialect) = branch.dialect
    {
        state.selected_dialect = dialect;
        state.selected_language = dialect.language();
    }
}
