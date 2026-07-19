//! Task execution

use super::agentic_loop::{coop_flag_wait, nested_coop_cancelled, task_cancelled_failure};
use super::agentic_turn::{
    MessageAppendSink, TurnToolBatchOutcome, TurnToolCancel, TurnToolCancelOutcome, TurnToolCtx,
    TurnToolState,
};
use super::budget::{record_llm_usage, tick_budget, RuntimeBudgetState};
use super::llm_retry::model_config_with_retry_observer;
use super::nested_worktree::NestedWorktreeGuard;
use super::session_activity::{ActivityReason, SessionActivityGuard};
use super::task_summary::last_assistant_plain_text;
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
        // Scope the dashboard chat turn context (session / user turn / reply
        // language) task-locally so approval, question and recorder plumbing
        // consume it without process-global environment variables.
        if let Some(chat_turn) = task.context.chat_turn.clone() {
            return anycode_core::scope_chat_turn(chat_turn, self.execute_task_inner(task)).await;
        }
        self.execute_task_inner(task).await
    }

    async fn execute_task_inner(&self, task: Task) -> Result<TaskResult, CoreError> {
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
        let canonical = super::canonical_agent_type(&task.agent_type);
        let agent = agents
            .get(&canonical)
            .or_else(|| agents.get(&task.agent_type))
            .ok_or_else(|| CoreError::AgentNotFound(task.id))?;

        // 2. 加载相关记忆
        let memories = self
            .memory_store
            .recall(&task.prompt, MemoryType::Project)
            .await?;

        let mut model_config = self.model_for_task(&task.agent_type).clone();
        if let Some(ref hint) = task.context.nested_model_override {
            model_config = crate::nested_model::resolve_nested_model_hint(&model_config, hint);
        }
        let weak_local = anycode_llm::capabilities_for_model_config(&model_config).weak_local_model;
        let system_append = {
            let mut parts = Vec::new();
            if weak_local {
                parts.push(anycode_llm::WEAK_LOCAL_TOOL_GUIDANCE.to_string());
            }
            if let Some(extra) = task.context.system_prompt_append.as_deref() {
                if !extra.trim().is_empty() {
                    parts.push(extra.to_string());
                }
            }
            if parts.is_empty() {
                None
            } else {
                Some(parts.join("\n\n"))
            }
        };

        // 3. 构建消息（system + context status + user）
        let mode = agent.runtime_mode();
        let mut messages: Vec<Message> = vec![Message {
            id: Uuid::new_v4(),
            role: MessageRole::System,
            content: MessageContent::Text(self.build_system_prompt(
                agent,
                task.context.working_directory.as_str(),
                system_append.as_deref(),
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
        let mut user_metadata = HashMap::new();
        if !task.context.user_vision_images.is_empty() {
            attach_vision_images(&mut user_metadata, &task.context.user_vision_images);
        }
        messages.push(Message {
            id: Uuid::new_v4(),
            role: MessageRole::User,
            content: MessageContent::Text(task.prompt.clone()),
            timestamp: chrono::Utc::now(),
            metadata: user_metadata,
        });

        // 4. 工具名与 schema（与 TUI turn 共用 tool_surface）
        let tools = self.tools.read().await;
        let raw =
            tool_surface::resolve_agent_tool_names(task.agent_type.as_str(), agent.tools(), &tools);
        let merged_denies = anycode_tools::merge_agent_type_tool_denies(
            task.agent_type.as_str(),
            &task.context.tool_deny_names,
        );
        let names = tool_surface::prepare_tool_names_for_llm(
            raw,
            &self.tool_name_deny,
            &self.claude_gating,
            &merged_denies,
            &task.context.tool_deny_prefixes,
        );
        let tool_schemas = tool_surface::build_tool_schemas(&names, &tools);
        drop(tools);

        // 5. 多轮 tool loop（assistant → tool_calls → 执行 → tool_result）
        let llm_config = model_config_with_retry_observer(&model_config, logger.clone(), task.id);
        let mut total_tool_calls: usize = 0;
        let mut artifacts: Vec<Artifact> = vec![];
        let mut budget_state = RuntimeBudgetState::new(task.context.budget);
        let loop_limits = task.context.loop_limits;

        for turn in 1..=loop_limits.max_agent_turns {
            let turn_tool_schemas =
                tool_surface::schemas_for_model_turn(&tool_schemas, &model_config, turn);
            logger.line(
                task.id,
                &format!("[turn_start] turn={}/{}", turn, loop_limits.max_agent_turns),
            );
            if nested_coop_cancelled(&task.context) {
                logger.line(task.id, "[task_end] status=cancelled reason=cancelled");
                return Ok(task_cancelled_failure());
            }
            if tick_budget(&logger, task.id, &mut budget_state) {
                logger.line(task.id, "[task_end] status=failed reason=budget");
                return Ok(TaskResult::Failure {
                    error: "运行时预算已用尽".to_string(),
                    details: Some(TerminationReason::Budget.as_str().to_string()),
                });
            }
            self.sync_plan_tree_context(&mut messages);
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

            let _llm_activity =
                SessionActivityGuard::start(logger.clone(), task.id, ActivityReason::ApiCall);
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
                            logger.line(task.id, "[task_end] status=cancelled reason=cancelled");
                            return Ok(task_cancelled_failure());
                        }
                        res = self.chat_with_failover(
                            messages.clone(),
                            turn_tool_schemas.clone(),
                            &llm_config,
                            task.id,
                            &logger,
                        ) => res,
                    }
                }
                None => {
                    self.chat_with_failover(
                        messages.clone(),
                        turn_tool_schemas.clone(),
                        &llm_config,
                        task.id,
                        &logger,
                    )
                    .await
                }
            };

            let mut response = match response_result {
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
                    logger.line(task.id, "[task_end] status=failed reason=error");
                    return Ok(TaskResult::Failure {
                        error: "LLM 调用失败".to_string(),
                        details: Some(e.to_string()),
                    });
                }
            };

            let should_recover_no_tool = turn == 1
                && total_tool_calls == 0
                && response.tool_calls.is_empty()
                && anycode_llm::capabilities_for_model_config(&model_config).weak_local_model
                && !turn_tool_schemas.is_empty();
            if should_recover_no_tool {
                for attempt in 1..=2u8 {
                    logger.line(
                        task.id,
                        &format!(
                            "[tool_recovery] turn=1 attempt={} reason=no_tool_response",
                            attempt
                        ),
                    );
                    if record_llm_usage(&logger, task.id, &mut budget_state, &response.usage) {
                        logger.line(task.id, "[task_end] status=failed reason=budget");
                        return Ok(TaskResult::Failure {
                            error: "运行时预算已用尽".to_string(),
                            details: Some(TerminationReason::Budget.as_str().to_string()),
                        });
                    }
                    if messages
                        .last()
                        .is_none_or(|m| m.role != MessageRole::Assistant)
                    {
                        messages.push(response.message.clone());
                    }
                    messages.push(Message {
                        id: Uuid::new_v4(),
                        role: MessageRole::User,
                        content: MessageContent::Text(if attempt == 1 {
                            anycode_llm::TOOL_RECOVERY_NUDGE.to_string()
                        } else {
                            anycode_llm::TOOL_RECOVERY_NUDGE_FORCE_GLOB.to_string()
                        }),
                        timestamp: chrono::Utc::now(),
                        metadata: HashMap::new(),
                    });
                    response = match self
                        .chat_with_failover(
                            messages.clone(),
                            turn_tool_schemas.clone(),
                            &llm_config,
                            task.id,
                            &logger,
                        )
                        .await
                    {
                        Ok(response) => response,
                        Err(error) => {
                            logger.line(task.id, "[task_end] status=failed reason=error");
                            return Ok(TaskResult::Failure {
                                error: "LLM 工具恢复调用失败".to_string(),
                                details: Some(error.to_string()),
                            });
                        }
                    };
                    messages.push(response.message.clone());
                    if !response.tool_calls.is_empty() {
                        break;
                    }
                }
                if response.tool_calls.is_empty() {
                    logger.line(task.id, "[task_end] status=failed reason=refusal_no_tool");
                    return Ok(TaskResult::Failure {
                        error: "模型未按任务要求调用工具".to_string(),
                        details: Some(TerminationReason::RefusalNoTool.as_str().to_string()),
                    });
                }
            }

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
                logger.line(task.id, "[task_end] status=failed reason=budget");
                return Ok(TaskResult::Failure {
                    error: "运行时预算已用尽".to_string(),
                    details: Some(TerminationReason::Budget.as_str().to_string()),
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
            // Tool-recovery already appended this assistant message to history;
            // patch its metadata in place instead of pushing a duplicate (same
            // behavior as execute_turn).
            if messages.last().is_some_and(|m| m.id == assistant_msg.id) {
                if let Some(last) = messages.last_mut() {
                    last.metadata = assistant_msg.metadata.clone();
                }
            } else {
                messages.push(assistant_msg);
            }

            let session_label = task.context.session_id.to_string();
            let turn_plain = messages
                .last()
                .and_then(|m| match &m.content {
                    MessageContent::Text(t) => Some(strip_llm_reasoning_xml_blocks(t)),
                    _ => None,
                })
                .unwrap_or_default();
            if !turn_plain.trim().is_empty() && response.tool_calls.is_empty() {
                logger.assistant_response(task.id, turn, &turn_plain);
            }

            let mut turn_tool_calls = response.tool_calls.clone();
            if turn_tool_calls.is_empty() {
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
                    turn_tool_calls.len()
                ),
            );

            let tool_ctx = TurnToolCtx {
                task_id: task.id,
                agent_type: &task.agent_type,
                working_directory: task.context.working_directory.as_str(),
                session_label: &session_label,
                turn,
                loop_limits,
                live_trace_tx: task.context.live_trace_tx.clone(),
            };
            let mut tool_state = TurnToolState {
                total_tool_calls,
                artifacts: std::mem::take(&mut artifacts),
                budget_state: budget_state.clone(),
                progress_seq: 0,
            };
            let mut sink = MessageAppendSink::Vec(&mut messages);
            match self
                .dispatch_turn_tool_calls(
                    &logger,
                    &tool_ctx,
                    &mut tool_state,
                    &TurnToolCancel::Nested(&task.context),
                    &mut sink,
                    turn_tool_calls,
                    false,
                    TurnToolCancelOutcome::TaskCancelled,
                )
                .await?
            {
                TurnToolBatchOutcome::Ok => {}
                TurnToolBatchOutcome::Cancelled(out) => {
                    if let Some(result) = out.into_task_result() {
                        return Ok(result);
                    }
                }
                TurnToolBatchOutcome::MaxToolCalls => {
                    logger.line(task.id, "[task_end] status=failed reason=max_tools");
                    return Ok(TaskResult::Failure {
                        error: "达到最大工具调用次数，已停止".to_string(),
                        details: Some(format!(
                            "{} max_tool_calls={}",
                            TerminationReason::MaxTools.as_str(),
                            loop_limits.max_tool_calls
                        )),
                    });
                }
                TurnToolBatchOutcome::BudgetExceeded => {
                    logger.line(task.id, "[task_end] status=failed reason=budget");
                    return Ok(TaskResult::Failure {
                        error: "运行时预算已用尽".to_string(),
                        details: Some(TerminationReason::Budget.as_str().to_string()),
                    });
                }
            }
            total_tool_calls = tool_state.total_tool_calls;
            artifacts = tool_state.artifacts;
            budget_state = tool_state.budget_state;
            if turn < loop_limits.max_agent_turns {
                self.maybe_auto_compact_messages(
                    task.id,
                    &task.agent_type,
                    task.context.working_directory.as_str(),
                    &model_config,
                    &mut messages,
                    response.usage.input_tokens,
                    turn,
                )
                .await?;
            }
        }

        // 正常收尾：最后一跳无 tool_calls，故末条消息即本轮 assistant。打满 MAX_AGENT_TURNS 且末尾为 Tool 时不走此路径，保留 summary。
        let stopped_after_final_answer = messages
            .last()
            .is_some_and(|message| message.role == MessageRole::Assistant)
            && messages.last().is_some_and(|message| {
                !message
                    .metadata
                    .contains_key(ANYCODE_TOOL_CALLS_METADATA_KEY)
            });
        if stopped_after_final_answer {
            if let Some(fast) = last_assistant_plain_text(&messages) {
                logger.line(task.id, "[task_end] status=completed reason=completed");
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
        }

        logger.line(task.id, "[task_end] status=failed reason=max_turns");
        Ok(TaskResult::Failure {
            error: "达到最大模型轮次，任务未完成".to_string(),
            details: Some(format!(
                "{} max_agent_turns={}",
                TerminationReason::MaxTurns.as_str(),
                loop_limits.max_agent_turns
            )),
        })
    }
}
