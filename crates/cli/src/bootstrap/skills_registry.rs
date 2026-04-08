//! Optional HTTP `skills.registry_url` manifest: merge local scan roots before `SkillCatalog::scan`.

use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct SkillRegistryManifest {
    /// 追加的技能根目录（须本机已存在）；与 OpenClaw 式自托管 manifest 对齐的极简子集。
    #[serde(default)]
    extra_scan_roots: Vec<PathBuf>,
}

pub(crate) async fn fetch_extra_skill_roots(url: &str) -> Vec<PathBuf> {
    let url = url.trim();
    if url.is_empty() {
        return Vec::new();
    }
    let Ok(client) = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
    else {
        return Vec::new();
    };
    let Ok(resp) = client.get(url).send().await else {
        tracing::warn!(target: "anycode_cli", "skills.registry_url: request failed");
        return Vec::new();
    };
    let Ok(text) = resp.text().await else {
        tracing::warn!(target: "anycode_cli", "skills.registry_url: empty body");
        return Vec::new();
    };
    let Some(paths) = parse_manifest_extra_scan_roots(&text) else {
        tracing::warn!(target: "anycode_cli", "skills.registry_url: invalid JSON manifest");
        return Vec::new();
    };
    filter_existing_skill_scan_roots(paths)
}

fn parse_manifest_extra_scan_roots(text: &str) -> Option<Vec<PathBuf>> {
    serde_json::from_str::<SkillRegistryManifest>(text)
        .ok()
        .map(|m| m.extra_scan_roots)
}

fn filter_existing_skill_scan_roots(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    paths.into_iter().filter(|p| p.is_dir()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn parse_manifest_rejects_invalid_json() {
        assert!(parse_manifest_extra_scan_roots("{").is_none());
    }

    #[test]
    fn parse_manifest_reads_extra_scan_roots() {
        let json = r#"{"extra_scan_roots":["/a","/b"]}"#;
        let v = parse_manifest_extra_scan_roots(json).expect("valid");
        assert_eq!(v.len(), 2);
        assert_eq!(v[0], PathBuf::from("/a"));
    }

    #[test]
    fn parse_manifest_default_empty_roots() {
        let json = "{}";
        let v = parse_manifest_extra_scan_roots(json).expect("valid");
        assert!(v.is_empty());
    }

    #[test]
    fn filter_keeps_only_existing_dirs() {
        let tmp = std::env::temp_dir().join(format!(
            "anycode_skill_registry_test_{}",
            std::process::id()
        ));
        fs::create_dir_all(&tmp).unwrap();
        let paths = vec![tmp.clone(), PathBuf::from("/nonexistent_path_xyz_abc_999")];
        let filtered = filter_existing_skill_scan_roots(paths);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0], tmp);
        let _ = fs::remove_dir(&tmp);
    }
}
