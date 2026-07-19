//! Native CDP browser tools (Cursor-aligned tool surface).

use crate::services::ToolServices;
use anycode_browser::BrowserService;
use anycode_core::prelude::*;
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Instant;

struct BrowserCtx {
    services: Arc<ToolServices>,
    policy: SecurityPolicy,
}

impl BrowserCtx {
    fn new(services: Arc<ToolServices>) -> Self {
        Self {
            services,
            policy: SecurityPolicy::sensitive_mutation(),
        }
    }

    fn browser(&self) -> Result<Arc<BrowserService>, CoreError> {
        self.services.browser_service().ok_or_else(|| {
            CoreError::Other(anyhow::anyhow!(
                "browser service not available — rebuild with --features tools-browser and enable built-in browser in Settings → Notifications"
            ))
        })
    }

    async fn session_id(
        &self,
        session_id: Option<&str>,
        project_id: Option<&str>,
    ) -> Result<String, CoreError> {
        let browser = self.browser()?;
        if let Some(id) = BrowserService::resolve_agent_session_id(session_id) {
            let pid = project_id.unwrap_or("default");
            browser
                .create_session(pid, None, Some(&id))
                .await
                .map_err(|e| CoreError::Other(anyhow::anyhow!("{e}")))?;
            return Ok(id);
        }
        Err(CoreError::Other(anyhow::anyhow!(
            "browser session_id required (ANYCODE_BROWSER_SESSION_ID or session_id param)"
        )))
    }
}

fn tool_ok(result: Value, t0: Instant) -> ToolOutput {
    ToolOutput {
        result,
        error: None,
        duration_ms: t0.elapsed().as_millis() as u64,
    }
}

fn map_browser_err<T>(r: Result<T, anycode_browser::BrowserError>) -> Result<T, CoreError> {
    r.map_err(|e| CoreError::Other(anyhow::anyhow!("{e}")))
}

#[derive(Deserialize)]
struct SessionFields {
    #[serde(default)]
    session_id: Option<String>,
    #[serde(default)]
    project_id: Option<String>,
}

pub struct BrowserTabsTool {
    ctx: BrowserCtx,
}

impl BrowserTabsTool {
    pub fn new(services: Arc<ToolServices>) -> Self {
        Self {
            ctx: BrowserCtx::new(services),
        }
    }
}

#[derive(Deserialize)]
struct TabsInput {
    action: String,
    #[serde(default)]
    tab_id: Option<String>,
    #[serde(flatten)]
    session: SessionFields,
}

#[async_trait]
impl Tool for BrowserTabsTool {
    fn name(&self) -> &str {
        "BrowserTabs"
    }
    fn description(&self) -> &str {
        "List, create, close, or select browser tabs in the shared workbench session. Prefer BrowserSnapshot to inspect page content."
    }
    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": { "type": "string", "enum": ["list", "new", "close", "select"] },
                "tab_id": { "type": "string" },
                "session_id": { "type": "string" },
                "project_id": { "type": "string" }
            },
            "required": ["action"]
        })
    }
    fn permission_mode(&self) -> PermissionMode {
        PermissionMode::Default
    }
    fn security_policy(&self) -> Option<&SecurityPolicy> {
        Some(&self.ctx.policy)
    }
    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, CoreError> {
        let t0 = Instant::now();
        let args: TabsInput = serde_json::from_value(input.input)?;
        let browser = self.ctx.browser()?;
        let sid = self
            .ctx
            .session_id(
                args.session.session_id.as_deref(),
                args.session.project_id.as_deref(),
            )
            .await?;
        let result = match args.action.as_str() {
            "list" => json!({ "tabs": map_browser_err(browser.list_tabs(&sid).await)? }),
            "new" => json!({ "tab_id": map_browser_err(browser.new_tab(&sid).await)? }),
            "close" => {
                let tab_id = args
                    .tab_id
                    .ok_or_else(|| CoreError::Other(anyhow::anyhow!("tab_id required")))?;
                map_browser_err(browser.close_tab(&sid, &tab_id).await)?;
                json!({ "ok": true })
            }
            "select" => {
                let tab_id = args
                    .tab_id
                    .ok_or_else(|| CoreError::Other(anyhow::anyhow!("tab_id required")))?;
                map_browser_err(browser.select_tab(&sid, &tab_id).await)?;
                json!({ "ok": true })
            }
            other => return Err(CoreError::Other(anyhow::anyhow!("unknown action: {other}"))),
        };
        Ok(tool_ok(result, t0))
    }
}

pub struct BrowserNavigateTool {
    ctx: BrowserCtx,
}
impl BrowserNavigateTool {
    pub fn new(services: Arc<ToolServices>) -> Self {
        Self {
            ctx: BrowserCtx::new(services),
        }
    }
}
#[derive(Deserialize)]
struct NavigateInput {
    url: String,
    #[serde(flatten)]
    session: SessionFields,
}
#[async_trait]
impl Tool for BrowserNavigateTool {
    fn name(&self) -> &str {
        "BrowserNavigate"
    }
    fn description(&self) -> &str {
        "Navigate the shared browser tab to a URL (http/https only). Follow with BrowserSnapshot — not BrowserScreenshot."
    }
    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "url": { "type": "string" },
                "session_id": { "type": "string" },
                "project_id": { "type": "string" }
            },
            "required": ["url"]
        })
    }
    fn permission_mode(&self) -> PermissionMode {
        PermissionMode::Default
    }
    fn security_policy(&self) -> Option<&SecurityPolicy> {
        Some(&self.ctx.policy)
    }
    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, CoreError> {
        let t0 = Instant::now();
        let args: NavigateInput = serde_json::from_value(input.input)?;
        let browser = self.ctx.browser()?;
        let sid = self
            .ctx
            .session_id(
                args.session.session_id.as_deref(),
                args.session.project_id.as_deref(),
            )
            .await?;
        let state = map_browser_err(browser.navigate(&sid, &args.url).await)?;
        Ok(tool_ok(
            serde_json::to_value(state).unwrap_or(json!({})),
            t0,
        ))
    }
}

pub struct BrowserSnapshotTool {
    ctx: BrowserCtx,
}
impl BrowserSnapshotTool {
    pub fn new(services: Arc<ToolServices>) -> Self {
        Self {
            ctx: BrowserCtx::new(services),
        }
    }
}
#[derive(Deserialize)]
struct SnapshotInput {
    #[serde(default, rename = "ref")]
    element_ref: Option<String>,
    #[serde(flatten)]
    session: SessionFields,
}
#[async_trait]
impl Tool for BrowserSnapshotTool {
    fn name(&self) -> &str {
        "BrowserSnapshot"
    }
    fn description(&self) -> &str {
        "Primary way to see the page: YAML accessibility tree with ref=eN handles for BrowserClick/BrowserType. Optional ref snapshots a subtree only (saves tokens). Do not use BrowserScreenshot for routine inspection."
    }
    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "ref": { "type": "string", "description": "Optional subtree root ref from a prior snapshot (region snapshot)." },
                "session_id": { "type": "string" },
                "project_id": { "type": "string" }
            }
        })
    }
    fn permission_mode(&self) -> PermissionMode {
        PermissionMode::Default
    }
    fn security_policy(&self) -> Option<&SecurityPolicy> {
        Some(&self.ctx.policy)
    }
    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, CoreError> {
        let t0 = Instant::now();
        let args: SnapshotInput = serde_json::from_value(input.input)?;
        let browser = self.ctx.browser()?;
        let sid = self
            .ctx
            .session_id(
                args.session.session_id.as_deref(),
                args.session.project_id.as_deref(),
            )
            .await?;
        let snap = map_browser_err(browser.snapshot(&sid, args.element_ref.as_deref()).await)?;
        Ok(tool_ok(
            json!({ "url": snap.url, "title": snap.title, "snapshot": snap.yaml }),
            t0,
        ))
    }
}

pub struct BrowserClickTool {
    ctx: BrowserCtx,
}
impl BrowserClickTool {
    pub fn new(services: Arc<ToolServices>) -> Self {
        Self {
            ctx: BrowserCtx::new(services),
        }
    }
}
#[derive(Deserialize)]
struct RefInput {
    #[serde(rename = "ref")]
    element_ref: String,
    #[serde(flatten)]
    session: SessionFields,
}
#[async_trait]
impl Tool for BrowserClickTool {
    fn name(&self) -> &str {
        "BrowserClick"
    }
    fn description(&self) -> &str {
        "Click an element by snapshot ref (from BrowserSnapshot)."
    }
    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "ref": { "type": "string" },
                "session_id": { "type": "string" },
                "project_id": { "type": "string" }
            },
            "required": ["ref"]
        })
    }
    fn permission_mode(&self) -> PermissionMode {
        PermissionMode::Default
    }
    fn security_policy(&self) -> Option<&SecurityPolicy> {
        Some(&self.ctx.policy)
    }
    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, CoreError> {
        let t0 = Instant::now();
        let args: RefInput = serde_json::from_value(input.input)?;
        let browser = self.ctx.browser()?;
        let sid = self
            .ctx
            .session_id(
                args.session.session_id.as_deref(),
                args.session.project_id.as_deref(),
            )
            .await?;
        map_browser_err(browser.click(&sid, &args.element_ref).await)?;
        Ok(tool_ok(json!({ "ok": true }), t0))
    }
}

pub struct BrowserTypeTool {
    ctx: BrowserCtx,
}
impl BrowserTypeTool {
    pub fn new(services: Arc<ToolServices>) -> Self {
        Self {
            ctx: BrowserCtx::new(services),
        }
    }
}
#[derive(Deserialize)]
struct TypeInput {
    #[serde(rename = "ref")]
    element_ref: String,
    text: String,
    #[serde(default)]
    submit: bool,
    #[serde(flatten)]
    session: SessionFields,
}
#[async_trait]
impl Tool for BrowserTypeTool {
    fn name(&self) -> &str {
        "BrowserType"
    }
    fn description(&self) -> &str {
        "Type text into an element by snapshot ref."
    }
    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "ref": { "type": "string" },
                "text": { "type": "string" },
                "submit": { "type": "boolean" },
                "session_id": { "type": "string" },
                "project_id": { "type": "string" }
            },
            "required": ["ref", "text"]
        })
    }
    fn permission_mode(&self) -> PermissionMode {
        PermissionMode::Default
    }
    fn security_policy(&self) -> Option<&SecurityPolicy> {
        Some(&self.ctx.policy)
    }
    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, CoreError> {
        let t0 = Instant::now();
        let args: TypeInput = serde_json::from_value(input.input)?;
        let browser = self.ctx.browser()?;
        let sid = self
            .ctx
            .session_id(
                args.session.session_id.as_deref(),
                args.session.project_id.as_deref(),
            )
            .await?;
        map_browser_err(
            browser
                .type_text(&sid, &args.element_ref, &args.text, args.submit)
                .await,
        )?;
        Ok(tool_ok(json!({ "ok": true }), t0))
    }
}

pub struct BrowserPressKeyTool {
    ctx: BrowserCtx,
}
impl BrowserPressKeyTool {
    pub fn new(services: Arc<ToolServices>) -> Self {
        Self {
            ctx: BrowserCtx::new(services),
        }
    }
}
#[derive(Deserialize)]
struct KeyInput {
    key: String,
    #[serde(flatten)]
    session: SessionFields,
}
#[async_trait]
impl Tool for BrowserPressKeyTool {
    fn name(&self) -> &str {
        "BrowserPressKey"
    }
    fn description(&self) -> &str {
        "Press a keyboard key in the active browser tab."
    }
    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "key": { "type": "string" },
                "session_id": { "type": "string" },
                "project_id": { "type": "string" }
            },
            "required": ["key"]
        })
    }
    fn permission_mode(&self) -> PermissionMode {
        PermissionMode::Default
    }
    fn security_policy(&self) -> Option<&SecurityPolicy> {
        Some(&self.ctx.policy)
    }
    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, CoreError> {
        let t0 = Instant::now();
        let args: KeyInput = serde_json::from_value(input.input)?;
        let browser = self.ctx.browser()?;
        let sid = self
            .ctx
            .session_id(
                args.session.session_id.as_deref(),
                args.session.project_id.as_deref(),
            )
            .await?;
        map_browser_err(browser.press_key(&sid, &args.key).await)?;
        Ok(tool_ok(json!({ "ok": true }), t0))
    }
}

pub struct BrowserScrollTool {
    ctx: BrowserCtx,
}
impl BrowserScrollTool {
    pub fn new(services: Arc<ToolServices>) -> Self {
        Self {
            ctx: BrowserCtx::new(services),
        }
    }
}
#[derive(Deserialize)]
struct ScrollInput {
    direction: String,
    #[serde(default = "default_scroll_amount")]
    amount: i32,
    #[serde(flatten)]
    session: SessionFields,
}
fn default_scroll_amount() -> i32 {
    400
}
#[async_trait]
impl Tool for BrowserScrollTool {
    fn name(&self) -> &str {
        "BrowserScroll"
    }
    fn description(&self) -> &str {
        "Scroll the active browser tab (up/down/left/right)."
    }
    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "direction": { "type": "string", "enum": ["up", "down", "left", "right"] },
                "amount": { "type": "integer" },
                "session_id": { "type": "string" },
                "project_id": { "type": "string" }
            },
            "required": ["direction"]
        })
    }
    fn permission_mode(&self) -> PermissionMode {
        PermissionMode::Default
    }
    fn security_policy(&self) -> Option<&SecurityPolicy> {
        Some(&self.ctx.policy)
    }
    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, CoreError> {
        let t0 = Instant::now();
        let args: ScrollInput = serde_json::from_value(input.input)?;
        let browser = self.ctx.browser()?;
        let sid = self
            .ctx
            .session_id(
                args.session.session_id.as_deref(),
                args.session.project_id.as_deref(),
            )
            .await?;
        map_browser_err(browser.scroll(&sid, &args.direction, args.amount).await)?;
        Ok(tool_ok(json!({ "ok": true }), t0))
    }
}

pub struct BrowserScreenshotTool {
    ctx: BrowserCtx,
}
impl BrowserScreenshotTool {
    pub fn new(services: Arc<ToolServices>) -> Self {
        Self {
            ctx: BrowserCtx::new(services),
        }
    }
}
#[async_trait]
impl Tool for BrowserScreenshotTool {
    fn name(&self) -> &str {
        "BrowserScreenshot"
    }
    fn description(&self) -> &str {
        "Capture PNG screenshot (base64). Expensive — requires approval. Use only when BrowserSnapshot is insufficient (canvas, charts, visual layout verification)."
    }
    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "session_id": { "type": "string" },
                "project_id": { "type": "string" }
            }
        })
    }
    fn permission_mode(&self) -> PermissionMode {
        PermissionMode::Default
    }
    fn security_policy(&self) -> Option<&SecurityPolicy> {
        Some(&self.ctx.policy)
    }
    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, CoreError> {
        let t0 = Instant::now();
        let session: SessionFields = serde_json::from_value(input.input)?;
        let browser = self.ctx.browser()?;
        let sid = self
            .ctx
            .session_id(session.session_id.as_deref(), session.project_id.as_deref())
            .await?;
        let shot = map_browser_err(browser.screenshot(&sid).await)?;
        Ok(tool_ok(serde_json::to_value(shot).unwrap_or(json!({})), t0))
    }
}

pub struct BrowserCdpTool {
    ctx: BrowserCtx,
}
impl BrowserCdpTool {
    pub fn new(services: Arc<ToolServices>) -> Self {
        Self {
            ctx: BrowserCtx::new(services),
        }
    }
}
#[derive(Deserialize)]
struct CdpInput {
    method: String,
    #[serde(default)]
    params: Value,
    #[serde(flatten)]
    session: SessionFields,
}
#[async_trait]
impl Tool for BrowserCdpTool {
    fn name(&self) -> &str {
        "BrowserCdp"
    }
    fn description(&self) -> &str {
        "Invoke a whitelisted CDP method on the active tab."
    }
    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "method": { "type": "string" },
                "params": { "type": "object" },
                "session_id": { "type": "string" },
                "project_id": { "type": "string" }
            },
            "required": ["method"]
        })
    }
    fn permission_mode(&self) -> PermissionMode {
        PermissionMode::Default
    }
    fn security_policy(&self) -> Option<&SecurityPolicy> {
        Some(&self.ctx.policy)
    }
    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, CoreError> {
        let t0 = Instant::now();
        let args: CdpInput = serde_json::from_value(input.input)?;
        let browser = self.ctx.browser()?;
        let sid = self
            .ctx
            .session_id(
                args.session.session_id.as_deref(),
                args.session.project_id.as_deref(),
            )
            .await?;
        let out = map_browser_err(browser.cdp(&sid, &args.method, args.params).await)?;
        Ok(tool_ok(out, t0))
    }
}

pub fn register_browser_tools(
    tools: &mut std::collections::HashMap<ToolName, Box<dyn Tool>>,
    services: Arc<ToolServices>,
) {
    tools.insert(
        "BrowserTabs".to_string(),
        Box::new(BrowserTabsTool::new(services.clone())),
    );
    tools.insert(
        "BrowserNavigate".to_string(),
        Box::new(BrowserNavigateTool::new(services.clone())),
    );
    tools.insert(
        "BrowserSnapshot".to_string(),
        Box::new(BrowserSnapshotTool::new(services.clone())),
    );
    tools.insert(
        "BrowserClick".to_string(),
        Box::new(BrowserClickTool::new(services.clone())),
    );
    tools.insert(
        "BrowserType".to_string(),
        Box::new(BrowserTypeTool::new(services.clone())),
    );
    tools.insert(
        "BrowserPressKey".to_string(),
        Box::new(BrowserPressKeyTool::new(services.clone())),
    );
    tools.insert(
        "BrowserScroll".to_string(),
        Box::new(BrowserScrollTool::new(services.clone())),
    );
    tools.insert(
        "BrowserScreenshot".to_string(),
        Box::new(BrowserScreenshotTool::new(services.clone())),
    );
    tools.insert(
        "BrowserCdp".to_string(),
        Box::new(BrowserCdpTool::new(services)),
    );
}

pub const BROWSER_TOOL_IDS: &[&str] = &[
    "BrowserTabs",
    "BrowserNavigate",
    "BrowserSnapshot",
    "BrowserClick",
    "BrowserType",
    "BrowserPressKey",
    "BrowserScroll",
    "BrowserScreenshot",
    "BrowserCdp",
];
