//! anyCode Channels
//!
//! 多通道支持 (来自 OpenClaw)。新增通道：实现 [`ChannelHandler`] 并在 [`ChannelRouter`] 注册
//! （参考 `wechat` 模块与 OpenClaw 的 Channel Plugin 思路）。

mod profile;
mod web_envelope;
mod wechat;

pub use profile::{profile_for_channel_type, ChannelProfile};
pub use web_envelope::{AnycodeWsEnvelopeV1, outbound_channel_message_json};
pub use wechat::WeChatChannel;

use anycode_core::prelude::*;
use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::{Mutex, RwLock};
use tracing::{info, warn};
use tokio_tungstenite::tungstenite::Message as WsMessage;

// ============================================================================
// CLI Channel
// ============================================================================

pub struct CliChannel {
    sender: mpsc::Sender<ChannelMessage>,
    receiver: Mutex<Option<mpsc::Receiver<ChannelMessage>>>,
}

impl CliChannel {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel(1000);
        Self {
            sender,
            receiver: Mutex::new(Some(receiver)),
        }
    }
}

#[async_trait]
impl ChannelHandler for CliChannel {
    fn channel_type(&self) -> ChannelType {
        ChannelType::CLI
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

// ============================================================================
// IDE Channel (LSP)
// ============================================================================

pub struct IdeChannel {
    sender: mpsc::Sender<ChannelMessage>,
    receiver: Mutex<Option<mpsc::Receiver<ChannelMessage>>>,
}

impl IdeChannel {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel(1000);
        Self {
            sender,
            receiver: Mutex::new(Some(receiver)),
        }
    }
}

#[async_trait]
impl ChannelHandler for IdeChannel {
    fn channel_type(&self) -> ChannelType {
        ChannelType::IDE
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

// ============================================================================
// Web Channel (WebSocket)
// ============================================================================

pub struct WebChannel {
    addr: String,
    connections: Arc<RwLock<HashMap<String, mpsc::Sender<WsMessage>>>>,
    receiver: Mutex<Option<mpsc::Receiver<ChannelMessage>>>,
    inbound_sender: mpsc::Sender<ChannelMessage>,
    listener_started: Arc<Mutex<bool>>,
}

impl WebChannel {
    pub fn new(addr: String) -> Self {
        let (inbound_sender, receiver) = mpsc::channel(1000);
        Self {
            addr,
            connections: Arc::new(RwLock::new(HashMap::new())),
            receiver: Mutex::new(Some(receiver)),
            inbound_sender,
            listener_started: Arc::new(Mutex::new(false)),
        }
    }

    async fn ensure_listener_started(&self) -> Result<(), CoreError> {
        let mut started = self.listener_started.lock().await;
        if *started {
            return Ok(());
        }
        let listener = tokio::net::TcpListener::bind(&self.addr)
            .await
            .map_err(|e| CoreError::Other(anyhow::anyhow!("bind {}: {}", self.addr, e)))?;
        let connections = self.connections.clone();
        let inbound_sender = self.inbound_sender.clone();
        tokio::spawn(async move {
            loop {
                let Ok((stream, _addr)) = listener.accept().await else {
                    continue;
                };
                let Ok(ws_stream) = tokio_tungstenite::accept_async(stream).await else {
                    continue;
                };
                let client_id = uuid::Uuid::new_v4().to_string();
                let (mut sink, mut source) = ws_stream.split();
                let (out_tx, mut out_rx) = mpsc::channel::<WsMessage>(128);
                connections
                    .write()
                    .await
                    .insert(client_id.clone(), out_tx.clone());

                let write_id = client_id.clone();
                let write_connections = connections.clone();
                tokio::spawn(async move {
                    while let Some(msg) = out_rx.recv().await {
                        if sink.send(msg).await.is_err() {
                            break;
                        }
                    }
                    write_connections.write().await.remove(&write_id);
                });

                let read_id = client_id.clone();
                let read_connections = connections.clone();
                let inbound_sender = inbound_sender.clone();
                let pong_out = out_tx;
                tokio::spawn(async move {
                    while let Some(Ok(msg)) = source.next().await {
                        if let WsMessage::Text(text) = msg {
                            let t = text.to_string();
                            if web_envelope::is_ping_json(&t) {
                                let _ = pong_out
                                    .send(WsMessage::Text(web_envelope::pong_json().into()))
                                    .await;
                                continue;
                            }
                            let parsed =
                                web_envelope::parse_inbound_text_to_channel_message(&t, &read_id);
                            let _ = inbound_sender.send(parsed).await;
                        }
                    }
                    read_connections.write().await.remove(&read_id);
                });
            }
        });
        *started = true;
        Ok(())
    }
}

#[async_trait]
impl ChannelHandler for WebChannel {
    fn channel_type(&self) -> ChannelType {
        ChannelType::Web
    }

    async fn send_message(&self, msg: ChannelMessage) -> Result<(), CoreError> {
        self.ensure_listener_started().await?;
        let connections = self.connections.read().await;
        let payload = outbound_channel_message_json(&msg)
            .map_err(|e| CoreError::SerializationError(e))?;
        for (_, sender) in connections.iter() {
            let _ = sender.send(WsMessage::Text(payload.clone().into())).await;
        }
        Ok(())
    }

    async fn message_stream(&self) -> Result<mpsc::Receiver<ChannelMessage>, CoreError> {
        self.ensure_listener_started().await?;
        let mut guard = self.receiver.lock().await;
        guard
            .take()
            .ok_or_else(|| CoreError::Other(anyhow::anyhow!("Receiver already taken")))
    }

    fn supports_streaming(&self) -> bool {
        true
    }
}

// ============================================================================
// Channel Router (统一路由)
// ============================================================================

pub struct ChannelRouter {
    channels: Arc<RwLock<HashMap<ChannelType, Box<dyn ChannelHandler>>>>,
    profiles: Arc<RwLock<HashMap<ChannelType, ChannelProfile>>>,
}

impl ChannelRouter {
    pub fn new() -> Self {
        let router = Self {
            channels: Arc::new(RwLock::new(HashMap::new())),
            profiles: Arc::new(RwLock::new(HashMap::new())),
        };
        router
    }

    pub async fn register_channel(&self, channel: Box<dyn ChannelHandler>) {
        let mut channels = self.channels.write().await;
        channels.insert(channel.channel_type(), channel);
    }

    pub async fn register_profile(&self, channel_type: ChannelType, profile: ChannelProfile) {
        let mut profiles = self.profiles.write().await;
        profiles.insert(channel_type, profile);
    }

    pub async fn register_default_profiles(&self) {
        self.register_profile(ChannelType::CLI, ChannelProfile::cli()).await;
        self.register_profile(ChannelType::IDE, ChannelProfile::ide()).await;
        self.register_profile(ChannelType::Web, ChannelProfile::web()).await;
        self.register_profile(ChannelType::WeChat, ChannelProfile::wechat()).await;
    }

    pub async fn profile_for(&self, channel_type: &ChannelType) -> Option<ChannelProfile> {
        self.profiles.read().await.get(channel_type).cloned()
    }

    pub async fn route_message(&self, msg: ChannelMessage) -> Result<(), CoreError> {
        let channels = self.channels.read().await;

        if let Some(channel) = channels.get(&msg.channel_type) {
            channel.send_message(msg).await
        } else {
            warn!("No handler for channel type: {:?}", msg.channel_type);
            Err(CoreError::Other(anyhow::anyhow!("No handler for channel")))
        }
    }

    pub async fn start_all_channels(&self) -> Result<Vec<tokio::task::JoinHandle<()>>, CoreError> {
        let mut handles = vec![];

        // 先把 receiver 都取出来，避免把 channels 的借用带进 spawn('static)
        let mut streams = Vec::new();
        {
            let channels = self.channels.read().await;
            for (channel_type, channel) in channels.iter() {
                let receiver = channel.message_stream().await?;
                streams.push((channel_type.clone(), receiver));
            }
        }

        for (channel_type, mut receiver) in streams {
            let router = self.clone();
            let handle = tokio::spawn(async move {
                info!("Starting channel: {:?}", channel_type);

                while let Some(msg) = receiver.recv().await {
                    if let Err(e) = router.route_message(msg).await {
                        tracing::error!("Route error: {}", e);
                    }
                }

                info!("Channel stopped: {:?}", channel_type);
            });

            handles.push(handle);
        }

        Ok(handles)
    }
}

impl Clone for ChannelRouter {
    fn clone(&self) -> Self {
        Self {
            channels: self.channels.clone(),
            profiles: self.profiles.clone(),
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cli_channel() {
        let channel = CliChannel::new();

        // 发送消息
        let msg = ChannelMessage {
            channel_type: ChannelType::CLI,
            channel_id: "test".to_string(),
            user_id: "user1".to_string(),
            content: "Hello".to_string(),
            timestamp: chrono::Utc::now(),
            reply_to: None,
        };

        assert!(channel.send_message(msg).await.is_ok());

        // 接收消息
        let mut receiver = channel.message_stream().await.unwrap();
        let received = receiver.recv().await.unwrap();

        assert_eq!(received.content, "Hello");
    }

    #[tokio::test]
    async fn test_channel_router() {
        let router = ChannelRouter::new();
        router.register_channel(Box::new(CliChannel::new())).await;

        let msg = ChannelMessage {
            channel_type: ChannelType::CLI,
            channel_id: "test".to_string(),
            user_id: "user1".to_string(),
            content: "Hello".to_string(),
            timestamp: chrono::Utc::now(),
            reply_to: None,
        };

        // 应该能路由到 CLI channel
        assert!(router.route_message(msg).await.is_ok());
    }

    #[tokio::test]
    async fn default_profiles_include_web_and_wechat() {
        let router = ChannelRouter::new();
        router.register_default_profiles().await;
        let web = router.profile_for(&ChannelType::Web).await.expect("web");
        let wechat = router
            .profile_for(&ChannelType::WeChat)
            .await
            .expect("wechat");
        assert_eq!(web.assistant_agent, "workspace-assistant");
        assert_eq!(wechat.assistant_agent, "workspace-assistant");
    }
}
