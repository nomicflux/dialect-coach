use super::actions::{
    BranchAction, LearningAction, MessageAction, PlanAction, SettingsAction, UserStateAction,
};
use super::helpers::*;
use dialect_coach_shared::UserState;
use dialect_coach_shared::models::ConversationBranch;
use dialect_coach_shared::{AgentAnalysis, LearningGoal, LearningItem, LearningItemType};

pub(crate) fn reduce_message(next: &mut UserState, action: MessageAction) {
    use MessageAction::*;
    use std::sync::Arc;
    match action {
        Add(mut msg) => {
            let current_leaf = next
                .branches
                .iter()
                .find(|b| b.id == next.active_branch_id)
                .and_then(|b| b.leaf_message_id);

            msg.parent_id = current_leaf;
            let new_msg_id = msg.id;
            let msg_dialect = msg.metadata.dialect;
            Arc::make_mut(&mut next.conversation_history).push(msg);

            if let Some(branch) = Arc::make_mut(&mut next.branches)
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
        Delete(id) => {
            next.conversation_history =
                Arc::new(delete_message((*next.conversation_history).clone(), id));
            next.branches = Arc::new(remove_message_from_branches((*next.branches).clone(), id));
        }
        UndoDelete(msg) => {
            next.conversation_history = Arc::new(undo_delete_message(
                (*next.conversation_history).clone(),
                msg.clone(),
            ));
            next.branches = Arc::new(restore_message_to_branch((*next.branches).clone(), &msg));
        }
    }
}

pub(crate) fn reduce_learning(next: &mut UserState, action: LearningAction) {
    use LearningAction::*;
    use std::sync::Arc;
    match action {
        AddItems(mistakes, explained, translated, exploratory) => {
            next.learning_items = Arc::new(add_learning_items_to_vec(
                (*next.learning_items).clone(),
                mistakes,
                explained,
                translated,
                exploratory,
                next.selected_dialect,
            ));
        }
        UpdateScores(analysis) => {
            next.learning_items = Arc::new(apply_score_updates(
                (*next.learning_items).clone(),
                &analysis,
            ));
            promote_plan_items(next, &analysis);
            check_step_completion(next);
        }
        DeleteItem(id) => {
            next.learning_items =
                Arc::new(delete_learning_item((*next.learning_items).clone(), id));
        }
        UndoDeleteItem(item) => {
            next.learning_items = Arc::new(undo_delete_learning_item(
                (*next.learning_items).clone(),
                item,
            ));
        }
        AddGoal(goal_text) => {
            let goal = LearningGoal {
                goal: goal_text,
                dialect: next.selected_dialect,
            };
            next.learning_goals = Arc::new(add_learning_goal((*next.learning_goals).clone(), goal));
        }
        DeleteGoal(index) => {
            next.learning_goals =
                Arc::new(delete_learning_goal((*next.learning_goals).clone(), index));
        }
    }
}

pub(crate) fn reduce_branch(next: &mut UserState, action: BranchAction) {
    use BranchAction::*;
    use std::sync::Arc;
    match action {
        Create(message_id) => {
            // Update the current branch's parent_message_id if it's None
            if let Some(current_branch) = Arc::make_mut(&mut next.branches)
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
            Arc::make_mut(&mut next.branches).push(new_branch);
            next.active_branch_id = new_branch_id;
            sync_to_active_branch(next);
        }
        Switch(branch_id) => {
            next.active_branch_id = branch_id;
            sync_to_active_branch(next);
        }
        Delete(branch_id) => {
            Arc::make_mut(&mut next.branches).retain(|b| b.id != branch_id);

            if next.active_branch_id == branch_id {
                next.active_branch_id = next
                    .branches
                    .first()
                    .map(|b| b.id)
                    .unwrap_or(next.active_branch_id);
                sync_to_active_branch(next);
            }
        }
        Rename(branch_id, name) => {
            if let Some(branch) = Arc::make_mut(&mut next.branches)
                .iter_mut()
                .find(|b| b.id == branch_id)
            {
                branch.name = Some(name);
            }
        }
    }
}

pub(crate) fn reduce_plan(next: &mut UserState, action: PlanAction) {
    use PlanAction::*;
    use std::sync::Arc;
    match action {
        Add(plan) => {
            Arc::make_mut(&mut next.language_plans).push(plan);
        }
        Delete(plan_id) => {
            Arc::make_mut(&mut next.language_plans).retain(|p| p.id != plan_id);
            if next.active_plan_id == Some(plan_id) {
                next.active_plan_id = None;
            }
        }
        SetActive(plan_id) => {
            next.active_plan_id = plan_id;
            if let Some(pid) = plan_id {
                if let Some(plan) = Arc::make_mut(&mut next.language_plans)
                    .iter_mut()
                    .find(|p| p.id == pid)
                {
                    plan.start();
                }
                // Activate the current step's items when plan becomes active
                activate_step_items(next, pid);
            }
        }
        AdvanceStep(plan_id) => {
            if let Some(plan) = Arc::make_mut(&mut next.language_plans)
                .iter_mut()
                .find(|p| p.id == plan_id)
            {
                plan.advance_step();
            }
            // Activate the new step's items after advancing
            activate_step_items(next, plan_id);
        }
        ActivateStep(plan_id) => {
            activate_step_items(next, plan_id);
        }
        Update(updated_plan) => {
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
                Arc::make_mut(&mut next.language_plans)[plan_idx] = updated_plan;
            } else {
                log::error!(
                    "Failed to update plan: Plan not found with ID {}",
                    updated_plan.id
                );
            }
        }
    }
}

pub(crate) fn reduce_settings(next: &mut UserState, action: SettingsAction) {
    use SettingsAction::*;
    match action {
        ChangeDialect(dialect) => {
            let old_language = next.selected_dialect.language();
            next.selected_dialect = dialect;
            let new_language = dialect.language();

            if old_language != new_language {
                create_new_branch_for_language(next);
            }
        }
        ChangeLanguage(language) => {
            // Only switch language if we can find a valid valid dialect for it
            if let Some(dialect) = UserState::default_dialect_for_language(language, next.show_experimental_dialects) {
                let old_language = next.selected_language;
                next.selected_language = language;
                next.selected_dialect = dialect;

                if old_language != language {
                    create_new_branch_for_language(next);
                }
            } else {
               log::warn!("No default dialect found for language: {:?}", language);
            }
        }
        ChangeFormality(formality) => {
            next.formality = formality;
        }
        ChangeTeachingMode(tm) => {
            next.teaching_mode = tm;
        }
        UpdateGender(gender) => {
            next.user_gender = gender;
        }
        UpdateLevel(level) => {
            next.set_level_for_dialect(next.selected_dialect, level);
        }
        ToggleTTS => {
            next.tts_enabled = !next.tts_enabled;
        }
        ToggleExperimentalDialects => {
            next.show_experimental_dialects = !next.show_experimental_dialects;
        }
        CycleDialect => {
            let old_language = next.selected_dialect.language();
            next.selected_dialect = cycle_dialect(next);
            let new_language = next.selected_dialect.language();

            if old_language != new_language {
                create_new_branch_for_language(next);
            }
        }
        CycleFormality => {
            next.formality = cycle_formality(next.formality);
        }
        CycleTeachingMode(is_admin) => {
            next.teaching_mode = cycle_teaching_mode(next.teaching_mode, is_admin);
        }
        SetArabicScript(script) => {
            next.language_options.arabic_script = script;
        }
        SetJapaneseScript(script) => {
            next.language_options.japanese_script = script;
        }
    }
}

pub(crate) fn apply_user_state_action(state: &UserState, action: UserStateAction) -> UserState {
    let mut next = state.clone();
    match action {
        UserStateAction::Message(a) => reduce_message(&mut next, a),
        UserStateAction::Learning(a) => reduce_learning(&mut next, a),
        UserStateAction::Branch(a) => reduce_branch(&mut next, a),
        UserStateAction::Plan(a) => reduce_plan(&mut next, a),
        UserStateAction::Settings(a) => reduce_settings(&mut next, a),
        UserStateAction::UpdateUsageStats(stats) => {
            next.usage_stats = stats;
        }
        _ => {} // Replace/Clear handled by wrapper
    }
    next
}

fn promote_plan_items(state: &mut UserState, analysis: &AgentAnalysis) {
    use std::sync::Arc;
    if let Some(plan) = state
        .language_plans
        .iter()
        .find(|p| Some(p.id) == state.active_plan_id)
        && let Some(step) = plan.steps.get(plan.current_step_index)
        && let dialect_coach_shared::StepType::Learning { content } = &step.step_type
    {
        for item in &content.items {
            let id = get_learning_item_id(item);
            if !state
                .learning_items
                .iter()
                .any(|i| get_learning_item_id(i) == id)
                && item_has_score(item, analysis)
            {
                Arc::make_mut(&mut state.learning_items).push(item.clone());
            }
        }
    }
}

fn check_step_completion(state: &mut UserState) {
    use std::sync::Arc;
    if let Some(plan) = Arc::make_mut(&mut state.language_plans)
        .iter_mut()
        .find(|p| Some(p.id) == state.active_plan_id)
        && let Some(step) = plan.steps.get(plan.current_step_index)
        && let dialect_coach_shared::StepType::Learning { content } = &step.step_type
    {
        let all_passed = content.items.iter().all(|plan_item| {
            state.learning_items.iter().any(|item| {
                get_learning_item_id(item) == get_learning_item_id(plan_item) && item.score >= 50
            })
        });

        if all_passed {
            plan.advance_step();
        }
    }
}

fn item_has_score(item: &LearningItem, analysis: &AgentAnalysis) -> bool {
    match &item.item {
        LearningItemType::Mistake(m) => analysis.mistake_scores.contains_key(&m.id),
        LearningItemType::Explanation(e) => analysis.explained_scores.contains_key(&e.id),
        LearningItemType::Translation(t) => analysis.translated_scores.contains_key(&t.id),
        LearningItemType::Exploration(e) => analysis.exploratory_scores.contains_key(&e.id),
    }
}

fn activate_step_items(state: &mut UserState, plan_id: uuid::Uuid) {
    use std::sync::Arc;
    if let Some(plan) = Arc::make_mut(&mut state.language_plans)
        .iter_mut()
        .find(|p| p.id == plan_id)
        && let Some(step) = plan.steps.get(plan.current_step_index)
        && let dialect_coach_shared::StepType::Learning { content } = &step.step_type
    {
        // Add items to learning list (idempotent due to helper)
        state.learning_items = Arc::new(merge_learning_items(
            (*state.learning_items).clone(),
            content.items.clone(),
        ));

        // Update plan status
        if plan.status == dialect_coach_shared::models::PlanStatus::NotStarted {
            plan.start();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dialect_coach_shared::models::{LearningItemScore, MistakeCategory};
    use dialect_coach_shared::{Dialect, LanguagePlan, Mistake, PlanContent, PlanStep, StepType};
    use std::sync::Arc;
    use uuid::Uuid;

    #[test]
    fn test_promote_plan_items() {
        let mut state = UserState::new(Uuid::new_v4());
        let dialect = Dialect::SpanishMexican;
        let mut plan = LanguagePlan::new(
            "Test Plan".to_string(),
            dialect,
            Some("Beginner".to_string()),
            vec![],
        );

        let mistake = Mistake::new(
            "mistake".to_string(),
            "correction".to_string(),
            MistakeCategory::SpellingError {
                context: "ctx".to_string(),
            },
        );
        let plan_item = LearningItem::new(LearningItemType::Mistake(mistake.clone()), dialect);

        plan.steps.push(PlanStep::new(
            1,
            "Step 1".to_string(),
            StepType::Learning {
                content: PlanContent {
                    items: vec![plan_item.clone()],
                },
            },
            "Learn this".to_string(),
        ));

        Arc::make_mut(&mut state.language_plans).push(plan.clone());
        state.active_plan_id = Some(plan.id);

        let mut analysis = AgentAnalysis::default();
        analysis
            .mistake_scores
            .insert(mistake.id, LearningItemScore { score: 10 });

        let action = LearningAction::UpdateScores(analysis);
        reduce_learning(&mut state, action);

        assert_eq!(state.learning_items.len(), 1);
        assert_eq!(get_learning_item_id(&state.learning_items[0]), mistake.id);
    }

    #[test]
    fn test_check_step_completion() {
        let mut state = UserState::new(Uuid::new_v4());
        let dialect = Dialect::SpanishMexican;
        let mut plan = LanguagePlan::new(
            "Test Plan".to_string(),
            dialect,
            Some("Beginner".to_string()),
            vec![],
        );

        let mistake = Mistake::new(
            "mistake".to_string(),
            "correction".to_string(),
            MistakeCategory::SpellingError {
                context: "ctx".to_string(),
            },
        );
        let plan_item = LearningItem::new(LearningItemType::Mistake(mistake.clone()), dialect);
        let mut active_item = plan_item.clone();
        active_item.score = 50;

        plan.steps.push(PlanStep::new(
            1,
            "Step 1".to_string(),
            StepType::Learning {
                content: PlanContent {
                    items: vec![plan_item.clone()],
                },
            },
            "Learn this".to_string(),
        ));
        plan.steps.push(PlanStep::new(
            2,
            "Step 2".to_string(),
            StepType::Learning {
                content: PlanContent { items: vec![] },
            },
            "Next Step".to_string(),
        ));

        Arc::make_mut(&mut state.language_plans).push(plan.clone());
        state.active_plan_id = Some(plan.id);
        Arc::make_mut(&mut state.learning_items).push(active_item);

        // Verification: current index should be 0
        assert_eq!(state.language_plans[0].current_step_index, 0);

        // Trigger logic manually via helper or action?
        // Action is easier to check integration.
        let analysis = AgentAnalysis::default();
        let action = LearningAction::UpdateScores(analysis);
        reduce_learning(&mut state, action);

        // Should have advanced
        assert_eq!(state.language_plans[0].current_step_index, 1);
    }

    #[test]
    fn test_activate_step_promotion() {
        let mut state = UserState::new(Uuid::new_v4());
        let dialect = Dialect::SpanishMexican;
        state.selected_dialect = dialect;
        let mut plan = LanguagePlan::new(
            "Test Plan".to_string(),
            dialect,
            Some("Beginner".to_string()),
            vec![],
        );

        let mistake = Mistake::new(
            "mistake".to_string(),
            "correction".to_string(),
            MistakeCategory::SpellingError {
                context: "ctx".to_string(),
            },
        );
        let plan_item = LearningItem::new(LearningItemType::Mistake(mistake.clone()), dialect);

        plan.steps.push(PlanStep::new(
            1,
            "Step 1".to_string(),
            StepType::Learning {
                content: PlanContent {
                    items: vec![plan_item.clone()],
                },
            },
            "Learn this".to_string(),
        ));

        Arc::make_mut(&mut state.language_plans).push(plan.clone());
        state.active_plan_id = Some(plan.id);

        // Initially no items
        assert!(state.learning_items.is_empty());

        // Activate
        let action = PlanAction::ActivateStep(plan.id);
        reduce_plan(&mut state, action);

        // Verify items moved to learning_items and plan status updated
        assert_eq!(state.learning_items.len(), 1);
        assert_eq!(get_learning_item_id(&state.learning_items[0]), mistake.id);

        let updated_plan = state.language_plans.first().unwrap();
        assert_eq!(
            updated_plan.status,
            dialect_coach_shared::models::PlanStatus::InProgress
        );
    }
}
