use dialect_coach_backend::crypto;
use uuid::Uuid;

/// Test JWT token generation and validation
#[tokio::test]
async fn test_jwt_generation_and_validation() {
    // Set a test JWT secret (required for JWT operations)
    unsafe {
        std::env::set_var(
            "JWT_SECRET",
            "test-secret-key-that-is-at-least-32-characters-long",
        );
    }

    let user_id = Uuid::new_v4();

    // Generate JWT token
    let token = crypto::jwt::generate_token(user_id).expect("Token generation should succeed");

    // Token should be non-empty
    assert!(!token.is_empty());

    // Validate token and extract user_id
    let extracted_user_id = crypto::jwt::validate_token(&token, crypto::jwt::Expiry::Enforce)
        .expect("Token validation should succeed");

    assert_eq!(extracted_user_id, user_id);
}

/// Test JWT validation with invalid token
#[tokio::test]
async fn test_jwt_validation_with_invalid_token() {
    unsafe {
        std::env::set_var(
            "JWT_SECRET",
            "test-secret-key-that-is-at-least-32-characters-long",
        );
    }

    let invalid_token = "invalid.jwt.token";

    let result = crypto::jwt::validate_token(invalid_token, crypto::jwt::Expiry::Enforce);

    assert!(result.is_err());
}

/// Test password hashing produces different hashes for same input
#[tokio::test]
async fn test_password_hashing_produces_different_hashes() {
    let password = "test_password_123";

    let hash1 = crypto::password::hash_password(password).expect("Hashing should succeed");
    let hash2 = crypto::password::hash_password(password).expect("Hashing should succeed");

    // Same password should produce different hashes (due to random salt)
    assert_ne!(hash1, hash2);

    // But both should verify against the original password
    assert!(crypto::password::verify_password(password, &hash1).expect("Verification should work"));
    assert!(crypto::password::verify_password(password, &hash2).expect("Verification should work"));
}

/// Test password verification with correct password
#[tokio::test]
async fn test_password_verification_success() {
    let password = "secure_password_123";
    let hash = crypto::password::hash_password(password).expect("Hashing should succeed");

    let result =
        crypto::password::verify_password(password, &hash).expect("Verification should not error");

    assert!(result, "Password verification should succeed");
}

/// Test password verification with incorrect password
#[tokio::test]
async fn test_password_verification_failure() {
    let password = "secure_password_123";
    let hash = crypto::password::hash_password(password).expect("Hashing should succeed");

    let result = crypto::password::verify_password("wrong_password", &hash)
        .expect("Verification should not error");

    assert!(
        !result,
        "Password verification should fail for wrong password"
    );
}
