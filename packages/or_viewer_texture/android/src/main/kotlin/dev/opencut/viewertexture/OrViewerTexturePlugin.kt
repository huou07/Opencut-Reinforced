package dev.opencut.viewertexture

import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.Rect
import android.net.Uri
import android.os.Handler
import android.os.Looper
import android.view.Surface
import io.flutter.embedding.engine.plugins.FlutterPlugin
import io.flutter.plugin.common.MethodCall
import io.flutter.plugin.common.MethodChannel
import io.flutter.view.TextureRegistry
import java.util.concurrent.CountDownLatch
import java.util.concurrent.Executors
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicBoolean

class OrViewerTexturePlugin : FlutterPlugin, MethodChannel.MethodCallHandler {
    private val mainHandler = Handler(Looper.getMainLooper())
    private val worker = Executors.newSingleThreadExecutor()
    private val workPending = AtomicBoolean(false)
    private val pendingFrameResults = mutableListOf<MethodChannel.Result>()
    private var frameRequestedWhilePending = false
    private val attached = AtomicBoolean(false)
    private val surfaceAvailable = AtomicBoolean(false)
    private var channel: MethodChannel? = null
    private var producer: TextureRegistry.SurfaceProducer? = null
    private var bitmap: Bitmap? = null
    private var registeredMediaSources: List<String>? = null
    private var nativeLoaded = false

    override fun onAttachedToEngine(binding: FlutterPlugin.FlutterPluginBinding) {
        attached.set(true)
        producerContext = binding.applicationContext
        val surfaceProducer = binding.textureRegistry.createSurfaceProducer()
        producer = surfaceProducer
        surfaceProducer.setCallback(object : TextureRegistry.SurfaceProducer.Callback {
            override fun onSurfaceAvailable() {
                surfaceAvailable.set(true)
                scheduleFrame(null)
            }

            override fun onSurfaceCleanup() {
                surfaceAvailable.set(false)
            }
        })
        channel = MethodChannel(binding.binaryMessenger, "or_viewer_texture").also {
            it.setMethodCallHandler(this)
        }
        try {
            System.loadLibrary("or_viewer_texture_jni")
            nativeLoaded = true
        } catch (_: UnsatisfiedLinkError) {
            // The texture remains unavailable if the native presenter did not load.
        }
    }

    override fun onDetachedFromEngine(binding: FlutterPlugin.FlutterPluginBinding) {
        attached.set(false)
        surfaceAvailable.set(false)
        channel?.setMethodCallHandler(null)
        channel = null
        if (nativeLoaded) nativeClearMediaFds()
        nativeLoaded = false
        registeredMediaSources = null
        producer?.release()
        producer = null
        bitmap?.recycle()
        bitmap = null
        producerContext = null
        worker.shutdownNow()
    }

    override fun onMethodCall(call: MethodCall, result: MethodChannel.Result) {
        when (call.method) {
            "textureId" -> result.success(producer?.id())
            "frameAvailable" -> scheduleFrame(result)
            "setMediaSources" -> setMediaSources(call, result)
            "clearMediaSources" -> {
                if (nativeLoaded) nativeClearMediaFds()
                registeredMediaSources = null
                result.success(true)
            }
            else -> result.notImplemented()
        }
    }

    private fun setMediaSources(call: MethodCall, result: MethodChannel.Result) {
        if (!nativeLoaded) {
            return result.error("MEDIA_SOURCE_UNAVAILABLE", "Media access is unavailable.", null)
        }
        val uris = call.arguments as? List<*> ?: return result.error(
            "INVALID_MEDIA_SOURCES", "The media source list is invalid.", null,
        )
        if (uris.size > MAX_MEDIA_SOURCES || uris.any {
                it !is String || it.length > MAX_MEDIA_SOURCE_URI_CHARS
            }) {
            return result.error(
                "INVALID_MEDIA_SOURCES", "The media source list exceeds its limit.", null,
            )
        }
        val sources = uris.filterIsInstance<String>()
        if (registeredMediaSources == sources) {
            result.success(true)
            return
        }

        val context = producerContext
            ?: return result.error("MEDIA_SOURCE_UNAVAILABLE", "Media access is unavailable.", null)
        try {
            worker.execute {
                val available = try {
                    nativeClearMediaFds()
                    for (source in sources) {
                        if (!source.startsWith("content://")) continue
                        try {
                            val uri = Uri.parse(source)
                            val descriptor = context.contentResolver.openFileDescriptor(uri, "r")
                                ?: continue
                            descriptor.use { nativeRegisterMediaFd(source, it.fd) }
                        } catch (_: Exception) {
                            // Unavailable or nonseekable media falls back to the preview error path.
                        }
                    }
                    true
                } catch (_: Exception) {
                    nativeClearMediaFds()
                    false
                }
                mainHandler.post {
                    if (!attached.get()) return@post
                    registeredMediaSources = if (available) sources else null
                    if (available) {
                        result.success(true)
                    } else {
                        result.error(
                            "MEDIA_SOURCE_UNAVAILABLE",
                            "A media source could not be opened for preview.",
                            null,
                        )
                    }
                }
            }
        } catch (_: RuntimeException) {
            result.error("MEDIA_SOURCE_UNAVAILABLE", "Media access is unavailable.", null)
        }
    }

    private var producerContext: android.content.Context? = null

    private fun scheduleFrame(result: MethodChannel.Result?) {
        if (!attached.get()) {
            result?.success(false)
            return
        }

        result?.let(pendingFrameResults::add)
        if (!workPending.compareAndSet(false, true)) {
            frameRequestedWhilePending = true
            return
        }

        presentScheduledFrame(allowFollowUp = true)
    }

    private fun presentScheduledFrame(allowFollowUp: Boolean) {
        try {
            worker.execute {
                val presented = try {
                    presentLatestFrame()
                } catch (_: Exception) {
                    false
                }
                mainHandler.post {
                    if (!attached.get()) {
                        frameRequestedWhilePending = false
                        pendingFrameResults.clear()
                        workPending.set(false)
                    } else if (!presented && allowFollowUp && frameRequestedWhilePending) {
                        frameRequestedWhilePending = false
                        presentScheduledFrame(allowFollowUp = false)
                    } else {
                        completeFrameResults(presented)
                    }
                }
            }
        } catch (_: RuntimeException) {
            completeFrameResults(false)
        }
    }

    private fun completeFrameResults(presented: Boolean) {
        frameRequestedWhilePending = false
        workPending.set(false)
        val results = pendingFrameResults.toList()
        pendingFrameResults.clear()
        results.forEach { it.success(presented) }
    }

    private fun presentLatestFrame(): Boolean {
        if (!nativeLoaded) return false
        val frame = nativeAcquireLatest() ?: return false
        try {
            if (!surfaceAvailable.get() || !attached.get()) return false
            val surface = surfaceFor(frame.width, frame.height) ?: return false
            val target = bitmap?.takeIf {
                it.width == frame.width && it.height == frame.height && !it.isRecycled
            } ?: Bitmap.createBitmap(frame.width, frame.height, Bitmap.Config.ARGB_8888).also {
                bitmap?.recycle()
                bitmap = it
            }
            val source = frame.pixels.duplicate().apply { position(0) }
            target.copyPixelsFromBuffer(source)
            val canvas = surface.lockCanvas(null)
            try {
                canvas.drawBitmap(
                    target,
                    null,
                    Rect(0, 0, canvas.width, canvas.height),
                    null,
                )
            } finally {
                surface.unlockCanvasAndPost(canvas)
            }
            return true
        } finally {
            nativeReleaseFrame(frame.releaseContext)
        }
    }

    private fun surfaceFor(width: Int, height: Int): Surface? {
        val latch = CountDownLatch(1)
        var surface: Surface? = null
        mainHandler.post {
            try {
                if (attached.get() && surfaceAvailable.get()) {
                    producer?.let {
                        it.setSize(width, height)
                        surface = it.surface
                    }
                }
            } finally {
                latch.countDown()
            }
        }
        if (!latch.await(2, TimeUnit.SECONDS)) return null
        return surface?.takeIf { it.isValid }
    }

    private external fun nativeAcquireLatest(): AndroidViewerFrame?
    private external fun nativeReleaseFrame(releaseContext: Long)
    private external fun nativeRegisterMediaFd(uri: String, fd: Int): Boolean
    private external fun nativeClearMediaFds()

    companion object {
        private const val MAX_MEDIA_SOURCES = 64
        private const val MAX_MEDIA_SOURCE_URI_CHARS = 8192
    }
}
