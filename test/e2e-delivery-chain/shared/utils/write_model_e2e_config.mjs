#!/usr/bin/env node
/**
 * Write e2e harness config for a model profile:
 *   agnes | local-1b (default: SGLang MiniCPM5-1B + minicpm5 parser)
 *   LOCAL_1B_BACKEND=ollama for legacy Ollama GGUF path.
 */
import { readFileSync, writeFileSync, mkdirSync, existsSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { homedir } from "node:os";

const harnessRoot = join(dirname(fileURLToPath(import.meta.url)), "../..");
const profile = (process.env.E2E_MODEL_PROFILE ?? process.argv[2] ?? "agnes").trim();
const outPath =
  process.argv[3] ?? join(harnessRoot, `out/e2e-anycode.${profile}.config.json`);
const userPath = join(homedir(), ".anycode/config.json");

let base = {};
if (existsSync(userPath)) {
  base = JSON.parse(readFileSync(userPath, "utf8"));
}

base.runtime = {
  ...(base.runtime ?? {}),
  max_agent_turns: Number(process.env.ANYCODE_MAX_AGENT_TURNS ?? 9999),
  max_tool_calls: Number(process.env.ANYCODE_MAX_TOOL_CALLS ?? 50_000),
};

base.security = {
  ...(base.security ?? {}),
  permission_mode: "bypass",
  require_approval: false,
  sandbox_mode: false,
};

base.skills = {
  ...(base.skills ?? {}),
  enabled: true,
};

function localSession() {
  return {
    ...(base.session ?? {}),
    context_window_auto: false,
    context_window_tokens: 32_768,
    auto_compact: true,
    auto_compact_ratio: 0.88,
  };
}

const localBackend = (process.env.LOCAL_1B_BACKEND ?? "sglang").trim().toLowerCase();

if (profile === "local-1b" || profile === "ollama" || profile === "sglang") {
  base.session = localSession();
  const agnesKey = base.provider_credentials?.agnes ?? base.api_key ?? "sglang";
  base.temperature = base.temperature ?? 0.7;
  base.max_tokens = Math.min(base.max_tokens ?? 4096, 4096);
  base.plan = "general";

  if (profile === "ollama" || localBackend === "ollama") {
    base.provider = "ollama";
    base.model = process.env.OLLAMA_MODEL ?? "minicpm5-1b-e2e";
    base.api_key = "ollama";
    base.base_url =
      process.env.OLLAMA_BASE_URL ?? "http://127.0.0.1:11434/v1/chat/completions";
    base.models = {
      ...(base.models ?? {}),
      active: { ...(base.models?.active ?? {}), chat: "ollama-minicpm5-1b" },
      items: [
        ...(base.models?.items ?? []).filter((i) => i.id !== "ollama-minicpm5-1b"),
        {
          id: "ollama-minicpm5-1b",
          provider: "ollama",
          model: process.env.OLLAMA_MODEL ?? "minicpm5-1b-e2e",
          base_url:
            process.env.OLLAMA_BASE_URL ?? "http://127.0.0.1:11434/v1/chat/completions",
          api_key: "ollama",
          capabilities: ["chat"],
          enabled: true,
          display_name: "Ollama MiniCPM5-1B (legacy)",
          max_tokens: 4096,
          temperature: 0.7,
          plan: "general",
          source: "e2e-harness",
        },
      ],
    };
  } else {
    const sglangModel = process.env.SGLANG_MODEL ?? "MiniCPM5-1B";
    const sglangBase =
      process.env.SGLANG_BASE_URL ?? "http://127.0.0.1:30000/v1/chat/completions";
    base.provider = "sglang";
    base.model = sglangModel;
    base.api_key = "sglang";
    base.base_url = sglangBase;
    base.models = {
      ...(base.models ?? {}),
      active: { ...(base.models?.active ?? {}), chat: "sglang-minicpm5-1b" },
      items: [
        ...(base.models?.items ?? []).filter((i) => i.id !== "sglang-minicpm5-1b"),
        {
          id: "sglang-minicpm5-1b",
          provider: "sglang",
          model: sglangModel,
          base_url: sglangBase,
          api_key: "sglang",
          capabilities: ["chat"],
          enabled: true,
          display_name: "SGLang MiniCPM5-1B",
          max_tokens: 4096,
          temperature: 0.7,
          plan: "general",
          source: "e2e-harness",
        },
      ],
    };
  }

  if (agnesKey && !["ollama", "sglang"].includes(agnesKey)) {
    base.provider_credentials = { ...(base.provider_credentials ?? {}), agnes: agnesKey };
  }
} else if (profile === "agnes") {
  const agnesKey = base.provider_credentials?.agnes ?? base.api_key;
  if (!agnesKey) {
    console.error("agnes profile: no API key in ~/.anycode/config.json provider_credentials.agnes");
    process.exit(1);
  }
  const agnesItem =
    (base.models?.items ?? []).find((i) => i.id === "agnes-2-0-flash") ??
    (base.models?.items ?? []).find((i) => String(i.model ?? "").includes("agnes"));
  const agnesModel = agnesItem?.model ?? "agnes-2.0-flash";
  const agnesBase = agnesItem?.base_url ?? "https://apihub.agnes-ai.com/v1/chat/completions";
  const agnesId = agnesItem?.id ?? "agnes-2-0-flash";
  base.api_key = agnesKey;
  base.provider = "custom";
  base.model = agnesModel;
  base.base_url = agnesBase;
  base.provider_credentials = { ...(base.provider_credentials ?? {}), agnes: agnesKey };
  base.models = {
    ...(base.models ?? {}),
    active: {
      ...(base.models?.active ?? {}),
      chat: agnesId,
    },
    items: [
      ...(base.models?.items ?? []).filter((i) => i.id !== agnesId),
      {
        id: agnesId,
        provider: "custom",
        model: agnesModel,
        base_url: agnesBase,
        api_key: agnesKey,
        capabilities: ["chat"],
        enabled: true,
        display_name: agnesItem?.display_name ?? "Agnes 2.0 Flash",
        max_tokens: Math.max(agnesItem?.max_tokens ?? 8192, 16384),
        temperature: agnesItem?.temperature ?? 0.7,
        plan: "general",
        source: "e2e-harness",
      },
    ],
  };
}

mkdirSync(dirname(outPath), { recursive: true });
writeFileSync(outPath, JSON.stringify(base, null, 2) + "\n");
const backend =
  profile === "local-1b" || profile === "sglang" || profile === "ollama"
    ? profile === "ollama" || localBackend === "ollama"
      ? "ollama"
      : "sglang"
    : profile;
console.log(`[write_model_e2e_config] profile=${profile} backend=${backend} -> ${outPath}`);
