use super::{
    AgentResponse, Dialect, Explained, Formality, Language, Mistake, TeachingMode, UsageStats,
    User, UserState,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MessageContent {
    UserMessage { content: String },
    AgentMessage { content: Box<AgentResponse> },
}

/// A message in a chat session
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub id: Uuid,
    pub parent_id: Option<Uuid>,
    pub metadata: MessageMetadata,
    pub content: MessageContent,
}

/// Metadata associated with a message
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessageMetadata {
    pub formality: Formality,
    pub teaching_mode: TeachingMode,
    pub language: Language,
    pub dialect: Dialect,
    pub timestamp: DateTime<Utc>,
    pub session_id: Uuid,
}

impl MessageMetadata {
    pub fn new(
        formality: Formality,
        teaching_mode: TeachingMode,
        language: Language,
        dialect: Dialect,
        timestamp: DateTime<Utc>,
        session_id: Uuid,
    ) -> Self {
        Self {
            formality,
            teaching_mode,
            language,
            dialect,
            timestamp,
            session_id,
        }
    }

    pub fn at_now(
        formality: Formality,
        teaching_mode: TeachingMode,
        language: Language,
        dialect: Dialect,
        session_id: Uuid,
    ) -> Self {
        Self::new(
            formality,
            teaching_mode,
            language,
            dialect,
            Utc::now(),
            session_id,
        )
    }
}

impl Message {
    pub fn user_message(
        content: String,
        metadata: MessageMetadata,
        parent_id: Option<Uuid>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            content: MessageContent::UserMessage { content },
            metadata,
            parent_id,
        }
    }

    pub fn agent_message(
        content: AgentResponse,
        metadata: MessageMetadata,
        parent_id: Option<Uuid>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            content: MessageContent::AgentMessage {
                content: Box::new(content),
            },
            metadata,
            parent_id,
        }
    }

    pub fn get_content(&self) -> String {
        match self.content.clone() {
            MessageContent::UserMessage { content } => content,
            MessageContent::AgentMessage { content } => content.response,
        }
    }

    pub fn is_agent(&self) -> bool {
        matches!(self.content, MessageContent::AgentMessage { .. })
    }
}

/// Past learning items grouped together
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct PastLearningItems {
    pub mistakes: Vec<Mistake>,
    pub explained: Vec<Explained>,
    pub translated: Vec<crate::models::agent::Translated>,
    pub exploratory: Vec<crate::models::agent::Exploratory>,
}

/// User message with past learning items for context
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserMessageWithContext {
    pub user_id: Uuid,
    pub message: Message,
    pub past_mistakes: Vec<Mistake>,
    pub past_explained: Vec<Explained>,
    pub past_translated: Vec<crate::models::agent::Translated>,
    pub past_exploratory: Vec<crate::models::agent::Exploratory>,
    pub active_branch_id: Uuid,
    pub context_messages: Vec<Message>,
    pub learning_goals: Vec<String>,
}

impl UserMessageWithContext {
    pub fn new(
        user_id: Uuid,
        message: Message,
        past_learning_items: PastLearningItems,
        active_branch_id: Uuid,
        context_messages: Vec<Message>,
        learning_goals: Vec<String>,
    ) -> Self {
        Self {
            user_id,
            message,
            past_mistakes: past_learning_items.mistakes,
            past_explained: past_learning_items.explained,
            past_translated: past_learning_items.translated,
            past_exploratory: past_learning_items.exploratory,
            active_branch_id,
            context_messages,
            learning_goals,
        }
    }
}

/// WebSocket messages for UserState persistence
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UserStateMessage {
    /// Request to save user state to backend
    Save(UserState),
    /// Request to load user state by user_id
    Load(Uuid),
    /// Response to save request (Ok = success, Err = error message)
    SaveResponse(Result<(), String>),
    /// Response to load request (Some = found, None = not found)
    LoadResponse(Option<UserState>),
    /// Update usage stats (sent after saving usage stats)
    UsageStatsUpdate(UsageStats),
}

/// WebSocket messages for User management
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UserMessage {
    /// Request to create a new user (Client → Server)
    CreateUser { user_id: Uuid, username: String },
    /// Response to create user request (Server → Client)
    CreateUserResponse(Result<User, String>),
    /// Request to sign in with username (Client → Server)
    SignIn { username: String },
    /// Response to sign in request (Server → Client)
    SignInResponse(Result<User, String>),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_metadata(session_id: Uuid) -> MessageMetadata {
        MessageMetadata::at_now(
            Formality::Casual,
            TeachingMode::Immersive,
            Language::Spanish,
            Dialect::SpanishArgentinian,
            session_id,
        )
    }

    #[test]
    fn test_new_message() {
        let session_id = Uuid::new_v4();
        let msg = Message::user_message("Hello".to_string(), test_metadata(session_id), None);

        assert_eq!(msg.metadata.session_id, session_id);
        assert_eq!(msg.get_content(), "Hello");
        assert_eq!(msg.metadata.formality, Formality::Casual);
        assert_eq!(msg.metadata.teaching_mode, TeachingMode::Immersive);
    }

    #[test]
    fn test_metadata_serialization() {
        let msg = Message::user_message("Hola".to_string(), test_metadata(Uuid::new_v4()), None);

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"formality\":\"casual\""));
        assert!(json.contains("\"teaching_mode\":\"immersive\""));

        // Verify metadata fields are not null
        assert!(!json.contains("\"formality\":null"));
        assert!(!json.contains("\"teaching_mode\":null"));
    }

    #[test]
    fn test_user_message_with_context_basic() {
        let msg = Message::user_message("Hola".to_string(), test_metadata(Uuid::new_v4()), None);

        let user_id = Uuid::new_v4();
        let branch_id = Uuid::new_v4();
        let context = UserMessageWithContext::new(
            user_id,
            msg.clone(),
            PastLearningItems::default(),
            branch_id,
            vec![],
            vec![],
        );

        assert_eq!(context.message.id, msg.id);
        assert_eq!(context.past_mistakes.len(), 0);
        assert_eq!(context.past_explained.len(), 0);
        assert_eq!(context.past_translated.len(), 0);
        assert_eq!(context.past_exploratory.len(), 0);
        assert_eq!(context.active_branch_id, branch_id);
        assert_eq!(context.context_messages.len(), 0);
    }

    #[test]
    fn test_user_message_with_context_with_learning_items() {
        use crate::models::agent::{Explained, Mistake, MistakeCategory};

        let msg = Message::user_message("Hola".to_string(), test_metadata(Uuid::new_v4()), None);

        let mistake = Mistake::new(
            "hablar".to_string(),
            "habla".to_string(),
            MistakeCategory::SpellingError {
                context: "habla".to_string(),
            },
        );

        let explained = Explained::new("órale".to_string(), "Mexican slang".to_string());

        let user_id = Uuid::new_v4();
        let branch_id = Uuid::new_v4();
        let context = UserMessageWithContext::new(
            user_id,
            msg.clone(),
            PastLearningItems {
                mistakes: vec![mistake.clone()],
                explained: vec![explained.clone()],
                translated: vec![],
                exploratory: vec![],
            },
            branch_id,
            vec![],
            vec![],
        );

        assert_eq!(context.past_mistakes.len(), 1);
        assert_eq!(context.past_mistakes[0].id, mistake.id);
        assert_eq!(context.past_explained.len(), 1);
        assert_eq!(context.past_explained[0].id, explained.id);
        assert_eq!(context.past_translated.len(), 0);
        assert_eq!(context.past_exploratory.len(), 0);
    }

    #[test]
    fn test_user_message_with_context_serialization() {
        use crate::models::agent::{Mistake, MistakeCategory};

        let msg = Message::user_message("Hola".to_string(), test_metadata(Uuid::new_v4()), None);

        let mistake = Mistake::new(
            "hablar".to_string(),
            "habla".to_string(),
            MistakeCategory::SpellingError {
                context: "habla".to_string(),
            },
        );

        let user_id = Uuid::new_v4();
        let branch_id = Uuid::new_v4();
        let context = UserMessageWithContext::new(
            user_id,
            msg,
            PastLearningItems {
                mistakes: vec![mistake],
                explained: vec![],
                translated: vec![],
                exploratory: vec![],
            },
            branch_id,
            vec![],
            vec![],
        );

        let json = serde_json::to_string(&context).unwrap();
        assert!(json.contains("\"message\""));
        assert!(json.contains("\"past_mistakes\""));
        assert!(json.contains("\"past_explained\""));
        assert!(json.contains("\"past_translated\""));
        assert!(json.contains("\"past_exploratory\""));
        assert!(json.contains("\"active_branch_id\""));
        assert!(json.contains("\"context_messages\""));
        assert!(json.contains("hablar"));

        let deserialized: UserMessageWithContext = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.past_mistakes.len(), 1);
        assert_eq!(deserialized.past_translated.len(), 0);
        assert_eq!(deserialized.past_exploratory.len(), 0);
    }

    #[test]
    fn test_user_message_with_context_with_translated() {
        use crate::models::agent::Translated;

        let msg = Message::user_message("Hola".to_string(), test_metadata(Uuid::new_v4()), None);

        let translated = Translated::new("hello".to_string(), "hola".to_string());

        let user_id = Uuid::new_v4();
        let branch_id = Uuid::new_v4();
        let context = UserMessageWithContext::new(
            user_id,
            msg.clone(),
            PastLearningItems {
                mistakes: vec![],
                explained: vec![],
                translated: vec![translated.clone()],
                exploratory: vec![],
            },
            branch_id,
            vec![],
            vec![],
        );

        assert_eq!(context.past_translated.len(), 1);
        assert_eq!(context.past_translated[0].id, translated.id);
    }

    #[test]
    fn test_user_message_with_context_with_exploratory() {
        use crate::models::agent::Exploratory;

        let msg =
            Message::user_message("Try this".to_string(), test_metadata(Uuid::new_v4()), None);

        let exploratory =
            Exploratory::new("Use subjunctive".to_string(), "Try 'Si fuera'".to_string());

        let user_id = Uuid::new_v4();
        let branch_id = Uuid::new_v4();
        let context = UserMessageWithContext::new(
            user_id,
            msg.clone(),
            PastLearningItems {
                mistakes: vec![],
                explained: vec![],
                translated: vec![],
                exploratory: vec![exploratory.clone()],
            },
            branch_id,
            vec![],
            vec![],
        );

        assert_eq!(context.past_exploratory.len(), 1);
        assert_eq!(context.past_exploratory[0].id, exploratory.id);
    }

    #[test]
    fn test_user_message_with_context_all_four_types() {
        use crate::models::agent::{Explained, Exploratory, Mistake, MistakeCategory, Translated};

        let msg = Message::user_message("Test".to_string(), test_metadata(Uuid::new_v4()), None);

        let mistake = Mistake::new(
            "hablar".to_string(),
            "habla".to_string(),
            MistakeCategory::SpellingError {
                context: "habla".to_string(),
            },
        );

        let explained = Explained::new("órale".to_string(), "Mexican slang".to_string());

        let translated = Translated::new("hello".to_string(), "hola".to_string());

        let exploratory =
            Exploratory::new("Use subjunctive".to_string(), "Try 'Si fuera'".to_string());

        let user_id = Uuid::new_v4();
        let branch_id = Uuid::new_v4();
        let context = UserMessageWithContext::new(
            user_id,
            msg.clone(),
            PastLearningItems {
                mistakes: vec![mistake.clone()],
                explained: vec![explained.clone()],
                translated: vec![translated.clone()],
                exploratory: vec![exploratory.clone()],
            },
            branch_id,
            vec![],
            vec![],
        );

        assert_eq!(context.past_mistakes.len(), 1);
        assert_eq!(context.past_explained.len(), 1);
        assert_eq!(context.past_translated.len(), 1);
        assert_eq!(context.past_exploratory.len(), 1);

        let json = serde_json::to_string(&context).unwrap();
        let deserialized: UserMessageWithContext = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.past_mistakes.len(), 1);
        assert_eq!(deserialized.past_explained.len(), 1);
        assert_eq!(deserialized.past_translated.len(), 1);
        assert_eq!(deserialized.past_exploratory.len(), 1);
    }

    #[test]
    fn test_user_state_message_save_serialization() {
        use crate::models::UserState;

        let user_id = Uuid::new_v4();
        let state = UserState::new(user_id);
        let msg = UserStateMessage::Save(state.clone());

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"Save\""));
        assert!(json.contains(&user_id.to_string()));

        let deserialized: UserStateMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, msg);
    }

    #[test]
    fn test_user_state_message_load_serialization() {
        let user_id = Uuid::new_v4();
        let msg = UserStateMessage::Load(user_id);

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"Load\""));
        assert!(json.contains(&user_id.to_string()));

        let deserialized: UserStateMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, msg);
    }

    #[test]
    fn test_user_state_message_save_response_ok() {
        let msg = UserStateMessage::SaveResponse(Ok(()));

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"SaveResponse\""));
        assert!(json.contains("\"Ok\""));

        let deserialized: UserStateMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, msg);
    }

    #[test]
    fn test_user_state_message_save_response_err() {
        let error_msg = "Database connection failed".to_string();
        let msg = UserStateMessage::SaveResponse(Err(error_msg.clone()));

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"SaveResponse\""));
        assert!(json.contains("\"Err\""));
        assert!(json.contains(&error_msg));

        let deserialized: UserStateMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, msg);
    }

    #[test]
    fn test_user_state_message_load_response_some() {
        use crate::models::UserState;

        let user_id = Uuid::new_v4();
        let state = UserState::new(user_id);
        let msg = UserStateMessage::LoadResponse(Some(state.clone()));

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"LoadResponse\""));
        assert!(json.contains(&user_id.to_string()));

        let deserialized: UserStateMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, msg);
    }

    #[test]
    fn test_user_state_message_load_response_none() {
        let msg = UserStateMessage::LoadResponse(None);

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"LoadResponse\""));
        assert!(json.contains("null"));

        let deserialized: UserStateMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, msg);
    }

    #[test]
    fn test_user_message_create_user_serialization() {
        let user_id = Uuid::new_v4();
        let username = "testuser".to_string();
        let msg = UserMessage::CreateUser {
            user_id,
            username: username.clone(),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"CreateUser\""));
        assert!(json.contains(&user_id.to_string()));
        assert!(json.contains("testuser"));

        let deserialized: UserMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, msg);
    }

    #[test]
    fn test_user_message_create_user_response_ok() {
        let user = User::new(Uuid::new_v4(), "alice".to_string());
        let msg = UserMessage::CreateUserResponse(Ok(user.clone()));

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"CreateUserResponse\""));
        assert!(json.contains("\"Ok\""));
        assert!(json.contains("alice"));

        let deserialized: UserMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, msg);
    }

    #[test]
    fn test_user_message_create_user_response_err() {
        let error_msg = "Username already exists".to_string();
        let msg = UserMessage::CreateUserResponse(Err(error_msg.clone()));

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"CreateUserResponse\""));
        assert!(json.contains("\"Err\""));
        assert!(json.contains(&error_msg));

        let deserialized: UserMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, msg);
    }

    #[test]
    fn test_user_message_sign_in_serialization() {
        let username = "bob".to_string();
        let msg = UserMessage::SignIn {
            username: username.clone(),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"SignIn\""));
        assert!(json.contains("bob"));

        let deserialized: UserMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, msg);
    }

    #[test]
    fn test_user_message_sign_in_response_ok() {
        let user = User::new(Uuid::new_v4(), "charlie".to_string());
        let msg = UserMessage::SignInResponse(Ok(user.clone()));

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"SignInResponse\""));
        assert!(json.contains("\"Ok\""));
        assert!(json.contains("charlie"));

        let deserialized: UserMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, msg);
    }

    #[test]
    fn test_user_message_sign_in_response_err() {
        let error_msg = "User not found".to_string();
        let msg = UserMessage::SignInResponse(Err(error_msg.clone()));

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"SignInResponse\""));
        assert!(json.contains("\"Err\""));
        assert!(json.contains(&error_msg));

        let deserialized: UserMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, msg);
    }
}
