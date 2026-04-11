/// 单行输入缓冲（UTF-8 安全光标），对齐常见终端行编辑。
#[derive(Default)]
pub(crate) struct InputState {
    pub(crate) chars: Vec<char>,
    pub(crate) cursor: usize,
}

impl InputState {
    pub(crate) fn is_empty(&self) -> bool {
        self.chars.is_empty()
    }

    pub(crate) fn clear(&mut self) {
        self.chars.clear();
        self.cursor = 0;
    }

    pub(crate) fn as_string(&self) -> String {
        self.chars.iter().collect()
    }

    pub(crate) fn insert(&mut self, c: char) {
        self.chars.insert(self.cursor, c);
        self.cursor += 1;
    }

    pub(crate) fn insert_str(&mut self, s: &str) {
        for c in s.chars() {
            self.chars.insert(self.cursor, c);
            self.cursor += 1;
        }
    }

    pub(crate) fn backspace(&mut self) -> bool {
        if self.cursor == 0 {
            return false;
        }
        self.cursor -= 1;
        self.chars.remove(self.cursor);
        true
    }

    pub(crate) fn delete_forward(&mut self) -> bool {
        if self.cursor >= self.chars.len() {
            return false;
        }
        self.chars.remove(self.cursor);
        true
    }

    pub(crate) fn move_left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    pub(crate) fn move_right(&mut self) {
        if self.cursor < self.chars.len() {
            self.cursor += 1;
        }
    }

    pub(crate) fn move_home(&mut self) {
        self.cursor = 0;
    }

    pub(crate) fn move_end(&mut self) {
        self.cursor = self.chars.len();
    }

    pub(crate) fn set_from_str(&mut self, s: &str) {
        self.chars = s.chars().collect();
        self.cursor = self.chars.len();
    }

    /// 将光标前移到上一行末尾（用于 Alt+↑ 等，可选）。
    /// 全程使用 **字符下标**，避免 `rfind` 字节偏移与 `cursor` 混用导致越界或异常。
    pub(crate) fn move_line_up(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let mut nl = None;
        for i in (0..self.cursor).rev() {
            if self.chars[i] == '\n' {
                nl = Some(i);
                break;
            }
        }
        let Some(nl_idx) = nl else {
            return;
        };
        let line_start = nl_idx + 1;
        let col = self.cursor.saturating_sub(line_start);
        let mut prev_nl = None;
        for i in (0..nl_idx).rev() {
            if self.chars[i] == '\n' {
                prev_nl = Some(i);
                break;
            }
        }
        let prev_line_start = prev_nl.map(|i| i + 1).unwrap_or(0);
        let prev_content_len = nl_idx.saturating_sub(prev_line_start);
        let target = prev_line_start + col.min(prev_content_len);
        self.cursor = target;
    }

    pub(crate) fn move_line_down(&mut self) {
        let after = &self.chars[self.cursor..];
        if let Some(rel) = after.iter().position(|&c| c == '\n') {
            self.cursor += rel + 1;
        }
    }

    pub(crate) fn line_char_bounds(&self) -> (usize, usize) {
        let mut line_start = 0usize;
        for i in (0..self.cursor).rev() {
            if self.chars[i] == '\n' {
                line_start = i + 1;
                break;
            }
        }
        let mut line_end = self.chars.len();
        for i in self.cursor..self.chars.len() {
            if self.chars[i] == '\n' {
                line_end = i;
                break;
            }
        }
        (line_start, line_end)
    }

    pub(crate) fn move_word_left(&mut self) {
        let mut i = self.cursor;
        if i == 0 {
            return;
        }
        i -= 1;
        while i > 0 && self.chars[i].is_whitespace() {
            i -= 1;
        }
        while i > 0 && !self.chars[i - 1].is_whitespace() {
            i -= 1;
        }
        self.cursor = i;
    }

    pub(crate) fn move_word_right(&mut self) {
        let n = self.chars.len();
        let mut i = self.cursor;
        while i < n && self.chars[i].is_whitespace() {
            i += 1;
        }
        while i < n && !self.chars[i].is_whitespace() {
            i += 1;
        }
        self.cursor = i;
    }

    pub(crate) fn delete_word_backward(&mut self) {
        let end = self.cursor;
        if end == 0 {
            return;
        }
        self.move_word_left();
        let start = self.cursor;
        if start < end {
            self.chars.drain(start..end);
        }
    }

    /// 删除从光标到当前行尾（不含换行符），对齐 readline `Ctrl+K`。
    pub(crate) fn delete_to_end_of_line(&mut self) {
        let (_, le) = self.line_char_bounds();
        while self.cursor < le {
            self.chars.remove(self.cursor);
        }
    }
}

/// `^R` 反向搜索：查询串 + 在匹配列表中的下标（自新到旧）。
#[derive(Default)]
pub(crate) struct RevSearchState {
    pub(crate) query: InputState,
    pub(crate) pick: usize,
}

impl RevSearchState {
    pub(crate) fn matches(&self, history: &[String]) -> Vec<String> {
        let qs = self.query.as_string();
        let q = trim_or_default(&qs);
        if q.is_empty() {
            return Vec::new();
        }
        history
            .iter()
            .rev()
            .filter(|s| s.contains(q))
            .cloned()
            .collect()
    }
}

/// 多行 Prompt：`>` / 续行缩进，按 **内区宽度** 自动折行。
/// 不绘制 `▌`（由 `Frame::set_cursor` 放置硬件光标，便于中文 IME 预编辑与之一致）。
pub(crate) fn prompt_multiline_lines_and_cursor(
    input: &InputState,
    inner_width: u16,
    // 首行 `/` 命令名后的 ghost 后缀（对齐 Claude 行内补全提示）。
    slash_ghost: Option<String>,
) -> (Vec<Line<'static>>, Option<(usize, u16)>) {
    let iw = inner_width as usize;
    let full: String = input.as_string();
    let before: String = input.chars[..input.cursor].iter().collect();
    let cur_line_idx = before.matches('\n').count();
    let mut line_start_char = 0usize;
    for (i, ch) in before.chars().enumerate() {
        if ch == '\n' {
            line_start_char = i + 1;
        }
    }
    let col = before.chars().count().saturating_sub(line_start_char);

    let style_pfx = style_brand();
    let style_txt = Style::default().fg(Color::White);

    if iw < 4 {
        let pw = text_display_width("> ");
        return (
            vec![Line::from(vec![
                Span::styled("> ", style_pfx),
                Span::styled("…", style_txt),
            ])],
            Some((0, pw as u16)),
        );
    }

    let prefix_first = "> ";
    let prefix_cont = "  ";
    let pw = text_display_width(prefix_first).max(text_display_width(prefix_cont));
    let content_w = iw.saturating_sub(pw).max(1);

    let logical: Vec<String> = if full.is_empty() {
        vec![String::new()]
    } else {
        full.split('\n').map(|s| s.to_string()).collect()
    };

    let mut out: Vec<Line<'static>> = Vec::new();
    let mut cursor_cell: Option<(usize, u16)> = None;
    for (i, line_text) in logical.iter().enumerate() {
        let segments = wrap_string_to_width(line_text.as_str(), content_w);
        let mut char_offset = 0usize;
        let mut cursor_done = false;
        for (si, seg) in segments.iter().enumerate() {
            let seg_char_len = seg.chars().count();
            let prefix = if i == 0 && si == 0 {
                prefix_first
            } else {
                prefix_cont
            };
            if i == cur_line_idx && !cursor_done && col <= char_offset + seg_char_len {
                let local = col.saturating_sub(char_offset);
                let left: String = seg.chars().take(local).collect();
                let right: String = seg.chars().skip(local).collect();
                let ghost = if i == 0 && si == 0 {
                    slash_ghost.as_ref().filter(|g| !g.is_empty())
                } else {
                    None
                };
                // 硬件光标在已输入末尾；ghost 为其后灰色提示，不计入光标列。
                let ox_usize = text_display_width(prefix) + text_display_width(left.as_str());
                let ox = u16::try_from(ox_usize).unwrap_or(u16::MAX);
                let mut spans = vec![
                    Span::styled(prefix, style_pfx),
                    Span::styled(left, style_txt),
                ];
                if let Some(g) = ghost {
                    spans.push(Span::styled(g.clone(), style_dim()));
                }
                spans.push(Span::styled(right, style_txt));
                out.push(Line::from(spans));
                let line_idx = out.len() - 1;
                cursor_cell = Some((line_idx, ox));
                cursor_done = true;
            } else {
                out.push(Line::from(vec![
                    Span::styled(prefix, style_pfx),
                    Span::styled(seg.clone(), style_txt),
                ]));
            }
            char_offset += seg_char_len;
        }
    }
    if cursor_cell.is_none() && !out.is_empty() {
        let w = text_display_width("> ");
        cursor_cell = Some((0, u16::try_from(w).unwrap_or(u16::MAX)));
    }
    (out, cursor_cell)
}

pub(crate) fn history_apply_up(history: &[String], idx: &mut Option<usize>, input: &mut InputState) {
    if history.is_empty() {
        return;
    }
    let i = match *idx {
        None => history.len().saturating_sub(1),
        Some(i) => i.saturating_sub(1),
    };
    *idx = Some(i);
    input.set_from_str(&history[i]);
}

pub(crate) fn history_apply_down(history: &[String], idx: &mut Option<usize>, input: &mut InputState) {
    let Some(cur) = *idx else {
        return;
    };
    if cur + 1 >= history.len() {
        *idx = None;
        input.clear();
    } else {
        let n = cur + 1;
        *idx = Some(n);
        input.set_from_str(&history[n]);
    }
}

#[allow(dead_code)]
pub(crate) fn format_cwd_header(cwd: &str, max_chars: usize) -> String {
    if cwd.chars().count() <= max_chars {
        return cwd.to_string();
    }
    cwd.rfind('/')
        .map(|i| format!("…{}", &cwd[i..]))
        .unwrap_or_else(|| truncate_preview(cwd, max_chars))
}
