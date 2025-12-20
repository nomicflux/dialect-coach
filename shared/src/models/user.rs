use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// User account with UUID, username, email, and admin flag
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub is_admin: bool,
}

impl User {
    pub fn new(id: Uuid, username: String, email: String) -> Self {
        Self::new_with_admin(id, username, email, false)
    }

    pub fn new_with_admin(id: Uuid, username: String, email: String, is_admin: bool) -> Self {
        Self {
            id,
            username,
            email,
            is_admin,
        }
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
        assert!(!user.is_admin);
    }

    #[test]
    fn test_user_new_with_admin() {
        let id = Uuid::new_v4();
        let user = User::new_with_admin(id, "admin".to_string(), "admin@example.com".to_string(), true);

        assert_eq!(user.id, id);
        assert!(user.is_admin);
    }

    #[test]
    fn test_user_serialization() {
        let user = User::new(
            Uuid::new_v4(),
            "testuser".to_string(),
            "test@example.com".to_string(),
        );
        let json = serde_json::to_string(&user).unwrap();

        assert!(json.contains("\"id\""));
        assert!(json.contains("\"username\""));
        assert!(json.contains("\"email\""));
        assert!(json.contains("\"is_admin\""));
        assert!(json.contains("testuser"));
        assert!(json.contains("test@example.com"));
    }

    #[test]
    fn test_user_deserialization() {
        let user = User::new(
            Uuid::new_v4(),
            "testuser".to_string(),
            "test@example.com".to_string(),
        );
        let json = serde_json::to_string(&user).unwrap();
        let deserialized: User = serde_json::from_str(&json).unwrap();

        assert_eq!(user.id, deserialized.id);
        assert_eq!(user.username, deserialized.username);
        assert_eq!(user.email, deserialized.email);
        assert_eq!(user.is_admin, deserialized.is_admin);
    }

    #[test]
    fn test_user_unicode_username() {
        let arabic_name = "أحمد".to_string();
        let user = User::new(
            Uuid::new_v4(),
            arabic_name.clone(),
            "test@example.com".to_string(),
        );

        assert_eq!(user.username, arabic_name);

        let json = serde_json::to_string(&user).unwrap();
        let deserialized: User = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.username, arabic_name);
    }

    #[test]
    fn test_user_clone() {
        let user = User::new(
            Uuid::new_v4(),
            "testuser".to_string(),
            "test@example.com".to_string(),
        );
        let cloned = user.clone();

        assert_eq!(user.id, cloned.id);
        assert_eq!(user.username, cloned.username);
        assert_eq!(user.email, cloned.email);
        assert_eq!(user.is_admin, cloned.is_admin);
    }
}
