use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{
    ConversationBranch, Dialect, Explained, Exploratory, Formality, Language, Message, MessageContent, MessageMetadata, Mistake,
    TeachingMode, Translated,
};

/// User-specific state that persists across sessions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserState {
    pub user_id: Uuid,
    pub learning_items: Vec<LearningItem>,
    pub conversation_history: Vec<Message>,
    pub tts_enabled: bool,
    pub selected_language: Language,
    pub selected_dialect: Dialect,
    pub formality: Formality,
    pub teaching_mode: TeachingMode,
    pub active_branch_id: Uuid,
    pub branches: Vec<ConversationBranch>,
}

/// A learning item with its associated mastery score
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LearningItem {
    pub item: LearningItemType,
    pub score: u8,
}

/// Types of learning items that can be tracked
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LearningItemType {
    Mistake(Mistake),
    Explanation(Explained),
    Translation(Translated),
    Exploration(Exploratory),
}

impl UserState {
    /// Create a new UserState with default values for a given user
    pub fn new(user_id: Uuid) -> Self {
        let initial_branch = ConversationBranch::new(None, None, None);
        let initial_branch_id = initial_branch.id;

        Self {
            user_id,
            learning_items: Vec::new(),
            conversation_history: Vec::new(),
            tts_enabled: false,
            selected_language: Language::Spanish,
            selected_dialect: Dialect::SpanishCuban,
            formality: Formality::Casual,
            teaching_mode: TeachingMode::Immersive,
            active_branch_id: initial_branch_id,
            branches: vec![initial_branch],
        }
    }

    pub fn create_msg(&self, session_id: Uuid, content: MessageContent) -> Message {
        let parent_id = self.get_last_message_in_active_branch();
        Message::new(
            content,
            MessageMetadata::at_now(
                self.formality,
                self.teaching_mode,
                self.selected_language,
                self.current_dialect(),
                session_id,
            ),
            parent_id,
        )

    }

    pub fn create_user_msg(&self, session_id: Uuid, content: &String) -> Message {
        self.create_msg(session_id, MessageContent::UserMessage { content: content.clone() })
    }

    fn get_last_message_in_active_branch(&self) -> Option<Uuid> {
        self.branches
            .iter()
            .find(|b| b.id == self.active_branch_id)
            .and_then(|b| b.leaf_message_id)
    }

    pub fn bcp47_tag(&self) -> String {
        self.selected_dialect.bcp47_tag().to_string()
    }

    pub fn current_dialect(&self) -> Dialect {
        self.selected_dialect
    }

    pub fn current_dialects(&self) -> Vec<Dialect> {
        Dialect::for_language(self.selected_language)
    }

    pub fn formality_display(&self) -> &'static str {
        match self.formality {
            Formality::Formal => "Formal",
            Formality::Casual => "Casual",
            Formality::DialectRich => "Dialect-Rich",
            Formality::Slang => "Slang",
        }
    }

    pub fn teaching_mode_display(&self) -> &'static str {
        match self.teaching_mode {
            TeachingMode::Immersive => "Immersive",
            TeachingMode::Corrective => "Corrective",
            TeachingMode::Explanatory => "Explanatory",
            TeachingMode::Interleaved => "Interleaved",
            TeachingMode::StoryTeller => "Storyteller",
            TeachingMode::Debug => "Debug",
        }
    }

    pub fn get_active_branch_messages(&self) -> Vec<&Message> {
        let leaf_id = self.branches
            .iter()
            .find(|b| b.id == self.active_branch_id)
            .and_then(|b| b.leaf_message_id);

        self.get_path_to_message(leaf_id)
    }

    fn get_path_to_message(&self, leaf_id: Option<Uuid>) -> Vec<&Message> {
        let mut path = Vec::new();
        let mut current = leaf_id;

        while let Some(msg_id) = current {
            if let Some(msg) = self.conversation_history.iter().find(|m| m.id == msg_id) {
                path.push(msg);
                current = msg.parent_id;
            } else {
                break;
            }
        }

        path.reverse();
        path
    }

    pub fn get_child_branches(&self, message_id: Uuid) -> Vec<&ConversationBranch> {
        self.branches
            .iter()
            .filter(|branch| branch.parent_message_id == Some(message_id))
            .collect()
    }

    pub fn find_branch_root(&self, branch_id: Uuid) -> Option<Uuid> {
        self.branches
            .iter()
            .find(|branch| branch.id == branch_id)
            .and_then(|branch| branch.parent_message_id)
    }
}

impl LearningItem {
    /// Create a new learning item with score 0
    pub fn new(item: LearningItemType) -> Self {
        Self { item, score: 0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::agent::{MistakeCategory};

    fn test_metadata(session_id: Uuid) -> MessageMetadata {
        MessageMetadata::at_now(Formality::Casual, TeachingMode::Immersive, Language::Spanish, Dialect::SpanishArgentinian, session_id)
    }

    fn create_test_user_state() -> UserState {
        UserState::new(Uuid::new_v4())
    }

    fn create_test_mistake() -> Mistake {
        Mistake::new(
            "hablar".to_string(),
            "habla".to_string(),
            MistakeCategory::SpellingError {
                context: "context".to_string(),
            },
        )
    }

    fn create_test_message() -> Message {
        Message::new(
            MessageContent::UserMessage { content: "test".to_string() },
            test_metadata(Uuid::new_v4()),
            None,
        )
    }

    #[test]
    fn test_user_state_serialization() {
        let state = create_test_user_state();
        let json = serde_json::to_string(&state).unwrap();

        assert!(json.contains("\"user_id\""));
        assert!(json.contains("\"learning_items\""));
        assert!(json.contains("\"conversation_history\""));
        assert!(json.contains("\"tts_enabled\""));
    }

    #[test]
    fn test_user_state_deserialization() {
        let state = create_test_user_state();
        let json = serde_json::to_string(&state).unwrap();
        let deserialized: UserState = serde_json::from_str(&json).unwrap();

        assert_eq!(state.user_id, deserialized.user_id);
        assert_eq!(state.learning_items, deserialized.learning_items);
        assert_eq!(state.tts_enabled, deserialized.tts_enabled);
    }

    #[test]
    fn test_conversation_history_in_user_state() {
        let mut state = create_test_user_state();
        let msg = create_test_message();
        state.conversation_history.push(msg.clone());

        assert_eq!(state.conversation_history.len(), 1);
        assert_eq!(state.conversation_history[0].id, msg.id);
    }

    #[test]
    fn test_learning_item_new() {
        let mistake = create_test_mistake();
        let item = LearningItem::new(LearningItemType::Mistake(mistake.clone()));

        assert_eq!(item.score, 0);
        match item.item {
            LearningItemType::Mistake(m) => assert_eq!(m.id, mistake.id),
            _ => panic!("Expected Mistake variant"),
        }
    }

    #[test]
    fn test_user_state_new() {
        let user_id = Uuid::new_v4();
        let state = UserState::new(user_id);

        assert_eq!(state.user_id, user_id);
        assert_eq!(state.learning_items.len(), 0);
        assert_eq!(state.conversation_history.len(), 0);
        assert_eq!(state.tts_enabled, false);
    }

    #[test]
    fn test_learning_item_serialization() {
        let mistake = create_test_mistake();
        let item = LearningItem::new(LearningItemType::Mistake(mistake));
        let json = serde_json::to_string(&item).unwrap();

        assert!(json.contains("\"score\""));
        assert!(json.contains("\"item\""));
    }

    #[test]
    fn test_get_active_branch_messages_empty() {
        let state = create_test_user_state();
        let messages = state.get_active_branch_messages();
        assert_eq!(messages.len(), 0);
    }

    #[test]
    fn test_get_active_branch_messages_single() {
        let mut state = create_test_user_state();
        let msg = Message::new(
            MessageContent::UserMessage { content: "test".to_string() },
            test_metadata(Uuid::new_v4()),
            None,
        );
        state.conversation_history.push(msg.clone());

        // Set the branch's leaf to this message
        if let Some(branch) = state.branches.iter_mut().find(|b| b.id == state.active_branch_id) {
            branch.leaf_message_id = Some(msg.id);
        }

        let messages = state.get_active_branch_messages();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].id, msg.id);
    }

    #[test]
    fn test_get_active_branch_messages_chain() {
        let mut state = create_test_user_state();

        // Create a chain: A → B → C
        let msg_a = Message::new(
            MessageContent::UserMessage { content: "A".to_string() },
            test_metadata(Uuid::new_v4()),
            None,
        );
        state.conversation_history.push(msg_a.clone());

        let msg_b = Message::new(
            MessageContent::UserMessage { content: "B".to_string() },
            test_metadata(Uuid::new_v4()),
            Some(msg_a.id),
        );
        state.conversation_history.push(msg_b.clone());

        let msg_c = Message::new(
            MessageContent::UserMessage { content: "C".to_string() },
            test_metadata(Uuid::new_v4()),
            Some(msg_b.id),
        );
        state.conversation_history.push(msg_c.clone());

        // Set the branch's leaf to msg_c
        if let Some(branch) = state.branches.iter_mut().find(|b| b.id == state.active_branch_id) {
            branch.leaf_message_id = Some(msg_c.id);
        }

        let messages = state.get_active_branch_messages();
        assert_eq!(messages.len(), 3);
        // Messages should be in order: A, B, C (root to leaf)
        assert_eq!(messages[0].id, msg_a.id);
        assert_eq!(messages[1].id, msg_b.id);
        assert_eq!(messages[2].id, msg_c.id);
        assert_eq!(messages[0].get_content(), "A");
        assert_eq!(messages[1].get_content(), "B");
        assert_eq!(messages[2].get_content(), "C");
    }

    #[test]
    fn test_get_active_branch_messages_excludes_other_branch() {
        let mut state = create_test_user_state();

        // Create main path: A → B
        let msg_a = Message::new(
            MessageContent::UserMessage { content: "A".to_string() },
            test_metadata(Uuid::new_v4()),
            None,
        );
        state.conversation_history.push(msg_a.clone());

        let msg_b = Message::new(
            MessageContent::UserMessage { content: "B".to_string() },
            test_metadata(Uuid::new_v4()),
            Some(msg_a.id),
        );
        state.conversation_history.push(msg_b.clone());

        // Create alternative path from A: A → X
        let msg_x = Message::new(
            MessageContent::UserMessage { content: "X".to_string() },
            test_metadata(Uuid::new_v4()),
            Some(msg_a.id),
        );
        state.conversation_history.push(msg_x.clone());

        // Set active branch leaf to B (so path is A → B, not A → X)
        if let Some(branch) = state.branches.iter_mut().find(|b| b.id == state.active_branch_id) {
            branch.leaf_message_id = Some(msg_b.id);
        }

        let messages = state.get_active_branch_messages();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].id, msg_a.id);
        assert_eq!(messages[1].id, msg_b.id);
        // msg_x should NOT be in the result
        assert!(!messages.iter().any(|m| m.id == msg_x.id));
    }

    #[test]
    fn test_get_child_branches_none() {
        let state = create_test_user_state();
        let message_id = Uuid::new_v4();
        let branches = state.get_child_branches(message_id);
        assert_eq!(branches.len(), 0);
    }

    #[test]
    fn test_get_child_branches_one() {
        let mut state = create_test_user_state();
        let message_id = Uuid::new_v4();
        let branch = ConversationBranch::new(Some(message_id), None, None);
        let branch_id = branch.id;
        state.branches.push(branch);

        let child_branches = state.get_child_branches(message_id);
        assert_eq!(child_branches.len(), 1);
        assert_eq!(child_branches[0].id, branch_id);
    }

    #[test]
    fn test_find_branch_root_exists() {
        let mut state = create_test_user_state();
        let parent_msg_id = Uuid::new_v4();
        let branch = ConversationBranch::new(Some(parent_msg_id), None, None);
        let branch_id = branch.id;
        state.branches.push(branch);

        let root = state.find_branch_root(branch_id);
        assert_eq!(root, Some(parent_msg_id));
    }

    #[test]
    fn test_find_branch_root_not_found() {
        let state = create_test_user_state();
        let fake_branch_id = Uuid::new_v4();
        let root = state.find_branch_root(fake_branch_id);
        assert_eq!(root, None);
    }
}
