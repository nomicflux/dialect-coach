use crate::app::app_state::SessionState;
use dialect_coach_shared::UserState;
use gloo::timers::callback::Timeout;
use log::info;
use std::rc::Rc;
use yew::prelude::*;

const DEBOUNCE_MS: u32 = 500;

#[hook]
pub fn use_debounced_save<F>(
    session: &UseReducerHandle<SessionState>,
    save_fn: F,
) -> Callback<()>
where
    F: Fn(&UserState) + 'static + Clone,
{
    let session_for_force = session.clone();
    let pending_timeout = use_state(|| Option::<Timeout>::None);
    let save_fn_rc = use_state(|| Rc::new(save_fn));

    // Watch for changes and schedule debounced save
    {
        let session = session.clone();
        let pending_timeout = pending_timeout.clone();
        let save_fn = (*save_fn_rc).clone();

        use_effect_with(session, move |state_handle| {
            // Cancel any pending timeout by dropping it
            pending_timeout.set(None);

            // Only schedule save if user exists and needs_save is true
            if let (true, Some(user)) = (state_handle.needs_save, state_handle.user.as_ref()) {
                // Schedule new timeout
                let user_clone = user.clone();
                let timeout = Timeout::new(DEBOUNCE_MS, move || {
                    info!("Debounced save triggered");
                    save_fn(&user_clone);
                });
                pending_timeout.set(Some(timeout));
            }

            || ()
        });
    }

    // Return force_save_now callback
    let save_fn_for_callback = (*save_fn_rc).clone();
    use_callback(session_for_force, move |_, state_handle| {
        if let Some(user) = state_handle.user.as_ref() {
            info!("Force save triggered");
            save_fn_for_callback(user);
        }
    })
}
