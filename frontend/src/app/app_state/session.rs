use super::user::UserDomainAction;
use super::user::reducer::apply_user_state_action;
use dialect_coach_shared::UserState;
use std::rc::Rc;
use yew::prelude::*;

#[derive(Clone, PartialEq)]
pub enum SessionAction {
    Login(UserState),
    Logout,
    UpdateUser(UserState),
    Domain(UserDomainAction),
    Saved,
}

#[derive(Clone, PartialEq, Default)]
pub struct SessionState {
    pub user: Option<UserState>,
    pub needs_save: bool,
}

impl Reducible for SessionState {
    type Action = SessionAction;

    fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
        match action {
            SessionAction::Login(user) => SessionState {
                user: Some(user),
                needs_save: false,
            }
            .into(),
            SessionAction::Logout => SessionState {
                user: None,
                needs_save: false,
            }
            .into(),
            SessionAction::UpdateUser(user) => SessionState {
                user: Some(user),
                needs_save: false,
            }
            .into(),
            SessionAction::Domain(domain_action) => {
                if let Some(user) = &self.user {
                    // Convert strictly typed domain action to generic action for the reducer
                    let new_user_state = apply_user_state_action(user, domain_action.into());
                    SessionState {
                        user: Some(new_user_state),
                        needs_save: true,
                    }
                    .into()
                } else {
                    self
                }
            }
            SessionAction::Saved => SessionState {
                user: self.user.clone(),
                needs_save: false,
            }
            .into(),
        }
    }
}
