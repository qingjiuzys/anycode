use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthUser {
    pub id: String,
    pub email: String,
    pub display_name: String,
    pub role: String,
    pub organization_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationSummary {
    pub id: String,
    pub name: String,
    pub plan_tier: String,
    pub sso_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionView {
    pub plan: String,
    pub status: String,
    pub billing_cycle: String,
    pub period_start: String,
    pub period_end: String,
    pub days_remaining: i64,
    pub payment_method_bound: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_provider: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntitlementsView {
    pub token_limit: i64,
    pub api_key_limit: i32,
    pub seat_limit: i32,
    pub seat_used: i32,
    pub tokens_used: i64,
    pub hosted_models_enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calls_limit_per_window: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calls_used_in_window: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calls_remaining: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quota_window_hours: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quota_resets_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkedDeviceView {
    pub id: String,
    pub device_name: String,
    pub platform: String,
    pub last_seen_at: String,
    pub created_at: String,
    pub revoked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudModelView {
    pub id: String,
    pub provider_id: String,
    pub display_name: String,
    pub context_window: i32,
    pub price_per_1m_input_cny: f64,
    pub price_per_1m_output_cny: f64,
    pub currency: String,
    pub min_plan: String,
    pub available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageSummaryView {
    pub tokens_used: i64,
    pub token_limit: i64,
    pub by_model: Vec<UsageByModelView>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageByModelView {
    pub model_id: String,
    pub total_tokens: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillingContactView {
    pub email: String,
    pub company_name: String,
    pub tax_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceView {
    pub id: String,
    pub number: String,
    pub period_start: String,
    pub period_end: String,
    pub amount_fen: i32,
    pub currency: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrgMemberView {
    pub id: String,
    pub name: String,
    pub email: String,
    pub role: String,
    pub status: String,
    pub last_active: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudApiKeyView {
    pub id: String,
    pub name: String,
    pub prefix: String,
    pub scopes: Vec<String>,
    pub created_at: String,
    pub expires_at: Option<String>,
    pub last_used_at: Option<String>,
    pub revoked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountBundle {
    pub user: AuthUser,
    pub organization: OrganizationSummary,
    pub subscription: SubscriptionView,
    pub entitlements: EntitlementsView,
    pub billing_contact: BillingContactView,
    pub invoices: Vec<InvoiceView>,
}
