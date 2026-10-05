use super::{
    AIActionRequest, AuthCredentials, InitialUserSettings, Message, UsageStats, User,
    UserMessageWithContext, UserState,
};
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
    /// The request's task panicked.
    Failed(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        AgentResponse, Dialect, Formality, Language, MessageMetadata, TeachingMode,
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
}
