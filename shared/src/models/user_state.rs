use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{
    Dialect, Explained, Exploratory, Formality, Language, Message, Mistake, TeachingMode,
    Translated,
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
        Self {
            user_id,
            learning_items: Vec::new(),
            conversation_history: Vec::new(),
            tts_enabled: false,
            selected_language: Language::Spanish,
            selected_dialect: Dialect::SpanishCuban,
            formality: Formality::Casual,
            teaching_mode: TeachingMode::Immersive,
        }
    }

    pub fn create_msg(&self, session_id: Uuid, content: &String) -> Message {
        let agent_response = super::AgentResponse::from(content);
        Message::new(
            session_id,
            "user".to_string(),
            agent_response,
            self.bcp47_tag(),
            self.formality,
            self.teaching_mode,
        )
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
    use crate::models::agent::{AgentResponse, MistakeCategory};

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
            Uuid::new_v4(),
            "user".to_string(),
            AgentResponse::from("test"),
            "es-MX".to_string(),
            Formality::Casual,
            TeachingMode::Immersive,
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
}
