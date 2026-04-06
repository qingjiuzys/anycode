use crate::app_config::{
    disable_feature_flag, enable_feature_flag, load_config_for_session, set_default_runtime_mode,
};

pub(crate) async fn handle_enable(
    config: Option<std::path::PathBuf>,
    feature: String,
) -> anyhow::Result<()> {
    let enabled = enable_feature_flag(config, &feature)?;
    println!("enabled features: {}", enabled.join(", "));
    Ok(())
}

pub(crate) async fn handle_disable(
    config: Option<std::path::PathBuf>,
    feature: String,
) -> anyhow::Result<()> {
    let enabled = disable_feature_flag(config, &feature)?;
    println!("enabled features: {}", enabled.join(", "));
    Ok(())
}

pub(crate) async fn handle_mode(
    config: Option<std::path::PathBuf>,
    ignore_approval: bool,
    mode: Option<String>,
) -> anyhow::Result<()> {
    if let Some(mode) = mode {
        let parsed = set_default_runtime_mode(config, &mode)?;
        println!("default mode: {}", parsed.as_str());
    } else {
        let config = load_config_for_session(config, ignore_approval).await?;
        println!("default mode: {}", config.runtime.default_mode.as_str());
    }
    Ok(())
}
