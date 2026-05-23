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

    pub(crate) fn tail(&self, task_id: TaskId, max_bytes: usize) -> String {
        if let Some(out) = &self.disk {
            out.tail(task_id, max_bytes).unwrap_or_default()
        } else {
            String::new()
        }
    }
}
