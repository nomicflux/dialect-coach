use crate::app::OptionalUserState;
use dialect_coach_shared::UserState;
use gloo::timers::callback::Timeout;
use log::info;
use std::rc::Rc;
use yew::prelude::*;

const DEBOUNCE_MS: u32 = 2000;

#[hook]
pub fn use_debounced_save<F>(user_state: &UseReducerHandle<OptionalUserState>, save_fn: F) -> Callback<()>
where
    F: Fn(&UserState) + 'static + Clone,
{
    let user_state_for_force = user_state.clone();
    let pending_timeout = use_state(|| Option::<Timeout>::None);
    let save_fn_rc = use_state(|| Rc::new(save_fn));

    // Watch for changes and schedule debounced save
    {
        let user_state = user_state.clone();
        let pending_timeout = pending_timeout.clone();
        let save_fn = (*save_fn_rc).clone();

        use_effect_with(user_state, move |state_handle| {
            // Cancel any pending timeout by dropping it
            pending_timeout.set(None);

            // Only schedule save if user_state exists
            if let Some(state) = state_handle.0.as_ref() {
                // Schedule new timeout
                let state_clone = state.clone();
                let timeout = Timeout::new(DEBOUNCE_MS, move || {
                    info!("Debounced save triggered");
                    save_fn(&state_clone);
                });

                pending_timeout.set(Some(timeout));
            }

            || ()
        });
    }

    // Return force_save_now callback
    let save_fn_for_callback = (*save_fn_rc).clone();
    use_callback(user_state_for_force, move |_, state_handle| {
        if let Some(state) = state_handle.0.as_ref() {
            info!("Force save triggered");
            save_fn_for_callback(state);
        }
    })
}
