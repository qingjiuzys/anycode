use crate::{auth::hash_token, config::ServiceConfig, db::AccountDb};
use anyhow::{anyhow, Context, Result};
use chrono::{Duration, Utc};
use lettre::{
    message::Mailbox,
    transport::smtp::{
        authentication::Credentials,
        client::{Tls, TlsParameters},
    },
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};
use rand::Rng;
use sqlx::Row;
use uuid::Uuid;

const CODE_TTL_MINUTES: i64 = 10;
const RESEND_SECONDS: i64 = 60;
const MAX_PER_HOUR: i64 = 6;

fn code_hash(email: &str, code: &str, pepper: &str) -> String {
    hash_token(&format!(
        "{}:{}:{}",
        email.trim().to_lowercase(),
        code,
        pepper
    ))
}

pub async fn send_registration_code(
    db: &AccountDb,
    config: &ServiceConfig,
    email: &str,
) -> Result<()> {
    let email = email.trim().to_lowercase();
    if !email.contains('@') {
        return Err(anyhow!("valid email required"));
    }
    let password = config
        .smtp_password
        .as_deref()
        .ok_or_else(|| anyhow!("SMTP_PASSWORD is not configured"))?;

    let recent: Option<chrono::DateTime<Utc>> = sqlx::query_scalar(
        "SELECT MAX(created_at) FROM email_verification_codes WHERE email = ? AND purpose = 'registration'",
    )
    .bind(&email)
    .fetch_one(db.pool())
    .await?;
    if recent.is_some_and(|t| t > Utc::now() - Duration::seconds(RESEND_SECONDS)) {
        return Err(anyhow!(
            "please wait 60 seconds before requesting another code"
        ));
    }
    let hourly: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM email_verification_codes WHERE email = ? AND created_at >= DATE_SUB(NOW(), INTERVAL 1 HOUR)",
    )
    .bind(&email)
    .fetch_one(db.pool())
    .await?;
    if hourly >= MAX_PER_HOUR {
        return Err(anyhow!("verification email hourly limit reached"));
    }

    let code = format!("{:06}", rand::thread_rng().gen_range(0..1_000_000u32));
    let message = Message::builder()
        .from(
            config
                .smtp_username
                .parse::<Mailbox>()
                .context("invalid SMTP_USERNAME")?,
        )
        .to(email.parse::<Mailbox>().context("invalid recipient email")?)
        .subject("anyCode 邮箱验证码")
        .body(format!(
            "您的 anyCode 注册验证码是：{code}\n\n验证码 10 分钟内有效，且只能使用一次。如非本人操作请忽略。"
        ))?;
    let tls = TlsParameters::new(config.smtp_host.clone())?;
    let mailer = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&config.smtp_host)
        .port(config.smtp_port)
        .tls(Tls::Wrapper(tls))
        .credentials(Credentials::new(
            config.smtp_username.clone(),
            password.to_string(),
        ))
        .build();
    mailer
        .send(message)
        .await
        .context("send verification email")?;

    sqlx::query(
        "INSERT INTO email_verification_codes (id, email, purpose, code_hash, expires_at) VALUES (?, ?, 'registration', ?, ?)",
    )
    .bind(format!("evc_{}", Uuid::new_v4()))
    .bind(&email)
    .bind(code_hash(&email, &code, password))
    .bind(Utc::now() + Duration::minutes(CODE_TTL_MINUTES))
    .execute(db.pool())
    .await?;
    Ok(())
}

pub async fn consume_registration_code(
    db: &AccountDb,
    config: &ServiceConfig,
    email: &str,
    code: &str,
) -> Result<bool> {
    let email = email.trim().to_lowercase();
    let pepper = config
        .smtp_password
        .as_deref()
        .ok_or_else(|| anyhow!("SMTP_PASSWORD is not configured"))?;
    let row = sqlx::query(
        "SELECT id, code_hash FROM email_verification_codes WHERE email = ? AND purpose = 'registration' AND consumed_at IS NULL AND expires_at > NOW() ORDER BY created_at DESC LIMIT 1",
    )
    .bind(&email)
    .fetch_optional(db.pool())
    .await?;
    let Some(row) = row else {
        return Ok(false);
    };
    let expected: String = row.get("code_hash");
    if expected != code_hash(&email, code.trim(), pepper) {
        return Ok(false);
    }
    let updated = sqlx::query(
        "UPDATE email_verification_codes SET consumed_at = NOW() WHERE id = ? AND consumed_at IS NULL",
    )
    .bind(row.get::<String, _>("id"))
    .execute(db.pool())
    .await?
    .rows_affected();
    Ok(updated == 1)
}
