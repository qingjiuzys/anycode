#!/usr/bin/env bash
# Copy bundled skills + default brand-kit into ~/.anycode/skills/
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${ANYCODE_SKILLS_DIR:-$HOME/.anycode/skills}"
SRC="$ROOT/skills-starter"
BRAND_ROOT="$ROOT/brand-kits"
DEFAULT_BRAND="${ANYCODE_DEFAULT_BRAND_KIT:-fde-editorial}"
mkdir -p "$DEST"

copy_brand_to_skill() {
  local skill_dir="$1"
  local brand_name="$2"
  local brand_src="$BRAND_ROOT/$brand_name"
  [[ -f "$brand_src/tokens.json" ]] || return 0
  mkdir -p "$skill_dir/brand/xlsx" "$skill_dir/brand/pptx" "$skill_dir/brand/docx"
  cp "$brand_src/tokens.json" "$skill_dir/brand/tokens.json"
  [[ -f "$brand_src/xlsx/theme.json" ]] && cp "$brand_src/xlsx/theme.json" "$skill_dir/brand/xlsx/theme.json"
  [[ -f "$brand_src/pptx/layouts.json" ]] && cp "$brand_src/pptx/layouts.json" "$skill_dir/brand/pptx/layouts.json"
  [[ -f "$brand_src/pptx/template.potx" ]] && cp "$brand_src/pptx/template.potx" "$skill_dir/brand/pptx/template.potx"
  [[ -f "$brand_src/docx/template.dotx" ]] && cp "$brand_src/docx/template.dotx" "$skill_dir/brand/docx/template.dotx"
}

for d in "$SRC"/*/; do
  id="$(basename "$d")"
  if [[ -f "$d/SKILL.md" ]]; then
    mkdir -p "$DEST/$id"
    cp "$d/SKILL.md" "$DEST/$id/SKILL.md"
    if [[ -f "$d/run" ]]; then
      cp "$d/run" "$DEST/$id/run"
      chmod +x "$DEST/$id/run"
    fi
    if [[ -d "$d/templates" ]]; then
      rm -rf "$DEST/$id/templates"
      cp -R "$d/templates" "$DEST/$id/templates"
    fi
    if [[ "$id" == *"-delivery" || "$id" == "office-pptx" || "$id" == "presentation-design" || "$id" == "anycode-ppt" || "$id" == "anycode-docx" || "$id" == "anycode-xlsx" || "$id" == "anycode-pdf" ]]; then
      copy_brand_to_skill "$DEST/$id" "$DEFAULT_BRAND"
    fi
    if [[ -d "$d/docs" ]]; then
      mkdir -p "$DEST/$id/docs"
      cp -R "$d/docs/"* "$DEST/$id/docs/" 2>/dev/null || true
    fi
    for doc in visual-format.md diagram-density.md components.md; do
      if [[ -f "$d/$doc" ]]; then
        cp "$d/$doc" "$DEST/$id/$doc"
      fi
    done
    if [[ -f "$SRC/_shared/deepseek-office.md" ]]; then
      mkdir -p "$DEST/_shared"
      cp "$SRC/_shared/deepseek-office.md" "$DEST/_shared/deepseek-office.md"
    fi
    echo "installed: $id -> $DEST/$id"
  fi
done
echo "Done. Enable skills.enabled in config and run: anycode skills list"
echo "Brand kits in repo: $(ls -1 "$BRAND_ROOT" | grep -v '^_' | tr '\n' ' ')"
echo "Set ANYCODE_BRAND_KITS_DIR=$BRAND_ROOT to resolve gov-formal / edu-clean / lingqi"
