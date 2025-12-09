pub mod app_state;
pub use app_state::OptionalUserState;
use app_state::{AppState, AppStateAction, UIState};

#[path = "app/helpers.rs"]
pub mod app_helpers;

#[path = "app/callbacks.rs"]
pub mod app_callbacks;

#[path = "app/websocket_hooks.rs"]
pub mod app_websocket_hooks;

#[path = "app/user_state/callbacks.rs"]
pub mod user_state_callbacks;

pub use app_state::callbacks;
pub use dialect_coach_shared::{LearningItem, LearningItemType, UserState};

use log::{error, info};
use yew::prelude::*;

use crate::components::{Header, MainContent, UserCreation, WelcomeScreen};
use crate::hooks::use_debounced_save;

use app_websocket_hooks::{use_chat_websocket, use_user_state_websocket, use_user_websocket};

#[function_component(App)]
pub fn app() -> Html {
    let app_state = use_reducer(AppState::default);
    let ui_state = use_reducer(UIState::default);

    // No user state until authentication
    let user_state = use_reducer(|| {
        info!("App starting with no user state");
        OptionalUserState {
            state: None,
            needs_save: false,
        }
    });

    // Set up debounced auto-save via WebSocket with retry queue
    let app_state_for_save = app_state.clone();
    let _force_save = use_debounced_save(&user_state, move |state| {
        let ws_service = app_state_for_save.user_state_ws_service.borrow();
        match ws_service.save_user_state(state) {
            Ok(()) => {
                info!("UserState save request sent via WebSocket");
            }
            Err(e) => {
                error!("Failed to send UserState save: {}", e);
                app_state_for_save
                    .dispatch(AppStateAction::QueuePendingSave(Box::new(state.clone())));
            }
        }
    });

    // WebSocket hooks
    use_chat_websocket(app_state.clone(), user_state.clone(), ui_state.clone());
    use_user_state_websocket(app_state.clone(), user_state.clone());
    use_user_websocket(app_state.clone(), ui_state.clone(), user_state.clone());

    // Auto-dismiss error messages after 5 seconds
    {
        let app_state = app_state.clone();
        let error_msg = app_state.error_message.clone();
        use_effect_with(error_msg, move |msg| {
            let timeout = msg.as_ref().map(|_| {
                let app_state = app_state.clone();
                gloo::timers::callback::Timeout::new(5000, move || {
                    app_state.dispatch(AppStateAction::ClearError);
                })
            });
            move || drop(timeout)
        });
    }

    html! {
        <div class="app">
            <Header
                app_state={app_state.clone()}
                ui_state={ui_state.clone()}
                user_state={user_state.clone()}
            />

            <main
                class="app-main"
                data-sidebar-collapsed={if ui_state.sidebar_collapsed { "true" } else { "false" }}
                data-learning-panel-collapsed={if ui_state.learning_panel_collapsed { "true" } else { "false" }}
            >
                {if ui_state.show_user_creation_page {
                    html! {
                        <UserCreation
                            app_state={app_state.clone()}
                            ui_state={ui_state.clone()}
                            user_state={user_state.clone()}
                        />
                    }
                } else if user_state.state.is_some() {
                    html! {
                        <MainContent
                            app_state={app_state.clone()}
                            ui_state={ui_state.clone()}
                            user_state={user_state.clone()}
                        />
                    }
                } else {
                    html! {
                        <WelcomeScreen />
                    }
                }}
            </main>
        </div>
    }
}
