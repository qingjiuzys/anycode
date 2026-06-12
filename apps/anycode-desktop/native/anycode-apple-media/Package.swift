// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "anycode-apple-media",
    platforms: [.macOS(.v13)],
    products: [
        .executable(name: "anycode-apple-media", targets: ["AnycodeAppleMedia"]),
    ],
    targets: [
        .executableTarget(
            name: "AnycodeAppleMedia",
            path: "Sources/AnycodeAppleMedia"
        ),
    ]
)
