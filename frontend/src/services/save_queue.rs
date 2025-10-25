use dialect_coach_shared::UserState;
use log::{info, warn};
use std::cell::RefCell;

use super::persistence::save_user_state;

/// Queue for pending UserState saves that failed
pub struct PendingSaveQueue {
    pending: RefCell<Option<UserState>>,
}

impl PendingSaveQueue {
    pub fn new() -> Self {
        Self {
            pending: RefCell::new(None),
        }
    }

    /// Enqueue a state for retry (replaces any existing pending state)
    pub fn enqueue(&self, state: UserState) {
        let had_pending = self.pending.borrow().is_some();
        *self.pending.borrow_mut() = Some(state);
        if !had_pending {
            info!("Queued user state save for retry");
        }
    }

    /// Retry all pending saves
    pub fn retry_all(&self) -> Result<(), String> {
        if let Some(state) = self.pending.borrow_mut().take() {
            info!("Retrying pending user state save");
            match save_user_state(&state) {
                Ok(()) => {
                    info!("Retry successful");
                    Ok(())
                }
                Err(e) => {
                    warn!("Retry failed: {}", e);
                    self.enqueue(state);
                    Err(e)
                }
            }
        } else {
            Ok(())
        }
    }

    /// Check if there are pending saves
    pub fn has_pending(&self) -> bool {
        self.pending.borrow().is_some()
    }
}

impl Default for PendingSaveQueue {
    fn default() -> Self {
        Self::new()
    }
}
