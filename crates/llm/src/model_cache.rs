//! On-demand model asset cache under `~/.anycode/models/`.

use std::path::PathBuf;

/// Root directory for user-managed model files (Whisper ggml, Piper voices, …).
pub fn anycode_models_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".anycode")
        .join("models")
}

pub fn whisper_model_path(model_id: &str) -> PathBuf {
    anycode_models_dir()
        .join("whisper")
        .join(format!("{model_id}.bin"))
}

pub fn piper_voice_dir(voice_id: &str) -> PathBuf {
    anycode_models_dir().join("piper").join(voice_id)
}

pub fn ensure_models_dir() -> std::io::Result<PathBuf> {
    let dir = anycode_models_dir();
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whisper_path_uses_model_id() {
        let p = whisper_model_path("tiny");
        assert!(p.to_string_lossy().contains("whisper"));
        assert!(p.to_string_lossy().ends_with("tiny.bin"));
    }
}
