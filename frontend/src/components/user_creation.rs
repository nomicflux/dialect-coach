use crate::app::app_callbacks::on_create_user_click;
use crate::app::app_state::{AppState, OptionalUserState, UIState, UIStateAction};
use yew::prelude::*;

#[derive(Properties)]
pub struct UserCreationProps {
    pub app_state: UseReducerHandle<AppState>,
    pub ui_state: UseReducerHandle<UIState>,
    pub user_state: UseReducerHandle<OptionalUserState>,
}

impl PartialEq for UserCreationProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

fn username_field(ui_state: &UseReducerHandle<UIState>) -> Html {
    let ui_state = ui_state.clone();
    let value = ui_state.create_username_input.clone();
    html! {
        <input
            type="text"
            placeholder="Username"
            value={value}
            oninput={Callback::from(move |e: InputEvent| {
                if let Some(input) = e.target_dyn_into::<web_sys::HtmlInputElement>() {
                    ui_state.dispatch(UIStateAction::SetCreateUsernameInput(input.value()));
                }
            })}
        />
    }
}

fn email_field(ui_state: &UseReducerHandle<UIState>) -> Html {
    let ui_state = ui_state.clone();
    let value = ui_state.create_email_input.clone();
    html! {
        <input
            type="email"
            placeholder="Email"
            value={value}
            oninput={Callback::from(move |e: InputEvent| {
                if let Some(input) = e.target_dyn_into::<web_sys::HtmlInputElement>() {
                    ui_state.dispatch(UIStateAction::SetCreateEmailInput(input.value()));
                }
            })}
        />
    }
}

fn invite_code_field(ui_state: &UseReducerHandle<UIState>) -> Html {
    let ui_state = ui_state.clone();
    let value = ui_state.create_invite_code_input.clone();
    html! {
        <input
            type="text"
            placeholder="Invite Code"
            value={value}
            oninput={Callback::from(move |e: InputEvent| {
                if let Some(input) = e.target_dyn_into::<web_sys::HtmlInputElement>() {
                    ui_state.dispatch(UIStateAction::SetCreateInviteCodeInput(input.value()));
                }
            })}
        />
    }
}

fn submit_button(
    app_state: &UseReducerHandle<AppState>,
    ui_state: &UseReducerHandle<UIState>,
    user_state: &UseReducerHandle<OptionalUserState>,
) -> Html {
    let onclick = on_create_user_click(app_state.clone(), ui_state.clone(), user_state.clone());
    html! {
        <button class="btn btn--primary" {onclick}>{"Create Account"}</button>
    }
}

#[function_component(UserCreation)]
pub fn user_creation(props: &UserCreationProps) -> Html {
    let UserCreationProps {
        app_state,
        ui_state,
        user_state,
    } = props;

    html! {
        <div class="user-creation-page">
            <h2>{"Create Your Account"}</h2>
            <div class="user-creation-form">
                {username_field(ui_state)}
                {email_field(ui_state)}
                {invite_code_field(ui_state)}
                {submit_button(app_state, ui_state, user_state)}
            </div>
        </div>
    }
}
