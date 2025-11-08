use serde::{Deserialize, Serialize};

/// Authentication credentials for user authentication
/// OAuth2-compatible design for future extensibility
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum AuthCredentials {
    InviteCode(String),
}

impl AuthCredentials {
    pub fn invite_code(code: String) -> Self {
        Self::InviteCode(code)
    }

    pub fn as_invite_code(&self) -> Option<&str> {
        match self {
            Self::InviteCode(code) => Some(code),
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
}
