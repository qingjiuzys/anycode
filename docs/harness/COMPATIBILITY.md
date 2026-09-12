# Compatibility and deliberate non-goals

| Surface | Added | Not claimed |
|---|---|---|
| Existing AnyCode | optional feature bridge, existing Message/LLMClient/Tool pipeline | complete lifecycle migration or existing regression pass |
| Pi | independent Rust implementation of small-kernel/host-extension boundaries | TS extension compatibility, a vendored Pi or CLI replacement |
| Graph | native versioned DAG + conditions + barrier join + Human + verifier + checkpoint | Grapl security API, LangGraph Python import, arbitrary cycles |
| Subagents | structured supervisor, lineage/cancel, shared budget, graph→kernel adapter | remote distributed worker service or finished UI tools |
| Computer | broker and concrete opt-in X11 backend | macOS, Windows, Wayland or real hardware validation |
| Skills | controlled metadata/search/activate + 12 instruction examples | a new execution backend for every described skill |
| 818cloud | real server-side SSO v2 contract client and local ACL interface | finished product login, desktop pairing, billing API or database mapping |
| Persistence | local journal/checkpoint with leases | distributed DB transactions, replicated budget ledger or exactly-once external effects |

New code has no shipping OS credentials or provider keys. Example UUIDs and fake providers are explicitly fixtures, not production identity or model results.
