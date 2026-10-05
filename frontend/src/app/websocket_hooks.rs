use crate::app::app_callbacks::{add_message, on_signin_response, sign_out, signed_in};
use crate::app::app_state::callbacks::{on_user_state_usage_stats_update, on_user_state_ws_open};
use crate::app::app_state::user::UserDomainAction;
use crate::app::app_state::{
    AppState, AppStateAction, LearningAction, SessionAction, SessionState, UIState, UIStateAction,
};
use crate::services::connection::{ConnectionEvents, RequestError};
use crate::utils::cookies;
use dialect_coach_shared::models::{Message, MessageContent};
use dialect_coach_shared::{
    AccountRequest, AgentResponse, ClientMessage, Dialect, LearningItem, LearningItemType, Reply,
    UsageStats,
};
use gloo::timers::callback::Interval;
use log::{error, info};
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

/// A signed-in tab renews its sign-in this often while connected.
const RENEW_INTERVAL_MS: u32 = 12 * 60 * 60 * 1000;

const REQUEST_LOST: &str = "Connection dropped before a reply arrived. Please try again.";
const CREATE_USER_LOST: &str = "Connection dropped while creating your account. It may already exist — try signing in with those details.";
const SESSION_REFUSED: &str = "Your session is no longer valid. Please sign in again.";

/// The learning items in a reply, with their scores, for the flash queue.
fn build_flash_items(content: &AgentResponse, dialect: Dialect) -> Vec<LearningItem> {
    let item = |item: LearningItemType, score: &u8| LearningItem::with_score(item, dialect, *score);
    let mistakes = content.mistakes.iter().flatten();
    let explained = content.explained.iter().flatten();
    let translated = content.translated.iter().flatten();
    let exploratory = content.exploratory.iter().flatten();
    mistakes
        .map(|(m, s)| item(LearningItemType::Mistake(m.clone()), s))
        .chain(explained.map(|(e, s)| item(LearningItemType::Explanation(e.clone()), s)))
        .chain(translated.map(|(t, s)| item(LearningItemType::Translation(t.clone()), s)))
        .chain(exploratory.map(|(x, s)| item(LearningItemType::Exploration(x.clone()), s)))
        .collect()
}

fn check_rate_limit_error(error_text: &str, app_state: &UseReducerHandle<AppState>) {
    if error_text.contains("Response agent rate limit exceeded")
        || error_text.contains("Anthropic quota exceeded")
    {
        app_state.dispatch(AppStateAction::SetResponseRateLimited(true));
    }
    if error_text.contains("Analysis agent rate limit exceeded") {
        app_state.dispatch(AppStateAction::SetAnalysisRateLimited(true));
    }
}

/// Apply the coach's reply: rate limits, autoplay, learning items, analysis, and the message itself.
pub fn apply_chat_reply(
    msg: Message,
    app_state: &UseReducerHandle<AppState>,
    session: &UseReducerHandle<SessionState>,
    ui_state: &UseReducerHandle<UIState>,
) {
    if let MessageContent::AgentMessage { content } = &msg.content {
        check_rate_limit_error(&content.response, app_state);
        // Dispatch to AppState for autoplay check (reads from AppState's own state)
        app_state.dispatch(AppStateAction::ProcessAgentMessage(msg.clone()));
        apply_learning_items(content, session, ui_state);
        apply_analysis(content, session);
    }
    session.dispatch(add_message(msg));
}

fn apply_learning_items(
    content: &AgentResponse,
    session: &UseReducerHandle<SessionState>,
    ui_state: &UseReducerHandle<UIState>,
) {
    // Learning items belong to a signed-in user's state.
    let Some(user) = session.user.as_ref() else {
        return;
    };
    let flash_items = build_flash_items(content, user.active_branch_dialect());
    if flash_items.is_empty() {
        return;
    }
    ui_state.dispatch(UIStateAction::QueueFlashItems(flash_items));
    session.dispatch(add_learning_items(content));
}

fn add_learning_items(content: &AgentResponse) -> SessionAction {
    SessionAction::Domain(UserDomainAction::Learning(LearningAction::AddItems(
        content.mistakes.clone().unwrap_or_default(),
        content.explained.clone().unwrap_or_default(),
        content.translated.clone().unwrap_or_default(),
        content.exploratory.clone().unwrap_or_default(),
    )))
}

fn apply_analysis(content: &AgentResponse, session: &UseReducerHandle<SessionState>) {
    if let Some(analysis) = content.analysis.clone() {
        info!(
            "Received analysis with {} mistake scores, {} explained scores, {} translated scores, {} exploratory scores",
            analysis.mistake_scores.len(),
            analysis.explained_scores.len(),
            analysis.translated_scores.len(),
            analysis.exploratory_scores.len()
        );
        session.dispatch(SessionAction::Domain(UserDomainAction::Learning(
            LearningAction::UpdateScores(analysis),
        )));
    }
}

/// The notice for requests lost while the connection was down.
/// Background work (saving, session checks) is redone on reconnect, so it needs none.
pub fn lost_notice(bodies: &[ClientMessage]) -> Option<&'static str> {
    let creating_account = bodies
        .iter()
        .any(|body| matches!(body, ClientMessage::Account(AccountRequest::CreateUser(_))));
    if creating_account {
        return Some(CREATE_USER_LOST);
    }
    bodies
        .iter()
        .any(|body| !is_background(body))
        .then_some(REQUEST_LOST)
}

fn is_background(body: &ClientMessage) -> bool {
    matches!(
        body,
        ClientMessage::SaveUserState(_)
            | ClientMessage::Account(
                AccountRequest::ValidateSession { .. } | AccountRequest::Reattach { .. }
            )
    )
}

/// Renew a signed-in tab's sign-in, refresh its usage, and re-save its state.
fn reattach(
    app_state: &UseReducerHandle<AppState>,
    session: &UseReducerHandle<SessionState>,
    token: String,
) {
    let request = ClientMessage::Account(AccountRequest::Reattach { token });
    let outcome = app_state.connection.request(request);
    let (app_state, session) = (app_state.clone(), session.clone());
    spawn_local(async move { apply_reattach(outcome.await, &app_state, &session) });
}

/// A lost renewal is redone on reconnect.
fn apply_reattach(
    outcome: Result<Reply, RequestError>,
    app_state: &UseReducerHandle<AppState>,
    session: &UseReducerHandle<SessionState>,
) {
    match outcome {
        Ok(Reply::Reattached(renewed)) => apply_renewal(renewed, app_state, session),
        Ok(other) => unreachable!("Reattach is answered with Reattached, got {:?}", other),
        Err(RequestError::Lost) => info!("Reattach lost; it is redone on reconnect"),
        Err(RequestError::Failed(e)) => {
            let notice = format!("Failed to renew sign-in: {}", e);
            app_state.dispatch(AppStateAction::SetError(notice));
        }
    }
}

fn apply_renewal(
    renewed: Result<(String, UsageStats), String>,
    app_state: &UseReducerHandle<AppState>,
    session: &UseReducerHandle<SessionState>,
) {
    match renewed {
        Ok((token, usage_stats)) => {
            cookies::set_session_token(&token);
            session.dispatch(SessionAction::UpdateUsageStats(usage_stats));
            on_user_state_ws_open(session.clone()).emit(());
        }
        Err(e) => {
            error!("Sign-in renewal refused: {}", e);
            sign_out(app_state, session);
            app_state.dispatch(AppStateAction::SetError(SESSION_REFUSED.to_string()));
        }
    }
}

/// Sign a fresh page in from its stored session token.
fn validate_session(
    app_state: &UseReducerHandle<AppState>,
    ui_state: &UseReducerHandle<UIState>,
    session: &UseReducerHandle<SessionState>,
) {
    let Some(token) = cookies::get_session_token() else {
        return;
    };
    let request = ClientMessage::Account(AccountRequest::ValidateSession { token });
    let outcome = app_state.connection.request(request);
    let on_response = on_signin_response(app_state.clone(), ui_state.clone(), session.clone());
    let ui_state = ui_state.clone();
    spawn_local(async move { apply_validation(outcome.await, &on_response, &ui_state) });
}

/// A rejected stored token is forgotten without a notice; a lost check is redone on reconnect.
fn apply_validation(
    outcome: Result<Reply, RequestError>,
    on_response: &Callback<dialect_coach_shared::SignInResult>,
    ui_state: &UseReducerHandle<UIState>,
) {
    match outcome.map(signed_in) {
        Ok(Ok(signed_in)) => on_response.emit(Ok(signed_in)),
        Ok(Err(e)) => {
            info!("Stored session not accepted: {}", e);
            cookies::clear_session_token();
            ui_state.dispatch(UIStateAction::SetSignInLoading(false));
        }
        Err(RequestError::Lost) => info!("Session check lost; it is redone on reconnect"),
        Err(RequestError::Failed(e)) => on_response.emit(Err(e)),
    }
}

fn on_open(
    app_state: &UseReducerHandle<AppState>,
    session: &UseReducerHandle<SessionState>,
    ui_state: &UseReducerHandle<UIState>,
) -> Callback<()> {
    let (app_state, session, ui_state) = (app_state.clone(), session.clone(), ui_state.clone());
    Callback::from(move |_| match app_state.session_token.clone() {
        Some(token) => reattach(&app_state, &session, token),
        None => validate_session(&app_state, &ui_state, &session),
    })
}

fn on_lost(app_state: &UseReducerHandle<AppState>) -> Callback<Vec<ClientMessage>> {
    let app_state = app_state.clone();
    Callback::from(move |bodies: Vec<ClientMessage>| {
        if let Some(notice) = lost_notice(&bodies) {
            app_state.dispatch(AppStateAction::SetError(notice.to_string()));
        }
    })
}

fn connection_events(
    app_state: &UseReducerHandle<AppState>,
    session: &UseReducerHandle<SessionState>,
    ui_state: &UseReducerHandle<UIState>,
) -> ConnectionEvents {
    let status_state = app_state.clone();
    ConnectionEvents {
        on_status: Callback::from(move |status| {
            status_state.dispatch(AppStateAction::SetConnectionState(status))
        }),
        on_open: on_open(app_state, session, ui_state),
        on_lost: on_lost(app_state),
        on_push: on_user_state_usage_stats_update(session.clone()),
    }
}

/// Wire the app to its single connection and start it.
#[hook]
pub fn use_connection(
    app_state: UseReducerHandle<AppState>,
    session: UseReducerHandle<SessionState>,
    ui_state: UseReducerHandle<UIState>,
) {
    use_connection_events(app_state.clone(), session.clone(), ui_state.clone());
    use_renewal(app_state.clone(), session);
    use_effect_with((), move |_| {
        // A stored session keeps the loading screen up until it is checked.
        if cookies::get_session_token().is_some() {
            ui_state.dispatch(UIStateAction::SetSignInLoading(true));
        }
        app_state.connection.start();
    });
}

/// The connection's events read who is signed in, so they are rebuilt when that changes.
#[hook]
fn use_connection_events(
    app_state: UseReducerHandle<AppState>,
    session: UseReducerHandle<SessionState>,
    ui_state: UseReducerHandle<UIState>,
) {
    use_effect_with(app_state.current_user.clone(), move |_| {
        let events = connection_events(&app_state, &session, &ui_state);
        app_state.connection.set_events(events);
    });
}

#[hook]
fn use_renewal(app_state: UseReducerHandle<AppState>, session: UseReducerHandle<SessionState>) {
    use_effect_with(app_state.current_user.clone(), move |_| {
        let renewal = app_state.session_token.clone().map(|token| {
            Interval::new(RENEW_INTERVAL_MS, move || {
                reattach(&app_state, &session, token.clone())
            })
        });
        move || drop(renewal)
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use dialect_coach_shared::{NewUser, UserState};
    use uuid::Uuid;

    fn sign_in() -> ClientMessage {
        ClientMessage::Account(AccountRequest::SignIn {
            username: "bob".to_string(),
            password: "pw".to_string(),
        })
    }

    fn create_user() -> ClientMessage {
        ClientMessage::Account(AccountRequest::CreateUser(Box::new(NewUser {
            username: "bob".to_string(),
            email: "bob@example.com".to_string(),
            credentials: dialect_coach_shared::AuthCredentials::InviteCode("CODE".to_string()),
            password: "pw".to_string(),
            initial_settings: None,
        })))
    }

    fn background() -> Vec<ClientMessage> {
        vec![
            ClientMessage::SaveUserState(Box::new(UserState::new(Uuid::new_v4()))),
            ClientMessage::Account(AccountRequest::Reattach {
                token: "t".to_string(),
            }),
            ClientMessage::Account(AccountRequest::ValidateSession {
                token: "t".to_string(),
            }),
        ]
    }

    #[test]
    fn test_lost_notice_ignores_background_work() {
        assert_eq!(lost_notice(&background()), None);
        assert_eq!(lost_notice(&[]), None);
    }

    #[test]
    fn test_lost_notice_for_user_requests() {
        let mut bodies = background();
        bodies.push(sign_in());
        assert_eq!(lost_notice(&bodies), Some(REQUEST_LOST));
    }

    #[test]
    fn test_lost_notice_for_chat_request() {
        let state = UserState::new(Uuid::new_v4());
        let msg = state.create_user_msg(Uuid::new_v4(), "Hola");
        let chat =
            dialect_coach_shared::UserMessageWithContext::builder(state.user_id, msg).build();
        let bodies = [ClientMessage::Chat(
            dialect_coach_shared::ChatRequest::Message(Box::new(chat)),
        )];
        assert_eq!(lost_notice(&bodies), Some(REQUEST_LOST));
    }

    #[test]
    fn test_lost_notice_for_study_request() {
        let translate =
            dialect_coach_shared::StudyRequest::Translate(dialect_coach_shared::TranslateRequest {
                phrase: "che".to_string(),
                context: "¿Qué hacés, che?".to_string(),
                dialect: "spanish_argentinian".to_string(),
                formality: None,
            });
        let mut bodies = background();
        bodies.push(ClientMessage::Study(translate));
        assert_eq!(lost_notice(&bodies), Some(REQUEST_LOST));
    }

    #[test]
    fn test_lost_account_creation_says_to_sign_in() {
        assert_eq!(
            lost_notice(&[sign_in(), create_user()]),
            Some(CREATE_USER_LOST)
        );
    }

    #[test]
    fn test_build_flash_items_keeps_scores_in_reply_order() {
        let mut content = AgentResponse::from("¡Hola!");
        let mistake = dialect_coach_shared::Mistake::new(
            "hablar".to_string(),
            "habla".to_string(),
            dialect_coach_shared::MistakeCategory::SpellingError {
                context: "habla".to_string(),
            },
        );
        let explained = dialect_coach_shared::Explained::new("che".to_string(), "hey".to_string());
        content.mistakes = Some(vec![(mistake.clone(), 3)]);
        content.explained = Some(vec![(explained.clone(), 7)]);

        let items = build_flash_items(&content, Dialect::SpanishArgentinian);

        let got: Vec<(Uuid, u8)> = items.iter().map(|i| (i.id(), i.score)).collect();
        assert_eq!(got, vec![(mistake.id, 3), (explained.id, 7)]);
        assert!(
            build_flash_items(&AgentResponse::from("x"), Dialect::SpanishArgentinian).is_empty()
        );
    }
}
