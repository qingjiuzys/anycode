//! SSE `data:` 行缓冲解析（OpenAI / Anthropic 流式等共用 wire 习惯：`data:` / `[DONE]` / 注释行）。

/// 单条 `data:` 有效载荷（不含 `data:` 前缀）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SseDataLine {
    /// OpenAI 风格结束标记。
    Done,
    /// `data:` 后的负载（已 trim）。
    Payload(String),
}

/// 跨 chunk 拼行，产出完整的 `data:` 负载；忽略注释行（`:` 开头）、非 `data` 行。
#[derive(Debug, Default)]
pub struct SseLineBuffer {
    buf: String,
}

impl SseLineBuffer {
    pub fn new() -> Self {
        Self::default()
    }

    /// 追加 UTF-8 片段（通常为 `bytes_stream` 解码结果），返回本片段内**完整行**对应的负载。
    pub fn push_str(&mut self, chunk: &str) -> Vec<SseDataLine> {
        self.buf.push_str(chunk);
        self.drain_complete_lines(false)
    }

    /// 流结束时尚未以 `\n` 结尾的尾部，按一行处理（若存在）。
    pub fn finish(&mut self) -> Vec<SseDataLine> {
        let out = self.drain_complete_lines(true);
        self.buf.clear();
        out
    }

    fn drain_complete_lines(&mut self, eof: bool) -> Vec<SseDataLine> {
        let mut out = Vec::new();
        loop {
            let Some(pos) = self.buf.find('\n') else {
                if eof && !self.buf.is_empty() {
                    let line = std::mem::take(&mut self.buf);
                    self.push_line_payload(&line, &mut out);
                }
                break;
            };
            let line = self.buf[..pos].trim_end_matches('\r').to_string();
            self.buf.drain(..=pos);
            self.push_line_payload(&line, &mut out);
        }
        out
    }

    fn push_line_payload(&self, line: &str, out: &mut Vec<SseDataLine>) {
        let line = line.trim_end();
        if line.is_empty() {
            return;
        }
        if line.starts_with(':') {
            return;
        }
        let Some(rest) = line.strip_prefix("data:") else {
            return;
        };
        let data = rest.trim_start();
        if data.is_empty() {
            return;
        }
        if data == "[DONE]" {
            out.push(SseDataLine::Done);
        } else {
            out.push(SseDataLine::Payload(data.to_string()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_data_across_chunks() {
        let mut b = SseLineBuffer::new();
        let mut acc = Vec::new();
        acc.extend(b.push_str("data: {\"a\":"));
        acc.extend(b.push_str("1}\n\n"));
        acc.extend(b.push_str("data: "));
        acc.extend(b.push_str("[DONE]\n\n"));
        assert_eq!(
            acc,
            vec![
                SseDataLine::Payload("{\"a\":1}".to_string()),
                SseDataLine::Done
            ]
        );
    }

    #[test]
    fn ignores_comments_and_empty() {
        let mut b = SseLineBuffer::new();
        let mut acc = Vec::new();
        acc.extend(b.push_str(": keepalive\n"));
        acc.extend(b.push_str("\n"));
        acc.extend(b.push_str("data: hi\n"));
        assert_eq!(acc, vec![SseDataLine::Payload("hi".to_string())]);
    }

    #[test]
    fn eof_tail_without_newline() {
        let mut b = SseLineBuffer::new();
        b.push_str("data: {\"x\":1}");
        let tail = b.finish();
        assert_eq!(tail, vec![SseDataLine::Payload("{\"x\":1}".to_string())]);
    }
}
