// swift-tools-version: 5.9
import PackageDescription

let package = Package(
  name: "or_viewer_texture",
  platforms: [.macOS("12.0")],
  products: [
    .library(name: "or-viewer-texture", targets: ["or_viewer_texture"])
  ],
  dependencies: [
    .package(name: "FlutterFramework", path: "../FlutterFramework")
  ],
  targets: [
    .target(
      name: "or_viewer_texture",
      dependencies: [.product(name: "FlutterFramework", package: "FlutterFramework")],
      path: "Sources/or_viewer_texture"
    )
  ]
)
