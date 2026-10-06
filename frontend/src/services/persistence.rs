//! What this browser keeps so a refresh loses nothing: the user's state until
//! the server confirms a save of it, and coach requests until they resolve.

use dialect_coach_shared::{ChatRequest, ClientEnvelope, ClientMessage, RequestId, UserState};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use uuid::Uuid;
use web_sys::window;

/// The user's state with changes the server has not confirmed.
#[derive(Serialize, Deserialize)]
struct UnsavedState {
    state: UserState,
    /// The save request carrying exactly this state, once one is submitted.
    saved_by: Option<RequestId>,
}

/// A coach request kept until it resolves.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredRequest {
    pub id: RequestId,
    pub request: ChatRequest,
    /// Written to the socket; after a refresh its reply can no longer arrive.
    pub sent: bool,
}

impl StoredRequest {
    pub fn new(request: ChatRequest) -> Self {
        Self {
            id: Uuid::new_v4(),
            request,
            sent: false,
        }
    }

    pub fn envelope(&self) -> ClientEnvelope {
        ClientEnvelope {
            id: self.id,
            body: ClientMessage::Chat(self.request.clone()),
        }
    }
}

pub fn add(outbox: &mut Vec<StoredRequest>, stored: StoredRequest) {
    outbox.push(stored);
}

pub fn mark_sent(outbox: &mut [StoredRequest], id: RequestId) {
    for stored in outbox.iter_mut().filter(|stored| stored.id == id) {
        stored.sent = true;
    }
}

pub fn remove(outbox: &mut Vec<StoredRequest>, id: RequestId) {
    outbox.retain(|stored| stored.id != id);
}

/// The state to sign in with: kept changes win, but usage is the server's count.
pub fn prefer_unsaved(server: UserState, unsaved: Option<UserState>) -> UserState {
    match unsaved {
        Some(state) => UserState {
            usage_stats: server.usage_stats,
            ..state
        },
        None => server,
    }
}

fn unsaved_key(user_id: Uuid) -> String {
    format!("dialect_coach_unsaved:{}", user_id)
}

fn outbox_key(user_id: Uuid) -> String {
    format!("dialect_coach_outbox:{}", user_id)
}

pub fn save_user_state(
    user_id: Uuid,
    state: &UserState,
    saved_by: Option<RequestId>,
) -> Result<(), String> {
    let unsaved = UnsavedState {
        state: state.clone(),
        saved_by,
    };
    write(&unsaved_key(user_id), &unsaved)
}

pub fn load_user_state(user_id: Uuid) -> Result<Option<UserState>, String> {
    let unsaved: Option<UnsavedState> = read(&unsaved_key(user_id))?;
    Ok(unsaved.map(|unsaved| unsaved.state))
}

/// Forget the kept state if save `id` carried it; a later change keeps it.
pub fn clear_user_state_if(user_id: Uuid, id: RequestId) -> Result<(), String> {
    let key = unsaved_key(user_id);
    let unsaved: Option<UnsavedState> = read(&key)?;
    match unsaved {
        Some(unsaved) if unsaved.saved_by == Some(id) => storage()?
            .remove_item(&key)
            .map_err(|e| format!("Failed to remove from localStorage: {:?}", e)),
        _ => Ok(()),
    }
}

pub fn store_outbox(user_id: Uuid, outbox: &[StoredRequest]) -> Result<(), String> {
    write(&outbox_key(user_id), &outbox)
}

pub fn load_outbox(user_id: Uuid) -> Result<Vec<StoredRequest>, String> {
    Ok(read(&outbox_key(user_id))?.unwrap_or_default())
}

pub fn update_outbox(
    user_id: Uuid,
    change: impl FnOnce(&mut Vec<StoredRequest>),
) -> Result<(), String> {
    let mut outbox = load_outbox(user_id)?;
    change(&mut outbox);
    store_outbox(user_id, &outbox)
}

fn write<T: Serialize + ?Sized>(key: &str, value: &T) -> Result<(), String> {
    let json = serde_json::to_string(value).expect("stored values serialize");
    storage()?
        .set_item(key, &json)
        .map_err(|e| format!("Failed to save to localStorage: {:?}", e))
}

fn read<T: DeserializeOwned>(key: &str) -> Result<Option<T>, String> {
    let json = storage()?
        .get_item(key)
        .map_err(|e| format!("Failed to read from localStorage: {:?}", e))?;
    json.map(|json| serde_json::from_str(&json))
        .transpose()
        .map_err(|e| format!("Failed to parse {}: {}", key, e))
}

fn storage() -> Result<web_sys::Storage, String> {
    window()
        .ok_or_else(|| "No window object".to_string())?
        .local_storage()
        .map_err(|e| format!("Failed to access localStorage: {:?}", e))?
        .ok_or_else(|| "localStorage not available".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use dialect_coach_shared::{AIActionRequest, Dialect, Formality, TtsUsage, UsageStats};

    fn explain() -> StoredRequest {
        StoredRequest::new(ChatRequest::Action {
            session_id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            action: Box::new(AIActionRequest::ExplainMessage {
                message_content: "Hola".to_string(),
                dialect: Dialect::SpanishArgentinian,
                formality: Formality::Informal,
            }),
        })
    }

    fn ids_and_sent(outbox: &[StoredRequest]) -> Vec<(RequestId, bool)> {
        outbox.iter().map(|s| (s.id, s.sent)).collect()
    }

    #[test]
    fn test_outbox_add_mark_sent_remove() {
        let (first, second) = (explain(), explain());
        let mut outbox = Vec::new();
        add(&mut outbox, first.clone());
        add(&mut outbox, second.clone());
        mark_sent(&mut outbox, second.id);
        assert_eq!(
            ids_and_sent(&outbox),
            vec![(first.id, false), (second.id, true)]
        );
        remove(&mut outbox, first.id);
        assert_eq!(ids_and_sent(&outbox), vec![(second.id, true)]);
    }

    #[test]
    fn test_stored_request_envelope_keeps_its_id() {
        let stored = explain();
        let envelope = stored.envelope();
        assert_eq!(envelope.id, stored.id);
        assert!(matches!(envelope.body, ClientMessage::Chat(_)));
    }

    #[test]
    fn test_prefer_unsaved_keeps_changes_with_server_usage() {
        let user_id = Uuid::new_v4();
        let mut server = UserState::new(user_id);
        let usage = UsageStats {
            tts_events: vec![TtsUsage {
                timestamp: 0,
                characters: 42,
            }],
            ..UsageStats::default()
        };
        server.usage_stats = usage.clone();
        let mut unsaved = UserState::new(user_id);
        unsaved.tts_enabled = !server.tts_enabled;

        let preferred = prefer_unsaved(server.clone(), Some(unsaved.clone()));
        assert_eq!(preferred.tts_enabled, unsaved.tts_enabled);
        assert_eq!(preferred.usage_stats, usage);
        assert_eq!(prefer_unsaved(server.clone(), None), server);
    }

    #[test]
    fn test_keys_are_per_user() {
        let user_id = Uuid::new_v4();
        assert_eq!(
            unsaved_key(user_id),
            format!("dialect_coach_unsaved:{}", user_id)
        );
        assert_eq!(
            outbox_key(user_id),
            format!("dialect_coach_outbox:{}", user_id)
        );
    }
}
