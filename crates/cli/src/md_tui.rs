//! Render CommonMark / GFM subset to ratatui `Line`s (pre-wrapped to column width).

use crate::i18n::tr;
use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use unicode_width::UnicodeWidthChar;

/// 防止恶意超大 Markdown 拖垮终端。
const MAX_MD_OUTPUT_LINES: usize = 16_384;
const MAX_MD_TEXT_RUN: usize = 256_000;

/// 按字节上限截断 `str`，保证落在 UTF-8 字符边界（避免 `is_char_boundary` panic）。
fn truncate_str_at_char_boundary(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

/// 与 Claude Code 类似：在支持 OSC 8 的终端里可 Ctrl/⌘+点击打开链接。  
/// 设置 `ANYCODE_OSC8_LINKS=1`（或 `true`/`yes`）启用。
fn env_osc8_links() -> bool {
    match std::env::var("ANYCODE_OSC8_LINKS") {
        Ok(v) => {
            let v = v.trim();
            v == "1" || v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("yes")
        }
        Err(_) => false,
    }
}

/// OSC 8 超链接（ST `\x1b\\` 终止，兼容 iTerm2 / Kitty / WezTerm / Windows Terminal 等）。
fn osc8_hyperlink(url: &str, visible: &str) -> String {
    let u: String = url
        .chars()
        .filter(|c| *c != '\x07' && *c != '\x1b')
        .collect();
    let vis = if visible.trim().is_empty() {
        u.as_str()
    } else {
        visible
    };
    format!("\x1b]8;;{u}\x1b\\{vis}\x1b]8;;\x1b\\")
}

fn char_display_width(c: char) -> usize {
    UnicodeWidthChar::width(c)
        .unwrap_or(0)
        .max(if c.is_whitespace() { 1 } else { 0 })
        .max(1)
}

fn str_display_width(s: &str) -> usize {
    s.chars().map(char_display_width).sum()
}

/// 字符串在终端中的显示宽度（含宽字符，用于 Prompt / Plain 折行）。
pub fn text_display_width(s: &str) -> usize {
    str_display_width(s)
}

/// 将单行按显示宽度自动折行（无样式），用于 Prompt、Plain 块等。
pub fn wrap_string_to_width(text: &str, content_width: usize) -> Vec<String> {
    if content_width == 0 {
        return vec![text.to_string()];
    }
    let width = content_width.max(1);
    if text.is_empty() {
        return vec![String::new()];
    }
    let mut rows: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut cur_w = 0usize;
    for ch in text.chars() {
        let cw = char_display_width(ch);
        if cur_w + cw > width && !cur.is_empty() {
            rows.push(cur);
            cur = String::new();
            cur_w = 0;
        }
        if cw > width {
            if !cur.is_empty() {
                rows.push(cur);
                cur = String::new();
                cur_w = 0;
            }
            rows.push(ch.to_string());
            continue;
        }
        if cur_w + cw > width && cur.is_empty() {
            rows.push(ch.to_string());
            continue;
        }
        cur.push(ch);
        cur_w += cw;
    }
    if !cur.is_empty() || rows.is_empty() {
        rows.push(cur);
    }
    rows
}

/// 将 ratatui 单行（多为单色）按宽度拆成多条 `Line`。
pub fn wrap_ratatui_line(line: Line<'static>, content_width: usize) -> Vec<Line<'static>> {
    let w = content_width.max(8);
    let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
    let style = line.spans.first().map(|s| s.style).unwrap_or_default();
    if text.is_empty() {
        return vec![line];
    }
    wrap_string_to_width(&text, w)
        .into_iter()
        .map(|row| Line::from(Span::styled(row, style)))
        .collect()
}

fn style_heading(level: HeadingLevel) -> Style {
    let base = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    match level {
        HeadingLevel::H1 | HeadingLevel::H2 => base.fg(Color::LightCyan),
        _ => base,
    }
}

fn style_inline_code() -> Style {
    Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::DIM)
}

fn style_block_quote() -> Style {
    Style::default().fg(Color::DarkGray)
}

fn style_link() -> Style {
    Style::default()
        .fg(Color::LightBlue)
        .add_modifier(Modifier::UNDERLINED)
}

fn style_dim() -> Style {
    Style::default().fg(Color::DarkGray)
}

fn style_body() -> Style {
    Style::default().fg(Color::Green)
}

/// 在固定宽度内把带样式的文本写成多行 `Line`（按字符宽度，支持 CJK）。
struct WrapWriter {
    width: usize,
    /// 每行开头的固定前缀（已计入 line_width）
    line_prefix: Vec<Span<'static>>,
    prefix_w: usize,
    out: Vec<Line<'static>>,
    line: Vec<Span<'static>>,
    line_w: usize,
    lines_emitted: usize,
}

impl WrapWriter {
    fn new(width: usize, line_prefix: Vec<Span<'static>>) -> Self {
        let prefix_w: usize = line_prefix
            .iter()
            .map(|s| str_display_width(s.content.as_ref()))
            .sum();
        Self {
            width: width.max(8),
            line_prefix,
            prefix_w,
            out: Vec::new(),
            line: Vec::new(),
            line_w: 0,
            lines_emitted: 0,
        }
    }

    fn set_block_prefix_stack(&mut self, stack: &[String]) {
        self.line_prefix = physical_prefix_from_stack(stack);
        self.prefix_w = self
            .line_prefix
            .iter()
            .map(|s| str_display_width(s.content.as_ref()))
            .sum();
        self.start_physical_line();
    }

    fn at_limit(&self) -> bool {
        self.lines_emitted >= MAX_MD_OUTPUT_LINES
    }

    fn start_physical_line(&mut self) {
        self.line = self.line_prefix.clone();
        self.line_w = self.prefix_w;
    }

    fn flush_line(&mut self) {
        if self.line.is_empty() || self.line.iter().all(|s| s.content.is_empty()) {
            self.line.clear();
            self.line_w = self.prefix_w;
            return;
        }
        if self.at_limit() {
            return;
        }
        self.out.push(Line::from(std::mem::take(&mut self.line)));
        self.lines_emitted += 1;
        self.start_physical_line();
    }

    fn push_char_styled(&mut self, style: Style, ch: char) {
        if self.at_limit() {
            return;
        }
        let cw = char_display_width(ch);
        if self.line_w + cw > self.width && self.line_w > self.prefix_w {
            self.flush_line();
        }
        if self.line_w + cw > self.width {
            // 极窄终端：硬切
            self.flush_line();
        }
        let sch = ch.to_string();
        if let Some(last) = self.line.last_mut() {
            if last.style == style {
                last.content = format!("{}{}", last.content, sch).into();
                self.line_w += cw;
                return;
            }
        }
        self.line.push(Span::styled(sch, style));
        self.line_w += cw;
    }

    fn push_str_styled(&mut self, style: Style, text: &str) {
        if text.len() > MAX_MD_TEXT_RUN {
            let head = truncate_str_at_char_boundary(text, MAX_MD_TEXT_RUN);
            self.push_str_styled(style, head);
            self.push_str_styled(style_dim(), " …[truncated]");
            return;
        }
        for ch in text.chars() {
            if ch == '\n' {
                self.flush_line();
                continue;
            }
            if ch == '\r' {
                continue;
            }
            self.push_char_styled(style, ch);
        }
    }

    /// 推送含不可见转义序列的内容时，用 `visual_width` 参与折行宽度计算（如 OSC 8 链接）。
    fn push_span_visual(&mut self, cell_text: String, visual_width: usize, style: Style) {
        if self.at_limit() {
            return;
        }
        let vw = visual_width.max(1);
        if self.line_w + vw > self.width && self.line_w > self.prefix_w {
            self.flush_line();
        }
        if self.line_w + vw > self.width && vw > self.width.saturating_sub(self.prefix_w) {
            self.flush_line();
        }
        self.line.push(Span::styled(cell_text, style));
        self.line_w += vw;
    }

    fn finish(mut self) -> Vec<Line<'static>> {
        self.flush_line();
        self.out
    }
}

fn physical_prefix_from_stack(stack: &[String]) -> Vec<Span<'static>> {
    if stack.is_empty() {
        Vec::new()
    } else {
        let p = stack.join("");
        vec![Span::styled(p, style_dim())]
    }
}

struct InlineStack {
    bold: u8,
    italic: u8,
    strike: u8,
    body_style: Style,
}

impl InlineStack {
    fn new(body_style: Style) -> Self {
        Self {
            bold: 0,
            italic: 0,
            strike: 0,
            body_style,
        }
    }

    fn base_style(&self, in_block_quote: bool) -> Style {
        let mut s = if in_block_quote {
            style_block_quote()
        } else {
            self.body_style
        };
        if self.bold > 0 {
            s = s.add_modifier(Modifier::BOLD);
        }
        if self.italic > 0 {
            s = s.add_modifier(Modifier::ITALIC);
        }
        if self.strike > 0 {
            s = s.add_modifier(Modifier::CROSSED_OUT);
        }
        s
    }
}

/// 在「非行首」的 `###` 等 ATX 标题前插入换行，缓解模型把标题粘在上一句末尾的情况。
fn loosen_glued_markdown_headings(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let mut out = String::with_capacity(input.len().saturating_add(32));
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '#' && i > 0 {
            let prev = chars[i - 1];
            // 跳过 `###` 中第 2、3 个 `#`（仅处理「行首连续 # 段」的第一个 `#`）
            if prev != '\n' && prev != '\r' && prev != '#' {
                let mut j = i;
                while j < chars.len() && chars[j] == '#' {
                    j += 1;
                }
                let level = j - i;
                if (1..=6).contains(&level) && j < chars.len() {
                    let next = chars[j];
                    // 仅当 ATX 标题后紧跟空白时处理，避免误伤 #rgb / URL 片段等
                    if next.is_whitespace() {
                        out.push('\n');
                    }
                }
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// 将 Markdown 渲染为已换行的 `Line`；`body_style` 控制正文/列表等默认颜色（助手用绿，用户用青等）。
pub fn render_markdown_styled(
    md: &str,
    content_width: usize,
    body_style: Style,
) -> Vec<Line<'static>> {
    let md_norm = loosen_glued_markdown_headings(md);
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TASKLISTS);
    opts.insert(Options::ENABLE_GFM);

    let parser = Parser::new_ext(md_norm.as_str(), opts);
    let mut writer = WrapWriter::new(content_width, Vec::new());
    writer.start_physical_line();

    let mut inline = InlineStack::new(body_style);
    let mut list_stack: Vec<(bool, u64)> = Vec::new(); // (ordered, item index)
    let mut pending_link_url: Option<String> = None;
    let mut block_prefix_depth: Vec<String> = Vec::new();
    let mut in_code_block = false;
    let mut heading_level: Option<HeadingLevel> = None;
    let mut block_quote_depth: u32 = 0;
    let mut in_image: bool = false;
    let mut pending_image_url: Option<String> = None;
    let mut image_alt: String = String::new();
    let mut table_rows: Vec<Vec<String>> = Vec::new();
    let mut table_row_cells: Vec<String> = Vec::new();
    let mut table_cell_buf: String = String::new();
    let mut in_table: bool = false;
    let mut in_table_cell: bool = false;
    let use_osc8 = env_osc8_links();
    let mut link_plain: Option<String> = None;

    for event in parser {
        if writer.at_limit() {
            break;
        }
        match event {
            Event::Start(Tag::Paragraph) => {
                if !writer.line.is_empty() && writer.line_w > writer.prefix_w {
                    writer.flush_line();
                }
            }
            Event::End(TagEnd::Paragraph) => {
                writer.flush_line();
            }
            Event::Start(Tag::Heading { level, .. }) => {
                writer.flush_line();
                // 与 Claude 网页一致：标题仅样式化，不保留 ATX `#` 前缀（终端里更易读）。
                inline = InlineStack::new(body_style);
                heading_level = Some(level);
            }
            Event::End(TagEnd::Heading(_)) => {
                writer.flush_line();
                heading_level = None;
            }
            Event::Start(Tag::BlockQuote(_)) => {
                writer.flush_line();
                block_quote_depth = block_quote_depth.saturating_add(1);
                block_prefix_depth.push("│ ".to_string());
                writer.set_block_prefix_stack(&block_prefix_depth);
            }
            Event::End(TagEnd::BlockQuote(_)) => {
                writer.flush_line();
                block_quote_depth = block_quote_depth.saturating_sub(1);
                let _ = block_prefix_depth.pop();
                writer.set_block_prefix_stack(&block_prefix_depth);
            }
            Event::Start(Tag::List(first)) => {
                writer.flush_line();
                list_stack.push((first.is_some(), first.unwrap_or(1)));
            }
            Event::End(TagEnd::List(_)) => {
                writer.flush_line();
                let _ = list_stack.pop();
            }
            Event::Start(Tag::Item) => {
                writer.flush_line();
                let prefix = if let Some((ordered, n)) = list_stack.last_mut() {
                    if *ordered {
                        let p = format!("{}. ", n);
                        *n = n.saturating_add(1);
                        p
                    } else {
                        "· ".to_string()
                    }
                } else {
                    "· ".to_string()
                };
                block_prefix_depth.push(prefix);
                writer.set_block_prefix_stack(&block_prefix_depth);
            }
            Event::End(TagEnd::Item) => {
                writer.flush_line();
                let _ = block_prefix_depth.pop();
                writer.set_block_prefix_stack(&block_prefix_depth);
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                writer.flush_line();
                in_code_block = true;
                let lang = match kind {
                    CodeBlockKind::Fenced(l) => l.to_string(),
                    CodeBlockKind::Indented => String::new(),
                };
                let mut fence = writer.line_prefix.clone();
                fence.push(Span::styled(
                    if lang.is_empty() {
                        "──── code ────".to_string()
                    } else {
                        format!("──── {lang} ────")
                    },
                    style_inline_code().add_modifier(Modifier::DIM),
                ));
                if !writer.at_limit() {
                    writer.out.push(Line::from(fence));
                    writer.lines_emitted += 1;
                }
                block_prefix_depth.push("  ".to_string());
                writer.set_block_prefix_stack(&block_prefix_depth);
            }
            Event::End(TagEnd::CodeBlock) => {
                writer.flush_line();
                let _ = block_prefix_depth.pop();
                writer.set_block_prefix_stack(&block_prefix_depth);
                in_code_block = false;
            }
            Event::Start(Tag::Strong) => {
                inline.bold = inline.bold.saturating_add(1);
            }
            Event::End(TagEnd::Strong) => {
                inline.bold = inline.bold.saturating_sub(1);
            }
            Event::Start(Tag::Emphasis) => {
                inline.italic = inline.italic.saturating_add(1);
            }
            Event::End(TagEnd::Emphasis) => {
                inline.italic = inline.italic.saturating_sub(1);
            }
            Event::Start(Tag::Strikethrough) => {
                inline.strike = inline.strike.saturating_add(1);
            }
            Event::End(TagEnd::Strikethrough) => {
                inline.strike = inline.strike.saturating_sub(1);
            }
            Event::Start(Tag::Link { dest_url, .. }) => {
                pending_link_url = Some(dest_url.to_string());
                if use_osc8 {
                    link_plain = Some(String::new());
                }
            }
            Event::End(TagEnd::Link) => {
                if let Some(url) = pending_link_url.take() {
                    if use_osc8 {
                        let vis = link_plain.take().unwrap_or_default();
                        let vis_trim = vis.trim();
                        let display = if vis_trim.is_empty() {
                            url.as_str()
                        } else {
                            vis_trim
                        };
                        let vw = str_display_width(display);
                        let cell = osc8_hyperlink(&url, display);
                        writer.push_span_visual(cell, vw, style_link());
                    } else {
                        link_plain = None;
                        let st = style_dim();
                        writer.push_str_styled(st, " ");
                        writer.push_str_styled(st, &format!("({url})"));
                    }
                }
            }
            Event::Code(code) => {
                if in_table && in_table_cell {
                    table_cell_buf.push('`');
                    table_cell_buf.push_str(code.as_ref());
                    table_cell_buf.push('`');
                    continue;
                }
                if use_osc8
                    && pending_link_url.is_some()
                    && link_plain.is_some()
                    && !in_code_block
                    && heading_level.is_none()
                {
                    if let Some(ref mut b) = link_plain {
                        b.push('`');
                        b.push_str(code.as_ref());
                        b.push('`');
                    }
                    continue;
                }
                let st = if in_code_block {
                    style_inline_code()
                } else {
                    style_inline_code().add_modifier(Modifier::BOLD)
                };
                writer.push_str_styled(st, code.as_ref());
            }
            Event::Text(t) => {
                if in_image {
                    image_alt.push_str(t.as_ref());
                    continue;
                }
                if in_table && in_table_cell {
                    table_cell_buf.push_str(t.as_ref());
                    continue;
                }
                if use_osc8
                    && pending_link_url.is_some()
                    && link_plain.is_some()
                    && !in_code_block
                    && heading_level.is_none()
                {
                    if let Some(ref mut b) = link_plain {
                        b.push_str(t.as_ref());
                    }
                    continue;
                }
                let st = if in_code_block {
                    style_inline_code()
                } else if let Some(level) = heading_level {
                    style_heading(level)
                } else if pending_link_url.is_some() {
                    style_link()
                } else {
                    inline.base_style(block_quote_depth > 0)
                };
                writer.push_str_styled(st, t.as_ref());
            }
            Event::SoftBreak | Event::HardBreak => {
                if in_table && in_table_cell {
                    table_cell_buf.push(' ');
                } else if use_osc8
                    && pending_link_url.is_some()
                    && link_plain.is_some()
                    && !in_code_block
                    && heading_level.is_none()
                {
                    if let Some(ref mut b) = link_plain {
                        b.push(' ');
                    }
                } else {
                    writer.flush_line();
                }
            }
            Event::Rule => {
                writer.flush_line();
                let mut spans = writer.line_prefix.clone();
                let dash_w = writer.width.saturating_sub(writer.prefix_w).min(48).max(4);
                spans.push(Span::styled(
                    "─".repeat(dash_w),
                    style_dim().add_modifier(Modifier::DIM),
                ));
                if !writer.at_limit() {
                    writer.out.push(Line::from(spans));
                    writer.lines_emitted += 1;
                }
            }
            Event::TaskListMarker(done) => {
                let mark = if done { "[x] " } else { "[ ] " };
                if in_table && in_table_cell {
                    table_cell_buf.push_str(mark);
                } else {
                    writer.push_str_styled(body_style, mark);
                }
            }
            Event::Start(Tag::Table(_aligns)) => {
                writer.flush_line();
                in_table = true;
                table_rows.clear();
            }
            Event::End(TagEnd::Table) => {
                for (ri, row) in table_rows.iter().enumerate() {
                    let label = if ri == 0 { "┌" } else { "│" };
                    let joined = row.join(" │ ");
                    let mut spans = writer.line_prefix.clone();
                    spans.push(Span::styled(
                        format!("{label} {joined}"),
                        if ri == 0 {
                            body_style.add_modifier(Modifier::BOLD)
                        } else {
                            body_style
                        },
                    ));
                    if !writer.at_limit() {
                        writer.out.push(Line::from(spans));
                        writer.lines_emitted += 1;
                    }
                }
                if !table_rows.is_empty() && !writer.at_limit() {
                    let mut spans = writer.line_prefix.clone();
                    spans.push(Span::styled("└ (table)", style_dim()));
                    writer.out.push(Line::from(spans));
                    writer.lines_emitted += 1;
                }
                table_rows.clear();
                in_table = false;
                writer.flush_line();
            }
            Event::Start(Tag::TableHead) => {}
            Event::End(TagEnd::TableHead) => {}
            Event::Start(Tag::TableRow) => {
                table_row_cells.clear();
            }
            Event::End(TagEnd::TableRow) => {
                if in_table {
                    table_rows.push(table_row_cells.clone());
                }
            }
            Event::Start(Tag::TableCell) => {
                in_table_cell = true;
                table_cell_buf.clear();
            }
            Event::End(TagEnd::TableCell) => {
                in_table_cell = false;
                table_row_cells.push(table_cell_buf.trim().to_string());
                table_cell_buf.clear();
            }
            Event::Html(html) | Event::InlineHtml(html) => {
                writer.push_str_styled(style_dim(), html.as_ref());
            }
            Event::FootnoteReference(l) => {
                writer.push_str_styled(style_dim(), &format!("[^{}]", l.as_ref()));
            }
            Event::Start(Tag::Image { dest_url, .. }) => {
                in_image = true;
                pending_image_url = Some(dest_url.to_string());
                image_alt.clear();
            }
            Event::End(TagEnd::Image) => {
                let url = pending_image_url.take().unwrap_or_default();
                let alt = std::mem::take(&mut image_alt);
                in_image = false;
                let cap = if alt.is_empty() {
                    format!("![]({url})")
                } else {
                    format!("![{alt}]({url})")
                };
                writer.push_str_styled(style_dim(), &cap);
            }
            Event::Start(Tag::FootnoteDefinition(_)) => {
                writer.flush_line();
                writer.push_str_styled(style_dim(), "[footnote] ");
            }
            Event::End(TagEnd::FootnoteDefinition) => {
                writer.flush_line();
            }
            Event::InlineMath(m) | Event::DisplayMath(m) => {
                writer.push_str_styled(style_dim(), &format!("`${}`", m.as_ref()));
            }
            Event::Start(Tag::MetadataBlock(_)) => {}
            Event::End(TagEnd::MetadataBlock(_)) => {
                writer.flush_line();
            }
            Event::Start(
                Tag::DefinitionList | Tag::DefinitionListTitle | Tag::DefinitionListDefinition,
            ) => {
                writer.flush_line();
            }
            Event::End(
                TagEnd::DefinitionList
                | TagEnd::DefinitionListTitle
                | TagEnd::DefinitionListDefinition,
            ) => {
                writer.flush_line();
            }
            Event::Start(Tag::Superscript | Tag::Subscript) => {}
            Event::End(TagEnd::Superscript | TagEnd::Subscript) => {}
            _ => {}
        }
    }

    let mut lines = writer.finish();
    if lines.len() >= MAX_MD_OUTPUT_LINES {
        lines.push(Line::from(Span::styled(
            tr("tui-md-truncated"),
            style_dim(),
        )));
    }
    lines
}

/// 助手气泡等：默认绿色正文样式。
pub fn render_markdown(md: &str, content_width: usize) -> Vec<Line<'static>> {
    render_markdown_styled(md, content_width, style_body())
}

/// 将普通文本按宽度换行成绿色正文行（用于工具结果等）。
pub fn wrap_plain_prefixed(
    prefix: &str,
    text: &str,
    style: Style,
    content_width: usize,
) -> Vec<Line<'static>> {
    let pfx = vec![Span::styled(prefix.to_string(), style)];
    let mut w = WrapWriter::new(content_width, pfx);
    w.start_physical_line();
    w.push_str_styled(style, text.trim_end());
    w.finish()
}

/// 首行仅 `bullet` 使用 `bullet_style`，正文用 `text_style`；续行左侧用空格对齐到 `bullet` 显示宽度（用于 ⏺ 单独呼吸闪烁）。
pub fn wrap_plain_bullet_prefixed(
    bullet: &str,
    bullet_style: Style,
    text: &str,
    text_style: Style,
    content_width: usize,
) -> Vec<Line<'static>> {
    let t = text.trim_end();
    let bw = str_display_width(bullet).max(1);
    let text_w = content_width.saturating_sub(bw).max(4);
    if t.is_empty() {
        return vec![Line::from(Span::styled(bullet.to_string(), bullet_style))];
    }
    let rows = wrap_string_to_width(t, text_w);
    let pad = " ".repeat(bw);
    rows.into_iter()
        .enumerate()
        .map(|(i, row)| {
            if i == 0 {
                Line::from(vec![
                    Span::styled(bullet.to_string(), bullet_style),
                    Span::styled(row, text_style),
                ])
            } else {
                Line::from(vec![
                    Span::styled(pad.clone(), text_style),
                    Span::styled(row, text_style),
                ])
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_heading_and_list() {
        let md = "# Title\n\n- a\n- b\n";
        let lines = render_markdown(md, 40);
        assert!(!lines.is_empty());
        let s: String = lines[0]
            .spans
            .iter()
            .map(|sp| sp.content.as_ref())
            .collect();
        assert!(s.contains("Title"));
        assert!(!s.contains("# "));
    }

    #[test]
    fn wrap_ascii_splits_rows() {
        let rows = wrap_string_to_width("abcdefghij", 4);
        assert_eq!(rows, vec!["abcd", "efgh", "ij"]);
    }

    #[test]
    fn wrap_bullet_prefixed_splits_text_only() {
        let lines = wrap_plain_bullet_prefixed(
            "> ",
            Style::default(),
            "abcdefgh",
            Style::default().add_modifier(Modifier::BOLD),
            6,
        );
        assert!(lines.len() >= 2);
        let row0: String = lines[0].spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(row0.starts_with("> "));
    }

    #[test]
    fn truncate_does_not_split_utf8() {
        let s = "αβγδ"; // 2 bytes per Greek letter
        let t = truncate_str_at_char_boundary(s, 3);
        assert!(t.len() <= 3);
        assert!(s.is_char_boundary(t.len()));
        assert_eq!(t, "α");
    }

    #[test]
    fn loosen_inserts_newline_before_glued_heading() {
        let s = loosen_glued_markdown_headings("分析### 小标题");
        assert!(s.contains('\n'));
        assert!(s.contains("分析\n###"));
    }

    #[test]
    fn osc8_contains_url_and_label() {
        let s = osc8_hyperlink("https://ex.test", "go");
        assert!(s.contains("https://ex.test"));
        assert!(s.contains("go"));
        assert!(s.starts_with('\x1b'));
    }
}
