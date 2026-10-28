import Cocoa
import FlutterMacOS
import UniformTypeIdentifiers
import Security
import AVFoundation

class MainFlutterWindow: NSWindow {
  override func awakeFromNib() {
    let flutterViewController = FlutterViewController()
    let windowFrame = self.frame
    self.contentViewController = flutterViewController
    self.setFrame(windowFrame, display: true)

    // The editor needs room for the media panel, the viewer and the inspector
    // side by side. Below this size the desktop layout previously clipped the
    // workspace actions and the timeline toolbar instead of laying them out
    // (measured 2026-10-25 at 1024x700 and 800x600). Keep the window resizable
    // but never smaller than the layout is allowed to be.
    self.contentMinSize = NSSize(width: 1024, height: 680)
    self.setFrameAutosaveName("ORMainWindow")
    // The xib still carries the template's 800x600 content rect. Enlarge only
    // that untouched first-run frame; a remembered user size is left alone
    // (and still bounded by contentMinSize).
    if abs(self.frame.width - 800) < 1, abs(self.frame.height - 600) < 1 {
      self.setContentSize(NSSize(width: 1440, height: 900))
      self.center()
    }

    RegisterGeneratedPlugins(registry: flutterViewController)
    NarrationBridge.register(
      with: flutterViewController.registrar(forPlugin: "NarrationBridge"))
    MacOSScopedFileAccessPlugin.register(
      with: flutterViewController.registrar(forPlugin: "MacOSScopedFileAccessPlugin"))

    super.awakeFromNib()
  }
}

private final class MacOSScopedFileAccessPlugin: NSObject, FlutterPlugin {
  private static let channelName = "or_macos_file_access"
  private let bookmarks = PersistentScopedBookmarks()

  static func register(with registrar: FlutterPluginRegistrar) {
    let plugin = MacOSScopedFileAccessPlugin()
    registrar.addMethodCallDelegate(
      plugin,
      channel: FlutterMethodChannel(
        name: channelName,
        binaryMessenger: registrar.messenger
      )
    )
    _ = try? plugin.bookmarks.restore()
  }

  func handle(_ call: FlutterMethodCall, result: @escaping FlutterResult) {
    switch call.method {
    case "openProject":
      openPanel(
        canChooseDirectories: false,
        allowsMultipleSelection: false,
        contentTypes: [projectType],
        bookmarkParentDirectory: true,
        result: result
      )
    case "openMedia":
      openPanel(
        canChooseDirectories: false,
        allowsMultipleSelection: true,
        contentTypes: nil,
        bookmarkParentDirectory: false,
        result: result
      )
    case "openMediaFolder":
      openPanel(
        canChooseDirectories: true,
        allowsMultipleSelection: false,
        contentTypes: nil,
        bookmarkParentDirectory: false,
        result: result
      )
    case "revealExport":
      _ = call.arguments as? [String: Any]
      guard let arguments = call.arguments as? [String: Any],
            let path = arguments["path"] as? String else {
        result(FlutterError(
          code: "INVALID_ARGUMENTS",
          message: "Revealing a file needs a path.",
          details: nil
        ))
        return
      }
      let url = URL(fileURLWithPath: path)
      guard FileManager.default.fileExists(atPath: path) else {
        result(FlutterError(
          code: "MISSING_FILE",
          message: "That file is no longer on disk.",
          details: path
        ))
        return
      }
      NSWorkspace.shared.activateFileViewerSelecting([url])
      result(true)
    case "saveProject":
      guard let arguments = call.arguments as? [String: Any],
            let suggestedName = arguments["suggestedName"] as? String else {
        result(FlutterError(
          code: "INVALID_ARGUMENTS",
          message: "Saving a project needs a suggested name.",
          details: nil
        ))
        return
      }
      let panel = NSOpenPanel()
      panel.canChooseDirectories = true
      panel.canChooseFiles = false
      panel.allowsMultipleSelection = false
      panel.prompt = "Choose Folder"
      panel.message = "Choose where to save \(suggestedName)."
      panel.begin { response in
        guard response == .OK, let folder = panel.url else {
          result(nil)
          return
        }
        do {
          try self.bookmarks.store(urls: [folder])
          result(folder.appendingPathComponent(suggestedName).path)
        } catch {
          result(FlutterError(
            code: "BOOKMARK_STORE_FAILED",
            message: "macOS could not retain access to the selected project folder.",
            details: nil
          ))
        }
      }
    case "saveExport":
      guard let arguments = call.arguments as? [String: Any],
            let suggestedName = arguments["suggestedName"] as? String,
            !suggestedName.isEmpty,
            URL(fileURLWithPath: suggestedName).lastPathComponent == suggestedName,
            suggestedName != ".",
            suggestedName != ".." else {
        result(FlutterError(
          code: "INVALID_ARGUMENTS",
          message: "Saving an export needs a valid file name.",
          details: nil
        ))
        return
      }
      chooseExportFolder(suggestedName: suggestedName, result: result)
    case "restoreBookmarks":
      do {
        result(try bookmarks.restore())
      } catch {
        result(FlutterError(
          code: "BOOKMARK_RESTORE_FAILED",
          message: "Previously selected files could not be reopened.",
          details: nil
        ))
      }
    default:
      result(FlutterMethodNotImplemented)
    }
  }

  private func chooseExportFolder(
    suggestedName: String,
    result: @escaping FlutterResult
  ) {
    let panel = NSOpenPanel()
    panel.canChooseDirectories = true
    panel.canChooseFiles = false
    panel.allowsMultipleSelection = false
    panel.prompt = "Export Here"
    panel.message = "Save \(suggestedName) in the selected folder."
    panel.begin { response in
      guard response == .OK, let folder = panel.url else {
        result(nil)
        return
      }
      let destination = folder.appendingPathComponent(suggestedName)
      var isDirectory = ObjCBool(false)
      if FileManager.default.fileExists(atPath: destination.path, isDirectory: &isDirectory) {
        guard !isDirectory.boolValue else {
          result(FlutterError(
            code: "EXPORT_DESTINATION_INVALID",
            message: "The export file name is already used by a folder.",
            details: nil
          ))
          return
        }
        let alert = NSAlert()
        alert.messageText = "Replace existing export?"
        alert.informativeText = "\(suggestedName) already exists in this folder."
        alert.addButton(withTitle: "Replace")
        alert.addButton(withTitle: "Cancel")
        guard alert.runModal() == .alertFirstButtonReturn else {
          result(nil)
          return
        }
      }
      do {
        try self.bookmarks.store(urls: [folder])
        result(destination.path)
      } catch {
        result(FlutterError(
          code: "BOOKMARK_STORE_FAILED",
          message: "macOS could not retain access to the selected export folder.",
          details: nil
        ))
      }
    }
  }

  private func openPanel(
    canChooseDirectories: Bool,
    allowsMultipleSelection: Bool,
    contentTypes: [UTType]?,
    bookmarkParentDirectory: Bool,
    result: @escaping FlutterResult
  ) {
    let panel = NSOpenPanel()
    panel.canChooseDirectories = canChooseDirectories
    panel.canChooseFiles = !canChooseDirectories
    panel.allowsMultipleSelection = allowsMultipleSelection
    if let contentTypes {
      panel.allowedContentTypes = contentTypes
    }
    panel.begin { response in
      guard response == .OK else {
        result(allowsMultipleSelection ? [] : nil)
        return
      }
      let urls = panel.urls
      let bookmarkURLs = bookmarkParentDirectory
        ? urls.map { $0.deletingLastPathComponent() }
        : urls
      do {
        try self.bookmarks.store(urls: bookmarkURLs)
        result(allowsMultipleSelection ? urls.map(\.path) : urls.first?.path)
      } catch {
        if bookmarkParentDirectory, let projectURL = urls.first {
          self.chooseProjectFolder(for: projectURL, result: result)
        } else {
          result(FlutterError(
            code: "BOOKMARK_STORE_FAILED",
            message: "macOS could not retain access to the selected file or folder.",
            details: nil
          ))
        }
      }
    }
  }

  private func chooseProjectFolder(
    for projectURL: URL,
    result: @escaping FlutterResult
  ) {
    let expectedFolder = projectURL.deletingLastPathComponent().standardizedFileURL
    let panel = NSOpenPanel()
    panel.canChooseDirectories = true
    panel.canChooseFiles = false
    panel.allowsMultipleSelection = false
    panel.directoryURL = expectedFolder
    panel.prompt = "Allow Access"
    panel.message = "Select the folder containing \(projectURL.lastPathComponent) so OR can save it safely."
    panel.begin { response in
      guard response == .OK, let folder = panel.url else {
        result(nil)
        return
      }
      guard folder.standardizedFileURL.path == expectedFolder.path else {
        result(FlutterError(
          code: "PROJECT_FOLDER_MISMATCH",
          message: "Choose the folder that contains the selected project.",
          details: nil
        ))
        return
      }
      do {
        try self.bookmarks.store(urls: [folder])
        result(projectURL.path)
      } catch {
        result(FlutterError(
          code: "BOOKMARK_STORE_FAILED",
          message: "macOS could not retain access to the selected project folder.",
          details: nil
        ))
      }
    }
  }

  private var projectType: UTType {
    UTType(filenameExtension: "orproj") ?? .data
  }
}

private final class PersistentScopedBookmarks {
  private let fileManager = FileManager.default
  private var activeURLs: [String: URL] = [:]

  deinit {
    for url in activeURLs.values {
      url.stopAccessingSecurityScopedResource()
    }
  }

  func store(urls: [URL]) throws {
    guard urls.count <= 1024 else {
      throw BookmarkError.tooManyPaths
    }
    var entries = try loadEntries()
    var startedURLs: [String: URL] = [:]
    do {
      for url in urls {
        let key = url.standardizedFileURL.path
        guard fileManager.fileExists(atPath: url.path) else {
          throw BookmarkError.missingPath
        }
        if isInAppContainer(url) {
          continue
        }
        if activeURLs[key] != nil {
          continue
        }
        let data = try url.bookmarkData(
          options: [.withSecurityScope],
          includingResourceValuesForKeys: nil,
          relativeTo: nil
        )
        entries[key] = data.base64EncodedString()
        guard url.startAccessingSecurityScopedResource() else {
          throw BookmarkError.accessUnavailable
        }
        startedURLs[key] = url
      }
      try saveEntries(entries)
    } catch {
      for url in startedURLs.values {
        url.stopAccessingSecurityScopedResource()
      }
      throw error
    }
    activeURLs.merge(startedURLs) { _, new in new }
  }

  @discardableResult
  func restore() throws -> Int {
    var entries = try loadEntries()
    var restoredCount = 0
    var refreshedEntries = false
    for (path, encoded) in entries {
      if activeURLs[path] != nil {
        continue
      }
      guard let data = Data(base64Encoded: encoded) else {
        continue
      }
      var isStale = false
      guard let url = try? URL(
        resolvingBookmarkData: data,
        options: [.withSecurityScope],
        relativeTo: nil,
        bookmarkDataIsStale: &isStale
      ) else {
        continue
      }
      guard url.startAccessingSecurityScopedResource() else {
        continue
      }
      activeURLs[path] = url
      restoredCount += 1
      if isStale,
         let refreshed = try? url.bookmarkData(
           options: [.withSecurityScope],
           includingResourceValuesForKeys: nil,
           relativeTo: nil
         ) {
        entries[path] = refreshed.base64EncodedString()
        refreshedEntries = true
      }
    }
    if refreshedEntries {
      try saveEntries(entries)
    }
    return restoredCount
  }

  private func isInAppContainer(_ url: URL) -> Bool {
    guard let supportURL = fileManager.urls(
      for: .applicationSupportDirectory,
      in: .userDomainMask
    ).first else {
      return false
    }
    let containerRoot = supportURL
      .deletingLastPathComponent()
      .deletingLastPathComponent()
      .standardizedFileURL
      .path
    return url.path == containerRoot || url.path.hasPrefix(containerRoot + "/")
  }

  private func entriesURL() throws -> URL {
    guard let supportURL = fileManager.urls(
      for: .applicationSupportDirectory,
      in: .userDomainMask
    ).first else {
      throw BookmarkError.storageUnavailable
    }
    return supportURL
      .appendingPathComponent("Opencut Reinforced", isDirectory: true)
      .appendingPathComponent("macos-file-bookmarks.v1.json")
  }

  private func loadEntries() throws -> [String: String] {
    let url = try entriesURL()
    guard fileManager.fileExists(atPath: url.path) else {
      return [:]
    }
    let data = try Data(contentsOf: url, options: .mappedIfSafe)
    guard data.count <= 4 * 1024 * 1024,
          let object = try JSONSerialization.jsonObject(with: data) as? [String: Any],
          object["version"] as? Int == 1,
          let entries = object["bookmarks"] as? [String: String],
          entries.count <= 4096 else {
      throw BookmarkError.invalidStore
    }
    return entries
  }

  private func saveEntries(_ entries: [String: String]) throws {
    let url = try entriesURL()
    try fileManager.createDirectory(
      at: url.deletingLastPathComponent(),
      withIntermediateDirectories: true
    )
    let data = try JSONSerialization.data(
      withJSONObject: ["version": 1, "bookmarks": entries],
      options: [.sortedKeys]
    )
    try data.write(to: url, options: .atomic)
  }

  private enum BookmarkError: Error {
    case tooManyPaths
    case missingPath
    case accessUnavailable
    case storageUnavailable
    case invalidStore
  }
}


/// One decoded `or_narration` call.
///
/// Recording methods carry no credential, so the keychain account check belongs
/// to the keychain methods only.
enum NarrationRoute: Equatable {
  case keychainGet(String)
  case keychainSet(String)
  case keychainDelete(String)
  case keychainHas(String)
  case recordPermission
  case recordPermissionRequest
  case recordPermissionSettings
  case recordStart(String)
  case recordStop
  case invalidAccount
  case invalidPath
  case notImplemented

  /// Decodes one call. Validation is per method: only the keychain methods need
  /// a credential account, so a recording call is never rejected for lacking one.
  static func decode(_ method: String, _ arguments: [String: Any]) -> NarrationRoute {
    switch method {
    case "keychainGet":
      return credential(arguments).map(NarrationRoute.keychainGet) ?? .invalidAccount
    case "keychainSet":
      return credential(arguments).map(NarrationRoute.keychainSet) ?? .invalidAccount
    case "keychainDelete":
      return credential(arguments).map(NarrationRoute.keychainDelete) ?? .invalidAccount
    case "keychainHas":
      return credential(arguments).map(NarrationRoute.keychainHas) ?? .invalidAccount
    case "recordPermission":
      return .recordPermission
    case "recordPermissionRequest":
      return .recordPermissionRequest
    case "recordPermissionSettings":
      return .recordPermissionSettings
    case "recordStart":
      guard let path = arguments["path"] as? String, !path.isEmpty else {
        return .invalidPath
      }
      return .recordStart(path)
    case "recordStop":
      return .recordStop
    default:
      return .notImplemented
    }
  }

  private static func credential(_ arguments: [String: Any]) -> String? {
    guard let account = arguments["account"] as? String,
          !account.isEmpty,
          account.count <= 128 else {
      return nil
    }
    return account
  }
}

/// Keychain-backed narration credentials and microphone capture.
///
/// API keys never enter the project file, a preferences file or the Dart layer:
/// Dart can write, delete and ask whether an account is configured, but it reads
/// the secret back only through this channel when a request is made. The item is
/// device-only and available when unlocked.
///
/// Microphone capture asks macOS for TCC access before the first take, so the app
/// appears under Privacy & Security → Microphone, and reports the grant state back
/// so the UI can send the user there after a denial.
final class NarrationBridge: NSObject, FlutterPlugin {
  private static let channelName = "or_narration"
  private static let service = "io.github.huou07.orApp.narration"
  /// System Settings moved its privacy panes; keep the current URL first and the
  /// pre-Ventura form as a fallback so a denial can always be reversed.
  private static let microphoneSettingsURLs = [
    "x-apple.systempreferences:com.apple.settings.PrivacySecurity.extension?Privacy_Microphone",
    "x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone",
  ]
  private var recorder: AVAudioRecorder?

  static func register(with registrar: FlutterPluginRegistrar) {
    registrar.addMethodCallDelegate(
      NarrationBridge(),
      channel: FlutterMethodChannel(
        name: channelName,
        binaryMessenger: registrar.messenger
      )
    )
  }

  func handle(_ call: FlutterMethodCall, result: @escaping FlutterResult) {
    let arguments = call.arguments as? [String: Any] ?? [:]
    dispatch(NarrationRoute.decode(call.method, arguments), arguments, result)
  }

  func dispatch(
    _ route: NarrationRoute,
    _ arguments: [String: Any],
    _ result: @escaping FlutterResult
  ) {
    switch route {
    case .keychainGet(let account):
      result(read(account))
    case .keychainSet(let account):
      guard let value = arguments["value"] as? String, !value.isEmpty else {
        result(FlutterError(
          code: "INVALID_SECRET",
          message: "The key is empty.",
          details: nil
        ))
        return
      }
      do {
        try write(account, value)
        result(true)
      } catch {
        result(FlutterError(
          code: "KEYCHAIN_WRITE_FAILED",
          message: "macOS could not store this key.",
          details: nil
        ))
      }
    case .keychainDelete(let account):
      delete(account)
      result(true)
    case .keychainHas(let account):
      result(read(account)?.isEmpty == false)
    case .recordPermission:
      result(Self.permissionState())
    case .recordPermissionRequest:
      requestPermission(result)
    case .recordPermissionSettings:
      openMicrophoneSettings(result)
    case .recordStart(let path):
      startRecording(path, result: result)
    case .recordStop:
      stopRecording(result: result)
    case .invalidAccount:
      result(FlutterError(
        code: "INVALID_ACCOUNT",
        message: "A narration credential needs a short account name.",
        details: nil
      ))
    case .invalidPath:
      result(FlutterError(
        code: "INVALID_PATH",
        message: "Recording needs an output path.",
        details: nil
      ))
    case .notImplemented:
      result(FlutterMethodNotImplemented)
    }
  }

  /// `granted`, `denied`, `restricted`, `undetermined` or `unknown`.
  private static func permissionState() -> String {
    switch AVCaptureDevice.authorizationStatus(for: .audio) {
    case .authorized:
      return "granted"
    case .denied:
      return "denied"
    case .restricted:
      return "restricted"
    case .notDetermined:
      return "undetermined"
    @unknown default:
      return "unknown"
    }
  }

  /// Presents the macOS microphone prompt the first time, then reports the state.
  ///
  /// `requestAccess` is what registers the app in Privacy & Security; reading the
  /// status alone never does. A decided state is reported without asking again.
  private func requestPermission(_ result: @escaping FlutterResult) {
    guard AVCaptureDevice.authorizationStatus(for: .audio) == .notDetermined else {
      result(Self.permissionState())
      return
    }
    AVCaptureDevice.requestAccess(for: .audio) { _ in
      DispatchQueue.main.async {
        result(Self.permissionState())
      }
    }
  }

  private func openMicrophoneSettings(_ result: @escaping FlutterResult) {
    for path in Self.microphoneSettingsURLs {
      if let url = URL(string: path), NSWorkspace.shared.open(url) {
        result(true)
        return
      }
    }
    result(false)
  }

  private func startRecording(_ path: String, result: @escaping FlutterResult) {
    guard AVCaptureDevice.authorizationStatus(for: .audio) == .authorized else {
      result(FlutterError(
        code: "PERMISSION_DENIED",
        message: "Microphone access is not granted. Allow Opencut Reinforced in "
          + "System Settings → Privacy & Security → Microphone, then try again.",
        details: Self.permissionState()
      ))
      return
    }
    let url = URL(fileURLWithPath: path)
    try? FileManager.default.createDirectory(
      at: url.deletingLastPathComponent(),
      withIntermediateDirectories: true
    )
    let settings: [String: Any] = [
      AVFormatIDKey: Int(kAudioFormatLinearPCM),
      AVSampleRateKey: 48_000.0,
      AVNumberOfChannelsKey: 1,
      AVLinearPCMBitDepthKey: 16,
      AVLinearPCMIsFloatKey: false,
      AVLinearPCMIsBigEndianKey: false,
    ]
    do {
      recorder?.stop()
      let next = try AVAudioRecorder(url: url, settings: settings)
      next.isMeteringEnabled = false
      guard next.prepareToRecord(), next.record() else {
        result(FlutterError(
          code: "RECORDING_FAILED",
          message: "The microphone could not start recording.",
          details: Self.permissionState()
        ))
        return
      }
      recorder = next
      result(true)
    } catch {
      result(FlutterError(
        code: "RECORDING_FAILED",
        message: "The microphone could not start recording.",
        details: error.localizedDescription
      ))
    }
  }

  private func stopRecording(result: @escaping FlutterResult) {
    guard let recorder else {
      result(FlutterError(
        code: "NOT_RECORDING",
        message: "No recording is in progress.",
        details: nil
      ))
      return
    }
    let seconds = recorder.currentTime
    recorder.stop()
    self.recorder = nil
    result(["duration_ms": Int((seconds * 1000).rounded())])
  }

  private func baseQuery(_ account: String) -> [CFString: Any] {
    [
      kSecClass: kSecClassGenericPassword,
      kSecAttrService: Self.service,
      kSecAttrAccount: account,
    ]
  }

  private func read(_ account: String) -> String? {
    var query = baseQuery(account)
    query[kSecReturnData] = true
    query[kSecMatchLimit] = kSecMatchLimitOne
    var item: CFTypeRef?
    guard SecItemCopyMatching(query as CFDictionary, &item) == errSecSuccess,
          let data = item as? Data,
          let value = String(data: data, encoding: .utf8) else {
      return nil
    }
    return value
  }

  private func write(_ account: String, _ value: String) throws {
    SecItemDelete(baseQuery(account) as CFDictionary)
    var query = baseQuery(account)
    query[kSecValueData] = Data(value.utf8)
    query[kSecAttrAccessible] = kSecAttrAccessibleWhenUnlockedThisDeviceOnly
    let status = SecItemAdd(query as CFDictionary, nil)
    guard status == errSecSuccess else {
      throw NSError(domain: NSOSStatusErrorDomain, code: Int(status))
    }
  }

  private func delete(_ account: String) {
    SecItemDelete(baseQuery(account) as CFDictionary)
  }
}
