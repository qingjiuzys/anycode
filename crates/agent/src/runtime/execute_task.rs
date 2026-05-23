//! Task execution

use super::agentic_loop::{coop_flag_wait, nested_coop_cancelled, task_cancelled_failure};
use super::artifacts::extract_artifacts;
use super::budget::{record_llm_usage, tick_budget, RuntimeBudgetState};
use super::limits::{MAX_AGENT_TURNS, MAX_TOOL_CALLS_TOTAL};
use super::nested_worktree::NestedWorktreeGuard;
use super::receipt::ReceiptGenerator;
use super::task_summary::{last_assistant_plain_text, llm_summary_receipt};
use super::tool_result_injection;
use super::tool_surface;
use super::{AgentRuntime, ParentToolSurfaceGuard};
use anycode_core::prelude::*;
use anycode_core::strip_llm_reasoning_xml_blocks;
use anycode_core::Artifact;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

impl AgentRuntime {
    /// 执行任务
    pub async fn execute_task(&self, task: Task) -> Result<TaskResult, CoreError> {
        let _parent_tool_surface = {
            let guard = self.tool_services.lock().ok();
            if let Some(svc) = guard.as_ref().and_then(|g| g.as_ref()) {
                svc.set_parent_task_tool_deny(
                    task.context.tool_deny_names.clone(),
                    task.context.tool_deny_prefixes.clone(),
                );
                Some(ParentToolSurfaceGuard {
                    services: Arc::clone(svc),
                })
            } else {
                None
            }
        };

        let _nested_wt = NestedWorktreeGuard(
            match (
                &task.context.nested_worktree_repo_root,
                &task.context.nested_worktree_path,
            ) {
                (Some(r), Some(p)) if !r.is_empty() && !p.is_empty() => {
                    Some((r.clone(), p.clone()))
                }
                _ => None,
            },
        );

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
            &task.context.tool_deny_names,
            &task.context.tool_deny_prefixes,
        );
        let tool_schemas = tool_surface::build_tool_schemas(&names, &tools);
        drop(tools);

        // 5. 多轮 tool loop（assistant → tool_calls → 执行 → tool_result）
        let mut model_config = self.model_for_task(&task.agent_type).clone();
        if let Some(ref hint) = task.context.nested_model_override {
            model_config = crate::nested_model::resolve_nested_model_hint(&model_config, hint);
        }
        let mut total_tool_calls: usize = 0;
        let mut artifacts: Vec<Artifact> = vec![];
        let mut last_model_turn: usize = 1;
        let mut budget_state = RuntimeBudgetState::new(task.context.budget);

        for turn in 1..=MAX_AGENT_TURNS {
            last_model_turn = turn;
            logger.line(
                task.id,
                &format!("[turn_start] turn={}/{}", turn, MAX_AGENT_TURNS),
            );
            if nested_coop_cancelled(&task.context) {
                logger.line(task.id, "[task_end] status=cancelled reason=cooperative");
                return Ok(task_cancelled_failure());
            }
            if tick_budget(&logger, task.id, &mut budget_state) {
                logger.line(task.id, "[task_end] status=failed reason=budget_exceeded");
                return Ok(TaskResult::Failure {
                    error: "运行时预算已用尽".to_string(),
                    details: Some("budget_exceeded".to_string()),
                });
            }
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
            let response_result = match task.context.nested_cancel.clone() {
                Some(flag) => {
                    tokio::select! {
                        biased;
                        () = coop_flag_wait(flag) => {
                            logger.line(
                                task.id,
                                "[llm_response_end] status=cancelled reason=cooperative_in_flight",
                            );
                            logger.line(task.id, "[task_end] status=cancelled reason=cooperative");
                            return Ok(task_cancelled_failure());
                        }
                        res = self.llm_client.chat(
                            messages.clone(),
                            tool_schemas.clone(),
                            &model_config,
                        ) => res,
                    }
                }
                None => {
                    self.llm_client
                        .chat(messages.clone(), tool_schemas.clone(), &model_config)
                        .await
                }
            };

            let response = match response_result {
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
            if record_llm_usage(&logger, task.id, &mut budget_state, &response.usage) {
                logger.line(task.id, "[task_end] status=failed reason=budget_exceeded");
                return Ok(TaskResult::Failure {
                    error: "运行时预算已用尽".to_string(),
                    details: Some("budget_exceeded".to_string()),
                });
            }

            // 先把 assistant 消息追加回上下文
            let mut assistant_msg = response.message.clone();
            if !response.tool_calls.is_empty() {
                if let Ok(v) = serde_json::to_value(&response.tool_calls) {
                    assistant_msg
                        .metadata
                        .insert(ANYCODE_TOOL_CALLS_METADATA_KEY.to_string(), v);
                }
            }
            messages.push(assistant_msg);

            let session_label = task.context.session_id.to_string();
            let turn_plain = messages
                .last()
                .and_then(|m| match &m.content {
                    MessageContent::Text(t) => Some(strip_llm_reasoning_xml_blocks(t)),
                    _ => None,
                })
                .unwrap_or_default();

            if response.tool_calls.is_empty() {
                self.pipeline_memory_hook_agent_turn(&session_label, task.id, turn, &turn_plain)
                    .await;
                self.maybe_session_notify_agent_turn(
                    &session_label,
                    task.id,
                    turn,
                    &turn_plain,
                    Some(task.context.working_directory.as_str()),
                );
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
                if nested_coop_cancelled(&task.context) {
                    logger.line(task.id, "[task_end] status=cancelled reason=cooperative");
                    return Ok(task_cancelled_failure());
                }
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

                tool_result_injection::log_tool_call_input(
                    &logger,
                    task.id,
                    turn,
                    total_tool_calls,
                    &tool_call,
                );
                tool_result_injection::log_tool_call_start(
                    &logger,
                    task.id,
                    turn,
                    total_tool_calls,
                    &tool_call,
                );
                let t0 = std::time::Instant::now();
                let tool_result = self
                    .execute_tool_call(task.id, &task.context.working_directory, &tool_call)
                    .await?;
                tool_result_injection::log_tool_call_end(
                    &logger,
                    task.id,
                    turn,
                    total_tool_calls,
                    &tool_call,
                    &tool_result,
                    t0.elapsed().as_millis(),
                );

                // 回注 tool_result（截断以防爆上下文）
                let prepared = tool_result_injection::prepare_tool_result_message(
                    task.id,
                    &tool_call,
                    &tool_result,
                    &logger,
                );
                messages.push(prepared.message);

                self.pipeline_memory_hook_tool_result(
                    &session_label,
                    task.id,
                    &tool_call.name,
                    &prepared.for_hook,
                )
                .await;
                self.maybe_session_notify_tool_result(
                    &session_label,
                    task.id,
                    turn,
                    &tool_call.name,
                    &prepared.for_hook,
                    Some(task.context.working_directory.as_str()),
                );

                // 基础 artifacts（V1）
                artifacts.extend(extract_artifacts(&tool_call, &tool_result));
                if nested_coop_cancelled(&task.context) {
                    logger.line(task.id, "[task_end] status=cancelled reason=cooperative");
                    return Ok(task_cancelled_failure());
                }
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

        let session_label = task.context.session_id.to_string();
        self.pipeline_memory_hook_agent_turn(
            &session_label,
            task.id,
            last_model_turn,
            &summary_text,
        )
        .await;
        self.maybe_session_notify_agent_turn(
            &session_label,
            task.id,
            last_model_turn,
            &summary_text,
            Some(task.context.working_directory.as_str()),
        );

        Ok(TaskResult::Success {
            output: summary_text,
            artifacts,
        })
    }
}
