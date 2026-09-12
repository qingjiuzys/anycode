use anycode_harness_core::{
    approval::ApprovalVault, budget::BudgetPool, Capabilities, Result, RunContext, Scope,
};
use anycode_harness_extensions::computer::*;
use async_trait::async_trait;
use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};
use uuid::Uuid;
struct Fake(AtomicUsize);
#[async_trait]
impl ComputerBackend for Fake {
    async fn observe(&self, _: &RunContext) -> Result<Observation> {
        Ok(Observation {
            frame_id: Uuid::new_v4(),
            width: 100,
            height: 100,
            png: b"\x89PNG\r\n\x1a\n".to_vec(),
            target: "42".into(),
        })
    }
    async fn act(&self, _: &RunContext, _: &str, _: &Action) -> Result<()> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}
fn ctx(device: Uuid) -> RunContext {
    RunContext::root(
        Scope {
            subject: Uuid::new_v4(),
            organization: None,
            tenant: None,
            project: Uuid::new_v4(),
            device: Some(device),
        },
        Capabilities::new(["computer.observe".into(), "computer.input".into()]).unwrap(),
        BudgetPool::new(1000).unwrap(),
        Duration::from_secs(10),
    )
    .unwrap()
}
#[tokio::test]
async fn action_needs_fresh_frame_and_bound_approval() {
    let d = Uuid::new_v4();
    let c = ctx(d);
    let backend = Arc::new(Fake(AtomicUsize::new(0)));
    let broker = ComputerBroker::new(d, backend.clone()).unwrap();
    let mut lease = broker.acquire(&c, Duration::from_secs(10)).await.unwrap();
    assert!(broker.acquire(&c, Duration::from_secs(1)).await.is_err());
    let obs = broker.observe(&c, &mut lease).await.unwrap();
    let action = Action::Click { x: 3, y: 4 };
    let binding = broker
        .approval_binding(&c, &lease, obs.frame_id, &action)
        .unwrap();
    let vault = ApprovalVault::default();
    let ticket = vault
        .issue_from_trusted_ui(binding, Duration::from_secs(10))
        .unwrap();
    broker
        .act(&c, &mut lease, obs.frame_id, action.clone(), ticket, &vault)
        .await
        .unwrap();
    assert_eq!(backend.0.load(Ordering::SeqCst), 1);
    assert!(broker
        .approval_binding(&c, &lease, obs.frame_id, &action)
        .is_err());
}
#[tokio::test]
async fn outside_click_and_arbitrary_key_combinations_denied() {
    let d = Uuid::new_v4();
    let c = ctx(d);
    let b = ComputerBroker::new(d, Arc::new(Fake(AtomicUsize::new(0)))).unwrap();
    let mut l = b.acquire(&c, Duration::from_secs(10)).await.unwrap();
    let o = b.observe(&c, &mut l).await.unwrap();
    assert!(b
        .approval_binding(&c, &l, o.frame_id, &Action::Click { x: 100, y: 0 })
        .is_err());
    assert!(b
        .approval_binding(
            &c,
            &l,
            o.frame_id,
            &Action::Key {
                key: "ctrl+alt+t".into()
            }
        )
        .is_err());
    assert!(b
        .approval_binding(
            &c,
            &l,
            o.frame_id,
            &Action::Type {
                text: "execute\nnow".into()
            }
        )
        .is_err());
}
#[tokio::test]
async fn unrelated_device_cannot_acquire() {
    let d = Uuid::new_v4();
    let b = ComputerBroker::new(d, Arc::new(Fake(AtomicUsize::new(0)))).unwrap();
    assert!(b
        .acquire(&ctx(Uuid::new_v4()), Duration::from_secs(10))
        .await
        .is_err());
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn macos_screencapture_observe_is_a_real_png() {
    let dir = tempfile::tempdir().unwrap();
    let backend =
        Arc::new(MacScreenCaptureBackend::host_default(dir.path().to_path_buf()).unwrap());
    let d = Uuid::new_v4();
    let c = ctx(d);
    let broker = ComputerBroker::new(d, backend.clone()).unwrap();
    let mut lease = broker.acquire(&c, Duration::from_secs(30)).await.unwrap();
    let obs = broker.observe(&c, &mut lease).await.unwrap();
    assert!(obs.png.starts_with(b"\x89PNG\r\n\x1a\n"), "real PNG header");
    assert!(obs.width >= 16 && obs.height >= 16, "{obs:?}");
    assert!(obs.png.len() > 64);
    assert_eq!(obs.target, "display");
    assert!(
        backend
            .act(&c, "display", &Action::Click { x: 1, y: 1 })
            .await
            .is_err(),
        "macOS backend is observe-only until Accessibility is enrolled"
    );
}
