 Here’s a concrete, atomic “add == save” plan with file-specific changes and diff snippets. The idea: the reducer sets a “needs save” flag on any add; a single effect observes that flag and immediately saves. No per-path callbacks; any new
  add path inherits this.

  1) Add a dirty flag to OptionalUserState (reducer-owned)

  - File: frontend/src/app/app_state.rs
  - Add a pub needs_save: bool to OptionalUserState (or wrap in a struct), default false.
  - In reduce, when handling UserStateAction::AddLearningItems (and any future add variants), set needs_save = true on the returned OptionalUserState.
  - Clear the flag only when a save succeeds (see step 3).

  Diff sketch:

  *** Update File: frontend/src/app/app_state.rs
  @@
   #[derive(Clone, PartialEq)]
  -pub struct OptionalUserState(pub Option<UserState>);
  +pub struct OptionalUserState {
  +    pub state: Option<UserState>,
  +    pub needs_save: bool,
  +}

   impl Reducible for OptionalUserState {
       type Action = UserStateAction;
  @@
  -        match action {
  -            UserStateAction::ReplaceUserState(mut new_state) => {
  -                new_state.rebuild_branches_from_history();
  -                OptionalUserState(Some(new_state)).into()
  -            }
  -            UserStateAction::ClearUserState => OptionalUserState(None).into(),
  -            _ => match &self.0 {
  -                Some(state) => {
  -                    let prepared = prepare_state_for_action(state);
  -                    OptionalUserState(Some(apply_user_state_action(&prepared, action))).into()
  -                }
  -                None => self,
  -            },
  -        }
  +        match action {
  +            UserStateAction::ReplaceUserState(mut new_state) => {
  +                new_state.rebuild_branches_from_history();
  +                OptionalUserState { state: Some(new_state), needs_save: false }.into()
  +            }
  +            UserStateAction::ClearUserState => OptionalUserState { state: None, needs_save: false }.into(),
  +            _ => match &self.state {
  +                Some(state) => {
  +                    let prepared = prepare_state_for_action(state);
  +                    let mut next_state = apply_user_state_action(&prepared, action);
  +                    let mut needs_save = self.needs_save;
  +                    if matches!(action, UserStateAction::AddLearningItems(..)) {
  +                        needs_save = true;
  +                    }
  +                    OptionalUserState { state: Some(next_state.clone()), needs_save }.into()
  +                }
  +                None => self,
  +            },
  +        }
       }
   }

  2) Add an effect to immediately save when needs_save is set

  - File: frontend/src/app.rs
  - After initializing user_state, add a use_effect_with on user_state.needs_save.
  - If needs_save is true and user_state.state is Some, call user_state_ws_service.save_user_state. On error, dispatch AppStateAction::QueuePendingSave(cloned_state). On success, dispatch a new action to clear needs_save.

  Add a new action to clear the flag:

  - File: frontend/src/app/app_state.rs
  - Enum UserStateAction: add MarkSaved.
  - In OptionalUserState::reduce, on MarkSaved, set needs_save = false (leave state untouched).

  Diff sketch:

  *** Update File: frontend/src/app/app_state.rs
  @@
   pub enum UserStateAction {
  +    MarkSaved,
       AddMessage(Message),
  @@
  -            _ => match &self.state {
  +            UserStateAction::MarkSaved => OptionalUserState { state: self.state.clone(), needs_save: false }.into(),
  +            _ => match &self.state {
  *** Update File: frontend/src/app.rs
  @@
       // Immediate save effect for learning-item additions
       {
           let app_state = app_state.clone();
           let user_state = user_state.clone();
           use_effect_with((user_state.needs_save, user_state.state.clone()), move |(needs_save, state_opt)| {
               if !*needs_save {
                   return || ();
               }
               if let Some(state) = state_opt {
                   let ws = app_state.user_state_ws_service.borrow();
                   match ws.save_user_state(&state) {
                       Ok(()) => {
                           app_state.dispatch(AppStateAction::QueuePendingSave(state.clone())); // optional: remove if not needed
                           user_state.dispatch(UserStateAction::MarkSaved);
                       }
                       Err(e) => {
                           error!("Immediate save failed: {}", e);
                           app_state.dispatch(AppStateAction::QueuePendingSave(state.clone()));
                           // keep needs_save true so next successful retry clears it
                       }
                   }
               }
               || ()
           });
       }

  (Note: remove the QueuePendingSave on success; keep it only on error. The snippet shows both; adjust accordingly.)

  3) Update all references to OptionalUserState.field access

  - Replace .0 with .state in files that read the user state:
      - frontend/src/app.rs
      - frontend/src/components/main_content.rs
      - frontend/src/app/user_state/callbacks.rs
      - Any other .0 usages.
  - No per-path wiring is needed; the reducer flag and global effect handle all adds.

  Example spot fix:

  -    let user_state = use_reducer(|| {
  -        info!("App starting with no user state");
  -        OptionalUserState(None)
  -    });
  +    let user_state = use_reducer(|| {
  +        info!("App starting with no user state");
  +        OptionalUserState { state: None, needs_save: false }
  +    });
  @@
  -                } else if user_state.0.is_some() {
  +                } else if user_state.state.is_some() {

  4) Ensure translation modal path still works without extra callbacks

  - No change needed to the modal or learning panel; they already dispatch AddLearningItems. The reducer sets needs_save = true, the effect saves immediately. Any new add path that dispatches AddLearningItems inherits this.

  5) (Optional) Keep debounced save for other state changes

  - You can retain use_debounced_save for non-learning-item mutations, but learning items no longer depend on it. If you want to remove debounce entirely, keep the same immediate effect but broaden the flag to “any state change” instead of
    just learning items.

  Summary of files touched

  - frontend/src/app/app_state.rs: add needs_save, add MarkSaved action, set flag on AddLearningItems, clear on MarkSaved.
  - frontend/src/app.rs: add effect watching needs_save to call save_user_state; adjust initial OptionalUserState construction.
  - Other files: adjust .0 to .state for OptionalUserState accesses (search/replace).

  This makes every AddLearningItems action automatically trigger an immediate save via a single, central effect—no per-path callbacks—and any future add path will be covered.
