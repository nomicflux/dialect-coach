use crate::app::app_callbacks::{on_signin_click, on_signout_click};
use crate::app::app_state::{
    AppState, AppStateAction, OptionalUserState, UIState, UIStateAction, UserStateGamificationExt,
};
use crate::components::gamification::{FluencyBar, StreakDisplay};
use crate::services::websocket::ConnectionState;
use yew::prelude::*;

#[derive(Properties)]
pub struct HeaderProps {
    pub app_state: UseReducerHandle<AppState>,
    pub ui_state: UseReducerHandle<UIState>,
    pub user_state: UseReducerHandle<OptionalUserState>,
}

impl PartialEq for HeaderProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

fn create_input_field(
    value: String,
    action: impl Fn(String) -> UIStateAction + 'static,
    ui_state: &UseReducerHandle<UIState>,
    input_type: String,
    placeholder: String,
) -> Html {
    let ui_state = ui_state.clone();
    html! {
        <input
            type={input_type}
            placeholder={placeholder}
            value={value}
            oninput={Callback::from(move |e: InputEvent| {
                if let Some(input) = e.target_dyn_into::<web_sys::HtmlInputElement>() {
                    ui_state.dispatch(action(input.value()));
                }
            })}
        />
    }
}

fn render_create_button(ui_state: &UseReducerHandle<UIState>) -> Html {
    let ui_state = ui_state.clone();
    html! {
        <button
            class="btn btn--primary"
            onclick={Callback::from(move |_: MouseEvent| {
                ui_state.dispatch(UIStateAction::ShowUserCreationPage);
            })}
        >
            {"Create Account"}
        </button>
    }
}

fn render_signin_form(
    app_state: &UseReducerHandle<AppState>,
    ui_state: &UseReducerHandle<UIState>,
) -> Html {
    html! {
        <div class="user-form">
            <label>{"Sign In"}</label>
            {create_input_field(
                ui_state.signin_username_input.clone(),
                UIStateAction::SetSigninUsernameInput,
                ui_state,
                "text".to_string(),
                "Username".to_string()
            )}
            {create_input_field(
                ui_state.signin_password_input.clone(),
                UIStateAction::SetSigninPasswordInput,
                ui_state,
                "password".to_string(),
                "Password".to_string()
            )}
            <button onclick={on_signin_click(app_state.clone(), ui_state.clone())}>
                {"Sign In"}
            </button>
        </div>
    }
}

#[function_component(Header)]
pub fn header(props: &HeaderProps) -> Html {
    let HeaderProps {
        app_state,
        ui_state,
        user_state,
    } = props;

    html! {
        <header class="app-header">
            <div class="container">
                <div>
                    <h1 class="app-title">{"🎯 Dialect Coach"}</h1>
                    <p class="app-subtitle">{"Practice Spanish, Arabic, and French dialects with AI agents"}</p>
                </div>

                <div class="user-section">
                    {if let Some(user) = app_state.current_user.as_ref() {
                        if let Some(stats) = user_state.state.as_ref().map(|s| s.gamification_stats()) {
                            html! {
                                <div class="user-signed-in">
                                    <FluencyBar xp={stats.xp} />
                                    <StreakDisplay current_streak={stats.streak.current_streak} />
                                    <span>{format!("Signed in as: {}", user.username)}</span>
                                    <button class="signout-button" onclick={on_signout_click(app_state.clone(), user_state.clone())}>
                                        {"Sign Out"}
                                    </button>
                                </div>
                            }
                        } else {
                             html! {
                                <div class="user-signed-in">
                                    <span>{format!("Signed in as: {}", user.username)}</span>
                                    <button class="signout-button" onclick={on_signout_click(app_state.clone(), user_state.clone())}>
                                        {"Sign Out"}
                                    </button>
                                </div>
                            }
                        }
                    } else {
                        html! {
                            <div class="user-forms">
                                {render_create_button(ui_state)}
                                {render_signin_form(app_state, ui_state)}
                            </div>
                        }
                    }}
                </div>

                // Connection status
                <div class="connection-status">
                    {match app_state.connection_state {
                        ConnectionState::Connected => html! { <span class="status-connected">{"● Ready to chat!"}</span> },
                        ConnectionState::Connecting => html! { <span class="status-connecting">{"⟳ Connecting..."}</span> },
                        ConnectionState::Reconnecting => html! { <span class="status-reconnecting">{"⟳ Reconnecting..."}</span> },
                        ConnectionState::Disconnected => html! { <span class="status-disconnected">{"○ Disconnected"}</span> },
                        ConnectionState::Failed => html! { <span class="status-failed">{"✖ Connection Failed"}</span> },
                    }}
                </div>

                // Error display - shown for both authenticated and unauthenticated states
                {if let Some(err) = (app_state.error_message).as_ref() {
                    html! {
                        <div class="error-banner">
                            <span>{format!("⚠️ {}", err)}</span>
                            <button class="error-close" onclick={{
                                let app_state = app_state.clone();
                                Callback::from(move |_: MouseEvent| {
                                    app_state.dispatch(AppStateAction::ClearError);
                                })
                            }}>
                                {"×"}
                            </button>
                        </div>
                    }
                } else {
                    html! {}
                }}
            </div>
        </header>
    }
}
