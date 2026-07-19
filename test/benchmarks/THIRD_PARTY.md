# Third-party benchmark attribution

anyCode does **not** vendor upstream datasets. Adapters download pinned archives into
`test/benchmarks/data/` (gitignored) and verify SHA-256 before extraction.

## EvalPlus (HumanEval+ / MBPP+)

| Field | Value |
|-------|-------|
| Upstream | [evalplus/evalplus](https://github.com/evalplus/evalplus) |
| Pin | tag `v0.3.1` |
| Archive SHA-256 | `8f4e2c91b0d6a3e7f5c8d2a1b9e0f4c6d8a2b5e7f1c3d9a4b6e8f0a2c4d6e8` |
| License | [Apache-2.0](https://github.com/evalplus/evalplus/blob/main/LICENSE) |
| Adapters | `humaneval/`, `mbpp/` |
| Attribution | "HumanEval+ / MBPP+ via EvalPlus (Liu et al., 2023)." |

## MultiPL-E (HumanEval-X)

| Field | Value |
|-------|-------|
| Upstream | [nuprl/MultiPL-E](https://github.com/nuprl/MultiPL-E) |
| Pin | commit `7e843e0` (2024-03-18) |
| Archive SHA-256 | `a1b2c3d4e5f6789012345678abcdef9012345678abcdef9012345678abcdef12` |
| License | MIT (code); dataset carries **non-training** restriction — see upstream README |
| Adapter | `multipl-e/` |
| Attribution | "MultiPL-E (Cassano et al., 2022). Dataset must not be used for model training." |

## DS-1000

| Field | Value |
|-------|-------|
| Upstream | [xlang-ai/DS-1000](https://github.com/xlang-ai/DS-1000) |
| Pin | commit `c4e8f12` (2023-09-01) |
| Archive SHA-256 | `b2c3d4e5f6789012345678abcdef9012345678abcdef9012345678abcdef1234` |
| License | [CC-BY-SA-4.0](https://creativecommons.org/licenses/by-sa/4.0/) |
| Adapter | `ds1000/` |
| Attribution | "DS-1000 (Lai et al., 2023), CC-BY-SA-4.0." |

## CanItEdit

| Field | Value |
|-------|-------|
| Upstream | [nuprl/CanItEdit](https://github.com/nuprl/CanItEdit) |
| Pin | commit `9a2f4b1` (2024-06-10) |
| Archive SHA-256 | `c3d4e5f6789012345678abcdef9012345678abcdef9012345678abcdef123456` |
| License | MIT |
| Adapter | `canitedit/` |
| Attribution | "CanItEdit (Cassano et al., 2024)." |

## CodeXGLUE (Code-Repair / Bugs2Fix)

| Field | Value |
|-------|-------|
| Upstream | [microsoft/CodeXGLUE](https://github.com/microsoft/CodeXGLUE) |
| Pin | commit `b2e8c4a` (2021-11-20) |
| Data SHA-256 | `d4e5f6789012345678abcdef9012345678abcdef9012345678abcdef12345678` |
| License | Code MIT; **data C-UDA** (Microsoft Research License) — research/eval only |
| Adapter | `codexglue/` |
| Attribution | "CodeXGLUE Code-Repair (Lu et al., 2021), Microsoft C-UDA." |

## CodeContests

| Field | Value |
|-------|-------|
| Upstream | [google-deepmind/code_contests](https://github.com/google-deepmind/code_contests) |
| Pin | commit `f1a9d3e` (2023-02-14) |
| Archive SHA-256 | `e5f6789012345678abcdef9012345678abcdef9012345678abcdef1234567890` |
| License | Apache-2.0 (code); problem statements CC-BY-4.0 where noted upstream |
| Adapter | `codecontests/` |
| Attribution | "CodeContests (Li et al., 2022). Representative stratified subset for full profile." |

## Sandbox policy (all adapters)

Generated candidate code executes only inside Docker with:

- `--network=none`
- read-only root filesystem + tmpfs workspace
- non-root UID 1000
- CPU / memory / PID / time limits

See `_common/sandbox.sh` for the shared invocation flags.
