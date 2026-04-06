use crate::workspace;
use anycode_core::RuntimeMode;
use std::path::PathBuf;

pub(crate) async fn handle_list(json: bool) -> anyhow::Result<()> {
    let projects = workspace::list_projects(50);
    if json {
        println!("{}", serde_json::to_string_pretty(&projects)?);
    } else if projects.is_empty() {
        println!("workspace projects: (none)");
    } else {
        for project in projects {
            println!("{}", project.path);
            if let Some(label) = project.label {
                println!("label: {}", label);
            }
            if let Some(mode) = project.default_mode {
                println!("mode: {}", mode);
            }
            if let Some(channel) = project.channel_profile {
                println!("channel: {}", channel);
            }
            println!();
        }
    }
    Ok(())
}

pub(crate) async fn handle_status() -> anyhow::Result<()> {
    println!("{}", workspace::current_workspace_status(10));
    Ok(())
}

pub(crate) async fn handle_touch(path: Option<PathBuf>) -> anyhow::Result<()> {
    let path =
        path.unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    workspace::touch_project_dir(path.clone());
    println!(
        "{}",
        std::fs::canonicalize(path)
            .unwrap_or_else(|_| PathBuf::from("."))
            .display()
    );
    Ok(())
}

pub(crate) async fn handle_set_mode(mode: String, path: Option<PathBuf>) -> anyhow::Result<()> {
    let parsed = RuntimeMode::parse(&mode)
        .ok_or_else(|| anyhow::anyhow!("unknown workspace mode: {}", mode))?;
    let path =
        path.unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    workspace::update_project_metadata(path, None, Some(parsed.as_str().to_string()), None)?;
    println!("workspace default mode: {}", parsed.as_str());
    Ok(())
}

pub(crate) async fn handle_set_channel(
    channel: String,
    path: Option<PathBuf>,
) -> anyhow::Result<()> {
    let path =
        path.unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    workspace::update_project_metadata(path, None, None, Some(channel.clone()))?;
    println!("workspace channel profile: {}", channel);
    Ok(())
}

pub(crate) async fn handle_label(label: String, path: Option<PathBuf>) -> anyhow::Result<()> {
    let path =
        path.unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    workspace::update_project_metadata(path, Some(label.clone()), None, None)?;
    println!("workspace label: {}", label);
    Ok(())
}
