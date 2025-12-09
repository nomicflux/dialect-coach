use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::Dialect;

/// A branch in the conversation tree
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConversationBranch {
    pub id: Uuid,
    pub parent_message_id: Option<Uuid>,
    pub leaf_message_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub name: Option<String>,
    pub dialect: Option<Dialect>,
    #[serde(default)]
    pub message_ids: Vec<Uuid>,
}

impl ConversationBranch {
    pub fn new(
        parent_message_id: Option<Uuid>,
        name: Option<String>,
        leaf_message_id: Option<Uuid>,
        dialect: Option<Dialect>,
        message_ids: Vec<Uuid>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            parent_message_id,
            leaf_message_id,
            message_ids,
            created_at: Utc::now(),
            name,
            dialect,
        }
    }

    pub fn set_dialect_if_none(&mut self, dialect: Dialect) {
        if self.dialect.is_none() {
            self.dialect = Some(dialect);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_branch_with_parent() {
        let parent_id = Uuid::new_v4();
        let branch = ConversationBranch::new(
            Some(parent_id),
            None,
            None,
            Some(Dialect::SpanishMexican),
            vec![],
        );

        assert_eq!(branch.parent_message_id, Some(parent_id));
        assert_eq!(branch.name, None);
        assert_eq!(branch.leaf_message_id, None);
        assert_eq!(branch.dialect, Some(Dialect::SpanishMexican));
        assert_eq!(branch.message_ids, Vec::<Uuid>::new());
    }

    #[test]
    fn test_new_branch_with_name() {
        let name = "Alternative discussion".to_string();
        let branch = ConversationBranch::new(
            None,
            Some(name.clone()),
            None,
            Some(Dialect::SpanishMexican),
            vec![],
        );

        assert_eq!(branch.parent_message_id, None);
        assert_eq!(branch.name, Some(name));
        assert_eq!(branch.leaf_message_id, None);
        assert_eq!(branch.dialect, Some(Dialect::SpanishMexican));
        assert_eq!(branch.message_ids, Vec::<Uuid>::new());
    }

    #[test]
    fn test_new_branch_root() {
        let branch = ConversationBranch::new(None, None, None, None, vec![]);

        assert_eq!(branch.parent_message_id, None);
        assert_eq!(branch.name, None);
        assert_eq!(branch.leaf_message_id, None);
        assert_eq!(branch.dialect, None);
        assert_eq!(branch.message_ids, Vec::<Uuid>::new());
    }

    #[test]
    fn test_branch_serialization() {
        let parent_id = Uuid::new_v4();
        let branch = ConversationBranch::new(
            Some(parent_id),
            Some("Test Branch".to_string()),
            None,
            Some(Dialect::SpanishMexican),
            vec![],
        );

        let json = serde_json::to_string(&branch).unwrap();
        assert!(json.contains(&branch.id.to_string()));
        assert!(json.contains(&parent_id.to_string()));
        assert!(json.contains("Test Branch"));
        assert!(json.contains("message_ids"));

        let deserialized: ConversationBranch = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, branch);
    }

    #[test]
    fn test_set_dialect_if_none_sets_when_none() {
        let mut branch = ConversationBranch::new(None, None, None, None, vec![]);
        assert_eq!(branch.dialect, None);

        branch.set_dialect_if_none(Dialect::SpanishMexican);
        assert_eq!(branch.dialect, Some(Dialect::SpanishMexican));
    }

    #[test]
    fn test_set_dialect_if_none_does_not_override() {
        let mut branch =
            ConversationBranch::new(None, None, None, Some(Dialect::SpanishCuban), vec![]);
        assert_eq!(branch.dialect, Some(Dialect::SpanishCuban));

        branch.set_dialect_if_none(Dialect::SpanishMexican);
        assert_eq!(branch.dialect, Some(Dialect::SpanishCuban));
    }
}
