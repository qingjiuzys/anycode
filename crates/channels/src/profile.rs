use anycode_core::RuntimeMode;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelProfile {
    pub id: &'static str,
    pub default_mode: RuntimeMode,
    pub assistant_agent: &'static str,
    pub supports_native_approval: bool,
    pub supports_inline_replies: bool,
}

impl ChannelProfile {
    pub fn cli() -> Self {
        Self {
            id: "cli",
            default_mode: RuntimeMode::Code,
            assistant_agent: "general-purpose",
            supports_native_approval: true,
            supports_inline_replies: false,
        }
    }

    pub fn ide() -> Self {
        Self {
            id: "ide",
            default_mode: RuntimeMode::Code,
            assistant_agent: "general-purpose",
            supports_native_approval: true,
            supports_inline_replies: true,
        }
    }

    pub fn web() -> Self {
        Self {
            id: "web",
            default_mode: RuntimeMode::Channel,
            assistant_agent: "workspace-assistant",
            supports_native_approval: true,
            supports_inline_replies: true,
        }
    }

    pub fn wechat() -> Self {
        Self {
            id: "wechat",
            default_mode: RuntimeMode::Channel,
            assistant_agent: "workspace-assistant",
            supports_native_approval: false,
            supports_inline_replies: true,
        }
    }
}

pub fn profile_for_channel_type(channel_type: &anycode_core::ChannelType) -> ChannelProfile {
    match channel_type {
        anycode_core::ChannelType::CLI => ChannelProfile::cli(),
        anycode_core::ChannelType::IDE => ChannelProfile::ide(),
        anycode_core::ChannelType::Web => ChannelProfile::web(),
        anycode_core::ChannelType::WeChat => ChannelProfile::wechat(),
        _ => ChannelProfile::web(),
    }
}
