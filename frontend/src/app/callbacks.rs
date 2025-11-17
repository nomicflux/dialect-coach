use crate::app::app_helpers::extract_learning_items;
use crate::app::app_state::{
    AppState, AppStateAction, OptionalUserState, UIState, UIStateAction, UserStateAction,
};
use dialect_coach_shared::{
    AIActionRequest, AuthCredentials, PastLearningItems, UserMessageWithContext,
};
use log::{error, info};
use uuid::Uuid;
use yew::prelude::*;

pub fn on_send_message(
    app_state: UseReducerHandle<AppState>,
    user_state: UseReducerHandle<OptionalUserState>,
) -> Callback<String> {
    let app_state = app_state.clone();
    let user_state = user_state.clone();

    Callback::from(move |content: String| {
        let state = match user_state.0.as_ref() {
            Some(s) => s,
            None => return,
        };

        info!("Sending message: {}", content);

        let session_id = (*app_state).session_id().unwrap_or_else(Uuid::new_v4);
        let msg = state.create_user_msg(session_id, &content);
        user_state.dispatch(UserStateAction::AddMessage(msg.clone()));

        // Extract learning items from user state
        let (past_mistakes, past_explained, past_translated, past_exploratory) =
            extract_learning_items(&user_state);

        // Get active branch context
        let active_branch_id = state.active_branch_id;
        let context_messages = state
            .get_active_branch_messages()
            .into_iter()
            .cloned()
            .collect();

        // Build UserMessageWithContext
        let msg_with_context = UserMessageWithContext::new(
            state.user_id,
            msg,
            PastLearningItems {
                mistakes: past_mistakes,
                explained: past_explained,
                translated: past_translated,
                exploratory: past_exploratory,
            },
            active_branch_id,
            context_messages,
            state.learning_goals.clone(),
        );

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
    user_state: UseReducerHandle<OptionalUserState>,
) -> Callback<()> {
    let app_state = app_state.clone();
    let user_state = user_state.clone();
    Callback::from(move |_| {
        if let Some(state) = user_state.0.as_ref() {
            let new_value = !state.tts_enabled;
            user_state.dispatch(UserStateAction::ToggleTTS);
            app_state.dispatch(AppStateAction::NotifyTTSEnabled(new_value));
        }
    })
}

pub fn on_create_user_click(
    app_state: UseReducerHandle<AppState>,
    ui_state: UseReducerHandle<UIState>,
    _user_state: UseReducerHandle<OptionalUserState>,
) -> Callback<MouseEvent> {
    Callback::from(move |_: MouseEvent| {
        let username = ui_state.create_username_input.clone();
        let email = ui_state.create_email_input.clone();
        let invite_code = ui_state.create_invite_code_input.clone();
        let credentials = AuthCredentials::InviteCode(invite_code);

        if let Err(e) = app_state
            .user_ws_service
            .borrow()
            .create_user(username, email, credentials)
        {
            error!("Failed to create user: {}", e);
            app_state.dispatch(AppStateAction::SetError(format!(
                "Failed to create user: {}",
                e
            )));
        }
    })
}

pub fn on_signin_click(
    app_state: UseReducerHandle<AppState>,
    ui_state: UseReducerHandle<UIState>,
) -> Callback<MouseEvent> {
    Callback::from(move |_: MouseEvent| {
        let username = ui_state.signin_username_input.clone();

        if let Err(e) = app_state.user_ws_service.borrow().sign_in(username) {
            error!("Failed to sign in: {}", e);
            app_state.dispatch(AppStateAction::SetError(format!(
                "Failed to sign in: {}",
                e
            )));
        }
    })
}

fn clear_create_form_inputs(ui_state: &UseReducerHandle<UIState>) {
    ui_state.dispatch(UIStateAction::ClearCreateUsernameInput);
    ui_state.dispatch(UIStateAction::ClearCreateEmailInput);
    ui_state.dispatch(UIStateAction::ClearCreateInviteCodeInput);
}

pub fn on_user_create_response(
    app_state: UseReducerHandle<AppState>,
    ui_state: UseReducerHandle<UIState>,
    _user_state: UseReducerHandle<OptionalUserState>,
) -> Callback<Result<dialect_coach_shared::User, String>> {
    Callback::from(
        move |result: Result<dialect_coach_shared::User, String>| match result {
            Ok(user) => {
                info!("User created successfully: {}", user.username);
                clear_create_form_inputs(&ui_state);
                ui_state.dispatch(UIStateAction::HideUserCreationPage);
                app_state.dispatch(AppStateAction::SetUser(user.clone()));
                app_state.dispatch(AppStateAction::CreateSession(Uuid::new_v4()));

                // Trigger sign-in to load/create UserState (reuses normal sign-in flow)
                if let Err(e) = app_state
                    .user_ws_service
                    .borrow()
                    .sign_in(user.username.clone())
                {
                    error!("Failed to sign in after user creation: {}", e);
                    app_state.dispatch(AppStateAction::SetError(format!(
                        "User created but sign in failed: {}",
                        e
                    )));
                }
            }
            Err(e) => {
                error!("Failed to create user: {}", e);
                app_state.dispatch(AppStateAction::SetError(format!("Create failed: {}", e)));
            }
        },
    )
}

fn clear_signin_form_inputs(ui_state: &UseReducerHandle<UIState>) {
    ui_state.dispatch(UIStateAction::ClearSigninUsernameInput);
    ui_state.dispatch(UIStateAction::ClearSigninInviteCodeInput);
}

pub fn on_user_signin_response(
    app_state: UseReducerHandle<AppState>,
    ui_state: UseReducerHandle<UIState>,
    _user_state: UseReducerHandle<OptionalUserState>,
) -> Callback<Result<dialect_coach_shared::User, String>> {
    Callback::from(
        move |result: Result<dialect_coach_shared::User, String>| match result {
            Ok(user) => {
                info!("Signed in successfully as: {}", user.username);
                clear_signin_form_inputs(&ui_state);
                app_state.dispatch(AppStateAction::SetUser(user.clone()));
                app_state.dispatch(AppStateAction::CreateSession(Uuid::new_v4()));
            }
            Err(e) => {
                error!("Sign in failed: {}", e);
                app_state.dispatch(AppStateAction::SetError(format!("Sign in failed: {}", e)));
            }
        },
    )
}

pub fn on_signout_click(
    app_state: UseReducerHandle<AppState>,
    user_state: UseReducerHandle<OptionalUserState>,
) -> Callback<MouseEvent> {
    Callback::from(move |_: MouseEvent| {
        info!("User signed out, clearing user state");
        app_state.dispatch(AppStateAction::DestroySession);
        user_state.dispatch(UserStateAction::ClearUserState);
        app_state.dispatch(AppStateAction::ClearUser);
    })
}

pub fn on_auto_start(
    app_state: UseReducerHandle<AppState>,
    user_state: UseReducerHandle<OptionalUserState>,
) -> Callback<()> {
    Callback::from(move |_| {
        let state = match user_state.0.as_ref() {
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
    user_state: UseReducerHandle<OptionalUserState>,
) -> Callback<Uuid> {
    Callback::from(move |parent_message_id: Uuid| {
        let state = match user_state.0.as_ref() {
            Some(s) => s,
            None => return,
        };

        let session_id = app_state.session_id().unwrap_or_else(Uuid::new_v4);
        let user_id = state.user_id;

        info!("Sending continue branch request");

        match app_state.ws_service.borrow().send_ai_action(
            AIActionRequest::ContinueBranch { parent_message_id },
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
    user_state: UseReducerHandle<OptionalUserState>,
) -> Callback<Uuid> {
    Callback::from(move |message_id: Uuid| {
        let state = match user_state.0.as_ref() {
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

pub fn on_translate_message(
    app_state: UseReducerHandle<AppState>,
    user_state: UseReducerHandle<OptionalUserState>,
) -> Callback<Uuid> {
    Callback::from(move |message_id: Uuid| {
        let state = match user_state.0.as_ref() {
            Some(s) => s,
            None => {
                error!("No user state for translate message");
                return;
            }
        };

        let session_id = app_state.session_id().unwrap_or_else(Uuid::new_v4);
        let user_id = state.user_id;

        info!("Sending translate message request");

        match app_state.ws_service.borrow().send_ai_action(
            AIActionRequest::TranslateMessage { message_id },
            session_id,
            user_id,
        ) {
            Ok(_) => {
                info!("Translate message request sent successfully");
                app_state.dispatch(AppStateAction::SetLoading);
            }
            Err(e) => {
                error!("Failed to send translate message request: {}", e);
                app_state.dispatch(AppStateAction::SetError(format!(
                    "Failed to translate message: {}",
                    e
                )));
            }
        }
    })
}
