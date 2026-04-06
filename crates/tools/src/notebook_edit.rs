//! `NotebookEdit` — 简化版 `.ipynb` 单元格编辑（对齐字段名与主路径）。

use crate::paths::resolve_path_fields;
use anycode_core::prelude::*;
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{json, Value};
use std::time::Instant;

pub struct NotebookEditTool {
    security_policy: SecurityPolicy,
}

impl NotebookEditTool {
    pub fn new(sandbox_mode: bool) -> Self {
        Self {
            security_policy: SecurityPolicy {
                allow_commands: vec![],
                deny_commands: vec![],
                require_approval: true,
                sandbox_mode,
                timeout_ms: None,
            },
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
struct NbInput {
    notebook_path: String,
    #[serde(default)]
    cell_id: Option<String>,
    #[serde(default)]
    new_source: String,
    #[serde(default)]
    cell_type: Option<String>,
    #[serde(default)]
    edit_mode: Option<String>,
}

fn set_cell_source(cell: &mut Value, src: &str) {
    if let Some(obj) = cell.as_object_mut() {
        obj.insert("source".to_string(), json!(src));
    }
}

fn find_cell_index(cells: &[Value], cell_id: Option<&str>) -> Option<usize> {
    let id = cell_id?;
    if let Ok(idx) = id.parse::<usize>() {
        if idx < cells.len() {
            return Some(idx);
        }
        return None;
    }
    for (i, c) in cells.iter().enumerate() {
        if let Some(meta) = c.get("metadata") {
            if meta.get("id").and_then(|v| v.as_str()) == Some(id) {
                return Some(i);
            }
        }
    }
    None
}

#[async_trait]
impl Tool for NotebookEditTool {
    fn name(&self) -> &str {
        "NotebookEdit"
    }

    fn description(&self) -> &str {
        "Edit Jupyter notebook cells (.ipynb): replace, insert, or delete by cell id or index."
    }

    fn api_tool_description(&self) -> String {
        format!(
            "{}\n\n\
            Mutate `.ipynb` JSON safely.\n\
            - `cell_id` may be a numeric index or a Jupyter metadata id string.\n\
            - `edit_mode`: `replace` updates `new_source`; `insert` adds a cell; `delete` removes.\n\
            - `cell_type` applies on insert (`code` or `markdown`).\n\
            - Prefer reading the notebook structure first if cell indices are unknown.",
            self.description()
        )
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "notebook_path": { "type": "string" },
                "cell_id": { "type": "string", "description": "Cell metadata id or numeric index" },
                "new_source": { "type": "string" },
                "cell_type": { "type": "string", "enum": ["code", "markdown"] },
                "edit_mode": { "type": "string", "enum": ["replace", "insert", "delete"] }
            },
            "required": ["notebook_path"]
        })
    }

    fn permission_mode(&self) -> PermissionMode {
        PermissionMode::Default
    }

    fn security_policy(&self) -> Option<&SecurityPolicy> {
        Some(&self.security_policy)
    }

    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, CoreError> {
        let start = Instant::now();
        let wd = input.working_directory.as_deref();
        let sandbox_in = input.sandbox_mode;
        let nb: NbInput =
            serde_json::from_value(input.input).map_err(CoreError::SerializationError)?;

        let path = resolve_path_fields(
            self.security_policy.sandbox_mode,
            sandbox_in,
            wd,
            &nb.notebook_path,
        )?;

        let raw = tokio::fs::read_to_string(&path).await?;
        let original_file = raw.clone();
        let mut root: Value = serde_json::from_str(&raw)
            .map_err(|e| CoreError::Other(anyhow::anyhow!("invalid notebook json: {}", e)))?;
        let cells = root
            .get_mut("cells")
            .and_then(|c| c.as_array_mut())
            .ok_or_else(|| CoreError::Other(anyhow::anyhow!("notebook missing cells array")))?;

        let mode = nb.edit_mode.as_deref().unwrap_or("replace");

        match mode {
            "delete" => {
                let idx = find_cell_index(cells, nb.cell_id.as_deref()).ok_or_else(|| {
                    CoreError::Other(anyhow::anyhow!("cell not found for delete"))
                })?;
                cells.remove(idx);
            }
            "insert" => {
                let ct = nb.cell_type.as_deref().unwrap_or("code");
                if ct != "code" && ct != "markdown" {
                    return Ok(ToolOutput {
                        result: json!({"error": "cell_type must be code or markdown for insert"}),
                        error: Some("bad cell_type".into()),
                        duration_ms: start.elapsed().as_millis() as u64,
                    });
                }
                let mut cell = json!({
                    "cell_type": ct,
                    "metadata": {},
                    "source": nb.new_source,
                    "outputs": []
                });
                if ct == "markdown" {
                    if let Some(obj) = cell.as_object_mut() {
                        obj.remove("outputs");
                    }
                }
                let at = nb
                    .cell_id
                    .as_deref()
                    .and_then(|id| find_cell_index(cells, Some(id)))
                    .map(|i| i + 1)
                    .unwrap_or(0);
                let idx = at.min(cells.len());
                cells.insert(idx, cell);
            }
            _ => {
                let idx = find_cell_index(cells, nb.cell_id.as_deref()).ok_or_else(|| {
                    CoreError::Other(anyhow::anyhow!("cell not found for replace"))
                })?;
                set_cell_source(&mut cells[idx], &nb.new_source);
                if let Some(ct) = &nb.cell_type {
                    if let Some(obj) = cells[idx].as_object_mut() {
                        obj.insert("cell_type".into(), json!(ct));
                    }
                }
            }
        }

        let updated = serde_json::to_string_pretty(&root).map_err(CoreError::SerializationError)?;
        tokio::fs::write(&path, &updated).await?;

        Ok(ToolOutput {
            result: json!({
                "notebook_path": path.to_string_lossy(),
                "edit_mode": mode,
                "original_file": original_file.chars().take(4096).collect::<String>(),
                "updated_file": updated.chars().take(4096).collect::<String>(),
            }),
            error: None,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}
