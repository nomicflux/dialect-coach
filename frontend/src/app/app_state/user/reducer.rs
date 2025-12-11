use super::actions::UserStateAction;
use super::helpers::*;
use dialect_coach_shared::models::ConversationBranch;
use dialect_coach_shared::UserState;
use std::rc::Rc;
use yew::prelude::*;

pub fn apply_user_state_action(state: &UserState, action: UserStateAction) -> UserState {
    let mut next = state.clone();
    match action {
        UserStateAction::AddMessage(mut msg) => {
            let current_leaf = next
                .branches
                .iter()
                .find(|b| b.id == next.active_branch_id)
                .and_then(|b| b.leaf_message_id);

            msg.parent_id = current_leaf;
            let new_msg_id = msg.id;
            let msg_dialect = msg.metadata.dialect;
            next.conversation_history.push(msg);

            if let Some(branch) = next
                .branches
                .iter_mut()
                .find(|b| b.id == next.active_branch_id)
            {
                branch.message_ids.push(new_msg_id);
                branch.leaf_message_id = Some(new_msg_id);
                // Set branch dialect from first message if dialect is None
                if branch.dialect.is_none() {
                    branch.dialect = Some(msg_dialect);
                }
            }
        }
        UserStateAction::AddLearningItems(mistakes, explained, translated, exploratory) => {
            next.learning_items = add_learning_items_to_vec(
                next.learning_items,
                mistakes,
                explained,
                translated,
                exploratory,
                next.selected_dialect,
            );
        }
        UserStateAction::UpdateScores(analysis) => {
            next.learning_items = apply_score_updates(next.learning_items, &analysis);
        }
        UserStateAction::ChangeDialect(dialect) => {
            let old_language = next.selected_dialect.language();
            next.selected_dialect = dialect;
            let new_language = dialect.language();

            if old_language != new_language {
                create_new_branch_for_language(&mut next);
            }
        }
        UserStateAction::ChangeLanguage(language) => {
            let old_language = next.selected_language;
            next.selected_language = language;
            next.selected_dialect =
                UserState::default_dialect_for_language(language, next.show_experimental_dialects);

            if old_language != language {
                create_new_branch_for_language(&mut next);
            }
        }
        UserStateAction::ChangeFormality(formality) => {
            next.formality = formality;
        }
        UserStateAction::ChangeTeachingMode(tm) => {
            next.teaching_mode = tm;
        }
        UserStateAction::UpdateUserGender(gender) => {
            next.user_gender = gender;
        }
        UserStateAction::UpdateLanguageLevel(dialect, level) => {
            next.set_level_for_dialect(dialect, level);
        }
        UserStateAction::ToggleTTS => {
            next.tts_enabled = !next.tts_enabled;
        }
        UserStateAction::ToggleShowExperimentalDialects => {
            next.show_experimental_dialects = !next.show_experimental_dialects;
        }
        UserStateAction::ReplaceUserState(new_state) => {
            next = new_state;
        }
        UserStateAction::UpdateUsageStats(new_stats) => {
            next.usage_stats = new_stats;
        }
        UserStateAction::DeleteLearningItem(id) => {
            next.learning_items = delete_learning_item(next.learning_items, id);
        }
        UserStateAction::UndoDeleteLearningItem(item) => {
            next.learning_items = undo_delete_learning_item(next.learning_items, item);
        }
        UserStateAction::DeleteMessage(id) => {
            next.conversation_history = delete_message(next.conversation_history, id);
            next.branches = remove_message_from_branches(next.branches, id);
        }
        UserStateAction::UndoDeleteMessage(msg) => {
            next.conversation_history = undo_delete_message(next.conversation_history, msg);
        }
        UserStateAction::CreateBranch(message_id) => {
            // Update the current branch's parent_message_id if it's None
            // This ensures both branches know where they diverged
            if let Some(current_branch) = next
                .branches
                .iter_mut()
                .find(|b| b.id == next.active_branch_id)
                && current_branch.parent_message_id.is_none()
            {
                current_branch.parent_message_id = Some(message_id);
            }

            // Create new branch with dialect from the branching message
            let dialect = next
                .conversation_history
                .iter()
                .find(|m| m.id == message_id)
                .map(|m| m.metadata.dialect);
            let message_ids = next
                .get_path_to_message(Some(message_id))
                .into_iter()
                .map(|m| m.id)
                .collect();
            let new_branch = ConversationBranch::new(
                Some(message_id),
                None,
                Some(message_id),
                dialect,
                message_ids,
            );
            let new_branch_id = new_branch.id;
            next.branches.push(new_branch);
            next.active_branch_id = new_branch_id;
            sync_to_active_branch(&mut next);
        }
        UserStateAction::SwitchBranch(branch_id) => {
            next.active_branch_id = branch_id;
            sync_to_active_branch(&mut next);
        }
        UserStateAction::DeleteBranch(branch_id) => {
            next.branches.retain(|b| b.id != branch_id);

            if next.active_branch_id == branch_id {
                next.active_branch_id = next
                    .branches
                    .first()
                    .map(|b| b.id)
                    .unwrap_or(next.active_branch_id);
                sync_to_active_branch(&mut next);
            }
        }
        UserStateAction::RenameBranch(branch_id, name) => {
            if let Some(branch) = next.branches.iter_mut().find(|b| b.id == branch_id) {
                branch.name = Some(name);
            }
        }
        UserStateAction::AddLearningGoal(goal) => {
            next.learning_goals = add_learning_goal(next.learning_goals, goal);
        }
        UserStateAction::DeleteLearningGoal(index) => {
            next.learning_goals = delete_learning_goal(next.learning_goals, index);
        }
        UserStateAction::CycleDialect => {
            let old_language = next.selected_dialect.language();
            next.selected_dialect = cycle_dialect(&next);
            let new_language = next.selected_dialect.language();

            if old_language != new_language {
                create_new_branch_for_language(&mut next);
            }
        }
        UserStateAction::CycleFormality => {
            next.formality = cycle_formality(next.formality);
        }
        UserStateAction::CycleTeachingMode => {
            next.teaching_mode = cycle_teaching_mode(next.teaching_mode);
        }
        UserStateAction::SetArabicScript(script) => {
            next.language_options.arabic_script = script;
        }
        UserStateAction::SetJapaneseScript(script) => {
            next.language_options.japanese_script = script;
        }
        UserStateAction::AddLanguagePlan(plan) => {
            next.language_plans.push(plan);
        }
        UserStateAction::DeleteLanguagePlan(plan_id) => {
            next.language_plans.retain(|p| p.id != plan_id);
            if next.active_plan_id == Some(plan_id) {
                next.active_plan_id = None;
            }
        }
        UserStateAction::SetActivePlan(plan_id) => {
            next.active_plan_id = plan_id;
            if let Some(pid) = plan_id
                && let Some(plan) = next.language_plans.iter_mut().find(|p| p.id == pid)
            {
                plan.start();
            }
        }
        UserStateAction::AdvancePlanStep(plan_id) => {
            if let Some(plan) = next.language_plans.iter_mut().find(|p| p.id == plan_id) {
                plan.advance_step();
            }
        }
        UserStateAction::UpdateLanguagePlan(updated_plan) => {
            if let Some(plan_idx) = next
                .language_plans
                .iter()
                .position(|p| p.id == updated_plan.id)
            {
                log::info!(
                    "Updating plan: {} ({})",
                    updated_plan.title,
                    updated_plan.id
                );
                next.language_plans[plan_idx] = updated_plan;
            } else {
                log::error!(
                    "Failed to update plan: Plan not found with ID {}",
                    updated_plan.id
                );
            }
        }
        UserStateAction::ClearUserState => {
            // This should never be called - ClearUserState is handled at OptionalUserState level
            // But we need this case for exhaustiveness
            panic!("ClearUserState should not reach apply_user_state_action");
        }
    }
    next
}

#[derive(Clone, PartialEq, Default)]
pub struct OptionalUserState {
    pub state: Option<UserState>,
    pub needs_save: bool,
}

impl Reducible for OptionalUserState {
    type Action = UserStateAction;

    fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
        match action {
            UserStateAction::ReplaceUserState(new_state) => {
                OptionalUserState {
                    state: Some(new_state),
                    needs_save: false,
                }
                .into()
            }
            UserStateAction::ClearUserState => OptionalUserState {
                state: None,
                needs_save: false,
            }
            .into(),
            _ => match &self.state {
                Some(state) => OptionalUserState {
                    state: Some(apply_user_state_action(state, action)),
                    needs_save: true,
                }
                .into(),
                None => self,
            },
        }
    }
}
