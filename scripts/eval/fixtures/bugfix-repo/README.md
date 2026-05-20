# Bugfix eval fixture

SWE-bench-lite style: `add` subtracts instead of adds. The mock-LLM scenario applies a scripted `Edit` fix; `cargo test` must pass afterward.
