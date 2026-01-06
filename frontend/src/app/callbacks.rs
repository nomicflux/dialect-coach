use crate::app::app_state::user::UserDomainAction;
use crate::app::app_state::{
    AppState, AppStateAction, MessageAction, SessionAction, SessionState, SettingsAction, UIState,
    UIStateAction,
};
use dialect_coach_shared::{
    AIActionRequest, AuthCredentials, InitialUserSettings, UserMessageWithContext, UserState,
};
use log::{error, info};
use uuid::Uuid;
use yew::prelude::*;

pub fn on_send_message(
    app_state: UseReducerHandle<AppState>,
    session: UseReducerHandle<SessionState>,
) -> Callback<String> {
    let app_state = app_state.clone();
    let session = session.clone();

    Callback::from(move |content: String| {
        let state = match session.user.as_ref() {
            Some(s) => s,
            None => return,
        };

        info!("Sending message: {}", content);

        let session_id = (*app_state).session_id().unwrap_or_else(Uuid::new_v4);
        let msg = state.create_user_msg(session_id, &content);
        session.dispatch(SessionAction::Domain(UserDomainAction::Message(
            MessageAction::Add(msg.clone()),
        )));

        // Extract learning items and goals using shared model logic
        let past_items = state.get_past_learning_items(&state.selected_dialect);
        let filtered_goals = state
            .get_learning_goals_for_dialect(&state.selected_dialect)
            .into_iter()
            .cloned()
            .collect();

        // Get active branch context
        let active_branch_id = state.active_branch_id;
        let context_messages = state
            .get_active_branch_messages()
            .into_iter()
            .cloned()
            .collect();

        // Build UserMessageWithContext
        let msg_with_context = UserMessageWithContext::builder(state.user_id, msg)
            .past_learning_items(past_items)
            .active_branch_id(active_branch_id)
            .active_plan(state.active_plan())
            .context_messages(context_messages)
            .learning_goals(filtered_goals)
            .user_gender(state.user_gender)
            .language_option(state.current_language_option())
            .language_level(state.current_language_level())
            .build();

        // Send through WebSocket
        match app_state
            .ws_service
            .borrow()
            .send_message(&msg_with_context)
        {
            Ok(_) => {
                info!("Message sent successfully");
                app_state.dispatch(AppStateAction::SetLoading);
                app_state.dispatch(AppStateAction::ClearError);
            }
            Err(e) => {
                error!("Failed to send message: {}", e);
                app_state.dispatch(AppStateAction::SetError(format!("Failed to send: {}", e)));
            }
        }
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

pub fn on_create_user_click(
    app_state: UseReducerHandle<AppState>,
    ui_state: UseReducerHandle<UIState>,
    _session: UseReducerHandle<SessionState>,
) -> Callback<(String, String, String, String, Option<InitialUserSettings>)> {
    Callback::from(
        move |(username, email, password, invite_code, initial_settings): (
            String,
            String,
            String,
            String,
            Option<InitialUserSettings>,
        )| {
            let credentials = AuthCredentials::InviteCode(invite_code);

            app_state.dispatch(AppStateAction::StorePendingInitialSettings(
                initial_settings.clone(),
            ));

            ui_state.dispatch(UIStateAction::SetSignInLoading(true));

            if let Err(e) = app_state.user_ws_service.borrow().create_user(
                username,
                email,
                credentials,
                password,
                initial_settings,
            ) {
                error!("Failed to create user: {}", e);
                ui_state.dispatch(UIStateAction::SetSignInLoading(false));
                app_state.dispatch(AppStateAction::SetError(format!(
                    "Failed to create user: {}",
                    e
                )));
            }
        },
    )
}

pub fn on_signin_click(
    app_state: UseReducerHandle<AppState>,
    ui_state: UseReducerHandle<UIState>,
) -> Callback<(String, String)> {
    Callback::from(move |(username, password): (String, String)| {
        ui_state.dispatch(UIStateAction::SetSignInLoading(true));
        if let Err(e) = app_state
            .user_ws_service
            .borrow()
            .sign_in(username, password)
        {
            error!("Failed to sign in: {}", e);
            ui_state.dispatch(UIStateAction::SetSignInLoading(false));
            app_state.dispatch(AppStateAction::SetError(format!(
                "Failed to sign in: {}",
                e
            )));
        }
    })
}

pub fn on_signin_response(
    app_state: UseReducerHandle<AppState>,
    ui_state: UseReducerHandle<UIState>,
    session: UseReducerHandle<SessionState>,
) -> Callback<Result<(dialect_coach_shared::User, UserState, String), String>> {
    Callback::from(
        move |result: Result<(dialect_coach_shared::User, UserState, String), String>| match result
        {
            Ok((user, user_state, token)) => {
                info!("Sign in successful: {}", user.username);
                crate::utils::cookies::set_session_token(&token);
                ui_state.dispatch(UIStateAction::HideUserCreationPage);
                ui_state.dispatch(UIStateAction::SetSignInLoading(false));
                app_state.dispatch(AppStateAction::SetUser(user));
                session.dispatch(SessionAction::Login(user_state));
                app_state.dispatch(AppStateAction::CreateSession(Uuid::new_v4()));
            }
            Err(e) => {
                error!("Sign in failed: {}", e);
                ui_state.dispatch(UIStateAction::SetSignInLoading(false));
                app_state.dispatch(AppStateAction::SetError(format!("Sign in failed: {}", e)));
            }
        },
    )
}

pub fn on_signout_click(
    app_state: UseReducerHandle<AppState>,
    session: UseReducerHandle<SessionState>,
) -> Callback<MouseEvent> {
    Callback::from(move |_: MouseEvent| {
        info!("User signed out, clearing user state and session cookie");
        crate::utils::cookies::clear_session_token();
        app_state.dispatch(AppStateAction::DestroySession);
        session.dispatch(SessionAction::Logout);
        app_state.dispatch(AppStateAction::ClearUser);
    })
}

pub fn on_auto_start(
    app_state: UseReducerHandle<AppState>,
    session: UseReducerHandle<SessionState>,
) -> Callback<()> {
    Callback::from(move |_| {
        let state = match session.user.as_ref() {
            Some(s) => s,
            None => return,
        };

        let session_id = app_state.session_id().unwrap_or_else(Uuid::new_v4);
        let user_id = state.user_id;

        info!("Sending auto-start conversation request");

        match app_state.ws_service.borrow().send_ai_action(
            AIActionRequest::StartConversation,
            session_id,
            user_id,
        ) {
            Ok(_) => {
                info!("Auto-start request sent successfully");
                app_state.dispatch(AppStateAction::SetLoading);
            }
            Err(e) => {
                error!("Failed to send auto-start request: {}", e);
                app_state.dispatch(AppStateAction::SetError(format!(
                    "Failed to start conversation: {}",
                    e
                )));
            }
        }
    })
}

pub fn on_continue_branch(
    app_state: UseReducerHandle<AppState>,
    session: UseReducerHandle<SessionState>,
) -> Callback<Uuid> {
    Callback::from(move |parent_message_id: Uuid| {
        let state = match session.user.as_ref() {
            Some(s) => s,
            None => return,
        };

        let session_id = app_state.session_id().unwrap_or_else(Uuid::new_v4);
        let user_id = state.user_id;

        info!("Sending continue branch request");

        // Use shared logic to build context
        let context = state.build_action_context();

        match app_state.ws_service.borrow().send_ai_action(
            AIActionRequest::ContinueBranch {
                parent_message_id,
                context: Box::new(context),
            },
            session_id,
            user_id,
        ) {
            Ok(_) => {
                info!("Continue branch request sent successfully");
                app_state.dispatch(AppStateAction::SetLoading);
            }
            Err(e) => {
                error!("Failed to send continue branch request: {}", e);
                app_state.dispatch(AppStateAction::SetError(format!(
                    "Failed to continue conversation: {}",
                    e
                )));
            }
        }
    })
}

pub fn on_explain_message(
    app_state: UseReducerHandle<AppState>,
    session: UseReducerHandle<SessionState>,
    ui_state: UseReducerHandle<UIState>,
) -> Callback<Uuid> {
    Callback::from(move |message_id: Uuid| {
        let state = match session.user.as_ref() {
            Some(s) => s,
            None => {
                error!("No user state for explain message");
                return;
            }
        };

        let session_id = app_state.session_id().unwrap_or_else(Uuid::new_v4);
        let user_id = state.user_id;

        info!("Sending explain message request");

        match app_state.ws_service.borrow().send_ai_action(
            AIActionRequest::ExplainMessage { message_id },
            session_id,
            user_id,
        ) {
            Ok(_) => {
                info!("Explain message request sent successfully");
                app_state.dispatch(AppStateAction::SetLoading);
                ui_state.dispatch(UIStateAction::SetExplainLoading { message_id });
            }
            Err(e) => {
                error!("Failed to send explain message request: {}", e);
                app_state.dispatch(AppStateAction::SetError(format!(
                    "Failed to explain message: {}",
                    e
                )));
            }
        }
    })
}
