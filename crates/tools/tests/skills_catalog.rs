//! Integration tests for `SkillCatalog::scan` (fixtures under `tests/fixtures/skills`).

use anycode_tools::SkillCatalog;
use std::path::PathBuf;

fn fixture_root(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/skills")
        .join(name)
}

#[test]
fn scan_later_root_overwrites_same_id() {
    let low = fixture_root("low");
    let high = fixture_root("high");
    let cat = SkillCatalog::scan(&[low, high], None, 60_000, false);
    let foo = cat.metas().iter().find(|m| m.id == "foo").expect("foo");
    assert_eq!(foo.description, "from high root (wins)");
}

#[test]
fn scan_skips_mismatched_directory_and_frontmatter_name() {
    let low = fixture_root("low");
    let cat = SkillCatalog::scan(&[low], None, 60_000, false);
    assert!(
        cat.metas().iter().all(|m| m.id != "bar"),
        "bar/ with name wrongdir should be skipped"
    );
}

#[test]
fn scan_allowlist_filters() {
    let low = fixture_root("low");
    let high = fixture_root("high");
    let cat = SkillCatalog::scan(&[low, high], Some(&["foo".to_string()]), 60_000, false);
    assert_eq!(cat.metas().len(), 1);
    assert_eq!(cat.metas()[0].id, "foo");
}

#[test]
fn baz_has_run_flag() {
    let low = fixture_root("low");
    let cat = SkillCatalog::scan(&[low], None, 60_000, false);
    let baz = cat.metas().iter().find(|m| m.id == "baz").expect("baz");
    assert!(baz.has_run);
}

#[test]
fn render_prompt_mentions_skill_ids() {
    let low = fixture_root("low");
    let high = fixture_root("high");
    let cat = SkillCatalog::scan(&[low, high], None, 60_000, false);
    let s = cat.render_prompt_subsection().expect("non-empty");
    assert!(s.contains("## Available skills"));
    assert!(s.contains("**foo**"));
    assert!(s.contains("**baz**"));
    assert!(s.contains("mode=plan"));
    assert!(s.contains("model=plan"));
    assert!(s.contains("approval=required"));
}

#[test]
fn parses_extended_skill_frontmatter() {
    let low = fixture_root("low");
    let high = fixture_root("high");
    let cat = SkillCatalog::scan(&[low, high], None, 60_000, false);
    let foo = cat.metas().iter().find(|m| m.id == "foo").expect("foo");
    assert_eq!(foo.mode.as_deref(), Some("plan"));
    assert_eq!(foo.model.as_deref(), Some("plan"));
    assert_eq!(foo.approval.as_deref(), Some("required"));
    assert_eq!(foo.channel_capabilities, vec!["inlineButtons"]);
}

#[test]
fn truncate_skill_output_appends_marker() {
    let s: String = (0..100).map(|_| 'x').collect();
    let t = anycode_tools::truncate_skill_output(s.clone(), 20);
    assert!(t.len() < s.len());
    assert!(t.contains("truncated"));
}

#[test]
fn render_prompt_allowlist_only_lists_matching_ids() {
    let low = fixture_root("low");
    let high = fixture_root("high");
    let cat = SkillCatalog::scan(&[low, high], None, 60_000, false);
    let s = cat
        .render_prompt_subsection_allowlist(Some(&["foo".to_string()]))
        .expect("foo in catalog");
    assert!(s.contains("**foo**"));
    assert!(!s.contains("**baz**"));
}

#[test]
fn render_prompt_allowlist_empty_ids_yield_none() {
    let low = fixture_root("low");
    let cat = SkillCatalog::scan(&[low], None, 60_000, false);
    assert!(cat
        .render_prompt_subsection_allowlist(Some(&[String::new(), "  ".to_string()]))
        .is_none());
}
