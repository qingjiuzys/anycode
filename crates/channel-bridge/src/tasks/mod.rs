//! Headless task execution for channels and cron.

mod tasks_run;
mod tasks_sink;
mod workflow_exec;

pub(crate) use tasks_run::{run_single_task_with_tail, RunTaskOptions};
pub(crate) use tasks_sink::ReplSink;
