use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use sha2::{Digest, Sha256};

fn derive_key(secret: &str) -> [u8; 32] {
    let digest = Sha256::digest(secret.as_bytes());
    digest.into()
}

pub fn encrypt_secret(plaintext: &str, master_secret: &str) -> Result<(String, String)> {
    if master_secret.trim().is_empty() {
        return Err(anyhow!("encryption secret is not configured"));
    }
    let key = derive_key(master_secret);
    let cipher = Aes256Gcm::new_from_slice(&key).context("invalid encryption key")?;
    let nonce_bytes: [u8; 12] = rand::random();
    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce_bytes), plaintext.as_bytes())
        .map_err(|e| anyhow!("encrypt failed: {e}"))?;
    Ok((B64.encode(ciphertext), B64.encode(nonce_bytes)))
}

pub fn decrypt_secret(
    ciphertext_b64: &str,
    nonce_b64: &str,
    master_secret: &str,
) -> Result<String> {
    if master_secret.trim().is_empty() {
        return Err(anyhow!("encryption secret is not configured"));
    }
    let key = derive_key(master_secret);
    let cipher = Aes256Gcm::new_from_slice(&key).context("invalid encryption key")?;
    let ciphertext = B64
        .decode(ciphertext_b64.trim())
        .context("invalid ciphertext encoding")?;
    let nonce_bytes = B64
        .decode(nonce_b64.trim())
        .context("invalid nonce encoding")?;
    let plaintext = cipher
        .decrypt(Nonce::from_slice(&nonce_bytes), ciphertext.as_ref())
        .map_err(|e| anyhow!("decrypt failed: {e}"))?;
    String::from_utf8(plaintext).context("invalid utf-8 in decrypted secret")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_encrypt_decrypt() {
        let secret = "test-master-secret-for-upstream-keys";
        let (ct, nonce) = encrypt_secret("agnes-api-key-123", secret).unwrap();
        let plain = decrypt_secret(&ct, &nonce, secret).unwrap();
        assert_eq!(plain, "agnes-api-key-123");
    }
}
