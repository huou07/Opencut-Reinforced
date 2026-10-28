import Cocoa
import FlutterMacOS
import XCTest

@testable import or_app

final class RunnerTests: XCTestCase {
  /// The microphone methods carry no keychain credential. Decoding every call
  /// through the credential check first made `recordPermission` answer
  /// `INVALID_ACCOUNT`, so macOS never received a TCC request and the app never
  /// appeared under Privacy & Security → Microphone.
  func testRecordingMethodsDoNotNeedACredentialAccount() {
    XCTAssertEqual(
      NarrationRoute.decode("recordPermission", [:]), .recordPermission)
    XCTAssertEqual(
      NarrationRoute.decode("recordPermissionRequest", [:]),
      .recordPermissionRequest)
    XCTAssertEqual(
      NarrationRoute.decode("recordPermissionSettings", [:]),
      .recordPermissionSettings)
    XCTAssertEqual(NarrationRoute.decode("recordStop", [:]), .recordStop)
    XCTAssertEqual(
      NarrationRoute.decode("recordStart", ["path": "/tmp/take.wav"]),
      .recordStart("/tmp/take.wav"))
  }

  func testRecordingWithoutAPathIsRejected() {
    XCTAssertEqual(NarrationRoute.decode("recordStart", [:]), .invalidPath)
    XCTAssertEqual(
      NarrationRoute.decode("recordStart", ["path": ""]), .invalidPath)
  }

  func testKeychainMethodsStillNeedAnAccount() {
    XCTAssertEqual(NarrationRoute.decode("keychainGet", [:]), .invalidAccount)
    XCTAssertEqual(
      NarrationRoute.decode("keychainGet", ["account": ""]), .invalidAccount)
    XCTAssertEqual(
      NarrationRoute.decode("keychainGet", ["account": String(repeating: "a", count: 129)]),
      .invalidAccount)
    XCTAssertEqual(
      NarrationRoute.decode("keychainSet", ["account": "elevenlabs"]),
      .keychainSet("elevenlabs"))
  }

  func testUnknownMethodStaysUnimplemented() {
    XCTAssertEqual(NarrationRoute.decode("nope", [:]), .notImplemented)
  }

  /// The dispatched call answers with a status string, never an error object.
  func testPermissionCallAnswersAStatus() {
    let bridge = NarrationBridge()
    let answered = expectation(description: "permission state")
    var value: Any?
    bridge.handle(
      FlutterMethodCall(methodName: "recordPermission", arguments: nil)
    ) { result in
      value = result
      answered.fulfill()
    }
    waitForExpectations(timeout: 5)
    XCTAssertTrue(
      value is String,
      "expected a status string, got \(String(describing: value))")
  }
}
