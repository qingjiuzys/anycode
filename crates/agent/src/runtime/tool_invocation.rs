//! Tool invocation chain: security → Claude gating → approval → execute → audit.

use super::tool_audit;
use super::AgentRuntime;
use anycode_core::prelude::*;
use std::collections::HashMap;

impl AgentRuntime {
    pub(super) async fn run_tool_invocation_pipeline(
        &self,
        tools: &HashMap<ToolName, Box<dyn Tool>>,
        task_id: TaskId,
        agent_type: &AgentType,
        working_directory: &str,
        tool_call: &ToolCall,
    ) -> Result<ToolOutput, CoreError> {
        tool_audit::append_tool_audit(
            task_id,
            "pre_check",
            working_directory,
            tool_call,
            "pending",
            None,
        );
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
                return self.tool_invocation_deny(
                    task_id,
                    working_directory,
                    tool_call,
                    "policy",
                    &reason,
                );
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
                    return self.tool_invocation_deny(
                        task_id,
                        working_directory,
                        tool_call,
                        "policy",
                        &reason,
                    );
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
                                return self.tool_invocation_deny(
                                    task_id,
                                    working_directory,
                                    tool_call,
                                    "approval",
                                    &reason,
                                );
                            }
                            Err(e) => return Err(e),
                        }
                    }
                }
            }
        }

        let input = ToolInput {
            name: tool_call.name.clone(),
            input: anycode_tools::coerce_tool_input(&tool_call.name, tool_call.input.clone()),
            working_directory: if working_directory.is_empty() {
                None
            } else {
                Some(working_directory.to_string())
            },
            sandbox_mode: self.sandbox_mode,
        };

        if let Ok(guard) = self.tool_services.lock() {
            if let Some(svc) = guard.as_ref() {
                svc.set_active_agent_type(Some(agent_type.as_str().to_string()));
            }
        }

        tool_audit::append_tool_audit(
            task_id,
            "execute",
            working_directory,
            tool_call,
            "allowed",
            None,
        );
        let out = tool.execute(input).await;
        match &out {
            Ok(o) => tool_audit::append_tool_audit(
                task_id,
                "result",
                working_directory,
                tool_call,
                if o.error.is_some() {
                    "tool_error"
                } else {
                    "ok"
                },
                o.error.as_deref(),
            ),
            Err(e) => tool_audit::append_tool_audit(
                task_id,
                "result",
                working_directory,
                tool_call,
                "runtime_error",
                Some(&e.to_string()),
            ),
        }
        match out {
            Ok(o) => Ok(o),
            Err(CoreError::PermissionDenied(reason)) => {
                self.tool_invocation_deny(task_id, working_directory, tool_call, "result", &reason)
            }
            Err(e @ CoreError::SerializationError(_)) => Ok(ToolOutput {
                result: serde_json::json!({
                    "error": e.to_string(),
                    "tool": tool_call.name,
                    "hint": "Malformed tool arguments; fix JSON shape and retry."
                }),
                error: Some(e.to_string()),
                duration_ms: 0,
            }),
            Err(e) => Err(e),
        }
    }

    fn tool_invocation_deny(
        &self,
        task_id: TaskId,
        working_directory: &str,
        tool_call: &ToolCall,
        audit_stage: &'static str,
        reason: &str,
    ) -> Result<ToolOutput, CoreError> {
        tool_audit::append_tool_audit(
            task_id,
            audit_stage,
            working_directory,
            tool_call,
            "denied",
            Some(reason),
        );
        self.log_task_line(
            task_id,
            &format!("[tool_denied] name={} reason={}", tool_call.name, reason),
        );
        Ok(ToolOutput {
            result: serde_json::json!({ "error": reason }),
            error: Some(reason.to_string()),
            duration_ms: 0,
        })
    }
}
