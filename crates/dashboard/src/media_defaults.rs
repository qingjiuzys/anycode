//! Optional STT defaults — Apple Speech is chosen manually in Settings (desktop app).

/// Previously auto-enabled whisper.cpp; now users pick Apple Speech or whisper in Settings.
pub fn ensure_default_local_stt() -> anyhow::Result<bool> {
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn does_not_auto_enable_stt() {
        assert!(!ensure_default_local_stt().unwrap());
    }
}
