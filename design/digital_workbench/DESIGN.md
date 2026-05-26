---
name: Digital Workbench
colors:
  surface: '#faf8ff'
  surface-dim: '#d2d9f4'
  surface-bright: '#faf8ff'
  surface-container-lowest: '#ffffff'
  surface-container-low: '#f2f3ff'
  surface-container: '#eaedff'
  surface-container-high: '#e2e7ff'
  surface-container-highest: '#dae2fd'
  on-surface: '#131b2e'
  on-surface-variant: '#434655'
  inverse-surface: '#283044'
  inverse-on-surface: '#eef0ff'
  outline: '#737686'
  outline-variant: '#c3c6d7'
  surface-tint: '#0053db'
  primary: '#004ac6'
  on-primary: '#ffffff'
  primary-container: '#2563eb'
  on-primary-container: '#eeefff'
  inverse-primary: '#b4c5ff'
  secondary: '#505f76'
  on-secondary: '#ffffff'
  secondary-container: '#d0e1fb'
  on-secondary-container: '#54647a'
  tertiary: '#943700'
  on-tertiary: '#ffffff'
  tertiary-container: '#bc4800'
  on-tertiary-container: '#ffede6'
  error: '#ba1a1a'
  on-error: '#ffffff'
  error-container: '#ffdad6'
  on-error-container: '#93000a'
  primary-fixed: '#dbe1ff'
  primary-fixed-dim: '#b4c5ff'
  on-primary-fixed: '#00174b'
  on-primary-fixed-variant: '#003ea8'
  secondary-fixed: '#d3e4fe'
  secondary-fixed-dim: '#b7c8e1'
  on-secondary-fixed: '#0b1c30'
  on-secondary-fixed-variant: '#38485d'
  tertiary-fixed: '#ffdbcd'
  tertiary-fixed-dim: '#ffb596'
  on-tertiary-fixed: '#360f00'
  on-tertiary-fixed-variant: '#7d2d00'
  background: '#faf8ff'
  on-background: '#131b2e'
  surface-variant: '#dae2fd'
typography:
  headline-lg:
    fontFamily: Inter
    fontSize: 24px
    fontWeight: '600'
    lineHeight: 32px
    letterSpacing: -0.02em
  headline-md:
    fontFamily: Inter
    fontSize: 18px
    fontWeight: '600'
    lineHeight: 28px
    letterSpacing: -0.01em
  body-md:
    fontFamily: Inter
    fontSize: 14px
    fontWeight: '400'
    lineHeight: 20px
  body-sm:
    fontFamily: Inter
    fontSize: 13px
    fontWeight: '400'
    lineHeight: 18px
  label-md:
    fontFamily: Inter
    fontSize: 12px
    fontWeight: '500'
    lineHeight: 16px
    letterSpacing: 0.01em
  code-sm:
    fontFamily: JetBrains Mono
    fontSize: 12px
    fontWeight: '400'
    lineHeight: 16px
rounded:
  sm: 0.125rem
  DEFAULT: 0.25rem
  md: 0.375rem
  lg: 0.5rem
  xl: 0.75rem
  full: 9999px
spacing:
  base: 4px
  xs: 4px
  sm: 8px
  md: 16px
  lg: 24px
  xl: 32px
  sidebar_width: 240px
  gutter: 16px
---

## Brand & Style
The design system is engineered for "Digital Workbench," a high-density console for developers and technical operators. The aesthetic follows a **Modern Corporate/Utility** style, drawing heavy inspiration from tools like Linear and Sentry.

The goal is to provide a "quiet" interface that recedes into the background, allowing complex data and code to remain the primary focus. It prioritizes clarity, information density, and functional hierarchy over decorative elements. Visual weight is managed through precise borders and subtle tonal shifts rather than shadows or vibrant gradients.

## Colors
The palette is rooted in a professional "Slate" scale to ensure high legibility and a calm working environment. 

- **Primary & Info**: Blue (#2563EB) is used for primary actions, active states, and informative highlights.
- **Success/Warning/Error**: Standard semantic colors are utilized for status indicators, ensuring immediate recognition of system health.
- **Neutral/Surface**: The background uses a cool-toned light gray (#F8FAFC) to create a subtle contrast with the white (#FFFFFF) component surfaces. Borders are kept thin and light (#E2E8F0) to define structure without adding visual clutter.

## Typography
The system utilizes **Inter** for all UI elements to provide a neutral, highly readable foundation. For Chinese characters, it falls back to **PingFang SC**. 

**JetBrains Mono** is reserved for technical data, including IDs, file paths, logs, and code snippets. Typography follows a high-density scale; the default body size is 14px, with 13px used for secondary information to maximize the data-to-pixel ratio.

## Layout & Spacing
This design system employs a **Fixed/Fluid Hybrid Grid**. The sidebar is fixed at 240px, while the main content area expands to fill the viewport.

- **Density**: A 4px base unit is used. High-density components (like tables and lists) utilize 8px (sm) or 12px padding.
- **Margins**: Main page containers use 24px (lg) margins on desktop, reducing to 16px (md) on smaller screens.
- **Alignment**: All elements align to a strict 4px grid to maintain visual rigor.

## Elevation & Depth
Depth is created through **Tonal Layers** and **Low-Contrast Outlines** rather than traditional shadows.

1.  **Level 0 (Background)**: #F8FAFC - The canvas.
2.  **Level 1 (Surface)**: #FFFFFF - Cards and main panels, defined by a 1px #E2E8F0 border.
3.  **Level 2 (Popovers/Modals)**: #FFFFFF - Uses a very subtle, diffused shadow (0 4px 12px rgba(0,0,0,0.05)) to separate floating elements from the surface.

Avoid using heavy drop shadows or neomorphic effects.

## Shapes
The shape language is **Soft** (4px default radius). This provides a hint of approachability while maintaining a precise, engineered look.

- **Components**: 4px (0.25rem) for buttons, inputs, and cards.
- **Status Badges**: Fully rounded (pill-shaped) to distinguish them from interactive buttons or cards.
- **Selection States**: 4px radius for active items in sidebars or dropdowns.

## Components

- **Sidebar**: 240px wide. Background #F8FAFC. Active states use a subtle #F1F5F9 background with a 2px blue vertical indicator on the far left or a subtle text color shift to Primary.
- **StatusBadge**: Pill-shaped. Uses a 10% opacity background of the semantic color (e.g., light green) with a solid colored dot and high-contrast text.
- **SectionCard**: White background, 1px #E2E8F0 border, 4px corner radius. No shadow.
- **PageHeader**: Headline-lg for titles. Includes a breadcrumb row above and an action row to the right. 
- **StatCard**: 1px border. Label in Secondary Text (label-md), Value in Primary Text (headline-md).
- **DataTable**: 1px horizontal borders only. No vertical lines. Row height 40px for high density. Header background #F8FAFC with semi-bold label-md text.
- **EventTimeline**: 2px wide vertical line in #E2E8F0. Severity is indicated by a colored ring on the node.
- **SseStatusBadge**: A 6px solid dot. Green (Connected), Amber (Reconnecting), Gray (Disconnected). Followed by label-md text.
- **GateStatusBar**: A segmented horizontal bar. Each segment represents a "Gate." Fill color based on gate status (Success/Error/Pending). Segments separated by 2px white gaps.
- **Buttons**:
    - *Primary*: Solid Blue, White text.
    - *Secondary*: White background, 1px Gray border, Dark Navy text.
    - *Ghost*: No background/border, Primary or Secondary text; appears on hover.