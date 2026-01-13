use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::Dialect;

/// A branch in the conversation tree
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConversationBranch {
    pub id: Uuid,
    pub parent_message_id: Option<Uuid>,
    pub leaf_message_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub name: Option<String>,
    pub dialect: Dialect,
    #[serde(default)]
    pub message_ids: Vec<Uuid>,
    #[serde(default)]
    pub active_plan_id: Option<Uuid>,
}

impl ConversationBranch {
    pub fn new(
        parent_message_id: Option<Uuid>,
        name: Option<String>,
        leaf_message_id: Option<Uuid>,
        dialect: Dialect,
        message_ids: Vec<Uuid>,
        active_plan_id: Option<Uuid>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            parent_message_id,
            leaf_message_id,
            message_ids,
            created_at: Utc::now(),
            name,
            dialect,
            active_plan_id,
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
            Dialect::SpanishMexican,
            vec![],
            None,
        );

        assert_eq!(branch.parent_message_id, Some(parent_id));
        assert_eq!(branch.name, None);
        assert_eq!(branch.leaf_message_id, None);
        assert_eq!(branch.dialect, Dialect::SpanishMexican);
        assert_eq!(branch.message_ids, Vec::<Uuid>::new());
    }

    #[test]
    fn test_new_branch_with_name() {
        let name = "Alternative discussion".to_string();
        let branch = ConversationBranch::new(
            None,
            Some(name.clone()),
            None,
            Dialect::SpanishMexican,
            vec![],
            None,
        );

        assert_eq!(branch.parent_message_id, None);
        assert_eq!(branch.name, Some(name));
        assert_eq!(branch.leaf_message_id, None);
        assert_eq!(branch.dialect, Dialect::SpanishMexican);
        assert_eq!(branch.message_ids, Vec::<Uuid>::new());
    }

    #[test]
    fn test_new_branch_root() {
        let branch = ConversationBranch::new(None, None, None, Dialect::SpanishMexican, vec![], None);

        assert_eq!(branch.parent_message_id, None);
        assert_eq!(branch.name, None);
        assert_eq!(branch.leaf_message_id, None);
        assert_eq!(branch.dialect, Dialect::SpanishMexican);
        assert_eq!(branch.message_ids, Vec::<Uuid>::new());
    }

    #[test]
    fn test_branch_serialization() {
        let parent_id = Uuid::new_v4();
        let branch = ConversationBranch::new(
            Some(parent_id),
            Some("Test Branch".to_string()),
            None,
            Dialect::SpanishMexican,
            vec![],
            None,
        );

        let json = serde_json::to_string(&branch).unwrap();
        assert!(json.contains(&branch.id.to_string()));
        assert!(json.contains(&parent_id.to_string()));
        assert!(json.contains("Test Branch"));
        assert!(json.contains("message_ids"));

        let deserialized: ConversationBranch = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, branch);
    }
}
