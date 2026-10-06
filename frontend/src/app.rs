pub mod app_state;
use app_state::{AppState, AppStateAction, SessionState, UIState};

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

use log::info;
use yew::prelude::*;

use crate::components::{
    DashboardButton, Header, LoadingScreen, MainContent, NewConversationButton, UserCreation,
    WelcomeScreen,
};
use crate::hooks::use_debounced_save;

use app_state::callbacks::{keep_user_state, save_user_state};
use app_websocket_hooks::use_connection;

#[function_component(App)]
pub fn app() -> Html {
    let app_state = use_reducer(AppState::default);
    let ui_state = use_reducer(UIState::default);

    // No user state until authentication
    let session = use_reducer(|| {
        info!("App starting with no user state");
        SessionState {
            user: None,
            needs_save: false,
        }
    });

    let _force_save = use_debounced_save(
        &session,
        keep_user_state(&app_state),
        save_user_state(&app_state, &session),
    );

    use_connection(app_state.clone(), session.clone(), ui_state.clone());

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
            <crate::components::icons::NeonAssets />
            <Header
                app_state={app_state.clone()}
                ui_state={ui_state.clone()}
                session={session.clone()}
            />

            <main
                class="app-main"
                data-learning-panel-collapsed={if ui_state.learning_panel_collapsed { "true" } else { "false" }}
            >
                <div class="dashboard-buttons-container">
                    <DashboardButton
                        app_state={app_state.clone()}
                        ui_state={ui_state.clone()}
                    />
                    <NewConversationButton
                        session={session.clone()}
                    />
                </div>
                {if ui_state.is_signing_in {
                    html! { <LoadingScreen /> }
                } else if ui_state.show_user_creation_page {
                    html! {
                        <UserCreation
                            app_state={app_state.clone()}
                            ui_state={ui_state.clone()}
                            session={session.clone()}
                        />
                    }
                } else if session.user.is_some() {
                    html! {
                        <MainContent
                            app_state={app_state.clone()}
                            ui_state={ui_state.clone()}
                            session={session.clone()}
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
