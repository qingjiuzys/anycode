//! Safe local file parsing for WeChat attachments (Excel-first for sales workflows).

use crate::model::{
    AttachmentParseStatus, ExcelColumnHint, ExcelSummary, ParsedFilePreview, TextFileSummary,
};
use crate::wc4_appmsg::is_spreadsheet_ext;
use std::fs;
use std::path::Path;

pub const DEFAULT_MAX_PARSE_BYTES: u64 = 20 * 1024 * 1024;
pub const MAX_PREVIEW_ROWS: usize = 20;
pub const MAX_PREVIEW_COLS: usize = 12;
pub const MAX_PREVIEW_LINES: usize = 30;

pub struct ParseLimits {
    pub max_bytes: u64,
}

impl Default for ParseLimits {
    fn default() -> Self {
        Self {
            max_bytes: DEFAULT_MAX_PARSE_BYTES,
        }
    }
}

pub fn parse_local_file(
    path: &Path,
    extension: Option<&str>,
    limits: &ParseLimits,
) -> (
    AttachmentParseStatus,
    Option<ParsedFilePreview>,
    Option<String>,
) {
    let meta = match fs::metadata(path) {
        Ok(m) => m,
        Err(e) => {
            return (
                AttachmentParseStatus::MissingFile,
                None,
                Some(format!("metadata: {e}")),
            );
        }
    };
    if !meta.is_file() {
        return (
            AttachmentParseStatus::MissingFile,
            None,
            Some("not a file".into()),
        );
    }
    if meta.len() > limits.max_bytes {
        return (
            AttachmentParseStatus::ParseFailed,
            None,
            Some(format!(
                "file exceeds max parse size {} bytes",
                limits.max_bytes
            )),
        );
    }

    let ext = extension
        .map(str::to_ascii_lowercase)
        .or_else(|| {
            path.extension()
                .and_then(|e| e.to_str())
                .map(str::to_ascii_lowercase)
        })
        .unwrap_or_default();

    if is_spreadsheet_ext(&ext) {
        match parse_excel(path) {
            Ok(summary) => (
                AttachmentParseStatus::Parsed,
                Some(ParsedFilePreview::Excel(summary)),
                None,
            ),
            Err(e) => (AttachmentParseStatus::ParseFailed, None, Some(e)),
        }
    } else if matches!(ext.as_str(), "txt" | "csv" | "md" | "json" | "log") {
        match parse_text(path) {
            Ok(summary) => (
                AttachmentParseStatus::Parsed,
                Some(ParsedFilePreview::Text(summary)),
                None,
            ),
            Err(e) => (AttachmentParseStatus::ParseFailed, None, Some(e)),
        }
    } else {
        (
            AttachmentParseStatus::Unsupported,
            None,
            Some(format!("unsupported extension: {ext}")),
        )
    }
}

fn parse_excel(path: &Path) -> Result<ExcelSummary, String> {
    use calamine::{open_workbook_auto, Reader};

    let mut workbook = open_workbook_auto(path).map_err(|e| format!("open workbook: {e}"))?;
    let sheet_names = workbook.sheet_names().to_vec();
    if sheet_names.is_empty() {
        return Err("workbook has no sheets".into());
    }
    let active = sheet_names[0].clone();
    let range = workbook
        .worksheet_range(&active)
        .map_err(|e| format!("read sheet: {e}"))?;

    let mut rows: Vec<Vec<String>> = Vec::new();
    for row in range.rows().take(MAX_PREVIEW_ROWS + 1) {
        let cells: Vec<String> = row
            .iter()
            .take(MAX_PREVIEW_COLS)
            .map(cell_to_string)
            .map(|s| redact_pii(&s))
            .collect();
        if cells.iter().any(|c| !c.is_empty()) {
            rows.push(cells);
        }
    }
    let row_count = rows.len();
    let column_count = rows.first().map(|r| r.len()).unwrap_or(0);
    let header_row = rows.first().cloned().unwrap_or_default();
    let preview_rows = if rows.len() > 1 {
        rows[1..].to_vec()
    } else {
        Vec::new()
    };
    let detected_fields = detect_ecommerce_columns(&header_row);

    Ok(ExcelSummary {
        sheet_names,
        active_sheet: active,
        row_count,
        column_count,
        header_row,
        preview_rows,
        detected_fields,
        pii_redacted: true,
    })
}

fn parse_text(path: &Path) -> Result<TextFileSummary, String> {
    let raw = fs::read_to_string(path).map_err(|e| format!("read text: {e}"))?;
    let lines: Vec<String> = raw
        .lines()
        .take(MAX_PREVIEW_LINES)
        .map(redact_pii)
        .collect();
    Ok(TextFileSummary {
        line_count: raw.lines().count(),
        preview_lines: lines,
        pii_redacted: true,
    })
}

fn cell_to_string(cell: &calamine::Data) -> String {
    use calamine::Data;
    match cell {
        Data::Empty => String::new(),
        Data::String(s) => s.clone(),
        Data::Float(f) => f.to_string(),
        Data::Int(i) => i.to_string(),
        Data::Bool(b) => b.to_string(),
        Data::DateTime(f) => f.to_string(),
        Data::DateTimeIso(s) | Data::DurationIso(s) => s.clone(),
        Data::Error(e) => format!("{e:?}"),
    }
}

fn detect_ecommerce_columns(headers: &[String]) -> Vec<ExcelColumnHint> {
    let patterns: &[(&str, &[&str])] = &[
        ("order_id", &["订单", "order", "orderid", "单号"]),
        ("sku", &["sku", "货号", "编码", "商品编码"]),
        ("product", &["商品", "product", "品名", "名称", "title"]),
        ("quantity", &["数量", "qty", "quantity", "件数"]),
        (
            "amount",
            &["金额", "amount", "price", "单价", "总价", "合计"],
        ),
        ("customer", &["客户", "customer", "买家", "收货人", "姓名"]),
        ("phone", &["手机", "phone", "tel", "电话"]),
        ("address", &["地址", "address", "收货地址"]),
    ];
    let mut out = Vec::new();
    for (idx, header) in headers.iter().enumerate() {
        let h = header.to_ascii_lowercase();
        for (category, keys) in patterns {
            if keys.iter().any(|k| h.contains(k)) {
                out.push(ExcelColumnHint {
                    name: header.clone(),
                    column_index: idx,
                    category: (*category).to_string(),
                });
                break;
            }
        }
    }
    out
}

pub fn redact_pii(input: &str) -> String {
    let mut out = input.to_string();
    // Mainland mobile numbers
    let re_phone = regex_simple_phone();
    out = re_phone.replace_all(&out, "[phone]").to_string();
    // Long digit sequences (ID-like)
    if out.chars().filter(|c| c.is_ascii_digit()).count() > 12 {
        if out.len() > 80 {
            out.truncate(77);
            out.push('…');
        }
    }
    out
}

fn regex_simple_phone() -> regex::Regex {
    regex::Regex::new(r"1[3-9]\d{9}").expect("phone regex")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn redacts_phone_numbers() {
        assert!(redact_pii("联系13800138000").contains("[phone]"));
    }

    #[test]
    fn detects_ecommerce_headers() {
        let hints = detect_ecommerce_columns(&[
            "订单号".into(),
            "SKU".into(),
            "数量".into(),
            "金额".into(),
        ]);
        assert!(hints.iter().any(|h| h.category == "order_id"));
        assert!(hints.iter().any(|h| h.category == "sku"));
    }

    #[test]
    fn parse_csv_text_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("note.txt");
        {
            let mut f = fs::File::create(&path).unwrap();
            writeln!(f, "hello sales").unwrap();
        }
        let (status, preview, err) = parse_local_file(&path, Some("txt"), &ParseLimits::default());
        assert!(err.is_none());
        assert_eq!(status, AttachmentParseStatus::Parsed);
        assert!(matches!(preview, Some(ParsedFilePreview::Text(_))));
    }
}
