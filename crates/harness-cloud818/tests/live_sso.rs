//! Live 818cloud SSO v2 against a real accounts process (Redis + Postgres).
//! Not a loopback stub. Default CI does not run this.
use anycode_harness_cloud818::identity::{
    authorize_token, AccountsClient, LocalAuthorization, MembershipAcl,
};
use anycode_harness_core::Capabilities;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use sha2::{Digest, Sha256};
use uuid::Uuid;

fn pkce() -> (String, String) {
    let verifier = "Vv-_".to_string() + &"a".repeat(39);
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    (verifier, challenge)
}

fn cookie_from(headers: &reqwest::header::HeaderMap) -> String {
    headers
        .get_all(reqwest::header::SET_COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .find_map(|v| {
            v.split(';')
                .next()
                .filter(|c| c.starts_with("lx_account="))
                .map(str::to_string)
        })
        .expect("lx_account cookie")
}

async fn login(http: &reqwest::Client, origin: &str) -> String {
    let res = http
        .post(format!("{origin}/api/v1/auth/otp/login"))
        .json(&serde_json::json!({"phone":"13800138000","code":"000000"}))
        .send()
        .await
        .expect("otp login");
    assert!(res.status().is_success(), "otp login {}", res.status());
    cookie_from(res.headers())
}

async fn mint_token(
    http: &reqwest::Client,
    origin: &str,
    cookie: &str,
    client_id: &str,
    secret: &str,
    redirect: &str,
    tenant_id: Option<Uuid>,
) -> String {
    let (verifier, challenge) = pkce();
    let state = "state-harness-live-01";
    let mut url = reqwest::Url::parse(&format!("{origin}/api/v2/sso/authorize")).unwrap();
    {
        let mut q = url.query_pairs_mut();
        q.append_pair("client_id", client_id);
        q.append_pair("redirect_uri", redirect);
        q.append_pair("state", state);
        q.append_pair("code_challenge", &challenge);
        q.append_pair("code_challenge_method", "S256");
        if let Some(tid) = tenant_id {
            q.append_pair("tenant_id", &tid.to_string());
        }
    }
    let res = http
        .get(url)
        .header(reqwest::header::COOKIE, cookie)
        .send()
        .await
        .expect("authorize");
    assert_eq!(
        res.status(),
        reqwest::StatusCode::SEE_OTHER,
        "authorize {}",
        res.status()
    );
    let loc = res
        .headers()
        .get(reqwest::header::LOCATION)
        .and_then(|v| v.to_str().ok())
        .expect("redirect");
    let loc_url = reqwest::Url::parse(loc).expect("callback url");
    let code = loc_url
        .query_pairs()
        .find(|(k, _)| k == "code")
        .map(|(_, v)| v.into_owned())
        .expect("code");
    let token_res = http
        .post(format!("{origin}/api/v2/sso/token"))
        .basic_auth(client_id, Some(secret))
        .json(&serde_json::json!({
            "code": code,
            "redirect_uri": redirect,
            "code_verifier": verifier
        }))
        .send()
        .await
        .expect("token");
    assert!(
        token_res.status().is_success(),
        "token {}",
        token_res.status()
    );
    let body: serde_json::Value = token_res.json().await.expect("token json");
    body["access_token"]
        .as_str()
        .expect("access_token")
        .to_string()
}

#[tokio::test]
#[ignore = "live lingxi-accounts SSO v2; export ANYCODE_HARNESS_SSO_ISSUER, PRODUCT_SSO_CLIENT_SECRET, ANYCODE_HARNESS_SSO_REDIRECT; optional ANYCODE_HARNESS_SSO_OTHER_SECRET"]
async fn live_accounts_pkce_introspect_acl_revoke_and_foreign_aud() {
    let issuer = std::env::var("ANYCODE_HARNESS_SSO_ISSUER").expect("issuer");
    let secret = std::env::var("PRODUCT_SSO_CLIENT_SECRET").expect("secret");
    let redirect = std::env::var("ANYCODE_HARNESS_SSO_REDIRECT")
        .unwrap_or_else(|_| "http://127.0.0.1:18781/api/auth/hop/v2/callback".into());
    let loopback = issuer.starts_with("http://127.0.0.1") || issuer.starts_with("http://localhost");
    let origin = issuer.trim_end_matches('/').to_string();
    let http = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap();

    let cookie = login(&http, &origin).await;
    let token = mint_token(&http, &origin, &cookie, "anycode", &secret, &redirect, None).await;
    let client = AccountsClient::new(&issuer, secret.clone(), loopback).expect("client");
    let identity = client
        .introspect(&token, None)
        .await
        .expect("personal introspect");
    assert!(identity.active);
    assert_eq!(identity.aud, "anycode");
    assert_eq!(identity.iss, client.issuer());
    assert_eq!(
        identity.context,
        anycode_harness_cloud818::identity::ContextKind::Personal
    );

    let project = Uuid::new_v4();
    let mut acl = MembershipAcl::default();
    assert!(
        authorize_token(&client, &token, None, project, None, &acl)
            .await
            .is_err(),
        "SSO success without membership must not authorize"
    );
    acl.grant(
        identity.sub,
        LocalAuthorization {
            project,
            capabilities: Capabilities::new(["fs.read".into()]).unwrap(),
        },
        None,
    )
    .unwrap();
    let authorized = authorize_token(&client, &token, None, project, None, &acl)
        .await
        .expect("explicit member");
    assert_eq!(authorized.scope.project, project);
    assert!(
        authorize_token(&client, &token, None, Uuid::new_v4(), None, &acl)
            .await
            .is_err(),
        "wrong project must be denied"
    );

    if let Ok(other_secret) = std::env::var("ANYCODE_HARNESS_SSO_OTHER_SECRET") {
        let foreign = mint_token(
            &http,
            &origin,
            &cookie,
            "eos",
            &other_secret,
            &redirect,
            None,
        )
        .await;
        assert!(
            client.introspect(&foreign, None).await.is_err(),
            "other product aud token must not introspect as anycode"
        );
    }

    let logout = http
        .post(format!("{origin}/api/v1/auth/logout"))
        .header(reqwest::header::COOKIE, &cookie)
        .send()
        .await
        .expect("logout");
    assert!(logout.status().is_success(), "logout {}", logout.status());
    assert!(
        client.introspect(&token, None).await.is_err(),
        "session logout must make the bound token inactive"
    );

    let cookie = login(&http, &origin).await;
    let token = mint_token(&http, &origin, &cookie, "anycode", &secret, &redirect, None).await;
    client.introspect(&token, None).await.expect("fresh token");
    client.revoke(&token).await.expect("revoke");
    assert!(
        client.introspect(&token, None).await.is_err(),
        "revoked token must be inactive"
    );
}

#[tokio::test]
#[ignore = "live enterprise grant versions; same env as live_accounts_pkce plus ADMIN user on the issuer"]
async fn live_enterprise_grant_version_change_denies_token() {
    let issuer = std::env::var("ANYCODE_HARNESS_SSO_ISSUER").expect("issuer");
    let secret = std::env::var("PRODUCT_SSO_CLIENT_SECRET").expect("secret");
    let redirect = std::env::var("ANYCODE_HARNESS_SSO_REDIRECT")
        .unwrap_or_else(|_| "http://127.0.0.1:18781/api/auth/hop/v2/callback".into());
    let loopback = issuer.starts_with("http://127.0.0.1") || issuer.starts_with("http://localhost");
    let origin = issuer.trim_end_matches('/').to_string();
    let http = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap();
    let cookie = login(&http, &origin).await;
    let session = http
        .get(format!("{origin}/api/v2/session"))
        .header(reqwest::header::COOKIE, &cookie)
        .send()
        .await
        .expect("session");
    assert!(
        session.status().is_success(),
        "session {}",
        session.status()
    );
    let session_json: serde_json::Value = session.json().await.expect("session json");
    let csrf = session_json["csrf_token"]
        .as_str()
        .expect("csrf")
        .to_string();
    let user_id = session_json["user"]["id"].as_str().expect("user id");

    let org = http
        .post(format!("{origin}/api/v2/organizations"))
        .header(reqwest::header::COOKIE, &cookie)
        .header(reqwest::header::ORIGIN, &origin)
        .header("x-csrf-token", &csrf)
        .json(&serde_json::json!({"name":"harness-live-acl"}))
        .send()
        .await
        .expect("create org");
    assert_eq!(
        org.status(),
        reqwest::StatusCode::CREATED,
        "create org {}",
        org.status()
    );
    let org_json: serde_json::Value = org.json().await.expect("org json");
    let org_id = org_json["id"].as_str().expect("org id");

    let tenant = http
        .post(format!("{origin}/api/v2/organizations/{org_id}/tenants"))
        .header(reqwest::header::COOKIE, &cookie)
        .header(reqwest::header::ORIGIN, &origin)
        .header("x-csrf-token", &csrf)
        .json(&serde_json::json!({
            "product_id":"anycode",
            "external_id": format!("harness-{}", Uuid::new_v4())
        }))
        .send()
        .await
        .expect("bind tenant");
    assert!(
        tenant.status().is_success(),
        "bind tenant requires admin on the issuer: {}",
        tenant.status()
    );
    let tenant_json: serde_json::Value = tenant.json().await.expect("tenant json");
    let tenant_id = Uuid::parse_str(tenant_json["id"].as_str().expect("tenant id")).unwrap();

    let grant = http
        .put(format!(
            "{origin}/api/v2/tenants/{tenant_id}/grants/{user_id}"
        ))
        .header(reqwest::header::COOKIE, &cookie)
        .header(reqwest::header::ORIGIN, &origin)
        .header("x-csrf-token", &csrf)
        .json(&serde_json::json!({"role":"editor","active":true,"version":0}))
        .send()
        .await
        .expect("grant");
    assert!(grant.status().is_success(), "grant {}", grant.status());

    let client = AccountsClient::new(&issuer, secret.clone(), loopback).expect("client");
    let personal = mint_token(&http, &origin, &cookie, "anycode", &secret, &redirect, None).await;
    let personal_id = client
        .introspect(&personal, None)
        .await
        .expect("personal context");
    assert_eq!(
        personal_id.context,
        anycode_harness_cloud818::identity::ContextKind::Personal
    );
    assert!(
        client.introspect(&personal, Some(tenant_id)).await.is_err(),
        "personal token must not satisfy an enterprise tenant"
    );

    let token = mint_token(
        &http,
        &origin,
        &cookie,
        "anycode",
        &secret,
        &redirect,
        Some(tenant_id),
    )
    .await;
    let identity = client
        .introspect(&token, Some(tenant_id))
        .await
        .expect("enterprise introspect");
    assert_eq!(
        identity.context,
        anycode_harness_cloud818::identity::ContextKind::Enterprise
    );
    let user_uuid = Uuid::parse_str(user_id).expect("user uuid");
    let sql =
        format!("UPDATE platform_members SET version = version + 1 WHERE user_id = '{user_uuid}'");
    let bumped_member = std::process::Command::new("docker")
        .args([
            "exec",
            "lingxi-accounts-postgres-1",
            "psql",
            "-U",
            "lingxi",
            "-d",
            "lingxi_accounts",
            "-v",
            "ON_ERROR_STOP=1",
            "-c",
            &sql,
        ])
        .output()
        .expect("docker exec psql");
    assert!(
        bumped_member.status.success(),
        "member version bump: {}",
        String::from_utf8_lossy(&bumped_member.stderr)
    );
    assert!(
        client.introspect(&token, Some(tenant_id)).await.is_err(),
        "member version change must make the token inactive"
    );

    let token = mint_token(
        &http,
        &origin,
        &cookie,
        "anycode",
        &secret,
        &redirect,
        Some(tenant_id),
    )
    .await;
    let identity = client
        .introspect(&token, Some(tenant_id))
        .await
        .expect("enterprise after member bump");
    let grant_version = identity
        .tenant
        .as_ref()
        .map(|t| t.grant_version)
        .expect("grant version");

    let bumped = http
        .put(format!(
            "{origin}/api/v2/tenants/{tenant_id}/grants/{user_id}"
        ))
        .header(reqwest::header::COOKIE, &cookie)
        .header(reqwest::header::ORIGIN, &origin)
        .header("x-csrf-token", &csrf)
        .json(&serde_json::json!({"role":"editor","active":true,"version":grant_version}))
        .send()
        .await
        .expect("bump grant");
    assert!(
        bumped.status().is_success(),
        "bump grant {}",
        bumped.status()
    );
    assert!(
        client.introspect(&token, Some(tenant_id)).await.is_err(),
        "stale grant version must make the token inactive"
    );
}
