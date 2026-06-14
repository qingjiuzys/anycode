import Foundation
import UserNotifications

enum NotifyError: LocalizedError {
    case permissionDenied
    case deliveryFailed(String)

    var errorDescription: String? {
        switch self {
        case .permissionDenied:
            return "Notification permission denied — enable in System Settings → Notifications"
        case .deliveryFailed(let reason):
            return "Notification failed: \(reason)"
        }
    }
}

func postUserNotification(title: String, body: String) throws {
    let center = UNUserNotificationCenter.current()
    var granted = false
    var done = false
    center.requestAuthorization(options: [.alert, .sound]) { ok, _ in
        granted = ok
        done = true
    }
    try pumpRunLoopUntil({ done }, timeout: 30)
    guard granted else { throw NotifyError.permissionDenied }

    let content = UNMutableNotificationContent()
    content.title = title
    content.body = body
    content.sound = .default

    let request = UNNotificationRequest(
        identifier: UUID().uuidString,
        content: content,
        trigger: nil
    )

    var posted = false
    var postError: Error?
    center.add(request) { err in
        postError = err
        posted = true
    }
    try pumpRunLoopUntil({ posted }, timeout: 30)
    if let postError { throw NotifyError.deliveryFailed(postError.localizedDescription) }
}
