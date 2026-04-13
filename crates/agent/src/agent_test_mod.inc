mod tests {
    use anycode_core::prelude::*;
    use anycode_security::SecurityLayer;
    use crate::{
        AgentClaudeToolGating, AgentRuntime, GeneralPurposeAgent, RuntimeCoreDeps,
        RuntimeMemoryOptions, RuntimePromptConfig, RuntimeToolPolicy,
    };
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::time::Duration;
    use tempfile::TempDir;
    use tokio::sync::Mutex;
    use uuid::Uuid;

    #[test]
    fn test_agent_type() {
        let agent_type = AgentType::new("test");
        assert_eq!(agent_type.as_str(), "test");
    }

    #[tokio::test]
    async fn test_general_purpose_agent() {
        let config = ModelConfig {
            provider: LLMProvider::Anthropic,
            model: "claude-3-5-sonnet-20241022".to_string(),
            base_url: None,
            temperature: Some(0.7),
            max_tokens: Some(4096),
            api_key: None,
        };

        let agent = GeneralPurposeAgent::new(config);
        assert_eq!(agent.agent_type().as_str(), "general-purpose");
        assert!(!agent.description().is_empty());
    }

    #[derive(Default)]
    struct DummyMemoryStore;

    #[async_trait]
    impl MemoryStore for DummyMemoryStore {
        async fn save(&self, _memory: Memory) -> Result<(), CoreError> {
            Ok(())
        }

        async fn recall(
            &self,
            _query: &str,
            _mem_type: MemoryType,
        ) -> Result<Vec<Memory>, CoreError> {
            Ok(vec![])
        }

        async fn update(&self, _id: &str, _memory: Memory) -> Result<(), CoreError> {
            Ok(())
        }

        async fn delete(&self, _id: &str) -> Result<(), CoreError> {
            Ok(())
        }
    }

    struct EchoTool;

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
    struct MockLLM {
        calls: Arc<Mutex<Vec<Vec<MessageRole>>>>,
        queue: Arc<Mutex<Vec<LLMResponse>>>,
        stream_queue: Arc<Mutex<Vec<Vec<StreamEvent>>>>,
    }

    impl MockLLM {
        fn new(responses: Vec<LLMResponse>) -> Self {
            Self {
                calls: Arc::new(Mutex::new(vec![])),
                queue: Arc::new(Mutex::new(responses)),
                stream_queue: Arc::new(Mutex::new(vec![])),
            }
        }

        fn with_stream_batches(
            responses: Vec<LLMResponse>,
            stream_batches: Vec<Vec<StreamEvent>>,
        ) -> Self {
            Self {
                calls: Arc::new(Mutex::new(vec![])),
                queue: Arc::new(Mutex::new(responses)),
                stream_queue: Arc::new(Mutex::new(stream_batches)),
            }
        }

        async fn call_roles(&self) -> Vec<Vec<MessageRole>> {
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
    struct StallChatLlm {
        stall_ms: u64,
        response: LLMResponse,
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
    struct DelayedDoneStreamLlm {
        recv_stall_ms: u64,
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

    fn msg_text(role: MessageRole, text: &str) -> Message {
        Message {
            id: Uuid::new_v4(),
            role,
            content: MessageContent::Text(text.to_string()),
            timestamp: chrono::Utc::now(),
            metadata: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_agent_runtime_tool_loop_injects_tool_result_message() {
        let temp = TempDir::new().unwrap();
        let disk = DiskTaskOutput::new(temp.path().to_path_buf());

        let first = LLMResponse {
            message: msg_text(MessageRole::Assistant, "calling tool"),
            tool_calls: vec![ToolCall {
                id: "tooluse_1".to_string(),
                name: "Echo".to_string(),
                input: serde_json::json!({ "text": "hi" }),
            }],
            usage: Usage {
                input_tokens: 1,
                output_tokens: 1,
                cache_creation_tokens: None,
                cache_read_tokens: None,
            },
        };
        let second = LLMResponse {
            message: msg_text(MessageRole::Assistant, "done"),
            tool_calls: vec![],
            usage: Usage {
                input_tokens: 1,
                output_tokens: 1,
                cache_creation_tokens: None,
                cache_read_tokens: None,
            },
        };

        let llm = Arc::new(MockLLM::new(vec![first, second]));
        let mut tools: HashMap<ToolName, Box<dyn Tool>> = HashMap::new();
        tools.insert("Echo".to_string(), Box::new(EchoTool));

        let runtime = AgentRuntime::new(
            RuntimeCoreDeps {
                llm_client: llm.clone(),
                tools,
                memory_store: Arc::new(DummyMemoryStore),
                default_model_config: ModelConfig {
                    provider: LLMProvider::Custom("mock".to_string()),
                    model: "mock".to_string(),
                    base_url: None,
                    temperature: None,
                    max_tokens: None,
                    api_key: None,
                },
                model_overrides: HashMap::new(),
                disk_output: Some(disk.clone()),
                security: Arc::new(SecurityLayer::new(PermissionMode::BypassPermissions)),
                sandbox_mode: false,
                prompt_config: RuntimePromptConfig::default(),
            },
            RuntimeMemoryOptions {
                memory_pipeline: None,
                memory_pipeline_settings: None,
                memory_project_autosave_enabled: false,
            },
            RuntimeToolPolicy {
                tool_name_deny: vec![],
                claude_gating: AgentClaudeToolGating::default(),
                expose_skill_on_explore_plan: false,
            },
        );

        let task = Task {
            id: Uuid::new_v4(),
            agent_type: AgentType::new("general-purpose"),
            prompt: "test".to_string(),
            context: TaskContext {
                session_id: Uuid::new_v4(),
                working_directory: ".".to_string(),
                environment: HashMap::new(),
                user_id: None,
                system_prompt_append: None,
                context_injections: vec![],
                nested_model_override: None,
                nested_worktree_path: None,
                nested_worktree_repo_root: None,
                nested_cancel: None,
            },
            created_at: chrono::Utc::now(),
        };

        let res = runtime.execute_task(task.clone()).await.unwrap();
        match res {
            TaskResult::Success {
                output,
                artifacts: _,
            } => assert_eq!(output, "done"),
            _ => panic!("expected success"),
        }

        let calls = llm.call_roles().await;
        // 第 1 次 LLM：system + context messages + user
        assert!(calls.len() >= 2);
        assert_eq!(calls[0].first(), Some(&MessageRole::System));
        assert_eq!(calls[0].last(), Some(&MessageRole::User));
        // 第 2 次 LLM：应包含 ToolResult（tool_result 回注）
        assert!(calls[1].contains(&MessageRole::Tool));

        // MVP smoke：日志含完整工具调用链路标记（供人工 / CI 检索）
        let log = disk.tail(task.id, 64 * 1024).unwrap();
        assert!(log.contains("[tool_call_input]"));
        assert!(log.contains("[tool_call_start]"));
        assert!(log.contains("[tool_call_end]"));
    }

    /// Sets [`TaskContext::nested_cancel`] when the tool runs so `execute_task` can exit after the tool boundary.
    struct EchoToolSetsCoopCancel {
        coop: Arc<AtomicBool>,
    }

    #[async_trait]
    impl Tool for EchoToolSetsCoopCancel {
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
            self.coop.store(true, Ordering::Release);
            Ok(ToolOutput {
                result: serde_json::json!({ "echo": input.input }),
                error: None,
                duration_ms: 1,
            })
        }
    }

    #[tokio::test]
    async fn execute_task_cooperative_cancel_before_first_llm() {
        let temp = TempDir::new().unwrap();
        let disk = DiskTaskOutput::new(temp.path().to_path_buf());
        let llm = Arc::new(MockLLM::new(vec![]));
        let mut tools: HashMap<ToolName, Box<dyn Tool>> = HashMap::new();
        tools.insert("Echo".to_string(), Box::new(EchoTool));

        let runtime = AgentRuntime::new(
            RuntimeCoreDeps {
                llm_client: llm.clone(),
                tools,
                memory_store: Arc::new(DummyMemoryStore),
                default_model_config: ModelConfig {
                    provider: LLMProvider::Custom("mock".to_string()),
                    model: "mock".to_string(),
                    base_url: None,
                    temperature: None,
                    max_tokens: None,
                    api_key: None,
                },
                model_overrides: HashMap::new(),
                disk_output: Some(disk),
                security: Arc::new(SecurityLayer::new(PermissionMode::BypassPermissions)),
                sandbox_mode: false,
                prompt_config: RuntimePromptConfig::default(),
            },
            RuntimeMemoryOptions {
                memory_pipeline: None,
                memory_pipeline_settings: None,
                memory_project_autosave_enabled: false,
            },
            RuntimeToolPolicy {
                tool_name_deny: vec![],
                claude_gating: AgentClaudeToolGating::default(),
                expose_skill_on_explore_plan: false,
            },
        );

        let coop = Arc::new(AtomicBool::new(true));
        let task = Task {
            id: Uuid::new_v4(),
            agent_type: AgentType::new("general-purpose"),
            prompt: "test".to_string(),
            context: TaskContext {
                session_id: Uuid::new_v4(),
                working_directory: ".".to_string(),
                environment: HashMap::new(),
                user_id: None,
                system_prompt_append: None,
                context_injections: vec![],
                nested_model_override: None,
                nested_worktree_path: None,
                nested_worktree_repo_root: None,
                nested_cancel: Some(coop),
            },
            created_at: chrono::Utc::now(),
        };

        let res = runtime.execute_task(task).await.unwrap();
        match res {
            TaskResult::Failure { error, .. } => {
                assert_eq!(error, NESTED_TASK_COOPERATIVE_CANCEL_ERROR);
            }
            _ => panic!("expected cooperative cancel failure, got {res:?}"),
        }
        assert!(llm.call_roles().await.is_empty(), "LLM must not run");
    }

    #[tokio::test]
    async fn execute_task_cooperative_cancel_after_tool() {
        let temp = TempDir::new().unwrap();
        let disk = DiskTaskOutput::new(temp.path().to_path_buf());

        let first = LLMResponse {
            message: msg_text(MessageRole::Assistant, "calling tool"),
            tool_calls: vec![ToolCall {
                id: "tooluse_1".to_string(),
                name: "Echo".to_string(),
                input: serde_json::json!({ "text": "hi" }),
            }],
            usage: Usage {
                input_tokens: 1,
                output_tokens: 1,
                cache_creation_tokens: None,
                cache_read_tokens: None,
            },
        };
        let second = LLMResponse {
            message: msg_text(MessageRole::Assistant, "should not run"),
            tool_calls: vec![],
            usage: Usage {
                input_tokens: 1,
                output_tokens: 1,
                cache_creation_tokens: None,
                cache_read_tokens: None,
            },
        };

        let coop = Arc::new(AtomicBool::new(false));
        let llm = Arc::new(MockLLM::new(vec![first, second]));
        let mut tools: HashMap<ToolName, Box<dyn Tool>> = HashMap::new();
        tools.insert(
            "Echo".to_string(),
            Box::new(EchoToolSetsCoopCancel {
                coop: coop.clone(),
            }),
        );

        let runtime = AgentRuntime::new(
            RuntimeCoreDeps {
                llm_client: llm.clone(),
                tools,
                memory_store: Arc::new(DummyMemoryStore),
                default_model_config: ModelConfig {
                    provider: LLMProvider::Custom("mock".to_string()),
                    model: "mock".to_string(),
                    base_url: None,
                    temperature: None,
                    max_tokens: None,
                    api_key: None,
                },
                model_overrides: HashMap::new(),
                disk_output: Some(disk),
                security: Arc::new(SecurityLayer::new(PermissionMode::BypassPermissions)),
                sandbox_mode: false,
                prompt_config: RuntimePromptConfig::default(),
            },
            RuntimeMemoryOptions {
                memory_pipeline: None,
                memory_pipeline_settings: None,
                memory_project_autosave_enabled: false,
            },
            RuntimeToolPolicy {
                tool_name_deny: vec![],
                claude_gating: AgentClaudeToolGating::default(),
                expose_skill_on_explore_plan: false,
            },
        );

        let task = Task {
            id: Uuid::new_v4(),
            agent_type: AgentType::new("general-purpose"),
            prompt: "test".to_string(),
            context: TaskContext {
                session_id: Uuid::new_v4(),
                working_directory: ".".to_string(),
                environment: HashMap::new(),
                user_id: None,
                system_prompt_append: None,
                context_injections: vec![],
                nested_model_override: None,
                nested_worktree_path: None,
                nested_worktree_repo_root: None,
                nested_cancel: Some(coop),
            },
            created_at: chrono::Utc::now(),
        };

        let res = runtime.execute_task(task).await.unwrap();
        match res {
            TaskResult::Failure { error, .. } => {
                assert_eq!(error, NESTED_TASK_COOPERATIVE_CANCEL_ERROR);
            }
            _ => panic!("expected cooperative cancel failure, got {res:?}"),
        }
        assert_eq!(llm.call_roles().await.len(), 1, "second LLM round must be skipped");
    }

    #[tokio::test]
    async fn execute_task_in_flight_llm_cooperative_cancel() {
        let temp = TempDir::new().unwrap();
        let disk = DiskTaskOutput::new(temp.path().to_path_buf());
        let response = LLMResponse {
            message: msg_text(MessageRole::Assistant, "should not return"),
            tool_calls: vec![],
            usage: Usage {
                input_tokens: 1,
                output_tokens: 0,
                cache_creation_tokens: None,
                cache_read_tokens: None,
            },
        };
        let llm = Arc::new(StallChatLlm {
            stall_ms: 60_000,
            response,
        });
        let mut tools: HashMap<ToolName, Box<dyn Tool>> = HashMap::new();
        tools.insert("Echo".to_string(), Box::new(EchoTool));

        let runtime = AgentRuntime::new(
            RuntimeCoreDeps {
                llm_client: llm,
                tools,
                memory_store: Arc::new(DummyMemoryStore),
                default_model_config: ModelConfig {
                    provider: LLMProvider::Custom("mock".to_string()),
                    model: "mock".to_string(),
                    base_url: None,
                    temperature: None,
                    max_tokens: None,
                    api_key: None,
                },
                model_overrides: HashMap::new(),
                disk_output: Some(disk),
                security: Arc::new(SecurityLayer::new(PermissionMode::BypassPermissions)),
                sandbox_mode: false,
                prompt_config: RuntimePromptConfig::default(),
            },
            RuntimeMemoryOptions {
                memory_pipeline: None,
                memory_pipeline_settings: None,
                memory_project_autosave_enabled: false,
            },
            RuntimeToolPolicy {
                tool_name_deny: vec![],
                claude_gating: AgentClaudeToolGating::default(),
                expose_skill_on_explore_plan: false,
            },
        );

        let coop = Arc::new(AtomicBool::new(false));
        let trip = coop.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(80)).await;
            trip.store(true, Ordering::Release);
        });

        let task = Task {
            id: Uuid::new_v4(),
            agent_type: AgentType::new("general-purpose"),
            prompt: "test".to_string(),
            context: TaskContext {
                session_id: Uuid::new_v4(),
                working_directory: ".".to_string(),
                environment: HashMap::new(),
                user_id: None,
                system_prompt_append: None,
                context_injections: vec![],
                nested_model_override: None,
                nested_worktree_path: None,
                nested_worktree_repo_root: None,
                nested_cancel: Some(coop),
            },
            created_at: chrono::Utc::now(),
        };

        let res = tokio::time::timeout(Duration::from_secs(3), runtime.execute_task(task))
            .await
            .expect("execute_task should finish after cooperative cancel, not stall on LLM");
        let res = res.expect("execute_task");
        match res {
            TaskResult::Failure { error, .. } => {
                assert_eq!(error, NESTED_TASK_COOPERATIVE_CANCEL_ERROR);
            }
            _ => panic!("expected cooperative cancel failure, got {res:?}"),
        }
    }

    #[tokio::test]
    async fn execute_turn_from_messages_in_flight_stream_cooperative_cancel() {
        let temp = TempDir::new().unwrap();
        let disk = DiskTaskOutput::new(temp.path().to_path_buf());
        let llm = Arc::new(DelayedDoneStreamLlm {
            recv_stall_ms: 60_000,
        });
        let tools: HashMap<ToolName, Box<dyn Tool>> = HashMap::new();

        let runtime = AgentRuntime::new(
            RuntimeCoreDeps {
                llm_client: llm,
                tools,
                memory_store: Arc::new(DummyMemoryStore),
                default_model_config: ModelConfig {
                    provider: LLMProvider::Custom("mock".to_string()),
                    model: "mock".to_string(),
                    base_url: None,
                    temperature: None,
                    max_tokens: None,
                    api_key: None,
                },
                model_overrides: HashMap::new(),
                disk_output: Some(disk.clone()),
                security: Arc::new(SecurityLayer::new(PermissionMode::BypassPermissions)),
                sandbox_mode: false,
                prompt_config: RuntimePromptConfig::default(),
            },
            RuntimeMemoryOptions {
                memory_pipeline: None,
                memory_pipeline_settings: None,
                memory_project_autosave_enabled: false,
            },
            RuntimeToolPolicy {
                tool_name_deny: vec![],
                claude_gating: AgentClaudeToolGating::default(),
                expose_skill_on_explore_plan: false,
            },
        );

        let agent_type = AgentType::new("general-purpose");
        let mut messages_vec = vec![runtime
            .build_system_message(&agent_type, ".")
            .await
            .unwrap()];
        messages_vec.push(msg_text(MessageRole::User, "hi"));
        let messages = Arc::new(Mutex::new(messages_vec));

        let coop = Arc::new(AtomicBool::new(false));
        let trip = coop.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(80)).await;
            trip.store(true, Ordering::Release);
        });

        let task_id = Uuid::new_v4();
        let err = tokio::time::timeout(
            Duration::from_secs(3),
            runtime.execute_turn_from_messages(
                task_id,
                &agent_type,
                messages.clone(),
                ".",
                Some(coop),
            ),
        )
        .await
        .expect("turn should return after cooperative cancel")
        .expect_err("expected cooperative cancel");
        assert!(
            err.is_cooperative_cancel(),
            "unexpected err: {err:?}"
        );

        let g = messages.lock().await;
        assert!(
            !g.iter().any(|m| m.role == MessageRole::Assistant),
            "placeholder assistant should be popped on stream cancel"
        );
    }

    #[tokio::test]
    async fn execute_turn_from_messages_in_flight_chat_cooperative_cancel() {
        let temp = TempDir::new().unwrap();
        let disk = DiskTaskOutput::new(temp.path().to_path_buf());
        let response = LLMResponse {
            message: msg_text(MessageRole::Assistant, "no"),
            tool_calls: vec![],
            usage: Usage {
                input_tokens: 1,
                output_tokens: 0,
                cache_creation_tokens: None,
                cache_read_tokens: None,
            },
        };
        let llm = Arc::new(StallChatLlm {
            stall_ms: 60_000,
            response,
        });
        let tools: HashMap<ToolName, Box<dyn Tool>> = HashMap::new();

        let runtime = AgentRuntime::new(
            RuntimeCoreDeps {
                llm_client: llm,
                tools,
                memory_store: Arc::new(DummyMemoryStore),
                default_model_config: ModelConfig {
                    provider: LLMProvider::Custom("mock".to_string()),
                    model: "mock".to_string(),
                    base_url: None,
                    temperature: None,
                    max_tokens: None,
                    api_key: None,
                },
                model_overrides: HashMap::new(),
                disk_output: Some(disk.clone()),
                security: Arc::new(SecurityLayer::new(PermissionMode::BypassPermissions)),
                sandbox_mode: false,
                prompt_config: RuntimePromptConfig::default(),
            },
            RuntimeMemoryOptions {
                memory_pipeline: None,
                memory_pipeline_settings: None,
                memory_project_autosave_enabled: false,
            },
            RuntimeToolPolicy {
                tool_name_deny: vec![],
                claude_gating: AgentClaudeToolGating::default(),
                expose_skill_on_explore_plan: false,
            },
        );

        let agent_type = AgentType::new("general-purpose");
        let mut messages_vec = vec![runtime
            .build_system_message(&agent_type, ".")
            .await
            .unwrap()];
        messages_vec.push(msg_text(MessageRole::User, "hi"));
        let messages = Arc::new(Mutex::new(messages_vec));

        let coop = Arc::new(AtomicBool::new(false));
        let trip = coop.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(80)).await;
            trip.store(true, Ordering::Release);
        });

        let task_id = Uuid::new_v4();
        let err = tokio::time::timeout(
            Duration::from_secs(3),
            runtime.execute_turn_from_messages(
                task_id,
                &agent_type,
                messages.clone(),
                ".",
                Some(coop),
            ),
        )
        .await
        .expect("turn should return after cooperative cancel")
        .expect_err("expected cooperative cancel");
        assert!(
            err.is_cooperative_cancel(),
            "unexpected err: {err:?}"
        );

        let g = messages.lock().await;
        assert!(
            !g.iter().any(|m| m.role == MessageRole::Assistant),
            "placeholder assistant should be popped on chat cancel"
        );
    }

    #[tokio::test]
    async fn test_execute_turn_from_messages_returns_final_text_and_injects_tool_result() {
        let temp = TempDir::new().unwrap();
        let disk = DiskTaskOutput::new(temp.path().to_path_buf());

        let first = LLMResponse {
            message: msg_text(MessageRole::Assistant, "calling tool"),
            tool_calls: vec![ToolCall {
                id: "tooluse_1".to_string(),
                name: "Echo".to_string(),
                input: serde_json::json!({ "text": "hi" }),
            }],
            usage: Usage {
                input_tokens: 1,
                output_tokens: 1,
                cache_creation_tokens: None,
                cache_read_tokens: None,
            },
        };
        let second = LLMResponse {
            message: msg_text(MessageRole::Assistant, "done"),
            tool_calls: vec![],
            usage: Usage {
                input_tokens: 1,
                output_tokens: 1,
                cache_creation_tokens: None,
                cache_read_tokens: None,
            },
        };

        let llm = Arc::new(MockLLM::new(vec![first, second]));
        let mut tools: HashMap<ToolName, Box<dyn Tool>> = HashMap::new();
        tools.insert("Echo".to_string(), Box::new(EchoTool));

        let runtime = AgentRuntime::new(
            RuntimeCoreDeps {
                llm_client: llm.clone(),
                tools,
                memory_store: Arc::new(DummyMemoryStore),
                default_model_config: ModelConfig {
                    provider: LLMProvider::Custom("mock".to_string()),
                    model: "mock".to_string(),
                    base_url: None,
                    temperature: None,
                    max_tokens: None,
                    api_key: None,
                },
                model_overrides: HashMap::new(),
                disk_output: Some(disk.clone()),
                security: Arc::new(SecurityLayer::new(PermissionMode::BypassPermissions)),
                sandbox_mode: false,
                prompt_config: RuntimePromptConfig::default(),
            },
            RuntimeMemoryOptions {
                memory_pipeline: None,
                memory_pipeline_settings: None,
                memory_project_autosave_enabled: false,
            },
            RuntimeToolPolicy {
                tool_name_deny: vec![],
                claude_gating: AgentClaudeToolGating::default(),
                expose_skill_on_explore_plan: false,
            },
        );

        let agent_type = AgentType::new("general-purpose");
        let mut messages = vec![runtime
            .build_system_message(&agent_type, ".")
            .await
            .unwrap()];
        messages.push(msg_text(MessageRole::User, "test"));
        let messages = Arc::new(Mutex::new(messages));

        let task_id = Uuid::new_v4();
        let out = runtime
            .execute_turn_from_messages(task_id, &agent_type, messages.clone(), ".", None)
            .await
            .unwrap();

        assert_eq!(out.final_text, "done");
        assert!(out.artifacts.is_empty()); // EchoTool 不在 extract_artifacts 匹配中
        assert_eq!(out.usage.max_input_tokens, 1);
        assert_eq!(out.usage.total_output_tokens, 2);

        let g = messages.lock().await;
        // 至少应注入一条 ToolResult
        assert!(g.iter().any(|m| matches!(m.role, MessageRole::Tool)));

        // 日志含完整工具调用链路标记（用于人工/CI 检索）
        let log = disk.tail(task_id, 64 * 1024).unwrap();
        assert!(log.contains("[tool_call_input]"));
        assert!(log.contains("[tool_call_start]"));
        assert!(log.contains("[tool_call_end]"));

        // assistant metadata 中应包含 tool calls（供 provider 重建历史）
        let has_metadata = g.iter().any(|m| {
            m.role == MessageRole::Assistant
                && m.metadata.contains_key(ANYCODE_TOOL_CALLS_METADATA_KEY)
        });
        assert!(has_metadata);
    }

    struct CountingBashTool {
        executed: Arc<AtomicBool>,
    }

    #[async_trait]
    impl Tool for CountingBashTool {
        fn name(&self) -> &str {
            "Bash"
        }
        fn description(&self) -> &str {
            "Mock bash for security tests"
        }
        fn schema(&self) -> serde_json::Value {
            serde_json::json!({
                "type": "object",
                "properties": { "command": { "type": "string" } },
                "required": ["command"]
            })
        }
        fn permission_mode(&self) -> PermissionMode {
            PermissionMode::Auto
        }
        fn security_policy(&self) -> Option<&SecurityPolicy> {
            None
        }
        async fn execute(&self, _input: ToolInput) -> Result<ToolOutput, CoreError> {
            self.executed.store(true, Ordering::SeqCst);
            Ok(ToolOutput {
                result: serde_json::json!({ "stdout": "ran" }),
                error: None,
                duration_ms: 1,
            })
        }
    }

    #[tokio::test]
    async fn test_security_denied_bash_skips_execute_and_logs_tool_denied() {
        let temp = TempDir::new().unwrap();
        let disk = DiskTaskOutput::new(temp.path().to_path_buf());
        let executed = Arc::new(AtomicBool::new(false));

        let first = LLMResponse {
            message: msg_text(MessageRole::Assistant, "dangerous bash"),
            tool_calls: vec![ToolCall {
                id: "tooluse_rm".to_string(),
                name: "Bash".to_string(),
                input: serde_json::json!({ "command": "rm -rf /tmp/anycode-test-target" }),
            }],
            usage: Usage {
                input_tokens: 1,
                output_tokens: 1,
                cache_creation_tokens: None,
                cache_read_tokens: None,
            },
        };
        let second = LLMResponse {
            message: msg_text(MessageRole::Assistant, "after deny"),
            tool_calls: vec![],
            usage: Usage {
                input_tokens: 1,
                output_tokens: 1,
                cache_creation_tokens: None,
                cache_read_tokens: None,
            },
        };

        let security = Arc::new(SecurityLayer::new(PermissionMode::Default));
        security
            .set_tool_policy(
                "Bash",
                SecurityPolicy::interactive_shell(),
            )
            .await;

        let llm = Arc::new(MockLLM::new(vec![first, second]));
        let mut tools: HashMap<ToolName, Box<dyn Tool>> = HashMap::new();
        tools.insert(
            "Bash".to_string(),
            Box::new(CountingBashTool {
                executed: executed.clone(),
            }),
        );

        let runtime = AgentRuntime::new(
            RuntimeCoreDeps {
                llm_client: llm,
                tools,
                memory_store: Arc::new(DummyMemoryStore),
                default_model_config: ModelConfig {
                    provider: LLMProvider::Custom("mock".to_string()),
                    model: "mock".to_string(),
                    base_url: None,
                    temperature: None,
                    max_tokens: None,
                    api_key: None,
                },
                model_overrides: HashMap::new(),
                disk_output: Some(disk.clone()),
                security,
                sandbox_mode: false,
                prompt_config: RuntimePromptConfig::default(),
            },
            RuntimeMemoryOptions {
                memory_pipeline: None,
                memory_pipeline_settings: None,
                memory_project_autosave_enabled: false,
            },
            RuntimeToolPolicy {
                tool_name_deny: vec![],
                claude_gating: AgentClaudeToolGating::default(),
                expose_skill_on_explore_plan: false,
            },
        );

        let task = Task {
            id: Uuid::new_v4(),
            agent_type: AgentType::new("general-purpose"),
            prompt: "test".to_string(),
            context: TaskContext {
                session_id: Uuid::new_v4(),
                working_directory: ".".to_string(),
                environment: HashMap::new(),
                user_id: None,
                system_prompt_append: None,
                context_injections: vec![],
                nested_model_override: None,
                nested_worktree_path: None,
                nested_worktree_repo_root: None,
                nested_cancel: None,
            },
            created_at: chrono::Utc::now(),
        };

        let _ = runtime.execute_task(task.clone()).await.unwrap();
        assert!(
            !executed.load(Ordering::SeqCst),
            "denied Bash must not run execute()"
        );

        let log = disk.tail(task.id, 64 * 1024).unwrap();
        assert!(
            log.contains("[tool_denied]") && log.contains("name=Bash"),
            "log should record denial: {}",
            log
        );
    }

    struct CountingFileWriteTool {
        executed: Arc<AtomicBool>,
    }

    #[async_trait]
    impl Tool for CountingFileWriteTool {
        fn name(&self) -> &str {
            "FileWrite"
        }
        fn description(&self) -> &str {
            "Mock FileWrite for security tests"
        }
        fn schema(&self) -> serde_json::Value {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "file_path": { "type": "string" },
                    "content": { "type": "string" }
                },
                "required": ["file_path"]
            })
        }
        fn permission_mode(&self) -> PermissionMode {
            PermissionMode::Auto
        }
        fn security_policy(&self) -> Option<&SecurityPolicy> {
            None
        }
        async fn execute(&self, _input: ToolInput) -> Result<ToolOutput, CoreError> {
            self.executed.store(true, Ordering::SeqCst);
            Ok(ToolOutput {
                result: serde_json::json!({ "success": true }),
                error: None,
                duration_ms: 1,
            })
        }
    }

    #[tokio::test]
    async fn test_security_denied_filewrite_silent_approval_skips_execute() {
        let temp = TempDir::new().unwrap();
        let disk = DiskTaskOutput::new(temp.path().to_path_buf());
        let executed = Arc::new(AtomicBool::new(false));

        let first = LLMResponse {
            message: msg_text(MessageRole::Assistant, "write file"),
            tool_calls: vec![ToolCall {
                id: "tooluse_fw".to_string(),
                name: "FileWrite".to_string(),
                input: serde_json::json!({ "file_path": "/tmp/anycode-fw.txt", "content": "x" }),
            }],
            usage: Usage {
                input_tokens: 1,
                output_tokens: 1,
                cache_creation_tokens: None,
                cache_read_tokens: None,
            },
        };
        let second = LLMResponse {
            message: msg_text(MessageRole::Assistant, "after deny"),
            tool_calls: vec![],
            usage: Usage {
                input_tokens: 1,
                output_tokens: 1,
                cache_creation_tokens: None,
                cache_read_tokens: None,
            },
        };

        let security = Arc::new(SecurityLayer::new_with_optional_callback(
            PermissionMode::Default,
            Some(Box::new(
                anycode_security::InteractiveApprovalCallback::new(
                    anycode_security::PromptFormat::Silent,
                ),
            )),
        ));
        security
            .set_tool_policy(
                "FileWrite",
                SecurityPolicy::sensitive_mutation(),
            )
            .await;

        let llm = Arc::new(MockLLM::new(vec![first, second]));
        let mut tools: HashMap<ToolName, Box<dyn Tool>> = HashMap::new();
        tools.insert(
            "FileWrite".to_string(),
            Box::new(CountingFileWriteTool {
                executed: executed.clone(),
            }),
        );

        let runtime = AgentRuntime::new(
            RuntimeCoreDeps {
                llm_client: llm,
                tools,
                memory_store: Arc::new(DummyMemoryStore),
                default_model_config: ModelConfig {
                    provider: LLMProvider::Custom("mock".to_string()),
                    model: "mock".to_string(),
                    base_url: None,
                    temperature: None,
                    max_tokens: None,
                    api_key: None,
                },
                model_overrides: HashMap::new(),
                disk_output: Some(disk.clone()),
                security,
                sandbox_mode: false,
                prompt_config: RuntimePromptConfig::default(),
            },
            RuntimeMemoryOptions {
                memory_pipeline: None,
                memory_pipeline_settings: None,
                memory_project_autosave_enabled: false,
            },
            RuntimeToolPolicy {
                tool_name_deny: vec![],
                claude_gating: AgentClaudeToolGating::default(),
                expose_skill_on_explore_plan: false,
            },
        );

        let task = Task {
            id: Uuid::new_v4(),
            agent_type: AgentType::new("general-purpose"),
            prompt: "test".to_string(),
            context: TaskContext {
                session_id: Uuid::new_v4(),
                working_directory: ".".to_string(),
                environment: HashMap::new(),
                user_id: None,
                system_prompt_append: None,
                context_injections: vec![],
                nested_model_override: None,
                nested_worktree_path: None,
                nested_worktree_repo_root: None,
                nested_cancel: None,
            },
            created_at: chrono::Utc::now(),
        };

        let _ = runtime.execute_task(task.clone()).await.unwrap();
        assert!(
            !executed.load(Ordering::SeqCst),
            "denied FileWrite must not run execute()"
        );

        let log = disk.tail(task.id, 64 * 1024).unwrap();
        assert!(
            log.contains("[tool_denied]") && log.contains("name=FileWrite"),
            "log should record denial: {}",
            log
        );
    }

    struct BigResultTool;

    #[async_trait]
    impl Tool for BigResultTool {
        fn name(&self) -> &str {
            "BigResult"
        }
        fn description(&self) -> &str {
            "Returns huge result"
        }
        fn schema(&self) -> serde_json::Value {
            serde_json::json!({ "type": "object", "properties": {}, "required": [] })
        }
        fn permission_mode(&self) -> PermissionMode {
            PermissionMode::Auto
        }
        fn security_policy(&self) -> Option<&SecurityPolicy> {
            None
        }
        async fn execute(&self, _input: ToolInput) -> Result<ToolOutput, CoreError> {
            let huge = "x".repeat(16 * 1024);
            Ok(ToolOutput {
                result: serde_json::json!({ "huge": huge }),
                error: None,
                duration_ms: 1,
            })
        }
    }

    #[tokio::test]
    async fn test_tool_result_truncation_is_logged() {
        let temp = TempDir::new().unwrap();
        let disk = DiskTaskOutput::new(temp.path().to_path_buf());

        let first = LLMResponse {
            message: msg_text(MessageRole::Assistant, "calling tool"),
            tool_calls: vec![ToolCall {
                id: "tooluse_1".to_string(),
                name: "BigResult".to_string(),
                input: serde_json::json!({}),
            }],
            usage: Usage {
                input_tokens: 1,
                output_tokens: 1,
                cache_creation_tokens: None,
                cache_read_tokens: None,
            },
        };
        let second = LLMResponse {
            message: msg_text(MessageRole::Assistant, "done"),
            tool_calls: vec![],
            usage: Usage {
                input_tokens: 1,
                output_tokens: 1,
                cache_creation_tokens: None,
                cache_read_tokens: None,
            },
        };

        let llm = Arc::new(MockLLM::new(vec![first, second]));
        let mut tools: HashMap<ToolName, Box<dyn Tool>> = HashMap::new();
        tools.insert("BigResult".to_string(), Box::new(BigResultTool));

        let runtime = AgentRuntime::new(
            RuntimeCoreDeps {
                llm_client: llm,
                tools,
                memory_store: Arc::new(DummyMemoryStore),
                default_model_config: ModelConfig {
                    provider: LLMProvider::Custom("mock".to_string()),
                    model: "mock".to_string(),
                    base_url: None,
                    temperature: None,
                    max_tokens: None,
                    api_key: None,
                },
                model_overrides: HashMap::new(),
                disk_output: Some(disk.clone()),
                security: Arc::new(SecurityLayer::new(PermissionMode::BypassPermissions)),
                sandbox_mode: false,
                prompt_config: RuntimePromptConfig::default(),
            },
            RuntimeMemoryOptions {
                memory_pipeline: None,
                memory_pipeline_settings: None,
                memory_project_autosave_enabled: false,
            },
            RuntimeToolPolicy {
                tool_name_deny: vec![],
                claude_gating: AgentClaudeToolGating::default(),
                expose_skill_on_explore_plan: false,
            },
        );

        let task = Task {
            id: Uuid::new_v4(),
            agent_type: AgentType::new("general-purpose"),
            prompt: "test".to_string(),
            context: TaskContext {
                session_id: Uuid::new_v4(),
                working_directory: ".".to_string(),
                environment: HashMap::new(),
                user_id: None,
                system_prompt_append: None,
                context_injections: vec![],
                nested_model_override: None,
                nested_worktree_path: None,
                nested_worktree_repo_root: None,
                nested_cancel: None,
            },
            created_at: chrono::Utc::now(),
        };

        let _ = runtime.execute_task(task.clone()).await.unwrap();
        let log = disk.tail(task.id, 64 * 1024).unwrap();
        assert!(log.contains("[tool_result] truncated=true"));
    }

    struct FileWriteLikeTool;

    #[async_trait]
    impl Tool for FileWriteLikeTool {
        fn name(&self) -> &str {
            "FileWrite"
        }
        fn description(&self) -> &str {
            "Mock FileWrite"
        }
        fn schema(&self) -> serde_json::Value {
            serde_json::json!({ "type": "object", "properties": { "file_path": { "type": "string" } }, "required": ["file_path"] })
        }
        fn permission_mode(&self) -> PermissionMode {
            PermissionMode::Auto
        }
        fn security_policy(&self) -> Option<&SecurityPolicy> {
            None
        }
        async fn execute(&self, input: ToolInput) -> Result<ToolOutput, CoreError> {
            let p = input
                .input
                .get("file_path")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            Ok(ToolOutput {
                result: serde_json::json!({ "success": true, "path": p }),
                error: None,
                duration_ms: 1,
            })
        }
    }

    #[tokio::test]
    async fn test_filewrite_artifact_is_returned() {
        let temp = TempDir::new().unwrap();
        let disk = DiskTaskOutput::new(temp.path().to_path_buf());

        let first = LLMResponse {
            message: msg_text(MessageRole::Assistant, "write file"),
            tool_calls: vec![ToolCall {
                id: "tooluse_1".to_string(),
                name: "FileWrite".to_string(),
                input: serde_json::json!({ "file_path": "/tmp/a.txt", "content": "hi" }),
            }],
            usage: Usage {
                input_tokens: 1,
                output_tokens: 1,
                cache_creation_tokens: None,
                cache_read_tokens: None,
            },
        };
        let second = LLMResponse {
            message: msg_text(MessageRole::Assistant, "done"),
            tool_calls: vec![],
            usage: Usage {
                input_tokens: 1,
                output_tokens: 1,
                cache_creation_tokens: None,
                cache_read_tokens: None,
            },
        };

        let llm = Arc::new(MockLLM::new(vec![first, second]));
        let mut tools: HashMap<ToolName, Box<dyn Tool>> = HashMap::new();
        tools.insert("FileWrite".to_string(), Box::new(FileWriteLikeTool));

        let runtime = AgentRuntime::new(
            RuntimeCoreDeps {
                llm_client: llm,
                tools,
                memory_store: Arc::new(DummyMemoryStore),
                default_model_config: ModelConfig {
                    provider: LLMProvider::Custom("mock".to_string()),
                    model: "mock".to_string(),
                    base_url: None,
                    temperature: None,
                    max_tokens: None,
                    api_key: None,
                },
                model_overrides: HashMap::new(),
                disk_output: Some(disk),
                security: Arc::new(SecurityLayer::new(PermissionMode::BypassPermissions)),
                sandbox_mode: false,
                prompt_config: RuntimePromptConfig::default(),
            },
            RuntimeMemoryOptions {
                memory_pipeline: None,
                memory_pipeline_settings: None,
                memory_project_autosave_enabled: false,
            },
            RuntimeToolPolicy {
                tool_name_deny: vec![],
                claude_gating: AgentClaudeToolGating::default(),
                expose_skill_on_explore_plan: false,
            },
        );

        let task = Task {
            id: Uuid::new_v4(),
            agent_type: AgentType::new("general-purpose"),
            prompt: "test".to_string(),
            context: TaskContext {
                session_id: Uuid::new_v4(),
                working_directory: ".".to_string(),
                environment: HashMap::new(),
                user_id: None,
                system_prompt_append: None,
                context_injections: vec![],
                nested_model_override: None,
                nested_worktree_path: None,
                nested_worktree_repo_root: None,
                nested_cancel: None,
            },
            created_at: chrono::Utc::now(),
        };

        let res = runtime.execute_task(task).await.unwrap();
        match res {
            TaskResult::Success { artifacts, .. } => {
                assert!(artifacts
                    .iter()
                    .any(|a| a.path.as_deref() == Some("/tmp/a.txt")));
            }
            _ => panic!("expected success"),
        }
    }

    #[derive(Clone, Default)]
    struct RecordingMemoryStore {
        saved: Arc<std::sync::Mutex<Vec<Memory>>>,
    }

    #[async_trait]
    impl MemoryStore for RecordingMemoryStore {
        async fn save(&self, memory: Memory) -> Result<(), CoreError> {
            self.saved.lock().unwrap().push(memory);
            Ok(())
        }

        async fn recall(
            &self,
            _query: &str,
            _mem_type: MemoryType,
        ) -> Result<Vec<Memory>, CoreError> {
            Ok(vec![])
        }

        async fn update(&self, _id: &str, _memory: Memory) -> Result<(), CoreError> {
            Ok(())
        }

        async fn delete(&self, _id: &str) -> Result<(), CoreError> {
            Ok(())
        }
    }

    #[derive(Clone, Default)]
    struct RecordingMemoryPipeline {
        ingested: Arc<std::sync::Mutex<Vec<String>>>,
    }

    #[async_trait]
    impl MemoryPipeline for RecordingMemoryPipeline {
        async fn ingest_fragment(
            &self,
            _session_id: &str,
            text: &str,
            _mem_type: MemoryType,
        ) -> Result<String, CoreError> {
            self.ingested.lock().unwrap().push(text.to_string());
            Ok(Uuid::new_v4().to_string())
        }

        async fn touch(&self, _fragment_id: &str) -> Result<(), CoreError> {
            Ok(())
        }

        async fn tick_decay(&self) -> Result<(), CoreError> {
            Ok(())
        }

        async fn materialize_for_prompt(
            &self,
            _query: &str,
            _mem_type: MemoryType,
        ) -> Result<Vec<Memory>, CoreError> {
            Ok(vec![])
        }

        async fn promote_fragment_to_hot(&self, _fragment_id: &str) -> Result<(), CoreError> {
            Ok(())
        }

        async fn promote_memory_to_vector(
            &self,
            _memory_id: &str,
            _mem_type: MemoryType,
        ) -> Result<(), CoreError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn execute_task_pipeline_hooks_ingest_tool_and_turn_fragments() {
        let temp = TempDir::new().unwrap();
        let disk = DiskTaskOutput::new(temp.path().to_path_buf());
        let first = LLMResponse {
            message: msg_text(MessageRole::Assistant, "calling tool"),
            tool_calls: vec![ToolCall {
                id: "tooluse_1".to_string(),
                name: "Echo".to_string(),
                input: serde_json::json!({ "text": "hi" }),
            }],
            usage: Usage {
                input_tokens: 1,
                output_tokens: 1,
                cache_creation_tokens: None,
                cache_read_tokens: None,
            },
        };
        let second = LLMResponse {
            message: msg_text(MessageRole::Assistant, "final done"),
            tool_calls: vec![],
            usage: Usage {
                input_tokens: 1,
                output_tokens: 1,
                cache_creation_tokens: None,
                cache_read_tokens: None,
            },
        };
        let llm = Arc::new(MockLLM::new(vec![first, second]));
        let mut tools: HashMap<ToolName, Box<dyn Tool>> = HashMap::new();
        tools.insert("Echo".to_string(), Box::new(EchoTool));
        let pipeline = Arc::new(RecordingMemoryPipeline::default());

        let runtime = AgentRuntime::new(
            RuntimeCoreDeps {
                llm_client: llm,
                tools,
                memory_store: Arc::new(DummyMemoryStore),
                default_model_config: ModelConfig {
                    provider: LLMProvider::Custom("mock".to_string()),
                    model: "mock".to_string(),
                    base_url: None,
                    temperature: None,
                    max_tokens: None,
                    api_key: None,
                },
                model_overrides: HashMap::new(),
                disk_output: Some(disk),
                security: Arc::new(SecurityLayer::new(PermissionMode::BypassPermissions)),
                sandbox_mode: false,
                prompt_config: RuntimePromptConfig::default(),
            },
            RuntimeMemoryOptions {
                memory_pipeline: Some(pipeline.clone()),
                memory_pipeline_settings: Some(MemoryPipelineSettings {
                    hook_after_tool_result: true,
                    hook_after_agent_turn: true,
                    hook_tool_deny_prefixes: vec![],
                    ..MemoryPipelineSettings::default()
                }),
                memory_project_autosave_enabled: false,
            },
            RuntimeToolPolicy {
                tool_name_deny: vec![],
                claude_gating: AgentClaudeToolGating::default(),
                expose_skill_on_explore_plan: false,
            },
        );

        let task = Task {
            id: Uuid::new_v4(),
            agent_type: AgentType::new("general-purpose"),
            prompt: "test".to_string(),
            context: TaskContext {
                session_id: Uuid::new_v4(),
                working_directory: ".".to_string(),
                environment: HashMap::new(),
                user_id: None,
                system_prompt_append: None,
                context_injections: vec![],
                nested_model_override: None,
                nested_worktree_path: None,
                nested_worktree_repo_root: None,
                nested_cancel: None,
            },
            created_at: chrono::Utc::now(),
        };

        let _ = runtime.execute_task(task).await.unwrap();
        let ingested = pipeline.ingested.lock().unwrap().clone();
        assert!(
            ingested.iter().any(|s| s.contains("[tool:Echo]")),
            "tool-result hook should ingest Echo result: {:?}",
            ingested
        );
        assert!(
            ingested.iter().any(|s| s.contains("[turn 2]") && s.contains("final done")),
            "assistant-turn hook should ingest final turn summary: {:?}",
            ingested
        );
    }

    #[tokio::test]
    async fn execute_turn_streaming_sets_non_zero_max_input_tokens() {
        let stream_batches = vec![vec![
            StreamEvent::Delta("streaming answer".to_string()),
            StreamEvent::Done,
        ]];
        let llm = Arc::new(MockLLM::with_stream_batches(vec![], stream_batches));

        let runtime = AgentRuntime::new(
            RuntimeCoreDeps {
                llm_client: llm,
                tools: HashMap::new(),
                memory_store: Arc::new(DummyMemoryStore),
                default_model_config: ModelConfig {
                    provider: LLMProvider::Custom("mock".to_string()),
                    model: "mock".to_string(),
                    base_url: None,
                    temperature: None,
                    max_tokens: None,
                    api_key: None,
                },
                model_overrides: HashMap::new(),
                disk_output: None,
                security: Arc::new(SecurityLayer::new(PermissionMode::BypassPermissions)),
                sandbox_mode: false,
                prompt_config: RuntimePromptConfig::default(),
            },
            RuntimeMemoryOptions {
                memory_pipeline: None,
                memory_pipeline_settings: None,
                memory_project_autosave_enabled: false,
            },
            RuntimeToolPolicy {
                tool_name_deny: vec![],
                claude_gating: AgentClaudeToolGating::default(),
                expose_skill_on_explore_plan: false,
            },
        );

        let messages = Arc::new(Mutex::new(vec![
            msg_text(MessageRole::System, "system"),
            msg_text(
                MessageRole::User,
                "this is a sufficiently long streaming prompt for token estimation",
            ),
        ]));
        let out = runtime
            .execute_turn_from_messages(
                Uuid::new_v4(),
                &AgentType::new("general-purpose"),
                messages,
                ".",
                None,
            )
            .await
            .unwrap();
        assert!(
            out.usage.max_input_tokens > 0,
            "streaming mode should expose non-zero input tokens"
        );
    }

    #[tokio::test]
    async fn execute_turn_streaming_prefers_usage_event_input_tokens() {
        let stream_batches = vec![vec![
            StreamEvent::Usage(Usage {
                input_tokens: 321,
                output_tokens: 7,
                cache_creation_tokens: None,
                cache_read_tokens: None,
            }),
            StreamEvent::Delta("ok".to_string()),
            StreamEvent::Done,
        ]];
        let llm = Arc::new(MockLLM::with_stream_batches(vec![], stream_batches));

        let runtime = AgentRuntime::new(
            RuntimeCoreDeps {
                llm_client: llm,
                tools: HashMap::new(),
                memory_store: Arc::new(DummyMemoryStore),
                default_model_config: ModelConfig {
                    provider: LLMProvider::Custom("mock".to_string()),
                    model: "mock".to_string(),
                    base_url: None,
                    temperature: None,
                    max_tokens: None,
                    api_key: None,
                },
                model_overrides: HashMap::new(),
                disk_output: None,
                security: Arc::new(SecurityLayer::new(PermissionMode::BypassPermissions)),
                sandbox_mode: false,
                prompt_config: RuntimePromptConfig::default(),
            },
            RuntimeMemoryOptions {
                memory_pipeline: None,
                memory_pipeline_settings: None,
                memory_project_autosave_enabled: false,
            },
            RuntimeToolPolicy {
                tool_name_deny: vec![],
                claude_gating: AgentClaudeToolGating::default(),
                expose_skill_on_explore_plan: false,
            },
        );

        let messages = Arc::new(Mutex::new(vec![
            msg_text(MessageRole::System, "system"),
            msg_text(MessageRole::User, "prompt"),
        ]));
        let out = runtime
            .execute_turn_from_messages(
                Uuid::new_v4(),
                &AgentType::new("general-purpose"),
                messages,
                ".",
                None,
            )
            .await
            .unwrap();
        assert_eq!(out.usage.max_input_tokens, 321);
        assert_eq!(out.usage.total_output_tokens, 7);
    }

    #[tokio::test]
    async fn execute_task_success_triggers_memory_autosave_when_enabled() {
        let temp = TempDir::new().unwrap();
        let disk = DiskTaskOutput::new(temp.path().to_path_buf());
        let resp = LLMResponse {
            message: msg_text(MessageRole::Assistant, "final answer"),
            tool_calls: vec![],
            usage: Usage {
                input_tokens: 1,
                output_tokens: 1,
                cache_creation_tokens: None,
                cache_read_tokens: None,
            },
        };
        let llm = Arc::new(MockLLM::new(vec![resp]));
        let mut tools: HashMap<ToolName, Box<dyn Tool>> = HashMap::new();
        tools.insert("Echo".to_string(), Box::new(EchoTool));

        let store = Arc::new(RecordingMemoryStore::default());
        let task_id = Uuid::new_v4();
        let runtime = AgentRuntime::new(
            RuntimeCoreDeps {
                llm_client: llm,
                tools,
                memory_store: store.clone(),
                default_model_config: ModelConfig {
                    provider: LLMProvider::Custom("mock".to_string()),
                    model: "mock".to_string(),
                    base_url: None,
                    temperature: None,
                    max_tokens: None,
                    api_key: None,
                },
                model_overrides: HashMap::new(),
                disk_output: Some(disk),
                security: Arc::new(SecurityLayer::new(PermissionMode::BypassPermissions)),
                sandbox_mode: false,
                prompt_config: RuntimePromptConfig::default(),
            },
            RuntimeMemoryOptions {
                memory_pipeline: None,
                memory_pipeline_settings: None,
                memory_project_autosave_enabled: true,
            },
            RuntimeToolPolicy {
                tool_name_deny: vec![],
                claude_gating: AgentClaudeToolGating::default(),
                expose_skill_on_explore_plan: false,
            },
        );

        let task = Task {
            id: task_id,
            agent_type: AgentType::new("general-purpose"),
            prompt: "title line\nrest".to_string(),
            context: TaskContext {
                session_id: Uuid::new_v4(),
                working_directory: ".".to_string(),
                environment: HashMap::new(),
                user_id: None,
                system_prompt_append: None,
                context_injections: vec![],
                nested_model_override: None,
                nested_worktree_path: None,
                nested_worktree_repo_root: None,
                nested_cancel: None,
            },
            created_at: chrono::Utc::now(),
        };

        let res = runtime.execute_task(task).await.unwrap();
        match res {
            TaskResult::Success { output, .. } => assert_eq!(output, "final answer"),
            _ => panic!("expected success"),
        }
        let saved = store.saved.lock().unwrap();
        assert_eq!(saved.len(), 1);
        assert_eq!(saved[0].mem_type, MemoryType::Project);
        assert_eq!(saved[0].id, task_id.to_string());
        assert_eq!(saved[0].title, "title line");
        assert_eq!(saved[0].content, "final answer");
    }

    #[test]
    fn goal_engine_max_attempts_cap_stops_even_when_infinite_retries() {
        use crate::GoalEngine;
        use anycode_core::{GoalProgress, GoalSpec};
        let engine = GoalEngine::new(GoalSpec {
            objective: "test".into(),
            done_when: None,
            allow_infinite_retries: true,
            max_attempts_cap: Some(2),
        });
        let mut p = GoalProgress::default();
        p.attempts = 2;
        assert!(!engine.should_continue(&p));
        p.attempts = 1;
        assert!(engine.should_continue(&p));
    }
}
