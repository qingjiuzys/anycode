//! Render CommonMark / GFM subset to ratatui `Line`s (pre-wrapped to column width).
//! Fenced / indented 代码块经 syntect 着色；过长块回退为单色 `style_inline_code`。

use crate::i18n::tr;
use crate::tui::palette;
use crate::tui::styles::style_dim;
use lru::LruCache;
use once_cell::sync::Lazy;
use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::num::NonZeroUsize;
use std::sync::Mutex;
use unicode_width::UnicodeWidthChar;

// 语法高亮相关
use syntect::easy::HighlightLines;
use syntect::highlighting::{Theme, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

/// Markdown 解析结果缓存
static MD_CACHE: Lazy<Mutex<LruCache<String, Vec<Line<'static>>>>> =
    Lazy::new(|| Mutex::new(LruCache::new(NonZeroUsize::new(100).unwrap()))); // 缓存 100 条消息

/// OSC 8 链接缓存（缓存最近生成的 200 个链接）
static OSC8_LINK_CACHE: Lazy<Mutex<LruCache<u64, String>>> =
    Lazy::new(|| Mutex::new(LruCache::new(NonZeroUsize::new(200).unwrap())));

/// 语法高亮主题（使用内置的 base16-ocean.dark 主题）
static SYNTAX_THEME: Lazy<Theme> = Lazy::new(|| {
    let theme_set = ThemeSet::load_defaults();
    theme_set.themes["base16-ocean.dark"].clone()
});

/// 代码块高亮缓存
struct CodeHighlightCache {
    cache: LruCache<String, Vec<Span<'static>>>,
    syntax_set: Option<SyntaxSet>,
}

fn cache_key_for_code_block(lang: &str, code: &str) -> String {
    // 短块直接键入全文，避免哈希碰撞；长块用长度 + 内容哈希。
    const INLINE_CAP: usize = 512;
    if code.len() <= INLINE_CAP {
        return format!("{lang}:{code}");
    }
    let mut h = DefaultHasher::new();
    code.hash(&mut h);
    format!("{lang}:{}:{:016x}", code.len(), h.finish())
}

impl CodeHighlightCache {
    fn new() -> Self {
        Self {
            cache: LruCache::new(NonZeroUsize::new(50).unwrap()), // 缓存 50 个代码块
            syntax_set: None,
        }
    }

    fn highlight_code(&mut self, code: &str, lang: &str) -> Vec<Span<'static>> {
        const MAX_HIGHLIGHT_BYTES: usize = 10_000;
        if code.len() > MAX_HIGHLIGHT_BYTES {
            return vec![Span::styled(code.to_string(), style_inline_code())];
        }

        let cache_key = cache_key_for_code_block(lang, code);

        // 尝试从缓存获取
        if let Some(cached) = self.cache.get(&cache_key) {
            return cached.clone();
        }

        // 延迟加载语法集合
        if self.syntax_set.is_none() {
            self.syntax_set = Some(SyntaxSet::load_defaults_newlines());
        }

        let syntax_set = self.syntax_set.as_ref().unwrap();

        // 查找语法
        let syntax = syntax_set
            .find_syntax_by_token(lang)
            .unwrap_or_else(|| syntax_set.find_syntax_plain_text());

        // 高亮代码
        let mut h = HighlightLines::new(syntax, &SYNTAX_THEME);
        let mut spans = Vec::new();

        for line in LinesWithEndings::from(code) {
            let ranges = h.highlight_line(line, syntax_set).unwrap_or_default();
            for (style, text) in ranges {
                let fg = Color::Rgb(style.foreground.r, style.foreground.g, style.foreground.b);
                let span = Span::styled(text.to_string(), Style::default().fg(fg));
                spans.push(span);
            }
        }

        // 保存到缓存
        self.cache.put(cache_key, spans.clone());
        spans
    }
}

/// 全局代码高亮缓存
static CODE_HIGHLIGHT_CACHE: Lazy<Mutex<CodeHighlightCache>> =
    Lazy::new(|| Mutex::new(CodeHighlightCache::new()));

fn flush_code_block_to_writer(writer: &mut WrapWriter, code: &str, lang: &str) {
    let spans = CODE_HIGHLIGHT_CACHE
        .lock()
        .unwrap()
        .highlight_code(code, lang);
    writer.push_spans_wrapping(&spans);
}

/// 防止恶意超大 Markdown 拖垮终端。
const MAX_MD_OUTPUT_LINES: usize = 16_384;
const MAX_MD_TEXT_RUN: usize = 256_000;

/// 带 Markdown 缓存的渲染函数（仅用于大于 1KB 的消息）
fn render_markdown_cached(md: &str, width: usize, enable_osc8: bool) -> Option<Vec<Line<'static>>> {
    // 只缓存较大的内容
    if md.len() < 1024 {
        return None;
    }

    let cache_key = format!(
        "{}|{}|{}|{}",
        md.len(),
        width,
        enable_osc8,
        palette::PALETTE_CACHE_VERSION
    );

    // 尝试从缓存获取（使用长度作为键避免复制大字符串）
    {
        let mut cache = MD_CACHE.lock().unwrap();
        if let Some(cached) = cache.get(&cache_key) {
            return Some(cached.clone());
        }
    }

    None
}

/// 保存 Markdown 渲染结果到缓存
fn cache_markdown_result(md: &str, width: usize, enable_osc8: bool, lines: Vec<Line<'static>>) {
    if md.len() < 1024 {
        return;
    }

    let cache_key = format!(
        "{}|{}|{}|{}",
        md.len(),
        width,
        enable_osc8,
        palette::PALETTE_CACHE_VERSION
    );
    MD_CACHE.lock().unwrap().put(cache_key, lines);
}

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
    // 生成缓存键
    let mut hasher = DefaultHasher::new();
    url.hash(&mut hasher);
    visible.hash(&mut hasher);
    let cache_key = hasher.finish();

    // 尝试从缓存获取
    {
        let mut cache = OSC8_LINK_CACHE.lock().unwrap();
        if let Some(cached) = cache.get(&cache_key) {
            return cached.clone();
        }
    }

    // 缓存未命中，生成链接
    let u: String = url
        .chars()
        .filter(|c| *c != '\x07' && *c != '\x1b')
        .collect();
    let vis = if visible.trim().is_empty() {
        u.as_str()
    } else {
        visible
    };
    let result = format!("\x1b]8;;{u}\x1b\\{vis}\x1b]8;;\x1b\\");

    // 保存到缓存
    OSC8_LINK_CACHE
        .lock()
        .unwrap()
        .put(cache_key, result.clone());

    result
}

/// ASCII 字符宽度缓存（预填充常用字符）
static WIDTH_CACHE: Lazy<[usize; 128]> = Lazy::new(|| {
    let mut cache = [0usize; 128];
    for (i, slot) in cache.iter_mut().enumerate() {
        let c = i as u8 as char;
        *slot = UnicodeWidthChar::width(c)
            .unwrap_or(0)
            .max(if c.is_whitespace() { 1 } else { 0 })
            .max(1);
    }
    cache
});

fn char_display_width(c: char) -> usize {
    // ASCII 字符使用缓存
    if c.is_ascii() {
        WIDTH_CACHE[c as usize]
    } else {
        UnicodeWidthChar::width(c)
            .unwrap_or(0)
            .max(if c.is_whitespace() { 1 } else { 0 })
            .max(1)
    }
}

fn str_display_width(s: &str) -> usize {
    s.chars().map(char_display_width).sum()
}

/// 字符串在终端中的显示宽度（含宽字符，用于 Prompt / Plain 折行）。
pub fn text_display_width(s: &str) -> usize {
    str_display_width(s)
}

/// 右侧用 ASCII 空格补齐到目标**显示宽度**（用于表格列对齐，如斜杠补全「命令 | 说明」）。
pub fn pad_end_to_display_width(s: &str, target_cols: usize) -> String {
    let w = text_display_width(s);
    if w >= target_cols {
        s.to_string()
    } else {
        format!("{}{}", s, " ".repeat(target_cols - w))
    }
}

/// 截断到不超过 `max_cols` 显示列；过长时在末尾保留一列给 `…`（若宽度允许）。
pub fn truncate_to_display_width(s: &str, max_cols: usize) -> String {
    if text_display_width(s) <= max_cols {
        return s.to_string();
    }
    if max_cols == 0 {
        return String::new();
    }
    if max_cols == 1 {
        return "…".to_string();
    }
    let budget = max_cols.saturating_sub(1);
    let mut out = String::new();
    let mut w = 0usize;
    for ch in s.chars() {
        let cw = char_display_width(ch);
        if w + cw > budget {
            break;
        }
        out.push(ch);
        w += cw;
    }
    if out.is_empty() {
        "…".to_string()
    } else {
        out.push('…');
        out
    }
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
    match level {
        HeadingLevel::H1 => Style::default()
            .fg(palette::accent()) // 橙色 H1
            .add_modifier(Modifier::BOLD), // 移除下划线，更简洁
        HeadingLevel::H2 => Style::default()
            .fg(palette::assistant_label()) // 淡紫色 H2
            .add_modifier(Modifier::BOLD),
        HeadingLevel::H3 => Style::default()
            .fg(palette::secondary()) // 紫色 H3
            .add_modifier(Modifier::BOLD),
        _ => Style::default()
            .fg(palette::text()) // 其他标题白色
            .add_modifier(Modifier::BOLD),
    }
}

fn style_inline_code() -> Style {
    Style::default()
        .fg(palette::accent()) // 使用橙色而非黄色，更符合Claude风格
        .add_modifier(Modifier::BOLD) // 改为BOLD而非DIM，更易读
}

fn style_block_quote() -> Style {
    Style::default()
        .fg(palette::blockquote_text()) // 灰紫色
        .add_modifier(Modifier::ITALIC) // 添加斜体使其更突出
}

fn style_link() -> Style {
    Style::default()
        .fg(palette::link()) // 紫色链接
        .add_modifier(Modifier::BOLD) // 改为BOLD更明显
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

    /// 将已着色的 `Span` 序列按显示宽度折行写入（fenced 代码块 syntect 输出）。
    fn push_spans_wrapping(&mut self, spans: &[Span<'static>]) {
        for sp in spans {
            let st = sp.style;
            for ch in sp.content.chars() {
                if ch == '\n' {
                    self.flush_line();
                } else if ch == '\r' {
                    continue;
                } else {
                    self.push_char_styled(st, ch);
                }
            }
        }
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
        stack
            .iter()
            .map(|piece| {
                let st = if piece.starts_with('│') {
                    Style::default().fg(palette::blockquote_rule())
                } else if piece.starts_with('·') {
                    // 无序列表使用淡紫色
                    Style::default()
                        .fg(palette::list_bullet())
                        .add_modifier(Modifier::BOLD)
                } else if piece.chars().next().is_some_and(|c| c.is_ascii_digit()) {
                    // 有序列表使用紫色
                    Style::default()
                        .fg(palette::secondary())
                        .add_modifier(Modifier::BOLD)
                } else {
                    style_dim()
                };
                Span::styled(piece.clone(), st)
            })
            .collect()
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

/// Markdown 装饰横线控制（流式 Inline 主区可关闭，避免与底栏 `─` 叠成满屏格线）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MarkdownChrome {
    pub suppress_horizontal_rules: bool,
    pub suppress_code_fence_banner: bool,
}

impl MarkdownChrome {
    fn is_default(&self) -> bool {
        !self.suppress_horizontal_rules && !self.suppress_code_fence_banner
    }
}

/// 将 Markdown 渲染为已换行的 `Line`；`body_style` 控制正文/列表等默认颜色（助手用绿，用户用青等）。
pub fn render_markdown_styled(
    md: &str,
    content_width: usize,
    body_style: Style,
    chrome: MarkdownChrome,
) -> Vec<Line<'static>> {
    let use_osc8 = env_osc8_links();

    // 尝试从缓存获取（仅对较大的内容）；非默认 chrome 不参与缓存以免串结果。
    if chrome.is_default() {
        if let Some(cached) = render_markdown_cached(md, content_width, use_osc8) {
            return cached;
        }
    }

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
    let mut link_plain: Option<String> = None;
    let mut code_block_buf = String::new();
    let mut code_block_lang = String::new();

    for event in parser {
        if writer.at_limit() {
            break;
        }
        match event {
            Event::Start(Tag::Paragraph)
                if !writer.line.is_empty() && writer.line_w > writer.prefix_w =>
            {
                writer.flush_line();
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
                code_block_lang = match kind {
                    CodeBlockKind::Fenced(l) => l.to_string(),
                    CodeBlockKind::Indented => String::new(),
                };
                code_block_buf.clear();
                if !chrome.suppress_code_fence_banner {
                    let lang = code_block_lang.as_str();
                    let mut fence = writer.line_prefix.clone();
                    fence.push(Span::styled(
                        if lang.is_empty() {
                            "──── code ────".to_string()
                        } else {
                            format!("──── {lang} ────")
                        },
                        Style::default()
                            .fg(palette::accent()) // 使用橙色使代码块更突出
                            .add_modifier(Modifier::BOLD), // 使用BOLD而非DIM
                    ));
                    if !writer.at_limit() {
                        writer.out.push(Line::from(fence));
                        writer.lines_emitted += 1;
                    }
                }
                block_prefix_depth.push("  ".to_string());
                writer.set_block_prefix_stack(&block_prefix_depth);
            }
            Event::End(TagEnd::CodeBlock) => {
                flush_code_block_to_writer(&mut writer, &code_block_buf, code_block_lang.as_str());
                writer.flush_line();
                let _ = block_prefix_depth.pop();
                writer.set_block_prefix_stack(&block_prefix_depth);
                in_code_block = false;
                code_block_buf.clear();
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
                if in_code_block {
                    code_block_buf.push_str(code.as_ref());
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
                if in_code_block {
                    code_block_buf.push_str(t.as_ref());
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
                } else if in_code_block {
                    code_block_buf.push('\n');
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
                if !chrome.suppress_horizontal_rules {
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
            }
            Event::TaskListMarker(done) => {
                // 避免 ASCII `[ ]`，减少与终端边框/表格线混成「左右括号」感。
                let mark = if done { "☑ " } else { "☐ " };
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
            Event::End(TagEnd::TableRow) if in_table => {
                table_rows.push(table_row_cells.clone());
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
                if in_code_block {
                    code_block_buf.push_str(html.as_ref());
                } else {
                    writer.push_str_styled(style_dim(), html.as_ref());
                }
            }
            Event::FootnoteReference(l) => {
                if in_code_block {
                    code_block_buf.push_str(&format!("†{}", l.as_ref()));
                } else {
                    writer.push_str_styled(style_dim(), &format!("†{}", l.as_ref()));
                }
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
                if in_code_block {
                    code_block_buf.push_str(m.as_ref());
                } else {
                    writer.push_str_styled(style_dim(), &format!("`${}`", m.as_ref()));
                }
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

    if chrome.is_default() {
        cache_markdown_result(md, content_width, use_osc8, lines.clone());
    }

    lines
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
    fn cache_key_distinguishes_same_length_different_content() {
        assert_ne!(
            super::cache_key_for_code_block("rust", "aa"),
            super::cache_key_for_code_block("rust", "bb")
        );
    }

    #[test]
    fn fenced_rust_emits_highlighted_source() {
        let md = "```rust\nfn main() {}\n```\n";
        let lines = render_markdown_styled(md, 40, Style::default(), MarkdownChrome::default());
        let joined: String = lines
            .iter()
            .flat_map(|l| l.spans.iter())
            .map(|s| s.content.as_ref())
            .collect();
        assert!(joined.contains("fn"));
        assert!(joined.contains("main"));
    }

    #[test]
    fn fenced_unknown_lang_still_renders_body() {
        let md = "```weirdlang\nhello\n```\n";
        let lines = render_markdown_styled(md, 40, Style::default(), MarkdownChrome::default());
        let joined: String = lines
            .iter()
            .flat_map(|l| l.spans.iter())
            .map(|s| s.content.as_ref())
            .collect();
        assert!(joined.contains("hello"));
    }

    #[test]
    fn renders_heading_and_list() {
        let md = "# Title\n\n- a\n- b\n";
        let lines = render_markdown_styled(
            md,
            40,
            Style::default().fg(Color::Green),
            MarkdownChrome::default(),
        );
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
    fn heading_h1_uses_claude_accent_palette() {
        let md = "# Title\n\n";
        let lines = render_markdown_styled(md, 40, Style::default(), MarkdownChrome::default());
        assert!(!lines.is_empty());
        let h1_fg = lines[0].spans.first().and_then(|s| s.style.fg);
        let expected = if std::env::var_os("NO_COLOR").is_some() {
            Some(Color::Reset)
        } else {
            Some(Color::Rgb(255, 140, 66))
        };
        assert_eq!(h1_fg, expected);
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

    #[test]
    fn pad_end_aligns_slash_command_column() {
        let a = pad_end_to_display_width("/a", 8);
        let b = pad_end_to_display_width("/clear", 8);
        assert_eq!(text_display_width(&a), 8);
        assert_eq!(text_display_width(&b), 8);
    }

    #[test]
    fn truncate_to_display_width_respects_wide_chars() {
        let s = "你好";
        assert!(text_display_width(s) >= 4);
        let t = truncate_to_display_width(s, 3);
        assert!(text_display_width(&t) <= 3);
    }
}
