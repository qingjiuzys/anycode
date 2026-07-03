//! Build YAML accessibility snapshot from page DOM.

pub fn snapshot_script(root_ref: Option<&str>) -> String {
    let root_literal = root_ref
        .map(|r| serde_json::to_string(r).unwrap_or_else(|_| "null".to_string()))
        .unwrap_or_else(|| "null".to_string());
    format!(
        r#"
(() => {{
  const MAX_DEPTH = 8;
  const MAX_NODES = 400;
  const rootRef = {root_literal};
  let count = 0;
  const lines = [];
  function roleOf(el) {{
    const explicit = el.getAttribute('role');
    if (explicit) return explicit;
    const tag = el.tagName.toLowerCase();
    if (tag === 'a') return 'link';
    if (tag === 'button') return 'button';
    if (tag === 'input') return 'input';
    if (tag === 'h1' || tag === 'h2' || tag === 'h3') return 'heading';
    if (tag === 'img') return 'image';
    return tag;
  }}
  function walk(el, depth, refMap) {{
    if (!el || depth > MAX_DEPTH || count >= MAX_NODES) return;
    if (el.nodeType !== Node.ELEMENT_NODE) return;
    const style = window.getComputedStyle(el);
    if (style.display === 'none' || style.visibility === 'hidden') return;
    count += 1;
    const ref = 'e' + count;
    refMap[ref] = el;
    const name = (el.getAttribute('aria-label')
      || el.getAttribute('alt')
      || el.getAttribute('title')
      || el.innerText
      || '').trim().replace(/\s+/g, ' ').slice(0, 120);
    const indent = '  '.repeat(depth);
    lines.push(`${{indent}}- ref=${{ref}} role=${{roleOf(el)}} name="${{name}}"`);
    for (const child of el.children) walk(child, depth + 1, refMap);
  }}
  const refMap = {{}};
  let root = document.body;
  if (rootRef && window.__anycodeBrowserRefs && window.__anycodeBrowserRefs[rootRef]) {{
    root = window.__anycodeBrowserRefs[rootRef];
    walk(root, 0, refMap);
  }} else if (rootRef) {{
    throw new Error('region ref not found: ' + rootRef);
  }} else {{
    walk(document.body, 0, refMap);
  }}
  window.__anycodeBrowserRefs = refMap;
  return {{
    title: document.title,
    url: location.href,
    yaml: lines.join('\n'),
  }};
}})()
"#
    )
}
