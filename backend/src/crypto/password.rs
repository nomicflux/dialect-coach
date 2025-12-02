use bcrypt::{hash, verify, BcryptError};

const COST: u32 = 12;

pub fn hash_password(password: &str) -> Result<String, BcryptError> {
    hash(password, COST)
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool, BcryptError> {
    verify(password, hash)
}
