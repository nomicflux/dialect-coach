use crate::app::app_state::user::{BranchAction, UserDomainAction};
use crate::app::app_state::{SessionAction, SessionState};
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct NewConversationButtonProps {
    pub session: UseReducerHandle<SessionState>,
}

fn new_conversation_click(session: UseReducerHandle<SessionState>) -> Callback<MouseEvent> {
    Callback::from(move |_| {
        session.dispatch(SessionAction::Domain(UserDomainAction::Branch(
            BranchAction::CreateNew,
        )));
    })
}

#[function_component(NewConversationButton)]
pub fn new_conversation_button(props: &NewConversationButtonProps) -> Html {
    if props.session.user.is_none() {
        return html! {};
    }
    let onclick = new_conversation_click(props.session.clone());
    html! {
        <button class="btn-dashboard-float btn-new-conversation" aria-label="New Conversation" {onclick}>
            <span class="dashboard-icon">{"\u{2795}"}</span>
            <span class="dashboard-label">{"New Conversation"}</span>
        </button>
    }
}
