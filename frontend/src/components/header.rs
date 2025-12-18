use crate::app::app_callbacks::{on_signin_click, on_signout_click};
use crate::app::app_state::{
    AppState, AppStateAction, SessionState, UIState, UIStateAction, UserStateGamificationExt,
};
use crate::components::gamification::{FluencyBar, StreakDisplay};
use crate::services::websocket::ConnectionState;
use yew::prelude::*;

#[derive(Properties)]
pub struct HeaderProps {
    pub app_state: UseReducerHandle<AppState>,
    pub ui_state: UseReducerHandle<UIState>,
    pub session: UseReducerHandle<SessionState>,
}

impl PartialEq for HeaderProps {
    fn eq(&self, _other: &Self) -> bool {
        false
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
        let username = signin_username.clone();
        let password = signin_password.clone();
        Callback::from(move |_: MouseEvent| {
            on_signin_click(app_state.clone()).emit(((*username).clone(), (*password).clone()));
        })
    };

    html! {
        <header class="app-header">
            <div class="container">
                <div>
                    <h1 class="app-title">{"🎯 Dialect Coach"}</h1>
                    <p class="app-subtitle">{"Practice Spanish, Arabic, and French dialects with AI agents"}</p>
                </div>

                <div class="user-section">
                    <button
                        class="btn-icon"
                        onclick={{
                            let ui_state = ui_state.clone();
                            Callback::from(move |_| ui_state.dispatch(UIStateAction::ToggleDrawer))
                        }}
                        title="Open Study Tools"
                    >
                        {"☰"}
                    </button>
                    {if let Some(user) = app_state.current_user.as_ref() {
                        if let Some(stats) = session.user.as_ref().map(|s| s.gamification_stats()) {
                            html! {
                                <div class="user-signed-in">
                                    <FluencyBar xp={stats.xp} />
                                    <StreakDisplay current_streak={stats.streak.current_streak} />
                                    <span>{format!("Signed in as: {}", user.username)}</span>
                                    <button class="signout-button" onclick={on_signout_click(app_state.clone(), session.clone())}>
                                        {"Sign Out"}
                                    </button>
                                </div>
                            }
                        } else {
                             html! {
                                <div class="user-signed-in">
                                    <span>{format!("Signed in as: {}", user.username)}</span>
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
