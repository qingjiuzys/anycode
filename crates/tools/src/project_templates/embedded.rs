//! Compile-time bundled `project-templates/` (used when repo tree is not on disk).

use anyhow::{Context, Result};
use rust_embed::Embed;
use std::path::Path;

#[derive(Embed)]
#[folder = "../../project-templates/"]
struct BundledProjectTemplates;

pub fn materialize_to(dest: &Path) -> Result<()> {
    for rel in BundledProjectTemplates::iter() {
        let rel = rel.as_ref();
        let asset = BundledProjectTemplates::get(rel)
            .with_context(|| format!("missing embedded template file: {rel}"))?;
        let out = dest.join(rel);
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create {}", parent.display()))?;
        }
        std::fs::write(&out, asset.data.as_ref())
            .with_context(|| format!("write {}", out.display()))?;
    }
    Ok(())
}

pub fn has_bundled_templates() -> bool {
    BundledProjectTemplates::get("flutter-app/manifest.json").is_some()
}
