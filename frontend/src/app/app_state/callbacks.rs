use crate::app::app_state::{AppState, AppStateAction, OptionalUserState, UserStateAction};
use dialect_coach_shared::UserState;
use dialect_coach_shared::models::Message;
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

pub fn on_user_state_ws_open(app_state: UseReducerHandle<AppState>, user_id: Uuid) -> Callback<()> {
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
            info!(
                "Received user state from backend with {} learning goals, {} messages, {} learning items",
                state.learning_goals.len(),
                state.conversation_history.len(),
                state.learning_items.len()
            );
            user_state.dispatch(UserStateAction::ReplaceUserState(state.clone()));
            app_state.dispatch(AppStateAction::NotifyTTSEnabled(state.tts_enabled));
        } else {
            // New user - create initial UserState
            if let Some(user) = app_state.current_user.as_ref() {
                info!("No user state found for new user, creating initial state");
                let new_state = UserState::new(user.id);
                user_state.dispatch(UserStateAction::ReplaceUserState(new_state.clone()));
                app_state.dispatch(AppStateAction::NotifyTTSEnabled(new_state.tts_enabled));
            } else {
                error!(
                    "Backend returned no user state and no current user - this should not happen"
                );
                app_state.dispatch(AppStateAction::SetError(
                    "Failed to load user state from backend".to_string(),
                ));
            }
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
    user_state: UseReducerHandle<OptionalUserState>,
) -> Callback<dialect_coach_shared::UsageStats> {
    Callback::from(move |usage_stats: dialect_coach_shared::UsageStats| {
        info!("Received usage stats update");
        user_state.dispatch(UserStateAction::UpdateUsageStats(usage_stats));
    })
}
