//! Continuous session turn execution

use super::agentic_loop::{
    coop_flag_wait_opt, opt_coop_cancelled, pop_assistant_placeholder,
    rehydrate_stream_llm_response,
};
use super::artifacts::extract_artifacts;
use super::evidence;
use super::limits::{MAX_AGENT_TURNS, MAX_TOOL_CALLS_TOTAL};
use super::memory_hooks;
use super::provider_errors::{
    core_error_is_context_overflow, error_indicates_context_overflow,
    provider_error_from_streamed_assistant_text,
};
use super::receipt::ReceiptGenerator;
use super::task_summary::llm_summary_receipt;
use super::tool_result_injection;
use super::tool_surface;
use super::AgentRuntime;
use anycode_core::prelude::*;
use anycode_core::strip_llm_reasoning_xml_blocks;
use anycode_core::Artifact;
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
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
            tool_deny_names,
            tool_deny_prefixes,
        );
        let tool_schemas = tool_surface::build_tool_schemas(&names, &tools);
        drop(tools);

        // 2) agentic loop：保持与 execute_task 的语义一致
        let model_config = self.model_for_task(agent_type).clone();
        let mut total_tool_calls: usize = 0;
        let mut artifacts: Vec<Artifact> = vec![];
        let mut last_assistant_text = String::new();
        let mut turn_usage = TurnTokenUsage::default();
        let mut last_model_turn: usize = 1;

        for turn in 1..=MAX_AGENT_TURNS {
            last_model_turn = turn;
            logger.line(
                task_id,
                &format!("[turn_start] turn={}/{}", turn, MAX_AGENT_TURNS),
            );
            if opt_coop_cancelled(&coop_cancel) {
                logger.line(task_id, "[task_end] status=cancelled reason=cooperative");
                return Err(CoreError::CooperativeCancel);
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

            let mut overflow_retried_this_turn = false;
            let llm_t0 = std::time::Instant::now();
            let (response, llm_streamed) = 'llm_attempt: loop {
                let messages_snapshot = {
                    let g = messages.lock().await;
                    g.clone()
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
                    tool_schemas.clone(),
                    &model_config,
                );
                let stream_open = tokio::select! {
                    biased;
                    () = coop_flag_wait_opt(coop_cancel.clone()) => {
                        pop_assistant_placeholder(&messages, assistant_id).await;
                        logger.line(
                            task_id,
                            "[llm_response_end] status=cancelled reason=cooperative_in_flight",
                        );
                        logger.line(task_id, "[task_end] status=cancelled reason=cooperative");
                        return Err(CoreError::CooperativeCancel);
                    }
                    r = stream_open => r,
                };

                if let Ok(mut rx) = stream_open {
                    streamed = true;
                    let mut received_any = false;
                    let mut stream_cancelled = false;
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
                                        StreamEvent::ToolCall(tc) => {
                                            received_any = true;
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
                        logger.line(task_id, "[task_end] status=cancelled reason=cooperative");
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
                    let chat_fut = self.llm_client.chat(
                        messages_snapshot,
                        tool_schemas.clone(),
                        &model_config,
                    );
                    let r = tokio::select! {
                        biased;
                        () = coop_flag_wait_opt(coop_cancel.clone()) => {
                            logger.line(
                                task_id,
                                "[llm_response_end] status=cancelled reason=cooperative_in_flight",
                            );
                            logger.line(task_id, "[task_end] status=cancelled reason=cooperative");
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
                let text_probe = strip_llm_reasoning_xml_blocks(raw_assistant_probe);
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
                        {
                            let mut g = messages.lock().await;
                            if g.last().is_some_and(|m| m.role == MessageRole::Assistant) {
                                g.pop();
                            }
                        }
                        logger.line(task_id, "[task_end] status=failed");
                        return Err(CoreError::LLMError(err));
                    }
                }
                break (response, streamed);
            };

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
            let text = strip_llm_reasoning_xml_blocks(raw_assistant);
            if !text.trim().is_empty() {
                last_assistant_text = text;
            }
            // If we streamed, assistant message is already in `messages`; no need to push again.
            // If we didn't stream, we already replaced placeholder with `r.message` above.

            let session_label = format!("tui_{}", task_id);

            if response.tool_calls.is_empty() {
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

            logger.line(
                task_id,
                &format!(
                    "[turn_end] turn={} tool_calls={}",
                    turn,
                    response.tool_calls.len()
                ),
            );

            for tool_call in response.tool_calls {
                if opt_coop_cancelled(&coop_cancel) {
                    logger.line(task_id, "[task_end] status=cancelled reason=cooperative");
                    return Err(CoreError::CooperativeCancel);
                }
                total_tool_calls += 1;
                if total_tool_calls > MAX_TOOL_CALLS_TOTAL {
                    logger.line(
                        task_id,
                        &format!(
                            "[task_end] status=failed reason=max_tool_calls({})",
                            MAX_TOOL_CALLS_TOTAL
                        ),
                    );
                    return Ok(TurnOutput {
                        final_text: last_assistant_text,
                        artifacts,
                        usage: turn_usage,
                    });
                }

                tool_result_injection::log_tool_call_input(
                    &logger,
                    task_id,
                    turn,
                    total_tool_calls,
                    &tool_call,
                );
                tool_result_injection::log_tool_call_start(
                    &logger,
                    task_id,
                    turn,
                    total_tool_calls,
                    &tool_call,
                );
                let t0 = std::time::Instant::now();
                let tool_result = self
                    .execute_tool_call(task_id, working_directory, &tool_call)
                    .await?;
                tool_result_injection::log_tool_call_end(
                    &logger,
                    task_id,
                    turn,
                    total_tool_calls,
                    &tool_call,
                    &tool_result,
                    t0.elapsed().as_millis(),
                );

                // 回注 tool_result（截断以防爆上下文）
                let prepared = tool_result_injection::prepare_tool_result_message(
                    task_id,
                    &tool_call,
                    &tool_result,
                    &logger,
                );
                evidence::append_tool_evidence(task_id, &tool_call.name, &prepared.for_hook);
                {
                    let mut g = messages.lock().await;
                    g.push(prepared.message);
                }

                self.pipeline_memory_hook_tool_result(
                    &session_label,
                    task_id,
                    &tool_call.name,
                    &prepared.for_hook,
                )
                .await;
                self.maybe_session_notify_tool_result(
                    &session_label,
                    task_id,
                    turn,
                    &tool_call.name,
                    &prepared.for_hook,
                    Some(working_directory),
                );

                artifacts.extend(extract_artifacts(&tool_call, &tool_result));
                if opt_coop_cancelled(&coop_cancel) {
                    logger.line(task_id, "[task_end] status=cancelled reason=cooperative");
                    return Err(CoreError::CooperativeCancel);
                }
            }
        }

        let user_line = {
            let g = messages.lock().await;
            memory_hooks::last_user_plain_text_for_autosave(&g)
        };
        if !last_assistant_text.trim().is_empty() {
            logger.line(task_id, "[task_end] status=completed");
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
            });
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
                nested_model_override: None,
                nested_worktree_path: None,
                nested_worktree_repo_root: None,
                nested_cancel: None,
                channel_progress_tx: None,
                tool_deny_names: vec![],
                tool_deny_prefixes: vec![],
                budget: TaskBudget::default(),
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

        logger.line(task_id, "[task_end] status=completed");
        logger.line(task_id, "== summary ==");
        for line in summary_text.lines() {
            logger.line(task_id, line);
        }

        self.maybe_autosave_memory(task_id, &user_line, &summary_text)
            .await;
        // 与 `execute_task` 的 summary 回执一致：须写入会话 `messages`，流式 REPL 仅靠
        // `build_stream_turn_plain(messages)` 渲染主区；仅返回 `TurnOutput` 会导致「有工具无总结」。
        if !summary_text.trim().is_empty() {
            let mut g = messages.lock().await;
            g.push(Message {
                id: Uuid::new_v4(),
                role: MessageRole::Assistant,
                content: MessageContent::Text(summary_text.clone()),
                timestamp: chrono::Utc::now(),
                metadata: HashMap::new(),
            });
        }

        let session_label = format!("tui_{}", task_id);
        self.pipeline_memory_hook_agent_turn(
            &session_label,
            task_id,
            last_model_turn,
            &summary_text,
        )
        .await;
        self.maybe_session_notify_agent_turn(
            &session_label,
            task_id,
            last_model_turn,
            &summary_text,
            Some(working_directory),
        );

        Ok(TurnOutput {
            final_text: summary_text,
            artifacts,
            usage: turn_usage,
        })
    }
}
