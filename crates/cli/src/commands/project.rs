//! `anycode project templates|init`

use anycode_tools::{apply_project_template, list_project_templates, ApplyTemplateOptions};
use std::path::PathBuf;

pub(crate) fn list_templates(json: bool) -> anyhow::Result<()> {
    let list = list_project_templates()?;
    if json {
        println!("{}", serde_json::to_string_pretty(&list)?);
    } else {
        for t in &list {
            let zh = t.name_zh.as_deref().unwrap_or(&t.name);
            println!("{}\t{zh}\t{}", t.id, t.description);
        }
    }
    Ok(())
}

pub(crate) async fn init(
    template: String,
    path: PathBuf,
    name: Option<String>,
    title: Option<String>,
    org: Option<String>,
    force: bool,
    flutter_create: bool,
) -> anyhow::Result<()> {
    let result = apply_project_template(
        &template,
        &path,
        ApplyTemplateOptions {
            project_name: name,
            app_title: title,
            bundle_org: org,
            force,
            run_flutter_create: flutter_create
                || std::env::var_os("ANYCODE_TEMPLATE_RUN_FLUTTER_CREATE").is_some(),
        },
    )?;
    crate::workspace::touch_project_dir(result.root.clone());
    println!(
        "Created {} at {} (package: {})",
        result.template_id,
        result.root.display(),
        result.project_name
    );
    if template == "flutter-app" {
        println!(
            "Agent-first scaffold (no Flutter at create). Agent prepares SDK via skill flutter-bootstrap."
        );
    }
    println!(
        "Next: cd {} && anycode run --agent goal-runner --goal \"Build MVP\" --done-when GOAL_ACCEPTANCE_OK \"…\"",
        result.root.display()
    );
    Ok(())
}
