use super::actions::{BranchAction, MessageAction, PlanAction, SettingsAction};
use super::*;
use dialect_coach_shared::models::MessageMetadata;
use dialect_coach_shared::models::{
    ConversationBranch, Dialect, Formality, Language, TeachingMode,
};
use dialect_coach_shared::{CefrLevel, LanguageLevel, Message, UsageStats, UserState};
use std::sync::Arc;
use uuid::Uuid;

fn apply_user_state_action(state: &UserState, action: UserStateAction) -> Option<UserState> {
    use super::reducer::*;
    let mut next = state.clone();
    match action {
        UserStateAction::Message(a) => reduce_message(&mut next, a),
        UserStateAction::Learning(a) => reduce_learning(&mut next, a),
        UserStateAction::Branch(a) => reduce_branch(&mut next, a),
        UserStateAction::Plan(a) => reduce_plan(&mut next, a),
        UserStateAction::Settings(a) => reduce_settings(&mut next, a),
        UserStateAction::UpdateUsageStats(s) => next.usage_stats = s,
        UserStateAction::ReplaceUserState(s) => return Some(s),
        UserStateAction::ClearUserState => return None,
    }
    Some(next)
}

fn create_test_message(session_id: Uuid, parent_id: Option<Uuid>) -> Message {
    Message::user_message(
        "test".to_string(),
        MessageMetadata::at_now(
            Formality::Informal,
            TeachingMode::Immersive,
            Language::Spanish,
            Dialect::SpanishMexican,
            session_id,
        ),
        parent_id,
    )
}

#[test]
fn test_create_branch_reducer() {
    let mut state = UserState::new(Uuid::new_v4());

    // Create message A and add to conversation
    let session_id = Uuid::new_v4();
    let msg_a = create_test_message(session_id, None);
    Arc::make_mut(&mut state.conversation_history).push(msg_a.clone());

    let initial_branch_count = state.branches.len();

    let action = UserStateAction::Branch(BranchAction::Create(msg_a.id));
    state = apply_user_state_action(&state, action).unwrap();

    assert_eq!(state.branches.len(), initial_branch_count + 1);
    let new_branch = state.branches.last().unwrap();
    assert_eq!(new_branch.parent_message_id, Some(msg_a.id));
    assert_eq!(new_branch.leaf_message_id, Some(msg_a.id));
    assert_eq!(state.active_branch_id, new_branch.id);
    // Phase 2: Verify message_ids is populated with path to fork point
    assert_eq!(new_branch.message_ids, vec![msg_a.id]);
}

#[test]
fn test_create_branch_copies_full_message_path() {
    let mut state = UserState::new(Uuid::new_v4());
    let session_id = Uuid::new_v4();

    // Create conversation chain: A -> B -> C
    let msg_a = create_test_message(session_id, None);
    let msg_b = create_test_message(session_id, Some(msg_a.id));
    let msg_c = create_test_message(session_id, Some(msg_b.id));

    Arc::make_mut(&mut state.conversation_history).push(msg_a.clone());
    Arc::make_mut(&mut state.conversation_history).push(msg_b.clone());
    Arc::make_mut(&mut state.conversation_history).push(msg_c.clone());

    // Update initial branch to point to C
    Arc::make_mut(&mut state.branches)[0].leaf_message_id = Some(msg_c.id);
    Arc::make_mut(&mut state.branches)[0].message_ids = vec![msg_a.id, msg_b.id, msg_c.id];

    // Branch from B
    let action = UserStateAction::Branch(BranchAction::Create(msg_b.id));
    state = apply_user_state_action(&state, action).unwrap();

    // New branch should have message_ids = [A, B]
    let new_branch = state.branches.last().unwrap();
    assert_eq!(new_branch.message_ids, vec![msg_a.id, msg_b.id]);
    assert_eq!(new_branch.parent_message_id, Some(msg_b.id));
    assert_eq!(new_branch.leaf_message_id, Some(msg_b.id));
}

#[test]
fn test_switch_branch_reducer() {
    let mut state = UserState::new(Uuid::new_v4());
    let new_branch_id = Uuid::new_v4();

    let action = UserStateAction::Branch(BranchAction::Switch(new_branch_id));
    state = apply_user_state_action(&state, action).unwrap();

    assert_eq!(state.active_branch_id, new_branch_id);
}

#[test]
fn test_add_message_appends_to_message_ids() {
    let mut state = UserState::new(Uuid::new_v4());
    let session_id = Uuid::new_v4();

    // Initial branch should have empty message_ids
    assert_eq!(state.branches[0].message_ids, Vec::<Uuid>::new());

    // Add first message
    let msg_a = create_test_message(session_id, None);
    let action = UserStateAction::Message(MessageAction::Add(msg_a.clone()));
    state = apply_user_state_action(&state, action).unwrap();

    // Branch should now have [A]
    assert_eq!(state.branches[0].message_ids, vec![msg_a.id]);
    assert_eq!(state.branches[0].leaf_message_id, Some(msg_a.id));

    // Add second message
    let msg_b = create_test_message(session_id, Some(msg_a.id));
    let action = UserStateAction::Message(MessageAction::Add(msg_b.clone()));
    state = apply_user_state_action(&state, action).unwrap();

    // Branch should now have [A, B]
    assert_eq!(state.branches[0].message_ids, vec![msg_a.id, msg_b.id]);
    assert_eq!(state.branches[0].leaf_message_id, Some(msg_b.id));

    // Add third message
    let msg_c = create_test_message(session_id, Some(msg_b.id));
    let action = UserStateAction::Message(MessageAction::Add(msg_c.clone()));
    state = apply_user_state_action(&state, action).unwrap();

    // Branch should now have [A, B, C]
    assert_eq!(
        state.branches[0].message_ids,
        vec![msg_a.id, msg_b.id, msg_c.id]
    );
    assert_eq!(state.branches[0].leaf_message_id, Some(msg_c.id));
}

#[test]
fn test_delete_message_preserves_other_branches() {
    // This test verifies the fix for the original bug:
    // Branch 1: A→B, Branch 2: A→C
    // Delete C from Branch 2, switch to Branch 1, delete B
    // Both branches should still show A
    let mut state = UserState::new(Uuid::new_v4());
    let session_id = Uuid::new_v4();

    // Create message A in initial branch
    let msg_a = create_test_message(session_id, None);
    state = apply_user_state_action(
        &state,
        UserStateAction::Message(MessageAction::Add(msg_a.clone())),
    )
    .unwrap();

    // Create message B in initial branch (now A→B)
    let msg_b = create_test_message(session_id, Some(msg_a.id));
    state = apply_user_state_action(
        &state,
        UserStateAction::Message(MessageAction::Add(msg_b.clone())),
    )
    .unwrap();
    let branch1_id = state.active_branch_id;

    // Branch after A to create Branch 2
    state = apply_user_state_action(
        &state,
        UserStateAction::Branch(BranchAction::Create(msg_a.id)),
    )
    .unwrap();
    let branch2_id = state.active_branch_id;

    // Create message C in Branch 2 (now A→C)
    let msg_c = create_test_message(session_id, Some(msg_a.id));
    state = apply_user_state_action(
        &state,
        UserStateAction::Message(MessageAction::Add(msg_c.clone())),
    )
    .unwrap();

    // Delete C from Branch 2
    state = apply_user_state_action(
        &state,
        UserStateAction::Message(MessageAction::Delete(msg_c.id)),
    )
    .unwrap();

    // Branch 2 should have [A]
    let branch2 = state.branches.iter().find(|b| b.id == branch2_id).unwrap();
    assert_eq!(branch2.message_ids, vec![msg_a.id]);
    assert_eq!(branch2.leaf_message_id, Some(msg_a.id));

    // Switch to Branch 1 (A→B)
    state = apply_user_state_action(
        &state,
        UserStateAction::Branch(BranchAction::Switch(branch1_id)),
    )
    .unwrap();

    // Delete B from Branch 1
    state = apply_user_state_action(
        &state,
        UserStateAction::Message(MessageAction::Delete(msg_b.id)),
    )
    .unwrap();

    // Branch 1 should still have [A]
    let branch1 = state.branches.iter().find(|b| b.id == branch1_id).unwrap();
    assert_eq!(branch1.message_ids, vec![msg_a.id]);
    assert_eq!(branch1.leaf_message_id, Some(msg_a.id));

    // Branch 2 should still have [A]
    let branch2 = state.branches.iter().find(|b| b.id == branch2_id).unwrap();
    assert_eq!(branch2.message_ids, vec![msg_a.id]);
    assert_eq!(branch2.leaf_message_id, Some(msg_a.id));

    // Verify they are distinct branches
    assert_ne!(branch1_id, branch2_id);
}

#[test]
fn test_delete_branch_reducer() {
    let mut state = UserState::new(Uuid::new_v4());

    let msg = create_test_message(Uuid::new_v4(), None);
    Arc::make_mut(&mut state.conversation_history).push(msg.clone());

    Arc::make_mut(&mut state.branches).push(ConversationBranch::new(
        None,
        Some("ToDelete".to_string()),
        Some(msg.id),
        Some(Dialect::SpanishMexican),
        vec![msg.id],
    ));
    let branch = state.branches.last().unwrap();
    let branch_id = branch.id;

    let initial_message_count = state.conversation_history.len();
    let action = UserStateAction::Branch(BranchAction::Delete(branch_id));
    state = apply_user_state_action(&state, action).unwrap();

    assert!(!state.branches.iter().any(|b| b.id == branch_id));
    assert_eq!(state.conversation_history.len(), initial_message_count);
}

#[test]
fn test_delete_active_branch_switches_to_first() {
    let mut state = UserState::new(Uuid::new_v4());
    let first_branch_id = state.branches.first().unwrap().id;

    let new_branch = ConversationBranch::new(
        None,
        Some("NewBranch".to_string()),
        None,
        Some(Dialect::SpanishMexican),
        vec![],
    );
    let new_branch_id = new_branch.id;
    Arc::make_mut(&mut state.branches).push(new_branch);
    state.active_branch_id = new_branch_id;

    let action = UserStateAction::Branch(BranchAction::Delete(new_branch_id));
    state = apply_user_state_action(&state, action).unwrap();

    assert_eq!(state.active_branch_id, first_branch_id);
}

#[test]
fn test_rename_branch_reducer() {
    let mut state = UserState::new(Uuid::new_v4());
    let branch_id = state.branches.first().unwrap().id;
    let new_name = "Renamed Branch".to_string();

    let action = UserStateAction::Branch(BranchAction::Rename(branch_id, new_name.clone()));
    state = apply_user_state_action(&state, action).unwrap();

    let branch = state.branches.iter().find(|b| b.id == branch_id).unwrap();
    assert_eq!(branch.name, Some(new_name));
}

#[test]
fn test_add_message_updates_leaf() {
    let mut state = UserState::new(Uuid::new_v4());
    let active_branch_id = state.active_branch_id;

    let msg1 = create_test_message(Uuid::new_v4(), None);
    let action1 = UserStateAction::Message(MessageAction::Add(msg1.clone()));
    state = apply_user_state_action(&state, action1).unwrap();

    let added_msg1 = state.conversation_history.last().unwrap();
    assert_eq!(added_msg1.parent_id, None);
    let msg1_id = added_msg1.id;

    let branch = state
        .branches
        .iter()
        .find(|b| b.id == active_branch_id)
        .unwrap();
    assert_eq!(branch.leaf_message_id, Some(msg1_id));

    let msg2 = create_test_message(Uuid::new_v4(), None);
    let action2 = UserStateAction::Message(MessageAction::Add(msg2));
    state = apply_user_state_action(&state, action2).unwrap();

    let added_msg2 = state.conversation_history.last().unwrap();
    assert_eq!(added_msg2.parent_id, Some(msg1_id));

    let branch = state
        .branches
        .iter()
        .find(|b| b.id == active_branch_id)
        .unwrap();
    assert_eq!(branch.leaf_message_id, Some(added_msg2.id));
}

#[test]
fn test_simple_branch_deletion() {
    let session_id = Uuid::new_v4();
    let mut state = UserState::new(session_id);

    // Create A→B
    let msg_a = create_test_message(session_id, None);
    Arc::make_mut(&mut state.conversation_history).push(msg_a.clone());
    let msg_b = create_test_message(session_id, Some(msg_a.id));
    Arc::make_mut(&mut state.conversation_history).push(msg_b.clone());

    // Update initial branch to point to B
    Arc::make_mut(&mut state.branches)[0].leaf_message_id = Some(msg_b.id);

    // Create branch C→D from B
    let branch_cd = ConversationBranch::new(
        Some(msg_b.id),
        Some("C+D".to_string()),
        None,
        Some(Dialect::SpanishMexican),
        vec![],
    );
    let branch_cd_id = branch_cd.id;
    Arc::make_mut(&mut state.branches).push(branch_cd);

    let msg_c = create_test_message(session_id, Some(msg_b.id));
    Arc::make_mut(&mut state.conversation_history).push(msg_c.clone());
    let msg_d = create_test_message(session_id, Some(msg_c.id));
    Arc::make_mut(&mut state.conversation_history).push(msg_d.clone());

    Arc::make_mut(&mut state.branches)
        .iter_mut()
        .find(|b| b.id == branch_cd_id)
        .unwrap()
        .leaf_message_id = Some(msg_d.id);

    // Create branch X→Y from B (make active)
    let branch_xy = ConversationBranch::new(
        Some(msg_b.id),
        Some("X+Y".to_string()),
        None,
        Some(Dialect::SpanishMexican),
        vec![],
    );
    let branch_xy_id = branch_xy.id;
    Arc::make_mut(&mut state.branches).push(branch_xy);
    state.active_branch_id = branch_xy_id;

    let msg_x = create_test_message(session_id, Some(msg_b.id));
    Arc::make_mut(&mut state.conversation_history).push(msg_x.clone());
    let msg_y = create_test_message(session_id, Some(msg_x.id));
    Arc::make_mut(&mut state.conversation_history).push(msg_y.clone());

    Arc::make_mut(&mut state.branches)
        .iter_mut()
        .find(|b| b.id == branch_xy_id)
        .unwrap()
        .leaf_message_id = Some(msg_y.id);

    // Delete branch C+D
    let action = UserStateAction::Branch(BranchAction::Delete(branch_cd_id));
    state = apply_user_state_action(&state, action).unwrap();

    // Assert: All messages remain (A, B, C, D, X, Y); only branch metadata is deleted
    assert_eq!(state.conversation_history.len(), 6);
    assert!(state.conversation_history.iter().any(|m| m.id == msg_a.id));
    assert!(state.conversation_history.iter().any(|m| m.id == msg_b.id));
    assert!(state.conversation_history.iter().any(|m| m.id == msg_c.id));
    assert!(state.conversation_history.iter().any(|m| m.id == msg_d.id));
    assert!(state.conversation_history.iter().any(|m| m.id == msg_x.id));
    assert!(state.conversation_history.iter().any(|m| m.id == msg_y.id));
    assert!(!state.branches.iter().any(|b| b.id == branch_cd_id));
}

#[test]
fn test_complex_nested_branch_deletion() {
    let session_id = Uuid::new_v4();
    let mut state = UserState::new(session_id);

    // Create A→B→X→W
    let msg_a = create_test_message(session_id, None);
    Arc::make_mut(&mut state.conversation_history).push(msg_a.clone());
    let msg_b = create_test_message(session_id, Some(msg_a.id));
    Arc::make_mut(&mut state.conversation_history).push(msg_b.clone());
    let msg_x = create_test_message(session_id, Some(msg_b.id));
    Arc::make_mut(&mut state.conversation_history).push(msg_x.clone());
    let msg_w = create_test_message(session_id, Some(msg_x.id));
    Arc::make_mut(&mut state.conversation_history).push(msg_w.clone());

    Arc::make_mut(&mut state.branches)[0].leaf_message_id = Some(msg_w.id);

    // Create branch C→D from B
    let branch_cd = ConversationBranch::new(
        Some(msg_b.id),
        Some("C+D".to_string()),
        None,
        Some(Dialect::SpanishMexican),
        vec![],
    );
    let branch_cd_id = branch_cd.id;
    Arc::make_mut(&mut state.branches).push(branch_cd);

    let msg_c = create_test_message(session_id, Some(msg_b.id));
    Arc::make_mut(&mut state.conversation_history).push(msg_c.clone());
    let msg_d = create_test_message(session_id, Some(msg_c.id));
    Arc::make_mut(&mut state.conversation_history).push(msg_d.clone());

    Arc::make_mut(&mut state.branches)
        .iter_mut()
        .find(|b| b.id == branch_cd_id)
        .unwrap()
        .leaf_message_id = Some(msg_d.id);

    // Create branch Y from X
    let branch_y = ConversationBranch::new(
        Some(msg_x.id),
        Some("Y".to_string()),
        None,
        Some(Dialect::SpanishMexican),
        vec![],
    );
    let branch_y_id = branch_y.id;
    Arc::make_mut(&mut state.branches).push(branch_y);

    let msg_y = create_test_message(session_id, Some(msg_x.id));
    Arc::make_mut(&mut state.conversation_history).push(msg_y.clone());

    Arc::make_mut(&mut state.branches)
        .iter_mut()
        .find(|b| b.id == branch_y_id)
        .unwrap()
        .leaf_message_id = Some(msg_y.id);

    // Delete branch Y
    let action = UserStateAction::Branch(BranchAction::Delete(branch_y_id));
    state = apply_user_state_action(&state, action).unwrap();

    // Assert: All messages remain (A, B, C, D, X, W, Y); only branch metadata deleted
    assert_eq!(state.conversation_history.len(), 7);
    assert!(state.conversation_history.iter().any(|m| m.id == msg_a.id));
    assert!(state.conversation_history.iter().any(|m| m.id == msg_b.id));
    assert!(state.conversation_history.iter().any(|m| m.id == msg_c.id));
    assert!(state.conversation_history.iter().any(|m| m.id == msg_d.id));
    assert!(state.conversation_history.iter().any(|m| m.id == msg_x.id));
    assert!(state.conversation_history.iter().any(|m| m.id == msg_w.id));
    assert!(state.conversation_history.iter().any(|m| m.id == msg_y.id));
    assert!(!state.branches.iter().any(|b| b.id == branch_y_id));

    // Delete branch C+D
    let action2 = UserStateAction::Branch(BranchAction::Delete(branch_cd_id));
    state = apply_user_state_action(&state, action2).unwrap();

    // Assert: All messages remain (A, B, C, D, X, W, Y); both branch metadata deleted
    assert_eq!(state.conversation_history.len(), 7);
    assert!(state.conversation_history.iter().any(|m| m.id == msg_a.id));
    assert!(state.conversation_history.iter().any(|m| m.id == msg_b.id));
    assert!(state.conversation_history.iter().any(|m| m.id == msg_c.id));
    assert!(state.conversation_history.iter().any(|m| m.id == msg_d.id));
    assert!(state.conversation_history.iter().any(|m| m.id == msg_x.id));
    assert!(state.conversation_history.iter().any(|m| m.id == msg_w.id));
    assert!(state.conversation_history.iter().any(|m| m.id == msg_y.id));
    assert!(!state.branches.iter().any(|b| b.id == branch_cd_id));
}

#[test]
fn test_delete_branch_with_sub_branches() {
    let session_id = Uuid::new_v4();
    let mut state = UserState::new(session_id);

    // Create A→B→C
    let msg_a = create_test_message(session_id, None);
    Arc::make_mut(&mut state.conversation_history).push(msg_a.clone());
    let msg_b = create_test_message(session_id, Some(msg_a.id));
    Arc::make_mut(&mut state.conversation_history).push(msg_b.clone());
    let msg_c = create_test_message(session_id, Some(msg_b.id));
    Arc::make_mut(&mut state.conversation_history).push(msg_c.clone());

    Arc::make_mut(&mut state.branches)[0].leaf_message_id = Some(msg_c.id);

    // Create branch D→E→F from C
    let branch_def = ConversationBranch::new(
        Some(msg_c.id),
        Some("D+E+F".to_string()),
        None,
        Some(Dialect::SpanishMexican),
        vec![],
    );
    let branch_def_id = branch_def.id;
    Arc::make_mut(&mut state.branches).push(branch_def);

    let msg_d = create_test_message(session_id, Some(msg_c.id));
    Arc::make_mut(&mut state.conversation_history).push(msg_d.clone());
    let msg_e = create_test_message(session_id, Some(msg_d.id));
    Arc::make_mut(&mut state.conversation_history).push(msg_e.clone());
    let msg_f = create_test_message(session_id, Some(msg_e.id));
    Arc::make_mut(&mut state.conversation_history).push(msg_f.clone());

    Arc::make_mut(&mut state.branches)
        .iter_mut()
        .find(|b| b.id == branch_def_id)
        .unwrap()
        .leaf_message_id = Some(msg_f.id);

    // Delete branch D→E→F
    let action = UserStateAction::Branch(BranchAction::Delete(branch_def_id));
    state = apply_user_state_action(&state, action).unwrap();

    // Assert: All messages remain (A, B, C, D, E, F); only branch metadata deleted
    assert_eq!(state.conversation_history.len(), 6);
    assert!(state.conversation_history.iter().any(|m| m.id == msg_a.id));
    assert!(state.conversation_history.iter().any(|m| m.id == msg_b.id));
    assert!(state.conversation_history.iter().any(|m| m.id == msg_c.id));
    assert!(state.conversation_history.iter().any(|m| m.id == msg_d.id));
    assert!(state.conversation_history.iter().any(|m| m.id == msg_e.id));
    assert!(state.conversation_history.iter().any(|m| m.id == msg_f.id));
    assert!(!state.branches.iter().any(|b| b.id == branch_def_id));
}

#[test]
fn test_delete_inactive_branch_preserves_active() {
    let session_id = Uuid::new_v4();
    let mut state = UserState::new(session_id);

    // Create A→B
    let msg_a = create_test_message(session_id, None);
    Arc::make_mut(&mut state.conversation_history).push(msg_a.clone());
    let msg_b = create_test_message(session_id, Some(msg_a.id));
    Arc::make_mut(&mut state.conversation_history).push(msg_b.clone());

    Arc::make_mut(&mut state.branches)[0].leaf_message_id = Some(msg_b.id);

    // Create branch C→D from B (inactive)
    let branch_cd = ConversationBranch::new(
        Some(msg_b.id),
        Some("Inactive".to_string()),
        None,
        Some(Dialect::SpanishMexican),
        vec![],
    );
    let branch_cd_id = branch_cd.id;
    Arc::make_mut(&mut state.branches).push(branch_cd);

    let msg_c = create_test_message(session_id, Some(msg_b.id));
    Arc::make_mut(&mut state.conversation_history).push(msg_c.clone());
    let msg_d = create_test_message(session_id, Some(msg_c.id));
    Arc::make_mut(&mut state.conversation_history).push(msg_d.clone());

    Arc::make_mut(&mut state.branches)
        .iter_mut()
        .find(|b| b.id == branch_cd_id)
        .unwrap()
        .leaf_message_id = Some(msg_d.id);

    // Create branch X→Y from B (make active)
    let branch_xy = ConversationBranch::new(
        Some(msg_b.id),
        Some("Active".to_string()),
        None,
        Some(Dialect::SpanishMexican),
        vec![],
    );
    let branch_xy_id = branch_xy.id;
    Arc::make_mut(&mut state.branches).push(branch_xy);
    state.active_branch_id = branch_xy_id;

    let msg_x = create_test_message(session_id, Some(msg_b.id));
    Arc::make_mut(&mut state.conversation_history).push(msg_x.clone());
    let msg_y = create_test_message(session_id, Some(msg_x.id));
    Arc::make_mut(&mut state.conversation_history).push(msg_y.clone());

    Arc::make_mut(&mut state.branches)
        .iter_mut()
        .find(|b| b.id == branch_xy_id)
        .unwrap()
        .leaf_message_id = Some(msg_y.id);

    // Get active branch messages before deletion
    let active_before: Vec<_> = state
        .get_active_branch_messages()
        .iter()
        .map(|m| m.id)
        .collect();

    // Delete the inactive C→D branch
    let action = UserStateAction::Branch(BranchAction::Delete(branch_cd_id));
    state = apply_user_state_action(&state, action).unwrap();

    // Get active branch messages after deletion
    let active_after: Vec<_> = state
        .get_active_branch_messages()
        .iter()
        .map(|m| m.id)
        .collect();

    // Assert: Active branch messages unchanged, all messages remain
    assert_eq!(active_before, active_after);
    assert_eq!(state.active_branch_id, branch_xy_id);
    assert!(state.conversation_history.iter().any(|m| m.id == msg_a.id));
    assert!(state.conversation_history.iter().any(|m| m.id == msg_b.id));
    assert!(state.conversation_history.iter().any(|m| m.id == msg_c.id));
    assert!(state.conversation_history.iter().any(|m| m.id == msg_d.id));
    assert!(state.conversation_history.iter().any(|m| m.id == msg_x.id));
    assert!(state.conversation_history.iter().any(|m| m.id == msg_y.id));
}

#[test]
fn test_update_usage_stats_preserves_conversation_branches() {
    let mut state = UserState::new(Uuid::new_v4());
    let msg = create_test_message(Uuid::new_v4(), None);
    state = apply_user_state_action(
        &state,
        UserStateAction::Message(MessageAction::Add(msg.clone())),
    )
    .unwrap();
    let history_len = state.conversation_history.len();
    let branch_ids: Vec<Uuid> = state.branches.iter().map(|b| b.id).collect();

    let updated = apply_user_state_action(
        &state,
        UserStateAction::UpdateUsageStats(UsageStats::default()),
    )
    .unwrap();

    assert_eq!(updated.conversation_history.len(), history_len);
    let updated_ids: Vec<Uuid> = updated.branches.iter().map(|b| b.id).collect();
    assert_eq!(updated_ids, branch_ids);
}

#[test]
fn test_add_score_accumulates() {
    assert_eq!(add_score(10, 8), 18);
    assert_eq!(add_score(50, 10), 60);
}

#[test]
fn test_add_score_caps_at_100() {
    assert_eq!(add_score(95, 10), 100);
    assert_eq!(add_score(100, 10), 100);
}

#[test]
fn test_add_score_handles_negative() {
    assert_eq!(add_score(20, -10), 10);
    assert_eq!(add_score(15, -10), 5);
}

#[test]
fn test_add_score_floors_at_zero() {
    assert_eq!(add_score(0, -5), 0);
    assert_eq!(add_score(3, -10), 0);
}

#[test]
fn test_cycle_formality() {
    assert_eq!(
        cycle_formality(Formality::Formal),
        Formality::ProfessionalCasual
    );
    assert_eq!(
        cycle_formality(Formality::ProfessionalCasual),
        Formality::Informal
    );
    assert_eq!(cycle_formality(Formality::Informal), Formality::Slang);
    assert_eq!(cycle_formality(Formality::Slang), Formality::Formal);
}

#[test]
fn test_cycle_teaching_mode() {
    assert_eq!(
        cycle_teaching_mode(TeachingMode::Immersive, true),
        TeachingMode::Corrective
    );
    assert_eq!(
        cycle_teaching_mode(TeachingMode::Corrective, true),
        TeachingMode::Explanatory
    );
    assert_eq!(
        cycle_teaching_mode(TeachingMode::Explanatory, true),
        TeachingMode::StoryTeller
    );
    assert_eq!(
        cycle_teaching_mode(TeachingMode::StoryTeller, true),
        TeachingMode::ErrorFinding
    );
    assert_eq!(
        cycle_teaching_mode(TeachingMode::ErrorFinding, true),
        TeachingMode::Debug
    );
    assert_eq!(
        cycle_teaching_mode(TeachingMode::Debug, true),
        TeachingMode::Immersive
    );
}

#[test]
fn test_cycle_dialect_action() {
    let mut state = UserState::new(Uuid::new_v4());
    state.selected_language = Language::Spanish;
    state.selected_dialect = Dialect::SpanishMexican;
    state.show_experimental_dialects = true;

    let action = UserStateAction::Settings(SettingsAction::CycleDialect);
    state = apply_user_state_action(&state, action).unwrap();

    assert_ne!(state.selected_dialect, Dialect::SpanishMexican);
}

#[test]
fn test_change_dialect_creates_branch() {
    let mut state = UserState::new(Uuid::new_v4());
    state.selected_language = Language::Spanish;
    state.selected_dialect = Dialect::SpanishMexican;

    let initial_branch_count = state.branches.len();
    let initial_branch_id = state.active_branch_id;

    // Change to a different Spanish dialect
    let action = UserStateAction::Settings(SettingsAction::ChangeDialect(Dialect::SpanishArgentinian));
    state = apply_user_state_action(&state, action).unwrap();

    // Verify new branch was created
    assert_eq!(state.branches.len(), initial_branch_count + 1);
    assert_ne!(state.active_branch_id, initial_branch_id);
    assert_eq!(state.selected_dialect, Dialect::SpanishArgentinian);

    // Verify new branch has the new dialect
    let new_branch = state.branches.iter().find(|b| b.id == state.active_branch_id).unwrap();
    assert_eq!(new_branch.dialect, Some(Dialect::SpanishArgentinian));
}

#[test]
fn test_cycle_formality_action() {
    let mut state = UserState::new(Uuid::new_v4());
    state.formality = Formality::Formal;

    let action = UserStateAction::Settings(SettingsAction::CycleFormality);
    state = apply_user_state_action(&state, action).unwrap();

    assert_eq!(state.formality, Formality::ProfessionalCasual);
}

#[test]
fn test_cycle_teaching_mode_action() {
    let mut state = UserState::new(Uuid::new_v4());
    state.teaching_mode = TeachingMode::Immersive;

    let action = UserStateAction::Settings(SettingsAction::CycleTeachingMode(true));
    state = apply_user_state_action(&state, action).unwrap();

    assert_eq!(state.teaching_mode, TeachingMode::Corrective);
}
#[test]
fn test_language_plan_reducers() {
    use dialect_coach_shared::models::{
        LanguagePlan, PlanContent, PlanStatus, PlanStep, StepStatus, StepType,
    };

    let mut state = UserState::new(Uuid::new_v4());
    let plan_id = Uuid::new_v4();
    let plan = LanguagePlan {
        id: plan_id,
        title: "Test Plan".to_string(),
        dialect: Dialect::SpanishMexican,
        description: None,
        steps: vec![
            PlanStep::new(
                1,
                "Welcome!".to_string(),
                StepType::Learning {
                    content: PlanContent::default(),
                },
                "Msg".to_string(),
            ),
            PlanStep::new(
                2,
                "Step 2".to_string(),
                StepType::Learning {
                    content: PlanContent::default(),
                },
                "Msg".to_string(),
            ),
        ],
        current_step_index: 0,
        status: PlanStatus::NotStarted,
        created_at: 0,
    };

    // Test AddLanguagePlan
    state = apply_user_state_action(&state, UserStateAction::Plan(PlanAction::Add(plan.clone())))
        .unwrap();
    assert_eq!(state.language_plans.len(), 1);
    assert_eq!(state.language_plans[0].id, plan_id);

    // Test SetActivePlan
    state = apply_user_state_action(
        &state,
        UserStateAction::Plan(PlanAction::SetActive(Some(plan_id))),
    )
    .unwrap();
    assert_eq!(state.active_plan_id, Some(plan_id));
    // Should auto-start
    assert_eq!(state.language_plans[0].status, PlanStatus::InProgress);
    assert_eq!(
        state.language_plans[0].current_step().unwrap().status,
        StepStatus::InProgress
    );

    // Test AdvancePlanStep
    state = apply_user_state_action(
        &state,
        UserStateAction::Plan(PlanAction::AdvanceStep(plan_id)),
    )
    .unwrap();
    assert_eq!(state.language_plans[0].current_step_index, 1);
    assert_eq!(
        state.language_plans[0].steps[0].status,
        StepStatus::Completed
    );
    assert_eq!(
        state.language_plans[0].steps[1].status,
        StepStatus::InProgress
    );

    // Test DeleteLanguagePlan
    state = apply_user_state_action(&state, UserStateAction::Plan(PlanAction::Delete(plan_id)))
        .unwrap();
    assert!(state.language_plans.is_empty());
    assert_eq!(state.active_plan_id, None);
}

#[test]
fn test_update_language_level_action() {
    let mut state = UserState::new(Uuid::new_v4());
    state.selected_dialect = Dialect::SpanishMexican;

    let action = UserStateAction::Settings(SettingsAction::UpdateLevel(LanguageLevel::Cefr(
        CefrLevel::C1,
    )));
    let updated = apply_user_state_action(&state, action).unwrap();

    assert_eq!(
        updated.get_level_for_dialect(&Dialect::SpanishMexican),
        LanguageLevel::Cefr(CefrLevel::C1)
    );
}
