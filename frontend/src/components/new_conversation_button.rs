use crate::app::app_state::user::{BranchAction, UserDomainAction};
use crate::app::app_state::{SessionAction, SessionState};
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct NewConversationButtonProps {
    pub session: UseReducerHandle<SessionState>,
}

#[function_component(NewConversationButton)]
pub fn new_conversation_button(props: &NewConversationButtonProps) -> Html {
    let NewConversationButtonProps { session } = props;

    if session.user.is_none() {
        return html! {};
    }

    let onclick = {
        let session = session.clone();
        Callback::from(move |_| {
            session.dispatch(SessionAction::Domain(UserDomainAction::Branch(
                BranchAction::CreateNew,
            )));
        })
    };

    html! {
        <button class="btn-dashboard-float btn-new-conversation" {onclick}>
            <span class="dashboard-icon">{"\u{2795}"}</span>
            <span>{"New Conversation"}</span>
        </button>
    }
}
