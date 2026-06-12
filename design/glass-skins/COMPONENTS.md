# Component HTML Structure

Fixed class names — React migration must preserve these.

## App Shell

```
.dw-shell                    flex column, height 100vh, bg var(--bg)
  .dw-sidebar.glass-panel     fixed left, width var(--sidebar-width)
  .dw-main-wrap              margin-left sidebar, flex column, flex 1
    .dw-topbar                height var(--topbar-height), glass, drag region (Tauri)
    .dw-main                  flex 1, overflow auto, padding 20-24px
```

### Sidebar blocks
- `.dw-sidebar-brand` — logo + "anyCode"
- `.dw-nav` — nav list
- `.dw-nav-link` — item; add `.dw-nav-link--active` when active
- `.dw-sidebar-footer` — workspace card / version

### Topbar blocks
- `.dw-topbar-left` — optional search
- `.dw-topbar-right` — `.status-dot`, lang, theme toggle, notifications, avatar
- `.no-drag` on all buttons (Tauri)

## Home page

| Class | Content |
|-------|---------|
| `.dw-home-hero` | Centered hero, position relative |
| `.hero-glow` | Absolute radial gradient behind title |
| `.dw-home-hero h1` | "Build with your **agent**" — accent on last word via `.accent-text` |
| `.dw-composer` | Glass panel, rounded var(--radius-lg), textarea + toolbar |
| `.dw-composer-toolbar` | attach / @ / code icons + `.btn-accent` send |
| `.dw-suggestion-grid` | CSS grid 3 cols, gap 16px |
| `.glass-card` | Suggestion cards |
| `.dw-home-footer` | Version text, muted |

## Conversations page

Three columns inside `.dw-main`:

| Class | Width | Role |
|-------|-------|------|
| `.conv-sidebar` | 240px | Session list |
| `.conv-thread` | flex 1 | Messages |
| `.conv-artifacts` | 280px | Artifacts panel |

### Bubbles
- `.bubble-user` — right aligned, `background: var(--solid-bubble)`
- `.bubble-assistant.glass-panel` — left aligned, glass
- `.tool-strip` — collapsible tool summary
- `.code-block` — filename header + pre body

### Composer
- `.dw-composer.dw-composer--sticky` — sticky bottom in thread column

## Overview page

- `.kpi-grid` — 4 columns, `.kpi-card.glass-card`
- `.chart-card.glass-card` — SVG line chart (no ECharts in prototype)
- `.overview-side` — session list + approval inbox cards

## Settings skin picker

- `.skin-picker-grid` — 2×2 grid
- `.skin-preview-card.glass-card` — mini sidebar mock + accent dot
- `.skin-preview-card--selected` — `outline: 2px solid var(--accent)`

## Icon convention

Use inline SVG 20×20, stroke `currentColor`, stroke-width 1.5. Do **not** use Material Symbols in new prototypes.

Example nav icon placeholder:
```html
<svg class="dw-icon" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M3 9.5L12 3l9 6.5V20a1 1 0 01-1 1h-5v-7H9v7H4a1 1 0 01-1-1V9.5z"/></svg>
```

## React file mapping

| Prototype class | React file |
|-----------------|------------|
| `.dw-shell` | `Layout.tsx` |
| `.dw-composer` | `HomeHeroComposer.tsx`, `ConversationComposer.tsx` |
| `.bubble-*` | `ConversationTranscript.tsx` |
| `.kpi-card` | `OverviewPage.tsx`, `KpiMetricGrid.tsx` |
| `.skin-picker-grid` | new `SkinPicker.tsx` |
