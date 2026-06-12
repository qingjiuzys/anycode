import Foundation

struct Request: Decodable {
    let op: String
    let audioPath: String?
    let imagePath: String?
    let locale: String?
    let languages: [String]?

    enum CodingKeys: String, CodingKey {
        case op
        case audioPath = "audio_path"
        case imagePath = "image_path"
        case locale
        case languages
    }
}

struct Response: Encodable {
    let ok: Bool
    let text: String?
    let error: String?
}

func writeResponse(_ response: Response) {
    let encoder = JSONEncoder()
    guard let data = try? encoder.encode(response),
          let line = String(data: data, encoding: .utf8)
    else {
        print("{\"ok\":false,\"error\":\"encode response failed\"}")
        return
    }
    print(line)
    fflush(stdout)
}

func readRequest() -> Request? {
    let input = FileHandle.standardInput
    guard let data = try? input.readToEnd(), !data.isEmpty else {
        return nil
    }
    return try? JSONDecoder().decode(Request.self, from: data)
}

let req = readRequest()
guard let req else {
    writeResponse(Response(ok: false, text: nil, error: "missing stdin JSON request"))
    exit(1)
}

switch req.op {
case "stt":
    guard let path = req.audioPath, !path.isEmpty else {
        writeResponse(Response(ok: false, text: nil, error: "audio_path required"))
        exit(1)
    }
    do {
        let text = try transcribeAudio(path: path, locale: req.locale ?? "zh-CN")
        writeResponse(Response(ok: true, text: text, error: nil))
    } catch {
        writeResponse(Response(ok: false, text: nil, error: error.localizedDescription))
    }
case "ocr":
    guard let path = req.imagePath, !path.isEmpty else {
        writeResponse(Response(ok: false, text: nil, error: "image_path required"))
        exit(1)
    }
    do {
        let langs = req.languages ?? ["zh-Hans", "en-US"]
        let text = try recognizeText(imagePath: path, languages: langs)
        writeResponse(Response(ok: true, text: text, error: nil))
    } catch {
        writeResponse(Response(ok: false, text: nil, error: error.localizedDescription))
    }
case "capabilities":
    writeResponse(Response(ok: true, text: "macos", error: nil))
default:
    writeResponse(Response(ok: false, text: nil, error: "unknown op: \(req.op)"))
    exit(1)
}
