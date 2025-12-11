use serde::{Deserialize, Serialize};

/// Version identifier for User schema
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum UserVersion {
    #[default]
    V1,
}

/// Version identifier for UserState schema
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum UserStateVersion {
    #[default]
    V1,
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_version_default() {
        assert_eq!(UserVersion::default(), UserVersion::V1);
    }

    #[test]
    fn test_user_state_version_default() {
        assert_eq!(UserStateVersion::default(), UserStateVersion::V1);
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

        assert_eq!(versioned.version, UserVersion::V1);
        assert_eq!(versioned.data, data);
    }

    #[test]
    fn test_versioned_data_new_with_user_state_version() {
        let data = serde_json::json!({"user_id": "test-id", "learning_items": []});
        let versioned = VersionedData::<UserStateVersion>::new(data.clone());

        assert_eq!(versioned.version, UserStateVersion::V1);
        assert_eq!(versioned.data, data);
    }

    #[test]
    fn test_versioned_data_with_version() {
        let data = serde_json::json!({"id": "test"});
        let versioned =
            VersionedData::with_version(UserVersion::V1, data.clone());

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
        let deserialized: VersionedData<UserVersion> =
            serde_json::from_str(&json).unwrap();

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
        let deserialized: VersionedData<UserStateVersion> =
            serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.version, original.version);
        assert_eq!(deserialized.data, original.data);
    }

    #[test]
    fn test_versioned_data_clone() {
        let original = VersionedData::with_version(
            UserVersion::V1,
            serde_json::json!({"id": "test"}),
        );
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

        let versioned =
            VersionedData::with_version(UserStateVersion::V1, data.clone());
        let json = serde_json::to_string(&versioned).unwrap();
        let deserialized: VersionedData<UserStateVersion> =
            serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized, versioned);
    }
}
