use crate::app::app_callbacks::on_create_user_click;
use crate::app::app_state::{AppState, SessionState};
use dialect_coach_shared::models::{
    Dialect, InitialUserSettings, Language, LanguageLevel, UserGender,
};
use yew::prelude::*;

#[derive(Properties)]
pub struct UserCreationProps {
    pub app_state: UseReducerHandle<AppState>,
    pub session: UseReducerHandle<SessionState>,
}

impl PartialEq for UserCreationProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[function_component(UserCreation)]
pub fn user_creation(props: &UserCreationProps) -> Html {
    let UserCreationProps { app_state, session } = props;

    let username = use_state(String::new);
    let email = use_state(String::new);
    let invite_code = use_state(String::new);
    let password = use_state(String::new);
    let selected_language = use_state(|| Language::Spanish);
    let selected_dialect = use_state(|| Dialect::SpanishArgentinian);
    let selected_level = use_state(|| LanguageLevel::B1);
    let selected_gender = use_state(|| UserGender::NonBinary);

    let on_submit = {
        let app_state = app_state.clone();
        let session = session.clone();
        let username = username.clone();
        let email = email.clone();
        let invite_code = invite_code.clone();
        let password = password.clone();
        let selected_language = selected_language.clone();
        let selected_dialect = selected_dialect.clone();
        let selected_level = selected_level.clone();
        let selected_gender = selected_gender.clone();

        let create_user = on_create_user_click(app_state, session);

        Callback::from(move |_| {
            let settings = InitialUserSettings {
                language: *selected_language,
                dialect: *selected_dialect,
                level: *selected_level,
                gender: *selected_gender,
            };
            create_user.emit((
                (*username).clone(),
                (*email).clone(),
                (*password).clone(),
                (*invite_code).clone(),
                Some(settings),
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
                <select
                    onchange={
                        let selected_language = selected_language.clone();
                        Callback::from(move |e: Event| {
                            if let Some(select) = e.target_dyn_into::<web_sys::HtmlSelectElement>() {
                                match select.value().as_str() {
                                    "spanish" => selected_language.set(Language::Spanish),
                                    "arabic" => selected_language.set(Language::Arabic),
                                    "french" => selected_language.set(Language::French),
                                    "english" => selected_language.set(Language::English),
                                    "japanese" => selected_language.set(Language::Japanese),
                                    _ => {}
                                }
                            }
                        })
                    }
                >
                    <option value="spanish">{"Spanish"}</option>
                    <option value="arabic">{"Arabic"}</option>
                    <option value="french">{"French"}</option>
                    <option value="english">{"English"}</option>
                    <option value="japanese">{"Japanese"}</option>
                </select>
                <select
                    onchange={
                        let selected_dialect = selected_dialect.clone();
                        Callback::from(move |e: Event| {
                            if let Some(select) = e.target_dyn_into::<web_sys::HtmlSelectElement>() {
                                match select.value().as_str() {
                                    "argentinian" => selected_dialect.set(Dialect::SpanishArgentinian),
                                    "mexican" => selected_dialect.set(Dialect::SpanishMexican),
                                    "castilian" => selected_dialect.set(Dialect::SpanishCastilian),
                                    _ => {}
                                }
                            }
                        })
                    }
                >
                    <option value="argentinian">{"Spanish - Argentinian"}</option>
                    <option value="mexican">{"Spanish - Mexican"}</option>
                    <option value="castilian">{"Spanish - Castilian"}</option>
                </select>
                <select
                    onchange={
                        let selected_level = selected_level.clone();
                        Callback::from(move |e: Event| {
                            if let Some(select) = e.target_dyn_into::<web_sys::HtmlSelectElement>() {
                                match select.value().as_str() {
                                    "a1" => selected_level.set(LanguageLevel::A1),
                                    "a2" => selected_level.set(LanguageLevel::A2),
                                    "b1" => selected_level.set(LanguageLevel::B1),
                                    "b2" => selected_level.set(LanguageLevel::B2),
                                    "c1" => selected_level.set(LanguageLevel::C1),
                                    "c2" => selected_level.set(LanguageLevel::C2),
                                    _ => {}
                                }
                            }
                        })
                    }
                >
                    <option value="a1">{"A1 - Beginner"}</option>
                    <option value="a2">{"A2 - Elementary"}</option>
                    <option value="b1" selected=true>{"B1 - Intermediate"}</option>
                    <option value="b2">{"B2 - Upper Intermediate"}</option>
                    <option value="c1">{"C1 - Advanced"}</option>
                    <option value="c2">{"C2 - Proficient"}</option>
                </select>
                <select
                    onchange={
                        let selected_gender = selected_gender.clone();
                        Callback::from(move |e: Event| {
                            if let Some(select) = e.target_dyn_into::<web_sys::HtmlSelectElement>() {
                                match select.value().as_str() {
                                    "male" => selected_gender.set(UserGender::Male),
                                    "female" => selected_gender.set(UserGender::Female),
                                    "nonbinary" => selected_gender.set(UserGender::NonBinary),
                                    _ => {}
                                }
                            }
                        })
                    }
                >
                    <option value="male">{"Male"}</option>
                    <option value="female">{"Female"}</option>
                    <option value="nonbinary" selected=true>{"Non-binary"}</option>
                </select>
                <button class="btn btn--primary" onclick={on_submit}>{"Create Account"}</button>
            </div>
        </div>
    }
}
