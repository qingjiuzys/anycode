//! Built-in project templates (e.g. `flutter-app`).

mod apply;
mod embedded;
mod manifest;

pub use apply::{apply_project_template, ApplyTemplateOptions, ApplyTemplateResult};
pub use manifest::{
    list_project_templates, resolve_project_templates_root, ProjectTemplateManifest,
};
