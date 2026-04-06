//! CDN 下载 + AES-128-ECB 解密 + 消息项解析。

use crate::wx::ilink::cdn_download_url;
use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use serde_json::Value;

fn decrypt_aes_128_ecb(key16: &[u8; 16], ciphertext: &[u8]) -> Result<Vec<u8>> {
    use aes::cipher::generic_array::GenericArray;
    use aes::cipher::{BlockDecrypt, KeyInit};
    use aes::Aes128;
    if ciphertext.len() % 16 != 0 {
        anyhow::bail!("密文长度非 16 倍数");
    }
    let cipher = Aes128::new(GenericArray::from_slice(key16));
    let mut out = Vec::with_capacity(ciphertext.len());
    for chunk in ciphertext.chunks_exact(16) {
        let mut block = GenericArray::clone_from_slice(chunk);
        cipher.decrypt_block(&mut block);
        out.extend_from_slice(&block);
    }
    pkcs7_unpad(&out)
}

fn pkcs7_unpad(data: &[u8]) -> Result<Vec<u8>> {
    if data.is_empty() {
        anyhow::bail!("空数据");
    }
    let n = *data.last().unwrap() as usize;
    if n == 0 || n > 16 || n > data.len() {
        anyhow::bail!("无效 PKCS#7");
    }
    if !data[data.len() - n..].iter().all(|&b| b as usize == n) {
        anyhow::bail!("无效 PKCS#7 填充");
    }
    Ok(data[..data.len() - n].to_vec())
}

fn parse_aes_key(aes_key_b64: &str) -> Result<[u8; 16]> {
    let raw = B64
        .decode(aes_key_b64.as_bytes())
        .context("AES key base64")?;
    if raw.len() == 16 {
        let mut k = [0u8; 16];
        k.copy_from_slice(&raw);
        return Ok(k);
    }
    let hex_str = String::from_utf8_lossy(&raw);
    let v = hex::decode(hex_str.trim())?;
    if v.len() != 16 {
        anyhow::bail!("AES key 长度应为 16 字节");
    }
    let mut k = [0u8; 16];
    k.copy_from_slice(&v);
    Ok(k)
}

pub async fn download_and_decrypt(
    client: &reqwest::Client,
    enc_q: &str,
    aes_key_b64: &str,
) -> Result<Vec<u8>> {
    let url = cdn_download_url(enc_q)?;
    let enc = client
        .get(&url)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .context("CDN GET")?
        .bytes()
        .await
        .context("CDN body")?;
    let key = parse_aes_key(aes_key_b64)?;
    decrypt_aes_128_ecb(&key, &enc)
}

fn detect_mime(head: &[u8]) -> &'static str {
    if head.len() >= 2 && head[0] == 0x89 && head[1] == 0x50 {
        return "image/png";
    }
    if head.len() >= 2 && head[0] == 0xff && head[1] == 0xd8 {
        return "image/jpeg";
    }
    if head.len() >= 2 && head[0] == 0x47 && head[1] == 0x49 {
        return "image/gif";
    }
    if head.len() >= 2 && head[0] == 0x52 && head[1] == 0x49 {
        return "image/webp";
    }
    "image/jpeg"
}

pub async fn download_image_from_item(
    client: &reqwest::Client,
    item: &Value,
) -> Option<(String, String)> {
    let (enc, key) = extract_cdn_keys(item)?;
    let bytes = download_and_decrypt(client, &enc, &key).await.ok()?;
    let mime = detect_mime(&bytes);
    let b64 = B64.encode(&bytes);
    Some((mime.to_string(), b64))
}

fn extract_cdn_keys(item: &Value) -> Option<(String, String)> {
    let cdn = item.get("cdn_media")?;
    let k = cdn.get("aes_key").and_then(|x| x.as_str())?;
    let e = cdn.get("encrypt_query_param").and_then(|x| x.as_str())?;
    if !k.is_empty() && !e.is_empty() {
        return Some((e.to_string(), k.to_string()));
    }
    let media = item.get("media")?;
    let e = media.get("encrypt_query_param")?.as_str()?;
    let k = media
        .get("aes_key")
        .and_then(|x| x.as_str())
        .or_else(|| item.get("aeskey").and_then(|x| x.as_str()))?;
    Some((e.to_string(), k.to_string()))
}

pub fn extract_text_from_item(item: &Value) -> String {
    for ptr in ["/text_item/text", "/textItem/text"] {
        if let Some(s) = item.pointer(ptr).and_then(|x| x.as_str()) {
            if !s.is_empty() {
                return s.to_string();
            }
        }
    }
    item.get("content")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string()
}

pub fn extract_user_text_and_image_item<'a>(items: &'a [Value]) -> (String, Option<&'a Value>) {
    use crate::wx::fields::item_type;
    let mut texts = Vec::new();
    let mut img = None;
    for it in items {
        let t = item_type(it);
        if t == 1 {
            let tx = extract_text_from_item(it);
            if !tx.is_empty() {
                texts.push(tx);
            }
        } else if t == 2 && img.is_none() {
            img = Some(it);
        }
    }
    (texts.join("\n"), img)
}
