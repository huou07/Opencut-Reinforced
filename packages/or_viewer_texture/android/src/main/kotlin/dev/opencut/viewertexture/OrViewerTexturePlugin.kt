package dev.opencut.viewertexture

import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.graphics.Bitmap
import android.graphics.Rect
import android.net.Uri
import android.os.Handler
import android.os.Looper
import android.os.Process
import android.os.SystemClock
import android.system.ErrnoException
import android.system.Os
import android.system.OsConstants
import java.nio.ByteBuffer
import java.nio.ByteOrder
import io.flutter.embedding.engine.plugins.FlutterPlugin
import io.flutter.plugin.common.MethodCall
import io.flutter.plugin.common.MethodChannel
import io.flutter.view.TextureRegistry
import java.io.FileNotFoundException
import java.util.concurrent.ArrayBlockingQueue
import java.util.concurrent.ThreadPoolExecutor
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicLong

class OrViewerTexturePlugin : FlutterPlugin, MethodChannel.MethodCallHandler {
    private val mainHandler = Handler(Looper.getMainLooper())
    private val worker = object : ThreadPoolExecutor(1, 1, 0L, TimeUnit.MILLISECONDS, ArrayBlockingQueue<Runnable>(8)) {
        override fun terminated() {
            if (nativeLoaded && !nativeClearMediaFds()) android.util.Log.e("OrViewerTexture", "Native media cleanup failed during detach")
            registeredMediaSources = null
            registeredOwner = null
            bitmap?.recycle()
            bitmap = null
            nativeLoaded = false
            super.terminated()
        }
    }
    private val workPending = AtomicBoolean(false)
    private val attached = AtomicBoolean(false)
    private val surfaceEpoch = AtomicLong(0)
    private val bindingEpoch = AtomicLong(0)
    private val staleDrops = AtomicLong(0)
    private val pendingFrameResults = mutableListOf<MethodChannel.Result>()
    private var frameRequestedWhilePending = false
    private var surfaceAvailable = false // Main thread only, including callbacks and drawing.
    private var channel: MethodChannel? = null
    private var producer: TextureRegistry.SurfaceProducer? = null
    private var textureRegistry: TextureRegistry? = null
    private var producerContext: Context? = null
    @Volatile private var nativeLoaded = false
    // Worker owns the bitmap. workPending retains exclusive ownership until
    // the main draw completes, so no copy/resize/recycle can race with drawing.
    private var bitmap: Bitmap? = null
    private var bgraSwapScratch: ByteBuffer? = null
    private var registeredMediaSources: List<String>? = null
    private var registeredOwner: String? = null
    private var registrationCount = 0L
    private var presentedFrames = 0L
    private var maxMainDrawMicros = 0L
    private var totalMainDrawMicros = 0L
    // Main-thread SurfaceProducer stage timings are updated only by its draw
    // callback and returned with the existing resource snapshot.
    private var maxSurfaceResizeMicros = 0L
    private var maxCanvasLockMicros = 0L
    private var maxCanvasDrawMicros = 0L
    private var maxCanvasPostMicros = 0L
    // Worker-owned stage timings separate FFI acquisition and CPU pixel copy
    // from the SurfaceProducer main-thread draw measured above.
    private var maxFrameAcquireMicros = 0L
    private var totalFrameAcquireMicros = 0L
    private var maxBitmapCopyMicros = 0L
    private var totalBitmapCopyMicros = 0L
    private var bitmapAllocations = 0L
    private var maxBitmapAllocationMicros = 0L
    private var surfaceCreations = 0L
    private var surfaceRestorations = 0L
    private var surfaceCleanups = 0L
    private var surfaceReleases = 0L
    private var peakPendingFrameResults = 0
    private var rejectedFrameRequests = 0L
    private val peakQueuedOperations = AtomicLong(0)

    override fun onAttachedToEngine(binding: FlutterPlugin.FlutterPluginBinding) {
        attached.set(true)
        producerContext = binding.applicationContext
        textureRegistry = binding.textureRegistry
        createProducer()
        channel = MethodChannel(binding.binaryMessenger, "or_viewer_texture").also {
            it.setMethodCallHandler(this)
        }
        try {
            System.loadLibrary("or_viewer_texture_jni")
            nativeLoaded = true
        } catch (_: UnsatisfiedLinkError) {
            nativeLoaded = false
        }
    }

    private fun createProducer() {
        if (producer != null || !attached.get()) return
        producer = textureRegistry?.createSurfaceProducer()?.also { created ->
            surfaceEpoch.incrementAndGet()
            surfaceAvailable = true // Creation does not issue onSurfaceAvailable.
            surfaceCreations++
            created.setCallback(object : TextureRegistry.SurfaceProducer.Callback {
                override fun onSurfaceAvailable() {
                    surfaceEpoch.incrementAndGet()
                    surfaceAvailable = true
                    surfaceRestorations++
                    scheduleFrame(null)
                }
                override fun onSurfaceCleanup() {
                    surfaceEpoch.incrementAndGet()
                    surfaceAvailable = false
                    surfaceCleanups++
                }
            })
        }
    }

    private fun releaseProducer() {
        surfaceEpoch.incrementAndGet()
        surfaceAvailable = false
        producer?.release()
        if (producer != null) surfaceReleases++
        producer = null
    }

    override fun onDetachedFromEngine(binding: FlutterPlugin.FlutterPluginBinding) {
        attached.set(false)
        bindingEpoch.incrementAndGet()
        channel?.setMethodCallHandler(null)
        channel = null
        releaseProducer()
        textureRegistry = null
        producerContext = null
        completeFrameResults(false)
        // Orderly termination runs cleanup after every operation holding a
        // descriptor or copied bitmap. Epoch checks invalidate queued draws.
        worker.shutdown()
    }

    override fun onMethodCall(call: MethodCall, result: MethodChannel.Result) {
        when (call.method) {
            "textureId" -> { createProducer(); result.success(producer?.id()) }
            "frameAvailable" -> scheduleFrame(result)
            "setMediaSources" -> setMediaSources(call, result)
            "clearMediaSources" -> clearMediaSources(result)
            "resourceSnapshot" -> resourceSnapshot(result)
            else -> result.notImplemented()
        }
    }

    private class MediaFailure(val code: String, message: String) : Exception(message)

    private fun clearNativeSources() {
        registeredMediaSources = null
        registeredOwner = null
        if (!nativeLoaded || !nativeClearMediaFds()) {
            throw MediaFailure("MEDIA_SOURCE_UNAVAILABLE", "Android media access is unavailable.")
        }
    }

    private fun clearMediaSources(result: MethodChannel.Result) {
        bindingEpoch.incrementAndGet()
        // Reserve one queue slot for a clear behind cancelled opens.
        submit(result, reserved = true) { clearNativeSources(); true }
    }

    private fun setMediaSources(call: MethodCall, result: MethodChannel.Result) {
        val arguments = call.arguments as? Map<*, *>
        val uris = (arguments?.get("sources") ?: call.arguments) as? List<*>
            ?: return result.error("INVALID_MEDIA_SOURCES", "The media source list is invalid.", null)
        if (uris.size > MAX_MEDIA_SOURCES || uris.any {
                it !is String || it.length > MAX_MEDIA_SOURCE_URI_CHARS || !it.startsWith("content://")
            }) {
            return result.error("INVALID_MEDIA_SOURCES", "The Android media source list exceeds its bounds or contains an invalid URI.", null)
        }
        val sources = uris.filterIsInstance<String>().distinct().sorted()
        val generation = (arguments?.get("generation") as? Number)?.toLong()
        val owner = arguments?.get("owner") as? String
        val context = producerContext
            ?: return result.error("MEDIA_SOURCE_UNAVAILABLE", "Android media access is unavailable.", null)
        val epoch = bindingEpoch.incrementAndGet()
        submit(result) {
            fun checkCurrent() {
                if (!nativeLoaded) throw MediaFailure("MEDIA_SOURCE_UNAVAILABLE", "Android native media access is unavailable.")
                if (!attached.get() || bindingEpoch.get() != epoch ||
                    (generation != null && nativeResourceSnapshot()?.get(0) != generation)) {
                    throw MediaFailure("STALE_PREVIEW_REQUEST", "A newer preview or project replaced this media request.")
                }
            }
            try {
                checkCurrent()
                // A duplicate may remain readable after Android revokes its
                // grant. Revalidate access even for a cached set, without
                // reopening descriptors on every frame.
                for (source in sources) {
                    if (context.checkUriPermission(Uri.parse(source), Process.myPid(), Process.myUid(),
                            Intent.FLAG_GRANT_READ_URI_PERMISSION) != PackageManager.PERMISSION_GRANTED) {
                        throw MediaFailure("MEDIA_PERMISSION_REQUIRED", "Access to an Android media source was revoked. Select it again and retry preview.")
                    }
                }
                if (registeredMediaSources != sources || registeredOwner != owner) {
                    clearNativeSources()
                    for (source in sources) {
                        checkCurrent()
                        val descriptor = context.contentResolver.openFileDescriptor(Uri.parse(source), "r")
                            ?: throw MediaFailure("MEDIA_SOURCE_MISSING", "An Android media source is missing. Restore or relink it and retry preview.")
                        descriptor.use {
                            if (!nativeRegisterMediaFd(source, it.fd)) {
                                try { Os.lseek(it.fileDescriptor, 0L, OsConstants.SEEK_CUR) }
                                catch (error: ErrnoException) {
                                    if (error.errno == OsConstants.ESPIPE) {
                                        throw MediaFailure("MEDIA_SOURCE_NOT_SEEKABLE", "This Android provider cannot seek. Choose a seekable media document.")
                                    }
                                }
                                throw MediaFailure("MEDIA_SOURCE_REGISTRATION_FAILED", "Android media registration failed. Select the source again and retry preview.")
                            }
                            registrationCount++
                        }
                        checkCurrent()
                    }
                    checkCurrent()
                    registeredMediaSources = sources
                    registeredOwner = owner
                }
                checkCurrent()
                true
            } catch (error: Exception) {
                clearNativeSources()
                when (error) {
                    is MediaFailure -> throw error
                    is SecurityException -> throw MediaFailure("MEDIA_PERMISSION_REQUIRED", "Access to an Android media source is unavailable. Select it again and retry preview.")
                    is FileNotFoundException -> throw MediaFailure("MEDIA_SOURCE_MISSING", "An Android media source is missing. Restore or relink it and retry preview.")
                    else -> throw MediaFailure("MEDIA_SOURCE_UNAVAILABLE", "An Android media source could not be opened. Select it again and retry preview.")
                }
            }
        }
    }

    private fun submit(result: MethodChannel.Result, reserved: Boolean = false, operation: () -> Any?) {
        try {
            if (!reserved && worker.queue.size >= 7) throw java.util.concurrent.RejectedExecutionException("Android operation queue is full")
            worker.execute {
                try {
                    peakQueuedOperations.accumulateAndGet(worker.queue.size.toLong()) { a, b -> maxOf(a, b) }
                    val value = operation()
                    mainHandler.post { result.success(value) }
                } catch (error: MediaFailure) {
                    mainHandler.post { result.error(error.code, error.message, null) }
                } catch (_: Exception) {
                    mainHandler.post { result.error("MEDIA_SOURCE_UNAVAILABLE", "Android media access is unavailable.", null) }
                }
            }
        } catch (_: RuntimeException) {
            result.error("MEDIA_SOURCE_UNAVAILABLE", "Android media access is unavailable.", null)
        }
    }

    private fun scheduleFrame(result: MethodChannel.Result?) {
        if (!attached.get() || producer == null) { result?.success(false); return }
        if (result != null) {
            if (pendingFrameResults.size == MAX_PENDING_FRAME_RESULTS) {
                rejectedFrameRequests++
                result.success(false)
                return
            }
            pendingFrameResults.add(result)
            peakPendingFrameResults = maxOf(peakPendingFrameResults, pendingFrameResults.size)
        }
        if (!workPending.compareAndSet(false, true)) {
            frameRequestedWhilePending = true
            return
        }
        presentScheduledFrame(allowFollowUp = true)
    }

    /**
     * Rust publishes premultiplied BGRA, which is the byte order a Flutter
     * external texture expects. `Bitmap.Config.ARGB_8888` stores bytes as
     * R, G, B, A, so `copyPixelsFromBuffer` would swap red and blue and show
     * red content as blue. Swap while copying instead, reusing one scratch
     * buffer so the present path stays allocation-free after the first frame.
     */
    private fun copyBgraIntoArgb8888(target: android.graphics.Bitmap, frame: AndroidViewerFrame) {
        val source = frame.pixels.duplicate().order(ByteOrder.nativeOrder()).apply { position(0) }
        val count = target.width * target.height
        val scratch = bgraSwapScratch
            ?.takeIf { it.capacity() >= count * 4 }
            ?: ByteBuffer.allocateDirect(count * 4).also { bgraSwapScratch = it }
        val sourcePixels = source.asIntBuffer()
        scratch.clear()
        val outputPixels = scratch.order(ByteOrder.nativeOrder()).asIntBuffer()
        repeat(count) {
            val pixel = sourcePixels.get()
            outputPixels.put(
                (pixel and 0xFF00FF00.toInt()) or
                    ((pixel and 0x00FF0000) ushr 16) or
                    ((pixel and 0x000000FF) shl 16),
            )
        }
        outputPixels.flip()
        target.copyPixelsFromBuffer(outputPixels)
    }

    private fun presentScheduledFrame(allowFollowUp: Boolean) {
        val epoch = surfaceEpoch.get()
        try {
            if (worker.queue.size >= 7) throw java.util.concurrent.RejectedExecutionException("Android operation queue is full")
            worker.execute {
                var copiedGeneration: Long? = null
                try {
                    if (nativeLoaded && attached.get()) {
                        val acquireStarted = SystemClock.elapsedRealtimeNanos()
                        val frame = nativeAcquireLatest()
                        val acquireMicros = (SystemClock.elapsedRealtimeNanos() - acquireStarted) / 1000
                        maxFrameAcquireMicros = maxOf(maxFrameAcquireMicros, acquireMicros)
                        totalFrameAcquireMicros += acquireMicros
                        if (frame != null) {
                            try {
                                if (epoch == surfaceEpoch.get() && nativeResourceSnapshot()?.get(0) == frame.generation) {
                                    val target = bitmap?.takeIf { it.width == frame.width && it.height == frame.height && !it.isRecycled }
                                        ?: run {
                                            val allocationStarted = SystemClock.elapsedRealtimeNanos()
                                            Bitmap.createBitmap(frame.width, frame.height, Bitmap.Config.ARGB_8888).also {
                                                maxBitmapAllocationMicros = maxOf(
                                                    maxBitmapAllocationMicros,
                                                    (SystemClock.elapsedRealtimeNanos() - allocationStarted) / 1000,
                                                )
                                                bitmapAllocations++
                                                bitmap?.recycle(); bitmap = it
                                            }
                                        }
                                    val copyStarted = SystemClock.elapsedRealtimeNanos()
                                    copyBgraIntoArgb8888(target, frame)
                                    val copyMicros = (SystemClock.elapsedRealtimeNanos() - copyStarted) / 1000
                                    maxBitmapCopyMicros = maxOf(maxBitmapCopyMicros, copyMicros)
                                    totalBitmapCopyMicros += copyMicros
                                    copiedGeneration = frame.generation
                                } else staleDrops.incrementAndGet()
                            } finally { nativeReleaseFrame(frame.releaseContext) }
                        }
                    }
                } catch (_: Exception) { copiedGeneration = null }
                val generation = copiedGeneration
                mainHandler.post {
                    var presented = false
                    if (generation != null && attached.get() && surfaceAvailable && epoch == surfaceEpoch.get() &&
                        nativeResourceSnapshot()?.get(0) == generation) {
                        val started = SystemClock.elapsedRealtimeNanos()
                        try {
                            val target = bitmap!!
                            producer?.let { surfaceProducer ->
                                val resizeStarted = SystemClock.elapsedRealtimeNanos()
                                surfaceProducer.setSize(target.width, target.height)
                                maxSurfaceResizeMicros = maxOf(
                                    maxSurfaceResizeMicros,
                                    (SystemClock.elapsedRealtimeNanos() - resizeStarted) / 1000,
                                )
                                val surface = surfaceProducer.surface
                                if (surface.isValid) {
                                    val lockStarted = SystemClock.elapsedRealtimeNanos()
                                    // Every preview frame covers the entire surface. Hardware
                                    // canvas buffers are not preserved between frames, so this
                                    // satisfies the Android contract while avoiding the software
                                    // canvas path for bitmap presentation.
                                    val canvas = surface.lockHardwareCanvas()
                                    maxCanvasLockMicros = maxOf(
                                        maxCanvasLockMicros,
                                        (SystemClock.elapsedRealtimeNanos() - lockStarted) / 1000,
                                    )
                                    try {
                                        val drawStarted = SystemClock.elapsedRealtimeNanos()
                                        canvas.drawBitmap(target, null, Rect(0, 0, canvas.width, canvas.height), null)
                                        maxCanvasDrawMicros = maxOf(
                                            maxCanvasDrawMicros,
                                            (SystemClock.elapsedRealtimeNanos() - drawStarted) / 1000,
                                        )
                                    } finally {
                                        val postStarted = SystemClock.elapsedRealtimeNanos()
                                        surface.unlockCanvasAndPost(canvas)
                                        maxCanvasPostMicros = maxOf(
                                            maxCanvasPostMicros,
                                            (SystemClock.elapsedRealtimeNanos() - postStarted) / 1000,
                                        )
                                    }
                                    presented = true
                                    presentedFrames++
                                }
                            }
                        } catch (_: Exception) { presented = false }
                        finally {
                            val elapsed = (SystemClock.elapsedRealtimeNanos() - started) / 1000
                            maxMainDrawMicros = maxOf(maxMainDrawMicros, elapsed)
                            totalMainDrawMicros += elapsed
                        }
                    } else if (generation != null) staleDrops.incrementAndGet()
                    if (!presented && attached.get() && producer != null && allowFollowUp && frameRequestedWhilePending) {
                        frameRequestedWhilePending = false
                        presentScheduledFrame(allowFollowUp = false)
                    } else completeFrameResults(presented)
                }
            }
        } catch (_: RuntimeException) { completeFrameResults(false) }
    }

    private fun completeFrameResults(presented: Boolean) {
        frameRequestedWhilePending = false
        workPending.set(false)
        val results = pendingFrameResults.toList()
        pendingFrameResults.clear()
        results.forEach { it.success(presented) }
    }

    private fun resourceSnapshot(result: MethodChannel.Result) {
        val mainMetrics = mapOf<String, Any>(
            "pendingPresentations" to if (workPending.get()) 1 else 0,
            "presentedFrames" to presentedFrames, "maxMainDrawMicros" to maxMainDrawMicros,
            "totalMainDrawMicros" to totalMainDrawMicros,
            "maxSurfaceResizeMicros" to maxSurfaceResizeMicros,
            "maxCanvasLockMicros" to maxCanvasLockMicros,
            "maxCanvasDrawMicros" to maxCanvasDrawMicros,
            "maxCanvasPostMicros" to maxCanvasPostMicros,
            "surfaceCreations" to surfaceCreations, "surfaceRestorations" to surfaceRestorations,
            "surfaceCleanups" to surfaceCleanups, "surfaceReleases" to surfaceReleases,
            "peakPendingFrameResults" to peakPendingFrameResults, "rejectedFrameRequests" to rejectedFrameRequests,
        )
        submit(result) {
            if (!nativeLoaded) throw MediaFailure("MEDIA_SOURCE_UNAVAILABLE", "Android native media access is unavailable.")
            val native = nativeResourceSnapshot()
                ?: throw MediaFailure("MEDIA_SOURCE_UNAVAILABLE", "Android media access is unavailable.")
            mainMetrics + mapOf(
                "generation" to native[0], "inFlightLeases" to native[1], "latestFrameBytes" to native[2],
                "duplicatedMediaFds" to native[3], "bitmapBytes" to (bitmap?.allocationByteCount ?: 0),
                "maxFrameAcquireMicros" to maxFrameAcquireMicros,
                "totalFrameAcquireMicros" to totalFrameAcquireMicros,
                "maxBitmapCopyMicros" to maxBitmapCopyMicros,
                "totalBitmapCopyMicros" to totalBitmapCopyMicros,
                "bitmapAllocations" to bitmapAllocations,
                "maxBitmapAllocationMicros" to maxBitmapAllocationMicros,
                "registrationCount" to registrationCount, "staleDrops" to staleDrops.get(),
                "queuedOperations" to worker.queue.size, "peakQueuedOperations" to peakQueuedOperations.get(),
            )
        }
    }

    private external fun nativeAcquireLatest(): AndroidViewerFrame?
    private external fun nativeReleaseFrame(releaseContext: Long)
    private external fun nativeRegisterMediaFd(uri: String, fd: Int): Boolean
    private external fun nativeClearMediaFds(): Boolean
    private external fun nativeResourceSnapshot(): LongArray?

    companion object {
        private const val MAX_PENDING_FRAME_RESULTS = 8
        private const val MAX_MEDIA_SOURCES = 64
        private const val MAX_MEDIA_SOURCE_URI_CHARS = 8192
    }
}
