//! Computer control broker: explicit enrollment, run-scoped lease, fresh observation,
//! and one-use action-bound approval. Screenshots are private artifacts, not logs.
use crate::process::{run_host_command, CommandSpec};
use anycode_harness_core::{
    approval::{ApprovalBinding, ApprovalTicket, ApprovalVault},
    Error, Result, RunContext,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::{Mutex, OwnedMutexGuard};
use uuid::Uuid;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Action {
    Click { x: u32, y: u32 },
    Type { text: String },
    Key { key: String },
    Scroll { down: bool, steps: u8 },
}
#[derive(Debug)]
pub struct Observation {
    pub frame_id: Uuid,
    pub width: u32,
    pub height: u32,
    pub png: Vec<u8>,
    pub target: String,
}
#[async_trait]
pub trait ComputerBackend: Send + Sync {
    async fn observe(&self, ctx: &RunContext) -> Result<Observation>;
    /// Implementations recheck foreground target. Never execute commands extracted
    /// from a web page, screenshot or model-provided executable string.
    async fn act(&self, ctx: &RunContext, target: &str, action: &Action) -> Result<()>;
}
pub struct ComputerBroker {
    device: Uuid,
    lock: Arc<Mutex<()>>,
    backend: Arc<dyn ComputerBackend>,
}
pub struct Lease {
    _guard: OwnedMutexGuard<()>,
    run: Uuid,
    scope: String,
    device: Uuid,
    expires: Instant,
    frame: Option<(Uuid, u32, u32, String, Instant)>,
}
impl ComputerBroker {
    pub fn new(device: Uuid, backend: Arc<dyn ComputerBackend>) -> Result<Self> {
        if device.is_nil() {
            return Err(Error::Invalid("device ID".into()));
        }
        Ok(Self {
            device,
            lock: Arc::new(Mutex::new(())),
            backend,
        })
    }
    /// HOST ONLY after the user enrolls this device. A cloud login never grants input.
    pub async fn acquire(&self, ctx: &RunContext, ttl: Duration) -> Result<Lease> {
        ctx.check()?;
        ctx.capabilities().require("computer.observe")?;
        if ctx.scope().device != Some(self.device)
            || ttl.is_zero()
            || ttl > Duration::from_secs(300)
        {
            return Err(Error::Denied("device scope or lease TTL".into()));
        }
        let guard = self
            .lock
            .clone()
            .try_lock_owned()
            .map_err(|_| Error::Capacity)?;
        Ok(Lease {
            _guard: guard,
            run: ctx.id(),
            scope: ctx.scope().binding()?,
            device: self.device,
            expires: Instant::now() + ttl,
            frame: None,
        })
    }
    fn check(&self, ctx: &RunContext, lease: &Lease) -> Result<()> {
        ctx.check()?;
        if lease.run != ctx.id()
            || lease.device != self.device
            || lease.scope != ctx.scope().binding()?
            || lease.expires <= Instant::now()
        {
            return Err(Error::Denied("invalid computer lease".into()));
        }
        Ok(())
    }
    pub async fn observe(&self, ctx: &RunContext, lease: &mut Lease) -> Result<Observation> {
        self.check(ctx, lease)?;
        ctx.capabilities().require("computer.observe")?;
        let obs = tokio::select! {_=ctx.cancelled()=>return Err(Error::Cancelled),r=self.backend.observe(ctx)=>r?};
        if obs.frame_id.is_nil()
            || obs.width == 0
            || obs.height == 0
            || obs.width > 16384
            || obs.height > 16384
            || obs.png.len() > 16 * 1024 * 1024
            || !obs.png.starts_with(b"\x89PNG\r\n\x1a\n")
        {
            return Err(Error::Invalid("invalid/oversized screenshot".into()));
        }
        self.check(ctx, lease)?;
        lease.frame = Some((
            obs.frame_id,
            obs.width,
            obs.height,
            obs.target.clone(),
            Instant::now(),
        ));
        Ok(obs)
    }
    pub fn approval_binding(
        &self,
        ctx: &RunContext,
        lease: &Lease,
        frame: Uuid,
        action: &Action,
    ) -> Result<ApprovalBinding> {
        self.check(ctx, lease)?;
        ctx.capabilities().require("computer.input")?;
        let (id, width, height, target, captured) = lease
            .frame
            .as_ref()
            .ok_or_else(|| Error::Denied("observe before acting".into()))?;
        if *id != frame || captured.elapsed() > Duration::from_secs(30) {
            return Err(Error::Denied("stale frame; observe again".into()));
        }
        match action {
            Action::Click { x, y } if *x >= *width || *y >= *height => {
                return Err(Error::Invalid("click outside frame".into()))
            }
            Action::Type { text }
                if text.is_empty() || text.len() > 4096 || text.chars().any(|c| c.is_control()) =>
            {
                return Err(Error::Invalid("type text bounds/control characters".into()))
            }
            Action::Key { key }
                if ![
                    "Return",
                    "Tab",
                    "Escape",
                    "BackSpace",
                    "Delete",
                    "Left",
                    "Right",
                    "Up",
                    "Down",
                    "Home",
                    "End",
                    "Page_Up",
                    "Page_Down",
                ]
                .contains(&key.as_str()) =>
            {
                return Err(Error::Denied("key not supported".into()))
            }
            Action::Scroll { steps, .. } if !(1..=20).contains(steps) => {
                return Err(Error::Invalid("scroll bounds".into()))
            }
            _ => {}
        }
        ApprovalBinding::new(
            ctx,
            "computer.action",
            &(self.device, frame, target, action),
        )
    }
    pub async fn act(
        &self,
        ctx: &RunContext,
        lease: &mut Lease,
        frame: Uuid,
        action: Action,
        ticket: ApprovalTicket,
        vault: &ApprovalVault,
    ) -> Result<()> {
        let binding = self.approval_binding(ctx, lease, frame, &action)?;
        vault.consume(ticket, &binding)?;
        let (_, _, _, target, _) = lease
            .frame
            .take()
            .ok_or_else(|| Error::Denied("missing observation".into()))?;
        self.check(ctx, lease)?; // every action invalidates the frame even on failure
        tokio::select! {_=ctx.cancelled()=>Err(Error::Uncertain("computer action interrupted".into())),r=self.backend.act(ctx,&target,&action)=>r}
    }
}
/// Concrete X11 backend, opt-in only. Does not claim Wayland/macOS/Windows support.
/// Use a dedicated virtual desktop/container. Native desktop permission is broad:
/// focus races cannot be eliminated by application-level checks alone.
pub struct X11Backend {
    pub xdotool: PathBuf,
    pub imagemagick_import: PathBuf,
    pub private_cwd: PathBuf,
    pub display: String,
    pub xauthority: Option<PathBuf>,
}
impl X11Backend {
    async fn command(&self, ctx: &RunContext, exe: PathBuf, args: Vec<String>) -> Result<Vec<u8>> {
        if !cfg!(target_os = "linux") {
            return Err(Error::Unsupported("X11 backend requires Linux".into()));
        }
        if self.display.is_empty() || self.display.len() > 128 {
            return Err(Error::Invalid("X11 display".into()));
        }
        let mut env = BTreeMap::new();
        env.insert("DISPLAY".into(), self.display.clone());
        env.insert("LC_ALL".into(), "C.UTF-8".into());
        if let Some(path) = &self.xauthority {
            env.insert("XAUTHORITY".into(), path.to_string_lossy().into_owned());
        }
        let result = run_host_command(
            ctx,
            &CommandSpec {
                executable: exe,
                arguments: args,
                cwd: self.private_cwd.clone(),
                environment: env,
                timeout: Duration::from_secs(15),
                output_limit: 16 * 1024 * 1024,
            },
            &[self.xdotool.clone(), self.imagemagick_import.clone()],
        )
        .await?;
        if result.code != Some(0) {
            return Err(Error::Host(
                "X11 helper failed; inspect private host diagnostics".into(),
            ));
        }
        Ok(result.stdout)
    }
    async fn target(&self, ctx: &RunContext) -> Result<String> {
        let out = self
            .command(ctx, self.xdotool.clone(), vec!["getactivewindow".into()])
            .await?;
        let target = String::from_utf8(out)
            .map_err(|_| Error::Host("window id encoding".into()))?
            .trim()
            .to_string();
        if target.is_empty() || target.len() > 20 || !target.bytes().all(|b| b.is_ascii_digit()) {
            return Err(Error::Invalid("window id".into()));
        }
        Ok(target)
    }
}
#[async_trait]
impl ComputerBackend for X11Backend {
    async fn observe(&self, ctx: &RunContext) -> Result<Observation> {
        let target = self.target(ctx).await?;
        let png = self
            .command(
                ctx,
                self.imagemagick_import.clone(),
                vec!["-window".into(), target.clone(), "png:-".into()],
            )
            .await?;
        if png.len() < 24 || !png.starts_with(b"\x89PNG\r\n\x1a\n") || &png[12..16] != b"IHDR" {
            return Err(Error::Invalid("PNG header".into()));
        }
        let width = u32::from_be_bytes(
            png[16..20]
                .try_into()
                .map_err(|_| Error::Invalid("PNG width".into()))?,
        );
        let height = u32::from_be_bytes(
            png[20..24]
                .try_into()
                .map_err(|_| Error::Invalid("PNG height".into()))?,
        );
        Ok(Observation {
            frame_id: Uuid::new_v4(),
            width,
            height,
            png,
            target,
        })
    }
    async fn act(&self, ctx: &RunContext, target: &str, action: &Action) -> Result<()> {
        if self.target(ctx).await? != target {
            return Err(Error::Denied(
                "foreground changed; observe and approve again".into(),
            ));
        }
        let args = match action {
            Action::Click { x, y } => vec![
                "mousemove".into(),
                "--window".into(),
                target.into(),
                x.to_string(),
                y.to_string(),
                "click".into(),
                "1".into(),
            ],
            Action::Type { text } => vec![
                "type".into(),
                "--window".into(),
                target.into(),
                "--clearmodifiers".into(),
                "--".into(),
                text.clone(),
            ],
            Action::Key { key } => vec![
                "key".into(),
                "--window".into(),
                target.into(),
                "--clearmodifiers".into(),
                key.clone(),
            ],
            Action::Scroll { down, steps } => vec![
                "click".into(),
                "--window".into(),
                target.into(),
                "--repeat".into(),
                steps.to_string(),
                if *down { "5" } else { "4" }.into(),
            ],
        };
        match self.command(ctx, self.xdotool.clone(), args).await {
            Ok(_) => Ok(()),
            Err(_) => Err(Error::Uncertain(
                "input may have been partially applied".into(),
            )),
        }
    }
}

/// Host-enrolled macOS ScreenCapture observe backend. Input (click/type) is
/// not implemented here: Accessibility is a separate, unverified enrollment.
pub struct MacScreenCaptureBackend {
    pub screencapture: PathBuf,
    pub private_cwd: PathBuf,
}

impl MacScreenCaptureBackend {
    pub fn host_default(private_cwd: PathBuf) -> Result<Self> {
        if !cfg!(target_os = "macos") {
            return Err(Error::Unsupported(
                "macOS ScreenCapture backend requires macOS".into(),
            ));
        }
        let screencapture = PathBuf::from("/usr/sbin/screencapture");
        if !screencapture.is_file() {
            return Err(Error::Unsupported("screencapture is not installed".into()));
        }
        if !private_cwd.is_absolute() || !private_cwd.is_dir() {
            return Err(Error::Invalid("macOS capture cwd".into()));
        }
        Ok(Self {
            screencapture,
            private_cwd,
        })
    }
}

fn png_dimensions(png: &[u8]) -> Result<(u32, u32)> {
    if png.len() < 24 || !png.starts_with(b"\x89PNG\r\n\x1a\n") || &png[12..16] != b"IHDR" {
        return Err(Error::Invalid("PNG header".into()));
    }
    let width = u32::from_be_bytes(
        png[16..20]
            .try_into()
            .map_err(|_| Error::Invalid("PNG width".into()))?,
    );
    let height = u32::from_be_bytes(
        png[20..24]
            .try_into()
            .map_err(|_| Error::Invalid("PNG height".into()))?,
    );
    Ok((width, height))
}

#[async_trait]
impl ComputerBackend for MacScreenCaptureBackend {
    async fn observe(&self, ctx: &RunContext) -> Result<Observation> {
        if !cfg!(target_os = "macos") {
            return Err(Error::Unsupported(
                "macOS ScreenCapture backend requires macOS".into(),
            ));
        }
        ctx.check()?;
        let out = self.private_cwd.join(format!("{}.png", Uuid::new_v4()));
        let result = run_host_command(
            ctx,
            &CommandSpec {
                executable: self.screencapture.clone(),
                arguments: vec![
                    "-x".into(),
                    "-t".into(),
                    "png".into(),
                    out.to_string_lossy().into_owned(),
                ],
                cwd: self.private_cwd.clone(),
                environment: BTreeMap::new(),
                timeout: Duration::from_secs(15),
                output_limit: 16 * 1024,
            },
            std::slice::from_ref(&self.screencapture),
        )
        .await;
        let png = match result {
            Ok(r) if r.code == Some(0) && out.is_file() => {
                let bytes = std::fs::read(&out);
                let _ = std::fs::remove_file(&out);
                bytes?
            }
            Ok(_) | Err(_) => {
                let _ = std::fs::remove_file(&out);
                return Err(Error::Host(
                    "screencapture failed; inspect host Screen Recording permission".into(),
                ));
            }
        };
        let (width, height) = png_dimensions(&png)?;
        Ok(Observation {
            frame_id: Uuid::new_v4(),
            width,
            height,
            png,
            target: "display".into(),
        })
    }

    async fn act(&self, ctx: &RunContext, _target: &str, _action: &Action) -> Result<()> {
        ctx.check()?;
        Err(Error::Unsupported(
            "macOS input requires a separate Accessibility enrollment; this backend is observe-only"
                .into(),
        ))
    }
}
