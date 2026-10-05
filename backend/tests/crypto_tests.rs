use dialect_coach_backend::crypto::{jwt, password};
use std::env;
use uuid::Uuid;

const TEST_SECRET: &str = "test_secret_key_for_jwt_testing_only";

fn setup_test_env() {
    unsafe {
        env::set_var("JWT_SECRET", TEST_SECRET);
    }
}

#[test]
fn test_password_hash_produces_different_hashes() {
    let password = "test_password_123";
    let hash1 = password::hash_password(password).expect("Failed to hash password");
    let hash2 = password::hash_password(password).expect("Failed to hash password");

    assert_ne!(
        hash1, hash2,
        "Same password should produce different hashes due to salt randomness"
    );
}

#[test]
fn test_password_verification_correct() {
    let password = "correct_password";
    let hash = password::hash_password(password).expect("Failed to hash password");

    let result = password::verify_password(password, &hash).expect("Failed to verify password");
    assert!(result, "Correct password should return true");
}

#[test]
fn test_password_verification_incorrect() {
    let password = "correct_password";
    let wrong_password = "wrong_password";
    let hash = password::hash_password(password).expect("Failed to hash password");

    let result =
        password::verify_password(wrong_password, &hash).expect("Failed to verify password");
    assert!(!result, "Incorrect password should return false");
}

#[test]
fn test_jwt_generation_produces_valid_format() {
    setup_test_env();
    let user_id = Uuid::new_v4();

    let token = jwt::generate_token(user_id).expect("Failed to generate token");

    assert!(!token.is_empty(), "Token should not be empty");
    assert_eq!(
        token.matches('.').count(),
        2,
        "JWT should have three parts separated by dots"
    );
}

#[test]
fn test_jwt_validation_returns_correct_user_id() {
    setup_test_env();
    let user_id = Uuid::new_v4();

    let token = jwt::generate_token(user_id).expect("Failed to generate token");
    let validated_id =
        jwt::validate_token(&token, jwt::Expiry::Enforce).expect("Failed to validate token");

    assert_eq!(
        user_id, validated_id,
        "Validated user_id should match original"
    );
}

#[test]
fn test_jwt_validation_expired_token() {
    setup_test_env();

    let expired_token = "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJzdWIiOiI1NTU1NTU1NS01NTU1LTU1NTUtNTU1NS01NTU1NTU1NTU1NTUiLCJleHAiOjB9.TH7FzEghUqU-FYBPYPl8wLX8n6j2Y8fBCXOyH98VN3A";

    let result = jwt::validate_token(expired_token, jwt::Expiry::Enforce);
    assert!(result.is_err(), "Expired token should return error");
}

#[test]
fn test_jwt_validation_invalid_token() {
    setup_test_env();

    let invalid_token = "invalid.token.here";

    let result = jwt::validate_token(invalid_token, jwt::Expiry::Enforce);
    assert!(result.is_err(), "Invalid token should return error");
}

#[test]
fn test_jwt_expired_signed_token_passes_only_when_expiry_ignored() {
    setup_test_env();
    let user_id = Uuid::new_v4();
    let claims = jwt::Claims {
        sub: user_id.to_string(),
        exp: 0,
    };
    let token = jsonwebtoken::encode(
        &jsonwebtoken::Header::new(jsonwebtoken::Algorithm::HS256),
        &claims,
        &jsonwebtoken::EncodingKey::from_secret(TEST_SECRET.as_bytes()),
    )
    .expect("Failed to encode token");

    assert!(jwt::validate_token(&token, jwt::Expiry::Enforce).is_err());
    assert_eq!(
        jwt::validate_token(&token, jwt::Expiry::Ignore).expect("signature is valid"),
        user_id
    );
}
