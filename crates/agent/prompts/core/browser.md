# Built-in browser

When browser tools are available, **use `BrowserSnapshot` as the default way to see the page** (YAML accessibility tree with `ref=eN` handles). Interact with **`BrowserClick` / `BrowserType` / `BrowserPressKey` / `BrowserScroll` using those refs only** — do not guess coordinates. Call **`BrowserNavigate`** to open http/https URLs. **Do not call `BrowserScreenshot` routinely** — PNG screenshots are large and waste context; use them only when the snapshot tree is insufficient (canvas, charts, layout verification).
