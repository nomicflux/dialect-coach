use crate::app::app_state::{AppState, AppStateAction, SessionAction, SessionState};
use crate::services::connection::RequestError;
use dialect_coach_shared::Reply;
use dialect_coach_shared::models::Message;
use log::{error, info};
use yew::prelude::*;

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

pub fn on_user_state_save_response() -> Callback<Result<(), String>> {
    Callback::from(move |result: Result<(), String>| match result {
        Ok(()) => info!("User state saved successfully to backend"),
        Err(e) => error!("Failed to save user state to backend: {}", e),
    })
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
