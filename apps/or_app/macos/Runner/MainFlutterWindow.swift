import Cocoa
import FlutterMacOS
import UniformTypeIdentifiers

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
