//! Artifact kind / MIME mapping (single source for runtime + dashboard heuristics).

use std::path::Path;

/// Canonical artifact kinds used in DB `artifacts.kind` and transcript cards.
pub fn artifact_kind_for_path(path: &str) -> &'static str {
    let lower = path.to_lowercase();
    let ext = Path::new(&lower)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    match ext {
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "bmp" | "heic" => "image",
        "mp4" | "webm" | "mov" | "m4v" | "mkv" => "video",
        "mp3" | "wav" | "m4a" | "aac" | "ogg" | "flac" => "audio",
        "pdf" => "pdf",
        "pptx" | "ppt" | "key" => "presentation",
        "docx" | "doc" | "rtf" | "odt" => "document",
        "xlsx" | "xls" | "csv" | "ods" => "spreadsheet",
        "md" | "markdown" => {
            if lower.contains("mindmap") || lower.contains("mind-map") || lower.contains("导图") {
                "mindmap"
            } else if lower.contains("report") {
                "report"
            } else {
                "file"
            }
        }
        "mmd" => "mindmap",
        "json" if lower.contains("workbook") => "spreadsheet",
        // Any HTML is a previewable report card (align with dashboard-ui kindForPath).
        "html" | "htm" => "report",
        "ipynb" => "notebook",
        _ => "file",
    }
}

/// Guess MIME from path extension.
pub fn mime_for_path(path: &str) -> &'static str {
    let lower = path.to_lowercase();
    let ext = Path::new(&lower)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    match ext {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "bmp" => "image/bmp",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "mov" => "video/quicktime",
        "mkv" => "video/x-matroska",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "m4a" => "audio/mp4",
        "ogg" => "audio/ogg",
        "pdf" => "application/pdf",
        "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "csv" => "text/csv",
        "md" | "markdown" | "mmd" => "text/markdown",
        "json" => "application/json",
        "html" | "htm" => "text/html",
        _ => "application/octet-stream",
    }
}

/// Whether this kind should render an inline card in the conversation stream.
pub fn artifact_kind_is_inline(kind: &str) -> bool {
    matches!(
        kind,
        "image"
            | "video"
            | "audio"
            | "pdf"
            | "mindmap"
            | "presentation"
            | "document"
            | "spreadsheet"
            | "media"
            | "report"
    )
}

/// Title from file name.
pub fn artifact_title_for_path(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(path)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_and_mime() {
        assert_eq!(artifact_kind_for_path("/a/b/c.png"), "image");
        assert_eq!(mime_for_path("/a/b/c.png"), "image/png");
        assert_eq!(artifact_kind_for_path("deck.pptx"), "presentation");
        assert_eq!(artifact_kind_for_path("notes.mindmap.md"), "mindmap");
        assert_eq!(artifact_kind_for_path("sheet.xlsx"), "spreadsheet");
        assert_eq!(artifact_kind_for_path("data.json"), "file");
        assert_eq!(artifact_kind_for_path("sales-workbook.json"), "spreadsheet");
        assert_eq!(artifact_kind_for_path("deck.html"), "report");
        assert_eq!(artifact_kind_for_path("notes.md"), "file");
        assert!(artifact_kind_is_inline("spreadsheet"));
        assert!(artifact_kind_is_inline("image"));
        assert!(artifact_kind_is_inline("report"));
        assert!(!artifact_kind_is_inline("bash"));
    }
}
