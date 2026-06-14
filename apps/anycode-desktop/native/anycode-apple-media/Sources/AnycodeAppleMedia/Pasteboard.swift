import AppKit
import Foundation
import UniformTypeIdentifiers

struct PasteboardItemPayload: Encodable {
    let kind: String
    let mimeType: String?
    let text: String?
    let dataBase64: String?

    enum CodingKeys: String, CodingKey {
        case kind
        case mimeType = "mime_type"
        case text
        case dataBase64 = "data_base64"
    }
}

func readPasteboardItems() throws -> [PasteboardItemPayload] {
    let pb = NSPasteboard.general
    var items: [PasteboardItemPayload] = []

    if let urls = pb.readObjects(forClasses: [NSURL.self], options: nil) as? [URL], !urls.isEmpty {
        for url in urls {
            items.append(PasteboardItemPayload(
                kind: "file_url",
                mimeType: nil,
                text: url.path,
                dataBase64: nil
            ))
        }
    }

    if let strings = pb.readObjects(forClasses: [NSString.self], options: nil) as? [String],
       let first = strings.first, !first.isEmpty
    {
        items.append(PasteboardItemPayload(
            kind: "text",
            mimeType: "text/plain",
            text: first,
            dataBase64: nil
        ))
    }

    if let data = pb.data(forType: .png) {
        items.append(PasteboardItemPayload(
            kind: "image",
            mimeType: "image/png",
            text: nil,
            dataBase64: data.base64EncodedString()
        ))
    } else if let data = pb.data(forType: .tiff) {
        items.append(PasteboardItemPayload(
            kind: "image",
            mimeType: "image/tiff",
            text: nil,
            dataBase64: data.base64EncodedString()
        ))
    }

    if let rtf = pb.data(forType: .rtf) {
        items.append(PasteboardItemPayload(
            kind: "rtf",
            mimeType: "text/rtf",
            text: nil,
            dataBase64: rtf.base64EncodedString()
        ))
    }

    return items
}

func encodePasteboardItems(_ items: [PasteboardItemPayload]) throws -> String {
    let data = try JSONEncoder().encode(items)
    guard let text = String(data: data, encoding: .utf8) else {
        throw NSError(domain: "Pasteboard", code: 1, userInfo: [NSLocalizedDescriptionKey: "encode failed"])
    }
    return text
}
