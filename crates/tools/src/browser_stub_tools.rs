//! Stub browser tools when `tools-browser` is disabled (keeps DEFAULT_TOOL_IDS validation passing).

use anycode_core::prelude::*;
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

use crate::services::ToolServices;

macro_rules! browser_stub_tool {
    ($struct_name:ident, $id:literal, $desc:literal, $sensitive:expr) => {
        pub struct $struct_name {
            security_policy: SecurityPolicy,
        }

        impl Default for $struct_name {
            fn default() -> Self {
                Self {
                    security_policy: if $sensitive {
                        SecurityPolicy::sensitive_mutation()
                    } else {
                        SecurityPolicy::default()
                    },
                }
            }
        }

        #[async_trait]
        impl Tool for $struct_name {
            fn name(&self) -> &str {
                $id
            }

            fn description(&self) -> &str {
                $desc
            }

            fn schema(&self) -> serde_json::Value {
                json!({ "type": "object", "properties": {} })
            }

            fn permission_mode(&self) -> PermissionMode {
                PermissionMode::Default
            }

            fn security_policy(&self) -> Option<&SecurityPolicy> {
                if $sensitive {
                    Some(&self.security_policy)
                } else {
                    None
                }
            }

            async fn execute(&self, _input: ToolInput) -> Result<ToolOutput, CoreError> {
                Err(CoreError::Other(anyhow::anyhow!("{}", browser_stub_message())))
            }
        }
    };
}

fn browser_stub_message() -> String {
    "Native browser tools require building with --features tools-browser (and Chromium via ANYCODE_CHROMIUM_PATH or desktop bundle).".into()
}

browser_stub_tool!(
    BrowserTabsStub,
    "BrowserTabs",
    "List, create, close, or select browser tabs.",
    false
);
browser_stub_tool!(
    BrowserNavigateStub,
    "BrowserNavigate",
    "Navigate the shared browser to a URL.",
    true
);
browser_stub_tool!(
    BrowserSnapshotStub,
    "BrowserSnapshot",
    "Accessibility snapshot of the active tab.",
    false
);
browser_stub_tool!(
    BrowserClickStub,
    "BrowserClick",
    "Click an element by snapshot ref.",
    true
);
browser_stub_tool!(
    BrowserTypeStub,
    "BrowserType",
    "Type text into an element by snapshot ref.",
    true
);
browser_stub_tool!(
    BrowserPressKeyStub,
    "BrowserPressKey",
    "Press a key in the browser.",
    true
);
browser_stub_tool!(
    BrowserScrollStub,
    "BrowserScroll",
    "Scroll the browser viewport.",
    true
);
browser_stub_tool!(
    BrowserScreenshotStub,
    "BrowserScreenshot",
    "Capture a PNG screenshot of the active tab.",
    false
);
browser_stub_tool!(
    BrowserCdpStub,
    "BrowserCdp",
    "Run a whitelisted CDP method.",
    true
);

pub fn register_browser_stub_tools(
    tools: &mut std::collections::HashMap<ToolName, Box<dyn Tool>>,
    _services: Arc<ToolServices>,
) {
    macro_rules! ins {
        ($t:ty) => {
            let b: Box<dyn Tool> = Box::new(<$t>::default());
            tools.insert(b.name().to_string(), b);
        };
    }
    ins!(BrowserTabsStub);
    ins!(BrowserNavigateStub);
    ins!(BrowserSnapshotStub);
    ins!(BrowserClickStub);
    ins!(BrowserTypeStub);
    ins!(BrowserPressKeyStub);
    ins!(BrowserScrollStub);
    ins!(BrowserScreenshotStub);
    ins!(BrowserCdpStub);
}
