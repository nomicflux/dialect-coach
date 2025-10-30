use super::{AgentResponse, Explained, Formality, Mistake, TeachingMode, User, UserState};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A message in a chat session
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub id: Uuid,
    pub session_id: Uuid,
    pub participant_id: String,
    pub content: AgentResponse,
    pub timestamp: DateTime<Utc>,
    pub language: String,
    pub metadata: MessageMetadata,
}

/// Metadata associated with a message
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessageMetadata {
    /// Desired formality level for agent responses
    pub formality: Formality,

    /// Teaching mode for agent behavior
    pub teaching_mode: TeachingMode,
}

impl MessageMetadata {
    pub fn new(formality: Formality, teaching_mode: TeachingMode) -> Self {
        Self {
            formality,
            teaching_mode,
        }
    }
}

impl Default for MessageMetadata {
    fn default() -> Self {
        Self {
            formality: Formality::Casual,
            teaching_mode: TeachingMode::Immersive,
        }
    }
}

impl Message {
    /// Create a new message
    pub fn new(
        session_id: Uuid,
        participant_id: String,
        content: AgentResponse,
        language: String,
        formality: Formality,
        teaching_mode: TeachingMode,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            session_id,
            participant_id,
            content,
            timestamp: Utc::now(),
            language,
            metadata: MessageMetadata::new(formality, teaching_mode),
        }
    }
}

/// User message with past learning items for context
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserMessageWithContext {
    pub message: Message,
    pub past_mistakes: Vec<Mistake>,
    pub past_explained: Vec<Explained>,
    pub past_translated: Vec<crate::models::agent::Translated>,
    pub past_exploratory: Vec<crate::models::agent::Exploratory>,
}

impl UserMessageWithContext {
    pub fn new(
        message: Message,
        past_mistakes: Vec<Mistake>,
        past_explained: Vec<Explained>,
        past_translated: Vec<crate::models::agent::Translated>,
        past_exploratory: Vec<crate::models::agent::Exploratory>,
    ) -> Self {
        Self {
            message,
            past_mistakes,
            past_explained,
            past_translated,
            past_exploratory,
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

    #[test]
    fn test_new_message() {
        let session_id = Uuid::new_v4();
        let content = AgentResponse::from("Hello!");
        let msg = Message::new(
            session_id,
            "user1".to_string(),
            content.clone(),
            "es-MX".to_string(),
            Formality::Casual,
            TeachingMode::Immersive,
        );

        assert_eq!(msg.session_id, session_id);
        assert_eq!(msg.participant_id, "user1");
        assert_eq!(msg.content, content);
        assert_eq!(msg.content.response, "Hello!");
        assert_eq!(msg.metadata.formality, Formality::Casual);
        assert_eq!(msg.metadata.teaching_mode, TeachingMode::Immersive);
    }

    #[test]
    fn test_metadata_serialization() {
        let content = AgentResponse::from("Hola");
        let msg = Message::new(
            Uuid::new_v4(),
            "user1".to_string(),
            content.clone(),
            "es-MX".to_string(),
            Formality::DialectRich,
            TeachingMode::Corrective,
        );

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"formality\":\"dialect_rich\""));
        assert!(json.contains("\"teaching_mode\":\"corrective\""));

        // Verify metadata fields are not null
        assert!(!json.contains("\"formality\":null"));
        assert!(!json.contains("\"teaching_mode\":null"));
    }

    #[test]
    fn test_user_message_with_context_basic() {
        let msg = Message::new(
            Uuid::new_v4(),
            "user1".to_string(),
            AgentResponse::from("Hola"),
            "es-MX".to_string(),
            Formality::Casual,
            TeachingMode::Immersive,
        );

        let context = UserMessageWithContext::new(msg.clone(), vec![], vec![], vec![], vec![]);

        assert_eq!(context.message.id, msg.id);
        assert_eq!(context.past_mistakes.len(), 0);
        assert_eq!(context.past_explained.len(), 0);
        assert_eq!(context.past_translated.len(), 0);
        assert_eq!(context.past_exploratory.len(), 0);
    }

    #[test]
    fn test_user_message_with_context_with_learning_items() {
        use crate::models::agent::{Explained, Mistake, MistakeCategory};

        let msg = Message::new(
            Uuid::new_v4(),
            "user1".to_string(),
            AgentResponse::from("Hola"),
            "es-MX".to_string(),
            Formality::Casual,
            TeachingMode::Immersive,
        );

        let mistake = Mistake::new(
            "hablar".to_string(),
            "habla".to_string(),
            MistakeCategory::SpellingError {
                context: "habla".to_string(),
            },
        );

        let explained = Explained::new(
            "órale".to_string(),
            "Mexican slang".to_string(),
        );

        let context = UserMessageWithContext::new(
            msg.clone(),
            vec![mistake.clone()],
            vec![explained.clone()],
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

        let msg = Message::new(
            Uuid::new_v4(),
            "user1".to_string(),
            AgentResponse::from("Hola"),
            "es-MX".to_string(),
            Formality::Casual,
            TeachingMode::Immersive,
        );

        let mistake = Mistake::new(
            "hablar".to_string(),
            "habla".to_string(),
            MistakeCategory::SpellingError {
                context: "habla".to_string(),
            },
        );

        let context = UserMessageWithContext::new(msg, vec![mistake], vec![], vec![], vec![]);

        let json = serde_json::to_string(&context).unwrap();
        assert!(json.contains("\"message\""));
        assert!(json.contains("\"past_mistakes\""));
        assert!(json.contains("\"past_explained\""));
        assert!(json.contains("\"past_translated\""));
        assert!(json.contains("\"past_exploratory\""));
        assert!(json.contains("hablar"));

        let deserialized: UserMessageWithContext = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.past_mistakes.len(), 1);
        assert_eq!(deserialized.past_translated.len(), 0);
        assert_eq!(deserialized.past_exploratory.len(), 0);
    }

    #[test]
    fn test_user_message_with_context_with_translated() {
        use crate::models::agent::Translated;

        let msg = Message::new(
            Uuid::new_v4(),
            "user1".to_string(),
            AgentResponse::from("Hola"),
            "es-MX".to_string(),
            Formality::Casual,
            TeachingMode::Immersive,
        );

        let translated = Translated::new(
            "hello".to_string(),
            "hola".to_string(),
        );

        let context = UserMessageWithContext::new(
            msg.clone(),
            vec![],
            vec![],
            vec![translated.clone()],
            vec![],
        );

        assert_eq!(context.past_translated.len(), 1);
        assert_eq!(context.past_translated[0].id, translated.id);
    }

    #[test]
    fn test_user_message_with_context_with_exploratory() {
        use crate::models::agent::Exploratory;

        let msg = Message::new(
            Uuid::new_v4(),
            "user1".to_string(),
            AgentResponse::from("Try this"),
            "es-MX".to_string(),
            Formality::Casual,
            TeachingMode::Immersive,
        );

        let exploratory = Exploratory::new(
            "Use subjunctive".to_string(),
            "Try 'Si fuera'".to_string(),
        );

        let context = UserMessageWithContext::new(
            msg.clone(),
            vec![],
            vec![],
            vec![],
            vec![exploratory.clone()],
        );

        assert_eq!(context.past_exploratory.len(), 1);
        assert_eq!(context.past_exploratory[0].id, exploratory.id);
    }

    #[test]
    fn test_user_message_with_context_all_four_types() {
        use crate::models::agent::{Exploratory, Explained, Mistake, MistakeCategory, Translated};

        let msg = Message::new(
            Uuid::new_v4(),
            "user1".to_string(),
            AgentResponse::from("Test"),
            "es-MX".to_string(),
            Formality::Casual,
            TeachingMode::Immersive,
        );

        let mistake = Mistake::new(
            "hablar".to_string(),
            "habla".to_string(),
            MistakeCategory::SpellingError {
                context: "habla".to_string(),
            },
        );

        let explained = Explained::new(
            "órale".to_string(),
            "Mexican slang".to_string(),
        );

        let translated = Translated::new(
            "hello".to_string(),
            "hola".to_string(),
        );

        let exploratory = Exploratory::new(
            "Use subjunctive".to_string(),
            "Try 'Si fuera'".to_string(),
        );

        let context = UserMessageWithContext::new(
            msg.clone(),
            vec![mistake.clone()],
            vec![explained.clone()],
            vec![translated.clone()],
            vec![exploratory.clone()],
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
