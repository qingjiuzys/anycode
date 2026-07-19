use anyhow::{Context, Result};
use std::env;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ServiceConfig {
    pub database_url: String,
    pub host: String,
    pub port: u16,
    pub cors_origins: Vec<String>,
    pub portal_dir: Option<PathBuf>,
    pub portal_url: String,
    pub stripe_secret_key: Option<String>,
    pub stripe_webhook_secret: Option<String>,
    pub stripe_price_pro: Option<String>,
    pub stripe_price_team: Option<String>,
    pub wechat_pay_app_id: Option<String>,
    pub wechat_pay_mch_id: Option<String>,
    pub wechat_pay_serial_no: Option<String>,
    pub wechat_pay_private_key: Option<String>,
    pub wechat_pay_private_key_path: Option<PathBuf>,
    pub wechat_pay_api_v3_key: Option<String>,
    pub wechat_pay_notify_url: Option<String>,
    pub wechat_pay_platform_cert: Option<String>,
    pub wechat_pay_platform_cert_path: Option<PathBuf>,
    pub wechat_pay_public_key_path: Option<PathBuf>,
    pub wechat_pay_skip_verify: bool,
    pub wechat_price_pro_monthly_fen: Option<i32>,
    pub wechat_price_team_monthly_fen: Option<i32>,
    pub wechat_price_cloud_5h_fen: Option<i32>,
    pub default_payment_provider: String,
    pub model_gateway_url: String,
    pub upstream_key_encryption_secret: Option<String>,
    pub ops_portal_dir: Option<PathBuf>,
    pub admin_bootstrap_email: Option<String>,
    pub admin_bootstrap_password: Option<String>,
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_username: String,
    pub smtp_password: Option<String>,
    pub identity_encryption_secret: Option<String>,
    pub audit_encryption_secret: Option<String>,
}

impl ServiceConfig {
    pub fn from_env() -> Result<Self> {
        let database_url = env::var("DATABASE_URL")
            .context("DATABASE_URL is required (MySQL connection string, e.g. mysql://user:pass@host:3306/anycode)")?;
        let host = env::var("ACCOUNT_SERVICE_HOST").unwrap_or_else(|_| "0.0.0.0".into());
        let port = env::var("ACCOUNT_SERVICE_PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(43200);
        let cors_origins = env::var("CORS_ORIGINS")
            .unwrap_or_else(|_| {
                "http://127.0.0.1:43180,http://localhost:43180,http://127.0.0.1:5173,http://localhost:5173,http://127.0.0.1:43200,http://localhost:43200".into()
            })
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        let portal_dir = env::var("ACCOUNT_PORTAL_DIR")
            .ok()
            .map(PathBuf::from)
            .filter(|p| p.is_dir());
        let portal_url =
            env::var("ACCOUNT_PORTAL_URL").unwrap_or_else(|_| format!("http://127.0.0.1:{port}"));
        let stripe_secret_key = env::var("STRIPE_SECRET_KEY").ok().filter(|s| !s.is_empty());
        let stripe_webhook_secret = env::var("STRIPE_WEBHOOK_SECRET")
            .ok()
            .filter(|s| !s.is_empty());
        let stripe_price_pro = env::var("STRIPE_PRICE_PRO").ok().filter(|s| !s.is_empty());
        let stripe_price_team = env::var("STRIPE_PRICE_TEAM").ok().filter(|s| !s.is_empty());

        let wechat_pay_app_id = env::var("WECHAT_PAY_APP_ID").ok().filter(|s| !s.is_empty());
        let wechat_pay_mch_id = env::var("WECHAT_PAY_MCH_ID").ok().filter(|s| !s.is_empty());
        let wechat_pay_serial_no = env::var("WECHAT_PAY_SERIAL_NO")
            .ok()
            .filter(|s| !s.is_empty());
        let wechat_pay_private_key = env::var("WECHAT_PAY_PRIVATE_KEY")
            .ok()
            .filter(|s| !s.is_empty());
        let wechat_pay_private_key_path = env::var("WECHAT_PAY_PRIVATE_KEY_PATH")
            .ok()
            .map(PathBuf::from)
            .filter(|p| p.is_file());
        let wechat_pay_api_v3_key = env::var("WECHAT_PAY_API_V3_KEY")
            .ok()
            .filter(|s| !s.is_empty());
        let wechat_pay_notify_url = env::var("WECHAT_PAY_NOTIFY_URL")
            .ok()
            .filter(|s| !s.is_empty())
            .or_else(|| {
                env::var("ACCOUNT_PUBLIC_URL").ok().map(|base| {
                    format!(
                        "{}/api/v1/billing/webhooks/wechat",
                        base.trim_end_matches('/')
                    )
                })
            });
        let wechat_pay_platform_cert = env::var("WECHAT_PAY_PLATFORM_CERT")
            .ok()
            .filter(|s| !s.is_empty());
        let wechat_pay_platform_cert_path = env::var("WECHAT_PAY_PLATFORM_CERT_PATH")
            .ok()
            .map(PathBuf::from)
            .filter(|p| p.is_file());
        let wechat_pay_public_key_path = env::var("WECHAT_PAY_PUBLIC_KEY_PATH")
            .ok()
            .map(PathBuf::from)
            .filter(|p| p.is_file());
        let wechat_pay_skip_verify = env::var("WECHAT_PAY_SKIP_VERIFY")
            .ok()
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        let wechat_price_pro_monthly_fen = env::var("WECHAT_PRICE_PRO_MONTHLY_FEN")
            .or_else(|_| env::var("WECHAT_PRICE_PRO_MONTHLY_CENTS"))
            .ok()
            .and_then(|s| s.parse().ok());
        let wechat_price_team_monthly_fen = env::var("WECHAT_PRICE_TEAM_MONTHLY_FEN")
            .or_else(|_| env::var("WECHAT_PRICE_TEAM_MONTHLY_CENTS"))
            .ok()
            .and_then(|s| s.parse().ok());
        let wechat_price_cloud_5h_fen = env::var("WECHAT_PRICE_CLOUD_5H_FEN")
            .or_else(|_| env::var("WECHAT_PRICE_CLOUD_5H_CENTS"))
            .ok()
            .and_then(|s| s.parse().ok());
        let default_payment_provider =
            env::var("DEFAULT_PAYMENT_PROVIDER").unwrap_or_else(|_| "wechat".into());
        let model_gateway_url = env::var("ANYCODE_MODEL_GATEWAY_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:43210".into())
            .trim_end_matches('/')
            .to_string();
        let upstream_key_encryption_secret = env::var("UPSTREAM_KEY_ENCRYPTION_SECRET")
            .ok()
            .filter(|s| !s.is_empty());
        let ops_portal_dir = env::var("OPS_PORTAL_DIR")
            .ok()
            .map(PathBuf::from)
            .filter(|p| p.is_dir());
        let admin_bootstrap_email = env::var("ADMIN_BOOTSTRAP_EMAIL")
            .ok()
            .filter(|s| !s.is_empty());
        let admin_bootstrap_password = env::var("ADMIN_BOOTSTRAP_PASSWORD")
            .ok()
            .filter(|s| !s.is_empty());
        let smtp_host = env::var("SMTP_HOST").unwrap_or_else(|_| "smtp.exmail.qq.com".to_string());
        let smtp_port = env::var("SMTP_PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(465);
        let smtp_username =
            env::var("SMTP_USERNAME").unwrap_or_else(|_| "admin@lingqicloud.com".to_string());
        let smtp_password = env::var("SMTP_PASSWORD").ok().filter(|s| !s.is_empty());
        let identity_encryption_secret = env::var("IDENTITY_ENCRYPTION_SECRET")
            .ok()
            .filter(|s| !s.is_empty());
        let audit_encryption_secret = env::var("AUDIT_ENCRYPTION_SECRET")
            .ok()
            .filter(|s| !s.is_empty());

        Ok(Self {
            database_url,
            host,
            port,
            cors_origins,
            portal_dir,
            portal_url,
            stripe_secret_key,
            stripe_webhook_secret,
            stripe_price_pro,
            stripe_price_team,
            wechat_pay_app_id,
            wechat_pay_mch_id,
            wechat_pay_serial_no,
            wechat_pay_private_key,
            wechat_pay_private_key_path,
            wechat_pay_api_v3_key,
            wechat_pay_notify_url,
            wechat_pay_platform_cert,
            wechat_pay_platform_cert_path,
            wechat_pay_public_key_path,
            wechat_pay_skip_verify,
            wechat_price_pro_monthly_fen,
            wechat_price_team_monthly_fen,
            wechat_price_cloud_5h_fen,
            default_payment_provider,
            model_gateway_url,
            upstream_key_encryption_secret,
            ops_portal_dir,
            admin_bootstrap_email,
            admin_bootstrap_password,
            smtp_host,
            smtp_port,
            smtp_username,
            smtp_password,
            identity_encryption_secret,
            audit_encryption_secret,
        })
    }

    pub fn wechat_private_key_pem(&self) -> Result<String> {
        if let Some(pem) = &self.wechat_pay_private_key {
            return Ok(pem.replace("\\n", "\n"));
        }
        if let Some(path) = &self.wechat_pay_private_key_path {
            return fs::read_to_string(path)
                .with_context(|| format!("read WECHAT_PAY_PRIVATE_KEY_PATH {}", path.display()));
        }
        Err(anyhow::anyhow!(
            "WECHAT_PAY_PRIVATE_KEY or WECHAT_PAY_PRIVATE_KEY_PATH required"
        ))
    }

    pub fn wechat_platform_cert_pem(&self) -> Result<Option<String>> {
        if let Some(pem) = &self.wechat_pay_platform_cert {
            return Ok(Some(pem.replace("\\n", "\n")));
        }
        if let Some(path) = &self.wechat_pay_platform_cert_path {
            let pem = fs::read_to_string(path).with_context(|| {
                format!("read WECHAT_PAY_PLATFORM_CERT_PATH {}", path.display())
            })?;
            return Ok(Some(pem));
        }
        Ok(None)
    }

    pub fn wechat_notify_verify_pem(&self) -> Result<Option<String>> {
        if let Some(pem) = self.wechat_platform_cert_pem()? {
            return Ok(Some(pem));
        }
        if let Some(path) = &self.wechat_pay_public_key_path {
            let pem = fs::read_to_string(path)
                .with_context(|| format!("read WECHAT_PAY_PUBLIC_KEY_PATH {}", path.display()))?;
            return Ok(Some(pem));
        }
        Ok(None)
    }
}
