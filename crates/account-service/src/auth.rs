use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rand::rngs::OsRng;
use sha2::{Digest, Sha256};
use uuid::Uuid;

pub fn hash_password(password: &str) -> String {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .expect("Argon2id password hashing failed")
        .to_string()
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    let hash = hash.trim();
    if hash.starts_with("$argon2id$") {
        return PasswordHash::new(hash).ok().is_some_and(|parsed| {
            Argon2::default()
                .verify_password(password.as_bytes(), &parsed)
                .is_ok()
        });
    }
    if let Some((salt, expected)) = parse_sha256_password_hash(hash) {
        let mut hasher = Sha256::new();
        hasher.update(salt.as_bytes());
        hasher.update(b":");
        hasher.update(password.as_bytes());
        let actual = hex_lower(&hasher.finalize());
        return constant_time_eq(actual.as_bytes(), expected.as_bytes());
    }
    false
}

pub fn password_needs_rehash(hash: &str) -> bool {
    !hash.trim().starts_with("$argon2id$")
}

pub fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex_lower(&hasher.finalize())
}

pub fn new_session_token() -> String {
    format!("acct_{}", Uuid::new_v4())
}

pub fn new_api_key_plaintext() -> String {
    format!("ackey_{}", Uuid::new_v4().simple())
}

fn parse_sha256_password_hash(hash: &str) -> Option<(&str, &str)> {
    let mut parts = hash.split('$');
    let scheme = parts.next()?;
    let salt = parts.next()?;
    let expected = parts.next()?;
    if parts.next().is_some() || scheme != "sha256" || salt.is_empty() || expected.len() != 64 {
        return None;
    }
    Some((salt, expected))
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    let max_len = a.len().max(b.len());
    let mut diff = a.len() ^ b.len();
    for i in 0..max_len {
        let av = a.get(i).copied().unwrap_or(0);
        let bv = b.get(i).copied().unwrap_or(0);
        diff |= (av ^ bv) as usize;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_roundtrip() {
        let hash = hash_password("secret123");
        assert!(hash.starts_with("$argon2id$"));
        assert!(verify_password("secret123", &hash));
        assert!(!verify_password("wrong", &hash));
    }

    #[test]
    fn legacy_sha256_remains_verifiable() {
        let salt = "legacy";
        let mut hasher = Sha256::new();
        hasher.update(b"legacy:secret123");
        let hash = format!("sha256${salt}${}", hex_lower(&hasher.finalize()));
        assert!(verify_password("secret123", &hash));
        assert!(password_needs_rehash(&hash));
    }
}
