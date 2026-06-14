import AVFoundation
import Foundation

enum AudioConvertError: LocalizedError {
    case inputMissing(String)
    case exportFailed(String)

    var errorDescription: String? {
        switch self {
        case .inputMissing(let path):
            return "Audio file not found: \(path)"
        case .exportFailed(let reason):
            return "Audio conversion failed: \(reason)"
        }
    }
}

func convertAudio(inputPath: String, outputPath: String, format: String) throws {
    let inputURL = URL(fileURLWithPath: inputPath)
    guard FileManager.default.fileExists(atPath: inputPath) else {
        throw AudioConvertError.inputMissing(inputPath)
    }

    let asset = AVURLAsset(url: inputURL)
    guard let session = AVAssetExportSession(asset: asset, presetName: AVAssetExportPresetAppleM4A) else {
        throw AudioConvertError.exportFailed("cannot create export session")
    }

    let outURL = URL(fileURLWithPath: outputPath)
    try? FileManager.default.removeItem(at: outURL)

    let targetFormat = format.lowercased()
    if targetFormat == "wav" {
        session.outputFileType = .wav
    } else if targetFormat == "m4a" {
        session.outputFileType = .m4a
    } else {
        session.outputFileType = .wav
    }
    session.outputURL = outURL

    var done = false
    var exportError: Error?
    session.exportAsynchronously {
        if let err = session.error {
            exportError = err
        }
        done = true
    }
    try pumpRunLoopUntil({ done }, timeout: 120)

    if let exportError { throw exportError }
    guard session.status == .completed else {
        throw AudioConvertError.exportFailed("status=\(session.status.rawValue)")
    }
}
