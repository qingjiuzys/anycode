//! Task execution

use super::agentic_loop::{coop_flag_wait, nested_coop_cancelled, task_cancelled_failure};
use super::agentic_turn::{
    MessageAppendSink, NoToolRecovery, TurnToolBatchOutcome, TurnToolCancel, TurnToolCancelOutcome,
    TurnToolCtx, TurnToolDispatchKernel, TurnToolState,
};
use super::budget::{
    record_llm_usage, tick_budget, token_budget_context_section, RuntimeBudgetState,
};
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
        let live_trace_tx = task.context.live_trace_tx.clone();
        let result = if let Some(chat_turn) = task.context.chat_turn.clone() {
            anycode_core::scope_chat_turn(chat_turn, self.execute_task_inner(task)).await
        } else {
            self.execute_task_inner(task).await
        };
        // Step 3b：任务级终止事件覆盖所有返回分支（含提前 return）；
        // 嵌套子代理经 forwarder 包装转发，顶层任务无通道时 no-op。
        match &result {
            Ok(res) => super::live_trace_emit::emit_turn_done(&live_trace_tx, terminal_status(res)),
            Err(_) => super::live_trace_emit::emit_turn_done(&live_trace_tx, "error"),
        }
        result
    }

    async fn execute_task_inner(&self, task: Task) -> Result<TaskResult, CoreError> {
        let _parent_tool_surface = ParentToolSurfaceGuard::attach(
            self,
            task.id,
            task.context.tool_deny_names.clone(),
            task.context.tool_deny_prefixes.clone(),
            task.context.budget,
        );

        // Step 3b：任务级 live trace 通道注册到 ToolServices 键控 map，嵌套 Agent
        // 工具按父 task id 查找接线；drop-guard 在任务结束（含提前返回）注销。
        let _live_trace_guard = super::LiveTraceRegistrationGuard::register(
            self,
            task.id,
            task.context.live_trace_tx.clone(),
        );

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

        // 2. 加载相关记忆（分类型预算）+ TaskCompiler + Skill routing
        let compiled = super::compile_context::compile_for_prompt(
            self.memory_store.as_ref(),
            &self.tool_services,
            &task.prompt,
            task.agent_type.as_str(),
            task.context.working_directory.as_str(),
            true,
        )
        .await?;
        let arm = compiled.arm;
        let memories: Vec<Memory> = compiled
            .recalled
            .iter()
            .flat_map(|(_, v)| v.iter().cloned())
            .collect();
        let compiler_sections = compiled.sections.clone();
        let gate_plan = compiled.gate_plan.clone();
        let expected_artifacts = compiled.expected_artifacts.clone();
        let task_family = compiled.family;
        let skill_denies = compiled.skill_denies.clone();
        logger.line(
            task.id,
            &super::compile_context::delivery_preflight_marker(&compiled.parts),
        );
        if let Some(plan) = &gate_plan {
            logger.line(
                task.id,
                &super::compile_context::gate_plan_marker(
                    task_family,
                    plan.requirements.len(),
                    arm,
                ),
            );
        }
        if !compiled.parts.selected_skill_ids.is_empty() {
            logger.line(
                task.id,
                &super::compile_context::skill_resolved_marker(&compiled.parts.selected_skill_ids),
            );
        }
        // Prefer attributed sections over the flat legacy blob when we have typed recall.
        let memories_for_legacy = if compiler_sections
            .iter()
            .any(|s| s.starts_with("## Memories ("))
        {
            &[][..]
        } else {
            memories.as_slice()
        };
        let mut context_injections = task.context.context_injections.clone();
        context_injections.extend(compiler_sections);
        // auto-memory 召回：按项目的 MEMORY.md 索引（含截断与 point-in-time 告诫）。
        if let Some(section) = self.automem_index_section(
            task.agent_type.as_str(),
            task.context.working_directory.as_str(),
        ) {
            context_injections.push(section);
        }
        if let Some(section) = token_budget_context_section(&task.context.budget) {
            context_injections.push(section);
        }

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
                memories_for_legacy,
                &context_injections,
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
        let mut merged_denies = anycode_tools::merge_agent_type_tool_denies(
            task.agent_type.as_str(),
            &task.context.tool_deny_names,
        );
        // 嵌套子代理默认禁止工具集（对齐 Claude Code `ALL_AGENT_DISALLOWED_TOOLS`）：
        // 子代理不应切换/退出计划、向用户提问、自我终止、再嵌套 worktree 隔离。
        if task.context.system_prompt_append.as_deref()
            == Some(super::nested_task::SUBAGENT_SYSTEM_APPEND)
        {
            merged_denies.extend(anycode_tools::subagent_default_tool_denies());
        }
        merged_denies.extend(skill_denies);
        let names = tool_surface::prepare_tool_names_for_llm(
            raw,
            &self.tool_name_deny,
            &self.claude_gating,
            &merged_denies,
            &task.context.tool_deny_prefixes,
        );
        // 零工具面 fail-fast：deny 叠加（含 config profile / skill / 子代理默认）
        // 把工具面清空时直接失败，避免模型在无工具可用的循环里空转。
        if let Err(zero) =
            tool_surface::ensure_nonempty_tool_surface(&names, task.agent_type.as_str())
        {
            tracing::warn!(kind = "zero_tool_surface", agent_type = %task.agent_type.as_str(), "{zero}");
            logger.line(task.id, "[task_end] status=failed reason=zero_tools");
            return Ok(TaskResult::Failure {
                error: zero,
                details: Some("zero_tools".to_string()),
            });
        }
        let tool_schemas = tool_surface::build_tool_schemas(&names, &tools);
        drop(tools);

        // 5. 多轮 tool loop（assistant → tool_calls → 执行 → tool_result）
        let llm_config = model_config_with_retry_observer(&model_config, logger.clone(), task.id);
        let mut total_tool_calls: usize = 0;
        let mut used_tools: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut artifacts: Vec<Artifact> = vec![];
        let mut budget_state = RuntimeBudgetState::new(task.context.budget);
        let mut guard_state = super::guard_verdict::GuardLoopState::default();
        let verification_shared = Arc::new(std::sync::Mutex::new(
            super::discoverable_verification::SessionVerificationState::default(),
        ));
        let loop_limits = if task_family == Some(anycode_core::TaskFamily::OfficeDelivery) {
            let capped = task.context.loop_limits.for_office_delivery();
            if capped != task.context.loop_limits {
                logger.line(
                    task.id,
                    &format!(
                        "[office_loop_caps] turns={} tools={} (clamped from {}/{})",
                        capped.max_agent_turns,
                        capped.max_tool_calls,
                        task.context.loop_limits.max_agent_turns,
                        task.context.loop_limits.max_tool_calls
                    ),
                );
            }
            capped
        } else {
            task.context.loop_limits
        };
        #[cfg(feature = "harness-v1")]
        if self.harness_unified_kernel_enabled() {
            return self
                .execute_task_via_unified_kernel(
                    &task,
                    messages,
                    &names,
                    model_config,
                    loop_limits,
                    super::harness_unified::UnifiedLifecycleSpec {
                        family: task_family,
                        gate_plan,
                        expected: expected_artifacts,
                    },
                )
                .await;
        }
        // 预算 hard-stop 的统一失败形态（loop 头 tick、no-tool 恢复、用量记录三处共用）。
        let budget_failure = || TaskResult::Failure {
            error: "运行时预算已用尽".to_string(),
            details: Some(TerminationReason::Budget.as_str().to_string()),
        };

        for turn in 1..=loop_limits.max_agent_turns {
            let turn_tool_schemas = tool_surface::schemas_for_model_turn(
                &tool_schemas,
                &model_config,
                turn,
                &used_tools,
            );
            logger.line(
                task.id,
                &format!("[turn_start] turn={}/{}", turn, loop_limits.max_agent_turns),
            );
            super::live_trace_emit::emit_turn_start(&task.context.live_trace_tx, turn);
            if nested_coop_cancelled(&task.context) {
                logger.line(task.id, "[task_end] status=cancelled reason=cancelled");
                return Ok(task_cancelled_failure());
            }
            if tick_budget(&logger, task.id, &mut budget_state) {
                logger.line(task.id, "[task_end] status=failed reason=budget");
                return Ok(budget_failure());
            }
            self.sync_plan_tree_context(&mut messages).await;
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
            super::live_trace_emit::emit_llm_request_start(&task.context.live_trace_tx, turn);

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
                            &messages,
                            turn_tool_schemas.clone(),
                            &llm_config,
                            task.id,
                            &logger,
                        ) => res,
                    }
                }
                None => {
                    self.chat_with_failover(
                        &messages,
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
                let mut sink = MessageAppendSink::Vec(&mut messages);
                match self
                    .recover_no_tool_response(
                        &logger,
                        task.id,
                        response,
                        &turn_tool_schemas,
                        &llm_config,
                        &mut budget_state,
                        &mut sink,
                        None,
                    )
                    .await
                {
                    NoToolRecovery::Recovered(r) => response = r,
                    NoToolRecovery::BudgetExceeded => {
                        logger.line(task.id, "[task_end] status=failed reason=budget");
                        return Ok(budget_failure());
                    }
                    NoToolRecovery::LlmFailed(error) => {
                        logger.line(task.id, "[task_end] status=failed reason=error");
                        return Ok(TaskResult::Failure {
                            error: "LLM 工具恢复调用失败".to_string(),
                            details: Some(error.to_string()),
                        });
                    }
                    NoToolRecovery::Exhausted(_) => {
                        logger.line(task.id, "[task_end] status=failed reason=refusal_no_tool");
                        return Ok(TaskResult::Failure {
                            error: "模型未按任务要求调用工具".to_string(),
                            details: Some(TerminationReason::RefusalNoTool.as_str().to_string()),
                        });
                    }
                }
            }

            logger.line(
                task.id,
                &format!(
                    "[llm_response_end] turn={} elapsed_ms={} input_tokens={} output_tokens={} agent_type={}",
                    turn,
                    t0.elapsed().as_millis(),
                    response.usage.input_tokens,
                    response.usage.output_tokens,
                    task.agent_type.as_str()
                ),
            );
            if record_llm_usage(&logger, task.id, &mut budget_state, &response.usage) {
                logger.line(task.id, "[task_end] status=failed reason=budget");
                return Ok(budget_failure());
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
                super::live_trace_emit::emit_assistant_done(
                    &task.context.live_trace_tx,
                    turn,
                    &turn_plain,
                );
            }

            let turn_tool_calls = response.tool_calls.clone();
            used_tools.extend(turn_tool_calls.iter().map(|tc| tc.name.clone()));
            if turn_tool_calls.is_empty() {
                let guard_input = super::guard_verdict::GuardEvalInput {
                    task_id: task.id,
                    agent_type: &task.agent_type,
                    working_directory: task.context.working_directory.as_str(),
                    session_label: &session_label,
                    turn,
                    task_family,
                    gate_plan: gate_plan.as_ref(),
                    expected_artifacts: &expected_artifacts,
                    artifacts: &artifacts,
                    assistant_text: &turn_plain,
                    task_prompt: &task.prompt,
                    live_trace_tx: &task.context.live_trace_tx,
                    verification: &verification_shared,
                    progress_seq: 0,
                    turn_style_verify_markers: false,
                };
                let mut sink = MessageAppendSink::Vec(&mut messages);
                match self
                    .evaluate_completion_guard(&logger, &guard_input, &mut guard_state, &mut sink)
                    .await
                {
                    super::guard_verdict::GuardVerdict::Completed => break,
                    super::guard_verdict::GuardVerdict::RepairInjected => continue,
                    super::guard_verdict::GuardVerdict::Partial { repair_message } => {
                        logger.line(task.id, "[task_end] status=partial reason=verification");
                        return Ok(TaskResult::Partial {
                            success: turn_plain,
                            remaining: repair_message
                                .unwrap_or_else(|| "verification incomplete".into()),
                        });
                    }
                    super::guard_verdict::GuardVerdict::Failed { repair_message } => {
                        logger.line(task.id, "[task_end] status=failed reason=verification");
                        return Ok(TaskResult::Failure {
                            error: "verification gates failed".into(),
                            details: repair_message,
                        });
                    }
                }
            }

            let tool_ctx = TurnToolCtx {
                task_id: task.id,
                agent_type: &task.agent_type,
                working_directory: task.context.working_directory.as_str(),
                session_label: &session_label,
                turn,
                loop_limits,
                live_trace_tx: task.context.live_trace_tx.clone(),
                verification: Some(Arc::clone(&verification_shared)),
            };
            let mut tool_state = TurnToolState {
                total_tool_calls,
                artifacts: std::mem::take(&mut artifacts),
                budget_state: budget_state.clone(),
                progress_seq: 0,
                checked_deliverables: std::collections::HashSet::new(),
                sandbox_escape_streak: 0,
                last_sandbox_escape_key: None,
            };
            let mut sink = MessageAppendSink::Vec(&mut messages);
            match self
                .run_turn_tool_dispatch_kernel(
                    &logger,
                    &mut tool_state,
                    &mut sink,
                    TurnToolDispatchKernel {
                        tool_ctx,
                        cancel: TurnToolCancel::Nested(&task.context),
                        turn_tool_calls,
                        record_evidence: false,
                        cancel_outcome: TurnToolCancelOutcome::TaskCancelled,
                    },
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
                // automem fork 的产物不写项目 autosave（避免记忆代理自身污染记忆）。
                if !super::automem::is_automem_agent_type(task.agent_type.as_str()) {
                    self.maybe_autosave_memory(task.id, &task.prompt, &fast)
                        .await;
                }
                self.maybe_automem_after_turn(
                    task.agent_type.as_str(),
                    task.context.working_directory.as_str(),
                    task.context.system_prompt_append.as_deref(),
                    task.id,
                    &messages,
                );
                return Ok(TaskResult::Success {
                    output: fast,
                    artifacts,
                });
            }
        }

        // A repair request consumed the final turn: the true cause is the
        // failed verification, not the turn budget — surface the diagnostics.
        if let Some(diag) = guard_state.last_repair_diagnostics {
            logger.line(task.id, "[task_end] status=failed reason=verification");
            return Ok(TaskResult::Failure {
                error: "验证未通过且修复轮次已用尽".to_string(),
                details: Some(diag),
            });
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

/// 任务终态 → live trace `turn_done.status` 的稳定取值。
fn terminal_status(result: &TaskResult) -> &'static str {
    match result {
        TaskResult::Success { .. } => "completed",
        TaskResult::Partial { .. } => "partial",
        TaskResult::Failure { .. } => "failed",
    }
}
