use anycode_harness_core::{Capabilities, Error, Result, Scope};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContextKind {
    Personal,
    Enterprise,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Admin,
    Editor,
    Viewer,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TenantGrant {
    pub organization_id: Uuid,
    pub tenant_id: Uuid,
    pub external_tenant_id: String,
    pub role: Role,
    pub organization_version: i64,
    pub member_version: i64,
    pub tenant_version: i64,
    pub grant_version: i64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Identity {
    pub active: bool,
    pub iss: String,
    pub sub: Uuid,
    pub aud: String,
    pub context: ContextKind,
    pub tenant: Option<TenantGrant>,
    pub scope: String,
}
impl Identity {
    pub fn validate(&self, issuer: &str, expected_tenant: Option<Uuid>) -> Result<()> {
        if !self.active || self.iss != issuer || self.aud != "anycode" || self.sub.is_nil() {
            return Err(Error::Denied("invalid account identity".into()));
        }
        match (&self.context, &self.tenant) {
            (ContextKind::Personal, None) if expected_tenant.is_none() => {}
            (ContextKind::Enterprise, Some(t)) => {
                if t.organization_id.is_nil()
                    || t.tenant_id.is_nil()
                    || t.external_tenant_id.trim().is_empty()
                    || t.external_tenant_id.len() > 256
                    || [
                        t.organization_version,
                        t.member_version,
                        t.tenant_version,
                        t.grant_version,
                    ]
                    .iter()
                    .any(|v| *v < 0)
                    || expected_tenant.is_some_and(|id| id != t.tenant_id)
                {
                    return Err(Error::Denied("tenant grant mismatch".into()));
                }
            }
            _ => return Err(Error::Denied("identity context/tenant mismatch".into())),
        }
        Ok(())
    }
}
#[derive(Clone, Debug)]
pub struct LocalAuthorization {
    pub project: Uuid,
    pub capabilities: Capabilities,
}
#[async_trait]
pub trait ProductAcl: Send + Sync {
    /// Resolve the local user UUID + product tenant mapping + project membership.
    /// SSO success/enterprise role alone MUST NOT authorize a local project.
    async fn authorize_project(
        &self,
        identity: &Identity,
        project: Uuid,
    ) -> Result<LocalAuthorization>;
}
pub struct AuthorizedScope {
    pub scope: Scope,
    pub capabilities: Capabilities,
}
pub async fn authorize(
    identity: &Identity,
    issuer: &str,
    expected_tenant: Option<Uuid>,
    project: Uuid,
    device: Option<Uuid>,
    acl: &dyn ProductAcl,
) -> Result<AuthorizedScope> {
    identity.validate(issuer, expected_tenant)?;
    let local = acl.authorize_project(identity, project).await?;
    if local.project != project {
        return Err(Error::Denied("ACL returned wrong project".into()));
    }
    let scope = Scope {
        subject: identity.sub,
        organization: identity.tenant.as_ref().map(|t| t.organization_id),
        tenant: identity.tenant.as_ref().map(|t| t.tenant_id),
        project,
        device,
    };
    scope.validate()?;
    // Device enrollment and local computer approval are separate, required checks.
    Ok(AuthorizedScope {
        scope,
        capabilities: local.capabilities,
    })
}
pub struct AccountsClient {
    issuer: String,
    secret: String,
    http: reqwest::Client,
}
impl AccountsClient {
    pub fn new(issuer: &str, secret: String, explicit_loopback_dev: bool) -> Result<Self> {
        let url = reqwest::Url::parse(issuer).map_err(|_| Error::Invalid("issuer URL".into()))?;
        let loopback = matches!(url.host_str(), Some("127.0.0.1" | "localhost" | "[::1]"));
        if url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.path() != "/"
            || url.query().is_some()
            || url.fragment().is_some()
            || !(url.scheme() == "https"
                || explicit_loopback_dev && loopback && url.scheme() == "http")
            || !(32..=512).contains(&secret.len())
        {
            return Err(Error::Invalid(
                "strict origin and strong confidential client secret required".into(),
            ));
        }
        let http = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|_| Error::Host("account HTTP client".into()))?;
        Ok(Self {
            issuer: url.origin().ascii_serialization(),
            secret,
            http,
        })
    }
    pub fn issuer(&self) -> &str {
        &self.issuer
    }
    pub async fn introspect(&self, token: &str, expected_tenant: Option<Uuid>) -> Result<Identity> {
        if token.len() != 43
            || !token
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b))
        {
            return Err(Error::Denied("opaque token required, not UUID/JWT".into()));
        }
        let mut response = self
            .http
            .post(format!("{}/api/v2/sso/introspect", self.issuer))
            .basic_auth("anycode", Some(&self.secret))
            .json(&serde_json::json!({"token":token}))
            .send()
            .await
            .map_err(|_| Error::Host("identity_service_unavailable".into()))?;
        if response.status().is_server_error() {
            return Err(Error::Host("identity_service_unavailable".into()));
        }
        if !response.status().is_success() {
            return Err(Error::Denied("account authentication denied".into()));
        }
        let mut body = vec![];
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| Error::Host("identity response interrupted".into()))?
        {
            if body.len() + chunk.len() > 16384 {
                return Err(Error::Invalid("identity response limit".into()));
            }
            body.extend_from_slice(&chunk);
        }
        let value: serde_json::Value = serde_json::from_slice(&body)?;
        if value["active"] != true {
            return Err(Error::Denied("inactive or revoked identity".into()));
        }
        let identity: Identity = serde_json::from_value(value)?;
        identity.validate(&self.issuer, expected_tenant)?;
        Ok(identity)
    }

    pub async fn exchange_code(
        &self,
        code: &str,
        redirect_uri: &str,
        code_verifier: &str,
    ) -> Result<String> {
        let opaque = |s: &str| {
            s.len() == 43
                && s.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b))
        };
        if !opaque(code) || !opaque(code_verifier) {
            return Err(Error::Denied(
                "opaque PKCE code and verifier required".into(),
            ));
        }
        let redirect =
            reqwest::Url::parse(redirect_uri).map_err(|_| Error::Invalid("redirect_uri".into()))?;
        if !redirect.username().is_empty()
            || redirect.password().is_some()
            || redirect.query().is_some()
            || redirect.fragment().is_some()
        {
            return Err(Error::Invalid(
                "redirect_uri must be an exact registered callback".into(),
            ));
        }
        let mut response = self
            .http
            .post(format!("{}/api/v2/sso/token", self.issuer))
            .basic_auth("anycode", Some(&self.secret))
            .json(&serde_json::json!({
                "code": code,
                "redirect_uri": redirect_uri,
                "code_verifier": code_verifier
            }))
            .send()
            .await
            .map_err(|_| Error::Host("identity_service_unavailable".into()))?;
        if response.status().is_server_error() {
            return Err(Error::Host("identity_service_unavailable".into()));
        }
        if !response.status().is_success() {
            return Err(Error::Denied("authorization code exchange denied".into()));
        }
        let mut body = vec![];
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| Error::Host("identity response interrupted".into()))?
        {
            if body.len() + chunk.len() > 16384 {
                return Err(Error::Invalid("identity response limit".into()));
            }
            body.extend_from_slice(&chunk);
        }
        let value: serde_json::Value = serde_json::from_slice(&body)?;
        let token = value["access_token"]
            .as_str()
            .filter(|t| opaque(t))
            .ok_or_else(|| Error::Denied("opaque access token required".into()))?
            .to_string();
        if value["token_type"] != "Bearer" {
            return Err(Error::Denied("bearer token required".into()));
        }
        Ok(token)
    }

    pub async fn revoke(&self, token: &str) -> Result<()> {
        if token.len() != 43
            || !token
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b))
        {
            return Err(Error::Denied("opaque token required, not UUID/JWT".into()));
        }
        let response = self
            .http
            .post(format!("{}/api/v2/sso/revoke", self.issuer))
            .basic_auth("anycode", Some(&self.secret))
            .json(&serde_json::json!({"token": token}))
            .send()
            .await
            .map_err(|_| Error::Host("identity_service_unavailable".into()))?;
        if response.status().is_server_error() {
            return Err(Error::Host("identity_service_unavailable".into()));
        }
        if !response.status().is_success() {
            return Err(Error::Denied("account authentication denied".into()));
        }
        Ok(())
    }
}

/// Introspect then ProductAcl. SSO success alone never authorizes a project.
pub async fn authorize_token(
    client: &AccountsClient,
    token: &str,
    expected_tenant: Option<Uuid>,
    project: Uuid,
    device: Option<Uuid>,
    acl: &dyn ProductAcl,
) -> Result<AuthorizedScope> {
    let identity = client.introspect(token, expected_tenant).await?;
    authorize(
        &identity,
        client.issuer(),
        expected_tenant,
        project,
        device,
        acl,
    )
    .await
}

/// Explicit membership only. Email equality or organization role is not enough.
#[derive(Default)]
pub struct MembershipAcl {
    members: std::collections::BTreeMap<(Uuid, Uuid), LocalAuthorization>,
    tenant_versions: std::collections::BTreeMap<Uuid, (i64, i64, i64, i64)>,
}

impl MembershipAcl {
    pub fn grant(
        &mut self,
        subject: Uuid,
        authz: LocalAuthorization,
        tenant: Option<&TenantGrant>,
    ) -> Result<()> {
        if subject.is_nil() || authz.project.is_nil() {
            return Err(Error::Invalid("membership grant".into()));
        }
        if let Some(t) = tenant {
            self.tenant_versions.insert(
                t.tenant_id,
                (
                    t.organization_version,
                    t.member_version,
                    t.tenant_version,
                    t.grant_version,
                ),
            );
        }
        self.members.insert((subject, authz.project), authz);
        Ok(())
    }

    pub fn revoke(&mut self, subject: Uuid, project: Uuid) {
        self.members.remove(&(subject, project));
    }
}

#[async_trait]
impl ProductAcl for MembershipAcl {
    async fn authorize_project(
        &self,
        identity: &Identity,
        project: Uuid,
    ) -> Result<LocalAuthorization> {
        if !identity.active {
            return Err(Error::Denied("inactive identity".into()));
        }
        if let Some(tenant) = &identity.tenant {
            match self.tenant_versions.get(&tenant.tenant_id) {
                Some(expected)
                    if *expected
                        == (
                            tenant.organization_version,
                            tenant.member_version,
                            tenant.tenant_version,
                            tenant.grant_version,
                        ) => {}
                _ => {
                    return Err(Error::Denied(
                        "tenant grant version changed or unknown".into(),
                    ))
                }
            }
        }
        self.members
            .get(&(identity.sub, project))
            .cloned()
            .ok_or_else(|| Error::Denied("not a project member".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn personal() -> Identity {
        Identity {
            active: true,
            iss: "https://accounts.818cloud.com".into(),
            sub: Uuid::new_v4(),
            aud: "anycode".into(),
            context: ContextKind::Personal,
            tenant: None,
            scope: "identity".into(),
        }
    }
    #[test]
    fn personal_login_does_not_imply_tenant_access() {
        let p = personal();
        assert!(p.validate(&p.iss, None).is_ok());
        assert!(p.validate(&p.iss, Some(Uuid::new_v4())).is_err());
    }
    #[test]
    fn product_and_issuer_are_exact() {
        let mut p = personal();
        p.aud = "eos".into();
        assert!(p.validate(&p.iss, None).is_err());
        let p = personal();
        assert!(p.validate("https://evil.test", None).is_err());
    }
    #[test]
    fn rejects_http_remote_or_credential_urls() {
        for url in [
            "http://remote.test",
            "https://x@y.test",
            "https://a.test/path",
            "https://a.test/?token=secret",
        ] {
            assert!(AccountsClient::new(url, "x".repeat(32), true).is_err());
        }
    }
    #[test]
    fn inactive_is_never_authenticated() {
        let mut p = personal();
        p.active = false;
        assert!(p.validate(&p.iss, None).is_err());
    }

    #[tokio::test]
    async fn membership_is_explicit_and_versioned() {
        let p = personal();
        let project = Uuid::new_v4();
        let mut acl = MembershipAcl::default();
        assert!(acl.authorize_project(&p, project).await.is_err());
        acl.grant(
            p.sub,
            LocalAuthorization {
                project,
                capabilities: Capabilities::new(["fs.read".into()]).unwrap(),
            },
            None,
        )
        .unwrap();
        assert_eq!(
            acl.authorize_project(&p, project).await.unwrap().project,
            project
        );
        assert!(acl.authorize_project(&p, Uuid::new_v4()).await.is_err());
        acl.revoke(p.sub, project);
        assert!(acl.authorize_project(&p, project).await.is_err());
    }

    #[tokio::test]
    async fn stale_enterprise_grant_version_is_denied() {
        let tenant_id = Uuid::new_v4();
        let grant = TenantGrant {
            organization_id: Uuid::new_v4(),
            tenant_id,
            external_tenant_id: "ext-1".into(),
            role: Role::Editor,
            organization_version: 1,
            member_version: 1,
            tenant_version: 1,
            grant_version: 1,
        };
        let identity = Identity {
            active: true,
            iss: "https://accounts.818cloud.com".into(),
            sub: Uuid::new_v4(),
            aud: "anycode".into(),
            context: ContextKind::Enterprise,
            tenant: Some(grant.clone()),
            scope: "identity".into(),
        };
        let project = Uuid::new_v4();
        let mut acl = MembershipAcl::default();
        acl.grant(
            identity.sub,
            LocalAuthorization {
                project,
                capabilities: Capabilities::new(["fs.read".into()]).unwrap(),
            },
            Some(&grant),
        )
        .unwrap();
        assert!(acl.authorize_project(&identity, project).await.is_ok());
        let mut stale = identity.clone();
        if let Some(t) = stale.tenant.as_mut() {
            t.grant_version = 2;
        }
        assert!(acl.authorize_project(&stale, project).await.is_err());
    }

    #[tokio::test]
    async fn introspect_then_acl_uses_the_real_http_endpoint() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let subject = Uuid::new_v4();
        let issuer = format!("http://127.0.0.1:{port}");
        let body = serde_json::json!({
            "active": true,
            "iss": issuer,
            "sub": subject,
            "aud": "anycode",
            "context": "personal",
            "tenant": null,
            "scope": "identity"
        })
        .to_string();
        std::thread::spawn(move || {
            for _ in 0..2 {
                let Ok((mut stream, _)) = listener.accept() else {
                    break;
                };
                let mut req = [0u8; 4096];
                let _ = std::io::Read::read(&mut stream, &mut req);
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = std::io::Write::write_all(&mut stream, response.as_bytes());
            }
        });
        let client = AccountsClient::new(&format!("{issuer}/"), "s".repeat(32), true).unwrap();
        let token = "A".repeat(43);
        let project = Uuid::new_v4();
        let mut acl = MembershipAcl::default();
        acl.grant(
            subject,
            LocalAuthorization {
                project,
                capabilities: Capabilities::new(["fs.read".into()]).unwrap(),
            },
            None,
        )
        .unwrap();
        let authorized = authorize_token(&client, &token, None, project, None, &acl)
            .await
            .unwrap();
        assert_eq!(authorized.scope.subject, subject);
        assert_eq!(authorized.scope.project, project);
        assert!(
            authorize_token(&client, &token, None, Uuid::new_v4(), None, &acl)
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn revoke_then_introspect_is_inactive() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let issuer = format!("http://127.0.0.1:{port}");
        let subject = Uuid::new_v4();
        let active = serde_json::json!({
            "active": true,
            "iss": issuer,
            "sub": subject,
            "aud": "anycode",
            "context": "personal",
            "tenant": null,
            "scope": "identity"
        })
        .to_string();
        let inactive = serde_json::json!({"active":false}).to_string();
        let revoked = serde_json::json!({"ok":true}).to_string();
        std::thread::spawn(move || {
            let mut seen_revoke = false;
            for _ in 0..4 {
                let Ok((mut stream, _)) = listener.accept() else {
                    break;
                };
                let mut req = [0u8; 4096];
                let n = std::io::Read::read(&mut stream, &mut req).unwrap_or(0);
                let head = String::from_utf8_lossy(&req[..n]);
                let body = if head.contains("/api/v2/sso/revoke") {
                    seen_revoke = true;
                    revoked.clone()
                } else if seen_revoke {
                    inactive.clone()
                } else {
                    active.clone()
                };
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = std::io::Write::write_all(&mut stream, response.as_bytes());
            }
        });
        let client = AccountsClient::new(&format!("{issuer}/"), "s".repeat(32), true).unwrap();
        let token = "B".repeat(43);
        let identity = client.introspect(&token, None).await.unwrap();
        assert!(identity.active);
        client.revoke(&token).await.unwrap();
        assert!(client.introspect(&token, None).await.is_err());
    }

    fn redis_cmd(port: u16, db: i64, args: &[&str]) -> std::io::Result<String> {
        use std::io::{Read, Write};
        let mut stream = std::net::TcpStream::connect(("127.0.0.1", port))?;
        stream.set_read_timeout(Some(std::time::Duration::from_secs(2)))?;
        stream.set_write_timeout(Some(std::time::Duration::from_secs(2)))?;
        let mut buf = String::new();
        let select = ["SELECT", &db.to_string()];
        buf.push_str(&format!("*{}\r\n", select.len()));
        for a in select {
            buf.push_str(&format!("${}\r\n{a}\r\n", a.len()));
        }
        buf.push_str(&format!("*{}\r\n", args.len()));
        for a in args {
            buf.push_str(&format!("${}\r\n{a}\r\n", a.len()));
        }
        stream.write_all(buf.as_bytes())?;
        let mut out = vec![0u8; 8192];
        let n = stream.read(&mut out)?;
        Ok(String::from_utf8_lossy(&out[..n]).into_owned())
    }

    fn token_digest(token: &str) -> String {
        use sha2::{Digest, Sha256};
        Sha256::digest(token.as_bytes())
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }

    #[tokio::test]
    #[ignore = "real Redis at 127.0.0.1:6382 (818cloud compose); run with --ignored when that instance is up"]
    async fn redis_lx_v2_token_revoke_then_introspect_inactive() {
        let redis_port: u16 = 6382;
        let token = "C".repeat(43);
        let subject = Uuid::new_v4();
        let key = format!("lx:v2:token:{}", token_digest(&token));
        let binding = serde_json::json!({
            "client_id": "anycode",
            "redirect_uri": "http://127.0.0.1:9/cb",
            "challenge": token,
            "user_id": subject,
            "session": "sess-harness",
            "access": null
        })
        .to_string();
        redis_cmd(redis_port, 2, &["SET", &key, &binding, "EX", "60"])
            .expect("Redis 127.0.0.1:6382 must be reachable; do not treat this as passed");
        let get = redis_cmd(redis_port, 2, &["GET", &key]).expect("GET token");
        assert!(get.contains("anycode"), "token must land in Redis: {get}");

        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let issuer = format!("http://127.0.0.1:{port}");
        let redis_key = key.clone();
        let issuer_for_server = issuer.clone();
        std::thread::spawn(move || {
            for _ in 0..4 {
                let Ok((mut stream, _)) = listener.accept() else {
                    break;
                };
                let mut req = [0u8; 4096];
                let n = std::io::Read::read(&mut stream, &mut req).unwrap_or(0);
                let head = String::from_utf8_lossy(&req[..n]);
                let body = if head.contains("/api/v2/sso/revoke") {
                    let _ = redis_cmd(redis_port, 2, &["DEL", &redis_key]);
                    serde_json::json!({"ok":true}).to_string()
                } else {
                    match redis_cmd(redis_port, 2, &["GET", &redis_key]) {
                        Ok(raw) if raw.contains("anycode") => serde_json::json!({
                            "active": true,
                            "iss": issuer_for_server,
                            "sub": subject,
                            "aud": "anycode",
                            "context": "personal",
                            "tenant": null,
                            "scope": "identity"
                        })
                        .to_string(),
                        _ => serde_json::json!({"active":false}).to_string(),
                    }
                };
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = std::io::Write::write_all(&mut stream, response.as_bytes());
            }
        });

        let client = AccountsClient::new(&format!("{issuer}/"), "s".repeat(32), true).unwrap();
        let identity = client
            .introspect(&token, None)
            .await
            .expect("token present in Redis must introspect active");
        assert_eq!(identity.sub, subject);
        client.revoke(&token).await.unwrap();
        assert!(
            client.introspect(&token, None).await.is_err(),
            "DEL of lx:v2:token must make introspect inactive"
        );
        let after = redis_cmd(redis_port, 2, &["GET", &key]).expect("GET after revoke");
        assert!(
            after.contains("$-1") || after.contains("*") && !after.contains("anycode"),
            "Redis key must be gone after revoke: {after}"
        );
    }

    #[tokio::test]
    #[ignore = "live 818cloud SSO; export ANYCODE_HARNESS_SSO_ISSUER, PRODUCT_SSO_CLIENT_SECRET, ANYCODE_HARNESS_SSO_TOKEN"]
    async fn live_818cloud_introspect_is_not_loopback_mocked() {
        let issuer = std::env::var("ANYCODE_HARNESS_SSO_ISSUER").expect("issuer");
        let secret = std::env::var("PRODUCT_SSO_CLIENT_SECRET").expect("secret");
        let token = std::env::var("ANYCODE_HARNESS_SSO_TOKEN").expect("token");
        let loopback =
            issuer.starts_with("http://127.0.0.1") || issuer.starts_with("http://localhost");
        let client = AccountsClient::new(&issuer, secret, loopback).expect("accounts client");
        let identity = client
            .introspect(&token, None)
            .await
            .expect("live introspect");
        assert!(identity.active);
        assert_eq!(identity.aud, "anycode");
        assert_eq!(identity.iss, client.issuer());
    }
}
