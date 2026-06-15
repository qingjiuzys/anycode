//! 任务落盘日志（`DiskTaskOutput`）的薄封装。

use anycode_core::prelude::*;

#[derive(Debug, Clone)]
pub(crate) struct RunLogger {
    disk: Option<DiskTaskOutput>,
}

impl RunLogger {
    pub(crate) fn new(disk: Option<DiskTaskOutput>) -> Self {
        Self { disk }
    }

    pub(crate) fn ensure_initialized(&self, task_id: TaskId) {
        if let Some(out) = &self.disk {
            let _ = out.ensure_initialized(task_id);
        }
    }

    pub(crate) fn line(&self, task_id: TaskId, line: &str) {
        if let Some(out) = &self.disk {
            let _ = out.append_line(task_id, line);
            if let Some(evt) = ExecutionTraceEvent::from_log_line(line) {
                if let Ok(value) = serde_json::to_value(evt) {
                    let _ = out.append_event_json(task_id, &value);
                }
            }
        }
    }

    pub(crate) fn gate(
        &self,
        task_id: TaskId,
        name: &str,
        status: &str,
        command: &str,
        output: &str,
    ) {
        let line = anycode_core::format_gate_log_line(name, status, command, output);
        self.line(task_id, &line);
    }

    /// Structured assistant text for dashboard conversation replay.
    pub(crate) fn assistant_response(&self, task_id: TaskId, turn: usize, text: &str) {
        let text = text.trim();
        if text.is_empty() {
            return;
        }
        let line = anycode_core::format_assistant_response_log_line(turn, text);
        self.line(task_id, &line);
    }

    pub(crate) fn session_keepalive(&self, task_id: TaskId, reason: &str, refcount: u32) {
        self.line(
            task_id,
            &format!("[session_keepalive] reason={reason} refcount={refcount}"),
        );
    }

    pub(crate) fn session_state(&self, task_id: TaskId, state: &str) {
        self.line(task_id, &format!("[session_state_changed] state={state}"));
    }

    pub(crate) fn api_retry(
        &self,
        task_id: TaskId,
        attempt: u32,
        delay_ms: u64,
        model: &str,
        source: &str,
    ) {
        self.line(
            task_id,
            &format!(
                "[api_retry] attempt={attempt} delay_ms={delay_ms} model={model} source={source}"
            ),
        );
    }

    pub(crate) fn turn_error(&self, task_id: TaskId, turn: usize, tool_name: &str, error: &str) {
        self.line(
            task_id,
            &format!(
                "[turn_error] turn={turn} tool={tool_name} error={}",
                error.replace('\n', " ")
            ),
        );
    }

    pub(crate) fn tool_synthetic_result(
        &self,
        task_id: TaskId,
        turn: usize,
        idx: usize,
        tool_name: &str,
        reason: &str,
    ) {
        self.line(
            task_id,
            &format!(
                "[tool_synthetic_result] turn={turn} idx={idx} name={tool_name} reason={reason}"
            ),
        );
    }

    pub(crate) fn tail(&self, task_id: TaskId, max_bytes: usize) -> String {
        if let Some(out) = &self.disk {
            out.tail(task_id, max_bytes).unwrap_or_default()
        } else {
            String::new()
        }
    }
}
