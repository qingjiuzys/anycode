//! `anycode setup` 可选步骤：配置 pipeline 本地 ONNX 嵌入，并写入 HF 下载端点（`embedding_hf_endpoint`）。

use crate::app_config::{
    load_anycode_config_resolved, save_anycode_config_resolved, AnyCodeConfig,
};
use std::path::PathBuf;

/// 在模型向导之后调用：TTY 上询问是否启用本地嵌入并更新 `~/.anycode/config.json`。
pub(crate) fn run_optional(config_file: Option<PathBuf>) -> anyhow::Result<()> {
    let term = console::Term::stdout();
    if !term.is_term() {
        return Ok(());
    }

    let Some(mut cfg) = load_anycode_config_resolved(config_file.clone())? else {
        return Ok(());
    };

    use dialoguer::{theme::ColorfulTheme, Select};

    const OPT_SKIP: &str = "跳过（不改变记忆/嵌入配置）";
    const OPT_CONFIGURE: &str = "启用 pipeline 记忆 + 本地嵌入，并选择下载源与模型";

    let intro = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("本地记忆向量（首次使用会下载 ONNX；可选镜像加速）")
        .items([OPT_SKIP, OPT_CONFIGURE])
        .default(0)
        .interact()?;

    if intro == 0 {
        return Ok(());
    }

    const EP_OFFICIAL: (&str, Option<&str>) = (
        "Hugging Face 官方 (huggingface.co)",
        Some("https://huggingface.co"),
    );
    const EP_MIRROR: (&str, Option<&str>) =
        ("国内镜像 (hf-mirror.com)", Some("https://hf-mirror.com"));
    const EP_ENV_ONLY: (&str, Option<&str>) =
        ("不写入端点（沿用环境变量 HF_ENDPOINT，若已设置）", None);

    let ep_idx = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("模型下载端点（国内访问 HF 不稳定时可选用镜像）")
        .items([EP_OFFICIAL.0, EP_MIRROR.0, EP_ENV_ONLY.0])
        .default(1)
        .interact()?;

    let endpoint_saved = match ep_idx {
        0 => EP_OFFICIAL.1.map(str::to_string),
        1 => EP_MIRROR.1.map(str::to_string),
        _ => EP_ENV_ONLY.1.map(String::from),
    };

    const MODELS: [(&str, &str); 4] = [
        ("AllMiniLML6-v2（英文小模型，常用）", "AllMiniLML6V2"),
        ("bge-small-zh（中文）", "BGESmallZHV15"),
        ("multilingual-e5-base（多语）", "MultilingualE5Base"),
        ("bge-small-en-v1.5（英文）", "BGESmallENV15"),
    ];
    let model_labels: Vec<_> = MODELS.iter().map(|(l, _)| *l).collect();
    let m_idx = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("本地嵌入模型（与 fastembed 内置枚举一致）")
        .items(&model_labels)
        .default(0)
        .interact()?;

    let model_id = MODELS[m_idx].1.to_string();

    apply_local_embedding(&mut cfg, model_id, endpoint_saved);
    save_anycode_config_resolved(config_file, &cfg)?;

    println!(
        "已写入本地嵌入配置（memory.backend=pipeline，embedding_provider=local）。\n\
若尚未使用本地 ONNX，请用 `cargo build -p anycode --features embedding-local` 构建后运行。"
    );
    Ok(())
}

fn apply_local_embedding(cfg: &mut AnyCodeConfig, model_id: String, endpoint: Option<String>) {
    cfg.memory.backend = "pipeline".to_string();
    cfg.memory.pipeline.embedding_enabled = Some(true);
    cfg.memory.pipeline.embedding_provider = Some("local".into());
    cfg.memory.pipeline.embedding_local_model = Some(model_id);
    cfg.memory.pipeline.embedding_hf_endpoint = endpoint;
}
