use anycode_harness_core::{budget::BudgetPool, Capabilities, Error, RunContext, Scope};
use anycode_harness_extensions::process::{run_host_command, CommandSpec};
use std::{collections::BTreeMap, path::PathBuf, time::Duration};
use uuid::Uuid;

fn ctx() -> RunContext {
    RunContext::root(
        Scope {
            subject: Uuid::new_v4(),
            organization: None,
            tenant: None,
            project: Uuid::new_v4(),
            device: None,
        },
        Capabilities::new([] as [String; 0]).unwrap(),
        BudgetPool::new(1000).unwrap(),
        Duration::from_secs(15),
    )
    .unwrap()
}

#[cfg(unix)]
#[tokio::test]
async fn cancel_kills_unix_process_group() {
    let sh = PathBuf::from("/bin/sh");
    let sleep = PathBuf::from("/bin/sleep");
    assert!(sh.is_file(), "/bin/sh required");
    assert!(sleep.is_file(), "/bin/sleep required");
    let tmp = tempfile::tempdir().unwrap();
    let grandchild_pid = tmp.path().join("grandchild.pid");
    let ctx = ctx();
    let spec = CommandSpec {
        executable: sh.clone(),
        arguments: vec![
            "-c".into(),
            format!(
                "/bin/sleep 30 & echo $! > {}; wait",
                grandchild_pid.display()
            ),
        ],
        cwd: tmp.path().to_path_buf(),
        environment: BTreeMap::new(),
        timeout: Duration::from_secs(8),
        output_limit: 4096,
    };
    let runner = {
        let ctx = ctx.clone();
        let spec = spec.clone();
        let sh = sh.clone();
        tokio::spawn(async move { run_host_command(&ctx, &spec, &[sh]).await })
    };
    let started = tokio::time::Instant::now();
    while !grandchild_pid.exists() && started.elapsed() < Duration::from_secs(2) {
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(
        grandchild_pid.exists(),
        "allowlisted /bin/sh must spawn /bin/sleep"
    );
    ctx.cancel();
    let result = runner.await.unwrap();
    assert!(
        matches!(result, Err(Error::Cancelled)),
        "cancel must surface, got {result:?}"
    );
    tokio::time::sleep(Duration::from_millis(150)).await;
    let pid: i32 = std::fs::read_to_string(&grandchild_pid)
        .expect("grandchild pid")
        .trim()
        .parse()
        .expect("pid");
    let alive = std::process::Command::new("kill")
        .args(["-0", &pid.to_string()])
        .status()
        .unwrap()
        .success();
    assert!(
        !alive,
        "grandchild /bin/sleep {pid} must die with the process group"
    );
}
