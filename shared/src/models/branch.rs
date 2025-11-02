use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A branch in the conversation tree
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConversationBranch {
    pub id: Uuid,
    pub parent_message_id: Option<Uuid>,
    pub leaf_message_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub name: Option<String>,
}

impl ConversationBranch {
    pub fn new(
        parent_message_id: Option<Uuid>,
        name: Option<String>,
        leaf_message_id: Option<Uuid>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            parent_message_id,
            leaf_message_id,
            created_at: Utc::now(),
            name,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_branch_with_parent() {
        let parent_id = Uuid::new_v4();
        let branch = ConversationBranch::new(Some(parent_id), None, None);

        assert_eq!(branch.parent_message_id, Some(parent_id));
        assert_eq!(branch.name, None);
        assert_eq!(branch.leaf_message_id, None);
    }

    #[test]
    fn test_new_branch_with_name() {
        let name = "Alternative discussion".to_string();
        let branch = ConversationBranch::new(None, Some(name.clone()), None);

        assert_eq!(branch.parent_message_id, None);
        assert_eq!(branch.name, Some(name));
        assert_eq!(branch.leaf_message_id, None);
    }

    #[test]
    fn test_new_branch_root() {
        let branch = ConversationBranch::new(None, None, None);

        assert_eq!(branch.parent_message_id, None);
        assert_eq!(branch.name, None);
        assert_eq!(branch.leaf_message_id, None);
    }

    #[test]
    fn test_branch_serialization() {
        let parent_id = Uuid::new_v4();
        let branch =
            ConversationBranch::new(Some(parent_id), Some("Test Branch".to_string()), None);

        let json = serde_json::to_string(&branch).unwrap();
        assert!(json.contains(&branch.id.to_string()));
        assert!(json.contains(&parent_id.to_string()));
        assert!(json.contains("Test Branch"));

        let deserialized: ConversationBranch = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, branch);
    }
}
