//! 微信 / IM 类通道：与 `anycode channel wechat`、Node 桥配合时，入站消息可映射为 `ChannelType::WeChat`。

use anycode_core::prelude::*;
use async_trait::async_trait;
use tokio::sync::mpsc;
use tokio::sync::Mutex;

/// 与 `CliChannel` 相同的内存双端队列模型，便于在集成测试中模拟微信入站。
/// 生产环境一般由桥接进程将微信消息转为 `Task` 或 HTTP，而非直接使用本结构。
pub struct WeChatChannel {
    sender: mpsc::Sender<ChannelMessage>,
    receiver: Mutex<Option<mpsc::Receiver<ChannelMessage>>>,
}

impl WeChatChannel {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel(1000);
        Self {
            sender,
            receiver: Mutex::new(Some(receiver)),
        }
    }
}

#[async_trait]
impl ChannelHandler for WeChatChannel {
    fn channel_type(&self) -> ChannelType {
        ChannelType::WeChat
    }

    async fn send_message(&self, msg: ChannelMessage) -> Result<(), CoreError> {
        self.sender
            .send(msg)
            .await
            .map_err(|e| CoreError::Other(anyhow::anyhow!("Send error: {}", e)))
    }

    async fn message_stream(&self) -> Result<mpsc::Receiver<ChannelMessage>, CoreError> {
        let mut guard = self.receiver.lock().await;
        guard
            .take()
            .ok_or_else(|| CoreError::Other(anyhow::anyhow!("Receiver already taken")))
    }

    fn supports_streaming(&self) -> bool {
        true
    }
}
