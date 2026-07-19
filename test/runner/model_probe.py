"""Model capability probes via Dashboard API."""

from __future__ import annotations

import json
import tempfile
import time
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any

from .dashboard_client import DashboardClient

MODEL_ALIASES = {
    "local-1b": "sglang-minicpm5-1b",
    "managed-minicpm5-1b": "managed-minicpm5-1b",
    "agnes": "agnes-chat",
    "cloud-auto": "cloud-auto",
}

TOOL_DEPENDENT_SUITES = {"browser", "skills"}


@dataclass
class ModelProbe:
    alias: str
    model_id: str
    status: str
    available: bool
    tools: bool | None = None
    chat: bool | None = None
    phase: str | None = None
    source: str | None = None
    notes: list[str] = field(default_factory=list)
    incompatible_suites: list[str] = field(default_factory=list)
    metrics: dict[str, Any] = field(default_factory=dict)


def resolve_model_id(alias: str) -> str:
    return MODEL_ALIASES.get(alias, alias)


def probe_models(client: DashboardClient, aliases: list[str]) -> list[ModelProbe]:
    if not client.health():
        raise RuntimeError("dashboard not healthy — start scripts/dashboard-e2e-server.sh or workbench")

    local_payload = client.probe_local_models()
    local_models = {
        m.get("id"): m for m in local_payload.get("models", []) if isinstance(m, dict)
    }
    registry = client.get_models_registry()
    registry_items = {
        item.get("id"): item
        for item in registry.get("items", [])
        if isinstance(item, dict)
    }
    _, cloud_payload = client.probe_cloud()

    probes: list[ModelProbe] = []
    for alias in aliases:
        model_id = resolve_model_id(alias)
        probe = _probe_one(alias, model_id, local_models, registry_items, cloud_payload)
        probes.append(probe)
    return probes


def _probe_one(
    alias: str,
    model_id: str,
    local_models: dict[str, Any],
    registry_items: dict[str, Any],
    cloud_payload: Any,
) -> ModelProbe:
    if model_id == "managed-minicpm5-1b":
        return _probe_local(alias, model_id, local_models.get(model_id))
    if model_id == "sglang-minicpm5-1b":
        return _probe_sglang(alias, model_id, registry_items)
    if model_id == "agnes-chat":
        return _probe_registry(alias, model_id, registry_items, expected_model="agnes-chat")
    if model_id == "cloud-auto":
        return _probe_cloud_auto(alias, model_id, registry_items, cloud_payload)
    if model_id in local_models:
        return _probe_local(alias, model_id, local_models[model_id])
    if model_id in registry_items or any(
        item.get("model") == model_id for item in registry_items.values()
    ):
        return _probe_registry(alias, model_id, registry_items)
    return ModelProbe(
        alias=alias,
        model_id=model_id,
        status="missing",
        available=False,
        notes=[f"unknown model alias {alias}"],
    )


def _probe_local(alias: str, model_id: str, status: dict[str, Any] | None) -> ModelProbe:
    if not status:
        return ModelProbe(
            alias=alias,
            model_id=model_id,
            status="not_installed",
            available=False,
            notes=["local model not present in /api/local-models"],
        )
    caps = status.get("capabilities") or {}
    tools = bool(caps.get("tools"))
    chat = bool(caps.get("chat"))
    phase = str(status.get("phase", "unknown"))
    available = phase in {"ready", "running"}
    notes: list[str] = []
    incompatible: list[str] = []
    metrics: dict[str, Any] = {
        "context_tokens": status.get("context_tokens"),
        "sha256": status.get("sha256"),
        "runtime": status.get("runtime"),
    }
    if model_id == "managed-minicpm5-1b" and phase == "running":
        client = DashboardClient()
        tool_ok, tool_notes, tool_metrics = _probe_local_tool_loop(client, model_id=model_id)
        notes.extend(tool_notes)
        metrics.update(tool_metrics)
        tools = tool_ok
    elif model_id == "managed-minicpm5-1b" and tools and available:
        notes.append("runtime not running — tool loop not verified (metadata tools=true)")
    if not tools:
        incompatible = sorted(TOOL_DEPENDENT_SUITES)
        notes.append("tools unavailable — browser/skills suites marked N/A, not failed")
    if not available:
        notes.append(f"phase={phase} — start or download model before full eval")
    return ModelProbe(
        alias=alias,
        model_id=model_id,
        status=phase,
        available=available,
        tools=tools,
        chat=chat,
        phase=phase,
        source="managed_local",
        notes=notes,
        incompatible_suites=incompatible,
        metrics=metrics,
    )


def _probe_local_tool_loop(
    client: DashboardClient,
    *,
    model_id: str,
    timeout: float = 300.0,
) -> tuple[bool, list[str], dict[str, Any]]:
    """Execute read-only Glob → tool_result → final answer when runtime is up."""
    notes: list[str] = []
    metrics: dict[str, Any] = {
        "tool_probe": "skipped",
        "model_id": model_id,
        "agent_profile": "general-purpose",
    }
    with tempfile.TemporaryDirectory(prefix="anycode-tool-probe-") as tmp:
        workspace = Path(tmp)
        (workspace / "probe.txt").write_text("PROBE_MARKER\n", encoding="utf-8")
        try:
            project_id = client.create_project(str(workspace.resolve()), f"tool-probe-{model_id}")
            session_id = client.create_session(project_id, "tool-probe")
            prompt = (
                "Use the Glob tool with pattern '*.txt' to list txt files in the workspace. "
                "Do not use any other tools. After the tool returns, reply with exactly TOOL_PROBE_OK."
            )
            status_code, payload = client.send_message(
                session_id,
                prompt,
                agent="general-purpose",
                timeout=timeout,
            )
            if status_code >= 400:
                notes.append(f"tool probe message failed ({status_code}): {payload}")
                metrics["tool_probe"] = "send_failed"
                return False, notes, metrics
            final = client.wait_session_done(session_id, timeout=timeout)
            trace = client.get_trace(session_id)
            event_types = {
                e.get("event_type")
                for e in trace.get("events", [])
                if isinstance(e, dict)
            }
            has_tool_start = bool(
                event_types & {"tool_call_start", "tool_start"}
            )
            has_tool_result = bool(
                event_types & {"tool_call_end", "tool_result"}
            )
            has_trace_error = bool(
                event_types & {"session_error", "turn_error", "llm_error"}
            )
            metrics.update(
                {
                    "tool_probe": (
                        "passed"
                        if final == "completed"
                        and has_tool_start
                        and has_tool_result
                        and not has_trace_error
                        else "failed"
                    ),
                    "tool_probe_session_status": final,
                    "tool_probe_event_types": sorted(t for t in event_types if t),
                    "tool_probe_trace_error": has_trace_error,
                }
            )
            if final != "completed":
                notes.append(f"tool probe session ended with {final}")
            if not has_tool_start:
                notes.append("tool probe missing tool_call_start/tool_start in trace")
            if not has_tool_result:
                notes.append("tool probe missing tool_call_end/tool_result in trace")
            if has_trace_error:
                notes.append("tool probe trace contains a terminal runtime error")
            ok = (
                final == "completed"
                and has_tool_start
                and has_tool_result
                and not has_trace_error
            )
            if ok:
                notes.append("tool probe passed: Glob tool_call → tool_result → final answer")
            return ok, notes, metrics
        except (TimeoutError, RuntimeError, OSError) as exc:
            notes.append(f"tool probe error: {exc}")
            metrics["tool_probe"] = "error"
            return False, notes, metrics


def _probe_sglang(
    alias: str,
    model_id: str,
    registry_items: dict[str, Any],
) -> ModelProbe:
    item = registry_items.get(model_id)
    if item is None:
        return ModelProbe(
            alias=alias,
            model_id=model_id,
            status="not_configured",
            available=False,
            source="sglang",
            notes=["sglang-minicpm5-1b not found in /api/settings/models"],
        )
    base_url = str(item.get("base_url") or "http://127.0.0.1:30000/v1/chat/completions")
    health_url = base_url.replace("/v1/chat/completions", "/health")
    notes: list[str] = [f"registry model={item.get('model')}"]
    metrics: dict[str, Any] = {
        "provider": item.get("provider"),
        "model": item.get("model"),
        "base_url": base_url,
    }
    try:
        import urllib.request

        with urllib.request.urlopen(health_url, timeout=3) as resp:
            available = 200 <= resp.status < 300
    except OSError as exc:
        available = False
        notes.append(f"sglang health check failed ({health_url}): {exc}")
    tools: bool | None = None
    if available:
        client = DashboardClient()
        tool_ok, tool_notes, tool_metrics = _probe_local_tool_loop(
            client, model_id=model_id
        )
        notes.extend(tool_notes)
        metrics.update(tool_metrics)
        tools = tool_ok
    else:
        notes.append("start SGLang with --tool-call-parser minicpm5 before tool probe")
    incompatible = sorted(TOOL_DEPENDENT_SUITES) if tools is False else []
    return ModelProbe(
        alias=alias,
        model_id=model_id,
        status="ready" if available else "offline",
        available=available,
        tools=tools,
        chat=True,
        phase="running" if available else "offline",
        source="sglang",
        notes=notes,
        incompatible_suites=incompatible,
        metrics=metrics,
    )


def _probe_registry(
    alias: str,
    model_id: str,
    registry_items: dict[str, Any],
    *,
    expected_model: str | None = None,
) -> ModelProbe:
    item = registry_items.get(model_id)
    if item is None:
        for candidate in registry_items.values():
            if candidate.get("model") == (expected_model or model_id):
                item = candidate
                model_id = str(candidate.get("id", model_id))
                break
    if item is None:
        return ModelProbe(
            alias=alias,
            model_id=model_id,
            status="not_configured",
            available=False,
            source="registry",
            notes=["model not found in /api/settings/models"],
        )
    provider = item.get("provider", "")
    return ModelProbe(
        alias=alias,
        model_id=model_id,
        status="configured",
        available=True,
        tools=True,
        chat=True,
        source=str(provider or "registry"),
        notes=[f"registry item model={item.get('model')}"],
        metrics={"provider": provider, "model": item.get("model")},
    )


def _probe_cloud_auto(
    alias: str,
    model_id: str,
    registry_items: dict[str, Any],
    cloud_payload: Any,
) -> ModelProbe:
    linked = False
    if isinstance(cloud_payload, dict):
        linked = bool(cloud_payload.get("linked"))
    item = registry_items.get("cloud-auto")
    notes: list[str] = []
    if not linked:
        notes.append("cloud session not linked — routing regression only when linked")
    probe = _probe_registry(alias, model_id, registry_items, expected_model="auto")
    probe.notes.extend(notes)
    probe.metrics["cloud_linked"] = linked
    if item is None and linked:
        probe.available = True
        probe.status = "linked"
    return probe


def filter_cases_for_probe(case_suite: str, probes: list[ModelProbe]) -> str | None:
    """Return 'na' when all targeted models mark suite incompatible."""
    if case_suite not in TOOL_DEPENDENT_SUITES:
        return None
    local = next(
        (
            p
            for p in probes
            if p.model_id in {"managed-minicpm5-1b", "sglang-minicpm5-1b"}
        ),
        None,
    )
    if local and case_suite in local.incompatible_suites:
        return "na"
    return None


def write_model_reports(results_dir: Path, probes: list[ModelProbe]) -> list[Path]:
    out: list[Path] = []
    models_dir = results_dir / "models"
    models_dir.mkdir(parents=True, exist_ok=True)
    for probe in probes:
        lines = [
            f"# Model probe — {probe.alias} (`{probe.model_id}`)",
            "",
            f"- probe_status: **{probe.status}**",
            f"- available: {probe.available}",
            f"- tools: {probe.tools}",
            f"- source: {probe.source or 'n/a'}",
            "",
        ]
        if probe.incompatible_suites:
            lines.append("## N/A suites (capability gap)")
            for suite in probe.incompatible_suites:
                lines.append(f"- `{suite}`")
            lines.append("")
        if probe.notes:
            lines.append("## Notes")
            for note in probe.notes:
                lines.append(f"- {note}")
            lines.append("")
        if probe.metrics:
            lines.append("## Metrics")
            lines.append("```json")
            lines.append(json.dumps(probe.metrics, indent=2, ensure_ascii=False))
            lines.append("```")
            lines.append("")
        path = models_dir / probe.alias / "report.md"
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text("\n".join(lines), encoding="utf-8")
        out.append(path)
        json_path = models_dir / probe.alias / "probe.json"
        json_path.write_text(
            json.dumps(asdict(probe), indent=2, ensure_ascii=False) + "\n",
            encoding="utf-8",
        )
        out.append(json_path)
    return out


def probe_latency(client: DashboardClient) -> float:
    started = time.time()
    client.health()
    return time.time() - started
