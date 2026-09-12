use crate::Result;
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

pub fn bytes_digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// Stable for JSON values regardless of map implementation or insertion order.
/// Floats use serde_json's number encoding; callers must not use NaN/Infinity.
pub fn canonical(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut keys: Vec<_> = map.keys().collect();
            keys.sort();
            let mut result = serde_json::Map::new();
            for key in keys {
                result.insert(key.clone(), canonical(&map[key]));
            }
            Value::Object(result)
        }
        Value::Array(values) => Value::Array(values.iter().map(canonical).collect()),
        _ => value.clone(),
    }
}
pub fn digest<T: Serialize>(value: &T) -> Result<String> {
    Ok(bytes_digest(&serde_json::to_vec(&canonical(
        &serde_json::to_value(value)?,
    ))?))
}
