use crate::app::app_state::SessionState;
use dialect_coach_shared::UserState;
use gloo::timers::callback::Timeout;
use log::info;
use std::rc::Rc;
use yew::prelude::*;

const DEBOUNCE_MS: u32 = 500;

type StateFn = Rc<dyn Fn(&UserState)>;

/// Every unsaved change goes to `keep_fn` at once, and to `save_fn` once changes
/// pause for `DEBOUNCE_MS`. Returns a callback that saves immediately.
#[hook]
pub fn use_debounced_save<K, F>(
    session: &UseReducerHandle<SessionState>,
    keep_fn: K,
    save_fn: F,
) -> Callback<()>
where
    K: Fn(&UserState) + 'static,
    F: Fn(&UserState) + 'static,
{
    let keep_fn = use_state(|| Rc::new(keep_fn) as StateFn);
    let save_fn = use_state(|| Rc::new(save_fn) as StateFn);
    use_save_on_change(session, (*keep_fn).clone(), (*save_fn).clone());
    let save_fn = (*save_fn).clone();
    use_callback(session.clone(), move |_, state_handle| {
        if let Some(user) = state_handle.user.as_ref() {
            info!("Force save triggered");
            save_fn(user);
        }
    })
}

#[hook]
fn use_save_on_change(
    session: &UseReducerHandle<SessionState>,
    keep_fn: StateFn,
    save_fn: StateFn,
) {
    let pending_timeout = use_state(|| Option::<Timeout>::None);
    use_effect_with(session.clone(), move |state_handle| {
        // Cancel any pending save by dropping it
        pending_timeout.set(None);
        if let (true, Some(user)) = (state_handle.needs_save, state_handle.user.clone()) {
            keep_fn(&user);
            let timeout = Timeout::new(DEBOUNCE_MS, move || {
                info!("Debounced save triggered");
                save_fn(&user);
            });
            pending_timeout.set(Some(timeout));
        }
    });
}
