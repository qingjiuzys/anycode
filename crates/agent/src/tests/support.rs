use anycode_core::prelude::*;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Default)]
pub(super) struct DummyMemoryStore;

#[async_trait]
impl MemoryStore for DummyMemoryStore {
    async fn save(&self, _memory: Memory) -> Result<(), CoreError> {
        Ok(())
    }

    async fn recall(&self, _query: &str, _mem_type: MemoryType) -> Result<Vec<Memory>, CoreError> {
        Ok(vec![])
    }

    async fn update(&self, _id: &str, _memory: Memory) -> Result<(), CoreError> {
        Ok(())
    }

    async fn delete(&self, _id: &str) -> Result<(), CoreError> {
        Ok(())
    }
}

pub(super) struct EchoTool;

#[async_trait]
impl Tool for EchoTool {
    fn name(&self) -> &str {
        "Echo"
    }

    fn description(&self) -> &str {
        "Echo input for tests"
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": { "text": { "type": "string" } },
            "required": ["text"]
        })
    }

    fn permission_mode(&self) -> PermissionMode {
        PermissionMode::Auto
    }

    fn security_policy(&self) -> Option<&SecurityPolicy> {
        None
    }

    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, CoreError> {
        Ok(ToolOutput {
            result: serde_json::json!({ "echo": input.input }),
            error: None,
            duration_ms: 1,
        })
    }
}

#[derive(Clone)]
pub(super) struct MockLLM {
    calls: Arc<Mutex<Vec<Vec<MessageRole>>>>,
    queue: Arc<Mutex<Vec<LLMResponse>>>,
    stream_queue: Arc<Mutex<Vec<Vec<StreamEvent>>>>,
}

impl MockLLM {
    pub(super) fn new(responses: Vec<LLMResponse>) -> Self {
        Self {
            calls: Arc::new(Mutex::new(vec![])),
            queue: Arc::new(Mutex::new(responses)),
            stream_queue: Arc::new(Mutex::new(vec![])),
        }
    }

    pub(super) fn with_stream_batches(
        responses: Vec<LLMResponse>,
        stream_batches: Vec<Vec<StreamEvent>>,
    ) -> Self {
        Self {
            calls: Arc::new(Mutex::new(vec![])),
            queue: Arc::new(Mutex::new(responses)),
            stream_queue: Arc::new(Mutex::new(stream_batches)),
        }
    }

    pub(super) async fn call_roles(&self) -> Vec<Vec<MessageRole>> {
        self.calls.lock().await.clone()
    }
}

#[async_trait]
impl LLMClient for MockLLM {
    async fn chat(
        &self,
        messages: Vec<Message>,
        _tools: Vec<ToolSchema>,
        _config: &ModelConfig,
    ) -> Result<LLMResponse, CoreError> {
        self.calls
            .lock()
            .await
            .push(messages.iter().map(|m| m.role.clone()).collect());
        let mut q = self.queue.lock().await;
        if q.is_empty() {
            return Err(CoreError::LLMError("mock queue empty".to_string()));
        }
        Ok(q.remove(0))
    }

    async fn chat_stream(
        &self,
        messages: Vec<Message>,
        _tools: Vec<ToolSchema>,
        _config: &ModelConfig,
    ) -> Result<tokio::sync::mpsc::Receiver<StreamEvent>, CoreError> {
        self.calls
            .lock()
            .await
            .push(messages.iter().map(|m| m.role.clone()).collect());
        let mut q = self.stream_queue.lock().await;
        let batch = if q.is_empty() { vec![] } else { q.remove(0) };
        drop(q);
        let cap = batch.len().max(1) + 1;
        let (tx, rx) = tokio::sync::mpsc::channel(cap);
        tokio::spawn(async move {
            for ev in batch {
                if tx.send(ev).await.is_err() {
                    break;
                }
            }
        });
        Ok(rx)
    }
}

/// `chat` stalls so `execute_task` / turn fallback can test in-flight `select!` cancel.
pub(super) struct StallChatLlm {
    pub(super) stall_ms: u64,
    pub(super) response: LLMResponse,
}

#[async_trait]
impl LLMClient for StallChatLlm {
    async fn chat(
        &self,
        _messages: Vec<Message>,
        _tools: Vec<ToolSchema>,
        _config: &ModelConfig,
    ) -> Result<LLMResponse, CoreError> {
        tokio::time::sleep(Duration::from_millis(self.stall_ms)).await;
        Ok(self.response.clone())
    }

    async fn chat_stream(
        &self,
        _messages: Vec<Message>,
        _tools: Vec<ToolSchema>,
        _config: &ModelConfig,
    ) -> Result<tokio::sync::mpsc::Receiver<StreamEvent>, CoreError> {
        Err(CoreError::LLMError("no stream".into()))
    }
}

/// Stream opens immediately; first `recv` blocks until `Done` is sent after `recv_stall_ms`.
pub(super) struct DelayedDoneStreamLlm {
    pub(super) recv_stall_ms: u64,
}

#[async_trait]
impl LLMClient for DelayedDoneStreamLlm {
    async fn chat(
        &self,
        _messages: Vec<Message>,
        _tools: Vec<ToolSchema>,
        _config: &ModelConfig,
    ) -> Result<LLMResponse, CoreError> {
        Err(CoreError::LLMError("stream only".into()))
    }

    async fn chat_stream(
        &self,
        _messages: Vec<Message>,
        _tools: Vec<ToolSchema>,
        _config: &ModelConfig,
    ) -> Result<tokio::sync::mpsc::Receiver<StreamEvent>, CoreError> {
        let (tx, rx) = tokio::sync::mpsc::channel(4);
        let stall = self.recv_stall_ms;
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(stall)).await;
            let _ = tx.send(StreamEvent::Done).await;
        });
        Ok(rx)
    }
}

pub(super) fn msg_text(role: MessageRole, text: &str) -> Message {
    Message {
        id: Uuid::new_v4(),
        role,
        content: MessageContent::Text(text.to_string()),
        timestamp: chrono::Utc::now(),
        metadata: HashMap::new(),
    }
}
