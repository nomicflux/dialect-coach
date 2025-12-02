use serde::{Deserialize, Serialize};

/// Authentication credentials for user authentication
/// OAuth2-compatible design for future extensibility
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum AuthCredentials {
    InviteCode(String),
    Password(String),
}

impl AuthCredentials {
    pub fn invite_code(code: String) -> Self {
        Self::InviteCode(code)
    }

    pub fn password(pwd: String) -> Self {
        Self::Password(pwd)
    }

    pub fn as_invite_code(&self) -> Option<&str> {
        match self {
            Self::InviteCode(code) => Some(code),
            Self::Password(_) => None,
        }
    }

    pub fn as_password(&self) -> Option<&str> {
        match self {
            Self::Password(pwd) => Some(pwd),
            Self::InviteCode(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invite_code_construction() {
        let code = "ABC123DEF456".to_string();
        let creds = AuthCredentials::invite_code(code.clone());
        assert_eq!(creds, AuthCredentials::InviteCode(code));
    }

    #[test]
    fn test_as_invite_code_some() {
        let code = "TEST-CODE-1".to_string();
        let creds = AuthCredentials::invite_code(code.clone());
        assert_eq!(creds.as_invite_code(), Some("TEST-CODE-1"));
    }

    #[test]
    fn test_as_invite_code_extracts_string() {
        let creds = AuthCredentials::invite_code("secret123".to_string());
        let extracted = creds.as_invite_code();
        assert!(extracted.is_some());
        assert_eq!(extracted.unwrap(), "secret123");
    }

    #[test]
    fn test_serialization() {
        let creds = AuthCredentials::invite_code("code123".to_string());
        let json = serde_json::to_string(&creds).unwrap();
        assert!(json.contains("InviteCode"));
        assert!(json.contains("code123"));
    }

    #[test]
    fn test_deserialization() {
        let creds = AuthCredentials::invite_code("test-code".to_string());
        let json = serde_json::to_string(&creds).unwrap();
        let deserialized: AuthCredentials = serde_json::from_str(&json).unwrap();
        assert_eq!(creds, deserialized);
    }

    #[test]
    fn test_clone() {
        let creds = AuthCredentials::invite_code("cloneable".to_string());
        let cloned = creds.clone();
        assert_eq!(creds, cloned);
    }

    #[test]
    fn test_password_construction() {
        let pwd = "secret123".to_string();
        let creds = AuthCredentials::password(pwd.clone());
        assert_eq!(creds, AuthCredentials::Password(pwd));
    }

    #[test]
    fn test_as_password_some() {
        let pwd = "mypassword".to_string();
        let creds = AuthCredentials::password(pwd.clone());
        assert_eq!(creds.as_password(), Some("mypassword"));
    }

    #[test]
    fn test_as_password_none_for_invite_code() {
        let creds = AuthCredentials::invite_code("CODE123".to_string());
        assert_eq!(creds.as_password(), None);
    }

    #[test]
    fn test_as_invite_code_none_for_password() {
        let creds = AuthCredentials::password("pwd123".to_string());
        assert_eq!(creds.as_invite_code(), None);
    }

    #[test]
    fn test_password_serialization() {
        let creds = AuthCredentials::password("test123".to_string());
        let json = serde_json::to_string(&creds).unwrap();
        assert!(json.contains("Password"));
        assert!(json.contains("test123"));
    }

    #[test]
    fn test_password_deserialization() {
        let creds = AuthCredentials::password("mypassword".to_string());
        let json = serde_json::to_string(&creds).unwrap();
        let deserialized: AuthCredentials = serde_json::from_str(&json).unwrap();
        assert_eq!(creds, deserialized);
    }

    #[test]
    fn test_password_clone() {
        let creds = AuthCredentials::password("cloneable_pwd".to_string());
        let cloned = creds.clone();
        assert_eq!(creds, cloned);
    }
}
