use super::{
    AgentResponse, AuthCredentials, Dialect, Explained, Formality, Language, LanguageOption,
    LearningGoal, Mistake, TeachingMode, UsageStats, User, UserGender, UserState,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MessageContent {
    UserMessage { content: String },
    AgentMessage { content: Box<AgentResponse> },
}

impl MessageContent {
    pub fn as_str(&self) -> &str {
        match self {
            Self::UserMessage { content } => content.as_str(),
            Self::AgentMessage { content } => (*content).as_str(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AIActionRequest {
    StartConversation,
    ContinueBranch { parent_message_id: Uuid },
    ExplainMessage { message_id: Uuid },
    TranslateMessage { message_id: Uuid },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhraseTranslation {
    pub target_text: String,
    pub english: String,
}

/// Request to translate a phrase to a dialect
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TranslateRequest {
    pub phrase: String,
    pub context: String,
    pub dialect: String,
    pub formality: Option<String>,
}

/// Response from translation endpoint
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TranslateResponse {
    pub original_sentence: String,
    pub segmented_phrases: Vec<PhraseTranslation>,
    pub success: bool,
    pub error: Option<String>,
}

/// A message in a chat session
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub id: Uuid,
    pub parent_id: Option<Uuid>,
    pub metadata: MessageMetadata,
    pub content: MessageContent,
}

impl Message {
    pub fn as_str(&self) -> &str {
        self.content.as_str()
    }
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
    pub learning_goals: Vec<LearningGoal>,
    pub user_gender: UserGender,
    pub language_option: Option<LanguageOption>,
}

pub struct UserMessageWithContextBuilder {
    user_id: Uuid,
    message: Message,
    past_learning_items: PastLearningItems,
    active_branch_id: Uuid,
    context_messages: Vec<Message>,
    learning_goals: Vec<LearningGoal>,
    user_gender: UserGender,
    language_option: Option<LanguageOption>,
}

impl UserMessageWithContextBuilder {
    pub fn new(user_id: Uuid, message: Message) -> Self {
        Self {
            user_id,
            message,
            past_learning_items: PastLearningItems::default(),
            active_branch_id: Uuid::nil(),
            context_messages: Vec::new(),
            learning_goals: Vec::new(),
            user_gender: UserGender::NonBinary,
            language_option: None,
        }
    }

    pub fn past_learning_items(mut self, items: PastLearningItems) -> Self {
        self.past_learning_items = items;
        self
    }

    pub fn active_branch_id(mut self, id: Uuid) -> Self {
        self.active_branch_id = id;
        self
    }

    pub fn context_messages(mut self, messages: Vec<Message>) -> Self {
        self.context_messages = messages;
        self
    }

    pub fn learning_goals(mut self, goals: Vec<LearningGoal>) -> Self {
        self.learning_goals = goals;
        self
    }

    pub fn user_gender(mut self, gender: UserGender) -> Self {
        self.user_gender = gender;
        self
    }

    pub fn language_option(mut self, option: Option<LanguageOption>) -> Self {
        self.language_option = option;
        self
    }

    pub fn build(self) -> UserMessageWithContext {
        UserMessageWithContext {
            user_id: self.user_id,
            message: self.message,
            past_mistakes: self.past_learning_items.mistakes,
            past_explained: self.past_learning_items.explained,
            past_translated: self.past_learning_items.translated,
            past_exploratory: self.past_learning_items.exploratory,
            active_branch_id: self.active_branch_id,
            context_messages: self.context_messages,
            learning_goals: self.learning_goals,
            user_gender: self.user_gender,
            language_option: self.language_option,
        }
    }
}

impl UserMessageWithContext {
    pub fn builder(user_id: Uuid, message: Message) -> UserMessageWithContextBuilder {
        UserMessageWithContextBuilder::new(user_id, message)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UserStateMessage {
    Save(UserState),
    Load(Uuid),
    SaveResponse(Result<(), String>),
    LoadResponse(Option<UserState>),
    UsageStatsUpdate(UsageStats),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UserMessage {
    /// Request to create a new user (Client → Server)
    CreateUser {
        username: String,
        email: String,
        credentials: AuthCredentials,
        password: String,
    },
    /// Response to create user request (Server → Client)
    /// Returns (User, JWT token) on success
    CreateUserResponse(Result<(User, String), String>),
    /// Request to sign in with username and password (Client → Server)
    SignIn {
        username: String,
        password: String,
    },
    /// Response to sign in request (Server → Client)
    /// Returns (User, JWT token) on success
    SignInResponse(Result<(User, String), String>),
    /// Request to validate a session token (Client → Server)
    ValidateSession { token: String },
    /// Response to session validation (Server → Client)
    ValidateSessionResponse(Result<User, String>),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_metadata(session_id: Uuid) -> MessageMetadata {
        MessageMetadata::at_now(
            Formality::Informal,
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
        assert_eq!(msg.metadata.formality, Formality::Informal);
        assert_eq!(msg.metadata.teaching_mode, TeachingMode::Immersive);
    }

    #[test]
    fn test_metadata_serialization() {
        let msg = Message::user_message("Hola".to_string(), test_metadata(Uuid::new_v4()), None);

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"formality\":\"informal\""));
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
        let context = UserMessageWithContext::builder(user_id, msg.clone())
            .past_learning_items(PastLearningItems::default())
            .active_branch_id(branch_id)
            .user_gender(UserGender::NonBinary)
            .build();

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
        let context = UserMessageWithContext::builder(user_id, msg.clone())
            .past_learning_items(PastLearningItems {
                mistakes: vec![mistake.clone()],
                explained: vec![explained.clone()],
                translated: vec![],
                exploratory: vec![],
            })
            .active_branch_id(branch_id)
            .user_gender(UserGender::NonBinary)
            .build();

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
        let context = UserMessageWithContext::builder(user_id, msg)
            .past_learning_items(PastLearningItems {
                mistakes: vec![mistake],
                explained: vec![],
                translated: vec![],
                exploratory: vec![],
            })
            .active_branch_id(branch_id)
            .user_gender(UserGender::NonBinary)
            .build();

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

        let translated = Translated::new("hello".to_string(), "hola".to_string(), None);

        let user_id = Uuid::new_v4();
        let branch_id = Uuid::new_v4();
        let context = UserMessageWithContext::builder(user_id, msg.clone())
            .past_learning_items(PastLearningItems {
                mistakes: vec![],
                explained: vec![],
                translated: vec![translated.clone()],
                exploratory: vec![],
            })
            .active_branch_id(branch_id)
            .user_gender(UserGender::NonBinary)
            .build();

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
        let context = UserMessageWithContext::builder(user_id, msg.clone())
            .past_learning_items(PastLearningItems {
                mistakes: vec![],
                explained: vec![],
                translated: vec![],
                exploratory: vec![exploratory.clone()],
            })
            .active_branch_id(branch_id)
            .user_gender(UserGender::NonBinary)
            .build();

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

        let translated = Translated::new("hello".to_string(), "hola".to_string(), None);

        let exploratory =
            Exploratory::new("Use subjunctive".to_string(), "Try 'Si fuera'".to_string());

        let user_id = Uuid::new_v4();
        let branch_id = Uuid::new_v4();
        let context = UserMessageWithContext::builder(user_id, msg.clone())
            .past_learning_items(PastLearningItems {
                mistakes: vec![mistake.clone()],
                explained: vec![explained.clone()],
                translated: vec![translated.clone()],
                exploratory: vec![exploratory.clone()],
            })
            .active_branch_id(branch_id)
            .user_gender(UserGender::NonBinary)
            .build();

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
        let username = "testuser".to_string();
        let email = "test@example.com".to_string();
        let credentials = AuthCredentials::InviteCode("CODE123".to_string());
        let password = "secure_password".to_string();
        let msg = UserMessage::CreateUser {
            username: username.clone(),
            email: email.clone(),
            credentials: credentials.clone(),
            password: password.clone(),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"CreateUser\""));
        assert!(json.contains("testuser"));
        assert!(json.contains("test@example.com"));
        assert!(json.contains("secure_password"));

        let deserialized: UserMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, msg);
    }

    #[test]
    fn test_user_message_create_user_response_ok() {
        let user = User::new(
            Uuid::new_v4(),
            "alice".to_string(),
            "alice@example.com".to_string(),
        );
        let token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.test".to_string();
        let msg = UserMessage::CreateUserResponse(Ok((user.clone(), token.clone())));

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"CreateUserResponse\""));
        assert!(json.contains("\"Ok\""));
        assert!(json.contains("alice"));
        assert!(json.contains("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9"));

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
        let password = "secret123".to_string();
        let msg = UserMessage::SignIn {
            username: username.clone(),
            password: password.clone(),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"SignIn\""));
        assert!(json.contains("bob"));
        assert!(json.contains("secret123"));

        let deserialized: UserMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, msg);
    }

    #[test]
    fn test_user_message_sign_in_response_ok() {
        let user = User::new(
            Uuid::new_v4(),
            "charlie".to_string(),
            "charlie@example.com".to_string(),
        );
        let token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.signin".to_string();
        let msg = UserMessage::SignInResponse(Ok((user.clone(), token.clone())));

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"SignInResponse\""));
        assert!(json.contains("\"Ok\""));
        assert!(json.contains("charlie"));
        assert!(json.contains("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9"));

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

    #[test]
    fn test_ai_action_request_serialization() {
        let action = AIActionRequest::StartConversation;
        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("StartConversation"));

        let action = AIActionRequest::ExplainMessage {
            message_id: Uuid::new_v4(),
        };
        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("ExplainMessage"));
    }

    #[test]
    fn test_user_message_validate_session_serialization() {
        let token = "jwt-token-abc123".to_string();
        let msg = UserMessage::ValidateSession {
            token: token.clone(),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"ValidateSession\""));
        assert!(json.contains("jwt-token-abc123"));

        let deserialized: UserMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, msg);
    }

    #[test]
    fn test_user_message_validate_session_response_ok() {
        let user = User::new(
            Uuid::new_v4(),
            "dave".to_string(),
            "dave@example.com".to_string(),
        );
        let msg = UserMessage::ValidateSessionResponse(Ok(user.clone()));

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"ValidateSessionResponse\""));
        assert!(json.contains("\"Ok\""));
        assert!(json.contains("dave"));

        let deserialized: UserMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, msg);
    }

    #[test]
    fn test_user_message_validate_session_response_err() {
        let error_msg = "Invalid or expired token".to_string();
        let msg = UserMessage::ValidateSessionResponse(Err(error_msg.clone()));

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"ValidateSessionResponse\""));
        assert!(json.contains("\"Err\""));
        assert!(json.contains(&error_msg));

        let deserialized: UserMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, msg);
    }

    #[test]
    fn test_auth_credentials_password_in_create_user() {
        let username = "newuser".to_string();
        let email = "new@example.com".to_string();
        let credentials = AuthCredentials::InviteCode("INVITE123".to_string());
        let password = "newpassword".to_string();
        let msg = UserMessage::CreateUser {
            username: username.clone(),
            email: email.clone(),
            credentials: credentials.clone(),
            password: password.clone(),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"CreateUser\""));
        assert!(json.contains("newuser"));
        assert!(json.contains("new@example.com"));
        assert!(json.contains("InviteCode"));
        assert!(json.contains("newpassword"));

        let deserialized: UserMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, msg);
    }
}
