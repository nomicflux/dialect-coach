use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
};
use dialect_coach_shared::{
    CreateInviteRequest, InviteCode, InviteListItem, InviteListResponse, InviteResponse,
};

use crate::AppState;

fn generate_invite_code() -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let mut rng = rand::thread_rng();
    (0..12)
        .map(|_| CHARSET[rng.gen_range(0..CHARSET.len())] as char)
        .collect()
}

fn current_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

fn calculate_invite_status(invite: &InviteCode, current_time: i64) -> String {
    if invite.is_used() {
        "used".to_string()
    } else if invite.is_expired(current_time) {
        "expired".to_string()
    } else {
        "active".to_string()
    }
}

fn verify_admin_token(headers: &HeaderMap, admin_token: &Option<String>) -> Result<(), StatusCode> {
    let expected_token = admin_token.as_ref().ok_or(StatusCode::UNAUTHORIZED)?;

    let auth_header = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if token == expected_token {
        Ok(())
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

pub async fn create_invite(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateInviteRequest>,
) -> Result<(StatusCode, Json<InviteResponse>), StatusCode> {
    verify_admin_token(&headers, &state.admin_token)?;

    let code = generate_invite_code();
    let created_at = current_timestamp();
    let expires_at = req
        .expires_days
        .map(|days| created_at + (days as i64 * 86400));

    let invite = InviteCode::new(code.clone(), expires_at);

    state
        .user_persistence
        .create_invite_code(&invite)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok((
        StatusCode::CREATED,
        Json(InviteResponse {
            code,
            created_at,
            expires_at,
        }),
    ))
}

pub async fn list_invites(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<InviteListResponse>, StatusCode> {
    verify_admin_token(&headers, &state.admin_token)?;

    let codes = state
        .user_persistence
        .list_invite_codes()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let now = current_timestamp();
    let invites = codes
        .into_iter()
        .map(|invite| {
            let status = calculate_invite_status(&invite, now);
            InviteListItem {
                code: invite.code,
                created_at: invite.created_date,
                expires_at: invite.expiration,
                used_at: invite.used_by.map(|_| invite.created_date),
                status,
            }
        })
        .collect();

    Ok(Json(InviteListResponse { invites }))
}

pub async fn delete_invite(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(code): Path<String>,
) -> Result<StatusCode, StatusCode> {
    verify_admin_token(&headers, &state.admin_token)?;

    let exists = state
        .user_persistence
        .load_invite_code(&code)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if exists.is_none() {
        return Err(StatusCode::NOT_FOUND);
    }

    state
        .user_persistence
        .delete_invite_code(&code)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_invite_code() {
        let code = generate_invite_code();
        assert_eq!(code.len(), 12);
        assert!(
            code.chars()
                .all(|c| "ABCDEFGHJKLMNPQRSTUVWXYZ23456789".contains(c))
        );
    }

    #[test]
    fn test_current_timestamp() {
        let ts = current_timestamp();
        assert!(ts > 0);
    }

    #[test]
    fn test_calculate_invite_status_active() {
        let invite = InviteCode::new("CODE".to_string(), None);
        let status = calculate_invite_status(&invite, current_timestamp());
        assert_eq!(status, "active");
    }

    #[test]
    fn test_calculate_invite_status_expired() {
        let invite = InviteCode::new("CODE".to_string(), Some(1000));
        let status = calculate_invite_status(&invite, 9999999999);
        assert_eq!(status, "expired");
    }

    #[test]
    fn test_calculate_invite_status_used() {
        let mut invite = InviteCode::new("CODE".to_string(), None);
        invite.mark_used(uuid::Uuid::new_v4());
        let status = calculate_invite_status(&invite, current_timestamp());
        assert_eq!(status, "used");
    }

    #[test]
    fn test_verify_admin_token_valid() {
        let mut headers = HeaderMap::new();
        headers.insert("Authorization", "Bearer secret123".parse().unwrap());
        let admin_token = Some("secret123".to_string());
        let result = verify_admin_token(&headers, &admin_token);
        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_admin_token_invalid() {
        let mut headers = HeaderMap::new();
        headers.insert("Authorization", "Bearer wrongtoken".parse().unwrap());
        let admin_token = Some("secret123".to_string());
        let result = verify_admin_token(&headers, &admin_token);
        assert_eq!(result, Err(StatusCode::UNAUTHORIZED));
    }

    #[test]
    fn test_verify_admin_token_missing_bearer() {
        let mut headers = HeaderMap::new();
        headers.insert("Authorization", "secret123".parse().unwrap());
        let admin_token = Some("secret123".to_string());
        let result = verify_admin_token(&headers, &admin_token);
        assert_eq!(result, Err(StatusCode::UNAUTHORIZED));
    }

    #[test]
    fn test_verify_admin_token_missing_header() {
        let headers = HeaderMap::new();
        let admin_token = Some("secret123".to_string());
        let result = verify_admin_token(&headers, &admin_token);
        assert_eq!(result, Err(StatusCode::UNAUTHORIZED));
    }

    #[test]
    fn test_verify_admin_token_not_configured() {
        let mut headers = HeaderMap::new();
        headers.insert("Authorization", "Bearer secret123".parse().unwrap());
        let admin_token = None;
        let result = verify_admin_token(&headers, &admin_token);
        assert_eq!(result, Err(StatusCode::UNAUTHORIZED));
    }
}
