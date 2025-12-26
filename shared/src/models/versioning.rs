use serde::{Deserialize, Serialize};

use super::{User, UserState};

/// Version identifier for User schema
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum UserVersion {
    V1,
    #[default]
    V2Admin,
}

/// Version identifier for UserState schema
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum UserStateVersion {
    V1,
    #[default]
    V2ScoredItems,
}

pub type UserStateV1 = UserState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserV1Data {
    pub id: uuid::Uuid,
    pub username: String,
    pub email: String,
}

pub const CURRENT_USER_VERSION: UserVersion = UserVersion::V2Admin;
pub const CURRENT_USER_STATE_VERSION: UserStateVersion = UserStateVersion::V2ScoredItems;

/// Generic wrapper for versioned data stored in sled
/// Stores the version enum and raw JSON data separately.
/// On load, the version determines which type to deserialize the data into.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VersionedData<V> {
    pub version: V,
    pub data: serde_json::Value,
}

impl<V: Default> VersionedData<V> {
    /// Create a new VersionedData wrapper with the default version
    pub fn new(data: serde_json::Value) -> Self {
        Self {
            version: V::default(),
            data,
        }
    }

    /// Create a VersionedData wrapper with an explicit version
    pub fn with_version(version: V, data: serde_json::Value) -> Self {
        Self { version, data }
    }
}

/// Migration function type for UserState JSON migrations
pub type UserStateMigrationFn = fn(serde_json::Value) -> serde_json::Value;

/// A single atomic migration step
pub struct UserStateMigrationStep {
    pub from: UserStateVersion,
    pub to: UserStateVersion,
    pub forward: UserStateMigrationFn,
    pub backward: UserStateMigrationFn,
}

/// Registry of all UserState migrations - each entry is an atomic step
pub const USER_STATE_MIGRATIONS: &[UserStateMigrationStep] = &[
    UserStateMigrationStep {
        from: UserStateVersion::V1,
        to: UserStateVersion::V2ScoredItems,
        forward: v1_to_v2_forward,
        backward: v1_to_v2_backward,
    },
    // Future migrations go here as new entries
];

// ============ V1 → V2 Migration Functions ============

fn v1_to_v2_forward(mut data: serde_json::Value) -> serde_json::Value {
    tracing::debug!("V1→V2 migration: starting forward migration");
    wrap_learning_items_in_conversation_history(&mut data);
    data
}

fn v1_to_v2_backward(mut data: serde_json::Value) -> serde_json::Value {
    tracing::debug!("V1→V2 migration: starting backward migration");
    unwrap_learning_items_in_conversation_history(&mut data);
    data
}

fn wrap_learning_items_in_conversation_history(data: &mut serde_json::Value) {
    let history = match data.get_mut("conversation_history").and_then(|v| v.as_array_mut()) {
        Some(h) => h,
        None => {
            tracing::debug!("V1→V2 migration: no conversation_history found");
            return;
        }
    };

    tracing::debug!("V1→V2 migration: processing {} messages", history.len());
    let mut wrapped_count = 0;

    for msg in history {
        // Path: msg["content"]["AgentMessage"]["content"] = AgentResponse
        if let Some(agent_response) = msg
            .get_mut("content")
            .and_then(|c| c.get_mut("AgentMessage"))
            .and_then(|am| am.get_mut("content"))
        {
            wrap_items_with_scores(agent_response);
            wrapped_count += 1;
        }
    }

    tracing::debug!("V1→V2 migration: wrapped items in {} agent messages", wrapped_count);
}

fn unwrap_learning_items_in_conversation_history(data: &mut serde_json::Value) {
    let history = match data.get_mut("conversation_history").and_then(|v| v.as_array_mut()) {
        Some(h) => h,
        None => {
            tracing::debug!("V2→V1 migration: no conversation_history found");
            return;
        }
    };

    tracing::debug!("V2→V1 migration: processing {} messages", history.len());

    for msg in history {
        // Path: msg["content"]["AgentMessage"]["content"] = AgentResponse
        if let Some(agent_response) = msg
            .get_mut("content")
            .and_then(|c| c.get_mut("AgentMessage"))
            .and_then(|am| am.get_mut("content"))
        {
            unwrap_items_from_scores(agent_response);
        }
    }
}

fn wrap_items_with_scores(response: &mut serde_json::Value) {
    for field in ["mistakes", "explained", "translated", "exploratory"] {
        if let Some(items) = response.get_mut(field).and_then(|v| v.as_array_mut()) {
            if items.is_empty() {
                continue;
            }
            tracing::debug!("V1→V2 migration: wrapping {} {} items", items.len(), field);
            let wrapped: Vec<serde_json::Value> = items
                .drain(..)
                .map(|item| serde_json::json!([item, 0]))
                .collect();
            *items = wrapped;
        }
    }
}

fn unwrap_items_from_scores(response: &mut serde_json::Value) {
    for field in ["mistakes", "explained", "translated", "exploratory"] {
        if let Some(items) = response.get_mut(field).and_then(|v| v.as_array_mut()) {
            let unwrapped: Vec<serde_json::Value> = items
                .drain(..)
                .filter_map(|tuple| tuple.as_array().and_then(|arr| arr.first().cloned()))
                .collect();
            *items = unwrapped;
        }
    }
}

// ============ Migration Runner ============

/// Run all migrations from given version to current, using the registry
pub fn run_user_state_migrations(
    mut version: UserStateVersion,
    mut data: serde_json::Value,
) -> serde_json::Value {
    while let Some(step) = USER_STATE_MIGRATIONS.iter().find(|m| m.from == version) {
        data = (step.forward)(data);
        version = step.to;
    }
    data
}

pub fn migrate_user_state_to_current(
    from_version: UserStateVersion,
    data: serde_json::Value,
) -> Result<UserState, serde_json::Error> {
    let migrated = run_user_state_migrations(from_version, data);
    serde_json::from_value(migrated)
}

pub fn migrate_user_to_current(from_version: UserVersion, data: serde_json::Value) -> User {
    match from_version {
        UserVersion::V1 => {
            let v1: UserV1Data = serde_json::from_value(data).expect("Valid V1 user data");
            User::new_with_admin(v1.id, v1.username, v1.email, false)
        }
        UserVersion::V2Admin => serde_json::from_value(data).expect("Valid V2 user data"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_version_default() {
        assert_eq!(UserVersion::default(), UserVersion::V2Admin);
    }

    #[test]
    fn test_user_state_version_default() {
        assert_eq!(UserStateVersion::default(), UserStateVersion::V2ScoredItems);
    }

    #[test]
    fn test_user_version_serialization() {
        let version = UserVersion::V1;
        let json = serde_json::to_string(&version).unwrap();
        let deserialized: UserVersion = serde_json::from_str(&json).unwrap();
        assert_eq!(version, deserialized);
    }

    #[test]
    fn test_user_state_version_serialization() {
        let version = UserStateVersion::V1;
        let json = serde_json::to_string(&version).unwrap();
        let deserialized: UserStateVersion = serde_json::from_str(&json).unwrap();
        assert_eq!(version, deserialized);
    }

    #[test]
    fn test_versioned_data_new_with_user_version() {
        let data = serde_json::json!({"id": "test-id", "name": "test"});
        let versioned = VersionedData::<UserVersion>::new(data.clone());

        assert_eq!(versioned.version, UserVersion::V2Admin);
        assert_eq!(versioned.data, data);
    }

    #[test]
    fn test_versioned_data_new_with_user_state_version() {
        let data = serde_json::json!({"user_id": "test-id", "learning_items": []});
        let versioned = VersionedData::<UserStateVersion>::new(data.clone());

        assert_eq!(versioned.version, UserStateVersion::V2ScoredItems);
        assert_eq!(versioned.data, data);
    }

    #[test]
    fn test_versioned_data_with_version() {
        let data = serde_json::json!({"id": "test"});
        let versioned = VersionedData::with_version(UserVersion::V1, data.clone());

        assert_eq!(versioned.version, UserVersion::V1);
        assert_eq!(versioned.data, data);
    }

    #[test]
    fn test_versioned_data_serialization_user_version() {
        let data = serde_json::json!({"id": "test", "value": 42});
        let versioned = VersionedData::with_version(UserVersion::V1, data);
        let json = serde_json::to_string(&versioned).unwrap();

        assert!(json.contains("\"version\""));
        assert!(json.contains("\"V1\""));
        assert!(json.contains("\"data\""));
        assert!(json.contains("\"id\""));
        assert!(json.contains("\"value\""));
    }

    #[test]
    fn test_versioned_data_deserialization_user_version() {
        let original = VersionedData::with_version(
            UserVersion::V1,
            serde_json::json!({"id": "test", "value": 42}),
        );
        let json = serde_json::to_string(&original).unwrap();
        let deserialized: VersionedData<UserVersion> = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.version, original.version);
        assert_eq!(deserialized.data, original.data);
    }

    #[test]
    fn test_versioned_data_serialization_user_state_version() {
        let data = serde_json::json!({"user_id": "123", "items": []});
        let versioned = VersionedData::with_version(UserStateVersion::V1, data);
        let json = serde_json::to_string(&versioned).unwrap();

        assert!(json.contains("\"version\""));
        assert!(json.contains("\"V1\""));
        assert!(json.contains("\"data\""));
    }

    #[test]
    fn test_versioned_data_deserialization_user_state_version() {
        let original = VersionedData::with_version(
            UserStateVersion::V1,
            serde_json::json!({"user_id": "123"}),
        );
        let json = serde_json::to_string(&original).unwrap();
        let deserialized: VersionedData<UserStateVersion> = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.version, original.version);
        assert_eq!(deserialized.data, original.data);
    }

    #[test]
    fn test_versioned_data_clone() {
        let original =
            VersionedData::with_version(UserVersion::V1, serde_json::json!({"id": "test"}));
        let cloned = original.clone();

        assert_eq!(cloned.version, original.version);
        assert_eq!(cloned.data, original.data);
    }

    #[test]
    fn test_versioned_data_equality() {
        let data = serde_json::json!({"id": "test"});
        let v1 = VersionedData::with_version(UserVersion::V1, data.clone());
        let v2 = VersionedData::with_version(UserVersion::V1, data);

        assert_eq!(v1, v2);
    }

    #[test]
    fn test_versioned_data_inequality_different_versions() {
        let data1 = serde_json::json!({"id": "test1"});
        let data2 = serde_json::json!({"id": "test2"});

        let v1 = VersionedData::with_version(UserVersion::V1, data1);
        let v2 = VersionedData::with_version(UserVersion::V1, data2);

        assert_ne!(v1, v2);
    }

    #[test]
    fn test_versioned_data_complex_json() {
        let data = serde_json::json!({
            "id": "user-123",
            "name": "John Doe",
            "items": [
                {"id": 1, "name": "item1"},
                {"id": 2, "name": "item2"}
            ],
            "metadata": {
                "created": "2024-01-01",
                "updated": "2024-01-02"
            }
        });

        let versioned = VersionedData::with_version(UserStateVersion::V1, data.clone());
        let json = serde_json::to_string(&versioned).unwrap();
        let deserialized: VersionedData<UserStateVersion> = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized, versioned);
    }

    // ============ Registry-Based Migration Tests ============

    #[test]
    fn test_v1_to_v2_forward_wraps_items() {
        // JSON structure matches Message serialization: content.AgentMessage.content
        let v1_data = serde_json::json!({
            "conversation_history": [{
                "id": "msg-1",
                "parent_id": null,
                "metadata": {},
                "content": {
                    "AgentMessage": {
                        "content": {
                            "response": "test",
                            "mistakes": [{"id": "1", "specific_mistake": "err"}],
                            "explained": [{"id": "2", "new_phrase": "phrase"}]
                        }
                    }
                }
            }]
        });

        let v2_data = super::v1_to_v2_forward(v1_data);

        let history = v2_data["conversation_history"].as_array().unwrap();
        let agent_response = &history[0]["content"]["AgentMessage"]["content"];
        let mistakes = agent_response["mistakes"].as_array().unwrap();
        let explained = agent_response["explained"].as_array().unwrap();

        // Should be wrapped as [item, 0]
        assert!(mistakes[0].as_array().is_some());
        assert_eq!(mistakes[0][1], 0);
        assert!(explained[0].as_array().is_some());
        assert_eq!(explained[0][1], 0);
    }

    #[test]
    fn test_v1_to_v2_backward_unwraps_items() {
        let v2_data = serde_json::json!({
            "conversation_history": [{
                "id": "msg-1",
                "parent_id": null,
                "metadata": {},
                "content": {
                    "AgentMessage": {
                        "content": {
                            "response": "test",
                            "mistakes": [[{"id": "1", "specific_mistake": "err"}, 5]]
                        }
                    }
                }
            }]
        });

        let v1_data = super::v1_to_v2_backward(v2_data);

        let history = v1_data["conversation_history"].as_array().unwrap();
        let agent_response = &history[0]["content"]["AgentMessage"]["content"];
        let mistakes = agent_response["mistakes"].as_array().unwrap();

        // Should be unwrapped to just item
        assert!(mistakes[0].as_object().is_some());
        assert_eq!(mistakes[0]["id"], "1");
    }

    #[test]
    fn test_run_migrations_v1_to_current() {
        let v1_data = serde_json::json!({
            "user_id": "test",
            "conversation_history": [{
                "id": "msg-1",
                "content": {
                    "AgentMessage": {
                        "content": {
                            "mistakes": [{"id": "1"}]
                        }
                    }
                }
            }]
        });

        let result = super::run_user_state_migrations(super::UserStateVersion::V1, v1_data);

        let history = result["conversation_history"].as_array().unwrap();
        let agent_response = &history[0]["content"]["AgentMessage"]["content"];
        let mistakes = agent_response["mistakes"].as_array().unwrap();
        assert!(mistakes[0].as_array().is_some()); // Wrapped as tuple
    }

    #[test]
    fn test_run_migrations_current_version_no_change() {
        let v2_data = serde_json::json!({
            "user_id": "test",
            "conversation_history": []
        });

        let result = super::run_user_state_migrations(
            super::UserStateVersion::V2ScoredItems,
            v2_data.clone(),
        );

        assert_eq!(result, v2_data); // No change for current version
    }

    fn extract_field_names(value: &serde_json::Value) -> Vec<String> {
        if let Some(obj) = value.as_object() {
            let mut fields: Vec<String> = obj.keys().cloned().collect();
            fields.sort();
            fields
        } else {
            Vec::new()
        }
    }

    #[test]
    fn test_user_structure_snapshot() {
        let user = User::new(
            uuid::Uuid::new_v4(),
            "testuser".to_string(),
            "test@example.com".to_string(),
        );

        let json = serde_json::to_value(&user).unwrap();
        let actual_fields = extract_field_names(&json);
        let expected_fields = vec!["email", "id", "is_admin", "username"];

        assert!(
            actual_fields == expected_fields,
            "STRUCTURE CHANGE DETECTED in User!\n\nExpected fields: {:?}\nActual fields:   {:?}\n\nTO FIX THIS TEST:\n1. Add new version variant to UserVersion enum (e.g., V2)\n2. Create UserV2 type alias in shared/src/models/versioning.rs\n3. Create migration struct and implement Migration<UserV1, UserV2>:\n   impl Migration<UserV1, UserV2> for UserV1ToV2 {{\n       fn migrate_forward(from: UserV1) -> UserV2 {{ ... }}\n       fn migrate_backward(to: UserV2) -> UserV1 {{ ... }}\n   }}\n4. Update CURRENT_USER_VERSION constant to V2\n5. Update this test's expected fields to match new structure\n\nSee shared/src/models/versioning.rs for migration examples.",
            expected_fields,
            actual_fields
        );
    }

    #[test]
    fn test_user_state_structure_snapshot() {
        let state = UserState::new(uuid::Uuid::new_v4());

        let json = serde_json::to_value(&state).unwrap();
        let actual_fields = extract_field_names(&json);
        let expected_fields = vec![
            "active_branch_id",
            "active_plan_id",
            "branches",
            "conversation_history",
            "dialect_levels",
            "formality",
            "language_options",
            "language_plans",
            "learning_goals",
            "learning_items",
            "selected_dialect",
            "selected_language",
            "show_experimental_dialects",
            "teaching_mode",
            "tts_enabled",
            "usage_stats",
            "user_gender",
            "user_id",
        ];

        assert!(
            actual_fields == expected_fields,
            "STRUCTURE CHANGE DETECTED in UserState!\n\nExpected fields: {:?}\nActual fields:   {:?}\n\nTO FIX THIS TEST:\n1. Add new version variant to UserStateVersion enum (e.g., V2)\n2. Create UserStateV2 type alias in shared/src/models/versioning.rs\n3. Create migration struct and implement Migration<UserStateV1, UserStateV2>:\n   impl Migration<UserStateV1, UserStateV2> for UserStateV1ToV2 {{\n       fn migrate_forward(from: UserStateV1) -> UserStateV2 {{ ... }}\n       fn migrate_backward(to: UserStateV2) -> UserStateV1 {{ ... }}\n   }}\n4. Update CURRENT_USER_STATE_VERSION constant to V2\n5. Update this test's expected fields to match new structure\n\nSee shared/src/models/versioning.rs for migration examples.",
            expected_fields,
            actual_fields
        );
    }

    #[test]
    fn test_migrate_user_v1_to_v2_admin() {
        let v1_data = serde_json::json!({
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "username": "testuser",
            "email": "test@example.com"
        });
        let user = migrate_user_to_current(UserVersion::V1, v1_data);
        assert_eq!(user.username, "testuser");
        assert!(!user.is_admin);
    }

    #[test]
    fn test_migrate_user_v2_admin_passthrough() {
        let v2_data = serde_json::json!({
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "username": "adminuser",
            "email": "admin@example.com",
            "is_admin": true
        });
        let user = migrate_user_to_current(UserVersion::V2Admin, v2_data);
        assert_eq!(user.username, "adminuser");
        assert!(user.is_admin);
    }
}
