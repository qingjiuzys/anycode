//! Headless task execution for channels and cron.

mod tasks_run;
mod tasks_sink;
mod workflow_exec;

pub(crate) use tasks_run::{run_single_task_with_tail, RunTaskOptions};
pub(crate) use tasks_sink::ReplSink;
pub(crate) use workflow_exec::run_workflow_path;

#[cfg(test)]
mod harness_live {
    use super::*;
    use crate::app_config::{load_runtime_config, LoadOpts};
    use crate::task_builders::build_headless_task;
    use anycode_bootstrap::{initialize_runtime, MemoryAttachMode, RuntimeHosts};
    use anycode_core::{FeatureFlag, TaskBudget, TaskResult};

    const MARKER: &str = "HARNESS_LIVE_LLM_MARKER_7c2e";

    async fn arm_live_provider(config: &mut anycode_config::Config) {
        if anycode_llm::normalize_provider_id(&config.llm.provider) != "anycode_cloud" {
            return;
        }
        if anycode_llm::refresh_cloud_access_token().await.is_ok() {
            return;
        }
        if let Some(key) = config
            .llm
            .provider_credentials
            .get("deepseek")
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
        {
            config.llm.provider = "deepseek".into();
            config.llm.api_key = key;
            config.llm.model = "deepseek-v4-flash".into();
            config.llm.base_url =
                anycode_llm::suggested_openai_base_for("deepseek").map(str::to_string);
        }
    }

    #[tokio::test]
    #[ignore = "paid LLM; export ANYCODE_HARNESS_LIVE_LLM=1; --test-threads=1; does not print secrets"]
    async fn live_scheduler_headless_task_fileread_uses_configured_provider() {
        assert_eq!(
            std::env::var("ANYCODE_HARNESS_LIVE_LLM").ok().as_deref(),
            Some("1"),
            "refusing to call a paid provider unless ANYCODE_HARNESS_LIVE_LLM=1"
        );
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("MARKER.txt"), MARKER).unwrap();
        let mut config = load_runtime_config(LoadOpts {
            config_file: None,
            ignore_approval: true,
            workspace_overlay: false,
            workspace_overlay_dir: Some(tmp.path().to_path_buf()),
        })
        .await
        .expect("load runtime config");
        arm_live_provider(&mut config).await;
        config
            .runtime
            .features
            .enable(FeatureFlag::HarnessV1UnifiedKernel.as_str());
        let runtime = initialize_runtime(
            &config,
            RuntimeHosts::default(),
            MemoryAttachMode::Shared,
            None,
            Some(tmp.path()),
        )
        .await
        .expect("initialize_runtime");
        let options = RunTaskOptions {
            budget: TaskBudget {
                token_budget_total: Some(200_000),
                max_duration_secs: Some(90),
                ..TaskBudget::default()
            },
            ..RunTaskOptions::default()
        };
        let task = build_headless_task(
            "explore".into(),
            "Read MARKER.txt with FileRead. Quote the exact file contents. Do not guess.".into(),
            tmp.path().to_path_buf(),
            &options,
            Some(&config),
        );
        let result = runtime
            .execute_task(task)
            .await
            .expect("scheduler-shaped execute_task FileRead");
        match result {
            TaskResult::Success { output, .. } => {
                assert!(
                    output.contains(MARKER),
                    "cron/headless task must return the real file marker"
                );
            }
            other => panic!("expected Success with file marker, got {other:?}"),
        }
    }
}
