use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// User account with UUID, username, and email
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
}

impl User {
    pub fn new(id: Uuid, username: String, email: String) -> Self {
        Self { id, username, email }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_new() {
        let id = Uuid::new_v4();
        let username = "testuser".to_string();
        let email = "test@example.com".to_string();
        let user = User::new(id, username.clone(), email.clone());

        assert_eq!(user.id, id);
        assert_eq!(user.username, username);
        assert_eq!(user.email, email);
    }

    #[test]
    fn test_user_serialization() {
        let user = User::new(Uuid::new_v4(), "testuser".to_string(), "test@example.com".to_string());
        let json = serde_json::to_string(&user).unwrap();

        assert!(json.contains("\"id\""));
        assert!(json.contains("\"username\""));
        assert!(json.contains("\"email\""));
        assert!(json.contains("testuser"));
        assert!(json.contains("test@example.com"));
    }

    #[test]
    fn test_user_deserialization() {
        let user = User::new(Uuid::new_v4(), "testuser".to_string(), "test@example.com".to_string());
        let json = serde_json::to_string(&user).unwrap();
        let deserialized: User = serde_json::from_str(&json).unwrap();

        assert_eq!(user.id, deserialized.id);
        assert_eq!(user.username, deserialized.username);
        assert_eq!(user.email, deserialized.email);
    }

    #[test]
    fn test_user_unicode_username() {
        let arabic_name = "أحمد".to_string();
        let user = User::new(Uuid::new_v4(), arabic_name.clone(), "test@example.com".to_string());

        assert_eq!(user.username, arabic_name);

        let json = serde_json::to_string(&user).unwrap();
        let deserialized: User = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.username, arabic_name);
    }

    #[test]
    fn test_user_clone() {
        let user = User::new(Uuid::new_v4(), "testuser".to_string(), "test@example.com".to_string());
        let cloned = user.clone();

        assert_eq!(user.id, cloned.id);
        assert_eq!(user.username, cloned.username);
        assert_eq!(user.email, cloned.email);
    }
}
