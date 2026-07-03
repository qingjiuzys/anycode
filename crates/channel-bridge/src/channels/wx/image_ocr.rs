//! WeChat inbound image OCR (macOS Apple Vision when helper is available).

use anycode_apple_media::{apple_media_available, ocr_image_bytes, NO_EXTRA_PATHS};

pub fn ocr_inbound_image(mime: &str, bytes: &[u8]) -> Option<String> {
    if !apple_media_available(NO_EXTRA_PATHS) {
        return None;
    }
    ocr_image_bytes(NO_EXTRA_PATHS, mime, bytes, None)
}
