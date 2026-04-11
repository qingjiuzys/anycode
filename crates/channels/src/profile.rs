use anycode_core::{ChannelType, RuntimeMode};

/// 审批模式
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApprovalMode {
    /// 始终询问（TUI模式）
    AlwaysAsk,
    /// 自动允许特定工具
    AutoAllow { allowed_tools: Vec<String> },
    /// 静默模式（Channel模式：无审批）
    Silent,
}

impl ApprovalMode {
    /// 判断是否需要交互式审批
    pub fn requires_interaction(&self) -> bool {
        matches!(self, ApprovalMode::AlwaysAsk)
    }

    /// 判断是否跳过所有审批
    pub fn is_silent(&self) -> bool {
        matches!(self, ApprovalMode::Silent)
    }

    /// 判断工具是否被自动允许
    pub fn is_tool_auto_allowed(&self, tool_name: &str) -> bool {
        match self {
            ApprovalMode::Silent => true, // 静默模式允许所有工具
            ApprovalMode::AutoAllow { allowed_tools } => {
                allowed_tools.iter().any(|t| t == tool_name || t == "*")
            }
            ApprovalMode::AlwaysAsk => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelProfile {
    pub id: &'static str,
    pub default_mode: RuntimeMode,
    pub assistant_agent: &'static str,
    pub supports_native_approval: bool,
    pub supports_inline_replies: bool,
    pub approval_mode: ApprovalMode,
}

impl ChannelProfile {
    pub fn cli() -> Self {
        Self {
            id: "cli",
            default_mode: RuntimeMode::Code,
            assistant_agent: "general-purpose",
            supports_native_approval: true,
            supports_inline_replies: false,
            approval_mode: ApprovalMode::AlwaysAsk, // TUI模式保持现有审批
        }
    }

    pub fn ide() -> Self {
        Self {
            id: "ide",
            default_mode: RuntimeMode::Code,
            assistant_agent: "general-purpose",
            supports_native_approval: true,
            supports_inline_replies: true,
            approval_mode: ApprovalMode::AlwaysAsk, // IDE模式保持现有审批
        }
    }

    pub fn web() -> Self {
        Self {
            id: "web",
            default_mode: RuntimeMode::Channel,
            assistant_agent: "workspace-assistant",
            supports_native_approval: true,
            supports_inline_replies: true,
            approval_mode: ApprovalMode::AlwaysAsk, // Web模式保持审批
        }
    }

    pub fn wechat() -> Self {
        Self {
            id: "wechat",
            default_mode: RuntimeMode::Channel,
            assistant_agent: "workspace-assistant",
            supports_native_approval: false,
            supports_inline_replies: true,
            approval_mode: ApprovalMode::Silent, // Channel模式：无审批
        }
    }

    pub fn telegram() -> Self {
        Self {
            id: "telegram",
            default_mode: RuntimeMode::Channel,
            assistant_agent: "workspace-assistant",
            supports_native_approval: false,
            supports_inline_replies: true,
            approval_mode: ApprovalMode::Silent, // Channel模式：无审批
        }
    }

    pub fn discord() -> Self {
        Self {
            id: "discord",
            default_mode: RuntimeMode::Channel,
            assistant_agent: "workspace-assistant",
            supports_native_approval: false,
            supports_inline_replies: true,
            approval_mode: ApprovalMode::Silent, // Channel模式：无审批
        }
    }

    /// 根据通道类型确定审批模式
    pub fn approval_mode_for_channel_type(channel_type: &ChannelType) -> ApprovalMode {
        match channel_type {
            ChannelType::WeChat | ChannelType::Telegram | ChannelType::Discord => {
                ApprovalMode::Silent // Channel模式：无审批
            }
            ChannelType::CLI | ChannelType::IDE => {
                ApprovalMode::AlwaysAsk // TUI模式：保持Claude级别审批
            }
            _ => ApprovalMode::AlwaysAsk, // 其他模式默认询问
        }
    }

    /// 判断是否为交互式通道（需要用户直接操作）
    pub fn is_interactive_channel(&self) -> bool {
        matches!(self.id, "cli" | "ide")
    }

    /// 判断是否为消息通道（异步消息）
    pub fn is_messaging_channel(&self) -> bool {
        matches!(
            self.id,
            "wechat" | "telegram" | "discord" | "slack" | "whatsapp"
        )
    }

    /// 获取通道的友好名称
    pub fn friendly_name(&self) -> &'static str {
        match self.id {
            "cli" => "命令行",
            "ide" => "IDE集成",
            "web" => "Web界面",
            "wechat" => "微信",
            "telegram" => "Telegram",
            "discord" => "Discord",
            "slack" => "Slack",
            "whatsapp" => "WhatsApp",
            _ => "未知通道",
        }
    }
}

pub fn profile_for_channel_type(channel_type: &anycode_core::ChannelType) -> ChannelProfile {
    let mut profile = match channel_type {
        anycode_core::ChannelType::CLI => ChannelProfile::cli(),
        anycode_core::ChannelType::IDE => ChannelProfile::ide(),
        anycode_core::ChannelType::Web => ChannelProfile::web(),
        anycode_core::ChannelType::WeChat => ChannelProfile::wechat(),
        anycode_core::ChannelType::Telegram => ChannelProfile::telegram(),
        anycode_core::ChannelType::Discord => ChannelProfile::discord(),
        _ => ChannelProfile::web(),
    };

    // 确保审批模式与通道类型对齐
    profile.approval_mode = ChannelProfile::approval_mode_for_channel_type(channel_type);
    profile
}
