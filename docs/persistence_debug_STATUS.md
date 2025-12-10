# Status: Plan Persistence Debugging

**Current Blocker**: Learning Plan changes (Title, Review Steps) revert upon page refresh.
**User Action**: User manually clicks "Save Changes".
**Result**: Logs say "saving", but data is not persisted.

## Verified Components
- **Backend Persistence (`Sled`)**: Functional. Other user state saves correctly.
- **Backend Handler (`handle_save_user_state`)**: Functional.
- **Debounce Hook**: Functional.

## Suspected Root Cause
The `UpdateLanguagePlan` action in the frontend reducer is likely **not triggering the `needs_save` flag**, or the data payload being dispatched is incorrect (e.g., mismatched IDs causing the update to be ignored).

## Debugging Plan
1.  **Confirm Reducer Logic**: Ensure `UserStateAction::UpdateLanguagePlan` falls through to the catch-all that sets `needs_save: true`.
2.  **Trace Data Payload**: We added logging to `app_state.rs`. We need to see if "Updating plan: ..." actually prints.
    - If it prints: The state *is* updating in memory. The issue is `needs_save` or the WebSocket transmission.
    - If it does NOT print: The ID look-up failed. This implies `PlanCreate` might be generating a NEW ID for an existing plan, or we are finding the wrong plan.

## Next Step
Check the logs from the `UpdateLanguagePlan` reducer (already added).
If `PlanCreate` messes up the ID, the reducer won't find the plan to update, causing a silent failure.
