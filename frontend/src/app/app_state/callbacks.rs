use crate::app::app_state::{AppState, AppStateAction, SessionAction, SessionState};
use crate::services::connection::RequestError;
use crate::services::persistence;
use dialect_coach_shared::models::Message;
use dialect_coach_shared::{ClientEnvelope, ClientMessage, Reply, RequestId, UserState};
use log::{error, info};
use uuid::Uuid;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

const UNKEPT: &str = "This browser couldn't keep your unsent changes.";

pub fn on_replay_message(app_state: UseReducerHandle<AppState>) -> Callback<Message> {
    let app_state = app_state.clone();

    Callback::from(move |msg: Message| {
        info!("Replaying message with TTS: {}", msg.get_content());
        app_state.dispatch(AppStateAction::Speak(msg));
    })
}

pub fn on_user_state_ws_open(session: UseReducerHandle<SessionState>) -> Callback<()> {
    Callback::from(move |_| {
        info!("Sign-in renewed; saving current state");
        session.dispatch(SessionAction::MarkDirty);
    })
}

/// A save's outcome; a lost save is redone when the connection reopens.
pub fn save_result(outcome: Result<Reply, RequestError>) -> Result<(), String> {
    match outcome {
        Ok(Reply::UserStateSaved(result)) => result,
        Ok(other) => unreachable!("a save is answered with UserStateSaved, got {:?}", other),
        Err(RequestError::Lost) => {
            Err("connection dropped before the save was acknowledged".into())
        }
        Err(RequestError::Failed(e)) => Err(e),
    }
}

/// Once save `id` is confirmed, this browser no longer needs the state it carried.
pub fn on_user_state_save_response(user_id: Uuid, id: RequestId) -> Callback<Result<(), String>> {
    Callback::from(move |result: Result<(), String>| match result {
        Ok(()) => {
            info!("User state saved successfully to backend");
            if let Err(e) = persistence::clear_user_state_if(user_id, id) {
                error!("Failed to forget the saved state: {}", e);
            }
        }
        Err(e) => error!("Failed to save user state to backend: {}", e),
    })
}

/// A failed write to browser storage means a refresh could lose unsent work.
pub fn report_unkept(app_state: &UseReducerHandle<AppState>, kept: Result<(), String>) {
    if let Err(e) = kept {
        error!("{}", e);
        app_state.dispatch(AppStateAction::SetError(UNKEPT.to_string()));
    }
}

/// Keep each unsaved change in this browser at once, so a refresh cannot lose it.
pub fn keep_user_state(app_state: &UseReducerHandle<AppState>) -> impl Fn(&UserState) + 'static {
    let app_state = app_state.clone();
    move |state| {
        let kept = persistence::save_user_state(state.user_id, state, None);
        report_unkept(&app_state, kept);
    }
}

/// Send the state to the server. It counts as saved once submitted; a save lost to a
/// dropped connection is redone when the connection reopens.
pub fn save_user_state(
    app_state: &UseReducerHandle<AppState>,
    session: &UseReducerHandle<SessionState>,
) -> impl Fn(&UserState) + 'static {
    let (app_state, session) = (app_state.clone(), session.clone());
    move |state| {
        let id = Uuid::new_v4();
        report_unkept(
            &app_state,
            persistence::save_user_state(state.user_id, state, Some(id)),
        );
        let body = ClientMessage::SaveUserState(Box::new(state.clone()));
        let saved = app_state.connection.submit(ClientEnvelope { id, body });
        info!("UserState save request submitted");
        session.dispatch(SessionAction::Saved);
        let on_saved = on_user_state_save_response(state.user_id, id);
        spawn_local(async move { on_saved.emit(save_result(saved.await)) });
    }
}

pub fn on_user_state_usage_stats_update(
    session: UseReducerHandle<SessionState>,
) -> Callback<dialect_coach_shared::UsageStats> {
    Callback::from(move |usage_stats: dialect_coach_shared::UsageStats| {
        info!("Received usage stats update");
        session.dispatch(SessionAction::UpdateUsageStats(usage_stats));
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_save_result() {
        assert_eq!(save_result(Ok(Reply::UserStateSaved(Ok(())))), Ok(()));
        let refused = Ok(Reply::UserStateSaved(Err("db down".to_string())));
        assert_eq!(save_result(refused), Err("db down".to_string()));
        assert!(save_result(Err(RequestError::Lost)).is_err());
        let failed = Err(RequestError::Failed("internal error".to_string()));
        assert_eq!(save_result(failed), Err("internal error".to_string()));
    }
}
