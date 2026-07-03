//! WeChat inbound voice → STT via `models.speech.stt`.

use super::cdn_media::{download_and_decrypt, extract_cdn_keys_for_voice_item};
use super::fields::item_type;
use anycode_llm::{
    config_file::read_config_value,
    media::{MediaClientRegistry, SttClient},
};
use anyhow::Result;
use serde_json::Value;

pub async fn transcribe_voice_items(items: &[Value]) -> Option<String> {
    let voice = items.iter().find(|it| item_type(it) == 3)?;
    let (_, cfg) = read_config_value(None).ok()?;
    let reg = MediaClientRegistry::from_config(&cfg);
    let prof = reg.stt.as_ref()?;
    let (enc, aes, full) = extract_cdn_keys_for_voice_item(voice)?;
    let client = reqwest::Client::new();
    let bytes = download_and_decrypt(&client, &enc, &aes, full.as_deref())
        .await
        .ok()?;
    let stt = SttClient::new(prof.profile.clone());
    stt.transcribe(&bytes, "voice.amr")
        .await
        .ok()
        .map(|r| r.text)
        .filter(|t| !t.trim().is_empty())
}

pub async fn transcribe_voice_items_result(items: &[Value]) -> Result<Option<String>> {
    Ok(transcribe_voice_items(items).await)
}
