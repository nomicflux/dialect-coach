use crate::app::app_state::callbacks::report_unkept;
use crate::app::app_state::user::UserDomainAction;
use crate::app::app_state::{
    AppState, AppStateAction, MessageAction, SessionAction, SessionState, SettingsAction, UIState,
    UIStateAction,
};
use crate::app::app_websocket_hooks::{apply_chat_reply, lost_notice};
use crate::services::connection::RequestError;
use crate::services::persistence::{self, StoredRequest};
use dialect_coach_shared::{
    AIActionRequest, AccountRequest, AuthCredentials, ChatRequest, ClientMessage,
    InitialUserSettings, Message, NewUser, Reply, SignInResult, User, UserMessageWithContext,
    UserState,
};
use log::{error, info};
use std::future::Future;
use uuid::Uuid;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

pub fn add_message(msg: Message) -> SessionAction {
    SessionAction::Domain(UserDomainAction::Message(MessageAction::Add(msg)))
}

/// Send a coach request and apply its reply. The typing indicator shows until it resolves.
fn send_chat(
    app_state: &UseReducerHandle<AppState>,
    session: &UseReducerHandle<SessionState>,
    ui_state: &UseReducerHandle<UIState>,
    stored: StoredRequest,
) -> impl Future<Output = ()> + use<> {
    app_state.dispatch(AppStateAction::SetLoading);
    let failure = chat_failure(&stored.request);
    let outcome = submit_kept(app_state, stored);
    let (app_state, session, ui_state) = (app_state.clone(), session.clone(), ui_state.clone());
    async move {
        let outcome = outcome.await;
        app_state.dispatch(AppStateAction::LoadingComplete);
        apply_chat_outcome(outcome, failure, &app_state, &session, &ui_state);
    }
}

/// Submit a coach request, kept in this browser until it resolves so a refresh can resend it.
fn submit_kept(
    app_state: &UseReducerHandle<AppState>,
    stored: StoredRequest,
) -> impl Future<Output = Result<Reply, RequestError>> + use<> {
    let (id, user_id, envelope) = (stored.id, chat_user(&stored.request), stored.envelope());
    let kept = persistence::update_outbox(user_id, |outbox| persistence::add(outbox, stored));
    report_unkept(app_state, kept);
    let outcome = app_state.connection.submit(envelope);
    let app_state = app_state.clone();
    async move {
        let outcome = outcome.await;
        let kept = persistence::update_outbox(user_id, |outbox| persistence::remove(outbox, id));
        report_unkept(&app_state, kept);
        outcome
    }
}

fn chat_user(request: &ChatRequest) -> Uuid {
    match request {
        ChatRequest::Message(with_context) => with_context.user_id,
        ChatRequest::Action { user_id, .. } => *user_id,
    }
}

/// The error shown when the coach fails to answer `request`.
fn chat_failure(request: &ChatRequest) -> &'static str {
    match request {
        ChatRequest::Message(_) => "Failed to send",
        ChatRequest::Action { action, .. } => match **action {
            AIActionRequest::StartConversation { .. } => "Failed to start conversation",
            AIActionRequest::ContinueBranch { .. } => "Failed to continue conversation",
            AIActionRequest::ExplainMessage { .. } => "Failed to explain message",
        },
    }
}

/// The kept coach requests, removed from storage; each resent one is kept again when submitted.
fn take_outbox(app_state: &UseReducerHandle<AppState>, user_id: Uuid) -> Vec<StoredRequest> {
    let stored = persistence::load_outbox(user_id).unwrap_or_else(|e| {
        error!("Failed to read kept coach requests: {}", e);
        vec![]
    });
    report_unkept(app_state, persistence::store_outbox(user_id, &[]));
    stored
}

/// After sign-in, resend the coach requests a refresh interrupted before they were
/// sent. Those already sent can no longer be answered, so they are reported lost.
pub fn restore_outbox(
    user_id: Uuid,
    app_state: &UseReducerHandle<AppState>,
    session: &UseReducerHandle<SessionState>,
    ui_state: &UseReducerHandle<UIState>,
) {
    let stored = take_outbox(app_state, user_id);
    let (sent, unsent): (Vec<_>, Vec<_>) = stored.into_iter().partition(|s| s.sent);
    let lost: Vec<ClientMessage> = sent.into_iter().map(|s| s.envelope().body).collect();
    if let Some(notice) = lost_notice(&lost) {
        app_state.dispatch(AppStateAction::SetError(notice.to_string()));
    }
    for stored in unsent {
        spawn_local(send_chat(app_state, session, ui_state, stored));
    }
}

/// A lost request just ends the wait (the connection reports the loss); a failure shows `failure`.
fn apply_chat_outcome(
    outcome: Result<Reply, RequestError>,
    failure: &str,
    app_state: &UseReducerHandle<AppState>,
    session: &UseReducerHandle<SessionState>,
    ui_state: &UseReducerHandle<UIState>,
) {
    match outcome {
        Ok(reply) => apply_chat_reply(chat_message(reply), app_state, session, ui_state),
        Err(RequestError::Lost) => info!("Coach request lost"),
        Err(RequestError::Failed(e)) => {
            app_state.dispatch(AppStateAction::SetError(format!("{}: {}", failure, e)))
        }
    }
}

fn chat_message(reply: Reply) -> Message {
    match reply {
        Reply::Chat(message) => *message,
        other => unreachable!(
            "a chat request is answered with Reply::Chat, got {:?}",
            other
        ),
    }
}

pub fn signed_in(reply: Reply) -> SignInResult {
    match reply {
        Reply::SignedIn(result) => *result,
        other => unreachable!(
            "a sign-in request is answered with SignedIn, got {:?}",
            other
        ),
    }
}

/// A coach action for the current conversation session.
fn chat_action(session_id: Option<Uuid>, user_id: Uuid, action: AIActionRequest) -> ChatRequest {
    ChatRequest::Action {
        session_id: session_id.unwrap_or_else(Uuid::new_v4),
        user_id,
        action: Box::new(action),
    }
}

/// The message with the learning items, goals and branch context the coach needs.
fn message_with_context(state: &UserState, msg: Message) -> UserMessageWithContext {
    let dialect = state.active_branch_dialect();
    let goals = state.get_learning_goals_for_dialect(&dialect);
    let context_messages = state.get_active_branch_messages();
    UserMessageWithContext::builder(state.user_id, msg)
        .past_learning_items(state.get_past_learning_items(&dialect))
        .active_branch_id(state.active_branch_id)
        .active_plan(state.active_plan())
        .context_messages(context_messages.into_iter().cloned().collect())
        .learning_goals(goals.into_iter().cloned().collect())
        .user_gender(state.user_gender)
        .language_option(state.current_language_option())
        .language_level(state.current_language_level())
        .build()
}

pub fn on_send_message(
    app_state: UseReducerHandle<AppState>,
    session: UseReducerHandle<SessionState>,
    ui_state: UseReducerHandle<UIState>,
) -> Callback<String> {
    Callback::from(move |content: String| {
        let Some(state) = session.user.as_ref() else {
            return;
        };
        info!("Sending message: {}", content);
        let session_id = app_state.session_id().unwrap_or_else(Uuid::new_v4);
        let msg = state.create_user_msg(session_id, &content);
        session.dispatch(add_message(msg.clone()));
        let request = ChatRequest::Message(Box::new(message_with_context(state, msg)));
        app_state.dispatch(AppStateAction::ClearError);
        let stored = StoredRequest::new(request);
        spawn_local(send_chat(&app_state, &session, &ui_state, stored));
    })
}

pub fn on_tts_toggle(
    app_state: UseReducerHandle<AppState>,
    session: UseReducerHandle<SessionState>,
) -> Callback<()> {
    let app_state = app_state.clone();
    let session = session.clone();
    Callback::from(move |_| {
        if let Some(state) = session.user.as_ref() {
            let new_value = !state.tts_enabled;
            session.dispatch(SessionAction::Domain(UserDomainAction::Settings(
                SettingsAction::ToggleTTS,
            )));
            app_state.dispatch(AppStateAction::NotifyTTSEnabled(new_value));
        }
    })
}

/// Send a sign-in or account creation with the loading screen up.
fn send_account(
    app_state: &UseReducerHandle<AppState>,
    ui_state: &UseReducerHandle<UIState>,
    session: &UseReducerHandle<SessionState>,
    request: AccountRequest,
) {
    ui_state.dispatch(UIStateAction::SetSignInLoading(true));
    let request = ClientMessage::Account(request);
    let outcome = app_state.connection.request(request);
    let on_response = on_signin_response(app_state.clone(), ui_state.clone(), session.clone());
    let ui_state = ui_state.clone();
    spawn_local(async move { apply_account_outcome(outcome.await, &on_response, &ui_state) });
}

/// A lost request just ends the wait; the connection reports the loss.
fn apply_account_outcome(
    outcome: Result<Reply, RequestError>,
    on_response: &Callback<SignInResult>,
    ui_state: &UseReducerHandle<UIState>,
) {
    match outcome {
        Ok(reply) => on_response.emit(signed_in(reply)),
        Err(RequestError::Lost) => ui_state.dispatch(UIStateAction::SetSignInLoading(false)),
        Err(RequestError::Failed(e)) => on_response.emit(Err(e)),
    }
}

/// Username, email, password, invite code and initial settings from the account form.
type AccountForm = (String, String, String, String, Option<InitialUserSettings>);

fn new_user((username, email, password, invite_code, initial_settings): AccountForm) -> NewUser {
    NewUser {
        username,
        email,
        credentials: AuthCredentials::InviteCode(invite_code),
        password,
        initial_settings,
    }
}

pub fn on_create_user_click(
    app_state: UseReducerHandle<AppState>,
    ui_state: UseReducerHandle<UIState>,
    session: UseReducerHandle<SessionState>,
) -> Callback<AccountForm> {
    Callback::from(move |form: AccountForm| {
        let new_user = new_user(form);
        let initial_settings = new_user.initial_settings.clone();
        app_state.dispatch(AppStateAction::StorePendingInitialSettings(
            initial_settings,
        ));
        let request = AccountRequest::CreateUser(Box::new(new_user));
        send_account(&app_state, &ui_state, &session, request);
    })
}

pub fn on_signin_click(
    app_state: UseReducerHandle<AppState>,
    ui_state: UseReducerHandle<UIState>,
    session: UseReducerHandle<SessionState>,
) -> Callback<(String, String)> {
    Callback::from(move |(username, password): (String, String)| {
        let request = AccountRequest::SignIn { username, password };
        send_account(&app_state, &ui_state, &session, request);
    })
}

fn apply_sign_in(
    (user, server_state, token): (User, UserState, String),
    app_state: &UseReducerHandle<AppState>,
    ui_state: &UseReducerHandle<UIState>,
    session: &UseReducerHandle<SessionState>,
) {
    info!("Sign in successful: {}", user.username);
    crate::utils::cookies::set_session_token(&token);
    ui_state.dispatch(UIStateAction::HideUserCreationPage);
    let (user_state, restored) = sign_in_state(user.id, server_state);
    app_state.dispatch(AppStateAction::NotifyTTSEnabled(user_state.tts_enabled));
    app_state.dispatch(AppStateAction::SetUser(user, token));
    session.dispatch(SessionAction::Login(user_state));
    if restored {
        session.dispatch(SessionAction::MarkDirty);
    }
    app_state.dispatch(AppStateAction::CreateSession(Uuid::new_v4()));
}

/// The state to sign in with, and whether it holds changes this browser kept
/// that the server never confirmed (they are then saved).
fn sign_in_state(user_id: Uuid, server_state: UserState) -> (UserState, bool) {
    let unsaved = persistence::load_user_state(user_id).unwrap_or_else(|e| {
        error!("Failed to read the kept state: {}", e);
        None
    });
    let restored = unsaved.is_some();
    (persistence::prefer_unsaved(server_state, unsaved), restored)
}

pub fn on_signin_response(
    app_state: UseReducerHandle<AppState>,
    ui_state: UseReducerHandle<UIState>,
    session: UseReducerHandle<SessionState>,
) -> Callback<SignInResult> {
    Callback::from(move |result: SignInResult| {
        ui_state.dispatch(UIStateAction::SetSignInLoading(false));
        match result {
            Ok(signed_in) => apply_sign_in(signed_in, &app_state, &ui_state, &session),
            Err(e) => {
                error!("Sign in failed: {}", e);
                app_state.dispatch(AppStateAction::SetError(format!("Sign in failed: {}", e)));
            }
        }
    })
}

pub fn sign_out(app_state: &UseReducerHandle<AppState>, session: &UseReducerHandle<SessionState>) {
    info!("User signed out, clearing user state and session cookie");
    crate::utils::cookies::clear_session_token();
    app_state.dispatch(AppStateAction::DestroySession);
    session.dispatch(SessionAction::Logout);
    app_state.dispatch(AppStateAction::ClearUser);
}

pub fn on_signout_click(
    app_state: UseReducerHandle<AppState>,
    session: UseReducerHandle<SessionState>,
) -> Callback<MouseEvent> {
    Callback::from(move |_: MouseEvent| sign_out(&app_state, &session))
}

pub fn on_auto_start(
    app_state: UseReducerHandle<AppState>,
    session: UseReducerHandle<SessionState>,
    ui_state: UseReducerHandle<UIState>,
) -> Callback<()> {
    Callback::from(move |_| {
        let Some(state) = session.user.as_ref() else {
            return;
        };
        info!("Sending auto-start conversation request");
        let context = Box::new(state.build_action_context(vec![]));
        let action = AIActionRequest::StartConversation { context };
        let request = chat_action(app_state.session_id(), state.user_id, action);
        let stored = StoredRequest::new(request);
        spawn_local(send_chat(&app_state, &session, &ui_state, stored));
    })
}

/// Continue the branch from `parent_message_id`, with the messages up to and including it.
fn continue_action(state: &UserState, parent_message_id: Uuid) -> AIActionRequest {
    let all_msgs = state.get_active_branch_messages();
    let context_messages = match all_msgs.iter().position(|m| m.id == parent_message_id) {
        Some(idx) => all_msgs[..=idx].iter().map(|m| (*m).clone()).collect(),
        None => vec![],
    };
    let context = Box::new(state.build_action_context(context_messages));
    AIActionRequest::ContinueBranch {
        parent_message_id,
        context,
    }
}

pub fn on_continue_branch(
    app_state: UseReducerHandle<AppState>,
    session: UseReducerHandle<SessionState>,
    ui_state: UseReducerHandle<UIState>,
) -> Callback<Uuid> {
    Callback::from(move |parent_message_id: Uuid| {
        let Some(state) = session.user.as_ref() else {
            return;
        };
        info!("Sending continue branch request");
        let action = continue_action(state, parent_message_id);
        let request = chat_action(app_state.session_id(), state.user_id, action);
        let stored = StoredRequest::new(request);
        spawn_local(send_chat(&app_state, &session, &ui_state, stored));
    })
}

/// The explain request for a message, or `None` if the message is gone.
fn explain_action(state: &UserState, message_id: Uuid) -> Option<AIActionRequest> {
    let message = state.msg_by_id(message_id)?;
    Some(AIActionRequest::ExplainMessage {
        message_content: message.get_content().to_string(),
        dialect: state.active_branch_dialect(),
        formality: state.active_branch_settings().formality,
    })
}

/// The message's explain button shows loading until the request resolves.
fn explain(
    app_state: &UseReducerHandle<AppState>,
    session: &UseReducerHandle<SessionState>,
    ui_state: &UseReducerHandle<UIState>,
    request: ChatRequest,
    message_id: Uuid,
) -> impl Future<Output = ()> + use<> {
    ui_state.dispatch(UIStateAction::SetExplainLoading { message_id });
    let sent = send_chat(app_state, session, ui_state, StoredRequest::new(request));
    let ui_state = ui_state.clone();
    async move {
        sent.await;
        ui_state.dispatch(UIStateAction::ClearExplainLoading { message_id });
    }
}

pub fn on_explain_message(
    app_state: UseReducerHandle<AppState>,
    session: UseReducerHandle<SessionState>,
    ui_state: UseReducerHandle<UIState>,
) -> Callback<Uuid> {
    Callback::from(move |message_id: Uuid| {
        let Some(state) = session.user.as_ref() else {
            return;
        };
        let Some(action) = explain_action(state, message_id) else {
            error!("Message not found for explain");
            return;
        };
        info!("Sending explain message request");
        let request = chat_action(app_state.session_id(), state.user_id, action);
        spawn_local(explain(
            &app_state, &session, &ui_state, request, message_id,
        ));
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use dialect_coach_shared::{Dialect, Formality, MessageMetadata, TeachingMode};

    fn state_with_messages(contents: &[&str]) -> (UserState, Vec<Uuid>) {
        let mut state = UserState::new(Uuid::new_v4());
        let mut ids = Vec::new();
        for content in contents {
            let msg = state.create_user_msg(Uuid::new_v4(), content);
            ids.push(msg.id);
            state = crate::app::app_state::user::reducer::apply_user_state_action(
                &state,
                UserDomainAction::Message(MessageAction::Add(msg)).into(),
            );
        }
        (state, ids)
    }

    #[test]
    fn test_chat_action_uses_given_session() {
        let (session_id, user_id) = (Uuid::new_v4(), Uuid::new_v4());
        let action = AIActionRequest::ExplainMessage {
            message_content: "Hola".to_string(),
            dialect: Dialect::SpanishArgentinian,
            formality: Formality::Informal,
        };
        match chat_action(Some(session_id), user_id, action) {
            ChatRequest::Action {
                session_id: s,
                user_id: u,
                ..
            } => {
                assert_eq!((s, u), (session_id, user_id))
            }
            other => panic!("unexpected request: {:?}", other),
        }
    }

    fn action(action: AIActionRequest) -> ChatRequest {
        chat_action(None, Uuid::new_v4(), action)
    }

    #[test]
    fn test_chat_user_and_failure_follow_the_request() {
        let (state, ids) = state_with_messages(&["Hola"]);
        let msg = state.create_user_msg(Uuid::new_v4(), "¿Qué tal?");
        let message = ChatRequest::Message(Box::new(message_with_context(&state, msg)));
        assert_eq!(chat_user(&message), state.user_id);
        assert_eq!(chat_failure(&message), "Failed to send");
        let context = Box::new(state.build_action_context(vec![]));
        let start = action(AIActionRequest::StartConversation { context });
        assert_eq!(chat_failure(&start), "Failed to start conversation");
        let resume = action(continue_action(&state, ids[0]));
        assert_eq!(chat_failure(&resume), "Failed to continue conversation");
        let explain = action(explain_action(&state, ids[0]).unwrap());
        assert_eq!(chat_failure(&explain), "Failed to explain message");
        let user_id = Uuid::new_v4();
        let explain_for = chat_action(None, user_id, explain_action(&state, ids[0]).unwrap());
        assert_eq!(chat_user(&explain_for), user_id);
    }

    #[test]
    fn test_message_with_context_carries_user_and_branch() {
        let (state, _) = state_with_messages(&["Hola"]);
        let msg = state.create_user_msg(Uuid::new_v4(), "¿Qué tal?");
        let with_context = message_with_context(&state, msg.clone());
        assert_eq!(with_context.user_id, state.user_id);
        assert_eq!(with_context.message.id, msg.id);
        assert_eq!(with_context.active_branch_id, state.active_branch_id);
        assert_eq!(with_context.context_messages.len(), 1);
    }

    fn continued_context(state: &UserState, parent: Uuid) -> Vec<Uuid> {
        match continue_action(state, parent) {
            AIActionRequest::ContinueBranch { context, .. } => {
                context.context_messages.iter().map(|m| m.id).collect()
            }
            other => panic!("unexpected action: {:?}", other),
        }
    }

    #[test]
    fn test_continue_action_context_stops_at_parent() {
        let (state, ids) = state_with_messages(&["uno", "dos", "tres"]);
        assert_eq!(continued_context(&state, ids[1]), ids[..2].to_vec());
        assert!(continued_context(&state, Uuid::new_v4()).is_empty());
    }

    #[test]
    fn test_explain_action_reads_the_message() {
        let (state, ids) = state_with_messages(&["Che, ¿qué onda?"]);
        match explain_action(&state, ids[0]) {
            Some(AIActionRequest::ExplainMessage {
                message_content, ..
            }) => {
                assert_eq!(message_content, "Che, ¿qué onda?")
            }
            other => panic!("unexpected action: {:?}", other),
        }
        assert!(explain_action(&state, Uuid::new_v4()).is_none());
    }

    #[test]
    fn test_new_user_uses_invite_code() {
        let form = (
            "bob".to_string(),
            "bob@example.com".to_string(),
            "pw".to_string(),
            "CODE".to_string(),
            None,
        );
        let user = new_user(form);
        assert_eq!(user.username, "bob");
        assert_eq!(
            user.credentials,
            AuthCredentials::InviteCode("CODE".to_string())
        );
    }

    #[test]
    fn test_reply_unwrapping() {
        let metadata = MessageMetadata::at_now(
            Formality::Informal,
            TeachingMode::Immersive,
            dialect_coach_shared::Language::Spanish,
            Dialect::SpanishArgentinian,
            Uuid::new_v4(),
        );
        let msg = Message::user_message("Hola".to_string(), metadata, None);
        assert_eq!(chat_message(Reply::Chat(Box::new(msg.clone()))).id, msg.id);
        let refused = signed_in(Reply::SignedIn(Box::new(Err("nope".to_string()))));
        assert_eq!(refused.err(), Some("nope".to_string()));
    }
}
