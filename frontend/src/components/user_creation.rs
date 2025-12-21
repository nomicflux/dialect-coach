use crate::app::app_callbacks::on_create_user_click;
use crate::app::app_state::{AppState, SessionState, UIState};
use dialect_coach_shared::models::{
    Dialect, InitialUserSettings, Language, LanguageLevel, UserGender,
    dialect_features,
};
use yew::prelude::*;

#[derive(Properties)]
pub struct UserCreationProps {
    pub app_state: UseReducerHandle<AppState>,
    pub ui_state: UseReducerHandle<UIState>,
    pub session: UseReducerHandle<SessionState>,
}

impl PartialEq for UserCreationProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[function_component(UserCreation)]
pub fn user_creation(props: &UserCreationProps) -> Html {
    let UserCreationProps { app_state, ui_state, session } = props;

    let username = use_state(String::new);
    let email = use_state(String::new);
    let invite_code = use_state(String::new);
    let password = use_state(String::new);
    let password_confirm = use_state(String::new);
    
    // Visibility toggles
    let show_password = use_state(|| false);
    let show_invite_code = use_state(|| false);
    
    // State for filtering
    let show_experimental = use_state(|| false);
    
    // Selection state
    let selected_language = use_state::<Option<Language>, _>(|| None);
    let selected_dialect = use_state::<Option<Dialect>, _>(|| None);
    let selected_level = use_state(|| LanguageLevel::B1);
    let selected_gender = use_state(|| UserGender::NonBinary);

    // Derived data
    let available_languages = {
        let show_exp = *show_experimental;
        Language::all()
            .into_iter()
            .filter(move |&lang| {
                if show_exp {
                    true
                } else {
                    // Check if language has ANY non-experimental dialects
                    let dialects = Dialect::all_dialects_for_language(lang);
                    dialects.into_iter().any(|d| !dialect_features(d).is_experimental)
                }
            })
            .collect::<Vec<_>>()
    };

    let available_dialects = {
        if let Some(lang) = *selected_language {
            let show_exp = *show_experimental;
             Dialect::all_dialects_for_language(lang)
                .into_iter()
                .filter(|&d| show_exp || !dialect_features(d).is_experimental)
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        }
    };

    // Handlers
    let on_toggle_experimental = {
        let show_experimental = show_experimental.clone();
        let selected_dialect = selected_dialect.clone();
        Callback::from(move |_| {
            let new_val = !*show_experimental;
            show_experimental.set(new_val);
            if !new_val {
                 if let Some(d) = *selected_dialect {
                     if dialect_features(d).is_experimental {
                         selected_dialect.set(None);
                     }
                 }
            }
        })
    };
    
    let toggle_password_visibility = {
        let show_password = show_password.clone();
        Callback::from(move |e: MouseEvent| {
            e.prevent_default();
            show_password.set(!*show_password);
        })
    };

    let toggle_invite_visibility = {
        let show_invite_code = show_invite_code.clone();
        Callback::from(move |e: MouseEvent| {
             e.prevent_default();
            show_invite_code.set(!*show_invite_code);
        })
    };

    let on_language_change = {
        let selected_language = selected_language.clone();
        let selected_dialect = selected_dialect.clone();
        Callback::from(move |e: Event| {
            if let Some(select) = e.target_dyn_into::<web_sys::HtmlSelectElement>() {
                let val = select.value();
                if val.is_empty() {
                    selected_language.set(None);
                } else {
                    let lang = match val.as_str() {
                        "Spanish" => Some(Language::Spanish),
                        "Arabic" => Some(Language::Arabic),
                        "French" => Some(Language::French),
                        "English" => Some(Language::English),
                        "Japanese" => Some(Language::Japanese),
                        _ => None,
                    };
                    selected_language.set(lang);
                }
                selected_dialect.set(None);
            }
        })
    };

    let on_dialect_change = {
        let selected_dialect = selected_dialect.clone();
        Callback::from(move |e: Event| {
            if let Some(select) = e.target_dyn_into::<web_sys::HtmlSelectElement>() {
                if let Some(dialect) = Dialect::from_id(&select.value()) {
                    selected_dialect.set(Some(dialect));
                } else {
                    selected_dialect.set(None);
                }
            }
        })
    };
    
    let passwords_match = *password == *password_confirm;
    
    let can_submit = !username.is_empty() 
        && !email.is_empty() 
        && !password.is_empty() 
        && passwords_match
        && !invite_code.is_empty()
        && selected_language.is_some() 
        && selected_dialect.is_some();

    let on_submit = {
        let app_state = app_state.clone();
        let ui_state = ui_state.clone();
        let session = session.clone();
        let username = username.clone();
        let email = email.clone();
        let invite_code = invite_code.clone();
        let password = password.clone();
        let selected_language = selected_language.clone();
        let selected_dialect = selected_dialect.clone();
        let selected_level = selected_level.clone();
        let selected_gender = selected_gender.clone();

        let create_user = on_create_user_click(app_state, ui_state, session);

        Callback::from(move |_| {
            if let (Some(lang), Some(dialect)) = (*selected_language, *selected_dialect) {
                 let settings = InitialUserSettings {
                    language: lang,
                    dialect: dialect,
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
            }
        })
    };
    
    // SVG Icons
    let eye_icon = html! {
        <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"></path>
            <circle cx="12" cy="12" r="3"></circle>
        </svg>
    };
    
    let eye_off_icon = html! {
        <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19m-6.72-1.07a3 3 0 1 1-4.24-4.24"></path>
            <line x1="1" y1="1" x2="23" y2="23"></line>
        </svg>
    };

    html! {
        <div class="user-creation-page">
            <div class="user-creation-card">
                <h2>{"Create Your Account"}</h2>
                
                <div class="user-creation-form">
                    <div class="form-group">
                        <input
                            class="input-field"
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
                    </div>

                    <div class="form-group">
                        <input
                            class="input-field"
                            type="email"
                            placeholder="Email Address"
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
                    </div>
                    
                    <div class="form-group">
                        <div class="input-wrapper">
                             <input
                                class="input-field"
                                type={if *show_password { "text" } else { "password" }}
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
                            <button class="toggle-visibility-btn" onclick={toggle_password_visibility.clone()}>
                                {if *show_password { eye_off_icon.clone() } else { eye_icon.clone() }}
                            </button>
                        </div>
                    </div>
                    
                     <div class="form-group">
                        <div class="input-wrapper">
                             <input
                                class="input-field"
                                type={if *show_password { "text" } else { "password" }}
                                placeholder="Confirm Password"
                                value={(*password_confirm).clone()}
                                oninput={
                                    let password_confirm = password_confirm.clone();
                                    Callback::from(move |e: InputEvent| {
                                        if let Some(input) = e.target_dyn_into::<web_sys::HtmlInputElement>() {
                                            password_confirm.set(input.value());
                                        }
                                    })
                                }
                            />
                            // Shared toggle with main password for UX simplicity or independent?
                            // Usually shared or independent. Let's make it follow the main toggle for now, 
                            // or we can add another toggle. 
                            // The request said "Fields should have option to show content". 
                            // Let's reuse the same toggle button logic for simplicity of UI density.
                             <button class="toggle-visibility-btn" onclick={toggle_password_visibility}>
                                {if *show_password { eye_off_icon.clone() } else { eye_icon.clone() }}
                            </button>
                        </div>
                         if !password.is_empty() && !password_confirm.is_empty() && !passwords_match {
                            <div class="error-message" style="text-align: left; font-size: 0.8em; margin-top: 4px;">
                                {"Passwords do not match"}
                            </div>
                        }
                    </div>

                    <div class="form-group">
                        <div class="input-wrapper">
                            <input
                                class="input-field"
                                type={if *show_invite_code { "text" } else { "password" }}
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
                            <button class="toggle-visibility-btn" onclick={toggle_invite_visibility}>
                                {if *show_invite_code { eye_off_icon } else { eye_icon }}
                            </button>
                        </div>
                    </div>

                    <div class="form-group">
                        <label class="checkbox-group">
                            <input 
                                type="checkbox" 
                                checked={*show_experimental}
                                onclick={on_toggle_experimental}
                                style="opacity: 0; position: absolute;" 
                            />
                            <div class="toggle-switch"></div>
                            <span class="checkbox-label">{"Show Experimental Dialects"}</span>
                        </label>
                    </div>

                    <div class="form-group">
                        <div class="select-wrapper">
                            <select
                                class="select-field"
                                onchange={on_language_change}
                            >
                                <option value="" selected={selected_language.is_none()} disabled=true>{"Select Language"}</option>
                                {
                                    for available_languages.iter().map(|lang| {
                                        let is_selected = *selected_language == Some(*lang);
                                        html! {
                                            <option value={lang.name().to_string()} selected={is_selected}>{lang.name()}</option>
                                        }
                                    })
                                }
                            </select>
                        </div>
                    </div>

                    <div class="form-group">
                         <div class="select-wrapper">
                            <select
                                class="select-field"
                                onchange={on_dialect_change}
                                disabled={selected_language.is_none()}
                            >
                                <option value="" selected={selected_dialect.is_none()} disabled=true>
                                    {if selected_language.is_none() { "Select a language first" } else { "Select Dialect" }}
                                </option>
                                {
                                    for available_dialects.iter().map(|dialect| {
                                        let is_selected = *selected_dialect == Some(*dialect);
                                        html! {
                                            <option value={dialect.id()} selected={is_selected}>{dialect.name()}</option>
                                        }
                                    })
                                }
                            </select>
                        </div>
                    </div>

                    <div class="form-group">
                         <div class="select-wrapper">
                            <select
                                class="select-field"
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
                        </div>
                    </div>

                    <div class="form-group">
                         <div class="select-wrapper">
                            <select
                                class="select-field"
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
                        </div>
                    </div>

                    <button 
                        class="btn btn-primary" 
                        onclick={on_submit}
                        disabled={!can_submit}
                    >
                        {"Create Account"}
                    </button>
                    
                    if !can_submit {
                         <div class="error-message">
                             {if !passwords_match && !password.is_empty() && !password_confirm.is_empty() {
                                 "Passwords must match."
                             } else {
                                 "Please fill in all fields to continue."
                             }}
                         </div>
                    }
                </div>
            </div>
        </div>
    }
}
