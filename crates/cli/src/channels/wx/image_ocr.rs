//! WeChat inbound image OCR (macOS Apple Vision when helper is available).

use crate::apple_media;

pub fn ocr_inbound_image(mime: &str, bytes: &[u8]) -> Option<String> {
    if !apple_media::apple_media_ocr_available() {
        return None;
    }
    apple_media::ocr_image(mime, bytes)
}
