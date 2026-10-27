use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{ConversationContext, LearningGoal, Message, UserState};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivityDefinition {
    pub activity_id: Uuid,
    pub title: String,
    pub shared_context: String,
    pub learner_brief: String,
    pub partner_brief: String,
    pub goals: Vec<LearningGoal>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActivityLifecycle {
    Active,
    Closed { closed_at: DateTime<Utc> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivitySnapshot {
    pub snapshot_id: Uuid,
    pub definition: ActivityDefinition,
    pub lifecycle: ActivityLifecycle,
    pub last_checked_message_id: Option<Uuid>,
}

impl ActivitySnapshot {
    pub fn new(definition: ActivityDefinition) -> Self {
        Self {
            snapshot_id: Uuid::new_v4(),
            definition,
            lifecycle: ActivityLifecycle::Active,
            last_checked_message_id: None,
        }
    }

    pub fn is_active(&self) -> bool {
        matches!(self.lifecycle, ActivityLifecycle::Active)
    }

    pub fn unchecked(&self) -> Self {
        let mut next = self.clone();
        next.snapshot_id = Uuid::new_v4();
        next.last_checked_message_id = None;
        for goal in &mut next.definition.goals {
            goal.completed = None;
        }
        next
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivityContext {
    pub branch_id: Uuid,
    pub snapshot: ActivitySnapshot,
    pub activity_message_ids: Vec<Uuid>,
}

impl ActivityContext {
    /// IDs must identify an ordered subsequence of the supplied conversation.
    pub fn dialogue<'a>(&self, messages: &'a [Message]) -> Result<Vec<&'a Message>, &'static str> {
        if !self.snapshot.is_active() {
            return Err("Activity is closed");
        }
        let mut cursor = 0;
        let mut selected = Vec::new();
        for id in &self.activity_message_ids {
            let offset = messages[cursor..]
                .iter()
                .position(|m| m.id == *id)
                .ok_or("Invalid activity dialogue")?;
            cursor += offset;
            selected.push(&messages[cursor]);
            cursor += 1;
        }
        Ok(selected)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateActivityRequest {
    pub user_id: Uuid,
    pub branch_id: Uuid,
    pub description: Option<String>,
    pub context: ConversationContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluateActivityRequest {
    pub user_id: Uuid,
    pub context: ConversationContext,
}

impl UserState {
    pub fn current_activity(&self, branch_id: Uuid) -> Option<&ActivitySnapshot> {
        self.branches
            .iter()
            .find(|b| b.id == branch_id)?
            .activity_snapshots
            .last()
    }

    pub fn activity_snapshot(
        &self,
        branch_id: Uuid,
        snapshot_id: Uuid,
    ) -> Option<&ActivitySnapshot> {
        self.branches
            .iter()
            .find(|b| b.id == branch_id)?
            .activity_snapshots
            .iter()
            .find(|s| s.snapshot_id == snapshot_id)
    }

    pub fn activity_at_message(
        &self,
        branch_id: Uuid,
        message: &Message,
    ) -> Option<&ActivitySnapshot> {
        self.activity_snapshot(branch_id, message.activity_snapshot_id?)
    }

    pub fn activity_context(
        &self,
        branch_id: Uuid,
        messages: &[Message],
    ) -> Option<ActivityContext> {
        let snapshot = self.current_activity(branch_id)?.clone();
        let activity_message_ids = messages
            .iter()
            .filter(|message| {
                self.activity_at_message(branch_id, message)
                    .is_some_and(|s| {
                        s.definition.activity_id == snapshot.definition.activity_id && s.is_active()
                    })
            })
            .map(|m| m.id)
            .collect();
        Some(ActivityContext {
            branch_id,
            snapshot,
            activity_message_ids,
        })
    }

    pub fn build_branch_action_context(
        &self,
        branch_id: Uuid,
        messages: Vec<Message>,
    ) -> Option<ConversationContext> {
        let branch = self.branches.iter().find(|b| b.id == branch_id)?;
        let mut selected = self.clone();
        selected.active_branch_id = branch_id;
        selected.selected_language = branch.dialect.language();
        Some(selected.build_action_context(messages))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AgentResponse, Dialect};

    fn definition() -> ActivityDefinition {
        ActivityDefinition {
            activity_id: Uuid::new_v4(),
            title: "Choose a route".into(),
            shared_context: "Travel together".into(),
            learner_brief: "Find an accessible route".into(),
            partner_brief: "The bridge is closed".into(),
            goals: vec![LearningGoal {
                goal: "Agree on a route".into(),
                dialect: Dialect::SpanishMexican,
                completed: None,
            }],
        }
    }

    #[test]
    fn dialogue_is_ordered_and_rejects_missing_or_duplicate_ids() {
        let user = UserState::new(Uuid::new_v4());
        let first = user.create_user_msg(Uuid::new_v4(), "¿Por dónde?");
        let second = Message::agent_message(
            AgentResponse::from("Por el parque"),
            first.metadata.clone(),
            Some(first.id),
        );
        let mut context = ActivityContext {
            branch_id: user.active_branch_id,
            snapshot: ActivitySnapshot::new(definition()),
            activity_message_ids: vec![first.id, second.id],
        };
        let messages = vec![first.clone(), second.clone()];
        assert_eq!(
            context
                .dialogue(&messages)
                .unwrap()
                .iter()
                .map(|m| m.id)
                .collect::<Vec<_>>(),
            context.activity_message_ids
        );
        context.activity_message_ids.reverse();
        assert!(context.dialogue(&messages).is_err());
        context.activity_message_ids = vec![first.id, first.id];
        assert!(context.dialogue(&messages).is_err());
        context.activity_message_ids = vec![Uuid::new_v4()];
        assert!(context.dialogue(&messages).is_err());
    }

    #[test]
    fn snapshots_round_trip_without_changing_prior_goals() {
        let original = ActivitySnapshot::new(definition());
        let mut next = original.unchecked();
        next.definition.goals[0].goal = "  別の道\nを決める  ".into();
        next.definition.goals[0].completed = Some(true);
        next.last_checked_message_id = Some(Uuid::new_v4());
        let encoded = serde_json::to_string(&vec![original.clone(), next.clone()]).unwrap();
        let decoded: Vec<ActivitySnapshot> = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, vec![original, next]);
        assert_ne!(decoded[0].snapshot_id, decoded[1].snapshot_id);
        assert_eq!(decoded[0].definition.goals[0].completed, None);
        assert_eq!(decoded[1].definition.goals[0].goal, "  別の道\nを決める  ");
    }

    #[test]
    fn old_browser_state_and_goal_load_with_defaults() {
        let user = UserState::new(Uuid::new_v4());
        let mut encoded = serde_json::to_value(&user).unwrap();
        for branch in encoded["branches"].as_array_mut().unwrap() {
            branch.as_object_mut().unwrap().remove("activity_snapshots");
        }
        let decoded: UserState = serde_json::from_value(encoded).unwrap();
        assert!(decoded.current_activity(decoded.active_branch_id).is_none());
        let goal: LearningGoal =
            serde_json::from_value(serde_json::json!({"goal":"Talk", "dialect":"spanish_mexican"}))
                .unwrap();
        assert_eq!(goal.completed, None);
        let message = user.create_user_msg(Uuid::new_v4(), "Hola");
        let mut request = serde_json::to_value(
            crate::UserMessageWithContext::builder(user.user_id, message).build(),
        )
        .unwrap();
        request.as_object_mut().unwrap().remove("activity");
        request["message"]
            .as_object_mut()
            .unwrap()
            .remove("activity_snapshot_id");
        let decoded: crate::UserMessageWithContext = serde_json::from_value(request).unwrap();
        assert!(decoded.activity.is_none());
        assert!(decoded.message.activity_snapshot_id.is_none());
    }
}
