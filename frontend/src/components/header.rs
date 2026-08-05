use crate::app::app_callbacks::{on_signin_click, on_signout_click};
use crate::app::app_state::{
    AppState, AppStateAction, SessionState, UIState, UIStateAction, UserStateGamificationExt,
};
use crate::components::gamification::{FluencyBar, StreakDisplay};
use crate::services::websocket::ConnectionState;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct HeaderProps {
    pub app_state: UseReducerHandle<AppState>,
    pub ui_state: UseReducerHandle<UIState>,
    pub session: UseReducerHandle<SessionState>,
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

#[function_component(Header)]
pub fn header(props: &HeaderProps) -> Html {
    let HeaderProps {
        app_state,
        ui_state,
        session,
    } = props;

    let signin_username = use_state(String::new);
    let signin_password = use_state(String::new);

    let on_signin = {
        let app_state = app_state.clone();
        let ui_state = ui_state.clone();
        let username = signin_username.clone();
        let password = signin_password.clone();
        Callback::from(move |_: MouseEvent| {
            on_signin_click(app_state.clone(), ui_state.clone())
                .emit(((*username).clone(), (*password).clone()));
        })
    };

    // Dot and label are separate nodes so narrow viewports can keep the
    // state indicator and drop the prose.
    let (status_class, status_dot, status_label) = match app_state.connection_state {
        ConnectionState::Connected => ("status-connected", "●", "Ready to chat!"),
        ConnectionState::Connecting => ("status-connecting", "⟳", "Connecting..."),
        ConnectionState::Reconnecting => ("status-reconnecting", "⟳", "Reconnecting..."),
        ConnectionState::Disconnected => ("status-disconnected", "○", "Disconnected"),
        ConnectionState::Failed => ("status-failed", "✖", "Connection Failed"),
    };

    html! {
        <header class="app-header">
            <div class="container">
                // Dashboard button moved to separate component
                <div class="header-brand">
                    <h1 class="app-title">{"🎯 Dialect Coach"}</h1>
                    // The pitch is for visitors deciding whether to sign up; once
                    // signed in it is a permanent band of dead space above the chat.
                    {if app_state.current_user.is_none() {
                        html! { <p class="app-subtitle">{"Practice Spanish, Arabic, and French dialects with AI agents"}</p> }
                    } else {
                        html! {}
                    }}
                </div>

                <div class="user-section">
                    // The chat WebSocket is only opened once a user is signed in
                    // (use_chat_websocket skips connecting otherwise), so before
                    // sign-in this indicator would report a disconnection that is
                    // simply the pre-auth state.
                    {if app_state.current_user.is_some() {
                        html! {
                            <div class="connection-status">
                                <span class={status_class} title={status_label}>
                                    <span class="status-dot">{status_dot}</span>
                                    <span class="status-label">{status_label}</span>
                                </span>
                            </div>
                        }
                    } else {
                        html! {}
                    }}
                    {if let Some(user) = app_state.current_user.as_ref() {
                        if let Some(stats) = session.user.as_ref().map(|s| s.gamification_stats()) {
                            html! {
                                <div class="user-signed-in">
                                    <FluencyBar xp={stats.xp} />
                                    <StreakDisplay current_streak={stats.streak.current_streak} />
                                    <span class="username" title="Signed in">{user.username.clone()}</span>
                                    <button class="signout-button" onclick={on_signout_click(app_state.clone(), session.clone())}>
                                        {"Sign Out"}
                                    </button>
                                </div>
                            }
                        } else {
                             html! {
                                <div class="user-signed-in">
                                    <span class="username" title="Signed in">{user.username.clone()}</span>
                                    <button class="signout-button" onclick={on_signout_click(app_state.clone(), session.clone())}>
                                        {"Sign Out"}
                                    </button>
                                </div>
                            }
                        }
                    } else {
                        html! {
                            <div class="user-forms">
                                {render_create_button(ui_state)}
                                <div class="user-form">
                                    <label>{"Sign In"}</label>
                                    <input
                                        type="text"
                                        placeholder="Username"
                                        value={(*signin_username).clone()}
                                        oninput={
                                            let signin_username = signin_username.clone();
                                            Callback::from(move |e: InputEvent| {
                                                if let Some(input) = e.target_dyn_into::<web_sys::HtmlInputElement>() {
                                                    signin_username.set(input.value());
                                                }
                                            })
                                        }
                                    />
                                    <input
                                        type="password"
                                        placeholder="Password"
                                        value={(*signin_password).clone()}
                                        oninput={
                                            let signin_password = signin_password.clone();
                                            Callback::from(move |e: InputEvent| {
                                                if let Some(input) = e.target_dyn_into::<web_sys::HtmlInputElement>() {
                                                    signin_password.set(input.value());
                                                }
                                            })
                                        }
                                    />
                                    <button onclick={on_signin}>
                                        {"Sign In"}
                                    </button>
                                </div>
                            </div>
                        }
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
