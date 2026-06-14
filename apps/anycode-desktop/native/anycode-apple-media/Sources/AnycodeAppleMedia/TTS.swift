import AVFoundation
import Foundation

enum TTSError: LocalizedError {
    case emptyText
    case synthesizerFailed
    case writeFailed(String)

    var errorDescription: String? {
        switch self {
        case .emptyText:
            return "TTS text is empty"
        case .synthesizerFailed:
            return "AVSpeechSynthesizer failed"
        case .writeFailed(let reason):
            return "Failed to write TTS audio: \(reason)"
        }
    }
}

func synthesizeSpeech(text: String, voice: String?, locale: String, outputPath: String) throws -> Data {
    let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
    guard !trimmed.isEmpty else { throw TTSError.emptyText }

    let utterance = AVSpeechUtterance(string: trimmed)
    utterance.rate = AVSpeechUtteranceDefaultSpeechRate
    utterance.voice = resolveVoice(voice: voice, locale: locale)

    let synthesizer = AVSpeechSynthesizer()
    var buffers: [AVAudioPCMBuffer] = []
    var format: AVAudioFormat?
    let done = DispatchSemaphore(value: 0)

    synthesizer.write(utterance) { buffer in
        guard let pcm = buffer as? AVAudioPCMBuffer else {
            done.signal()
            return
        }
        if pcm.frameLength == 0 {
            done.signal()
            return
        }
        if format == nil {
            format = pcm.format
        }
        buffers.append(pcm)
    }

    if done.wait(timeout: .now() + 120) == .timedOut {
        throw TTSError.synthesizerFailed
    }
    guard let format, !buffers.isEmpty else {
        throw TTSError.synthesizerFailed
    }

    let outURL = URL(fileURLWithPath: outputPath)
    try? FileManager.default.removeItem(at: outURL)
    let file = try AVAudioFile(forWriting: outURL, settings: format.settings)
    for pcm in buffers {
        try file.write(from: pcm)
    }
    return try Data(contentsOf: outURL)
}

private func resolveVoice(voice: String?, locale: String) -> AVSpeechSynthesisVoice? {
    if let voice, !voice.isEmpty, let v = AVSpeechSynthesisVoice(identifier: voice) {
        return v
    }
    if let v = AVSpeechSynthesisVoice(language: locale) {
        return v
    }
    return AVSpeechSynthesisVoice(language: "zh-CN") ?? AVSpeechSynthesisVoice(language: "en-US")
}
