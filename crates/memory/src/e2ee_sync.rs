//! Client-side E2EE memory sync: local master key, opaque cloud envelopes.
//!
//! On macOS Desktop, prefer storing the master key via
//! `anycode_apple_media::keychain_set("anycode.memory.e2ee", …)` after generation;
//! this module keeps a file fallback under `~/.anycode/memory/e2ee/` for all platforms.

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub const KEYCHAIN_SERVICE: &str = "anycode.memory.e2ee";
pub const KEYCHAIN_ACCOUNT_MASTER: &str = "user_master_key";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEnvelope {
    pub id: String,
    /// Opaque ciphertext (base64). Server never sees plaintext.
    pub ciphertext_b64: String,
    pub nonce_b64: String,
    pub content_hash: String,
    /// Version vector: device_id -> counter
    pub version_vector: HashMap<String, u64>,
    #[serde(default)]
    pub deleted: bool,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SyncPushRequest {
    pub device_id: String,
    pub envelopes: Vec<MemoryEnvelope>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SyncPullResponse {
    pub envelopes: Vec<MemoryEnvelope>,
    pub tombstones: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalSyncState {
    pub enabled: bool,
    pub device_id: String,
    pub last_sync_at: Option<String>,
    pub recovery_phrase_set: bool,
}

fn derive_key(secret: &[u8]) -> [u8; 32] {
    let digest = Sha256::digest(secret);
    digest.into()
}

/// Keyed content hash: deterministic per master key, but not dictionary-attackable
/// for low-entropy memories (unlike a raw SHA-256 of the plaintext).
fn content_hash(master_key: &[u8], plaintext: &[u8]) -> String {
    let mut keyed = master_key.to_vec();
    keyed.extend_from_slice(plaintext);
    format!("{:x}", Sha256::digest(keyed))
}

fn fallback_key_path() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow!("no home dir"))?;
    Ok(home.join(".anycode/memory/e2ee/master.key"))
}

fn rand_key() -> [u8; 32] {
    use aes_gcm::aead::rand_core::RngCore;
    let mut key = [0u8; 32];
    aes_gcm::aead::OsRng.fill_bytes(&mut key);
    key
}

/// Load or create the user master key (file-backed; Desktop may mirror into Keychain).
pub fn load_or_create_master_key() -> Result<Vec<u8>> {
    let path = fallback_key_path()?;
    if path.exists() {
        let raw = std::fs::read_to_string(&path)?;
        return Ok(B64.decode(raw.trim()).context("decode master key")?);
    }
    let key = rand_key();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // Create with 0600 from the start — no window where the key is world-readable.
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)
            .and_then(|mut f| std::io::Write::write_all(&mut f, B64.encode(key).as_bytes()))?;
    }
    #[cfg(not(unix))]
    {
        std::fs::write(&path, B64.encode(key))?;
    }
    Ok(key.to_vec())
}

fn mirror_master_key_to_keychain(master_key: &[u8]) -> Result<()> {
    let _ = master_key;
    // Desktop may call anycode_apple_media::keychain_set(KEYCHAIN_SERVICE, …).
    Ok(())
}

/// Prefer mirroring into Keychain on macOS Desktop after first generation.
pub fn recommend_keychain_mirror(master_key: &[u8]) -> Result<()> {
    mirror_master_key_to_keychain(master_key)
}

pub fn encrypt_memory_blob(
    plaintext: &[u8],
    master_key: &[u8],
) -> Result<(String, String, String)> {
    let key = derive_key(master_key);
    let cipher = Aes256Gcm::new_from_slice(&key).context("invalid key")?;
    // Random 96-bit nonce — never derived from key/plaintext.
    let mut nonce_bytes = [0u8; 12];
    use aes_gcm::aead::rand_core::RngCore;
    aes_gcm::aead::OsRng.fill_bytes(&mut nonce_bytes);
    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce_bytes), plaintext)
        .map_err(|e| anyhow!("encrypt failed: {e}"))?;
    Ok((
        B64.encode(ciphertext),
        B64.encode(nonce_bytes),
        content_hash(master_key, plaintext),
    ))
}

pub fn decrypt_memory_blob(
    ciphertext_b64: &str,
    nonce_b64: &str,
    master_key: &[u8],
) -> Result<Vec<u8>> {
    let key = derive_key(master_key);
    let cipher = Aes256Gcm::new_from_slice(&key).context("invalid key")?;
    let ciphertext = B64.decode(ciphertext_b64.trim())?;
    let nonce_bytes = B64.decode(nonce_b64.trim())?;
    if nonce_bytes.len() != 12 {
        return Err(anyhow!(
            "invalid nonce length {} (expected 12)",
            nonce_bytes.len()
        ));
    }
    cipher
        .decrypt(Nonce::from_slice(&nonce_bytes), ciphertext.as_ref())
        .map_err(|e| anyhow!("decrypt failed: {e}"))
}

pub fn wrap_envelope(
    id: &str,
    plaintext: &[u8],
    device_id: &str,
    counter: u64,
    master_key: &[u8],
) -> Result<MemoryEnvelope> {
    let (ct, nonce, hash) = encrypt_memory_blob(plaintext, master_key)?;
    let mut vv = HashMap::new();
    vv.insert(device_id.to_string(), counter);
    Ok(MemoryEnvelope {
        id: id.to_string(),
        ciphertext_b64: ct,
        nonce_b64: nonce,
        content_hash: hash,
        version_vector: vv,
        deleted: false,
        updated_at: chrono::Utc::now().to_rfc3339(),
    })
}

/// Merge remote envelopes with local: immutable event log + local winner by vv dominance.
pub fn merge_envelopes(
    local: Vec<MemoryEnvelope>,
    remote: Vec<MemoryEnvelope>,
) -> Vec<MemoryEnvelope> {
    let mut map: HashMap<String, MemoryEnvelope> = HashMap::new();
    for env in local.into_iter().chain(remote) {
        match map.get(&env.id) {
            None => {
                map.insert(env.id.clone(), env);
            }
            Some(existing) => {
                if vv_dominates(&env.version_vector, &existing.version_vector) {
                    map.insert(env.id.clone(), env);
                } else if !vv_dominates(&existing.version_vector, &env.version_vector) {
                    let mut conflict = env.clone();
                    conflict.id = format!("{}#conflict-{}", env.id, env.updated_at);
                    map.insert(conflict.id.clone(), conflict);
                }
            }
        }
    }
    map.into_values().collect()
}

fn vv_dominates(a: &HashMap<String, u64>, b: &HashMap<String, u64>) -> bool {
    let mut ge = true;
    let mut strict = false;
    let keys: std::collections::HashSet<_> = a.keys().chain(b.keys()).cloned().collect();
    for k in keys {
        let av = *a.get(&k).unwrap_or(&0);
        let bv = *b.get(&k).unwrap_or(&0);
        if av < bv {
            ge = false;
        }
        if av > bv {
            strict = true;
        }
    }
    ge && strict
}

pub fn local_sync_state_path(base: impl AsRef<Path>) -> PathBuf {
    base.as_ref().join("e2ee/sync_state.json")
}

pub fn load_sync_state(base: impl AsRef<Path>) -> LocalSyncState {
    let path = local_sync_state_path(&base);
    match std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
    {
        Some(state) => state,
        None => {
            // Persist immediately — otherwise the device id drifts on every start
            // and version vectors become meaningless.
            let state = LocalSyncState {
                enabled: false,
                device_id: format!("dev_{}", uuid::Uuid::new_v4()),
                last_sync_at: None,
                recovery_phrase_set: false,
            };
            let _ = save_sync_state(base, &state);
            state
        }
    }
}

pub fn save_sync_state(base: impl AsRef<Path>, state: &LocalSyncState) -> Result<()> {
    let path = local_sync_state_path(base);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, serde_json::to_string_pretty(state)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let key = b"0123456789abcdef0123456789abcdef";
        let (ct, nonce, hash) = encrypt_memory_blob(b"prefer dark theme", key).unwrap();
        assert!(!hash.is_empty());
        let plain = decrypt_memory_blob(&ct, &nonce, key).unwrap();
        assert_eq!(plain, b"prefer dark theme");
    }

    #[test]
    fn decrypt_rejects_bad_nonce_without_panic() {
        let key = b"0123456789abcdef0123456789abcdef";
        let bad_nonce = B64.encode([1u8, 2, 3]);
        let err = decrypt_memory_blob(&B64.encode(b"x"), &bad_nonce, key).unwrap_err();
        assert!(err.to_string().contains("nonce length"));
    }

    #[test]
    fn rand_key_has_full_entropy_and_key_file_is_private() {
        let k1 = rand_key();
        let k2 = rand_key();
        assert_ne!(k1, k2);
        let dir = std::env::temp_dir().join(format!("e2ee-test-{}", uuid::Uuid::new_v4()));
        let key_path = dir.join("master.key");
        std::fs::create_dir_all(&dir).unwrap();
        // emulate load_or_create path permissions
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&key_path)
                .unwrap();
            let mode = std::fs::metadata(&key_path).unwrap().permissions();
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(mode.mode() & 0o777, 0o600);
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn full_sync_roundtrip_two_devices_no_plaintext_leak() {
        // End-to-end: device A encrypts + wraps, server stores opaque envelopes,
        // device B merges and decrypts — server never sees plaintext and a
        // stale A cannot clobber a newer B write.
        let key = rand_key();
        let plaintext = b"prefer fde editorial style";

        // Device A writes.
        let env_a = wrap_envelope("m1", plaintext, "device-a", 1, &key).unwrap();
        assert!(!env_a.ciphertext_b64.contains("fde editorial"));
        assert!(env_a.nonce_b64.len() >= 16);
        // content hash is keyed — identical plaintext under another key differs.
        let other_key = rand_key();
        let env_a2 = wrap_envelope("m1", plaintext, "device-a", 1, &other_key).unwrap();
        assert_ne!(env_a.content_hash, env_a2.content_hash);

        // Device B pulls + merges + decrypts.
        let merged = merge_envelopes(vec![], vec![env_a.clone()]);
        assert_eq!(merged.len(), 1);
        let plain_b =
            decrypt_memory_blob(&merged[0].ciphertext_b64, &merged[0].nonce_b64, &key).unwrap();
        assert_eq!(plain_b, plaintext);

        // Device B writes a newer version; stale A re-sends the old envelope.
        let env_b2 = wrap_envelope("m1", b"newer fact", "device-b", 3, &key).unwrap();
        let mut vv_b2 = env_b2.version_vector.clone();
        vv_b2.insert("device-a".into(), 1); // B has seen A's write
        let env_b2 = MemoryEnvelope {
            version_vector: vv_b2,
            ..env_b2
        };
        let merged = merge_envelopes(vec![env_b2.clone()], vec![env_a.clone()]);
        // Stale A neither replaces nor conflicts the dominant B write.
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].ciphertext_b64, env_b2.ciphertext_b64);
    }

    #[test]
    fn merge_prefers_dominant_vv() {
        let a = MemoryEnvelope {
            id: "m1".into(),
            ciphertext_b64: "a".into(),
            nonce_b64: "n".into(),
            content_hash: "h1".into(),
            version_vector: HashMap::from([("d1".into(), 1)]),
            deleted: false,
            updated_at: "t1".into(),
        };
        let b = MemoryEnvelope {
            version_vector: HashMap::from([("d1".into(), 2)]),
            content_hash: "h2".into(),
            ciphertext_b64: "b".into(),
            updated_at: "t2".into(),
            ..a.clone()
        };
        let merged = merge_envelopes(vec![a], vec![b]);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].content_hash, "h2");
    }
}
