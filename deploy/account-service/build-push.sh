#!/usr/bin/env bash
# Build (linux/amd64) and push anycode account+portal image to Aliyun ACR.
# Secrets (DATABASE_URL, WECHAT_PAY_API_V3_KEY) are injected at deploy time via K8s Secret — never build-args.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
REGISTRY="${REGISTRY:-registry.cn-zhangjiakou.aliyuncs.com/818cloud/anycode}"
VERSION="$(grep '^version' "$ROOT/Cargo.toml" | head -1 | sed 's/.*"\(.*\)".*/\1/')"
TAG="${TAG:-$VERSION}"
GIT_SHA="$(git -C "$ROOT" rev-parse --short HEAD 2>/dev/null || echo unknown)"
IMMUTABLE_TAG="${TAG}-${GIT_SHA}"

# Ensure WeChat PEM files exist in build context (public keys only; API v3 key is runtime Secret)
"$ROOT/deploy/account-service/scripts/sync-wechat-certs.sh"

DOWNLOAD_DIR="$ROOT/crates/account-portal/public/downloads"
if ! compgen -G "$DOWNLOAD_DIR/anyCode_"*"_aarch64.dmg" >/dev/null 2>&1; then
  echo "No macOS DMG in $DOWNLOAD_DIR" >&2
  echo "Run: ./scripts/build-account-image.sh   (macOS, signed DMG + image)" >&2
  echo "  or: ./scripts/release-desktop-local.sh && $0" >&2
  exit 1
fi
echo "Desktop DMG baked into portal image:"
ls -lh "$DOWNLOAD_DIR"/*.dmg 2>/dev/null | sed 's/^/  /'

echo "Building $REGISTRY:$IMMUTABLE_TAG (linux/amd64, immutable tag + latest alias)"
# ACR: NEVER enable provenance/sbom — they push an OCI attestation index that
# shows as "正常" with empty 大小/镜像ID in the console and breaks pulls.
# Also: do NOT store WeChat PEMs under /run/secrets in the image — ACK/K8s
# volume mounts on /run/secrets hide baked-in certs and make wechat_pay_configured=false
# even when WECHAT_PAY_API_V3_KEY is set. Use /app/wechat-certs instead.
docker buildx build \
  --platform linux/amd64 \
  --provenance=false \
  --sbom=false \
  -f "$ROOT/deploy/account-service/Dockerfile" \
  -t "$REGISTRY:$IMMUTABLE_TAG" \
  -t "$REGISTRY:$TAG" \
  -t "$REGISTRY:latest" \
  --push \
  "$ROOT"

echo "Done: $REGISTRY:$IMMUTABLE_TAG (pushed)"
echo "Rollback: kubectl set image deployment/anycode-account anycode-account=$REGISTRY:<previous-tag>"
echo "Verify:   docker pull --platform linux/amd64 $REGISTRY:$IMMUTABLE_TAG"
# Fail closed if ACR got an empty attestation index again
if ! docker manifest inspect "$REGISTRY:$TAG" 2>/dev/null | grep -q '"layers"'; then
  echo "ERROR: $REGISTRY:$TAG has no layers (likely attestation index). Re-push with --provenance=false --sbom=false." >&2
  exit 1
fi
SIZE="$(docker manifest inspect "$REGISTRY:$TAG" 2>/dev/null | python3 -c 'import json,sys; m=json.load(sys.stdin); print(sum(l["size"] for l in m.get("layers",[])))' 2>/dev/null || echo 0)"
echo "Verified layers OK (approx uncompressed layer bytes: $SIZE)"
