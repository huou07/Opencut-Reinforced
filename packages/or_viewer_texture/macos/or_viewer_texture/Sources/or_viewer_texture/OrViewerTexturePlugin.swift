import CoreVideo
import Darwin
import FlutterMacOS

@usableFromInline @frozen struct OrViewerPixelBuffer {
  var pixels: UnsafePointer<UInt8>?
  var width: Int
  var height: Int
  var releaseContext: UnsafeMutableRawPointer?
}

private typealias AcquireLatest = @convention(c) (UnsafeMutableRawPointer) -> UInt8
private typealias ReleaseFrame = @convention(c) (UnsafeMutableRawPointer?) -> Void

private struct ViewerFfi {
  let acquire: AcquireLatest
  let release: ReleaseFrame
}

private final class PixelBufferRelease {
  let context: UnsafeMutableRawPointer?
  let release: ReleaseFrame

  init(context: UnsafeMutableRawPointer?, release: @escaping ReleaseFrame) {
    self.context = context
    self.release = release
  }

  func run() {
    release(context)
  }
}

private let releasePixelBufferBytes: CVPixelBufferReleaseBytesCallback = { refcon, _ in
  guard let refcon else { return }
  Unmanaged<PixelBufferRelease>.fromOpaque(refcon).takeRetainedValue().run()
}

private func viewerFfi() -> ViewerFfi? {
  for index in 0..<_dyld_image_count() {
    guard let image = _dyld_get_image_name(index),
          String(cString: image).contains("or_app_bridge"),
          let handle = dlopen(image, RTLD_NOW | RTLD_NOLOAD) else {
      continue
    }
    defer { dlclose(handle) }
    guard let acquireSymbol = dlsym(handle, "or_viewer_acquire_latest"),
          let releaseSymbol = dlsym(handle, "or_viewer_release_frame") else {
      continue
    }
    return ViewerFfi(
      acquire: unsafeBitCast(acquireSymbol, to: AcquireLatest.self),
      release: unsafeBitCast(releaseSymbol, to: ReleaseFrame.self)
    )
  }
  return nil
}

private final class OrViewerTexture: NSObject, FlutterTexture {
  func copyPixelBuffer() -> Unmanaged<CVPixelBuffer>? {
    guard let ffi = viewerFfi() else { return nil }
    let framePointer = UnsafeMutablePointer<OrViewerPixelBuffer>.allocate(capacity: 1)
    framePointer.initialize(to: OrViewerPixelBuffer(pixels: nil, width: 0, height: 0, releaseContext: nil))
    defer {
      framePointer.deinitialize(count: 1)
      framePointer.deallocate()
    }
    guard ffi.acquire(UnsafeMutableRawPointer(framePointer)) != 0 else { return nil }
    let frame = framePointer.pointee
    guard let pixels = frame.pixels, frame.width > 0, frame.height > 0,
          frame.width <= Int.max / frame.height / 4,
          frame.width * frame.height * 4 <= 256 * 1024 * 1024 else {
      ffi.release(frame.releaseContext)
      return nil
    }

    let release = PixelBufferRelease(context: frame.releaseContext, release: ffi.release)
    let refcon = Unmanaged.passRetained(release).toOpaque()
    var pixelBuffer: CVPixelBuffer?
    let status = CVPixelBufferCreateWithBytes(
      kCFAllocatorDefault,
      frame.width,
      frame.height,
      kCVPixelFormatType_32BGRA,
      UnsafeMutableRawPointer(mutating: pixels),
      frame.width * 4,
      releasePixelBufferBytes,
      refcon,
      nil,
      &pixelBuffer
    )
    guard status == kCVReturnSuccess, let pixelBuffer else {
      Unmanaged<PixelBufferRelease>.fromOpaque(refcon).takeRetainedValue().run()
      return nil
    }
    return Unmanaged.passRetained(pixelBuffer)
  }
}

public class OrViewerTexturePlugin: NSObject, FlutterPlugin {
  public static func register(with registrar: FlutterPluginRegistrar) {
    _ = registrar.textures.register(OrViewerTexture())
  }
}
