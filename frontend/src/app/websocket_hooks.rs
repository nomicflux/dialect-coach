use crate::app::app_callbacks::{
    on_user_create_response, on_user_signin_response, on_validate_session_response,
};
use crate::app::app_state::callbacks::{
    on_user_state_load_response, on_user_state_save_response, on_user_state_usage_stats_update,
    on_user_state_ws_open,
};
use crate::app::app_state::user::UserDomainAction;
use crate::app::app_state::{
    AppState, AppStateAction, LearningAction, MessageAction, SessionAction, SessionState, UIState,
    UIStateAction,
};
use crate::services::websocket::ConnectionState;
use dialect_coach_shared::models::{Message, MessageContent};
use log::{error, info};
use yew::prelude::*;

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

#[hook]
pub fn use_chat_websocket(
    app_state: UseReducerHandle<AppState>,
    session: UseReducerHandle<SessionState>,
    ui_state: UseReducerHandle<UIState>,
) {
    let current_user = app_state.current_user.clone();

    use_effect_with(current_user, move |user_opt| {
        let had_user = user_opt.is_some();
        let app_state = app_state.clone();
        let session = session.clone();
        let ui_state = ui_state.clone();
        let ws_service_clone = app_state.ws_service.clone();

        if had_user {
            info!("Authenticated - initializing chat WebSocket connection");

            {
                let mut ws = ws_service_clone.borrow_mut();

                // Set up callbacks
                ws.set_on_open(Callback::from(move |_| {
                    info!("Chat WebSocket opened");
                }));

                let asc = app_state.clone();
                ws.set_on_close(Callback::from(move |_| {
                    info!("Chat WebSocket closed");
                    asc.dispatch(AppStateAction::SetError("Connection closed".to_string()));
                }));

                let asc = app_state.clone();
                ws.set_on_error(Callback::from(move |err| {
                    error!("Chat WebSocket error: {}", err);
                    asc.dispatch(AppStateAction::SetError(err));
                }));

                let asc = app_state.clone();
                let session_handle = session.clone();
                let uis = ui_state.clone();
                ws.set_on_message(Callback::from(move |msg: Message| {
                    asc.dispatch(AppStateAction::LoadingComplete);

                    // Clear explain loading for the parent message if this is an agent response
                    if msg.is_agent()
                        && let Some(parent_id) = msg.parent_id {
                            uis.dispatch(UIStateAction::ClearExplainLoading { message_id: parent_id });
                        }

                    let msg_clone = msg.clone();
                    match msg.content {
                        MessageContent::UserMessage { .. } => {
                            session_handle.dispatch(SessionAction::Domain(UserDomainAction::Message(
                                MessageAction::Add(msg_clone.clone()),
                            )));
                        }
                        MessageContent::AgentMessage { content } => {
                            check_rate_limit_error(&content.response, &asc);

                            // Dispatch to AppState for autoplay check (reads from AppState's own state)
                            asc.dispatch(AppStateAction::ProcessAgentMessage(msg_clone.clone()));

                            let mistakes = content.mistakes.clone().unwrap_or_default();
                            let explained = content.explained.clone().unwrap_or_default();
                            let translated = content.translated.clone().unwrap_or_default();
                            let exploratory = content.exploratory.clone().unwrap_or_default();

                            if !mistakes.is_empty()
                                || !explained.is_empty()
                                || !translated.is_empty()
                                || !exploratory.is_empty()
                            {
                                session_handle.dispatch(SessionAction::Domain(UserDomainAction::Learning(
                                    LearningAction::AddItems(
                                        mistakes,
                                        explained,
                                        translated,
                                        exploratory,
                                    ),
                                )));
                            }
                            if let Some(analysis) = content.analysis.clone() {
                                info!(
                                    "Received analysis with {} mistake scores, {} explained scores, {} translated scores, {} exploratory scores",
                                    analysis.mistake_scores.len(),
                                    analysis.explained_scores.len(),
                                    analysis.translated_scores.len(),
                                    analysis.exploratory_scores.len()
                                );
                                session_handle.dispatch(SessionAction::Domain(UserDomainAction::Learning(
                                    LearningAction::UpdateScores(analysis),
                                )));
                            }
                        }
                    }

                    session_handle.dispatch(SessionAction::Domain(UserDomainAction::Message(
                        MessageAction::Add(msg_clone.clone()),
                    )));
                }));

                let asc = app_state.clone();
                ws.set_on_state_change(Callback::from(move |new_state| {
                    info!("Chat connection state changed to: {:?}", new_state);
                    asc.dispatch(AppStateAction::SetConnectionState(new_state));

                    // Handle reconnecting state - the WebSocketService will attempt
                    // automatic reconnection, but we need to trigger it from the app layer
                    // since we can't easily call methods from within the async task
                    if matches!(new_state, ConnectionState::Reconnecting) {
                        // Schedule a reconnect attempt
                        let ws_clone = asc.ws_service.clone();
                        gloo::timers::callback::Timeout::new(1000, move || {
                            info!("Triggering reconnection from app layer");
                            ws_clone.borrow_mut().reconnect();
                        })
                        .forget();
                    }

                    // Retry pending saves when connected
                    if matches!(new_state, ConnectionState::Connected) {
                        asc.dispatch(AppStateAction::RetryPendingSaves);
                    }
                }));

                // Connect
                ws.connect();
            } // Drop the borrow here
        } else {
            info!("Not authenticated, skipping chat WebSocket connection");
        }

        // Cleanup - disconnect when effect re-runs or component unmounts
        move || {
            if had_user {
                info!("Disconnecting chat WebSocket");
                ws_service_clone.borrow_mut().disconnect();
            }
        }
    });
}

#[hook]
pub fn use_user_state_websocket(
    app_state: UseReducerHandle<AppState>,
    session: UseReducerHandle<SessionState>,
) {
    let current_user = app_state.current_user.clone();

    use_effect_with(current_user, move |user_opt| {
        let app_state = app_state.clone();
        let session = session.clone();
        let ws_service_clone = app_state.user_state_ws_service.clone();

        if let Some(user) = user_opt {
            info!(
                "User authenticated - initializing user state WebSocket connection for user: {}",
                user.id
            );
            let user_id = user.id;
            let mut ws = ws_service_clone.borrow_mut();

            ws.set_on_open(on_user_state_ws_open(app_state.clone(), user_id));
            ws.set_on_load_response(on_user_state_load_response(
                app_state.clone(),
                session.clone(),
            ));
            ws.set_on_save_response(on_user_state_save_response());
            ws.set_on_usage_stats_update(on_user_state_usage_stats_update(session.clone()));
            ws.connect();
        } else {
            info!("No user authenticated, skipping user state WebSocket connection");
        }

        let had_user = user_opt.is_some();
        move || {
            if had_user {
                info!("User state WebSocket cleanup");
            }
        }
    });
}

#[hook]
pub fn use_user_websocket(
    app_state: UseReducerHandle<AppState>,
    ui_state: UseReducerHandle<crate::app::app_state::UIState>,
    session: UseReducerHandle<SessionState>,
) {
    use_effect_with((), move |_| {
        let app_state = app_state.clone();
        let ui_state = ui_state.clone();
        let session = session.clone();

        info!("Initializing user WebSocket connection");

        let ws_clone = app_state.user_ws_service.clone();
        let mut ws = ws_clone.borrow_mut();

        ws.set_on_create_response(on_user_create_response(
            app_state.clone(),
            ui_state.clone(),
            session.clone(),
        ));
        ws.set_on_signin_response(on_user_signin_response(
            app_state.clone(),
            session.clone(),
        ));
        ws.set_on_validate_session_response(on_validate_session_response(
            app_state.clone(),
            session.clone(),
        ));

        let ws_clone_for_open = ws_clone.clone();
        ws.set_on_open(Callback::from(move |_| {
            info!("User WebSocket opened");

            // Check for session cookie and validate if exists
            if let Some(token) = crate::utils::cookies::get_session_token() {
                info!("Found session token, validating session");
                if let Err(e) = ws_clone_for_open.borrow().validate_session(token) {
                    error!("Failed to send session validation: {}", e);
                }
            } else {
                info!("No session token found");
            }
        }));

        ws.connect();

        move || {
            info!("User WebSocket cleanup");
        }
    });
}
