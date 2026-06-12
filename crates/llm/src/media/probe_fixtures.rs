//! Tiny audio fixtures for dashboard connectivity probes.

/// One second of silence: 16 kHz mono 16-bit PCM WAV.
pub fn minimal_wav_silence_1s() -> Vec<u8> {
    let sample_rate = 16_000u32;
    let channels = 1u16;
    let bits = 16u16;
    let data_bytes = (sample_rate * 2) as usize;
    let mut wav = Vec::new();
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36u32 + data_bytes as u32).to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&channels.to_le_bytes());
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&(sample_rate * channels as u32 * bits as u32 / 8).to_le_bytes());
    wav.extend_from_slice(&(channels * bits / 8).to_le_bytes());
    wav.extend_from_slice(&bits.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&(data_bytes as u32).to_le_bytes());
    wav.extend(std::iter::repeat(0u8).take(data_bytes));
    wav
}
