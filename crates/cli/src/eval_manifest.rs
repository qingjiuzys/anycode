//! Single source of truth for eval scenarios (`scripts/eval/scenarios.json`).

use serde::Deserialize;

const MANIFEST_JSON: &str = include_str!("../../../scripts/eval/scenarios.json");

#[derive(Debug, Clone, Deserialize, serde::Serialize)]
pub(crate) struct CliEvalScenario {
    pub id: String,
    pub area: String,
    pub command: Vec<String>,
    pub expect: String,
    pub acceptance: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct MockFixtureScenario {
    pub id: String,
    pub area: String,
    pub fixture: String,
    pub acceptance: String,
}

#[derive(Debug, Deserialize)]
struct EvalManifest {
    cli_scenarios: Vec<CliEvalScenario>,
    mock_fixtures: Vec<MockFixtureScenario>,
}

fn load_manifest() -> EvalManifest {
    serde_json::from_str(MANIFEST_JSON).expect("scripts/eval/scenarios.json must parse")
}

pub(crate) fn cli_scenarios() -> Vec<CliEvalScenario> {
    load_manifest().cli_scenarios
}

pub(crate) fn mock_fixture_scenarios() -> Vec<MockFixtureScenario> {
    load_manifest().mock_fixtures
}
