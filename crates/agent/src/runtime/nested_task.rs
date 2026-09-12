//! Nested sub-agent execution (`SubAgentExecutor`).

use super::nested_worktree;
use super::AgentRuntime;
use anycode_core::prelude::*;
use async_trait::async_trait;
use std::collections::HashMap;
use tokio::sync::mpsc::UnboundedSender;
use uuid::Uuid;

/// 子代理默认追加提示词（D6：对齐 Claude Code `appendSubagentSystemPrompt`）。
/// 提示子代理聚焦任务、避免重复上下文与冗长回执。
/// 该常量同时作为「是否为嵌套子代理任务」的判定标记（见 `execute_task` 子代理工具过滤）。
pub(crate) const SUBAGENT_SYSTEM_APPEND: &str = "\
你是被父任务派出的子代理，只完成父任务交给你的这个任务。\
请保持专注：不要重复描述父任务已提供的上下文，不要主动扩大任务范围。\
输出只包含任务要求的结果；完成后用简短一句话总结即可。";

#[async_trait]
impl SubAgentExecutor for AgentRuntime {
    async fn run_nested_task(&self, invoke: NestedTaskInvoke) -> Result<NestedTaskRun, CoreError> {
        #[cfg(feature = "harness-v1")]
        if self.harness_unified_kernel_enabled() {
            return self.run_nested_task_via_supervisor(invoke).await;
        }
        let mut wd = invoke.working_directory;
        let wt_roots = {
            let iso = invoke
                .isolation
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty());
            if iso.is_some_and(|s| s.eq_ignore_ascii_case("worktree")) {
                let (repo, wt) = nested_worktree::create_nested_worktree(&wd).await?;
                wd = wt.clone();
                Some((repo, wt))
            } else {
                None
            }
        };

        let task_id = invoke.task_id.unwrap_or_else(Uuid::new_v4);
        let agent_type_str = invoke.agent_type.as_str().to_string();
        let parent_task_id = invoke.parent_task_id;
        // Step 3b：子任务事件经 Subagent 包装转发到父 live trace 通道；
        // 父侧无通道时保持 None（嵌套运行不可见的旧行为）。
        let nested_trace_tx = invoke.live_trace_tx.as_ref().map(|parent_tx| {
            spawn_nested_trace_forwarder(
                parent_tx.clone(),
                task_id,
                agent_type_str.clone(),
                parent_task_id,
            )
        });

        let task = Task {
            id: task_id,
            agent_type: invoke.agent_type,
            prompt: invoke.prompt,
            context: TaskContext {
                session_id: Uuid::new_v4(),
                working_directory: wd,
                environment: HashMap::new(),
                user_id: None,
                system_prompt_append: Some(SUBAGENT_SYSTEM_APPEND.to_string()),
                context_injections: invoke.context_injections.clone(),
                nested_model_override: invoke.model.clone(),
                nested_worktree_repo_root: wt_roots.as_ref().map(|(r, _)| r.clone()),
                nested_worktree_path: wt_roots.as_ref().map(|(_, p)| p.clone()),
                nested_cancel: invoke.cancel.clone(),
                channel_progress_tx: None,
                live_trace_tx: nested_trace_tx,
                tool_deny_names: invoke.tool_deny_names.clone(),
                tool_deny_prefixes: invoke.tool_deny_prefixes.clone(),
                user_vision_images: vec![],
                budget: invoke.budget.unwrap_or_default(),
                loop_limits: anycode_core::resolve_agent_loop_limits(None, None),
                chat_turn: anycode_core::current_chat_turn(),
            },
            created_at: chrono::Utc::now(),
        };
        let result = self.execute_task(task).await?;
        // 嵌套可观测性（token 指标链路）：子任务日志落在自己的 tasks/<id>/output.log，
        // 父 recorder 只 tail 父日志。此处把子任务完成标记写入父日志，dashboard recorder
        // 据此摄取子任务 log 的 llm_response_end → llm_usage（payload 带 agent_type）。
        // 子任务日志此时已完整（execute_task 同步写完）；best-effort，失败不影响结果。
        log_nested_task_end_to_parent(
            &self.disk_output,
            parent_task_id,
            task_id,
            &agent_type_str,
            nested_status_str(&result),
        );
        Ok(NestedTaskRun { task_id, result })
    }

    fn agent_catalog(&self) -> Vec<(String, String)> {
        // 同步上下文（工具 schema 构建路径）：`try_read` 繁忙时返回空目录；
        // schema 每轮重建，窗口期错过会自愈。
        let Ok(agents) = self.agents.try_read() else {
            return Vec::new();
        };
        super::summaries_from_agents(&agents)
    }
}

#[cfg(feature = "harness-v1")]
impl AgentRuntime {
    async fn run_nested_task_via_supervisor(
        &self,
        invoke: NestedTaskInvoke,
    ) -> Result<NestedTaskRun, CoreError> {
        use super::harness_bridge::KernelChildExecutor;
        use super::harness_tools::child_kernel_limits;
        use anycode_harness_core::{journal::MemoryJournal, Capabilities};
        use anycode_harness_extensions::subagents::{Assignment, Supervisor};
        use std::path::PathBuf;
        use std::sync::atomic::Ordering;
        use std::time::Duration;

        fn host_err(err: impl std::fmt::Display) -> CoreError {
            CoreError::Other(anyhow::anyhow!(err.to_string()))
        }

        let this = self.upgraded_self()?;
        let (parent, hub_wd) = this
            .harness_hub
            .current()
            .map_err(|e| host_err(format!("unified Agent requires a Host RunContext: {e}")))?;
        parent
            .capabilities()
            .require("agent.spawn")
            .map_err(|e| host_err(e))?;
        let wd = if invoke.working_directory.trim().is_empty() {
            hub_wd
        } else {
            PathBuf::from(&invoke.working_directory)
        };
        let agent = invoke.agent_type.as_str();
        let readonly = matches!(agent, "explore" | "plan" | "planner" | "reviewer");
        let (profile, caps) = if readonly {
            (
                if agent == "planner" { "plan" } else { agent },
                vec!["fs.read".into()],
            )
        } else {
            (
                "implementer",
                vec![
                    "fs.read".into(),
                    "fs.write".into(),
                    "worktree.create".into(),
                ],
            )
        };
        let assignment = Assignment {
            agent: profile.into(),
            prompt: invoke.prompt,
            capabilities: Capabilities::new(caps).map_err(|e| host_err(e))?,
        };
        let journal = std::sync::Arc::new(MemoryJournal::default());
        let mut executor =
            KernelChildExecutor::new(this.clone(), wd, journal, child_kernel_limits(&parent));
        if !readonly {
            let Some(isolation) = this.harness_hub.write_isolation() else {
                return Err(host_err("write child requires an isolated worktree host"));
            };
            executor = executor.with_write_isolation(isolation);
        }
        let supervisor = Supervisor::new(&parent, 1, 4).map_err(|e| host_err(e))?;
        let run = supervisor.run(&parent, assignment, &executor);
        let outcome = if let Some(flag) = invoke.cancel.clone() {
            tokio::select! {
                _ = async {
                    while !flag.load(Ordering::Acquire) {
                        tokio::time::sleep(Duration::from_millis(20)).await;
                    }
                } => {
                    parent.cancel();
                    return Err(CoreError::CooperativeCancel);
                }
                result = run => result,
            }
        } else {
            run.await
        }
        .map_err(|e| host_err(e))?;
        let text = outcome
            .output
            .get("text")
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .unwrap_or_else(|| outcome.output.to_string());
        let result = if outcome.partial {
            TaskResult::Partial {
                success: text,
                remaining: "harness child returned partial".into(),
            }
        } else {
            TaskResult::Success {
                output: text,
                artifacts: vec![],
            }
        };
        log_nested_task_end_to_parent(
            &self.disk_output,
            invoke.parent_task_id,
            outcome.run_id,
            agent,
            nested_status_str(&result),
        );
        Ok(NestedTaskRun {
            task_id: invoke.task_id.unwrap_or(outcome.run_id),
            result,
        })
    }
}

/// 唯一打标点：把嵌套任务的内部 live trace 事件包成 `Subagent` 转发给父通道。
/// 发送端全部 drop 后转发任务退出；父 bridge 已关闭（send 失败）时停止转发，
/// 嵌套任务本身不受影响（事件仅丢失，不写父时间线）。
fn spawn_nested_trace_forwarder(
    parent_tx: UnboundedSender<LiveTraceEvent>,
    task_id: Uuid,
    agent_type: String,
    parent_task_id: Option<Uuid>,
) -> UnboundedSender<LiveTraceEvent> {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<LiveTraceEvent>();
    tokio::spawn(async move {
        while let Some(evt) = rx.recv().await {
            let wrapped = LiveTraceEvent::Subagent {
                task_id,
                agent_type: agent_type.clone(),
                parent_task_id,
                event: Box::new(evt),
            };
            if parent_tx.send(wrapped).is_err() {
                break;
            }
        }
    });
    tx
}

/// 父日志中的嵌套任务完成标记（dashboard `log_parser` 的 `nested_task_end` 消费）。
/// 单行 KV 格式与 `[task_end]` 等既有标记一致。
pub(crate) fn nested_task_end_marker(task_id: Uuid, agent_type: &str, status: &str) -> String {
    format!("[nested_task_end] task_id={task_id} agent_type={agent_type} status={status}")
}

fn nested_status_str(result: &TaskResult) -> &'static str {
    match result {
        TaskResult::Success { .. } => "completed",
        TaskResult::Partial { .. } => "partial",
        TaskResult::Failure { .. } => "failed",
    }
}

/// 把嵌套任务完成标记追加到父任务 output.log（best-effort：无父 id / 无磁盘句柄 / 写失败均静默）。
fn log_nested_task_end_to_parent(
    disk: &Option<DiskTaskOutput>,
    parent_task_id: Option<Uuid>,
    task_id: Uuid,
    agent_type: &str,
    status: &str,
) {
    let (Some(disk), Some(parent)) = (disk.as_ref(), parent_task_id) else {
        return;
    };
    let _ = disk.append_line(parent, &nested_task_end_marker(task_id, agent_type, status));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_task_end_marker_is_single_line_kv() {
        let tid = Uuid::new_v4();
        let line = nested_task_end_marker(tid, "explore", "completed");
        assert!(line.starts_with("[nested_task_end] "));
        assert!(line.contains(&format!("task_id={tid}")));
        assert!(line.contains("agent_type=explore"));
        assert!(line.contains("status=completed"));
        assert!(!line.contains('\n'));
    }

    #[test]
    fn nested_status_str_maps_variants() {
        assert_eq!(
            nested_status_str(&TaskResult::Success {
                output: String::new(),
                artifacts: vec![],
            }),
            "completed"
        );
        assert_eq!(
            nested_status_str(&TaskResult::Failure {
                error: "e".into(),
                details: None,
            }),
            "failed"
        );
        assert_eq!(
            nested_status_str(&TaskResult::Partial {
                success: "s".into(),
                remaining: "r".into(),
            }),
            "partial"
        );
    }

    #[test]
    fn marker_appended_to_parent_log_only_when_parent_and_disk_present() {
        let dir = tempfile::tempdir().unwrap();
        let disk = DiskTaskOutput::new(dir.path().to_path_buf());
        let parent = Uuid::new_v4();
        let nested = Uuid::new_v4();

        log_nested_task_end_to_parent(&Some(disk.clone()), Some(parent), nested, "plan", "failed");
        let content = std::fs::read_to_string(disk.output_path(parent)).unwrap();
        assert_eq!(
            content.trim(),
            nested_task_end_marker(nested, "plan", "failed")
        );

        // 无父 id / 无磁盘句柄 → 静默不写
        log_nested_task_end_to_parent(&Some(disk.clone()), None, nested, "plan", "failed");
        log_nested_task_end_to_parent(&None, Some(parent), nested, "plan", "failed");
        let content = std::fs::read_to_string(disk.output_path(parent)).unwrap();
        assert_eq!(content.lines().count(), 1);
    }

    #[tokio::test]
    async fn nested_trace_forwarder_wraps_inner_events_with_identity() {
        let (parent_tx, mut parent_rx) = tokio::sync::mpsc::unbounded_channel::<LiveTraceEvent>();
        let task_id = Uuid::new_v4();
        let parent_task_id = Uuid::new_v4();
        let child_tx = spawn_nested_trace_forwarder(
            parent_tx,
            task_id,
            "explore".to_string(),
            Some(parent_task_id),
        );

        child_tx
            .send(LiveTraceEvent::TurnStart { turn: 1 })
            .unwrap();
        let wrapped = parent_rx.recv().await.expect("forwarded event");
        match wrapped {
            LiveTraceEvent::Subagent {
                task_id: tid,
                agent_type,
                parent_task_id: pid,
                event,
            } => {
                assert_eq!(tid, task_id);
                assert_eq!(agent_type, "explore");
                assert_eq!(pid, Some(parent_task_id));
                assert_eq!(*event, LiveTraceEvent::TurnStart { turn: 1 });
            }
            other => panic!("expected Subagent wrapper, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn nested_trace_forwarder_exits_when_child_senders_drop() {
        let (parent_tx, mut parent_rx) = tokio::sync::mpsc::unbounded_channel::<LiveTraceEvent>();
        let child_tx =
            spawn_nested_trace_forwarder(parent_tx, Uuid::new_v4(), "plan".to_string(), None);
        child_tx
            .send(LiveTraceEvent::TurnStart { turn: 1 })
            .unwrap();
        assert!(parent_rx.recv().await.is_some());
        // 子侧全部 sender drop 后转发任务退出、父通道随之关闭。
        drop(child_tx);
        let drained: Vec<_> = std::iter::from_fn(|| parent_rx.try_recv().ok()).collect();
        assert!(drained.is_empty());
        assert!(parent_rx.recv().await.is_none());
    }

    #[tokio::test]
    async fn nested_trace_forwarder_stops_when_parent_channel_closed() {
        let (parent_tx, parent_rx) = tokio::sync::mpsc::unbounded_channel::<LiveTraceEvent>();
        let child_tx =
            spawn_nested_trace_forwarder(parent_tx, Uuid::new_v4(), "plan".to_string(), None);
        drop(parent_rx);
        // 父通道关闭后转发失败退出；子侧发送不 panic（事件仅丢失）。
        for _ in 0..8 {
            let _ = child_tx.send(LiveTraceEvent::TurnStart { turn: 1 });
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let _ = child_tx.send(LiveTraceEvent::TurnDone {
            status: "completed".to_string(),
        });
    }
}
