#!/usr/bin/env python3
"""Scan account-portal/public/downloads and write releases.json + latest.json + SHA256SUMS.txt."""

from __future__ import annotations

import hashlib
import json
import re
import sys
from datetime import datetime, timezone
from pathlib import Path

BASE_URL = "https://anycode.work/downloads"

# anyCode_0.40.0_aarch64.dmg | anyCode_0.40.0_x86_64.dmg | anyCode_0.40.0_x64.msi|exe
VERSIONED_RE = re.compile(
    r"^anyCode_(?P<version>\d+\.\d+\.\d+)_(?P<arch>aarch64|x86_64|x64)\.(?P<ext>dmg|msi|exe)$"
)
LATEST_RE = re.compile(
    r"^anyCode_latest_(?P<arch>aarch64|x86_64|x64)\.(?P<ext>dmg|msi|exe)$"
)

ARCH_TO_PLATFORM = {
    ("aarch64", "dmg"): "macos-aarch64",
    ("x86_64", "dmg"): "macos-x86_64",
    ("x64", "msi"): "windows-x64",
    ("x64", "exe"): "windows-x64",
}


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def platform_for(arch: str, ext: str) -> str | None:
    return ARCH_TO_PLATFORM.get((arch, ext))


def version_key(v: str) -> tuple[int, ...]:
    return tuple(int(x) for x in v.split("."))


def main() -> int:
    if len(sys.argv) < 2:
        print("usage: regen-downloads-manifest.py <downloads_dir>", file=sys.stderr)
        return 2
    download_dir = Path(sys.argv[1])
    download_dir.mkdir(parents=True, exist_ok=True)

    artifacts: list[dict] = []
    checksum_lines: list[str] = []

    for path in sorted(download_dir.iterdir()):
        if not path.is_file():
            continue
        name = path.name
        if name in {"latest.json", "releases.json", "SHA256SUMS.txt", ".gitkeep"}:
            continue
        if name.startswith("."):
            continue

        m = VERSIONED_RE.match(name)
        if not m:
            # keep latest_* in checksums but not as historical artifacts
            if LATEST_RE.match(name):
                digest = sha256_file(path)
                checksum_lines.append(f"{digest}  {name}")
            continue

        arch = m.group("arch")
        ext = m.group("ext")
        version = m.group("version")
        platform = platform_for(arch, ext)
        if not platform:
            continue

        digest = sha256_file(path)
        checksum_lines.append(f"{digest}  {name}")
        latest_name = f"anyCode_latest_{arch}.{ext}"
        artifacts.append(
            {
                "platform": platform,
                "version": version,
                "arch": arch,
                "filename": name,
                "url": f"{BASE_URL}/{name}",
                "latest_url": f"{BASE_URL}/{latest_name}",
                "sha256": digest,
                "ext": ext,
            }
        )

    # Prefer .msi over .exe for the same version on windows when both exist
    by_key: dict[tuple[str, str], dict] = {}
    for art in artifacts:
        key = (art["platform"], art["version"])
        prev = by_key.get(key)
        if prev is None:
            by_key[key] = art
            continue
        if prev["ext"] == "exe" and art["ext"] == "msi":
            by_key[key] = art

    artifacts = sorted(
        by_key.values(),
        key=lambda a: (a["platform"], version_key(a["version"])),
        reverse=True,
    )

    latest_by_platform: dict[str, str] = {}
    for art in artifacts:
        plat = art["platform"]
        if plat not in latest_by_platform:
            latest_by_platform[plat] = art["version"]
        art["latest"] = art["version"] == latest_by_platform.get(plat)

    platforms: dict[str, dict] = {}
    for plat, ver in latest_by_platform.items():
        art = next(a for a in artifacts if a["platform"] == plat and a["version"] == ver)
        platforms[plat] = {
            "version": art["version"],
            "arch": art["arch"],
            "filename": art["filename"],
            "url": art["url"],
            "latest_url": art["latest_url"],
            "sha256": art["sha256"],
            "ext": art["ext"],
        }

    releases = {
        "generated_at": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "latest_by_platform": latest_by_platform,
        "platforms": platforms,
        "artifacts": artifacts,
    }
    (download_dir / "releases.json").write_text(
        json.dumps(releases, indent=2) + "\n", encoding="utf-8"
    )

    # Backward-compatible latest.json (prefer macos-aarch64, else first platform)
    primary = platforms.get("macos-aarch64") or next(iter(platforms.values()), None)
    if primary:
        latest_payload = {
            "version": primary["version"],
            "arch": primary["arch"],
            "filename": primary["filename"],
            "url": primary["url"],
            "latest_url": primary["latest_url"],
            "sha256": primary["sha256"],
            "platforms": platforms,
        }
        (download_dir / "latest.json").write_text(
            json.dumps(latest_payload, indent=2) + "\n", encoding="utf-8"
        )

    checksum_lines = sorted(set(checksum_lines))
    (download_dir / "SHA256SUMS.txt").write_text(
        "\n".join(checksum_lines) + ("\n" if checksum_lines else ""),
        encoding="utf-8",
    )

    print(f"wrote {download_dir / 'releases.json'} ({len(artifacts)} artifacts)")
    print(f"platforms: {', '.join(sorted(platforms)) or '(none)'}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
