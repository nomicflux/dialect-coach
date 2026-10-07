use crate::app::app_callbacks::on_signin_click;
use crate::app::app_state::{
    AppState, AppStateAction, SessionState, UIState, UIStateAction, UserStateGamificationExt,
};
use crate::components::gamification::{FluencyBar, StreakDisplay};
use crate::components::{DashboardButton, NewConversationButton};
use crate::services::connection::ConnectionStatus;
use dialect_coach_shared::models::gamification::GamificationStats;
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

/// Class, dot and label for the connection indicator.
fn status_display(status: ConnectionStatus) -> (&'static str, &'static str, &'static str) {
    match status {
        ConnectionStatus::Connected => ("status-connected", "●", "Ready to chat!"),
        ConnectionStatus::Connecting => ("status-connecting", "⟳", "Connecting..."),
        ConnectionStatus::Reconnecting => ("status-reconnecting", "⟳", "Reconnecting..."),
    }
}

#[hook]
fn use_signin_callback(
    props: &HeaderProps,
    username: UseStateHandle<String>,
    password: UseStateHandle<String>,
) -> Callback<MouseEvent> {
    let app_state = props.app_state.clone();
    let ui_state = props.ui_state.clone();
    let session = props.session.clone();
    Callback::from(move |_| {
        on_signin_click(app_state.clone(), ui_state.clone(), session.clone())
            .emit(((*username).clone(), (*password).clone()));
    })
}

fn render_header_actions(props: &HeaderProps) -> Html {
    html! {
        <div class="header-actions">
            <DashboardButton app_state={props.app_state.clone()} ui_state={props.ui_state.clone()} />
            <NewConversationButton session={props.session.clone()} />
        </div>
    }
}

fn render_brand(is_visitor: bool) -> Html {
    html! {
        <div class="header-brand">
            <h1 class="app-title">{"🎯 Dialect Coach"}</h1>
            {if is_visitor {
                html! { <p class="app-subtitle">{"Practice Spanish, Arabic, and French dialects with AI agents"}</p> }
            } else { html! {} }}
        </div>
    }
}

fn render_status(props: &HeaderProps) -> Html {
    if props.app_state.current_user.is_none() && !props.ui_state.is_signing_in {
        return html! {};
    }
    let (class, dot, label) = status_display(props.app_state.connection_state);
    html! {
        <div class="connection-status">
            <span class={class} title={label}>
                <span class="status-dot">{dot}</span>
                <span class="status-label">{label}</span>
            </span>
        </div>
    }
}

fn render_signed_in(username: &str, stats: Option<GamificationStats>) -> Html {
    let badges = stats
        .map(|stats| {
            html! {
                <>
                    <FluencyBar xp={stats.xp} />
                    <StreakDisplay current_streak={stats.streak.current_streak} />
                </>
            }
        })
        .unwrap_or_default();
    html! {
        <div class="user-signed-in">
            {badges}
            <span class="username" title="Signed in">{username.to_owned()}</span>
        </div>
    }
}

fn input_setter(state: UseStateHandle<String>) -> Callback<InputEvent> {
    Callback::from(move |e: InputEvent| {
        if let Some(input) = e.target_dyn_into::<web_sys::HtmlInputElement>() {
            state.set(input.value());
        }
    })
}

fn render_sign_in_form(
    ui_state: &UseReducerHandle<UIState>,
    username: &UseStateHandle<String>,
    password: &UseStateHandle<String>,
    on_signin: Callback<MouseEvent>,
) -> Html {
    html! {
        <div class="user-forms">
            {render_create_button(ui_state)}
            <div class="user-form">
                <label>{"Sign In"}</label>
                <input type="text" placeholder="Username" value={(**username).clone()} oninput={input_setter(username.clone())} />
                <input type="password" placeholder="Password" value={(**password).clone()} oninput={input_setter(password.clone())} />
                <button onclick={on_signin}>{"Sign In"}</button>
            </div>
        </div>
    }
}

fn render_error_banner(app_state: &UseReducerHandle<AppState>) -> Html {
    let Some(err) = app_state.error_message.as_ref() else {
        return html! {};
    };
    let state = app_state.clone();
    let onclick = Callback::from(move |_: MouseEvent| state.dispatch(AppStateAction::ClearError));
    html! {
        <div class="error-banner">
            <span>{format!("⚠️ {}", err)}</span>
            <button class="error-close" {onclick}>{"×"}</button>
        </div>
    }
}

#[function_component(Header)]
pub fn header(props: &HeaderProps) -> Html {
    let username = use_state(String::new);
    let password = use_state(String::new);
    let on_signin = use_signin_callback(props, username.clone(), password.clone());
    html! {
        <header class="app-header">
            <div class="container">
                {render_header_actions(props)}
                {render_brand(props.app_state.current_user.is_none())}
                <div class="user-section">
                    {render_status(props)}
                    {render_user_account(props, &username, &password, on_signin)}
                </div>
                {render_error_banner(&props.app_state)}
            </div>
        </header>
    }
}

fn render_user_account(
    props: &HeaderProps,
    username: &UseStateHandle<String>,
    password: &UseStateHandle<String>,
    on_signin: Callback<MouseEvent>,
) -> Html {
    if let Some(user) = props.app_state.current_user.as_ref() {
        render_signed_in(
            &user.username,
            props.session.user.as_ref().map(|s| s.gamification_stats()),
        )
    } else {
        render_sign_in_form(&props.ui_state, username, password, on_signin)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_display_labels() {
        assert_eq!(
            status_display(ConnectionStatus::Connected).2,
            "Ready to chat!"
        );
        assert_eq!(
            status_display(ConnectionStatus::Connecting).2,
            "Connecting..."
        );
        assert_eq!(
            status_display(ConnectionStatus::Reconnecting).2,
            "Reconnecting..."
        );
    }
}
