//! `anycode setup` — memory / embedding strategy step (runs after model, before channels).

use crate::app_config::{
    load_anycode_config_resolved, save_anycode_config_resolved, AnyCodeConfig, MemoryConfigFile,
    MemoryPipelineConfigFile,
};
use crate::i18n::{tr, tr_args};
use fluent_bundle::FluentArgs;
use std::path::PathBuf;

/// Preset applied by the setup wizard and unit tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MemorySetupPreset {
    /// Disable persistent memory / recall (`memory.backend = noop`).
    Noop,
    SimpleFile,
    HybridBackend,
    PipelineNoEmbedding,
    PipelineHttp {
        embedding_base_url: String,
        embedding_model: String,
    },
    #[cfg(feature = "embedding-local")]
    PipelineLocalOnnx {
        model_id: String,
        hf_endpoint: Option<String>,
    },
}

fn clear_pipeline_embedding_local_fields(
    pipeline: &mut crate::app_config::MemoryPipelineConfigFile,
) {
    pipeline.embedding_local_model = None;
    pipeline.embedding_hf_endpoint = None;
    pipeline.embedding_local_cache_dir = None;
}

fn apply_noop_memory(cfg: &mut AnyCodeConfig) {
    cfg.memory.backend = "noop".to_string();
}

fn apply_hybrid_memory(cfg: &mut AnyCodeConfig) {
    let path = cfg.memory.path.clone();
    let auto_save = cfg.memory.auto_save;
    cfg.memory.backend = "hybrid".to_string();
    cfg.memory.pipeline = MemoryPipelineConfigFile::default();
    cfg.memory.path = path;
    cfg.memory.auto_save = auto_save;
}

fn apply_pipeline_no_embedding_memory(cfg: &mut AnyCodeConfig) {
    cfg.memory.backend = "pipeline".to_string();
    cfg.memory.pipeline.embedding_enabled = Some(false);
    cfg.memory.pipeline.embedding_model = None;
    cfg.memory.pipeline.embedding_base_url = None;
    cfg.memory.pipeline.embedding_provider = None;
    clear_pipeline_embedding_local_fields(&mut cfg.memory.pipeline);
}

fn apply_simple_file_memory(cfg: &mut AnyCodeConfig) {
    let path = cfg.memory.path.clone();
    let auto_save = cfg.memory.auto_save;
    cfg.memory = MemoryConfigFile::default();
    cfg.memory.path = path;
    cfg.memory.auto_save = auto_save;
}

fn apply_pipeline_http_memory(
    cfg: &mut AnyCodeConfig,
    embedding_base_url: String,
    embedding_model: String,
) {
    cfg.memory.backend = "pipeline".to_string();
    cfg.memory.pipeline.embedding_enabled = Some(true);
    cfg.memory.pipeline.embedding_provider = Some("http".into());
    cfg.memory.pipeline.embedding_base_url = Some(embedding_base_url);
    cfg.memory.pipeline.embedding_model = Some(embedding_model);
    clear_pipeline_embedding_local_fields(&mut cfg.memory.pipeline);
}

#[cfg(feature = "embedding-local")]
fn apply_pipeline_local_memory(
    cfg: &mut AnyCodeConfig,
    model_id: String,
    hf_endpoint: Option<String>,
) {
    cfg.memory.backend = "pipeline".to_string();
    cfg.memory.pipeline.embedding_enabled = Some(true);
    cfg.memory.pipeline.embedding_provider = Some("local".into());
    cfg.memory.pipeline.embedding_local_model = Some(model_id);
    cfg.memory.pipeline.embedding_hf_endpoint = hf_endpoint;
    cfg.memory.pipeline.embedding_base_url = None;
    cfg.memory.pipeline.embedding_model = None;
}

/// Apply a memory preset to `cfg` (wizard branches and tests).
pub(crate) fn apply_memory_preset(cfg: &mut AnyCodeConfig, preset: MemorySetupPreset) {
    match preset {
        MemorySetupPreset::Noop => apply_noop_memory(cfg),
        MemorySetupPreset::SimpleFile => apply_simple_file_memory(cfg),
        MemorySetupPreset::HybridBackend => apply_hybrid_memory(cfg),
        MemorySetupPreset::PipelineNoEmbedding => apply_pipeline_no_embedding_memory(cfg),
        MemorySetupPreset::PipelineHttp {
            embedding_base_url,
            embedding_model,
        } => apply_pipeline_http_memory(cfg, embedding_base_url, embedding_model),
        #[cfg(feature = "embedding-local")]
        MemorySetupPreset::PipelineLocalOnnx {
            model_id,
            hf_endpoint,
        } => apply_pipeline_local_memory(cfg, model_id, hf_endpoint),
    }
}

#[cfg(feature = "embedding-local")]
fn run_local_onnx_subwizard(
    cfg: &mut AnyCodeConfig,
    theme: &dialoguer::theme::ColorfulTheme,
) -> anyhow::Result<()> {
    use dialoguer::Select;

    let ep_labels = vec![
        tr("setup-memory-ep-official"),
        tr("setup-memory-ep-mirror"),
        tr("setup-memory-ep-env"),
    ];
    let ep_idx = Select::with_theme(theme)
        .with_prompt(tr("setup-memory-local-prompt-ep"))
        .items(&ep_labels)
        .default(1)
        .interact()?;

    let endpoint_saved = match ep_idx {
        0 => Some("https://huggingface.co".to_string()),
        1 => Some("https://hf-mirror.com".to_string()),
        _ => None,
    };

    const MODELS: [(&str, &str); 4] = [
        ("setup-memory-local-model-allmini", "AllMiniLML6V2"),
        ("setup-memory-local-model-bge-zh", "BGESmallZHV15"),
        ("setup-memory-local-model-e5-base", "MultilingualE5Base"),
        ("setup-memory-local-model-bge-en", "BGESmallENV15"),
    ];
    let model_labels: Vec<String> = MODELS.iter().map(|(fid, _)| tr(fid)).collect();
    let m_idx = Select::with_theme(theme)
        .with_prompt(tr("setup-memory-local-prompt-model"))
        .items(&model_labels)
        .default(0)
        .interact()?;

    let model_id = MODELS[m_idx].1.to_string();
    apply_memory_preset(
        cfg,
        MemorySetupPreset::PipelineLocalOnnx {
            model_id,
            hf_endpoint: endpoint_saved,
        },
    );
    println!("{}", tr("setup-memory-local-done-print"));
    Ok(())
}

fn interactive_http_embedding_prompts(
    cfg: &mut AnyCodeConfig,
    theme: &dialoguer::theme::ColorfulTheme,
) -> anyhow::Result<()> {
    use dialoguer::Input;

    println!("{}", tr("setup-memory-http-apikey-hint"));
    let default_url = cfg
        .base_url
        .clone()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "https://api.openai.com/v1".to_string());
    let url: String = Input::with_theme(theme)
        .with_prompt(tr("setup-memory-http-prompt-base-url"))
        .default(default_url)
        .interact_text()?;
    let url = url.trim().to_string();
    if url.is_empty() {
        anyhow::bail!("{}", tr("setup-memory-http-empty-url"));
    }
    let model: String = Input::with_theme(theme)
        .with_prompt(tr("setup-memory-http-prompt-model"))
        .default("text-embedding-3-small".to_string())
        .interact_text()?;
    let model = model.trim().to_string();
    if model.is_empty() {
        anyhow::bail!("{}", tr("setup-memory-http-empty-model"));
    }
    if let Err(reason) = probe_embedding_url_quick_check(&url) {
        let mut a = FluentArgs::new();
        a.set("reason", reason.clone());
        println!("{}", tr_args("setup-memory-http-probe-warn", &a));
        use dialoguer::Confirm;
        if !Confirm::with_theme(theme)
            .with_prompt(tr("setup-memory-http-probe-continue"))
            .default(true)
            .interact()?
        {
            anyhow::bail!("{}", tr("setup-memory-http-probe-aborted"));
        }
    }
    apply_memory_preset(
        cfg,
        MemorySetupPreset::PipelineHttp {
            embedding_base_url: url,
            embedding_model: model,
        },
    );
    Ok(())
}

/// Best-effort TCP/TLS reachability (HEAD, then GET) for embedding `base_url`. Failure is non-fatal for setup (user may continue).
fn probe_embedding_url_quick_check(url_raw: &str) -> Result<(), String> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;
    rt.block_on(async move {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(4))
            .redirect(reqwest::redirect::Policy::limited(8))
            .build()
            .map_err(|e| e.to_string())?;
        if client.head(url_raw).send().await.is_ok() {
            return Ok(());
        }
        client
            .get(url_raw)
            .send()
            .await
            .map(|_| ())
            .map_err(|e| e.to_string())
    })
}

/// Wizard shows Skip / noop plus three memory presets: Hybrid as “Markdown”, then remote vectors;
/// with `embedding-local`, also local ONNX. Other [`MemorySetupPreset`] variants stay for tests and JSON-only flows.
fn interactive_memory_step(cfg: &mut AnyCodeConfig) -> anyhow::Result<bool> {
    use dialoguer::theme::ColorfulTheme;
    use dialoguer::Select;

    let theme = ColorfulTheme::default();
    let prompt_items = vec![
        tr("setup-memory-opt-skip"),
        tr("setup-memory-opt-noop"),
        tr("setup-memory-opt-markdown"),
        tr("setup-memory-opt-remote-vector"),
    ];
    #[cfg(feature = "embedding-local")]
    let mut prompt_items = prompt_items;
    #[cfg(feature = "embedding-local")]
    prompt_items.push(tr("setup-memory-opt-local-vector"));

    let idx = Select::with_theme(&theme)
        .with_prompt(tr("setup-memory-prompt"))
        .items(&prompt_items)
        .default(2)
        .interact()?;

    #[cfg(not(feature = "embedding-local"))]
    match idx {
        0 => Ok(false),
        1 => {
            apply_memory_preset(cfg, MemorySetupPreset::Noop);
            Ok(true)
        }
        2 => {
            apply_memory_preset(cfg, MemorySetupPreset::HybridBackend);
            Ok(true)
        }
        3 => {
            interactive_http_embedding_prompts(cfg, &theme)?;
            Ok(true)
        }
        _ => unreachable!(),
    }

    #[cfg(feature = "embedding-local")]
    match idx {
        0 => Ok(false),
        1 => {
            apply_memory_preset(cfg, MemorySetupPreset::Noop);
            Ok(true)
        }
        2 => {
            apply_memory_preset(cfg, MemorySetupPreset::HybridBackend);
            Ok(true)
        }
        3 => {
            interactive_http_embedding_prompts(cfg, &theme)?;
            Ok(true)
        }
        4 => {
            run_local_onnx_subwizard(cfg, &theme)?;
            Ok(true)
        }
        _ => unreachable!(),
    }
}

pub(crate) fn run_after_model_step(config_file: Option<PathBuf>) -> anyhow::Result<()> {
    let Some(mut cfg) = load_anycode_config_resolved(config_file.clone())? else {
        return Ok(());
    };
    let term = console::Term::stdout();
    if !term.is_term() {
        println!("{}", tr("setup-memory-non-tty-hint"));
        return Ok(());
    }
    if !interactive_memory_step(&mut cfg)? {
        return Ok(());
    }
    save_anycode_config_resolved(config_file, &cfg)?;
    Ok(())
}

pub(crate) fn apply_to_in_memory_wizard_config(cfg: &mut AnyCodeConfig) -> anyhow::Result<()> {
    let term = console::Term::stdout();
    if !term.is_term() {
        println!("{}", tr("setup-memory-non-tty-hint"));
        return Ok(());
    }
    let _modified = interactive_memory_step(cfg)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_config::AnyCodeConfig;

    fn dummy_config() -> AnyCodeConfig {
        serde_json::from_value(serde_json::json!({
            "provider": "z.ai",
            "plan": "coding",
            "api_key": "k",
            "model": "glm-5",
            "temperature": 0.7,
            "max_tokens": 8192u32,
            "base_url": "https://api.example.com/v1",
        }))
        .expect("minimal AnyCodeConfig JSON for tests")
    }

    #[test]
    fn preset_noop_sets_backend() {
        let mut cfg = dummy_config();
        cfg.memory.backend = "file".into();
        apply_memory_preset(&mut cfg, MemorySetupPreset::Noop);
        assert_eq!(cfg.memory.backend, "noop");
    }

    #[test]
    fn preset_simple_file_resets_pipeline_defaults() {
        let mut cfg = dummy_config();
        cfg.memory.backend = "pipeline".to_string();
        cfg.memory.pipeline.embedding_provider = Some("http".into());
        cfg.memory.pipeline.embedding_enabled = Some(true);
        cfg.memory.pipeline.embedding_model = Some("m".into());
        cfg.memory.pipeline.embedding_base_url = Some("https://x/v1".into());
        cfg.memory.pipeline.embedding_local_model = Some("AllMiniLML6V2".into());

        apply_memory_preset(&mut cfg, MemorySetupPreset::SimpleFile);

        assert_eq!(cfg.memory.backend, "file");
        assert!(cfg.memory.pipeline.embedding_provider.is_none());
        assert!(cfg.memory.pipeline.embedding_model.is_none());
        assert!(cfg.memory.pipeline.embedding_base_url.is_none());
        assert!(cfg.memory.pipeline.embedding_local_model.is_none());
    }

    #[test]
    fn preset_hybrid_sets_backend_and_resets_pipeline() {
        let mut cfg = dummy_config();
        cfg.memory.backend = "pipeline".to_string();
        cfg.memory.pipeline.embedding_enabled = Some(true);
        cfg.memory.pipeline.embedding_provider = Some("http".into());
        cfg.memory.pipeline.embedding_model = Some("m".into());
        cfg.memory.pipeline.embedding_base_url = Some("https://e".into());

        apply_memory_preset(&mut cfg, MemorySetupPreset::HybridBackend);

        assert_eq!(cfg.memory.backend, "hybrid");
        assert!(cfg.memory.pipeline.embedding_model.is_none());
        assert!(cfg.memory.pipeline.embedding_base_url.is_none());
        assert!(cfg.memory.pipeline.embedding_provider.is_none());
    }

    #[test]
    fn preset_pipeline_no_embedding_disables_vectors() {
        let mut cfg = dummy_config();
        cfg.memory.backend = "file".into();
        cfg.memory.pipeline.embedding_model = Some("m".into());

        apply_memory_preset(&mut cfg, MemorySetupPreset::PipelineNoEmbedding);

        assert_eq!(cfg.memory.backend, "pipeline");
        assert_eq!(cfg.memory.pipeline.embedding_enabled, Some(false));
        assert!(cfg.memory.pipeline.embedding_model.is_none());
    }

    #[test]
    fn preset_pipeline_http_sets_remote_embedding() {
        let mut cfg = dummy_config();
        cfg.memory.pipeline.embedding_local_model = Some("Old".into());
        cfg.memory.pipeline.embedding_hf_endpoint = Some("https://old".into());

        apply_memory_preset(
            &mut cfg,
            MemorySetupPreset::PipelineHttp {
                embedding_base_url: "https://openai-compat/v1".to_string(),
                embedding_model: "text-embedding-3-small".to_string(),
            },
        );

        assert_eq!(cfg.memory.backend, "pipeline");
        assert_eq!(
            cfg.memory.pipeline.embedding_provider.as_deref(),
            Some("http")
        );
        assert_eq!(cfg.memory.pipeline.embedding_enabled, Some(true));
        assert_eq!(
            cfg.memory.pipeline.embedding_base_url.as_deref(),
            Some("https://openai-compat/v1")
        );
        assert_eq!(
            cfg.memory.pipeline.embedding_model.as_deref(),
            Some("text-embedding-3-small")
        );
        assert!(cfg.memory.pipeline.embedding_local_model.is_none());
        assert!(cfg.memory.pipeline.embedding_hf_endpoint.is_none());
    }

    #[cfg(feature = "embedding-local")]
    #[test]
    fn preset_pipeline_local_sets_onnx_fields() {
        let mut cfg = dummy_config();
        cfg.memory.pipeline.embedding_base_url = Some("https://x".into());
        cfg.memory.pipeline.embedding_model = Some("embed".into());

        apply_memory_preset(
            &mut cfg,
            MemorySetupPreset::PipelineLocalOnnx {
                model_id: "BGESmallZHV15".to_string(),
                hf_endpoint: Some("https://hf-mirror.com".to_string()),
            },
        );

        assert_eq!(cfg.memory.backend, "pipeline");
        assert_eq!(
            cfg.memory.pipeline.embedding_provider.as_deref(),
            Some("local")
        );
        assert_eq!(
            cfg.memory.pipeline.embedding_local_model.as_deref(),
            Some("BGESmallZHV15")
        );
        assert_eq!(
            cfg.memory.pipeline.embedding_hf_endpoint.as_deref(),
            Some("https://hf-mirror.com")
        );
        assert!(cfg.memory.pipeline.embedding_base_url.is_none());
        assert!(cfg.memory.pipeline.embedding_model.is_none());
    }
}
