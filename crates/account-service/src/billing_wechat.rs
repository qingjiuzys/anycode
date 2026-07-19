//! WeChat Pay API v3 — Native (扫码) prepaid checkout.

use crate::billing::{insert_pending_order, PaymentOrderView, PendingOrderInput};
use crate::config::ServiceConfig;
use crate::db::AccountDb;
use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use chrono::{Duration, Utc};
use rsa::pkcs1v15::SigningKey;
use rsa::pkcs8::DecodePrivateKey;
use rsa::signature::{RandomizedSigner, SignatureEncoding};
use rsa::RsaPrivateKey;
use serde::Deserialize;
use sha2::Sha256;
use uuid::Uuid;

const WECHAT_API_BASE: &str = "https://api.mch.weixin.qq.com";

pub fn wechat_pay_configured(config: &ServiceConfig) -> bool {
    config.wechat_pay_app_id.is_some()
        && config.wechat_pay_mch_id.is_some()
        && config.wechat_pay_serial_no.is_some()
        && config.wechat_private_key_pem().is_ok()
        && config.wechat_pay_api_v3_key.is_some()
        && config.wechat_pay_notify_url.is_some()
        && (config.wechat_pay_skip_verify
            || config.wechat_notify_verify_pem().ok().flatten().is_some())
}

pub async fn plan_amount_fen(
    db: &AccountDb,
    config: &ServiceConfig,
    plan: &str,
    cycle: &str,
) -> Result<i32> {
    let from_env = match plan {
        "cloud_5h" => config.wechat_price_cloud_5h_fen,
        "pro" => config.wechat_price_pro_monthly_fen,
        "team" => config.wechat_price_team_monthly_fen,
        _ => return Err(anyhow!("invalid plan for wechat pay")),
    };
    let monthly = if let Some(v) = from_env {
        v
    } else {
        let limits = crate::plan::limits_for_plan(db, plan).await;
        if limits.monthly_price_fen <= 0 {
            return Err(anyhow!("plan has no configured price"));
        }
        limits.monthly_price_fen
    };
    Ok(match cycle {
        "monthly" => monthly,
        "yearly" => {
            if from_env.is_some() {
                monthly * 10
            } else {
                let limits = crate::plan::limits_for_plan(db, plan).await;
                if limits.yearly_price_fen > 0 {
                    limits.yearly_price_fen
                } else {
                    monthly * 10
                }
            }
        }
        _ => return Err(anyhow!("invalid billing cycle")),
    })
}

pub async fn create_native_order(
    config: &ServiceConfig,
    db: &AccountDb,
    org_id: &str,
    plan: &str,
    billing_cycle: &str,
) -> Result<PaymentOrderView> {
    if !wechat_pay_configured(config) {
        return Err(anyhow!("WeChat Pay is not configured"));
    }
    let amount_fen = plan_amount_fen(db, config, plan, billing_cycle).await?;
    let out_trade_no = Uuid::new_v4().simple().to_string();
    let description = if plan == "cloud_5h" {
        "anycode Cloud 5h 配额包（1000次/5小时）".to_string()
    } else {
        format!(
            "anycode {} {}",
            plan,
            if billing_cycle == "yearly" {
                "1 year"
            } else {
                "1 month"
            }
        )
    };
    let notify_url = config
        .wechat_pay_notify_url
        .as_deref()
        .ok_or_else(|| anyhow!("WECHAT_PAY_NOTIFY_URL missing"))?;

    let body = serde_json::json!({
        "appid": config.wechat_pay_app_id,
        "mchid": config.wechat_pay_mch_id,
        "description": description,
        "out_trade_no": out_trade_no,
        "notify_url": notify_url,
        "amount": {
            "total": amount_fen,
            "currency": "CNY"
        }
    });
    let body_str = serde_json::to_string(&body)?;

    let path = "/v3/pay/transactions/native";
    let auth = build_authorization(config, "POST", path, &body_str)?;
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{WECHAT_API_BASE}{path}"))
        .header("Authorization", auth)
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .body(body_str)
        .send()
        .await
        .context("wechat native pay request")?;

    let status = resp.status();
    let text = resp.text().await.context("wechat native pay body")?;
    if !status.is_success() {
        return Err(anyhow!("wechat pay error {status}: {text}"));
    }

    let parsed: NativePayResponse = serde_json::from_str(&text)
        .with_context(|| format!("wechat pay response parse: {text}"))?;
    let expires_at = Utc::now() + Duration::minutes(30);
    let order_id = insert_pending_order(
        db,
        &PendingOrderInput {
            org_id: org_id.to_string(),
            provider: "wechat".into(),
            plan: plan.into(),
            billing_cycle: billing_cycle.into(),
            amount_fen,
            currency: "CNY".into(),
            out_trade_no,
            code_url: Some(parsed.code_url.clone()),
            expires_at,
        },
    )
    .await?;

    Ok(PaymentOrderView {
        id: order_id,
        provider: "wechat".into(),
        plan: plan.into(),
        billing_cycle: billing_cycle.into(),
        amount_fen,
        currency: "CNY".into(),
        status: "pending".into(),
        code_url: Some(parsed.code_url),
        expires_at: expires_at.to_rfc3339(),
        paid_at: None,
    })
}

#[derive(Debug, Deserialize)]
struct NativePayResponse {
    code_url: String,
}

#[derive(Debug, Deserialize)]
pub struct WechatNotifyEnvelope {
    pub id: String,
    pub event_type: String,
    pub resource: WechatNotifyResource,
}

#[derive(Debug, Deserialize)]
pub struct WechatNotifyResource {
    pub ciphertext: String,
    pub nonce: String,
    pub associated_data: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WechatTransaction {
    out_trade_no: String,
    transaction_id: String,
    trade_state: String,
}

pub async fn handle_wechat_notify(
    config: &ServiceConfig,
    db: &AccountDb,
    headers: &axum::http::HeaderMap,
    body: &str,
) -> Result<()> {
    if config.wechat_pay_skip_verify {
        tracing::warn!("WECHAT_PAY_SKIP_VERIFY=1: skipping notify signature verification");
    } else if wechat_pay_configured(config) {
        verify_notify_signature(config, headers, body)?;
    }

    let envelope: WechatNotifyEnvelope =
        serde_json::from_str(body).context("wechat notify json")?;
    if envelope.event_type != "TRANSACTION.SUCCESS" {
        return Ok(());
    }

    let api_v3_key = config
        .wechat_pay_api_v3_key
        .as_deref()
        .ok_or_else(|| anyhow!("WECHAT_PAY_API_V3_KEY missing"))?;
    let plaintext = decrypt_resource(api_v3_key, &envelope.resource)?;
    let txn: WechatTransaction =
        serde_json::from_str(&plaintext).context("wechat transaction json")?;
    if txn.trade_state != "SUCCESS" {
        return Ok(());
    }

    crate::billing::mark_order_paid_by_out_trade_no(db, &txn.out_trade_no, &txn.transaction_id)
        .await?;
    Ok(())
}

fn verify_notify_signature(
    config: &ServiceConfig,
    headers: &axum::http::HeaderMap,
    body: &str,
) -> Result<()> {
    let timestamp = header_str(headers, "Wechatpay-Timestamp")?;
    let nonce = header_str(headers, "Wechatpay-Nonce")?;
    let signature = header_str(headers, "Wechatpay-Signature")?;
    let _serial = header_str(headers, "Wechatpay-Serial")?;

    let verify_pem = config
        .wechat_notify_verify_pem()
        .ok()
        .flatten()
        .ok_or_else(|| {
            anyhow!("WECHAT_PAY_PUBLIC_KEY_PATH or WECHAT_PAY_PLATFORM_CERT required for notify verification")
        })?;

    let message = format!("{timestamp}\n{nonce}\n{body}\n");
    let sig_bytes = B64.decode(signature).context("wechat signature base64")?;

    use rsa::pkcs1v15::VerifyingKey;
    use rsa::pkcs8::DecodePublicKey;
    use rsa::signature::Verifier;
    use rsa::RsaPublicKey;

    let public_key =
        RsaPublicKey::from_public_key_pem(&verify_pem).context("wechat notify verify key pem")?;
    let verifying_key = VerifyingKey::<Sha256>::new(public_key);
    let sig = rsa::pkcs1v15::Signature::try_from(sig_bytes.as_slice())
        .map_err(|e| anyhow!("invalid signature bytes: {e}"))?;
    verifying_key
        .verify(message.as_bytes(), &sig)
        .map_err(|_| anyhow!("wechat notify signature mismatch"))?;
    Ok(())
}

fn header_str(headers: &axum::http::HeaderMap, name: &str) -> Result<String> {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("missing header {name}"))
}

fn decrypt_resource(api_v3_key: &str, resource: &WechatNotifyResource) -> Result<String> {
    use aes_gcm::aead::{Aead, KeyInit};
    use aes_gcm::{Aes256Gcm, Nonce};

    if api_v3_key.len() != 32 {
        return Err(anyhow!("WECHAT_PAY_API_V3_KEY must be 32 bytes"));
    }
    let cipher = Aes256Gcm::new_from_slice(api_v3_key.as_bytes())
        .map_err(|e| anyhow!("aes key init: {e}"))?;
    let nonce_bytes = resource.nonce.as_bytes();
    if nonce_bytes.len() != 12 {
        return Err(anyhow!("invalid notify nonce length"));
    }
    let nonce = Nonce::from_slice(nonce_bytes);
    let ciphertext = B64
        .decode(&resource.ciphertext)
        .context("ciphertext base64")?;
    let associated = resource.associated_data.as_deref().unwrap_or("");
    let plain = cipher
        .decrypt(
            nonce,
            aes_gcm::aead::Payload {
                msg: &ciphertext,
                aad: associated.as_bytes(),
            },
        )
        .map_err(|e| anyhow!("decrypt notify resource: {e}"))?;
    String::from_utf8(plain).context("notify plaintext utf8")
}

fn build_authorization(
    config: &ServiceConfig,
    method: &str,
    path: &str,
    body: &str,
) -> Result<String> {
    let mch_id = config
        .wechat_pay_mch_id
        .as_deref()
        .ok_or_else(|| anyhow!("WECHAT_PAY_MCH_ID missing"))?;
    let serial = config
        .wechat_pay_serial_no
        .as_deref()
        .ok_or_else(|| anyhow!("WECHAT_PAY_SERIAL_NO missing"))?;
    let pem = config.wechat_private_key_pem()?;
    let private_key = RsaPrivateKey::from_pkcs8_pem(&pem).context("parse merchant private key")?;
    let signing_key = SigningKey::<Sha256>::new(private_key);

    let timestamp = Utc::now().timestamp().to_string();
    let nonce: String = Uuid::new_v4().simple().to_string();
    let message = format!("{method}\n{path}\n{timestamp}\n{nonce}\n{body}\n");
    let signature = signing_key.sign_with_rng(&mut rand::thread_rng(), message.as_bytes());
    let sig_b64 = B64.encode(signature.to_bytes());

    Ok(format!(
        "WECHATPAY2-SHA256-RSA2048 mchid=\"{mch_id}\",nonce_str=\"{nonce}\",signature=\"{sig_b64}\",timestamp=\"{timestamp}\",serial_no=\"{serial}\""
    ))
}

#[cfg(test)]
mod tests {
    use super::build_authorization;
    use crate::config::ServiceConfig;

    fn test_config_with_key(pem: &str) -> ServiceConfig {
        ServiceConfig {
            database_url: String::new(),
            host: String::new(),
            port: 43200,
            cors_origins: vec![],
            portal_dir: None,
            portal_url: "http://127.0.0.1:43200".into(),
            stripe_secret_key: None,
            stripe_webhook_secret: None,
            stripe_price_pro: None,
            stripe_price_team: None,
            wechat_pay_app_id: Some("wxtest".into()),
            wechat_pay_mch_id: Some("1900000109".into()),
            wechat_pay_serial_no: Some("serial".into()),
            wechat_pay_private_key: Some(pem.into()),
            wechat_pay_private_key_path: None,
            wechat_pay_api_v3_key: Some("01234567890123456789012345678901".into()),
            wechat_pay_notify_url: Some("https://example.com/notify".into()),
            wechat_pay_platform_cert: None,
            wechat_pay_platform_cert_path: None,
            wechat_pay_public_key_path: None,
            wechat_pay_skip_verify: true,
            wechat_price_pro_monthly_fen: Some(19900),
            wechat_price_team_monthly_fen: Some(69900),
            wechat_price_cloud_5h_fen: Some(4900),
            default_payment_provider: "wechat".into(),
            model_gateway_url: "http://127.0.0.1:43210".into(),
            upstream_key_encryption_secret: None,
            ops_portal_dir: None,
            admin_bootstrap_email: None,
            admin_bootstrap_password: None,
            smtp_host: "smtp.exmail.qq.com".into(),
            smtp_port: 465,
            smtp_username: "admin@lingqicloud.com".into(),
            smtp_password: None,
            identity_encryption_secret: None,
            audit_encryption_secret: None,
        }
    }

    use rsa::pkcs8::{EncodePrivateKey, LineEnding};
    use rsa::RsaPrivateKey;

    #[test]
    fn authorization_header_format() {
        let mut rng = rand::thread_rng();
        let private_key = RsaPrivateKey::new(&mut rng, 2048).expect("rsa key");
        let pem = private_key
            .to_pkcs8_pem(LineEnding::LF)
            .expect("pem")
            .to_string();
        let config = test_config_with_key(&pem);
        let auth = build_authorization(&config, "POST", "/v3/pay/transactions/native", "{}")
            .expect("auth");
        assert!(auth.starts_with("WECHATPAY2-SHA256-RSA2048 "));
        assert!(auth.contains("mchid=\"1900000109\""));
    }
}
