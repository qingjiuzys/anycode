import AVFoundation
import Foundation
import Speech

enum STTError: LocalizedError {
    case permissionDenied
    case recognizerUnavailable
    case emptyResult
    case fileMissing(String)
    case timedOut

    var errorDescription: String? {
        switch self {
        case .permissionDenied:
            return "Speech recognition permission denied — enable in System Settings → Privacy → Speech Recognition"
        case .recognizerUnavailable:
            return "Speech recognizer unavailable for this locale"
        case .emptyResult:
            return "No speech detected in recording"
        case .fileMissing(let path):
            return "Audio file not found: \(path)"
        case .timedOut:
            return "Speech recognition timed out"
        }
    }
}

private func ensureSpeechAuthorization() throws {
    switch SFSpeechRecognizer.authorizationStatus() {
    case .authorized:
        return
    case .denied, .restricted:
        throw STTError.permissionDenied
    case .notDetermined:
        break
    @unknown default:
        break
    }

    var done = false
    var status: SFSpeechRecognizerAuthorizationStatus = .notDetermined
    DispatchQueue.main.async {
        SFSpeechRecognizer.requestAuthorization { s in
            status = s
            done = true
        }
    }
    try pumpRunLoopUntil({ done }, timeout: 120)
    guard status == .authorized else {
        throw STTError.permissionDenied
    }
}

func transcribeAudio(path: String, locale: String) throws -> String {
    let url = URL(fileURLWithPath: path)
    guard FileManager.default.fileExists(atPath: path) else {
        throw STTError.fileMissing(path)
    }

    try ensureSpeechAuthorization()

    let localeId = Locale(identifier: locale)
    guard let recognizer = SFSpeechRecognizer(locale: localeId), recognizer.isAvailable else {
        throw STTError.recognizerUnavailable
    }

    let request = SFSpeechURLRecognitionRequest(url: url)
    request.shouldReportPartialResults = false
    if #available(macOS 13, *) {
        request.addsPunctuation = true
    }

    var resultText: String?
    var resultError: Error?
    var done = false

    let task = recognizer.recognitionTask(with: request) { result, error in
        if let error {
            resultError = error
            done = true
            return
        }
        guard let result, result.isFinal else { return }
        resultText = result.bestTranscription.formattedString
        done = true
    }

    do {
        try pumpRunLoopUntil({ done }, timeout: 120)
    } catch RunLoopWaitError.timedOut {
        task.cancel()
        throw STTError.timedOut
    }

    if let resultError { throw resultError }
    let text = resultText?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
    if text.isEmpty {
        throw STTError.emptyResult
    }
    return text
}
