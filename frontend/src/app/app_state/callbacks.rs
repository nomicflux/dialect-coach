use crate::app::app_state::{AppState, AppStateAction, SessionAction, SessionState};
use dialect_coach_shared::UserState;
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
        info!("User state WebSocket opened");
        session.dispatch(SessionAction::MarkDirty);
    })
}

pub fn on_user_state_load_response(
    app_state: UseReducerHandle<AppState>,
    session: UseReducerHandle<SessionState>,
) -> Callback<Option<UserState>> {
    let app_state = app_state.clone();
    Callback::from(move |loaded_state: Option<UserState>| {
        if let Some(state) = loaded_state {
            info!(
                "Received user state from backend with {} learning goals, {} messages, {} learning items",
                state.learning_goals.len(),
                state.conversation_history.len(),
                state.learning_items.len()
            );
            session.dispatch(SessionAction::UpdateUser(state.clone()));
            app_state.dispatch(AppStateAction::NotifyTTSEnabled(state.tts_enabled));
        }
    })
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
