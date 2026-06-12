import Foundation

/// Pump the main run loop until `condition` is true (CLI tools have no active run loop by default).
func pumpRunLoopUntil(
    _ condition: @escaping () -> Bool,
    timeout: TimeInterval
) throws {
    let deadline = Date().addingTimeInterval(timeout)
    while !condition() {
        if Date() > deadline {
            throw RunLoopWaitError.timedOut
        }
        RunLoop.main.run(mode: .default, before: Date().addingTimeInterval(0.05))
    }
}

enum RunLoopWaitError: LocalizedError {
    case timedOut

    var errorDescription: String? {
        switch self {
        case .timedOut:
            return "Operation timed out"
        }
    }
}
