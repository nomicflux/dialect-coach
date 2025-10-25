use dialect_coach_shared::UserState;
use log::info;
use web_sys::window;

const USER_STATE_KEY: &str = "dialect_coach_user_state";

pub fn save_user_state(state: &UserState) -> Result<(), String> {
    let storage = get_local_storage()?;
    let json = serde_json::to_string(state)
        .map_err(|e| format!("Failed to serialize UserState: {}", e))?;

    storage
        .set_item(USER_STATE_KEY, &json)
        .map_err(|e| format!("Failed to save to localStorage: {:?}", e))?;

    info!("UserState saved to localStorage");
    Ok(())
}

pub fn load_user_state() -> Result<Option<UserState>, String> {
    let storage = get_local_storage()?;

    match storage.get_item(USER_STATE_KEY) {
        Ok(Some(json)) => {
            let state = serde_json::from_str(&json)
                .map_err(|e| format!("Failed to deserialize UserState: {}", e))?;
            info!("UserState loaded from localStorage");
            Ok(Some(state))
        }
        Ok(None) => {
            info!("No UserState found in localStorage");
            Ok(None)
        }
        Err(e) => Err(format!("Failed to read from localStorage: {:?}", e)),
    }
}

fn get_local_storage() -> Result<web_sys::Storage, String> {
    window()
        .ok_or_else(|| "No window object".to_string())?
        .local_storage()
        .map_err(|e| format!("Failed to access localStorage: {:?}", e))?
        .ok_or_else(|| "localStorage not available".to_string())
}
