use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// User account with UUID and username
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct User {
    pub id: Uuid,
    pub username: String,
}

impl User {
    pub fn new(id: Uuid, username: String) -> Self {
        Self { id, username }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_new() {
        let id = Uuid::new_v4();
        let username = "testuser".to_string();
        let user = User::new(id, username.clone());

        assert_eq!(user.id, id);
        assert_eq!(user.username, username);
    }

    #[test]
    fn test_user_serialization() {
        let user = User::new(Uuid::new_v4(), "testuser".to_string());
        let json = serde_json::to_string(&user).unwrap();

        assert!(json.contains("\"id\""));
        assert!(json.contains("\"username\""));
        assert!(json.contains("testuser"));
    }

    #[test]
    fn test_user_deserialization() {
        let user = User::new(Uuid::new_v4(), "testuser".to_string());
        let json = serde_json::to_string(&user).unwrap();
        let deserialized: User = serde_json::from_str(&json).unwrap();

        assert_eq!(user.id, deserialized.id);
        assert_eq!(user.username, deserialized.username);
    }

    #[test]
    fn test_user_unicode_username() {
        let arabic_name = "أحمد".to_string();
        let user = User::new(Uuid::new_v4(), arabic_name.clone());

        assert_eq!(user.username, arabic_name);

        let json = serde_json::to_string(&user).unwrap();
        let deserialized: User = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.username, arabic_name);
    }

    #[test]
    fn test_user_clone() {
        let user = User::new(Uuid::new_v4(), "testuser".to_string());
        let cloned = user.clone();

        assert_eq!(user.id, cloned.id);
        assert_eq!(user.username, cloned.username);
    }
}
