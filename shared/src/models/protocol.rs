use super::{
    AIActionRequest, AuthCredentials, Dialect, EnrichRequest, EnrichResponse, GrammarRequest,
    GrammarResponse, InitialUserSettings, Message, TranslateRequest, TranslateResponse, UsageStats,
    User, UserMessageWithContext, UserState, plan::import::SimpleImportLanguagePlan,
};
use crate::tts::TtsRequest;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Identifies a request; its reply carries the same id.
pub type RequestId = Uuid;

/// User, their state, and a session token; or why sign-in failed.
pub type SignInResult = Result<(User, UserState, String), String>;

/// A client request on the single WebSocket connection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientEnvelope {
    pub id: RequestId,
    pub body: ClientMessage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMessage {
    Account(AccountRequest),
    SaveUserState(Box<UserState>),
    /// Answered in the order sent, one at a time.
    Chat(ChatRequest),
    /// Each answered in its own task, so a slow one holds up nothing.
    Study(StudyRequest),
}

/// Requests that sign a connection in.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AccountRequest {
    SignIn {
        username: String,
        password: String,
    },
    CreateUser(Box<NewUser>),
    /// Fresh page: an expired token is rejected.
    ValidateSession {
        token: String,
    },
    /// Signed-in tab reconnecting: the token is renewed even if expired.
    Reattach {
        token: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewUser {
    pub username: String,
    pub email: String,
    pub credentials: AuthCredentials,
    pub password: String,
    pub initial_settings: Option<InitialUserSettings>,
}

/// Requests answered by the coach.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChatRequest {
    Message(Box<UserMessageWithContext>),
    Action {
        session_id: Uuid,
        user_id: Uuid,
        action: Box<AIActionRequest>,
    },
}

/// Study aids: selection lookups, item enrichment, plan generation and voice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StudyRequest {
    Translate(TranslateRequest),
    Grammar(GrammarRequest),
    Enrich(EnrichRequest),
    GeneratePlan(PlanRequest),
    Synthesize(TtsRequest),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanRequest {
    pub dialect: Dialect,
    pub source: PlanSource,
}

/// The material a plan is generated from.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlanSource {
    Text(String),
    File {
        content_type: String,
        base64: String,
    },
}

/// Synthesized speech, base64-encoded.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeechAudio {
    pub audio_base64: String,
    pub duration_ms: u32,
}

/// Why a learning item could not be enriched.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnrichFailure {
    InvalidInput,
    Server,
}

/// Everything the server sends on the connection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    Reply {
        id: RequestId,
        body: Reply,
    },
    /// Pushed to every connection of the user after usage is saved.
    UsageStats(UsageStats),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Reply {
    /// Answers SignIn, CreateUser and ValidateSession.
    SignedIn(Box<SignInResult>),
    /// Renewed token and current usage.
    Reattached(Result<(String, UsageStats), String>),
    UserStateSaved(Result<(), String>),
    /// Answers every ChatRequest (success or error message).
    Chat(Box<Message>),
    Translated(TranslateResponse),
    Grammar(GrammarResponse),
    Enriched(Result<EnrichResponse, EnrichFailure>),
    Plan(Result<SimpleImportLanguagePlan, String>),
    Speech(Result<SpeechAudio, String>),
    /// The request's task panicked.
    Failed(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        AgentResponse, Formality, Language, MessageMetadata, PartialLearningItem, PartialMistake,
        PhraseTranslation, TeachingMode,
    };
    use serde::de::DeserializeOwned;

    fn assert_round_trip<T: Serialize + DeserializeOwned>(value: &T) {
        let json = serde_json::to_value(value).unwrap();
        let back: T = serde_json::from_value(json.clone()).unwrap();
        assert_eq!(serde_json::to_value(&back).unwrap(), json);
    }

    fn envelope(body: ClientMessage) -> ClientEnvelope {
        ClientEnvelope {
            id: Uuid::new_v4(),
            body,
        }
    }

    fn account(request: AccountRequest) -> ClientEnvelope {
        envelope(ClientMessage::Account(request))
    }

    fn reply(body: Reply) -> ServerMessage {
        ServerMessage::Reply {
            id: Uuid::new_v4(),
            body,
        }
    }

    fn metadata() -> MessageMetadata {
        MessageMetadata::at_now(
            Formality::Informal,
            TeachingMode::Immersive,
            Language::Spanish,
            Dialect::SpanishArgentinian,
            Uuid::new_v4(),
        )
    }

    fn signed_in_user() -> (User, UserState, String) {
        let user = User::new(Uuid::new_v4(), "charlie".into(), "c@example.com".into());
        let state = UserState::new(user.id);
        (user, state, "eyJhbGciOiJIUzI1NiJ9.signin".into())
    }

    #[test]
    fn test_auth_requests_round_trip() {
        assert_round_trip(&account(AccountRequest::SignIn {
            username: "bob".into(),
            password: "secret123".into(),
        }));
        assert_round_trip(&account(AccountRequest::CreateUser(Box::new(NewUser {
            username: "newuser".into(),
            email: "new@example.com".into(),
            credentials: AuthCredentials::InviteCode("INVITE123".into()),
            password: "newpassword".into(),
            initial_settings: Some(InitialUserSettings::default()),
        }))));
        assert_round_trip(&account(AccountRequest::ValidateSession {
            token: "t".into(),
        }));
        assert_round_trip(&account(AccountRequest::Reattach { token: "t".into() }));
    }

    #[test]
    fn test_state_and_chat_requests_round_trip() {
        let state = UserState::new(Uuid::new_v4());
        assert_round_trip(&envelope(ClientMessage::SaveUserState(Box::new(state))));
        let msg = Message::user_message("Hola".into(), metadata(), None);
        let chat = UserMessageWithContext::builder(Uuid::new_v4(), msg).build();
        assert_round_trip(&envelope(ClientMessage::Chat(ChatRequest::Message(
            Box::new(chat),
        ))));
        assert_round_trip(&envelope(ClientMessage::Chat(ChatRequest::Action {
            session_id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            action: Box::new(AIActionRequest::ExplainMessage {
                message_content: "Hola".into(),
                dialect: Dialect::SpanishArgentinian,
                formality: Formality::Informal,
            }),
        })));
    }

    #[test]
    fn test_auth_replies_round_trip() {
        assert_round_trip(&reply(Reply::SignedIn(Box::new(Ok(signed_in_user())))));
        assert_round_trip(&reply(Reply::SignedIn(Box::new(Err(
            "User not found".into()
        )))));
        let usage = UsageStats::default();
        assert_round_trip(&reply(Reply::Reattached(Ok(("t".into(), usage)))));
        assert_round_trip(&reply(Reply::Reattached(Err("Invalid token".into()))));
    }

    #[test]
    fn test_other_replies_and_push_round_trip() {
        assert_round_trip(&reply(Reply::UserStateSaved(Ok(()))));
        assert_round_trip(&reply(Reply::UserStateSaved(Err("db down".into()))));
        let response = AgentResponse::from("¡Hola!");
        let msg = Message::agent_message(response, metadata(), None);
        assert_round_trip(&reply(Reply::Chat(Box::new(msg))));
        assert_round_trip(&reply(Reply::Failed("internal error".into())));
        assert_round_trip(&ServerMessage::UsageStats(UsageStats::default()));
    }

    fn study(request: StudyRequest) -> ClientEnvelope {
        envelope(ClientMessage::Study(request))
    }

    #[test]
    fn test_study_requests_round_trip() {
        assert_round_trip(&study(StudyRequest::Translate(TranslateRequest {
            phrase: "che".into(),
            context: "¿Qué hacés, che?".into(),
            dialect: "spanish_argentinian".into(),
            formality: Some("informal".into()),
        })));
        assert_round_trip(&study(StudyRequest::Grammar(GrammarRequest {
            phrase: "hacés".into(),
            context: "¿Qué hacés?".into(),
            dialect: "spanish_argentinian".into(),
        })));
        assert_round_trip(&study(StudyRequest::Enrich(EnrichRequest {
            dialect: Dialect::SpanishArgentinian,
            partial_data: PartialLearningItem::Mistake(PartialMistake {
                specific_mistake: Some("vos sos".into()),
                correction: None,
                mistake_category: None,
            }),
        })));
        assert_round_trip(&study(StudyRequest::Synthesize(TtsRequest::new(
            Uuid::new_v4(),
            "Hola".into(),
            Dialect::SpanishArgentinian,
        ))));
    }

    #[test]
    fn test_plan_requests_round_trip() {
        for source in [
            PlanSource::Text("Unit 1: greetings".into()),
            PlanSource::File {
                content_type: "application/pdf".into(),
                base64: "JVBERi0xLjQ=".into(),
            },
        ] {
            assert_round_trip(&study(StudyRequest::GeneratePlan(PlanRequest {
                dialect: Dialect::SpanishArgentinian,
                source,
            })));
        }
    }

    #[test]
    fn test_study_replies_round_trip() {
        assert_round_trip(&reply(Reply::Translated(TranslateResponse {
            original_sentence: "che".into(),
            segmented_phrases: vec![PhraseTranslation {
                target_text: "che".into(),
                english: "hey".into(),
            }],
            success: true,
            error: None,
        })));
        assert_round_trip(&reply(Reply::Grammar(GrammarResponse {
            original_phrase: "hacés".into(),
            explanations: vec![],
            success: false,
            error: Some("Invalid dialect: x".into()),
        })));
        let enriched = EnrichResponse {
            enriched_item: serde_json::json!({"correction": "vos sos"}),
        };
        assert_round_trip(&reply(Reply::Enriched(Ok(enriched))));
        assert_round_trip(&reply(Reply::Enriched(Err(EnrichFailure::InvalidInput))));
    }

    #[test]
    fn test_plan_and_speech_replies_round_trip() {
        let plan = SimpleImportLanguagePlan {
            title: "Greetings".into(),
            dialect: Dialect::SpanishArgentinian,
            description: None,
            steps: vec![],
        };
        assert_round_trip(&reply(Reply::Plan(Ok(plan))));
        assert_round_trip(&reply(Reply::Plan(Err("Empty text content".into()))));
        let audio = SpeechAudio {
            audio_base64: "SUQz".into(),
            duration_ms: 1200,
        };
        assert_round_trip(&reply(Reply::Speech(Ok(audio))));
        assert_round_trip(&reply(Reply::Speech(Err("TTS rate limit exceeded".into()))));
    }
}
