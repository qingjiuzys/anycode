import AppKit
import Foundation
import Vision

enum OCRError: LocalizedError {
    case fileMissing(String)
    case imageLoadFailed
    case emptyResult

    var errorDescription: String? {
        switch self {
        case .fileMissing(let path):
            return "Image file not found: \(path)"
        case .imageLoadFailed:
            return "Could not load image for OCR"
        case .emptyResult:
            return "No text recognized in image"
        }
    }
}

func recognizeText(imagePath: String, languages: [String]) throws -> String {
    guard FileManager.default.fileExists(atPath: imagePath) else {
        throw OCRError.fileMissing(imagePath)
    }
    let url = URL(fileURLWithPath: imagePath)
    guard let nsImage = NSImage(contentsOf: url),
          let cgImage = nsImage.cgImage(forProposedRect: nil, context: nil, hints: nil)
    else {
        throw OCRError.imageLoadFailed
    }

    let request = VNRecognizeTextRequest()
    request.recognitionLevel = .accurate
    request.usesLanguageCorrection = true
    if #available(macOS 13, *) {
        request.automaticallyDetectsLanguage = true
    }
    request.recognitionLanguages = languages

    let handler = VNImageRequestHandler(cgImage: cgImage, options: [:])
    try handler.perform([request])

    let lines: [String] = (request.results ?? [])
        .compactMap { $0.topCandidates(1).first?.string }
    let text = lines.joined(separator: "\n").trimmingCharacters(in: .whitespacesAndNewlines)
    if text.isEmpty {
        throw OCRError.emptyResult
    }
    return text
}
