import Foundation

struct Request: Decodable {
    let op: String
    let audioPath: String?
    let imagePath: String?
    let inputPath: String?
    let outputPath: String?
    let format: String?
    let locale: String?
    let languages: [String]?
    let text: String?
    let voice: String?
    let title: String?
    let body: String?
    let service: String?
    let account: String?
    let secret: String?

    enum CodingKeys: String, CodingKey {
        case op
        case audioPath = "audio_path"
        case imagePath = "image_path"
        case inputPath = "input_path"
        case outputPath = "output_path"
        case format
        case locale
        case languages
        case text
        case voice
        case title
        case body
        case service
        case account
        case secret
    }
}

struct CapabilitiesResponse: Encodable {
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

struct Response: Encodable {
    let ok: Bool
    let text: String?
    let error: String?
    let dataBase64: String?
    let capabilities: CapabilitiesResponse?

    enum CodingKeys: String, CodingKey {
        case ok, text, error
        case dataBase64 = "data_base64"
        case capabilities
    }
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
    writeResponse(Response(ok: false, text: nil, error: "missing stdin JSON request", dataBase64: nil, capabilities: nil))
    exit(1)
}

let helperPath = ProcessInfo.processInfo.arguments.first

switch req.op {
case "stt":
    guard let path = req.audioPath, !path.isEmpty else {
        writeResponse(Response(ok: false, text: nil, error: "audio_path required", dataBase64: nil, capabilities: nil))
        exit(1)
    }
    do {
        let text = try transcribeAudio(path: path, locale: req.locale ?? "zh-CN")
        writeResponse(Response(ok: true, text: text, error: nil, dataBase64: nil, capabilities: nil))
    } catch {
        writeResponse(Response(ok: false, text: nil, error: error.localizedDescription, dataBase64: nil, capabilities: nil))
    }
case "ocr":
    guard let path = req.imagePath, !path.isEmpty else {
        writeResponse(Response(ok: false, text: nil, error: "image_path required", dataBase64: nil, capabilities: nil))
        exit(1)
    }
    do {
        let langs = req.languages ?? ["zh-Hans", "en-US"]
        let text = try recognizeText(imagePath: path, languages: langs)
        writeResponse(Response(ok: true, text: text, error: nil, dataBase64: nil, capabilities: nil))
    } catch {
        writeResponse(Response(ok: false, text: nil, error: error.localizedDescription, dataBase64: nil, capabilities: nil))
    }
case "convert":
    guard let input = req.inputPath, let output = req.outputPath else {
        writeResponse(Response(ok: false, text: nil, error: "input_path and output_path required", dataBase64: nil, capabilities: nil))
        exit(1)
    }
    do {
        try convertAudio(inputPath: input, outputPath: output, format: req.format ?? "wav")
        writeResponse(Response(ok: true, text: output, error: nil, dataBase64: nil, capabilities: nil))
    } catch {
        writeResponse(Response(ok: false, text: nil, error: error.localizedDescription, dataBase64: nil, capabilities: nil))
    }
case "tts":
    guard let text = req.text, let output = req.outputPath else {
        writeResponse(Response(ok: false, text: nil, error: "text and output_path required", dataBase64: nil, capabilities: nil))
        exit(1)
    }
    do {
        let data = try synthesizeSpeech(
            text: text,
            voice: req.voice,
            locale: req.locale ?? "zh-CN",
            outputPath: output
        )
        writeResponse(Response(
            ok: true,
            text: nil,
            error: nil,
            dataBase64: data.base64EncodedString(),
            capabilities: nil
        ))
    } catch {
        writeResponse(Response(ok: false, text: nil, error: error.localizedDescription, dataBase64: nil, capabilities: nil))
    }
case "capabilities":
    let caps = buildCapabilities(helperPath: helperPath)
    writeResponse(Response(
        ok: true,
        text: nil,
        error: nil,
        dataBase64: nil,
        capabilities: CapabilitiesResponse(
            stt: caps.stt,
            ocr: caps.ocr,
            tts: caps.tts,
            notify: caps.notify,
            keychain: caps.keychain,
            pasteboard: caps.pasteboard,
            platform: caps.platform,
            helperPath: caps.helperPath,
            speechAuthorized: caps.speechAuthorized,
            microphoneAuthorized: caps.microphoneAuthorized
        )
    ))
case "notify":
    guard let title = req.title, let body = req.body else {
        writeResponse(Response(ok: false, text: nil, error: "title and body required", dataBase64: nil, capabilities: nil))
        exit(1)
    }
    do {
        try postUserNotification(title: title, body: body)
        writeResponse(Response(ok: true, text: nil, error: nil, dataBase64: nil, capabilities: nil))
    } catch {
        writeResponse(Response(ok: false, text: nil, error: error.localizedDescription, dataBase64: nil, capabilities: nil))
    }
case "keychain_get":
    guard let service = req.service, let account = req.account else {
        writeResponse(Response(ok: false, text: nil, error: "service and account required", dataBase64: nil, capabilities: nil))
        exit(1)
    }
    do {
        let value = try keychainGet(service: service, account: account)
        writeResponse(Response(ok: true, text: value, error: nil, dataBase64: nil, capabilities: nil))
    } catch {
        writeResponse(Response(ok: false, text: nil, error: error.localizedDescription, dataBase64: nil, capabilities: nil))
    }
case "keychain_set":
    guard let service = req.service, let account = req.account, let secret = req.secret else {
        writeResponse(Response(ok: false, text: nil, error: "service, account, and secret required", dataBase64: nil, capabilities: nil))
        exit(1)
    }
    do {
        try keychainSet(service: service, account: account, secret: secret)
        writeResponse(Response(ok: true, text: nil, error: nil, dataBase64: nil, capabilities: nil))
    } catch {
        writeResponse(Response(ok: false, text: nil, error: error.localizedDescription, dataBase64: nil, capabilities: nil))
    }
case "pasteboard_read":
    do {
        let items = try readPasteboardItems()
        let json = try encodePasteboardItems(items)
        writeResponse(Response(ok: true, text: json, error: nil, dataBase64: nil, capabilities: nil))
    } catch {
        writeResponse(Response(ok: false, text: nil, error: error.localizedDescription, dataBase64: nil, capabilities: nil))
    }
default:
    writeResponse(Response(ok: false, text: nil, error: "unknown op: \(req.op)", dataBase64: nil, capabilities: nil))
    exit(1)
}
