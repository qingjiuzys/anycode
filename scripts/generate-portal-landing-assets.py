#!/usr/bin/env python3
"""Generate account-portal landing assets via Agnes image API."""
from __future__ import annotations

import json
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "crates" / "account-portal" / "public" / "assets" / "landing"
CONFIG = Path.home() / ".anycode" / "config.json"

PROMPTS: dict[str, str] = {
    "hero-bg": (
        "Cinematic wide hero background for AI developer platform: deep navy and violet "
        "gradient atmosphere, subtle light rays, abstract cloud compute nodes, premium "
        "corporate minimal style, no text, no logos, 16:9"
    ),
    "feature-account": (
        "Abstract product card visual: secure cloud identity and login, soft purple glow, "
        "minimal 3D shapes, dark elegant, no text"
    ),
    "feature-models": (
        "Abstract product card visual: AI model routing and neural network hub, blue tones, "
        "minimal futuristic, no text"
    ),
    "feature-billing": (
        "Abstract product card visual: subscription billing dashboard aesthetic, neutral "
        "stone and silver tones, minimal, no text"
    ),
    "feature-devices": (
        "Abstract product card visual: laptop and mobile device sync, green accent, "
        "minimal tech, no text"
    ),
}


def load_agnes() -> tuple[str, str, str]:
    cfg = json.loads(CONFIG.read_text())
    key = cfg["provider_credentials"]["agnes"]
    items = cfg.get("models", {}).get("items", [])
    img = next((i for i in items if i.get("id") == "agnes-image-2-1-flash"), None)
    base = (img or {}).get("base_url") or "https://apihub.agnes-ai.com/v1"
    model = (img or {}).get("model") or "agnes-image-2.1-flash"
    return key, base.rstrip("/"), model


def generate_image(api_key: str, base_url: str, model: str, prompt: str) -> str:
    url = f"{base_url}/images/generations"
    body = json.dumps(
        {"model": model, "prompt": prompt, "n": 1, "size": "1024x1024"}
    ).encode()
    req = urllib.request.Request(
        url,
        data=body,
        headers={"Authorization": f"Bearer {api_key}", "Content-Type": "application/json"},
    )
    with urllib.request.urlopen(req, timeout=180) as resp:
        data = json.loads(resp.read().decode())
    item = data.get("data", [{}])[0]
    if item.get("url"):
        return item["url"]
    if item.get("b64_json"):
        return f"data:image/png;base64,{item['b64_json']}"
    raise RuntimeError(f"no image in response: {list(data.keys())}")


def download(url: str, dest: Path) -> None:
    dest.parent.mkdir(parents=True, exist_ok=True)
    if url.startswith("data:"):
        import base64

        header, b64 = url.split(",", 1)
        dest.write_bytes(base64.b64decode(b64))
        return
    with urllib.request.urlopen(url, timeout=120) as resp:
        dest.write_bytes(resp.read())


def generate_video(api_key: str, submit_url: str, status_tpl: str, model: str, prompt: str) -> str:
    body = json.dumps({"model": model, "prompt": prompt}).encode()
    req = urllib.request.Request(
        submit_url,
        data=body,
        headers={"Authorization": f"Bearer {api_key}", "Content-Type": "application/json"},
    )
    with urllib.request.urlopen(req, timeout=60) as resp:
        data = json.loads(resp.read().decode())
    job_id = data.get("id") or data.get("task_id") or (data.get("data") or {}).get("id")
    if not job_id:
        raise RuntimeError(f"no job id: {data}")
    status_url = status_tpl.replace("{id}", job_id)
    for _ in range(60):
        time.sleep(5)
        poll = urllib.request.Request(
            status_url,
            headers={"Authorization": f"Bearer {api_key}"},
        )
        with urllib.request.urlopen(poll, timeout=60) as resp:
            payload = json.loads(resp.read().decode())
        status = (payload.get("status") or "").lower()
        if status in {"completed", "succeeded", "success"}:
            for key in ("video_url", "url"):
                if payload.get(key, "").startswith("http"):
                    return payload[key]
            for ptr in ("/data/0/url", "/data/0/video_url", "/result/url"):
                node = payload
                for part in ptr.strip("/").split("/"):
                    node = node.get(part, {}) if isinstance(node, dict) else {}
                if isinstance(node, str) and node.startswith("http"):
                    return node
            raise RuntimeError(f"completed but no url: {payload}")
        if status in {"failed", "error", "cancelled", "canceled"}:
            raise RuntimeError(f"video failed: {payload}")
    raise RuntimeError("video poll timeout")


def main() -> int:
    if not CONFIG.is_file():
        print(f"missing {CONFIG}", file=sys.stderr)
        return 1
    api_key, base_url, model = load_agnes()
    OUT.mkdir(parents=True, exist_ok=True)
    manifest: dict[str, str] = {}

    for name, prompt in PROMPTS.items():
        dest = OUT / f"{name}.jpg"
        if dest.is_file() and dest.stat().st_size > 10_000:
            print(f"skip {name} (exists)")
            manifest[name] = f"/assets/landing/{name}.jpg"
            continue
        print(f"generating {name}...")
        try:
            image_url = generate_image(api_key, base_url, model, prompt)
            download(image_url, dest)
            manifest[name] = f"/assets/landing/{name}.jpg"
            print(f"  saved {dest} ({dest.stat().st_size} bytes)")
            time.sleep(1)
        except urllib.error.HTTPError as e:
            print(f"  failed {name}: HTTP {e.code} {e.read().decode()[:200]}", file=sys.stderr)
        except Exception as e:
            print(f"  failed {name}: {e}", file=sys.stderr)

    (OUT / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")

    video_dest = OUT / "hero-loop.mp4"
    if not video_dest.is_file() or video_dest.stat().st_size < 50_000:
        print("generating hero video...")
        try:
            items = json.loads(CONFIG.read_text()).get("models", {}).get("items", [])
            vid = next((i for i in items if i.get("id") == "agnes-video-v2-0"), None)
            submit = "https://apihub.agnes-ai.com/v1/videos"
            status_tpl = "https://apihub.agnes-ai.com/v1/videos/{id}"
            if vid and vid.get("endpoint_overrides"):
                submit = vid["endpoint_overrides"].get("submit", submit)
                status_tpl = vid["endpoint_overrides"].get("status", status_tpl)
            vmodel = (vid or {}).get("model") or "agnes-video-v2.0"
            vurl = generate_video(
                api_key,
                submit,
                status_tpl,
                vmodel,
                "Slow cinematic loop: abstract purple blue light flowing in dark space, AI cloud platform atmosphere, seamless, no text",
            )
            download(vurl, video_dest)
            manifest["hero-loop"] = "/assets/landing/hero-loop.mp4"
            print(f"  saved {video_dest} ({video_dest.stat().st_size} bytes)")
            (OUT / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
        except Exception as e:
            print(f"  video skipped: {e}", file=sys.stderr)

    print(f"done — {len(manifest)} assets in {OUT}")
    return 0 if manifest else 1


if __name__ == "__main__":
    raise SystemExit(main())
