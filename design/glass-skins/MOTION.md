# Motion Specification

CSS-only animations. **No** framer-motion, no JS animation libraries.

## Durations

| Token | Value | Use |
|-------|-------|-----|
| `--motion-fast` | `150ms` | Focus rings, button hover |
| `--motion-normal` | `200ms` | Card hover lift |
| `--motion-slow` | `300ms` | Panel expand (optional) |

Easing: `ease` or `ease-out` only. No spring curves in CSS.

## Interactions

### Composer focus (`.dw-composer textarea`)
```css
box-shadow: 0 0 0 2px var(--accent);
transition: box-shadow 150ms ease;
```

### Suggestion / glass card hover (`.glass-card:not(.glass-card--static)`)
```css
transform: translateY(-2px);
border-color: var(--border-strong);
box-shadow: var(--shadow-float);
transition: transform 200ms ease, border-color 200ms ease, box-shadow 200ms ease;
```

### Nav active (`.dw-nav-link--active`)
- Background: `var(--accent-muted)`
- Optional left bar: `border-left: 2px solid var(--accent)` (mono skin: white)

### Typing indicator (`.typing-dots`)
Three dots, `pulse-dot` keyframes, stagger 0 / 150ms / 300ms.

### Skin swatch select (`.skin-swatch[aria-pressed="true"]`)
```css
box-shadow: 0 0 0 2px var(--bg), 0 0 0 4px var(--accent);
transform: scale(1.05);
```

## Reduced motion

```css
@media (prefers-reduced-motion: reduce) {
  *, *::before, *::after {
    animation-duration: 0.01ms !important;
    transition-duration: 0.01ms !important;
  }
}
```

Disable: card translateY, typing pulse, swatch scale.

## React migration (Phase 5)

Copy rules into `crates/dashboard-ui/src/index.css` `@layer components`. Do not add new npm deps.
