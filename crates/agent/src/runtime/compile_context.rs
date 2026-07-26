//! Shared task compilation for execute_task and execute_turn.

use crate::task_compiler::{
    attributed_memories_sections, CompileArmFlags, CompiledPromptParts, MemoryRecallBudgets,
    TaskCompiler,
};
use anycode_core::ExpectedArtifact;
use anycode_core::{builtin_web_and_rust_pack, GatePlan, Memory, MemoryType, TaskFamily};
use anycode_tools::{resolve_capabilities, SkillCatalog, SkillResolutionContext, SkillsGovernance};
use std::path::Path;
use std::sync::{Arc, Mutex as StdMutex};

/// Everything both orchestrators need from one TaskCompiler pass.
pub struct CompiledContext {
    pub parts: CompiledPromptParts,
    pub sections: Vec<String>,
    pub gate_plan: Option<GatePlan>,
    pub expected_artifacts: Vec<ExpectedArtifact>,
    pub family: Option<TaskFamily>,
    pub skill_denies: Vec<String>,
    pub arm: CompileArmFlags,
    pub recalled: Vec<(MemoryType, Vec<Memory>)>,
}

/// Extract skill catalog + governance from shared tool services (single copy —
/// used to live in both execute_task and execute_turn).
pub fn skill_catalog_and_governance(
    tool_services: &StdMutex<Option<Arc<anycode_tools::ToolServices>>>,
) -> (Option<Arc<SkillCatalog>>, Option<SkillsGovernance>) {
    let services = tool_services.lock().ok().and_then(|g| g.clone());
    match services {
        Some(s) => {
            let gov = s.skills_governance.lock().ok().map(|g| g.clone());
            (Some(s.skill_catalog.clone()), gov)
        }
        None => (None, None),
    }
}

/// Recall typed memories + run TaskCompiler + derive sections/gates/denies.
/// `strict_recall`: execute_task fails the whole task on recall error;
/// execute_turn swallows to default (pass `false`).
pub async fn compile_for_prompt(
    memory_store: &dyn anycode_core::MemoryStore,
    tool_services: &StdMutex<Option<Arc<anycode_tools::ToolServices>>>,
    prompt: &str,
    agent_type: &str,
    working_directory: &str,
    strict_recall: bool,
) -> Result<CompiledContext, anycode_core::CoreError> {
    let arm = CompileArmFlags::from_eval_env();
    let recalled = if strict_recall {
        recall_typed_memories(memory_store, prompt).await?
    } else {
        recall_typed_memories(memory_store, prompt)
            .await
            .unwrap_or_default()
    };
    let (skill_catalog, governance) = skill_catalog_and_governance(tool_services);
    let parts = compile_task_context(
        prompt,
        &recalled,
        arm,
        skill_catalog.as_deref(),
        governance.as_ref(),
        agent_type,
        Some(Path::new(working_directory)),
    );
    let sections = compiler_context_sections(&parts);
    let gate_plan = parts.gate_plan.clone();
    let expected_artifacts = parts.task_spec.expected_artifacts.clone();
    let family = parts.task_spec.family;
    let skill_denies = skill_tool_denies_for_arm(&parts, arm);
    Ok(CompiledContext {
        parts,
        sections,
        gate_plan,
        expected_artifacts,
        family,
        skill_denies,
        arm,
        recalled,
    })
}

/// Stable one-line markers so both orchestrators log gates/skills identically.
pub fn gate_plan_marker(
    family: Option<TaskFamily>,
    requirements: usize,
    arm: CompileArmFlags,
) -> String {
    format!(
        "[gate_plan_created] family={:?} requirements={} arm=exp:{}:skills:{}",
        family, requirements, arm.experience_enabled as u8, arm.production_skills_enabled as u8
    )
}

pub fn skill_resolved_marker(selected: &[String]) -> String {
    format!("[skill_resolved] selected={}", selected.join(","))
}

pub async fn recall_typed_memories(
    store: &dyn anycode_core::MemoryStore,
    prompt: &str,
) -> Result<Vec<(MemoryType, Vec<Memory>)>, anycode_core::CoreError> {
    let budgets = MemoryRecallBudgets::default();
    let mut recalled = Vec::new();
    for (mt, limit) in [
        (MemoryType::User, budgets.user),
        (MemoryType::Feedback, budgets.feedback),
        (MemoryType::Project, budgets.project),
        (MemoryType::Reference, budgets.reference),
    ] {
        let mut hits = store.recall(prompt, mt).await?;
        hits.truncate(limit);
        recalled.push((mt, hits));
    }
    Ok(recalled)
}

pub fn compile_task_context(
    prompt: &str,
    recalled: &[(MemoryType, Vec<Memory>)],
    arm: CompileArmFlags,
    skill_catalog: Option<&SkillCatalog>,
    governance: Option<&SkillsGovernance>,
    agent_type: &str,
    project_root: Option<&Path>,
) -> CompiledPromptParts {
    let pack = builtin_web_and_rust_pack();
    let mut parts = TaskCompiler::new(&pack)
        .with_arm(arm)
        .compile(prompt, recalled);

    if let (Some(catalog), Some(gov)) = (skill_catalog, governance) {
        let resolution = resolve_capabilities(
            &parts.task_spec.required_capabilities,
            catalog,
            gov,
            &SkillResolutionContext {
                agent_type: agent_type.to_string(),
                project_root: project_root.map(|p| p.to_path_buf()),
                platform: std::env::consts::OS.to_string(),
                production_skills_enabled: arm.production_skills_enabled,
            },
        );
        parts.denied_skill_ids = resolution.denied_skill_ids;
        if arm.production_skills_enabled {
            parts.skill_segment = resolution.prompt_segment;
            parts.selected_skill_ids = resolution
                .selected
                .iter()
                .filter(|s| !s.skill_id.is_empty())
                .map(|s| s.skill_id.clone())
                .collect();
            if !parts.selected_skill_ids.is_empty() {
                parts
                    .task_spec
                    .extras
                    .insert("selected_skills".into(), parts.selected_skill_ids.join(","));
            }
        } else {
            parts.skill_segment = String::new();
            parts.selected_skill_ids.clear();
        }
        parts.task_spec.extras.insert(
            "eval_arm".into(),
            format!(
                "experience={} skills={}",
                arm.experience_enabled as u8, arm.production_skills_enabled as u8
            ),
        );
    }

    parts
}

pub fn compiler_context_sections(parts: &CompiledPromptParts) -> Vec<String> {
    let mut sections = attributed_memories_sections(&parts.memories_by_type);
    let task_seg = parts.task_spec.to_prompt_segment();
    if !task_seg.trim().is_empty() {
        sections.push(task_seg);
    }
    if !parts.preferences_segment.trim().is_empty() {
        sections.push(parts.preferences_segment.clone());
    }
    if !parts.experience_segment.trim().is_empty() {
        sections.push(parts.experience_segment.clone());
    }
    if !parts.skill_segment.trim().is_empty() {
        sections.push(parts.skill_segment.clone());
    }
    if let Some(plan) = &parts.gate_plan {
        if !plan.is_empty() {
            let gates: Vec<_> = plan
                .requirements
                .iter()
                .map(|r| format!("{}:{}", r.id, r.validator_id))
                .collect();
            sections.push(format!(
                "## Gate Plan\nindependent validators will check before completion:\n- {}",
                gates.join("\n- ")
            ));
        }
    }
    sections
}

/// Extra tool deny names when the eval/production arm disables Skills.
pub fn skill_tool_denies_for_arm(
    _parts: &CompiledPromptParts,
    arm: CompileArmFlags,
) -> Vec<String> {
    // Always deny Skill tools when the arm disables production skills — do not
    // depend on catalog/governance being present (dashboard may omit governance).
    if !arm.production_skills_enabled {
        return vec!["Skill".into(), "SkillSearch".into()];
    }
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task_compiler::TaskCompiler;
    use anycode_core::builtin_web_and_rust_pack;

    #[test]
    fn skills_off_always_denies_skill_tools() {
        let pack = builtin_web_and_rust_pack();
        let parts = TaskCompiler::new(&pack)
            .with_arm(CompileArmFlags {
                experience_enabled: false,
                production_skills_enabled: false,
                eval_mode: true,
            })
            .compile("export report.docx", &[]);
        let denies = skill_tool_denies_for_arm(
            &parts,
            CompileArmFlags {
                experience_enabled: false,
                production_skills_enabled: false,
                eval_mode: true,
            },
        );
        assert!(denies.contains(&"Skill".into()));
        assert!(denies.contains(&"SkillSearch".into()));
    }
}
