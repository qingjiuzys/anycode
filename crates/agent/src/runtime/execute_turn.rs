//! Continuous session turn execution

use super::agentic_loop::{
    coop_flag_wait_opt, opt_coop_cancelled, pop_assistant_placeholder,
    rehydrate_stream_llm_response,
};
use super::agentic_turn::{
    MessageAppendSink, TurnToolBatchOutcome, TurnToolCancel, TurnToolCancelOutcome, TurnToolCtx,
    TurnToolState,
};
use super::budget::{
    record_llm_usage, tick_budget, token_budget_context_section, RuntimeBudgetState,
};
use super::execute_turn_finalize::TurnFinalizeParams;
use super::live_trace_emit;
use super::llm_retry::model_config_with_retry_observer;
use super::memory_hooks;
use super::progress_update;
use super::provider_errors::{
    core_error_is_context_overflow, error_indicates_context_overflow,
    provider_error_from_streamed_assistant_text,
};
use super::session_activity::{ActivityReason, SessionActivityGuard};
use super::tool_surface;
use super::AgentRuntime;
use anycode_core::prelude::*;
use anycode_core::strip_llm_reasoning_for_display;
use anycode_core::Artifact;
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::Mutex;
use uuid::Uuid;

impl AgentRuntime {
    /// 执行一次“连续会话”的 agentic turn：从传入的 `messages` 继续跑同一轮工具循环，
    /// 并在结束后返回 `TurnOutput`：最终 assistant 文本、artifacts、以及聚合的 `TurnTokenUsage`（max input、sum output、cache 累计等）。
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
        coop_cancel: Option<Arc<AtomicBool>>,
        tool_deny_names: &[String],
        tool_deny_prefixes: &[String],
        budget: TaskBudget,
        loop_limits: AgentLoopLimits,
        live_trace_tx: Option<UnboundedSender<LiveTraceEvent>>,
    ) -> Result<TurnOutput, CoreError> {
        let logger = self.logger();
        logger.ensure_initialized(task_id);
        logger.line(
            task_id,
            &format!("[task_start] agent_type={}", agent_type.as_str()),
        );

        // 1) 工具名与 schema（与 `execute_task` 共用 tool_surface，避免漂移）
        let agent_tools = {
            let agents = self.agents.read().await;
            let canonical = super::canonical_agent_type(agent_type);
            let agent = agents
                .get(&canonical)
                .or_else(|| agents.get(agent_type))
                .ok_or_else(|| CoreError::AgentNotFound(Uuid::new_v4()))?;
            agent.tools()
        };

        let tools = self.tools.read().await;
        let raw = tool_surface::resolve_agent_tool_names(agent_type.as_str(), agent_tools, &tools);
        let mut merged_denies =
            anycode_tools::merge_agent_type_tool_denies(agent_type.as_str(), tool_deny_names);
        // Placeholder; skill arm denies applied after compile below.
        let names = tool_surface::prepare_tool_names_for_llm(
            raw.clone(),
            &self.tool_name_deny,
            &self.claude_gating,
            &merged_denies,
            tool_deny_prefixes,
        );
        let mut tool_schemas = tool_surface::build_tool_schemas(&names, &tools);
        drop(tools);

        // Inject TaskCompiler / Experience / Skill routing once per turn (latest user prompt).
        let (gate_plan, expected_artifacts, task_family) = {
            let prompt = {
                let g = messages.lock().await;
                g.iter()
                    .rev()
                    .find(|m| {
                        m.role == MessageRole::User
                            && !m
                                .metadata
                                .get(ANYCODE_CONTEXT_USER_METADATA_KEY)
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false)
                    })
                    .and_then(|m| match &m.content {
                        MessageContent::Text(t) => Some(t.clone()),
                        _ => None,
                    })
                    .unwrap_or_default()
            };
            if prompt.trim().is_empty() {
                (None, Vec::new(), None)
            } else {
                let compiled = super::compile_context::compile_for_prompt(
                    self.memory_store.as_ref(),
                    &self.tool_services,
                    &prompt,
                    agent_type.as_str(),
                    working_directory,
                    false,
                )
                .await
                // non-strict mode swallows recall errors internally — infallible here
                .unwrap_or_else(|e| unreachable!("non-strict compile must not fail: {e}"));
                let arm = compiled.arm;
                if !compiled.skill_denies.is_empty() {
                    logger.line(
                        task_id,
                        &format!(
                            "[skill_tools_denied] tools={} arm=exp:{}:skills:{}",
                            compiled.skill_denies.join(","),
                            arm.experience_enabled as u8,
                            arm.production_skills_enabled as u8
                        ),
                    );
                    merged_denies.extend(compiled.skill_denies.iter().cloned());
                    let tools = self.tools.read().await;
                    let names = tool_surface::prepare_tool_names_for_llm(
                        raw,
                        &self.tool_name_deny,
                        &self.claude_gating,
                        &merged_denies,
                        tool_deny_prefixes,
                    );
                    tool_schemas = tool_surface::build_tool_schemas(&names, &tools);
                    drop(tools);
                }
                let mut sections = compiled.sections.clone();
                if let Some(section) = token_budget_context_section(&budget) {
                    sections.push(section);
                }
                {
                    let preflight =
                        super::compile_context::delivery_preflight_marker(&compiled.parts);
                    logger.line(task_id, &preflight);
                    live_trace_emit::try_emit(
                        &live_trace_tx,
                        LiveTraceEvent::ProgressUpdate {
                            turn: 0,
                            seq: 0,
                            phase: "intent".into(),
                            work_stage: Some("compile".into()),
                            summary: preflight,
                            next: None,
                            discovery: None,
                            evidence_refs: compiled.parts.selected_skill_ids.clone(),
                        },
                    );
                }
                if let Some(plan) = &compiled.gate_plan {
                    let marker = super::compile_context::gate_plan_marker(
                        compiled.family,
                        plan.requirements.len(),
                        arm,
                    );
                    logger.line(task_id, &marker);
                    live_trace_emit::try_emit(
                        &live_trace_tx,
                        LiveTraceEvent::ProgressUpdate {
                            turn: 0,
                            seq: 1,
                            phase: "gate".into(),
                            work_stage: Some("compile".into()),
                            summary: marker,
                            next: None,
                            discovery: None,
                            evidence_refs: vec![],
                        },
                    );
                }
                if !compiled.parts.selected_skill_ids.is_empty() {
                    let marker = super::compile_context::skill_resolved_marker(
                        &compiled.parts.selected_skill_ids,
                    );
                    logger.line(task_id, &marker);
                    live_trace_emit::try_emit(
                        &live_trace_tx,
                        LiveTraceEvent::ProgressUpdate {
                            turn: 0,
                            seq: 2,
                            phase: "skill".into(),
                            work_stage: Some("compile".into()),
                            summary: marker,
                            next: None,
                            discovery: None,
                            evidence_refs: compiled.parts.selected_skill_ids.clone(),
                        },
                    );
                }
                {
                    let mut g = messages.lock().await;
                    // Drop prior compiler context injections for this turn boundary.
                    g.retain(|m| {
                        !(m.role == MessageRole::User
                            && m.metadata
                                .get(ANYCODE_CONTEXT_USER_METADATA_KEY)
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false)
                            && match &m.content {
                                MessageContent::Text(t) => {
                                    t.starts_with("## Task Spec")
                                        || t.starts_with("## Experience Pack")
                                        || t.starts_with("## Selected Skills")
                                        || t.starts_with("## Gate Plan")
                                        || t.starts_with("## Preferences")
                                        || t.starts_with("## Memories (")
                                }
                                _ => false,
                            })
                    });
                    for section in sections {
                        let mut metadata = HashMap::new();
                        metadata.insert(
                            ANYCODE_CONTEXT_USER_METADATA_KEY.to_string(),
                            serde_json::Value::Bool(true),
                        );
                        g.push(Message {
                            id: Uuid::new_v4(),
                            role: MessageRole::User,
                            content: MessageContent::Text(section),
                            timestamp: chrono::Utc::now(),
                            metadata,
                        });
                    }
                }
                (
                    compiled.gate_plan,
                    compiled.expected_artifacts,
                    compiled.family,
                )
            }
        };
        let mut repairs_used: u32 = 0;
        let mut last_repair_diagnostics: Option<String> = None;
        let verification_shared = Arc::new(std::sync::Mutex::new(
            super::discoverable_verification::SessionVerificationState::default(),
        ));
        let mut evidence_repairs_used: u32 = 0;

        // 2) agentic loop：保持与 execute_task 的语义一致
        let model_config = self.model_for_task(agent_type).clone();
        let llm_config = model_config_with_retry_observer(&model_config, logger.clone(), task_id);
        let mut total_tool_calls: usize = 0;
        let mut used_tools: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut artifacts: Vec<Artifact> = vec![];
        let mut last_assistant_text = String::new();
        let mut turn_usage = TurnTokenUsage::default();
        let mut last_model_turn: usize = 1;
        let mut budget_state = RuntimeBudgetState::new(budget);
        let mut termination_reason = TerminationReason::MaxTurns;
        let mut progress_seq: u32 = 0;
        let mut stream_progress_seq: Option<u32>;

        for turn in 1..=loop_limits.max_agent_turns {
            last_model_turn = turn;
            stream_progress_seq = None;
            let turn_tool_schemas = tool_surface::schemas_for_model_turn(
                &tool_schemas,
                &model_config,
                turn,
                &used_tools,
            );
            logger.line(
                task_id,
                &format!("[turn_start] turn={}/{}", turn, loop_limits.max_agent_turns),
            );
            live_trace_emit::emit_turn_start(&live_trace_tx, turn);
            if opt_coop_cancelled(&coop_cancel) {
                self.emit_cancelled_turn_receipt(TurnFinalizeParams {
                    task_id,
                    agent_type,
                    messages: &messages,
                    working_directory,
                    live_trace_tx: &live_trace_tx,
                    loop_limits,
                    last_model_turn: turn.saturating_sub(1).max(1),
                    total_tool_calls,
                    artifacts: artifacts.clone(),
                    turn_usage,
                    termination_reason: TerminationReason::Cancelled,
                })
                .await;
                return Err(CoreError::CooperativeCancel);
            }
            if tick_budget(&logger, task_id, &mut budget_state) {
                return Ok(self
                    .finalize_incomplete_turn(TurnFinalizeParams {
                        task_id,
                        agent_type,
                        messages: &messages,
                        working_directory,
                        live_trace_tx: &live_trace_tx,
                        loop_limits,
                        last_model_turn: turn.saturating_sub(1).max(1),
                        total_tool_calls,
                        artifacts,
                        turn_usage,
                        termination_reason: TerminationReason::Budget,
                    })
                    .await);
            }
            {
                let mut g = messages.lock().await;
                self.sync_plan_tree_context(&mut g).await;
            }
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
            live_trace_emit::emit_llm_request_start(&live_trace_tx, turn);

            let mut overflow_retried_this_turn = false;
            let llm_t0 = std::time::Instant::now();
            let _llm_activity =
                SessionActivityGuard::start(logger.clone(), task_id, ActivityReason::ApiCall);
            let (mut response, mut llm_streamed) = 'llm_attempt: loop {
                let messages_snapshot = {
                    let g = messages.lock().await;
                    crate::reply_language::inject_ephemeral_reply_language_reminder(&g)
                };
                // Prefer streaming: TUI can render deltas incrementally via shared `messages`.
                // Fallback to non-stream chat if streaming is not supported / fails.
                let mut tool_calls: Vec<ToolCall> = vec![];
                let mut streamed = false;
                let assistant_id = Uuid::new_v4();
                let mut stream_usage: Option<Usage> = None;

                // Insert an empty assistant message first so UI can show deltas as they arrive.
                {
                    let mut g = messages.lock().await;
                    g.push(Message {
                        id: assistant_id,
                        role: MessageRole::Assistant,
                        content: MessageContent::Text(String::new()),
                        timestamp: chrono::Utc::now(),
                        metadata: HashMap::new(),
                    });
                }

                let stream_open = self.llm_client.chat_stream(
                    messages_snapshot.clone(),
                    turn_tool_schemas.clone(),
                    &llm_config,
                );
                let stream_open = tokio::select! {
                    biased;
                    () = coop_flag_wait_opt(coop_cancel.clone()) => {
                        pop_assistant_placeholder(&messages, assistant_id).await;
                        logger.line(
                            task_id,
                            "[llm_response_end] status=cancelled reason=cooperative_in_flight",
                        );
                        self.emit_cancelled_turn_receipt(TurnFinalizeParams {
                            task_id,
                            agent_type,
                            messages: &messages,
                            working_directory,
                            live_trace_tx: &live_trace_tx,
                            loop_limits,
                            last_model_turn: turn,
                            total_tool_calls,
                            artifacts: artifacts.clone(),
                            turn_usage,
                            termination_reason: TerminationReason::Cancelled,
                        })
                        .await;
                        return Err(CoreError::CooperativeCancel);
                    }
                    r = stream_open => r,
                };

                if let Ok(mut rx) = stream_open {
                    streamed = true;
                    let mut received_any = false;
                    let mut stream_cancelled = false;
                    let mut turn_has_tool_calls = false;
                    loop {
                        tokio::select! {
                            biased;
                            () = coop_flag_wait_opt(coop_cancel.clone()) => {
                                stream_cancelled = true;
                                break;
                            }
                            ev = rx.recv() => {
                                match ev {
                                    None => break,
                                    Some(ev) => match ev {
                                        StreamEvent::Delta(d) => {
                                            if !d.is_empty() {
                                                received_any = true;
                                                live_trace_emit::emit_assistant_delta(
                                                    &live_trace_tx,
                                                    turn,
                                                    &d,
                                                    turn_has_tool_calls,
                                                );
                                                let mut g = messages.lock().await;
                                                if let Some(last) = g.last_mut() {
                                                    if last.id == assistant_id {
                                                        if let MessageContent::Text(t) = &mut last.content {
                                                            t.push_str(&d);
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        StreamEvent::Reasoning(r) => {
                                            if !r.trim().is_empty() {
                                                received_any = true;
                                                live_trace_emit::emit_thinking_delta(
                                                    &live_trace_tx,
                                                    turn,
                                                    &r,
                                                );
                                                let mut g = messages.lock().await;
                                                if let Some(last) = g.last_mut() {
                                                    if last.id == assistant_id {
                                                        last.metadata.insert(
                                                            ANYCODE_REASONING_CONTENT_METADATA_KEY
                                                                .to_string(),
                                                            serde_json::Value::String(r),
                                                        );
                                                    }
                                                }
                                            }
                                        }
                                        StreamEvent::ToolCall(tc) => {
                                            received_any = true;
                                            if !turn_has_tool_calls {
                                                turn_has_tool_calls = true;
                                                live_trace_emit::emit_assistant_narration_mark(
                                                    &live_trace_tx,
                                                    turn,
                                                );
                                                progress_seq += 1;
                                                stream_progress_seq = Some(progress_seq);
                                                let tool_names = vec![tc.name.clone()];
                                                let evt = progress_update::build_tool_round_progress(
                                                    turn as u32,
                                                    progress_seq,
                                                    "",
                                                    &tool_names,
                                                    turn as u32,
                                                    &[1],
                                                    turn == 1,
                                                );
                                                live_trace_emit::emit_progress_update(
                                                    &live_trace_tx,
                                                    evt,
                                                );
                                            }
                                            tool_calls.push(tc)
                                        }
                                        StreamEvent::Usage(u) => {
                                            received_any = true;
                                            stream_usage = Some(u);
                                        }
                                        StreamEvent::Done => break,
                                    },
                                }
                            }
                        }
                    }
                    if stream_cancelled {
                        pop_assistant_placeholder(&messages, assistant_id).await;
                        logger.line(
                            task_id,
                            "[llm_response_end] status=cancelled reason=cooperative_in_flight",
                        );
                        self.emit_cancelled_turn_receipt(TurnFinalizeParams {
                            task_id,
                            agent_type,
                            messages: &messages,
                            working_directory,
                            live_trace_tx: &live_trace_tx,
                            loop_limits,
                            last_model_turn: turn,
                            total_tool_calls,
                            artifacts: artifacts.clone(),
                            turn_usage,
                            termination_reason: TerminationReason::Cancelled,
                        })
                        .await;
                        return Err(CoreError::CooperativeCancel);
                    }
                    if !received_any {
                        streamed = false;
                    }
                }

                // If streaming didn't work, do the normal one-shot request and replace the placeholder assistant message.
                let response = if streamed {
                    rehydrate_stream_llm_response(
                        &messages,
                        assistant_id,
                        tool_calls,
                        stream_usage,
                        &messages_snapshot,
                    )
                    .await
                } else {
                    // Stream did not produce a final message: drop placeholder before non-stream
                    // chat so we never leave a stale assistant row (OpenClaw 5.19 failover parity).
                    pop_assistant_placeholder(&messages, assistant_id).await;
                    let chat_fut = self.chat_with_failover(
                        &messages_snapshot,
                        turn_tool_schemas.clone(),
                        &llm_config,
                        task_id,
                        &logger,
                    );
                    let r = tokio::select! {
                        biased;
                        () = coop_flag_wait_opt(coop_cancel.clone()) => {
                            logger.line(
                                task_id,
                                "[llm_response_end] status=cancelled reason=cooperative_in_flight",
                            );
                            self.emit_cancelled_turn_receipt(TurnFinalizeParams {
                                task_id,
                                agent_type,
                                messages: &messages,
                                working_directory,
                                live_trace_tx: &live_trace_tx,
                                loop_limits,
                                last_model_turn: turn,
                                total_tool_calls,
                                artifacts: artifacts.clone(),
                                turn_usage,
                                termination_reason: TerminationReason::Cancelled,
                            })
                            .await;
                            return Err(CoreError::CooperativeCancel);
                        }
                        res = chat_fut => match res {
                            Ok(r) => r,
                            Err(e) if !overflow_retried_this_turn && core_error_is_context_overflow(&e) => {
                                overflow_retried_this_turn = true;
                                self.recover_from_context_overflow(
                                    task_id,
                                    agent_type,
                                    working_directory,
                                    &messages,
                                )
                                .await?;
                                continue 'llm_attempt;
                            }
                            Err(e) => return Err(e),
                        },
                    };
                    {
                        let mut g = messages.lock().await;
                        g.push(r.message.clone());
                    }
                    r
                };

                let raw_assistant_probe = match &response.message.content {
                    MessageContent::Text(t) => t.as_str(),
                    _ => "",
                };
                let text_probe = strip_llm_reasoning_for_display(raw_assistant_probe);
                if response.tool_calls.is_empty() {
                    if let Some(err) = provider_error_from_streamed_assistant_text(&text_probe)
                        .or_else(|| {
                            provider_error_from_streamed_assistant_text(raw_assistant_probe)
                        })
                    {
                        if !overflow_retried_this_turn && error_indicates_context_overflow(&err) {
                            overflow_retried_this_turn = true;
                            {
                                let mut g = messages.lock().await;
                                if g.last().is_some_and(|m| m.role == MessageRole::Assistant) {
                                    g.pop();
                                }
                            }
                            self.recover_from_context_overflow(
                                task_id,
                                agent_type,
                                working_directory,
                                &messages,
                            )
                            .await?;
                            continue 'llm_attempt;
                        }
                        logger.line(
                            task_id,
                            &format!(
                                "[llm_response_end] status=stream_error_as_body turn={} detail={}",
                                turn, err
                            ),
                        );
                        if let Ok(Some(fb)) = self
                            .try_failover_on_provider_body_error(
                                &messages_snapshot,
                                turn_tool_schemas.clone(),
                                &llm_config,
                                task_id,
                                &logger,
                                &err,
                            )
                            .await
                        {
                            {
                                let mut g = messages.lock().await;
                                if g.last().is_some_and(|m| m.role == MessageRole::Assistant) {
                                    g.pop();
                                }
                                g.push(fb.message.clone());
                            }
                            break (fb, false);
                        }
                        {
                            let mut g = messages.lock().await;
                            if g.last().is_some_and(|m| m.role == MessageRole::Assistant) {
                                g.pop();
                            }
                        }
                        logger.line(task_id, "[task_end] status=failed reason=error");
                        return Err(CoreError::LLMError(err));
                    }
                }
                break (response, streamed);
            };

            let should_recover_no_tool = turn == 1
                && total_tool_calls == 0
                && response.tool_calls.is_empty()
                && anycode_llm::capabilities_for_model_config(&model_config).weak_local_model
                && !turn_tool_schemas.is_empty();
            if should_recover_no_tool {
                for attempt in 1..=2u8 {
                    logger.line(
                        task_id,
                        &format!(
                            "[tool_recovery] turn=1 attempt={} reason=no_tool_response",
                            attempt
                        ),
                    );
                    turn_usage.max_input_tokens =
                        turn_usage.max_input_tokens.max(response.usage.input_tokens);
                    turn_usage.total_output_tokens += response.usage.output_tokens;
                    if record_llm_usage(&logger, task_id, &mut budget_state, &response.usage) {
                        return Ok(self
                            .finalize_incomplete_turn(TurnFinalizeParams {
                                task_id,
                                agent_type,
                                messages: &messages,
                                working_directory,
                                live_trace_tx: &live_trace_tx,
                                loop_limits,
                                last_model_turn: turn,
                                total_tool_calls,
                                artifacts,
                                turn_usage,
                                termination_reason: TerminationReason::Budget,
                            })
                            .await);
                    }
                    {
                        let mut history = messages.lock().await;
                        if history
                            .last()
                            .is_none_or(|m| m.role != MessageRole::Assistant)
                        {
                            history.push(response.message.clone());
                        }
                        history.push(Message {
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
                    }
                    let snapshot = messages.lock().await.clone();
                    response = self
                        .chat_with_failover(
                            &snapshot,
                            turn_tool_schemas.clone(),
                            &llm_config,
                            task_id,
                            &logger,
                        )
                        .await?;
                    llm_streamed = false;
                    messages.lock().await.push(response.message.clone());
                    if !response.tool_calls.is_empty() {
                        break;
                    }
                }
                if response.tool_calls.is_empty() {
                    let refusal = match &response.message.content {
                        MessageContent::Text(text) => text.clone(),
                        _ => String::new(),
                    };
                    logger.line(task_id, "[task_end] status=failed reason=refusal_no_tool");
                    live_trace_emit::emit_turn_done(&live_trace_tx, "refusal_no_tool");
                    return Ok(TurnOutput {
                        final_text: refusal,
                        artifacts,
                        usage: turn_usage,
                        termination_reason: TerminationReason::RefusalNoTool,
                    });
                }
            }

            turn_usage.max_input_tokens =
                turn_usage.max_input_tokens.max(response.usage.input_tokens);
            turn_usage.total_output_tokens += response.usage.output_tokens;
            turn_usage.total_cache_read_tokens += response.usage.cache_read_tokens.unwrap_or(0);
            turn_usage.total_cache_creation_tokens +=
                response.usage.cache_creation_tokens.unwrap_or(0);

            logger.line(
                task_id,
                &format!(
                    "[llm_response_end] turn={} elapsed_ms={} input_tokens={} output_tokens={} streamed={}",
                    turn,
                    llm_t0.elapsed().as_millis(),
                    response.usage.input_tokens,
                    response.usage.output_tokens,
                    llm_streamed
                ),
            );
            if record_llm_usage(&logger, task_id, &mut budget_state, &response.usage) {
                return Ok(self
                    .finalize_incomplete_turn(TurnFinalizeParams {
                        task_id,
                        agent_type,
                        messages: &messages,
                        working_directory,
                        live_trace_tx: &live_trace_tx,
                        loop_limits,
                        last_model_turn: turn,
                        total_tool_calls,
                        artifacts,
                        turn_usage,
                        termination_reason: TerminationReason::Budget,
                    })
                    .await);
            }

            // 若本轮有 tool_calls，写入 metadata 供 OpenAI 兼容 provider 重建历史
            let mut assistant_msg = response.message.clone();
            if !response.tool_calls.is_empty() {
                assistant_msg.metadata.insert(
                    ANYCODE_TOOL_CALLS_METADATA_KEY.to_string(),
                    serde_json::to_value(&response.tool_calls)?,
                );
                // Also update the in-place message in history with metadata.
                let mut g = messages.lock().await;
                if let Some(last) = g.last_mut() {
                    if last.id == assistant_msg.id {
                        last.metadata = assistant_msg.metadata.clone();
                    }
                }
            }

            // 保留「最后一条非空」正文：部分 API 在收尾会再给一条空 assistant，避免覆盖掉仍应作为 turn 摘要的上一段文字。
            let raw_assistant = match &assistant_msg.content {
                MessageContent::Text(t) => t.as_str(),
                _ => "",
            };
            let text = strip_llm_reasoning_for_display(raw_assistant);
            if !response.tool_calls.is_empty() {
                live_trace_emit::emit_assistant_narration_mark(&live_trace_tx, turn);
                let tool_names: Vec<String> = response
                    .tool_calls
                    .iter()
                    .map(|tc| tc.name.clone())
                    .collect();
                let indices: Vec<u32> = (1..=response.tool_calls.len() as u32).collect();
                let prefer_intent = turn == 1 && stream_progress_seq.is_none() && progress_seq == 0;
                let seq = stream_progress_seq.unwrap_or_else(|| {
                    progress_seq += 1;
                    progress_seq
                });
                if !text.trim().is_empty() {
                    last_assistant_text = text.clone();
                }
                let progress_evt = progress_update::build_tool_round_progress(
                    turn as u32,
                    seq,
                    &text,
                    &tool_names,
                    turn as u32,
                    &indices,
                    prefer_intent,
                );
                live_trace_emit::emit_progress_update(&live_trace_tx, progress_evt);
            } else if !text.trim().is_empty() {
                last_assistant_text = text.clone();
                live_trace_emit::emit_assistant_done(&live_trace_tx, turn, &text);
                logger.assistant_response(task_id, turn, &text);
            }
            // If we streamed, assistant message is already in `messages`; no need to push again.
            // If we didn't stream, we already replaced placeholder with `r.message` above.

            let session_label = format!("tui_{}", task_id);

            let turn_tool_calls = response.tool_calls.clone();
            used_tools.extend(turn_tool_calls.iter().map(|tc| tc.name.clone()));
            if turn_tool_calls.is_empty() {
                let guard_out = self
                    .completion_guard
                    .evaluate(
                        &task_id.to_string(),
                        task_family,
                        gate_plan.as_ref(),
                        &expected_artifacts,
                        &artifacts,
                        std::path::Path::new(working_directory),
                        repairs_used,
                        last_repair_diagnostics.as_deref(),
                    )
                    .await;
                match guard_out.decision {
                    super::completion_guard::GuardDecision::Complete => {
                        let verification_snapshot = verification_shared
                            .lock()
                            .map(|g| g.clone())
                            .unwrap_or_default();
                        if let Some(msg) = super::discoverable_verification::maybe_evidence_repair(
                            &verification_snapshot,
                            &last_assistant_text,
                            evidence_repairs_used,
                        ) {
                            evidence_repairs_used += 1;
                            last_repair_diagnostics = Some(msg.clone());
                            let marker = format!(
                                "[evidence_repair_requested] repairs_used={evidence_repairs_used}"
                            );
                            logger.line(task_id, &marker);
                            live_trace_emit::try_emit(
                                &live_trace_tx,
                                LiveTraceEvent::ProgressUpdate {
                                    turn: turn as u32,
                                    seq: progress_seq.saturating_add(1),
                                    phase: "verify".into(),
                                    work_stage: Some("discover".into()),
                                    summary: marker,
                                    next: Some("discover and run official verification".into()),
                                    discovery: None,
                                    evidence_refs: vec![],
                                },
                            );
                            let mut g = messages.lock().await;
                            let mut metadata = HashMap::new();
                            metadata.insert(
                                ANYCODE_CONTEXT_USER_METADATA_KEY.to_string(),
                                serde_json::Value::Bool(true),
                            );
                            g.push(Message {
                                id: Uuid::new_v4(),
                                role: MessageRole::User,
                                content: MessageContent::Text(msg),
                                timestamp: chrono::Utc::now(),
                                metadata,
                            });
                            continue;
                        }
                        let marker = format!(
                            "[verification_finished] passed=1 results={}",
                            guard_out
                                .report
                                .as_ref()
                                .map(|r| r.results.len())
                                .unwrap_or(0)
                        );
                        logger.line(task_id, &marker);
                        live_trace_emit::try_emit(
                            &live_trace_tx,
                            LiveTraceEvent::ProgressUpdate {
                                turn: turn as u32,
                                seq: progress_seq.saturating_add(1),
                                phase: "verify".into(),
                                work_stage: Some("complete".into()),
                                summary: marker,
                                next: None,
                                discovery: None,
                                evidence_refs: vec![],
                            },
                        );
                        termination_reason = TerminationReason::Completed;
                        self.pipeline_memory_hook_agent_turn(
                            &session_label,
                            task_id,
                            turn,
                            &last_assistant_text,
                        )
                        .await;
                        self.maybe_session_notify_agent_turn(
                            &session_label,
                            task_id,
                            turn,
                            &last_assistant_text,
                            Some(working_directory),
                        );
                        logger.line(task_id, &format!("[turn_end] turn={} tool_calls=0", turn));
                        break;
                    }
                    super::completion_guard::GuardDecision::Repair => {
                        let msg = guard_out.repair_message.unwrap_or_default();
                        last_repair_diagnostics = Some(msg.clone());
                        repairs_used += 1;
                        let marker = format!(
                            "[repair_requested] repairs_used={repairs_used} verification_started=1"
                        );
                        logger.line(task_id, &marker);
                        live_trace_emit::try_emit(
                            &live_trace_tx,
                            LiveTraceEvent::ProgressUpdate {
                                turn: turn as u32,
                                seq: progress_seq.saturating_add(1),
                                phase: "verify".into(),
                                work_stage: Some("repair".into()),
                                summary: marker,
                                next: Some("fix gate failures then re-check".into()),
                                discovery: None,
                                evidence_refs: vec![],
                            },
                        );
                        let mut g = messages.lock().await;
                        let mut metadata = HashMap::new();
                        metadata.insert(
                            ANYCODE_CONTEXT_USER_METADATA_KEY.to_string(),
                            serde_json::Value::Bool(true),
                        );
                        g.push(Message {
                            id: Uuid::new_v4(),
                            role: MessageRole::User,
                            content: MessageContent::Text(msg),
                            timestamp: chrono::Utc::now(),
                            metadata,
                        });
                        continue;
                    }
                    super::completion_guard::GuardDecision::Partial
                    | super::completion_guard::GuardDecision::Failed => {
                        termination_reason = TerminationReason::Partial;
                        if let Some(msg) = guard_out.repair_message {
                            last_assistant_text = format!("{last_assistant_text}\n\n{msg}");
                        }
                        logger.line(
                            task_id,
                            &format!(
                                "[turn_end] turn={} tool_calls=0 verification={}",
                                turn,
                                match guard_out.decision {
                                    super::completion_guard::GuardDecision::Partial => "partial",
                                    _ => "failed",
                                }
                            ),
                        );
                        break;
                    }
                }
            }

            logger.line(
                task_id,
                &format!(
                    "[turn_end] turn={} tool_calls={}",
                    turn,
                    turn_tool_calls.len()
                ),
            );

            let tool_ctx = TurnToolCtx {
                task_id,
                agent_type,
                working_directory,
                session_label: &session_label,
                turn,
                loop_limits,
                live_trace_tx: live_trace_tx.clone(),
                verification: Some(Arc::clone(&verification_shared)),
            };
            let mut tool_state = TurnToolState {
                total_tool_calls,
                artifacts: std::mem::take(&mut artifacts),
                budget_state: budget_state.clone(),
                progress_seq,
            };
            let mut sink = MessageAppendSink::Shared(&messages);
            match self
                .dispatch_turn_tool_calls(
                    &logger,
                    &tool_ctx,
                    &mut tool_state,
                    &TurnToolCancel::Coop(coop_cancel.clone()),
                    &mut sink,
                    turn_tool_calls,
                    true,
                    TurnToolCancelOutcome::TurnCancelled,
                )
                .await?
            {
                TurnToolBatchOutcome::Ok => {}
                TurnToolBatchOutcome::Cancelled(out) => {
                    if let Some(err) = out.into_core_error() {
                        return Err(err);
                    }
                }
                TurnToolBatchOutcome::MaxToolCalls => {
                    return Ok(self
                        .finalize_incomplete_turn(TurnFinalizeParams {
                            task_id,
                            agent_type,
                            messages: &messages,
                            working_directory,
                            live_trace_tx: &live_trace_tx,
                            loop_limits,
                            last_model_turn: turn,
                            total_tool_calls: tool_state.total_tool_calls,
                            artifacts: tool_state.artifacts,
                            turn_usage,
                            termination_reason: TerminationReason::MaxTools,
                        })
                        .await);
                }
                TurnToolBatchOutcome::BudgetExceeded => {
                    return Ok(self
                        .finalize_incomplete_turn(TurnFinalizeParams {
                            task_id,
                            agent_type,
                            messages: &messages,
                            working_directory,
                            live_trace_tx: &live_trace_tx,
                            loop_limits,
                            last_model_turn: turn,
                            total_tool_calls: tool_state.total_tool_calls,
                            artifacts: tool_state.artifacts,
                            turn_usage,
                            termination_reason: TerminationReason::Budget,
                        })
                        .await);
                }
            }
            total_tool_calls = tool_state.total_tool_calls;
            artifacts = tool_state.artifacts;
            budget_state = tool_state.budget_state;
            progress_seq = tool_state.progress_seq;
            if turn < loop_limits.max_agent_turns {
                self.maybe_auto_compact_shared(
                    task_id,
                    agent_type,
                    working_directory,
                    &model_config,
                    &messages,
                    response.usage.input_tokens,
                    turn,
                )
                .await?;
            }
        }

        let user_line = {
            let g = messages.lock().await;
            memory_hooks::last_user_plain_text_for_autosave(&g)
        };
        if termination_reason == TerminationReason::Completed
            && !last_assistant_text.trim().is_empty()
        {
            logger.session_state(task_id, "idle");
            live_trace_emit::emit_turn_done(&live_trace_tx, "completed");
            logger.line(task_id, "[task_end] status=completed reason=completed");
            logger.line(
                task_id,
                &format!(
                    "[final_output] source=assistant reply_chars={}",
                    last_assistant_text.chars().count()
                ),
            );
            logger.line(task_id, "== assistant_final ==");
            for line in last_assistant_text.lines() {
                logger.line(task_id, line);
            }
            self.maybe_autosave_memory(task_id, &user_line, &last_assistant_text)
                .await;
            return Ok(TurnOutput {
                final_text: last_assistant_text,
                artifacts,
                usage: turn_usage,
                termination_reason,
            });
        }

        let output = self
            .finalize_incomplete_turn(TurnFinalizeParams {
                task_id,
                agent_type,
                messages: &messages,
                working_directory,
                live_trace_tx: &live_trace_tx,
                loop_limits,
                last_model_turn,
                total_tool_calls,
                artifacts,
                turn_usage,
                termination_reason,
            })
            .await;

        Ok(output)
    }
}
