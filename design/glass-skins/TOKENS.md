# Glass Skins — Design Tokens (Source of Truth)

All HTML prototypes and React migration **must** use CSS variables from `shared/tokens.css`. Do not hardcode hex in component HTML except in this file and `tokens.css`.

## Skin IDs

| ID | Label (zh) | Label (en) |
|----|------------|------------|
| `mono` | 无彩色 | Monochrome |
| `indigo` | 电光蓝紫 | Indigo |
| `coral` | 珊瑚暖橙 | Coral |
| `teal` | 青绿 | Teal |

## Accent (per skin, mode-independent hue)

| Skin | `--accent` | `--accent-hover` | `--accent-muted` | `--accent-glow` |
|------|------------|------------------|------------------|-----------------|
| mono | `#ffffff` | `#e5e5e5` | `rgba(255,255,255,0.12)` | `rgba(255,255,255,0.04)` |
| indigo | `#6e6bff` | `#8b7cff` | `rgba(110,107,255,0.18)` | `rgba(110,107,255,0.18)` |
| coral | `#e8826b` | `#df8f7c` | `rgba(232,130,107,0.16)` | `rgba(232,130,107,0.16)` |
| teal | `#2dd4bf` | `#22d3ee` | `rgba(45,212,191,0.14)` | `rgba(45,212,191,0.14)` |

Light mode uses the **same accent hex**; glow opacity is halved in CSS.

## Surfaces (mode-dependent, all skins)

### Dark (`html.dark`)

| Token | Value |
|-------|-------|
| `--bg` | `#0a0a0c` |
| `--bg-elevated` | `#121214` |
| `--glass-bg` | `rgba(255,255,255,0.04)` |
| `--glass-bg-strong` | `rgba(255,255,255,0.08)` |
| `--on-surface` | `#f5f5f7` |
| `--on-surface-variant` | `#8e8e93` |
| `--border` | `rgba(255,255,255,0.08)` |
| `--border-strong` | `rgba(255,255,255,0.14)` |
| `--solid-bubble` | `#2c2c2e` |

### Light (`html:not(.dark)`)

| Token | Value |
|-------|-------|
| `--bg` | `#f7f7f8` |
| `--bg-elevated` | `#ffffff` |
| `--glass-bg` | `rgba(255,255,255,0.80)` |
| `--glass-bg-strong` | `rgba(255,255,255,0.92)` |
| `--on-surface` | `#1d1d1f` |
| `--on-surface-variant` | `#6e6e73` |
| `--border` | `rgba(0,0,0,0.08)` |
| `--border-strong` | `rgba(0,0,0,0.12)` |
| `--solid-bubble` | `#e8e8ed` |

## Semantic (shared across skins)

### Dark

| Token | Value |
|-------|-------|
| `--success` | `#4ade80` |
| `--warn` | `#facc15` |
| `--error` | `#f87171` |

### Light

| Token | Value |
|-------|-------|
| `--success` | `#16a34a` |
| `--warn` | `#ca8a04` |
| `--error` | `#dc2626` |

## Layout

| Token | Value |
|-------|-------|
| `--sidebar-width` | `240px` |
| `--topbar-height` | `44px` |
| `--radius-sm` | `8px` |
| `--radius-md` | `12px` |
| `--radius-lg` | `16px` |
| `--radius-full` | `9999px` |
| `--font-sans` | `"Inter", "PingFang SC", "Microsoft YaHei", system-ui, sans-serif` |
| `--font-mono` | `"JetBrains Mono", ui-monospace, monospace` |

## React mapping (Phase 1)

Map existing Tailwind `@theme` tokens:

- `--color-primary` → `var(--accent)`
- `--color-primary-container` → `var(--accent-hover)`
- `--color-background` / `--color-surface` → `var(--bg)`
- `--color-on-surface` → `var(--on-surface)`

Storage keys:

- Skin: `anycode-dashboard-skin` (`mono` | `indigo` | `coral` | `teal`)
- Theme: `anycode-dashboard-theme` (`light` | `dark`)
