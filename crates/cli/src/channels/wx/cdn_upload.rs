//! Outbound CDN upload (getuploadurl + AES-128-ECB POST), aligned with openclaw-weixin.

use super::ilink::{cdn_upload_url, WeChatApi, CDN_BASE};
use aes::cipher::generic_array::GenericArray;
use aes::cipher::{BlockEncrypt, KeyInit};
use aes::Aes128;
use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use md5;
use rand::RngCore;
use serde_json::{json, Value};

/// `getUploadUrl.media_type` for generic files.
pub const UPLOAD_MEDIA_FILE: i64 = 3;

pub struct UploadedCdnMedia {
    pub encrypt_query_param: String,
    pub aes_key_b64: String,
}

pub fn aes_ecb_padded_size(raw: usize) -> usize {
    ((raw + 15) / 16) * 16
}

fn pkcs7_pad(data: &[u8]) -> Vec<u8> {
    let pad = 16 - (data.len() % 16);
    let mut out = data.to_vec();
    out.extend(std::iter::repeat(pad as u8).take(pad));
    out
}

pub fn encrypt_aes_128_ecb(key: &[u8; 16], plaintext: &[u8]) -> Vec<u8> {
    let padded = pkcs7_pad(plaintext);
    let cipher = Aes128::new(GenericArray::from_slice(key));
    let mut out = Vec::with_capacity(padded.len());
    for chunk in padded.chunks_exact(16) {
        let mut block = GenericArray::clone_from_slice(chunk);
        cipher.encrypt_block(&mut block);
        out.extend_from_slice(&block);
    }
    out
}

pub async fn upload_bytes_to_cdn(
    api: &WeChatApi,
    plaintext: &[u8],
    to_user_id: &str,
) -> Result<UploadedCdnMedia> {
    let rawsize = plaintext.len();
    let rawfilemd5 = format!("{:x}", md5::compute(plaintext));
    let filesize = aes_ecb_padded_size(rawsize);
    let mut filekey_bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut filekey_bytes);
    let filekey = hex::encode(filekey_bytes);
    let mut aeskey = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut aeskey);

    let body = json!({
        "filekey": filekey,
        "media_type": UPLOAD_MEDIA_FILE,
        "to_user_id": to_user_id,
        "rawsize": rawsize,
        "rawfilemd5": rawfilemd5,
        "filesize": filesize,
        "no_need_thumb": true,
        "aeskey": hex::encode(aeskey),
    });
    let resp = api.get_upload_url(body).await?;
    let upload_full_url = resp
        .get("upload_full_url")
        .and_then(|x| x.as_str())
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let upload_param = resp
        .get("upload_param")
        .and_then(|x| x.as_str())
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    let ciphertext = encrypt_aes_128_ecb(&aeskey, plaintext);
    let download_param = upload_buffer_with_retries(
        api,
        &ciphertext,
        upload_full_url.as_deref(),
        upload_param.as_deref(),
        &filekey,
    )
    .await?;

    Ok(UploadedCdnMedia {
        encrypt_query_param: download_param,
        aes_key_b64: B64.encode(aeskey),
    })
}

async fn upload_buffer_with_retries(
    api: &WeChatApi,
    ciphertext: &[u8],
    upload_full_url: Option<&str>,
    upload_param: Option<&str>,
    filekey: &str,
) -> Result<String> {
    let url = if let Some(full) = upload_full_url {
        full.to_string()
    } else if let Some(param) = upload_param {
        cdn_upload_url(param, filekey)?
    } else {
        anyhow::bail!("getUploadUrl returned no upload_full_url or upload_param");
    };
    if !url.starts_with(CDN_BASE) && !url.contains("weixin.qq.com") {
        anyhow::bail!("untrusted CDN upload URL");
    }
    let client = api.http_client();
    let mut last_err = None;
    for attempt in 1..=3 {
        let res = client
            .post(&url)
            .header("Content-Type", "application/octet-stream")
            .body(ciphertext.to_vec())
            .timeout(std::time::Duration::from_secs(60))
            .send()
            .await;
        match res {
            Ok(r) => {
                let status = r.status();
                if status.is_client_error() {
                    let msg = r
                        .headers()
                        .get("x-error-message")
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or("client error")
                        .to_string();
                    anyhow::bail!("CDN upload client error {status}: {msg}");
                }
                if !status.is_success() {
                    let msg = r
                        .headers()
                        .get("x-error-message")
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or("server error")
                        .to_string();
                    last_err = Some(anyhow::anyhow!("CDN upload server error: {msg}"));
                    continue;
                }
                let param = r
                    .headers()
                    .get("x-encrypted-param")
                    .and_then(|v| v.to_str().ok())
                    .map(str::to_string)
                    .or_else(|| {
                        // Some gateways return JSON body instead of header.
                        None
                    });
                if let Some(p) = param.filter(|s| !s.is_empty()) {
                    return Ok(p);
                }
                if let Ok(body) = r.json::<Value>().await {
                    for key in [
                        "encrypt_query_param",
                        "encrypted_query_param",
                        "download_param",
                    ] {
                        if let Some(s) = body.get(key).and_then(|x| x.as_str()) {
                            if !s.is_empty() {
                                return Ok(s.to_string());
                            }
                        }
                    }
                }
                last_err = Some(anyhow::anyhow!(
                    "CDN upload response missing x-encrypted-param"
                ));
            }
            Err(e) => {
                last_err = Some(e.into());
            }
        }
        if attempt < 3 {
            tokio::time::sleep(std::time::Duration::from_millis(500 * attempt as u64)).await;
        }
    }
    Err(last_err.unwrap_or_else(|| anyhow::anyhow!("CDN upload failed")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn padded_size_rounds_up() {
        assert_eq!(aes_ecb_padded_size(1), 16);
        assert_eq!(aes_ecb_padded_size(16), 16);
        assert_eq!(aes_ecb_padded_size(17), 32);
    }

    #[test]
    fn encrypt_produces_padded_ciphertext() {
        let key = [7u8; 16];
        let enc = encrypt_aes_128_ecb(&key, b"hello");
        assert_eq!(enc.len(), 16);
    }
}
