use yew::prelude::*;

#[derive(Properties)]
pub struct WelcomeScreenProps {}

impl PartialEq for WelcomeScreenProps {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

#[function_component(WelcomeScreen)]
pub fn welcome_screen(_props: &WelcomeScreenProps) -> Html {
    html! {
        <div class="welcome-container">
            <h2>{"Welcome to Dialect Coach"}</h2>
            <p>{"Please sign in or create an account above to start practicing."}</p>
        </div>
    }
}
