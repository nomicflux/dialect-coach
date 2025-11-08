use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Invite code for user registration with optional expiration
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct InviteCode {
    pub code: String,
    pub created_date: i64,
    pub used_by: Option<Uuid>,
    pub expiration: Option<i64>,
}

impl InviteCode {
    pub fn new(code: String, expiration: Option<i64>) -> Self {
        let created_date = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        Self {
            code,
            created_date,
            used_by: None,
            expiration,
        }
    }

    pub fn is_expired(&self, current_time: i64) -> bool {
        if let Some(exp) = self.expiration {
            current_time > exp
        } else {
            false
        }
    }

    pub fn is_used(&self) -> bool {
        self.used_by.is_some()
    }

    pub fn mark_used(&mut self, user_id: Uuid) {
        self.used_by = Some(user_id);
    }

    pub fn is_valid(&self, current_time: i64) -> bool {
        !self.is_expired(current_time) && !self.is_used()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invite_code_creation() {
        let code = "ABC123DEF456".to_string();
        let invite = InviteCode::new(code.clone(), None);

        assert_eq!(invite.code, code);
        assert_eq!(invite.used_by, None);
        assert_eq!(invite.expiration, None);
    }

    #[test]
    fn test_invite_code_with_expiration() {
        let code = "CODE123".to_string();
        let expiration = 9999999999i64;
        let invite = InviteCode::new(code.clone(), Some(expiration));

        assert_eq!(invite.code, code);
        assert_eq!(invite.expiration, Some(expiration));
        assert_eq!(invite.used_by, None);
    }

    #[test]
    fn test_is_expired_with_future_expiration() {
        let invite = InviteCode::new("CODE".to_string(), Some(9999999999i64));
        let current_time = 1000000000i64;

        assert!(!invite.is_expired(current_time));
    }

    #[test]
    fn test_is_expired_with_past_expiration() {
        let invite = InviteCode::new("CODE".to_string(), Some(1000000000i64));
        let current_time = 9999999999i64;

        assert!(invite.is_expired(current_time));
    }

    #[test]
    fn test_is_expired_no_expiration() {
        let invite = InviteCode::new("CODE".to_string(), None);
        let current_time = 9999999999i64;

        assert!(!invite.is_expired(current_time));
    }

    #[test]
    fn test_is_used_when_not_used() {
        let invite = InviteCode::new("CODE".to_string(), None);

        assert!(!invite.is_used());
    }

    #[test]
    fn test_is_used_when_used() {
        let mut invite = InviteCode::new("CODE".to_string(), None);
        let user_id = Uuid::new_v4();
        invite.mark_used(user_id);

        assert!(invite.is_used());
        assert_eq!(invite.used_by, Some(user_id));
    }

    #[test]
    fn test_mark_used() {
        let mut invite = InviteCode::new("CODE".to_string(), None);
        let user_id = Uuid::new_v4();

        invite.mark_used(user_id);

        assert_eq!(invite.used_by, Some(user_id));
    }

    #[test]
    fn test_is_valid_when_not_expired_and_not_used() {
        let invite = InviteCode::new("CODE".to_string(), Some(9999999999i64));
        let current_time = 1000000000i64;

        assert!(invite.is_valid(current_time));
    }

    #[test]
    fn test_is_valid_when_expired() {
        let invite = InviteCode::new("CODE".to_string(), Some(1000000000i64));
        let current_time = 9999999999i64;

        assert!(!invite.is_valid(current_time));
    }

    #[test]
    fn test_is_valid_when_used() {
        let mut invite = InviteCode::new("CODE".to_string(), Some(9999999999i64));
        let user_id = Uuid::new_v4();
        invite.mark_used(user_id);
        let current_time = 1000000000i64;

        assert!(!invite.is_valid(current_time));
    }

    #[test]
    fn test_is_valid_when_both_expired_and_used() {
        let mut invite = InviteCode::new("CODE".to_string(), Some(1000000000i64));
        let user_id = Uuid::new_v4();
        invite.mark_used(user_id);
        let current_time = 9999999999i64;

        assert!(!invite.is_valid(current_time));
    }

    #[test]
    fn test_serialization() {
        let user_id = Uuid::new_v4();
        let mut invite = InviteCode::new("CODE123".to_string(), Some(9999999999i64));
        invite.mark_used(user_id);

        let json = serde_json::to_string(&invite).unwrap();
        let deserialized: InviteCode = serde_json::from_str(&json).unwrap();

        assert_eq!(invite, deserialized);
    }

    #[test]
    fn test_deserialization() {
        let json = r#"{
            "code": "ABC123",
            "created_date": 1700000000,
            "used_by": null,
            "expiration": 9999999999
        }"#;

        let invite: InviteCode = serde_json::from_str(json).unwrap();

        assert_eq!(invite.code, "ABC123");
        assert_eq!(invite.created_date, 1700000000);
        assert_eq!(invite.used_by, None);
        assert_eq!(invite.expiration, Some(9999999999));
    }

    #[test]
    fn test_clone() {
        let invite = InviteCode::new("CODE".to_string(), Some(9999999999i64));
        let cloned = invite.clone();

        assert_eq!(invite, cloned);
    }
}
