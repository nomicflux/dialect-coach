use crate::app::app_callbacks::on_create_user_click;
use crate::app::app_state::{AppState, OptionalUserState, UIState};
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

#[function_component(UserCreation)]
pub fn user_creation(props: &UserCreationProps) -> Html {
    let UserCreationProps {
        app_state,
        ui_state: _ui_state, // Unused for input values now
        user_state,
    } = props;

    let username = use_state(String::new);
    let email = use_state(String::new);
    let invite_code = use_state(String::new);
    let password = use_state(String::new);

    let on_submit = {
        let app_state = app_state.clone();
        let ui_state = props.ui_state.clone();
        let user_state = user_state.clone();
        let username = username.clone();
        let email = email.clone();
        let invite_code = invite_code.clone();
        let password = password.clone();

        let create_user = on_create_user_click(app_state, ui_state, user_state);

        Callback::from(move |_| {
            create_user.emit((
                (*username).clone(),
                (*email).clone(),
                (*password).clone(),
                (*invite_code).clone(),
            ));
        })
    };

    html! {
        <div class="user-creation-page">
            <h2>{"Create Your Account"}</h2>
            <div class="user-creation-form">
                <input
                    type="text"
                    placeholder="Username"
                    value={(*username).clone()}
                    oninput={
                        let username = username.clone();
                        Callback::from(move |e: InputEvent| {
                            if let Some(input) = e.target_dyn_into::<web_sys::HtmlInputElement>() {
                                username.set(input.value());
                            }
                        })
                    }
                />
                <input
                    type="email"
                    placeholder="Email"
                    value={(*email).clone()}
                    oninput={
                        let email = email.clone();
                        Callback::from(move |e: InputEvent| {
                            if let Some(input) = e.target_dyn_into::<web_sys::HtmlInputElement>() {
                                email.set(input.value());
                            }
                        })
                    }
                />
                <input
                    type="text"
                    placeholder="Invite Code"
                    value={(*invite_code).clone()}
                    oninput={
                        let invite_code = invite_code.clone();
                        Callback::from(move |e: InputEvent| {
                            if let Some(input) = e.target_dyn_into::<web_sys::HtmlInputElement>() {
                                invite_code.set(input.value());
                            }
                        })
                    }
                />
                <input
                    type="password"
                    placeholder="Password"
                    value={(*password).clone()}
                    oninput={
                        let password = password.clone();
                        Callback::from(move |e: InputEvent| {
                            if let Some(input) = e.target_dyn_into::<web_sys::HtmlInputElement>() {
                                password.set(input.value());
                            }
                        })
                    }
                />
                <button class="btn btn--primary" onclick={on_submit}>{"Create Account"}</button>
            </div>
        </div>
    }
}
