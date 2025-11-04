use crate::app::app_state::{AppState, AppStateAction, OptionalUserState, UserStateAction};
use dialect_coach_shared::models::Message;
use dialect_coach_shared::UserState;
use log::{error, info};
use uuid::Uuid;
use yew::prelude::*;

pub fn on_replay_message(app_state: UseReducerHandle<AppState>) -> Callback<Message> {
    let app_state = app_state.clone();

    Callback::from(move |msg: Message| {
        info!("Replaying message with TTS: {}", msg.get_content());
        app_state.dispatch(AppStateAction::Speak(msg));
    })
}

pub fn on_user_state_ws_open(
    app_state: UseReducerHandle<AppState>,
    user_id: Uuid,
) -> Callback<()> {
    Callback::from(move |_| {
        info!(
            "User state WebSocket opened, loading state for user: {}",
            user_id
        );
        app_state.dispatch(AppStateAction::LoadUserState(user_id));
        app_state.dispatch(AppStateAction::RetryPendingSaves);
    })
}

pub fn on_user_state_load_response(
    app_state: UseReducerHandle<AppState>,
    user_state: UseReducerHandle<OptionalUserState>,
) -> Callback<Option<UserState>> {
    let app_state = app_state.clone();
    Callback::from(move |loaded_state: Option<UserState>| {
        if let Some(state) = loaded_state {
            info!("Received user state from backend");
            user_state.dispatch(UserStateAction::ReplaceUserState(state.clone()));
            app_state.dispatch(AppStateAction::NotifyTTSEnabled(state.tts_enabled));
        } else {
            info!("No existing user state on backend, using current state");
        }
    })
}

pub fn on_user_state_save_response() -> Callback<Result<(), String>> {
    Callback::from(move |result: Result<(), String>| match result {
        Ok(()) => info!("User state saved successfully to backend"),
        Err(e) => error!("Failed to save user state to backend: {}", e),
    })
}
