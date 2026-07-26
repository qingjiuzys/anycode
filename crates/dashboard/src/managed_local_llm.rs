//! Manifest-driven managed local model runtimes.

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

pub const MINICPM5_PRESET_ID: &str = "managed-minicpm5-1b";
const SGLANG_WEIGHTS_DIR: &str = "weights";
const SGLANG_HF_REPO_APPLE: &str = "mlx-community/MiniCPM5-1B-4bit";
const SGLANG_HF_REPO_CUDA: &str = "openbmb/MiniCPM5-1B";
const OLLAMA_MANAGED_MODEL: &str = "minicpm5-1b-managed";
const OLLAMA_DEFAULT_PORT: u16 = 11_434;

fn use_ollama_on_aarch64() -> bool {
    std::env::var("ANYCODE_MANAGED_OLLAMA")
        .ok()
        .is_some_and(|v| matches!(v.trim(), "1" | "true" | "yes"))
}

fn builtin_ollama_minicpm5_descriptor() -> ManagedModelDescriptor {
    ManagedModelDescriptor {
        id: MINICPM5_PRESET_ID.into(),
        version: "ollama-minicpm5-2026-07".into(),
        display_name: "MiniCPM5-1B (Ollama · legacy, no native tool_calls)".into(),
        model_id: OLLAMA_MANAGED_MODEL.into(),
        file_name: ".installed".into(),
        download_url: "ollama://minicpm5-1b".into(),
        sha256: "skip".repeat(16),
        size_bytes: 688_000_000,
        architectures: vec!["aarch64".into()],
        context_tokens: 32_768,
        minimum_ram_bytes: 4 * 1024 * 1024 * 1024,
        license: "Apache-2.0".into(),
        capabilities: RuntimeCapabilities {
            chat: true,
            tools: true,
            vision: false,
        },
        runtime: "ollama".into(),
        runtime_args: Vec::new(),
        preview: false,
    }
}

fn builtin_sglang_minicpm5_descriptor(
    hf_repo: &str,
    size_bytes: u64,
    version: &str,
    display_suffix: &str,
) -> ManagedModelDescriptor {
    ManagedModelDescriptor {
        id: MINICPM5_PRESET_ID.into(),
        version: version.into(),
        display_name: format!("MiniCPM5-1B ({display_suffix})"),
        model_id: "minicpm5-1b".into(),
        file_name: SGLANG_WEIGHTS_DIR.into(),
        download_url: format!("hf://{hf_repo}"),
        sha256: "skip".repeat(16),
        size_bytes,
        architectures: vec![std::env::consts::ARCH.into()],
        context_tokens: 32_768,
        minimum_ram_bytes: 4 * 1024 * 1024 * 1024,
        license: "Apache-2.0".into(),
        capabilities: RuntimeCapabilities {
            chat: true,
            tools: true,
            vision: false,
        },
        runtime: "sglang".into(),
        runtime_args: Vec::new(),
        preview: false,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ManagedLocalPhase {
    NotInstalled,
    Downloading,
    Ready,
    Starting,
    Running,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeCapabilities {
    pub chat: bool,
    pub tools: bool,
    pub vision: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManagedModelDescriptor {
    pub id: String,
    pub version: String,
    pub display_name: String,
    pub model_id: String,
    pub file_name: String,
    pub download_url: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub architectures: Vec<String>,
    pub context_tokens: u32,
    pub minimum_ram_bytes: u64,
    pub license: String,
    pub capabilities: RuntimeCapabilities,
    pub runtime: String,
    pub runtime_args: Vec<String>,
    pub preview: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagedLocalStatus {
    #[serde(flatten)]
    pub descriptor: ManagedModelDescriptor,
    pub phase: ManagedLocalPhase,
    pub model_path: Option<String>,
    pub base_url: Option<String>,
    pub port: Option<u16>,
    pub download_bytes: u64,
    pub download_total: u64,
    pub disk_free_bytes: Option<u64>,
    pub ram_total_bytes: Option<u64>,
    /// Legacy `/local-llm/status` compatibility.
    pub tool_calls_supported: bool,
    pub last_error: Option<String>,
}

struct RuntimeState {
    phase: ManagedLocalPhase,
    download_bytes: u64,
    download_total: u64,
    port: Option<u16>,
    child: Option<Child>,
    last_error: Option<String>,
    cancel_download: bool,
}

struct ManagedRuntimeInner {
    root: PathBuf,
    descriptors: HashMap<String, ManagedModelDescriptor>,
    states: HashMap<String, RuntimeState>,
}

#[derive(Clone)]
pub struct ManagedRuntimeManager {
    inner: Arc<Mutex<ManagedRuntimeInner>>,
}

/// Source compatibility for the original single-model integration.
pub type ManagedLocalLlm = ManagedRuntimeManager;

impl Default for ManagedRuntimeManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ManagedRuntimeManager {
    pub fn new() -> Self {
        let root = anycode_llm::anycode_models_dir();
        let mut descriptors = builtin_descriptors();
        match load_manifest_descriptors(&root) {
            Ok(custom) => {
                for descriptor in custom {
                    if let Some(existing) = descriptors
                        .iter_mut()
                        .find(|existing| existing.id == descriptor.id)
                    {
                        *existing = descriptor;
                    } else {
                        descriptors.push(descriptor);
                    }
                }
            }
            Err(error) => tracing::warn!(%error, "ignoring invalid local model manifest"),
        }
        Self::with_descriptors(root, descriptors)
            .expect("built-in local model descriptors must be valid")
    }

    pub fn with_descriptors(
        root: PathBuf,
        descriptors: Vec<ManagedModelDescriptor>,
    ) -> Result<Self> {
        let mut by_id = HashMap::new();
        let mut states = HashMap::new();
        for descriptor in descriptors {
            validate_descriptor(&descriptor)?;
            if by_id
                .insert(descriptor.id.clone(), descriptor.clone())
                .is_some()
            {
                return Err(anyhow!("duplicate local model id: {}", descriptor.id));
            }
            let installed = is_model_installed(&root, &descriptor);
            states.insert(
                descriptor.id.clone(),
                RuntimeState {
                    phase: if installed {
                        ManagedLocalPhase::Ready
                    } else {
                        ManagedLocalPhase::NotInstalled
                    },
                    download_bytes: partial_download_bytes_for(&root, &descriptor),
                    download_total: descriptor.size_bytes,
                    port: None,
                    child: None,
                    last_error: None,
                    cancel_download: false,
                },
            );
        }
        Ok(Self {
            inner: Arc::new(Mutex::new(ManagedRuntimeInner {
                root,
                descriptors: by_id,
                states,
            })),
        })
    }

    pub async fn list_status(&self) -> Vec<ManagedLocalStatus> {
        let ids = {
            let inner = self.inner.lock().await;
            let mut ids = inner.descriptors.keys().cloned().collect::<Vec<_>>();
            ids.sort();
            ids
        };
        let mut statuses = Vec::with_capacity(ids.len());
        for id in ids {
            if let Ok(status) = self.status(&id).await {
                statuses.push(status);
            }
        }
        statuses
    }

    pub async fn status(&self, id: &str) -> Result<ManagedLocalStatus> {
        let mut inner = self.inner.lock().await;
        let descriptor = inner
            .descriptors
            .get(id)
            .cloned()
            .ok_or_else(|| anyhow!("unknown local model: {id}"))?;
        let root = inner.root.clone();
        let state = inner.states.get_mut(id).expect("descriptor state");
        if let Some(child) = state.child.as_mut() {
            if let Some(exit) = child.try_wait().context("inspect runtime process")? {
                state.child = None;
                state.port = None;
                state.phase = ManagedLocalPhase::Error;
                state.last_error = Some(format!("runtime exited: {exit}"));
            }
        }
        let path = model_storage_path(&root, &descriptor);
        let tool_calls_supported = descriptor.capabilities.tools;
        let model_path = is_model_installed(&root, &descriptor).then(|| path.display().to_string());
        Ok(ManagedLocalStatus {
            descriptor,
            phase: state.phase.clone(),
            model_path,
            base_url: state.port.map(|port| format!("http://127.0.0.1:{port}/v1")),
            port: state.port,
            download_bytes: state.download_bytes,
            download_total: state.download_total,
            disk_free_bytes: disk_free_bytes(&root),
            ram_total_bytes: system_ram_bytes(),
            tool_calls_supported,
            last_error: state.last_error.clone(),
        })
    }

    pub async fn legacy_status(&self) -> ManagedLocalStatus {
        self.status(MINICPM5_PRESET_ID)
            .await
            .expect("built-in MiniCPM descriptor")
    }

    pub async fn start_download(&self, id: &str) -> Result<()> {
        {
            let mut inner = self.inner.lock().await;
            let descriptor = inner
                .descriptors
                .get(id)
                .cloned()
                .ok_or_else(|| anyhow!("unknown local model: {id}"))?;
            ensure_compatible(&descriptor)?;
            let root = inner.root.clone();
            let state = inner.states.get_mut(id).expect("descriptor state");
            if state.phase == ManagedLocalPhase::Downloading {
                return Ok(());
            }
            state.cancel_download = false;
            state.phase = ManagedLocalPhase::Downloading;
            state.download_bytes = partial_download_bytes_for(&root, &descriptor);
            state.last_error = None;
        }
        let manager = self.clone();
        let id = id.to_string();
        tokio::spawn(async move {
            if let Err(error) = manager.download_model(&id).await {
                let mut inner = manager.inner.lock().await;
                if let Some(state) = inner.states.get_mut(&id) {
                    state.phase = ManagedLocalPhase::Error;
                    state.last_error = Some(error.to_string());
                }
            }
        });
        Ok(())
    }

    pub async fn cancel_download(&self, id: &str) -> Result<()> {
        let mut inner = self.inner.lock().await;
        let descriptor = inner
            .descriptors
            .get(id)
            .cloned()
            .ok_or_else(|| anyhow!("unknown local model: {id}"))?;
        let installed = is_model_installed(&inner.root, &descriptor);
        let state = inner.states.get_mut(id).expect("descriptor state");
        state.cancel_download = true;
        if state.phase == ManagedLocalPhase::Downloading {
            state.phase = if installed {
                ManagedLocalPhase::Ready
            } else {
                ManagedLocalPhase::NotInstalled
            };
        }
        Ok(())
    }

    pub async fn delete_model(&self, id: &str) -> Result<()> {
        self.stop(id).await?;
        let (root, descriptor) = self.descriptor_and_root(id).await?;
        if descriptor.runtime == "ollama" {
            if let Some(ollama) = resolve_ollama_binary() {
                let ollama_string = ollama.display().to_string();
                let managed = OLLAMA_MANAGED_MODEL.to_string();
                let _ = tokio::task::spawn_blocking(move || {
                    let _ = std::process::Command::new(&ollama_string)
                        .args(["rm", &managed])
                        .status();
                })
                .await;
            }
        }
        let version_dir = model_version_dir(&root, &descriptor);
        if version_dir.is_dir() {
            tokio::fs::remove_dir_all(&version_dir).await?;
        }
        let mut inner = self.inner.lock().await;
        let state = inner.states.get_mut(id).expect("descriptor state");
        state.phase = ManagedLocalPhase::NotInstalled;
        state.download_bytes = 0;
        state.last_error = None;
        Ok(())
    }

    pub async fn start(&self, id: &str) -> Result<()> {
        let (root, descriptor) = self.descriptor_and_root(id).await?;
        ensure_compatible(&descriptor)?;
        if !is_model_installed(&root, &descriptor) {
            return Err(anyhow!("model not installed"));
        }
        match descriptor.runtime.as_str() {
            "ollama" => self.start_ollama(id, &descriptor).await,
            "sglang" => self.start_sglang(id, &root, &descriptor).await,
            other => Err(anyhow!("unsupported managed runtime: {other}")),
        }
    }

    async fn start_ollama(&self, id: &str, descriptor: &ManagedModelDescriptor) -> Result<()> {
        let ollama = resolve_ollama_binary().ok_or_else(|| {
            anyhow!("ollama not found — install from https://ollama.com/download")
        })?;
        if system_ram_bytes().is_some_and(|ram| ram < descriptor.minimum_ram_bytes) {
            return Err(anyhow!(
                "insufficient RAM: requires at least {} bytes",
                descriptor.minimum_ram_bytes
            ));
        }
        {
            let inner = self.inner.lock().await;
            let state = inner.states.get(id).expect("descriptor state");
            if state.phase == ManagedLocalPhase::Running {
                return Ok(());
            }
        }
        {
            let mut inner = self.inner.lock().await;
            let state = inner.states.get_mut(id).expect("descriptor state");
            state.phase = ManagedLocalPhase::Starting;
            state.last_error = None;
        }
        let mut spawned_child = None;
        if !ollama_api_up().await {
            let child = Command::new(&ollama)
                .arg("serve")
                .kill_on_drop(true)
                .spawn()
                .with_context(|| format!("spawn {}", ollama.display()))?;
            spawned_child = Some(child);
            if !wait_for_ollama_api(30).await {
                if let Some(mut child) = spawned_child.take() {
                    let _ = child.kill().await;
                }
                let mut inner = self.inner.lock().await;
                let state = inner.states.get_mut(id).expect("descriptor state");
                state.phase = ManagedLocalPhase::Error;
                state.last_error = Some("ollama serve failed to start".into());
                return Err(anyhow!("ollama API not reachable"));
            }
        }
        if !ollama_has_model(OLLAMA_MANAGED_MODEL).await {
            if let Some(mut child) = spawned_child.take() {
                let _ = child.kill().await;
            }
            let mut inner = self.inner.lock().await;
            let state = inner.states.get_mut(id).expect("descriptor state");
            state.phase = ManagedLocalPhase::Error;
            state.last_error = Some(format!("ollama model {OLLAMA_MANAGED_MODEL} missing"));
            return Err(anyhow!("managed ollama model not installed"));
        }
        sync_registry(descriptor, OLLAMA_DEFAULT_PORT)?;
        let mut inner = self.inner.lock().await;
        let state = inner.states.get_mut(id).expect("descriptor state");
        state.port = Some(OLLAMA_DEFAULT_PORT);
        state.child = spawned_child;
        state.phase = ManagedLocalPhase::Running;
        Ok(())
    }

    pub async fn stop(&self, id: &str) -> Result<()> {
        let mut inner = self.inner.lock().await;
        let descriptor = inner
            .descriptors
            .get(id)
            .cloned()
            .ok_or_else(|| anyhow!("unknown local model: {id}"))?;
        let installed = is_model_installed(&inner.root, &descriptor);
        let state = inner.states.get_mut(id).expect("descriptor state");
        if let Some(mut child) = state.child.take() {
            let _ = child.kill().await;
            let _ = child.wait().await;
        }
        state.port = None;
        state.phase = if installed {
            ManagedLocalPhase::Ready
        } else {
            ManagedLocalPhase::NotInstalled
        };
        Ok(())
    }

    async fn descriptor_and_root(&self, id: &str) -> Result<(PathBuf, ManagedModelDescriptor)> {
        let inner = self.inner.lock().await;
        let descriptor = inner
            .descriptors
            .get(id)
            .cloned()
            .ok_or_else(|| anyhow!("unknown local model: {id}"))?;
        Ok((inner.root.clone(), descriptor))
    }

    async fn download_model(&self, id: &str) -> Result<()> {
        let (root, descriptor) = self.descriptor_and_root(id).await?;
        match descriptor.runtime.as_str() {
            "ollama" => self.download_ollama_model(id, &root, &descriptor).await,
            "sglang" => self.download_sglang_model(id, &root, &descriptor).await,
            _ => self.download_model_legacy(id, &root, &descriptor).await,
        }
    }

    async fn download_model_legacy(
        &self,
        id: &str,
        root: &Path,
        descriptor: &ManagedModelDescriptor,
    ) -> Result<()> {
        let dir = model_version_dir(root, descriptor);
        tokio::fs::create_dir_all(&dir).await?;
        let partial = partial_download_path_for(root, descriptor);
        let mut offset = partial.metadata().map(|m| m.len()).unwrap_or(0);
        if offset > descriptor.size_bytes {
            tokio::fs::remove_file(&partial).await?;
            offset = 0;
        }
        let remaining = descriptor.size_bytes.saturating_sub(offset);
        if disk_free_bytes(root).is_some_and(|free| free < remaining + 128 * 1024 * 1024) {
            return Err(anyhow!("insufficient disk space"));
        }
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(3600))
            .build()?;
        let mut response = request_download(&client, &descriptor.download_url, offset).await?;
        if offset > 0 && !valid_range_response(response.status(), response.headers(), offset) {
            offset = 0;
            response = request_download(&client, &descriptor.download_url, 0).await?;
        }
        if !response.status().is_success() {
            return Err(anyhow!("download failed: {}", response.status()));
        }
        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .append(offset > 0)
            .truncate(offset == 0)
            .open(&partial)
            .await?;
        let mut stream = response.bytes_stream();
        use futures::StreamExt;
        while let Some(chunk) = stream.next().await {
            {
                let inner = self.inner.lock().await;
                if inner
                    .states
                    .get(id)
                    .is_some_and(|state| state.cancel_download)
                {
                    return Ok(());
                }
            }
            let chunk = chunk?;
            file.write_all(&chunk).await?;
            offset += chunk.len() as u64;
            let mut inner = self.inner.lock().await;
            inner
                .states
                .get_mut(id)
                .expect("descriptor state")
                .download_bytes = offset;
        }
        file.flush().await?;
        if offset != descriptor.size_bytes {
            return Err(anyhow!(
                "download size mismatch: expected {}, got {offset}",
                descriptor.size_bytes
            ));
        }
        verify_sha256(&partial, &descriptor.sha256).await?;
        tokio::fs::rename(&partial, model_file_path_for(root, descriptor)).await?;
        let mut inner = self.inner.lock().await;
        let state = inner.states.get_mut(id).expect("descriptor state");
        state.phase = ManagedLocalPhase::Ready;
        state.download_bytes = descriptor.size_bytes;
        Ok(())
    }

    async fn download_ollama_model(
        &self,
        id: &str,
        root: &Path,
        descriptor: &ManagedModelDescriptor,
    ) -> Result<()> {
        let ollama = resolve_ollama_binary().ok_or_else(|| {
            anyhow!("ollama not found — install from https://ollama.com/download")
        })?;
        let base_model = ollama_base_model_from_descriptor(descriptor)?;
        let marker = model_storage_path(root, descriptor);
        tokio::fs::create_dir_all(marker.parent().unwrap_or(root)).await?;
        if marker.is_file() && ollama_has_model(OLLAMA_MANAGED_MODEL).await {
            let mut inner = self.inner.lock().await;
            let state = inner.states.get_mut(id).expect("descriptor state");
            state.phase = ManagedLocalPhase::Ready;
            state.download_bytes = descriptor.size_bytes;
            return Ok(());
        }
        if disk_free_bytes(root)
            .is_some_and(|free| free < descriptor.size_bytes + 256 * 1024 * 1024)
        {
            return Err(anyhow!("insufficient disk space"));
        }
        {
            let mut inner = self.inner.lock().await;
            inner
                .states
                .get_mut(id)
                .expect("descriptor state")
                .download_bytes = descriptor.size_bytes / 20;
        }
        let ollama_string = ollama.display().to_string();
        let base_model_string = base_model.to_string();
        let id_string = id.to_string();
        let manager = self.clone();
        if !ollama_has_model(base_model).await {
            let ollama_pull = ollama_string.clone();
            tokio::task::spawn_blocking(move || run_ollama_pull(&ollama_pull, &base_model_string))
                .await
                .context("ollama pull task")??;
        }
        {
            let inner = manager.inner.lock().await;
            if inner
                .states
                .get(&id_string)
                .is_some_and(|state| state.cancel_download)
            {
                return Ok(());
            }
        }
        {
            let mut inner = self.inner.lock().await;
            inner
                .states
                .get_mut(id)
                .expect("descriptor state")
                .download_bytes = descriptor.size_bytes * 9 / 10;
        }
        let modelfile = format!(
            "FROM {base_model}:latest\nPARAMETER num_ctx {}\n",
            descriptor.context_tokens
        );
        let ollama_create = ollama_string.clone();
        tokio::task::spawn_blocking(move || {
            run_ollama_create(&ollama_create, OLLAMA_MANAGED_MODEL, &modelfile)
        })
        .await
        .context("ollama create task")??;
        if !ollama_has_model(OLLAMA_MANAGED_MODEL).await {
            return Err(anyhow!("ollama create {OLLAMA_MANAGED_MODEL} failed"));
        }
        tokio::fs::write(&marker, format!("ollama:{OLLAMA_MANAGED_MODEL}\n")).await?;
        let mut inner = self.inner.lock().await;
        let state = inner.states.get_mut(id).expect("descriptor state");
        state.phase = ManagedLocalPhase::Ready;
        state.download_bytes = descriptor.size_bytes;
        Ok(())
    }

    async fn download_sglang_model(
        &self,
        id: &str,
        root: &Path,
        descriptor: &ManagedModelDescriptor,
    ) -> Result<()> {
        let repo = hf_repo_from_descriptor(descriptor)?;
        let weights_dir = model_storage_path(root, descriptor);
        tokio::fs::create_dir_all(&weights_dir).await?;
        if weights_dir.join("config.json").is_file() {
            let mut inner = self.inner.lock().await;
            let state = inner.states.get_mut(id).expect("descriptor state");
            state.phase = ManagedLocalPhase::Ready;
            state.download_bytes = descriptor.size_bytes;
            return Ok(());
        }
        let remaining = descriptor.size_bytes;
        if disk_free_bytes(root).is_some_and(|free| free < remaining + 512 * 1024 * 1024) {
            return Err(anyhow!("insufficient disk space"));
        }
        let venv = sglang_venv_path();
        let python = sglang_python_path();
        tokio::task::spawn_blocking(move || install_sglang_venv(&venv))
            .await
            .context("sglang venv install task")??;
        {
            let mut inner = self.inner.lock().await;
            inner
                .states
                .get_mut(id)
                .expect("descriptor state")
                .download_bytes = descriptor.size_bytes / 10;
        }
        let weights_string = weights_dir.display().to_string();
        let repo = repo.to_string();
        let python_clone = python.clone();
        tokio::task::spawn_blocking(move || {
            install_hf_weights(&python_clone, &repo, Path::new(&weights_string))
        })
        .await
        .context("hf download task")??;
        if !weights_dir.join("config.json").is_file() {
            return Err(anyhow!(
                "huggingface download incomplete: missing config.json"
            ));
        }
        let mut inner = self.inner.lock().await;
        let state = inner.states.get_mut(id).expect("descriptor state");
        state.phase = ManagedLocalPhase::Ready;
        state.download_bytes = descriptor.size_bytes;
        Ok(())
    }

    async fn start_sglang(
        &self,
        id: &str,
        root: &Path,
        descriptor: &ManagedModelDescriptor,
    ) -> Result<()> {
        let weights = model_storage_path(root, descriptor);
        if !weights.join("config.json").is_file() {
            return Err(anyhow!("model weights not installed"));
        }
        if system_ram_bytes().is_some_and(|ram| ram < descriptor.minimum_ram_bytes) {
            return Err(anyhow!(
                "insufficient RAM: requires at least {} bytes",
                descriptor.minimum_ram_bytes
            ));
        }
        let python = sglang_python_path();
        if !python.is_file() {
            return Err(anyhow!(
                "sglang runtime missing at {}; download the model first",
                python.display()
            ));
        }
        let port = pick_loopback_port()?;
        {
            let mut inner = self.inner.lock().await;
            let state = inner.states.get_mut(id).expect("descriptor state");
            if state.child.is_some() {
                return Ok(());
            }
            state.phase = ManagedLocalPhase::Starting;
            state.last_error = None;
            let mut cmd = Command::new(&python);
            cmd.arg("-m")
                .arg("sglang.launch_server")
                .arg("--model-path")
                .arg(&weights)
                .arg("--served-model-name")
                .arg(&descriptor.model_id)
                .arg("--tool-call-parser")
                .arg("minicpm5")
                .arg("--context-length")
                .arg(descriptor.context_tokens.to_string())
                .arg("--host")
                .arg("127.0.0.1")
                .arg("--port")
                .arg(port.to_string())
                .arg("--disable-cuda-graph");
            if cfg!(target_arch = "aarch64") {
                cmd.env("SGLANG_USE_MLX", "1");
            }
            cmd.env("SGLANG_ALLOW_OVERWRITE_LONGER_CONTEXT_LEN", "1");
            let child = cmd
                .kill_on_drop(true)
                .spawn()
                .with_context(|| format!("spawn sglang at {}", python.display()))?;
            state.port = Some(port);
            state.child = Some(child);
        }
        if !wait_for_health(port, 300).await {
            self.stop(id).await?;
            let mut inner = self.inner.lock().await;
            let state = inner.states.get_mut(id).expect("descriptor state");
            state.phase = ManagedLocalPhase::Error;
            state.last_error = Some("sglang health check failed".into());
            return Err(anyhow!("sglang failed to start"));
        }
        sync_registry(descriptor, port)?;
        let mut inner = self.inner.lock().await;
        inner.states.get_mut(id).expect("descriptor state").phase = ManagedLocalPhase::Running;
        Ok(())
    }
}

pub fn builtin_descriptors() -> Vec<ManagedModelDescriptor> {
    if cfg!(target_arch = "aarch64") {
        if use_ollama_on_aarch64() {
            return vec![builtin_ollama_minicpm5_descriptor()];
        }
        return vec![builtin_sglang_minicpm5_descriptor(
            SGLANG_HF_REPO_APPLE,
            750_000_000,
            "sglang-minicpm5-mlx-2026-07",
            "SGLang · native tools",
        )];
    }
    vec![builtin_sglang_minicpm5_descriptor(
        SGLANG_HF_REPO_CUDA,
        2_200_000_000,
        "sglang-minicpm5-2026-07",
        "SGLang",
    )]
}

fn load_manifest_descriptors(root: &Path) -> Result<Vec<ManagedModelDescriptor>> {
    let path = std::env::var("ANYCODE_LOCAL_MODEL_MANIFEST")
        .map(PathBuf::from)
        .unwrap_or_else(|_| root.join("manifest.json"));
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let descriptors: Vec<ManagedModelDescriptor> = serde_json::from_slice(
        &std::fs::read(&path).with_context(|| format!("read {}", path.display()))?,
    )
    .with_context(|| format!("parse {}", path.display()))?;
    for descriptor in &descriptors {
        validate_descriptor(descriptor)?;
    }
    Ok(descriptors)
}

fn validate_descriptor(descriptor: &ManagedModelDescriptor) -> Result<()> {
    if descriptor.id.trim().is_empty()
        || descriptor.version.trim().is_empty()
        || descriptor.file_name.trim().is_empty()
    {
        return Err(anyhow!("local model descriptor has empty identity fields"));
    }
    let managed_runtime = descriptor.runtime == "sglang" || descriptor.runtime == "ollama";
    if !managed_runtime && descriptor.download_url.trim().is_empty() {
        return Err(anyhow!("download_url required for {}", descriptor.id));
    }
    if !managed_runtime
        && (descriptor.sha256.len() != 64
            || !descriptor.sha256.chars().all(|c| c.is_ascii_hexdigit()))
    {
        return Err(anyhow!("invalid SHA-256 for {}", descriptor.id));
    }
    if descriptor.size_bytes == 0 || descriptor.context_tokens == 0 {
        return Err(anyhow!("invalid size/context for {}", descriptor.id));
    }
    Ok(())
}

fn ensure_compatible(descriptor: &ManagedModelDescriptor) -> Result<()> {
    let arch = std::env::consts::ARCH;
    if !descriptor
        .architectures
        .iter()
        .any(|candidate| candidate == arch)
    {
        return Err(anyhow!(
            "model {} does not support architecture {arch}",
            descriptor.id
        ));
    }
    Ok(())
}

fn model_version_dir(root: &Path, descriptor: &ManagedModelDescriptor) -> PathBuf {
    root.join(&descriptor.id).join(&descriptor.version)
}

fn model_file_path_for(root: &Path, descriptor: &ManagedModelDescriptor) -> PathBuf {
    model_version_dir(root, descriptor).join(&descriptor.file_name)
}

fn model_storage_path(root: &Path, descriptor: &ManagedModelDescriptor) -> PathBuf {
    model_file_path_for(root, descriptor)
}

fn is_model_installed(root: &Path, descriptor: &ManagedModelDescriptor) -> bool {
    let path = model_storage_path(root, descriptor);
    if descriptor.runtime == "sglang" {
        path.join("config.json").is_file()
    } else if descriptor.runtime == "ollama" {
        path.is_file()
    } else {
        path.is_file()
    }
}

fn ollama_base_model_from_descriptor(descriptor: &ManagedModelDescriptor) -> Result<&str> {
    let url = descriptor.download_url.trim();
    let Some(model) = url.strip_prefix("ollama://") else {
        return Err(anyhow!(
            "expected ollama:// download_url for ollama runtime"
        ));
    };
    if model.is_empty() {
        return Err(anyhow!("empty ollama base model id"));
    }
    Ok(model)
}

fn resolve_ollama_binary() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("ANYCODE_OLLAMA_PATH") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Some(path);
        }
    }
    std::process::Command::new("which")
        .arg("ollama")
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() {
                    return Some(PathBuf::from(path));
                }
            }
            None
        })
        .or_else(|| {
            for candidate in ["/usr/local/bin/ollama", "/opt/homebrew/bin/ollama"] {
                let path = PathBuf::from(candidate);
                if path.is_file() {
                    return Some(path);
                }
            }
            None
        })
}

fn run_ollama_pull(ollama: &str, model: &str) -> Result<()> {
    let status = std::process::Command::new(ollama)
        .args(["pull", model])
        .status()
        .with_context(|| format!("ollama pull {model}"))?;
    if !status.success() {
        return Err(anyhow!("ollama pull {model} failed"));
    }
    Ok(())
}

fn run_ollama_create(ollama: &str, name: &str, modelfile: &str) -> Result<()> {
    let temp = std::env::temp_dir().join(format!("anycode-Modelfile.{name}"));
    std::fs::write(&temp, modelfile).with_context(|| format!("write {}", temp.display()))?;
    let status = std::process::Command::new(ollama)
        .args(["create", name, "-f", &temp.display().to_string()])
        .status()
        .with_context(|| format!("ollama create {name}"))?;
    let _ = std::fs::remove_file(&temp);
    if !status.success() {
        return Err(anyhow!("ollama create {name} failed"));
    }
    Ok(())
}

async fn ollama_api_up() -> bool {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .unwrap_or_default()
        .get(format!("http://127.0.0.1:{OLLAMA_DEFAULT_PORT}/api/tags"))
        .send()
        .await
        .is_ok_and(|response| response.status().is_success())
}

async fn wait_for_ollama_api(seconds: u64) -> bool {
    for _ in 0..seconds {
        if ollama_api_up().await {
            return true;
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
    false
}

async fn ollama_has_model(name: &str) -> bool {
    let Ok(response) = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap_or_default()
        .get(format!("http://127.0.0.1:{OLLAMA_DEFAULT_PORT}/api/tags"))
        .send()
        .await
    else {
        return false;
    };
    let Ok(body) = response.text().await else {
        return false;
    };
    body.to_ascii_lowercase()
        .contains(&name.to_ascii_lowercase())
}

fn hf_repo_from_descriptor(descriptor: &ManagedModelDescriptor) -> Result<&str> {
    let url = descriptor.download_url.trim();
    let Some(repo) = url.strip_prefix("hf://") else {
        return Err(anyhow!("expected hf:// download_url for sglang runtime"));
    };
    if repo.is_empty() {
        return Err(anyhow!("empty huggingface repo id"));
    }
    Ok(repo)
}

fn sglang_venv_path() -> PathBuf {
    std::env::var("ANYCODE_SGLANG_VENV")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            anycode_llm::anycode_models_dir()
                .parent()
                .map(|p| p.join("venvs/sglang-minicpm5"))
                .unwrap_or_else(|| PathBuf::from("~/.anycode/venvs/sglang-minicpm5"))
        })
}

fn sglang_python_path() -> PathBuf {
    sglang_venv_path().join("bin/python")
}

fn partial_download_path_for(root: &Path, descriptor: &ManagedModelDescriptor) -> PathBuf {
    model_version_dir(root, descriptor).join(format!("{}.partial", descriptor.file_name))
}

fn partial_download_bytes_for(root: &Path, descriptor: &ManagedModelDescriptor) -> u64 {
    partial_download_path_for(root, descriptor)
        .metadata()
        .map(|metadata| metadata.len())
        .unwrap_or(0)
}

async fn request_download(
    client: &reqwest::Client,
    url: &str,
    offset: u64,
) -> Result<reqwest::Response> {
    let mut request = client.get(url);
    if offset > 0 {
        request = request.header(reqwest::header::RANGE, format!("bytes={offset}-"));
    }
    Ok(request.send().await?)
}

fn valid_range_response(
    status: reqwest::StatusCode,
    headers: &reqwest::header::HeaderMap,
    offset: u64,
) -> bool {
    status == reqwest::StatusCode::PARTIAL_CONTENT
        && headers
            .get(reqwest::header::CONTENT_RANGE)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.starts_with(&format!("bytes {offset}-")))
}

async fn verify_sha256(path: &Path, expected: &str) -> Result<()> {
    let mut file = tokio::fs::File::open(path).await?;
    let mut digest = Sha256::new();
    let mut buffer = vec![0_u8; 1024 * 1024];
    loop {
        let read = file.read(&mut buffer).await?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    let actual = format!("{:x}", digest.finalize());
    if !actual.eq_ignore_ascii_case(expected) {
        return Err(anyhow!(
            "SHA-256 mismatch: expected {expected}, got {actual}"
        ));
    }
    Ok(())
}

fn sync_registry(descriptor: &ManagedModelDescriptor, port: u16) -> Result<()> {
    let mut config = anycode_config::load_or_default_anycode_config(None)?;
    let provider = match descriptor.runtime.as_str() {
        "sglang" => "sglang",
        "ollama" => "ollama",
        _ => "openai",
    };
    let item = anycode_llm::ConfiguredModelFile {
        id: descriptor.id.clone(),
        display_name: Some(descriptor.display_name.clone()),
        provider: provider.into(),
        model: descriptor.model_id.clone(),
        capabilities: vec![anycode_llm::ModelCapability::Chat],
        api_key: Some(provider.into()),
        api_key_ref: None,
        plan: None,
        base_url: Some(format!("http://127.0.0.1:{port}/v1/chat/completions")),
        temperature: None,
        max_tokens: Some(descriptor.context_tokens),
        extra_headers: None,
        endpoint_overrides: None,
        enabled: true,
        tags: Some(vec!["local".into(), "managed-runtime".into()]),
        source: Some("managed_local_runtime".into()),
    };
    let items = config.models.items.get_or_insert_with(Vec::new);
    anycode_llm::upsert_registry_item(items, item);
    config
        .models
        .active
        .get_or_insert_with(HashMap::new)
        .insert("chat".into(), descriptor.id.clone());
    config.provider = provider.into();
    config.model = descriptor.model_id.clone();
    config.api_key = provider.to_string();
    config.base_url = Some(format!("http://127.0.0.1:{port}/v1/chat/completions"));
    anycode_config::save_anycode_config(&config)
}

fn resolve_sglang_host_python() -> Result<String> {
    if let Ok(path) = std::env::var("ANYCODE_SGLANG_PYTHON") {
        let path = path.trim().to_string();
        if !path.is_empty() {
            return Ok(path);
        }
    }
    for candidate in ["python3.12", "python3.11"] {
        if sglang_host_python_minor(candidate).is_ok_and(|minor| minor == 11 || minor == 12) {
            return Ok(candidate.to_string());
        }
    }
    Err(anyhow!(
        "SGLang requires Python 3.11 or 3.12 (system python3 is 3.14). Install: brew install python@3.11"
    ))
}

fn sglang_host_python_minor(cmd: &str) -> Result<u32> {
    let output = std::process::Command::new(cmd)
        .args([
            "-c",
            "import sys; print(sys.version_info.major, sys.version_info.minor)",
        ])
        .output()
        .with_context(|| format!("run {cmd} --version"))?;
    if !output.status.success() {
        return Err(anyhow!("{cmd} is not usable"));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let mut parts = text.split_whitespace();
    let major: u32 = parts
        .next()
        .and_then(|v| v.parse().ok())
        .ok_or_else(|| anyhow!("invalid python version from {cmd}"))?;
    let minor: u32 = parts
        .next()
        .and_then(|v| v.parse().ok())
        .ok_or_else(|| anyhow!("invalid python version from {cmd}"))?;
    if major != 3 {
        return Err(anyhow!("{cmd} is not Python 3"));
    }
    Ok(minor)
}

fn sglang_venv_python_minor(python: &Path) -> Option<u32> {
    let output = std::process::Command::new(python)
        .args(["-c", "import sys; print(sys.version_info.minor)"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8_lossy(&output.stdout).trim().parse().ok()
}

fn sglang_vendor_dir() -> PathBuf {
    std::env::var("ANYCODE_SGLANG_SOURCE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            anycode_llm::anycode_models_dir()
                .parent()
                .map(|p| p.join("vendor/sglang"))
                .unwrap_or_else(|| PathBuf::from("~/.anycode/vendor/sglang"))
        })
}

fn ensure_sglang_source_repo(repo: &Path) -> Result<()> {
    if repo.join("python/sglang").is_dir() {
        return Ok(());
    }
    if let Some(parent) = repo.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let status = std::process::Command::new("git")
        .args([
            "clone",
            "--depth",
            "1",
            "https://github.com/sgl-project/sglang.git",
            &repo.display().to_string(),
        ])
        .status()
        .context("git clone sglang")?;
    if !status.success() {
        return Err(anyhow!(
            "git clone sglang failed — install Xcode CLT and ensure network access to github.com"
        ));
    }
    Ok(())
}

fn prepare_sglang_mps_pyproject(repo: &Path) -> Result<()> {
    let python_dir = repo.join("python");
    let pyproject = python_dir.join("pyproject.toml");
    let pyproject_other = python_dir.join("pyproject_other.toml");
    if pyproject_other.is_file() {
        if pyproject.is_file() {
            std::fs::remove_file(&pyproject)?;
        }
        std::fs::rename(&pyproject_other, &pyproject)?;
    }
    if !pyproject.is_file() {
        return Err(anyhow!(
            "sglang source missing python/pyproject.toml after MPS prepare"
        ));
    }
    Ok(())
}

fn pip_index_url() -> Option<String> {
    if let Ok(index) = std::env::var("ANYCODE_PIP_INDEX_URL") {
        let index = index.trim().to_string();
        if !index.is_empty() {
            return Some(index);
        }
    }
    if cfg!(target_arch = "aarch64") {
        return Some("https://pypi.tuna.tsinghua.edu.cn/simple".into());
    }
    None
}

fn pip_install(pip: &Path, args: &[&str]) -> Result<()> {
    let mut cmd_args: Vec<String> = args.iter().map(|s| (*s).to_string()).collect();
    if let Some(index) = pip_index_url() {
        let pos = cmd_args
            .iter()
            .position(|a| a == "install")
            .ok_or_else(|| anyhow!("pip_install requires an `install` subcommand"))?;
        cmd_args.insert(pos + 1, index);
        cmd_args.insert(pos + 1, "-i".into());
    }
    let status = std::process::Command::new(pip)
        .env("PIP_DEFAULT_TIMEOUT", "120")
        .args(&cmd_args)
        .status()
        .with_context(|| format!("pip {}", cmd_args.join(" ")))?;
    if !status.success() {
        return Err(anyhow!("pip install failed: {}", cmd_args.join(" ")));
    }
    Ok(())
}

fn python_imports_sglang(python: &Path) -> bool {
    std::process::Command::new(python)
        .args([
            "-c",
            "import sglang; import sglang.srt.entrypoints.http_server",
        ])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn install_sglang_venv_cuda(pip: &Path) -> Result<()> {
    pip_install(
        pip,
        &[
            "install",
            "-q",
            "-U",
            "pip",
            "wheel",
            "huggingface_hub[cli]",
        ],
    )?;
    pip_install(pip, &["install", "-q", "sglang[srt]"])?;
    Ok(())
}

/// Mac MLX: PyPI `sglang` pulls CUDA-only `triton` — use source `all_mps` only.
fn install_sglang_venv_apple_mlx(pip: &Path, python: &Path) -> Result<()> {
    pip_install(pip, &["install", "-q", "-U", "pip", "wheel"])?;
    pip_install(
        pip,
        &[
            "install",
            "-q",
            "huggingface_hub[cli]",
            "mlx",
            "mlx-lm",
            "torch",
        ],
    )?;
    let repo = sglang_vendor_dir();
    ensure_sglang_source_repo(&repo)?;
    prepare_sglang_mps_pyproject(&repo)?;
    let mut editable_args = vec![
        "install".to_string(),
        "-q".to_string(),
        "-e".to_string(),
        "python[all_mps]".to_string(),
    ];
    if let Some(index) = pip_index_url() {
        editable_args.insert(1, index);
        editable_args.insert(1, "-i".into());
    }
    let status = std::process::Command::new(pip)
        .env("PIP_DEFAULT_TIMEOUT", "120")
        .args(&editable_args)
        .current_dir(&repo)
        .status()
        .with_context(|| format!("pip install -e python[all_mps] in {}", repo.display()))?;
    if !status.success() {
        return Err(anyhow!(
            "Mac SGLang 需从源码安装 python[all_mps]（需访问 GitHub + PyPI 镜像）。\
             失败目录: {}。Ollama 无原生 tool_calls，不适合工具/PPT 任务。",
            repo.display()
        ));
    }
    if !python_imports_sglang(python) {
        return Err(anyhow!(
            "SGLang 安装后仍无法启动 launch_server；请检查 ~/.anycode/vendor/sglang 与 venv 依赖"
        ));
    }
    Ok(())
}

fn install_sglang_venv(venv: &Path) -> Result<()> {
    let host_python = resolve_sglang_host_python()?;
    let venv_python = venv.join("bin/python");
    if venv_python.is_file() {
        let minor = sglang_venv_python_minor(&venv_python);
        if !minor.is_some_and(|m| m == 11 || m == 12) {
            std::fs::remove_dir_all(venv).with_context(|| {
                format!("remove incompatible sglang venv at {}", venv.display())
            })?;
        }
    }
    if !venv_python.is_file() {
        std::fs::create_dir_all(venv.parent().unwrap_or_else(|| Path::new(".")))?;
        let status = std::process::Command::new(&host_python)
            .args(["-m", "venv", &venv.display().to_string()])
            .status()
            .with_context(|| format!("create sglang venv with {host_python}"))?;
        if !status.success() {
            return Err(anyhow!("{host_python} -m venv failed"));
        }
    }
    let pip = venv.join("bin/pip");
    let python = venv.join("bin/python");
    if cfg!(target_arch = "aarch64") {
        install_sglang_venv_apple_mlx(&pip, &python)?;
    } else {
        install_sglang_venv_cuda(&pip)?;
    }
    // minicpm5 parser may require main; best-effort upgrade without re-resolving CUDA deps.
    if cfg!(target_arch = "aarch64") {
        let _ = std::process::Command::new(&pip)
            .args([
                "install",
                "-q",
                "--no-deps",
                "git+https://github.com/sgl-project/sglang.git@main#subdirectory=python",
            ])
            .status();
    } else {
        let _ = std::process::Command::new(&pip)
            .args([
                "install",
                "-q",
                "git+https://github.com/sgl-project/sglang.git@main#subdirectory=python",
            ])
            .status();
    }
    let status = std::process::Command::new(&python)
        .args(["-c", "import sglang"])
        .status()
        .context("verify sglang import")?;
    if !status.success() {
        return Err(anyhow!("sglang import failed after install"));
    }
    if cfg!(target_arch = "aarch64") && !python_imports_sglang(&python) {
        return Err(anyhow!(
            "SGLang launch_server 不可用（Mac 需 python[all_mps] 源码安装，不能用 PyPI 的 sglang 轮子）"
        ));
    }
    Ok(())
}

fn install_hf_weights(python: &Path, repo: &str, dest: &Path) -> Result<()> {
    std::fs::create_dir_all(dest)?;
    let bin_dir = python.parent().unwrap_or_else(|| Path::new("."));
    let dest_string = dest.display().to_string();
    let script = format!(
        "from huggingface_hub import snapshot_download\n\
         snapshot_download(repo_id={repo:?}, local_dir={dest:?})\n",
        repo = repo,
        dest = dest_string,
    );
    let status = std::process::Command::new(python)
        .args(["-c", &script])
        .status()
        .context("huggingface_hub snapshot_download")?;
    if status.success() {
        return Ok(());
    }
    for cli_name in ["hf", "huggingface-cli"] {
        let cli = bin_dir.join(cli_name);
        if !cli.is_file() {
            continue;
        }
        let status = std::process::Command::new(&cli)
            .args(["download", repo, "--local-dir", &dest_string])
            .status()
            .with_context(|| format!("{cli_name} download"))?;
        if status.success() {
            return Ok(());
        }
    }
    Err(anyhow!("huggingface download failed for {repo}"))
}

fn pick_loopback_port() -> Result<u16> {
    (47100u16..=47199)
        .find(|port| std::net::TcpListener::bind(("127.0.0.1", *port)).is_ok())
        .ok_or_else(|| anyhow!("no free loopback port"))
}

async fn wait_for_health(port: u16, seconds: u64) -> bool {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .unwrap_or_default();
    let url = format!("http://127.0.0.1:{port}/health");
    for _ in 0..seconds {
        if client
            .get(&url)
            .send()
            .await
            .is_ok_and(|response| response.status().is_success())
        {
            return true;
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
    false
}

fn disk_free_bytes(root: &Path) -> Option<u64> {
    let _ = std::fs::create_dir_all(root);
    #[cfg(unix)]
    {
        use std::ffi::CString;
        use std::mem::MaybeUninit;
        let path = CString::new(root.to_string_lossy().as_bytes()).ok()?;
        let mut stat = MaybeUninit::<libc::statfs>::uninit();
        if unsafe { libc::statfs(path.as_ptr(), stat.as_mut_ptr()) } != 0 {
            return None;
        }
        let stat = unsafe { stat.assume_init() };
        Some(stat.f_bavail * stat.f_bsize as u64)
    }
    #[cfg(not(unix))]
    {
        None
    }
}

fn system_ram_bytes() -> Option<u64> {
    #[cfg(target_os = "macos")]
    {
        use std::mem::MaybeUninit;
        let mut count = std::mem::size_of::<u64>();
        let mut memory = MaybeUninit::<u64>::uninit();
        let name = std::ffi::CString::new("hw.memsize").ok()?;
        if unsafe {
            libc::sysctlbyname(
                name.as_ptr(),
                memory.as_mut_ptr().cast(),
                &mut count,
                std::ptr::null_mut(),
                0,
            )
        } == 0
        {
            return Some(unsafe { memory.assume_init() });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn descriptor(id: &str, bytes: &[u8]) -> ManagedModelDescriptor {
        ManagedModelDescriptor {
            id: id.into(),
            version: "1".into(),
            display_name: id.into(),
            model_id: id.into(),
            file_name: format!("{id}.gguf"),
            download_url: "https://example.invalid/model".into(),
            sha256: format!("{:x}", Sha256::digest(bytes)),
            size_bytes: bytes.len() as u64,
            architectures: vec![std::env::consts::ARCH.into()],
            context_tokens: 2048,
            minimum_ram_bytes: 1,
            license: "test".into(),
            capabilities: RuntimeCapabilities {
                chat: true,
                tools: false,
                vision: false,
            },
            runtime: "llama-server".into(),
            runtime_args: vec!["{model}".into(), "{port}".into()],
            preview: true,
        }
    }

    #[tokio::test]
    async fn supports_multiple_descriptors_and_versioned_paths() {
        let dir = tempdir().unwrap();
        let manager = ManagedRuntimeManager::with_descriptors(
            dir.path().into(),
            vec![descriptor("alpha", b"a"), descriptor("beta", b"b")],
        )
        .unwrap();
        let statuses = manager.list_status().await;
        assert_eq!(statuses.len(), 2);
        assert!(model_file_path_for(dir.path(), &statuses[0].descriptor)
            .to_string_lossy()
            .contains("/1/"));
    }

    #[tokio::test]
    async fn download_checksum_accepts_match_and_rejects_mismatch() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("model");
        tokio::fs::write(&path, b"verified").await.unwrap();
        let expected = format!("{:x}", Sha256::digest(b"verified"));
        verify_sha256(&path, &expected).await.unwrap();
        assert!(verify_sha256(&path, &"0".repeat(64)).await.is_err());
    }

    #[test]
    fn range_response_requires_matching_content_range() {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::CONTENT_RANGE,
            "bytes 42-99/100".parse().unwrap(),
        );
        assert!(valid_range_response(
            reqwest::StatusCode::PARTIAL_CONTENT,
            &headers,
            42
        ));
        assert!(!valid_range_response(reqwest::StatusCode::OK, &headers, 42));
    }
}
