import AVFoundation
import Foundation
import Speech

struct CapabilitiesPayload: Encodable {
    let stt: Bool
    let ocr: Bool
    let tts: Bool
    let notify: Bool
    let keychain: Bool
    let pasteboard: Bool
    let platform: String
    let helperPath: String?
    let speechAuthorized: Bool?
    let microphoneAuthorized: Bool?

    enum CodingKeys: String, CodingKey {
        case stt, ocr, tts, notify, keychain, pasteboard, platform
        case helperPath = "helper_path"
        case speechAuthorized = "speech_authorized"
        case microphoneAuthorized = "microphone_authorized"
    }
}

func buildCapabilities(helperPath: String?) -> CapabilitiesPayload {
    let speechStatus = SFSpeechRecognizer.authorizationStatus()
    let micStatus = AVCaptureDevice.authorizationStatus(for: .audio)
    return CapabilitiesPayload(
        stt: true,
        ocr: true,
        tts: true,
        notify: true,
        keychain: true,
        pasteboard: true,
        platform: "macos",
        helperPath: helperPath,
        speechAuthorized: speechStatus == .authorized,
        microphoneAuthorized: micStatus == .authorized
    )
}

func encodeCapabilities(_ caps: CapabilitiesPayload) -> String? {
    let encoder = JSONEncoder()
    guard let data = try? encoder.encode(caps) else { return nil }
    return String(data: data, encoding: .utf8)
}
