use crate::app::app_callbacks::{on_create_user_click, on_signin_click, on_signout_click};
use crate::app::app_state::{AppState, AppStateAction, OptionalUserState, UIState, UIStateAction};
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

#[function_component(Header)]
pub fn header(props: &HeaderProps) -> Html {
    let HeaderProps {
        app_state,
        ui_state,
        user_state,
    } = props.clone();

    html! {
        <header class="app-header">
            <div class="container">
                <div>
                    <h1 class="app-title">{"🎯 Dialect Coach"}</h1>
                    <p class="app-subtitle">{"Practice Spanish, Arabic, and French dialects with AI agents"}</p>
                </div>

                // User management section
                <div class="user-section">
                    {if let Some(user) = app_state.current_user.as_ref() {
                        html! {
                            <div class="user-signed-in">
                                <span>{format!("Signed in as: {}", user.username)}</span>
                                <button class="signout-button" onclick={on_signout_click(app_state.clone(), user_state.clone())}>
                                    {"Sign Out"}
                                </button>
                            </div>
                        }
                    } else {
                        html! {
                            <div class="user-forms">
                                <div class="user-form">
                                    <label>{"Create: "}</label>
                                    <input
                                        type="text"
                                        value={ui_state.create_username_input.clone()}
                                        oninput={{
                                            let ui_state = ui_state.clone();
                                            Callback::from(move |e: InputEvent| {
                                                if let Some(input) = e.target_dyn_into::<web_sys::HtmlInputElement>() {
                                                    ui_state.dispatch(UIStateAction::SetCreateUsernameInput(input.value()));
                                                }
                                            })
                                        }}
                                    />
                                    <button onclick={on_create_user_click(app_state.clone(), ui_state.clone(), user_state.clone())}>
                                        {"Create Account"}
                                    </button>
                                </div>
                                <div class="user-form">
                                    <label>{"Sign In: "}</label>
                                    <input
                                        type="text"
                                        value={ui_state.signin_username_input.clone()}
                                        oninput={{
                                            let ui_state = ui_state.clone();
                                            Callback::from(move |e: InputEvent| {
                                                if let Some(input) = e.target_dyn_into::<web_sys::HtmlInputElement>() {
                                                    ui_state.dispatch(UIStateAction::SetSigninUsernameInput(input.value()));
                                                }
                                            })
                                        }}
                                    />
                                    <button onclick={on_signin_click(app_state.clone(), ui_state.clone())}>
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
