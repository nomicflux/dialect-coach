use crate::app::app_helpers::extract_learning_items;
use crate::app::app_state::{
    AppState, AppStateAction, OptionalUserState, UIState, UIStateAction, UserStateAction,
};
use dialect_coach_shared::{AuthCredentials, PastLearningItems, UserMessageWithContext};
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

pub fn on_prompt_click(
    app_state: UseReducerHandle<AppState>,
    user_state: UseReducerHandle<OptionalUserState>,
    ui_state: UseReducerHandle<UIState>,
) -> Callback<(String, String)> {
    let app_state = app_state.clone();
    let user_state = user_state.clone();
    let ui_state = ui_state.clone();

    Callback::from(move |(button_id, english_phrase): (String, String)| {
        let state = match user_state.0.as_ref() {
            Some(s) => s,
            None => return,
        };

        info!(
            "Translating prompt from button '{}': {}",
            button_id, english_phrase
        );

        // Set loading state for specific button
        ui_state.dispatch(UIStateAction::PushTranslatingButton(button_id));
        app_state.dispatch(AppStateAction::ClearError);

        let ui_state = ui_state.clone();
        let current_dialect = state.current_dialect();
        let formality = state.formality;
        let app_state = app_state.clone();

        // Start async translation
        wasm_bindgen_futures::spawn_local(async move {
            match app_state
                .translation_service
                .translate_phrase(&english_phrase, current_dialect, Some(formality))
                .await
            {
                Ok(translated) => {
                    info!(
                        "Translation success: '{}' -> '{}'",
                        english_phrase, translated
                    );
                    ui_state.dispatch(UIStateAction::EnterInputPrompt(translated));
                    ui_state.dispatch(UIStateAction::ClearTranslatingButton);
                }
                Err(e) => {
                    error!("Translation failed: {}", e);
                    // Fallback to English phrase on error
                    ui_state.dispatch(UIStateAction::EnterInputPrompt(english_phrase));
                    app_state.dispatch(AppStateAction::SetError(format!(
                        "Translation failed, using English phrase: {}",
                        e
                    )));
                    ui_state.dispatch(UIStateAction::ClearTranslatingButton);
                }
            }
        });
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
