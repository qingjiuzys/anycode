//! Agent 运行时（LLM + 工具循环、落盘、回执）。

mod artifacts;
mod limits;
mod logging;
mod receipt;
mod session;
mod task_summary;
mod tool_surface;

use crate::compact::{CompactionHooks, DefaultCompactionHooks};
use crate::goal_engine::GoalEngine;
use crate::prompt_assembler::{
    relevant_memories_context_section, runtime_mode_context_section, slash_commands_context_section,
};
use crate::system_prompt::{compose_effective_system_prompt, RuntimePromptConfig};
use crate::{ExploreAgent, GeneralPurposeAgent, GoalAgent, PlanAgent, WorkspaceAssistantAgent};
use anycode_core::prelude::*;
use anycode_core::Artifact;
use anycode_security::SecurityLayer;
use anycode_tools::CompiledClaudePermissionRules;
use artifacts::{extract_artifacts, truncate_text};
use async_trait::async_trait;
use limits::{
    MAX_AGENT_TURNS, MAX_TOOL_CALLS_TOTAL, TOOL_INPUT_LOG_MAX_BYTES, TOOL_RESULT_MAX_BYTES,
};
use logging::RunLogger;
use receipt::ReceiptGenerator;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex as StdMutex};
use task_summary::{last_assistant_plain_text, llm_summary_receipt};
use tokio::sync::{Mutex, RwLock};
use tracing::warn;
use uuid::Uuid;

const MEMORY_AUTOSAVE_TITLE_MAX_CHARS: usize = 200;
const MEMORY_AUTOSAVE_CONTENT_MAX_BYTES: usize = 64 * 1024;

fn last_user_plain_text_for_autosave(msgs: &[Message]) -> String {
    msgs.iter()
        .rev()
        .find_map(|m| {
            if m.role == MessageRole::User {
                match &m.content {
                    MessageContent::Text(t) if !t.trim().is_empty() => Some(t.clone()),
                    _ => None,
                }
            } else {
                None
            }
        })
        .unwrap_or_default()
}

/// 工具权限门控（deny/allow/ask 编译规则 + 可选 MCP 首轮隐藏）。
#[derive(Default)]
pub struct AgentClaudeToolGating {
    pub rules: Option<Arc<CompiledClaudePermissionRules>>,
    pub defer_mcp_tools: bool,
    pub mcp_defer_allowlist: Option<Arc<StdMutex<HashSet<String>>>>,
}

/// Agent 运行时
pub struct AgentRuntime {
    agents: Arc<RwLock<HashMap<AgentType, Box<dyn Agent>>>>,
    llm_client: Arc<dyn LLMClient>,
    tools: Arc<RwLock<HashMap<ToolName, Box<dyn Tool>>>>,
    memory_store: Arc<dyn MemoryStore>,
    default_model_config: ModelConfig,
    model_overrides: HashMap<AgentType, ModelConfig>,
    disk_output: Option<DiskTaskOutput>,
    /// 权限与审批（策略、沙箱、工具确认）
    security: Arc<SecurityLayer>,
    /// 与 config.security.sandbox_mode 对齐：工具内路径/cwd 约束
    sandbox_mode: bool,
    /// `config.json` 的 system_prompt_override / append（已解析 `@path`）
    prompt_config: RuntimePromptConfig,
    /// `memory.auto_save` 且非 noop 后端时，任务成功结束后写入一条 Project 记忆。
    memory_project_autosave_enabled: bool,
    /// 在 LLM 请求前从工具名列表中剔除匹配项（如 `mcp__.*` deny 正则）。
    tool_name_deny: Vec<Regex>,
    claude_gating: AgentClaudeToolGating,
    compaction_hooks: Arc<dyn CompactionHooks>,
}

impl AgentRuntime {
    fn context_messages_from_sections(&self, sections: Vec<String>) -> Vec<Message> {
        sections
            .into_iter()
            .filter(|section| !section.trim().is_empty())
            .map(|section| Message {
                id: Uuid::new_v4(),
                role: MessageRole::User,
                content: MessageContent::Text(section),
                timestamp: chrono::Utc::now(),
                metadata: HashMap::new(),
            })
            .collect()
    }

    fn build_context_sections(
        &self,
        mode: RuntimeMode,
        memories: &[Memory],
        extra_sections: &[String],
    ) -> Vec<String> {
        let mut sections = vec![
            runtime_mode_context_section(mode),
            slash_commands_context_section(),
        ];
        if let Some(section) = self.prompt_config.workspace_section.as_deref() {
            let t = section.trim();
            if !t.is_empty() {
                sections.push(t.to_string());
            }
        }
        if let Some(section) = self.prompt_config.channel_section.as_deref() {
            let t = section.trim();
            if !t.is_empty() {
                sections.push(t.to_string());
            }
        }
        if let Some(section) = self.prompt_config.workflow_section.as_deref() {
            let t = section.trim();
            if !t.is_empty() {
                sections.push(t.to_string());
            }
        }
        if let Some(section) = self.prompt_config.goal_section.as_deref() {
            let t = section.trim();
            if !t.is_empty() {
                sections.push(t.to_string());
            }
        }
        if let Some(section) = relevant_memories_context_section(memories) {
            sections.push(section);
        }
        if !self.prompt_config.prompt_fragments.is_empty() {
            sections.push(self.prompt_config.prompt_fragments.join("\n\n"));
        }
        sections.extend(
            extra_sections
                .iter()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty()),
        );
        sections
    }

    pub fn new(
        llm_client: Arc<dyn LLMClient>,
        tools: HashMap<ToolName, Box<dyn Tool>>,
        memory_store: Arc<dyn MemoryStore>,
        default_model_config: ModelConfig,
        model_overrides: HashMap<AgentType, ModelConfig>,
        disk_output: Option<DiskTaskOutput>,
        security: Arc<SecurityLayer>,
        sandbox_mode: bool,
        prompt_config: RuntimePromptConfig,
        memory_project_autosave_enabled: bool,
        tool_name_deny: Vec<Regex>,
        claude_gating: AgentClaudeToolGating,
        expose_skill_on_explore_plan: bool,
    ) -> Self {
        let mut agents = HashMap::new();

        // 注册内置 agents
        let gp_agent =
            Box::new(GeneralPurposeAgent::new(default_model_config.clone())) as Box<dyn Agent>;
        agents.insert(AgentType::new("general-purpose"), gp_agent);

        let explore_agent = Box::new(ExploreAgent::new(
            default_model_config.clone(),
            expose_skill_on_explore_plan,
        )) as Box<dyn Agent>;
        agents.insert(AgentType::new("explore"), explore_agent);

        let plan_agent = Box::new(PlanAgent::new(
            default_model_config.clone(),
            expose_skill_on_explore_plan,
        )) as Box<dyn Agent>;
        agents.insert(AgentType::new("plan"), plan_agent);

        let workspace_agent = Box::new(WorkspaceAssistantAgent::new(
            default_model_config.clone(),
            expose_skill_on_explore_plan,
        )) as Box<dyn Agent>;
        agents.insert(AgentType::new("workspace-assistant"), workspace_agent);

        let goal_agent = Box::new(GoalAgent::new(default_model_config.clone())) as Box<dyn Agent>;
        agents.insert(AgentType::new("goal"), goal_agent);

        Self {
            agents: Arc::new(RwLock::new(agents)),
            llm_client,
            tools: Arc::new(RwLock::new(tools)),
            memory_store,
            default_model_config,
            model_overrides,
            disk_output,
            security,
            sandbox_mode,
            prompt_config,
            memory_project_autosave_enabled,
            tool_name_deny,
            claude_gating,
            compaction_hooks: Arc::new(DefaultCompactionHooks::new()),
        }
    }

    async fn maybe_autosave_memory(&self, task_id: TaskId, prompt: &str, output: &str) {
        if !self.memory_project_autosave_enabled {
            return;
        }
        let line0 = prompt.lines().next().unwrap_or("").trim();
        let title = if line0.chars().count() > MEMORY_AUTOSAVE_TITLE_MAX_CHARS {
            line0
                .chars()
                .take(MEMORY_AUTOSAVE_TITLE_MAX_CHARS)
                .collect::<String>()
        } else {
            line0.to_string()
        };
        let title = if title.is_empty() {
            "(empty prompt)".to_string()
        } else {
            title
        };
        let (content, _) = truncate_text(output.to_string(), MEMORY_AUTOSAVE_CONTENT_MAX_BYTES);
        let now = chrono::Utc::now();
        let memory = Memory {
            id: task_id.to_string(),
            mem_type: MemoryType::Project,
            title,
            content,
            tags: vec![],
            scope: MemoryScope::Project,
            created_at: now,
            updated_at: now,
        };
        if let Err(e) = self.memory_store.save(memory).await {
            warn!(target: "anycode_agent", "memory auto_save failed: {}", e);
        }
    }

    fn log_task_line(&self, task_id: TaskId, line: &str) {
        if let Some(out) = &self.disk_output {
            let _ = out.append_line(task_id, line);
        }
    }

    fn logger(&self) -> RunLogger {
        RunLogger::new(self.disk_output.clone())
    }

    fn model_for_task(&self, agent_type: &AgentType) -> &ModelConfig {
        self.model_overrides
            .get(agent_type)
            .unwrap_or(&self.default_model_config)
    }

    fn model_for_summary(&self) -> &ModelConfig {
        // 优先 routing.agents.summary，其次复用 plan，再回退 default
        self.model_overrides
            .get(&AgentType::new("summary"))
            .or_else(|| self.model_overrides.get(&AgentType::new("plan")))
            .unwrap_or(&self.default_model_config)
    }

    fn runtime_mode_for_agent(agent_type: &AgentType) -> RuntimeMode {
        match agent_type.as_str() {
            "plan" => RuntimeMode::Plan,
            "explore" => RuntimeMode::Explore,
            "workspace-assistant" | "channel" => RuntimeMode::Channel,
            "goal" => RuntimeMode::Goal,
            _ => RuntimeMode::Code,
        }
    }

    /// 构建 TUI 会话使用的初始 `system` 消息（不注入 memory，避免引入额外不确定性）。
    pub async fn build_system_message(
        &self,
        agent_type: &AgentType,
        working_directory: &str,
    ) -> Result<Message, CoreError> {
        let agents = self.agents.read().await;
        let agent = agents
            .get(agent_type)
            .ok_or_else(|| CoreError::AgentNotFound(Uuid::new_v4()))?;

        let prompt = self.build_system_prompt(agent, working_directory, None)?;

        Ok(Message {
            id: Uuid::new_v4(),
            role: MessageRole::System,
            content: MessageContent::Text(prompt),
            timestamp: chrono::Utc::now(),
            metadata: HashMap::new(),
        })
    }

    /// 构建 TUI 会话初始消息（system + 上下文状态消息；不注入 memory）。
    pub async fn build_session_messages(
        &self,
        agent_type: &AgentType,
        working_directory: &str,
    ) -> Result<Vec<Message>, CoreError> {
        let system = self
            .build_system_message(agent_type, working_directory)
            .await?;
        let mode = Self::runtime_mode_for_agent(agent_type);
        let mut messages = vec![system];
        messages.extend(
            self.context_messages_from_sections(self.build_context_sections(mode, &[], &[])),
        );
        Ok(messages)
    }

    /// 注册自定义 Agent
    pub async fn register_agent(&self, agent: Box<dyn Agent>) {
        let mut agents = self.agents.write().await;
        agents.insert(agent.agent_type().clone(), agent);
    }

    /// 执行一次“连续会话”的 agentic turn：从传入的 `messages` 继续跑同一轮工具循环，
    /// 并在结束后返回最后一条 assistant 自然语言文本、本轮产生的 artifacts、以及本 turn 内各轮 LLM 请求的 **最大 input_tokens**（供 TUI 自动压缩阈值）。
    ///
    /// 关键点：
    /// - 不重建 system/user：由调用方（TUI）维护 messages 历史。
    /// - 工具循环与 `execute_task` 相同：assistant → tool_calls → 执行工具 → tool_result 回注。
    /// - 优先展示最终 assistant 文本；若收尾无可用正文，则退化为 summary 回执，避免 TUI 出现“无总结”空白。
    /// - `messages` 使用 `Arc<Mutex<_>>`：仅在快照/追加时短暂加锁，便于 UI 在 LLM/工具执行中读取增量。
    pub async fn execute_turn_from_messages(
        &self,
        task_id: TaskId,
        agent_type: &AgentType,
        messages: Arc<Mutex<Vec<Message>>>,
        working_directory: &str,
    ) -> Result<(String, Vec<Artifact>, u32), CoreError> {
        let logger = self.logger();
        logger.ensure_initialized(task_id);
        logger.line(
            task_id,
            &format!("[task_start] agent_type={}", agent_type.as_str()),
        );

        // 1) 工具名与 schema（与 `execute_task` 共用 tool_surface，避免漂移）
        let agent_tools = {
            let agents = self.agents.read().await;
            let agent = agents
                .get(agent_type)
                .ok_or_else(|| CoreError::AgentNotFound(Uuid::new_v4()))?;
            agent.tools()
        };

        let tools = self.tools.read().await;
        let raw = tool_surface::resolve_agent_tool_names(agent_type.as_str(), agent_tools, &tools);
        let names = tool_surface::prepare_tool_names_for_llm(
            raw,
            &self.tool_name_deny,
            &self.claude_gating,
        );
        let tool_schemas = tool_surface::build_tool_schemas(&names, &tools);
        drop(tools);

        // 2) agentic loop：保持与 execute_task 的语义一致
        let model_config = self.model_for_task(agent_type).clone();
        let mut total_tool_calls: usize = 0;
        let mut artifacts: Vec<Artifact> = vec![];
        let mut last_assistant_text = String::new();
        let mut max_input_tokens: u32 = 0;

        for turn in 1..=MAX_AGENT_TURNS {
            logger.line(
                task_id,
                &format!("[turn_start] turn={}/{}", turn, MAX_AGENT_TURNS),
            );
            logger.line(
                task_id,
                &format!(
                    "[llm_request_start] turn={} model={} base_url={}",
                    turn,
                    model_config.model,
                    model_config
                        .base_url
                        .clone()
                        .unwrap_or_else(|| "<default>".to_string())
                ),
            );

            let t0 = std::time::Instant::now();
            let messages_snapshot = {
                let g = messages.lock().await;
                g.clone()
            };
            let response = self
                .llm_client
                .chat(messages_snapshot, tool_schemas.clone(), &model_config)
                .await?;

            max_input_tokens = max_input_tokens.max(response.usage.input_tokens);

            logger.line(
                task_id,
                &format!(
                    "[llm_response_end] turn={} elapsed_ms={} input_tokens={} output_tokens={}",
                    turn,
                    t0.elapsed().as_millis(),
                    response.usage.input_tokens,
                    response.usage.output_tokens
                ),
            );

            // 先把 assistant 消息追加回上下文；若本轮有 tool_calls，写入 metadata 供 OpenAI 兼容 provider 重建历史
            let mut assistant_msg = response.message.clone();
            if !response.tool_calls.is_empty() {
                assistant_msg.metadata.insert(
                    ANYCODE_TOOL_CALLS_METADATA_KEY.to_string(),
                    serde_json::to_value(&response.tool_calls)?,
                );
            }

            // 保留「最后一条非空」正文：部分 API 在收尾会再给一条空 assistant，避免覆盖掉仍应作为 turn 摘要的上一段文字。
            let text = match &assistant_msg.content {
                MessageContent::Text(t) => t.clone(),
                _ => String::new(),
            };
            if !text.trim().is_empty() {
                last_assistant_text = text;
            }

            {
                let mut g = messages.lock().await;
                g.push(assistant_msg);
            }

            if response.tool_calls.is_empty() {
                logger.line(task_id, &format!("[turn_end] turn={} tool_calls=0", turn));
                break;
            }

            logger.line(
                task_id,
                &format!(
                    "[turn_end] turn={} tool_calls={}",
                    turn,
                    response.tool_calls.len()
                ),
            );

            for tool_call in response.tool_calls {
                total_tool_calls += 1;
                if total_tool_calls > MAX_TOOL_CALLS_TOTAL {
                    logger.line(
                        task_id,
                        &format!(
                            "[task_end] status=failed reason=max_tool_calls({})",
                            MAX_TOOL_CALLS_TOTAL
                        ),
                    );
                    return Ok((last_assistant_text, artifacts, max_input_tokens));
                }

                // 记录工具入参（截断避免日志/上下文过大）
                let tool_input_json = serde_json::to_string(&tool_call.input)
                    .unwrap_or_else(|_| "<unserializable>".to_string());
                let (tool_input_json, truncated) =
                    truncate_text(tool_input_json, TOOL_INPUT_LOG_MAX_BYTES);
                logger.line(
                    task_id,
                    &format!(
                        "[tool_call_input] turn={} idx={} name={} truncated={}",
                        turn, total_tool_calls, tool_call.name, truncated
                    ),
                );
                logger.line(task_id, &tool_input_json);

                logger.line(
                    task_id,
                    &format!(
                        "[tool_call_start] turn={} idx={} name={}",
                        turn, total_tool_calls, tool_call.name
                    ),
                );
                let t0 = std::time::Instant::now();
                let tool_result = self
                    .execute_tool_call(task_id, working_directory, &tool_call)
                    .await?;
                logger.line(
                    task_id,
                    &format!(
                        "[tool_call_end] turn={} idx={} name={} elapsed_ms={} error={}",
                        turn,
                        total_tool_calls,
                        tool_call.name,
                        t0.elapsed().as_millis(),
                        tool_result
                            .error
                            .clone()
                            .unwrap_or_else(|| "<none>".to_string())
                    ),
                );

                // 回注 tool_result（截断以防爆上下文）
                let tool_text = if let Some(err) = tool_result.error.clone() {
                    format!("ERROR: {}\nRESULT: {}", err, tool_result.result)
                } else {
                    format!("{}", tool_result.result)
                };
                let (tool_text, truncated) = truncate_text(tool_text, TOOL_RESULT_MAX_BYTES);
                if truncated {
                    logger.line(
                        task_id,
                        &format!(
                            "[tool_result] truncated=true max_bytes={}",
                            TOOL_RESULT_MAX_BYTES
                        ),
                    );
                }

                let mut metadata = HashMap::new();
                metadata.insert(
                    "tool_name".to_string(),
                    serde_json::Value::String(tool_call.name.clone()),
                );

                {
                    let mut g = messages.lock().await;
                    g.push(Message {
                        id: Uuid::new_v4(),
                        role: MessageRole::Tool,
                        content: MessageContent::ToolResult {
                            tool_use_id: tool_call.id.clone(),
                            content: tool_text,
                            is_error: tool_result.error.is_some(),
                        },
                        timestamp: chrono::Utc::now(),
                        metadata,
                    });
                }

                artifacts.extend(extract_artifacts(&tool_call, &tool_result));
            }
        }

        logger.line(task_id, "[task_end] status=completed");
        let user_line = {
            let g = messages.lock().await;
            last_user_plain_text_for_autosave(&g)
        };
        if !last_assistant_text.trim().is_empty() {
            self.maybe_autosave_memory(task_id, &user_line, &last_assistant_text)
                .await;
            return Ok((last_assistant_text, artifacts, max_input_tokens));
        }

        let output_tail = logger.tail(task_id, 24 * 1024);
        let artifacts_brief = ReceiptGenerator::artifacts_brief(&artifacts);
        let summary_model = self.model_for_summary().clone();
        let summary_task = Task {
            id: task_id,
            agent_type: agent_type.clone(),
            prompt: user_line.clone(),
            context: TaskContext {
                session_id: Uuid::new_v4(),
                working_directory: working_directory.to_string(),
                environment: HashMap::new(),
                user_id: None,
                system_prompt_append: None,
                context_injections: vec![],
            },
            created_at: chrono::Utc::now(),
        };
        let summary_text = llm_summary_receipt(
            &self.llm_client,
            &summary_model,
            &summary_task,
            total_tool_calls,
            MAX_AGENT_TURNS,
            MAX_TOOL_CALLS_TOTAL,
            &artifacts_brief,
            &output_tail,
        )
        .await;
        self.maybe_autosave_memory(task_id, &user_line, &summary_text)
            .await;
        Ok((summary_text, artifacts, max_input_tokens))
    }

    /// 执行任务
    pub async fn execute_task(&self, task: Task) -> Result<TaskResult, CoreError> {
        let logger = self.logger();
        logger.ensure_initialized(task.id);
        logger.line(
            task.id,
            &format!("[task_start] agent_type={}", task.agent_type.as_str()),
        );

        // 1. 获取 Agent
        let agents = self.agents.read().await;
        let agent = agents
            .get(&task.agent_type)
            .ok_or_else(|| CoreError::AgentNotFound(task.id))?;

        // 2. 加载相关记忆
        let memories = self
            .memory_store
            .recall(&task.prompt, MemoryType::Project)
            .await?;

        // 3. 构建消息（system + context status + user）
        let mode = Self::runtime_mode_for_agent(agent.agent_type());
        let mut messages: Vec<Message> = vec![Message {
            id: Uuid::new_v4(),
            role: MessageRole::System,
            content: MessageContent::Text(self.build_system_prompt(
                agent,
                task.context.working_directory.as_str(),
                task.context.system_prompt_append.as_deref(),
            )?),
            timestamp: chrono::Utc::now(),
            metadata: HashMap::new(),
        }];
        messages.extend(
            self.context_messages_from_sections(self.build_context_sections(
                mode,
                &memories,
                &task.context.context_injections,
            )),
        );

        // 用户消息
        messages.push(Message {
            id: Uuid::new_v4(),
            role: MessageRole::User,
            content: MessageContent::Text(task.prompt.clone()),
            timestamp: chrono::Utc::now(),
            metadata: HashMap::new(),
        });

        // 4. 工具名与 schema（与 TUI turn 共用 tool_surface）
        let tools = self.tools.read().await;
        let raw =
            tool_surface::resolve_agent_tool_names(task.agent_type.as_str(), agent.tools(), &tools);
        let names = tool_surface::prepare_tool_names_for_llm(
            raw,
            &self.tool_name_deny,
            &self.claude_gating,
        );
        let tool_schemas = tool_surface::build_tool_schemas(&names, &tools);
        drop(tools);

        // 5. 多轮 tool loop（assistant → tool_calls → 执行 → tool_result）
        let model_config = self.model_for_task(&task.agent_type);
        let mut total_tool_calls: usize = 0;
        let mut artifacts: Vec<Artifact> = vec![];

        for turn in 1..=MAX_AGENT_TURNS {
            logger.line(
                task.id,
                &format!("[turn_start] turn={}/{}", turn, MAX_AGENT_TURNS),
            );
            logger.line(
                task.id,
                &format!(
                    "[llm_request_start] turn={} model={} base_url={}",
                    turn,
                    model_config.model,
                    model_config
                        .base_url
                        .clone()
                        .unwrap_or_else(|| "<default>".to_string())
                ),
            );

            let t0 = std::time::Instant::now();
            let response = match self
                .llm_client
                .chat(messages.clone(), tool_schemas.clone(), model_config)
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    logger.line(
                        task.id,
                        &format!(
                            "[llm_response_end] status=error turn={} elapsed_ms={} error={}",
                            turn,
                            t0.elapsed().as_millis(),
                            e
                        ),
                    );
                    logger.line(task.id, "[task_end] status=failed");
                    return Ok(TaskResult::Failure {
                        error: "LLM 调用失败".to_string(),
                        details: Some(e.to_string()),
                    });
                }
            };

            logger.line(
                task.id,
                &format!(
                    "[llm_response_end] turn={} elapsed_ms={} input_tokens={} output_tokens={}",
                    turn,
                    t0.elapsed().as_millis(),
                    response.usage.input_tokens,
                    response.usage.output_tokens
                ),
            );

            // 先把 assistant 消息追加回上下文；若本轮有 tool_calls，写入 metadata 供 OpenAI 兼容 provider 重建历史
            let mut assistant_msg = response.message.clone();
            if !response.tool_calls.is_empty() {
                if let Ok(v) = serde_json::to_value(&response.tool_calls) {
                    assistant_msg
                        .metadata
                        .insert(ANYCODE_TOOL_CALLS_METADATA_KEY.to_string(), v);
                }
            }
            messages.push(assistant_msg);

            if response.tool_calls.is_empty() {
                logger.line(task.id, &format!("[turn_end] turn={} tool_calls=0", turn));
                break;
            }

            logger.line(
                task.id,
                &format!(
                    "[turn_end] turn={} tool_calls={}",
                    turn,
                    response.tool_calls.len()
                ),
            );

            for tool_call in response.tool_calls {
                total_tool_calls += 1;
                if total_tool_calls > MAX_TOOL_CALLS_TOTAL {
                    logger.line(
                        task.id,
                        &format!(
                            "[task_end] status=failed reason=max_tool_calls({})",
                            MAX_TOOL_CALLS_TOTAL
                        ),
                    );
                    return Ok(TaskResult::Failure {
                        error: "达到最大工具调用次数，已停止".to_string(),
                        details: Some(format!("max_tool_calls={}", MAX_TOOL_CALLS_TOTAL)),
                    });
                }

                // 记录工具入参（截断避免日志/上下文过大）
                let tool_input_json = serde_json::to_string(&tool_call.input)
                    .unwrap_or_else(|_| "<unserializable>".to_string());
                let (tool_input_json, truncated) =
                    truncate_text(tool_input_json, TOOL_INPUT_LOG_MAX_BYTES);
                logger.line(
                    task.id,
                    &format!(
                        "[tool_call_input] turn={} idx={} name={} truncated={}",
                        turn, total_tool_calls, tool_call.name, truncated
                    ),
                );
                logger.line(task.id, &tool_input_json);

                logger.line(
                    task.id,
                    &format!(
                        "[tool_call_start] turn={} idx={} name={}",
                        turn, total_tool_calls, tool_call.name
                    ),
                );
                let t0 = std::time::Instant::now();
                let tool_result = self
                    .execute_tool_call(task.id, &task.context.working_directory, &tool_call)
                    .await?;
                logger.line(
                    task.id,
                    &format!(
                        "[tool_call_end] turn={} idx={} name={} elapsed_ms={} error={}",
                        turn,
                        total_tool_calls,
                        tool_call.name,
                        t0.elapsed().as_millis(),
                        tool_result
                            .error
                            .clone()
                            .unwrap_or_else(|| "<none>".to_string())
                    ),
                );

                // 回注 tool_result（截断以防爆上下文）
                let tool_text = if let Some(err) = tool_result.error.clone() {
                    format!("ERROR: {}\nRESULT: {}", err, tool_result.result)
                } else {
                    format!("{}", tool_result.result)
                };
                let (tool_text, truncated) = truncate_text(tool_text, TOOL_RESULT_MAX_BYTES);
                if truncated {
                    logger.line(
                        task.id,
                        &format!(
                            "[tool_result] truncated=true max_bytes={}",
                            TOOL_RESULT_MAX_BYTES
                        ),
                    );
                }
                let mut tool_meta = HashMap::new();
                tool_meta.insert(
                    "tool_name".to_string(),
                    serde_json::Value::String(tool_call.name.clone()),
                );
                messages.push(Message {
                    id: Uuid::new_v4(),
                    role: MessageRole::Tool,
                    content: MessageContent::ToolResult {
                        tool_use_id: tool_call.id.clone(),
                        content: tool_text,
                        is_error: tool_result.error.is_some(),
                    },
                    timestamp: chrono::Utc::now(),
                    metadata: tool_meta,
                });

                // 基础 artifacts（V1）
                artifacts.extend(extract_artifacts(&tool_call, &tool_result));
            }
        }

        // 正常收尾：最后一跳无 tool_calls，故末条消息即本轮 assistant。打满 MAX_AGENT_TURNS 且末尾为 Tool 时不走此路径，保留 summary。
        if let Some(fast) = last_assistant_plain_text(&messages) {
            logger.line(task.id, "[task_end] status=completed");
            logger.line(
                task.id,
                &format!(
                    "[final_output] source=assistant reply_chars={}",
                    fast.chars().count()
                ),
            );
            logger.line(task.id, "== assistant_final ==");
            for line in fast.lines() {
                logger.line(task.id, line);
            }
            self.maybe_autosave_memory(task.id, &task.prompt, &fast)
                .await;
            return Ok(TaskResult::Success {
                output: fast,
                artifacts,
            });
        }

        // 7. 生成总结（末条非 assistant、assistant 正文为空、或仅 tool_calls 等）
        let output_tail = logger.tail(task.id, 24 * 1024);
        let artifacts_brief = ReceiptGenerator::artifacts_brief(&artifacts);

        let summary_model = self.model_for_summary().clone();
        let summary_text = llm_summary_receipt(
            &self.llm_client,
            &summary_model,
            &task,
            total_tool_calls,
            MAX_AGENT_TURNS,
            MAX_TOOL_CALLS_TOTAL,
            &artifacts_brief,
            &output_tail,
        )
        .await;

        logger.line(task.id, "[task_end] status=completed");
        logger.line(task.id, "== summary ==");
        for line in summary_text.lines() {
            logger.line(task.id, line);
        }

        self.maybe_autosave_memory(task.id, &task.prompt, &summary_text)
            .await;

        Ok(TaskResult::Success {
            output: summary_text,
            artifacts,
        })
    }

    pub async fn execute_goal_task(
        &self,
        task: Task,
        spec: GoalSpec,
    ) -> Result<(TaskResult, GoalProgress), CoreError> {
        let engine = GoalEngine::new(spec);
        let mut progress = GoalProgress::default();
        let mut current_task = engine.prime_task(task);
        let mut last_result = TaskResult::Failure {
            error: "goal did not run".to_string(),
            details: None,
        };

        while engine.should_continue(&progress) {
            let result = self.execute_task(current_task.clone()).await?;
            engine.update(&mut progress, &result);
            last_result = result.clone();
            if progress.completed {
                break;
            }
            current_task.context.context_injections = vec![format!(
                "## Goal Retry Context\nPrevious attempt count: {}.\nLast error: {:?}\nLast output: {:?}",
                progress.attempts, progress.last_error, progress.last_output
            )];
        }

        Ok((last_result, progress))
    }

    fn build_system_prompt(
        &self,
        agent: &Box<dyn Agent>,
        working_directory: &str,
        task_append: Option<&str>,
    ) -> Result<String, CoreError> {
        Ok(compose_effective_system_prompt(
            &self.prompt_config,
            agent.as_ref(),
            working_directory,
            task_append,
        ))
    }

    async fn execute_tool_call(
        &self,
        task_id: TaskId,
        working_directory: &str,
        tool_call: &ToolCall,
    ) -> Result<ToolOutput, CoreError> {
        let tools = self.tools.read().await;
        let tool = tools
            .get(&tool_call.name)
            .ok_or_else(|| CoreError::ToolNotFound(tool_call.name.clone()))?;

        match self
            .security
            .check_tool_call(&tool_call.name, &tool_call.input)
            .await
        {
            Ok(_) => {}
            Err(CoreError::PermissionDenied(reason)) => {
                self.log_task_line(
                    task_id,
                    &format!("[tool_denied] name={} reason={}", tool_call.name, reason),
                );
                return Ok(ToolOutput {
                    result: serde_json::json!({ "error": reason.clone() }),
                    error: Some(reason),
                    duration_ms: 0,
                });
            }
            Err(e) => return Err(e),
        }

        if !self.security.is_bypass_permissions().await {
            if let Some(rules) = &self.claude_gating.rules {
                let args_json =
                    serde_json::to_string(&tool_call.input).unwrap_or_else(|_| "{}".into());
                if rules.content_denies(&tool_call.name, &args_json)
                    && !rules.content_allows(&tool_call.name, &args_json)
                {
                    let reason =
                        "Permission deny rule matched (tool arguments matched ruleContent)"
                            .to_string();
                    self.log_task_line(
                        task_id,
                        &format!("[tool_denied] name={} reason={}", tool_call.name, reason),
                    );
                    return Ok(ToolOutput {
                        result: serde_json::json!({ "error": reason.clone() }),
                        error: Some(reason),
                        duration_ms: 0,
                    });
                }
                if rules.needs_ask(&tool_call.name, &args_json) {
                    let skip_second_prompt = self
                        .security
                        .skip_redundant_claude_ask_after_tool_check(&tool_call.name)
                        .await;
                    if !skip_second_prompt {
                        match self
                            .security
                            .confirm_claude_ask_or_deny(&tool_call.name, &tool_call.input)
                            .await
                        {
                            Ok(()) => {}
                            Err(CoreError::PermissionDenied(reason)) => {
                                self.log_task_line(
                                    task_id,
                                    &format!(
                                        "[tool_denied] name={} reason={}",
                                        tool_call.name, reason
                                    ),
                                );
                                return Ok(ToolOutput {
                                    result: serde_json::json!({ "error": reason.clone() }),
                                    error: Some(reason),
                                    duration_ms: 0,
                                });
                            }
                            Err(e) => return Err(e),
                        }
                    }
                }
            }
        }

        let input = ToolInput {
            name: tool_call.name.clone(),
            input: tool_call.input.clone(),
            working_directory: if working_directory.is_empty() {
                None
            } else {
                Some(working_directory.to_string())
            },
            sandbox_mode: self.sandbox_mode,
        };

        tool.execute(input).await
    }
}

#[async_trait]
impl SubAgentExecutor for AgentRuntime {
    async fn run_nested_task(
        &self,
        agent_type: AgentType,
        prompt: String,
        working_directory: String,
    ) -> Result<TaskResult, CoreError> {
        let task = Task {
            id: Uuid::new_v4(),
            agent_type,
            prompt,
            context: TaskContext {
                session_id: Uuid::new_v4(),
                working_directory,
                environment: HashMap::new(),
                user_id: None,
                system_prompt_append: None,
                context_injections: vec![],
            },
            created_at: chrono::Utc::now(),
        };
        self.execute_task(task).await
    }
}
