import Foundation
import Security

enum KeychainError: LocalizedError {
    case readFailed(OSStatus)
    case writeFailed(OSStatus)

    var errorDescription: String? {
        switch self {
        case .readFailed(let status):
            return "Keychain read failed (status \(status))"
        case .writeFailed(let status):
            return "Keychain write failed (status \(status))"
        }
    }
}

func keychainGet(service: String, account: String) throws -> String? {
    let query: [String: Any] = [
        kSecClass as String: kSecClassGenericPassword,
        kSecAttrService as String: service,
        kSecAttrAccount as String: account,
        kSecReturnData as String: true,
        kSecMatchLimit as String: kSecMatchLimitOne,
    ]
    var item: CFTypeRef?
    let status = SecItemCopyMatching(query as CFDictionary, &item)
    if status == errSecItemNotFound { return nil }
    if status != errSecSuccess { throw KeychainError.readFailed(status) }
    guard let data = item as? Data, let text = String(data: data, encoding: .utf8) else {
        return nil
    }
    return text
}

func keychainSet(service: String, account: String, secret: String) throws {
    let data = Data(secret.utf8)
    let query: [String: Any] = [
        kSecClass as String: kSecClassGenericPassword,
        kSecAttrService as String: service,
        kSecAttrAccount as String: account,
    ]
    let attrs: [String: Any] = [kSecValueData as String: data]
    let update = SecItemUpdate(query as CFDictionary, attrs as CFDictionary)
    if update == errSecSuccess { return }
    if update != errSecItemNotFound { throw KeychainError.writeFailed(update) }

    var addQuery = query
    addQuery[kSecValueData as String] = data
    let add = SecItemAdd(addQuery as CFDictionary, nil)
    if add != errSecSuccess { throw KeychainError.writeFailed(add) }
}
