// swift-tools-version:5.5
import PackageDescription

let package = Package(
    name: "EdaxCore",
    products: [
        .library(name: "EdaxCore", targets: ["EdaxCore"]),
    ],
    targets: [
        .target(name: "EdaxCore", path: "Sources/EdaxCore"),
        .testTarget(name: "EdaxCoreTests", dependencies: ["EdaxCore"], path: "Tests/EdaxCoreTests"),
    ]
)
