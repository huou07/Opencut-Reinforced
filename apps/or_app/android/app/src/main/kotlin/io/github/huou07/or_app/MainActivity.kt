package io.github.huou07.or_app

import android.content.Intent
import android.content.Context
import android.provider.DocumentsContract
import android.provider.OpenableColumns
import android.net.Uri
import android.os.Handler
import android.os.Looper
import android.system.Os
import android.util.Log
import io.flutter.embedding.android.FlutterActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodCall
import io.flutter.plugin.common.MethodChannel
import java.io.ByteArrayOutputStream
import java.io.File
import java.io.FileInputStream
import java.io.FileOutputStream
import java.io.IOException
import java.io.InputStream
import java.security.MessageDigest
import java.util.UUID
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.Executors
import java.util.concurrent.atomic.AtomicBoolean

private const val SAF_CHANNEL = "io.github.huou07.or_app/saf_storage"
private const val SAF_PICK_REQUEST = 0x4f52
private const val MAX_PROJECT_BYTES = 64L * 1024 * 1024
private const val MAX_CAPTION_BYTES = 8L * 1024 * 1024
private const val MAX_CAPTION_EXPORT_BYTES = 16L * 1024 * 1024

class MainActivity : FlutterActivity() {
    private external fun initializeAudioContext(context: Context): Boolean

    private data class PendingPick(
        val method: String,
        val result: MethodChannel.Result,
        val requiredModes: Int,
    )

    private class SafFailure(val code: String, message: String) : Exception(message)

    private val worker = Executors.newSingleThreadExecutor()
    private val mainHandler = Handler(Looper.getMainLooper())
    private val exportTransfers = ConcurrentHashMap<String, AtomicBoolean>()
    private val captionExportUris = ConcurrentHashMap<String, Uri>()
    private var pendingPick: PendingPick? = null

    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        System.loadLibrary("or_app_bridge")
        if (!initializeAudioContext(applicationContext)) {
            throw IllegalStateException("Android audio runtime could not be initialized.")
        }
        super.configureFlutterEngine(flutterEngine)
        if (applicationInfo.flags and android.content.pm.ApplicationInfo.FLAG_DEBUGGABLE != 0) {
            try {
                val fixture = Class.forName("io.github.huou07.or_app.SafFixtureControl")
                    .getDeclaredConstructor().newInstance() as io.flutter.embedding.engine.plugins.FlutterPlugin
                flutterEngine.plugins.add(fixture)
            } catch (_: ClassNotFoundException) {
                // The acceptance setup bridge is absent from release builds.
            }
        }
        MethodChannel(flutterEngine.dartExecutor.binaryMessenger, SAF_CHANNEL)
            .setMethodCallHandler(::handleSafCall)
    }

    private fun handleSafCall(call: MethodCall, result: MethodChannel.Result) {
        when (call.method) {
            "openCaptionFile" -> launchPicker(
                Intent.ACTION_OPEN_DOCUMENT,
                "openCaptionFile",
                result,
                Intent.FLAG_GRANT_READ_URI_PERMISSION,
                requiredModes = Intent.FLAG_GRANT_READ_URI_PERMISSION,
                mimeType = "*/*",
                allowMultiple = false,
            )
            "deleteCaptionFile" -> {
                val workingPath = call.argument<String>("workingPath")
                if (workingPath == null) {
                    result.error("CAPTION_PICK_FAILED", "The temporary caption file is invalid.", null)
                } else {
                    runIo(result, "CAPTION_PICK_FAILED") {
                        requireManagedCaptionFile(workingPath).delete()
                        null
                    }
                }
            }
            "createCaptionExport" -> {
                val extension = call.argument<String>("extension")
                    ?.lowercase()
                    ?.takeIf { it == "srt" || it == "vtt" }
                val mimeType = call.argument<String>("mimeType")
                val expectedMimeType = if (extension == "vtt") "text/vtt" else "application/x-subrip"
                if (extension == null || mimeType != expectedMimeType) {
                    result.error("CAPTION_EXPORT_PATH_INVALID", "The caption export format is invalid.", null)
                } else {
                    val suggestedName = call.argument<String>("suggestedName")
                        ?.substringAfterLast('/')
                        ?.substringAfterLast('\\')
                        ?.takeIf(String::isNotBlank)
                        ?.let { if (it.endsWith(".$extension", ignoreCase = true)) it else "$it.$extension" }
                        ?: "captions.$extension"
                    launchPicker(
                        Intent.ACTION_CREATE_DOCUMENT,
                        "createCaptionExport",
                        result,
                        Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_WRITE_URI_PERMISSION,
                        suggestedName,
                        requiredModes = Intent.FLAG_GRANT_WRITE_URI_PERMISSION,
                        mimeType = mimeType,
                    )
                }
            }
            "publishCaptionExport" -> {
                val workingPath = call.argument<String>("workingPath")
                if (workingPath == null) {
                    result.error("CAPTION_EXPORT_FAILED", "The caption export location is invalid.", null)
                } else {
                    runIo(result, "CAPTION_EXPORT_FAILED") {
                        publishCaptionExport(workingPath)
                        null
                    }
                }
            }
            "discardCaptionExport" -> {
                val workingPath = call.argument<String>("workingPath")
                if (workingPath == null) {
                    result.error("CAPTION_EXPORT_FAILED", "The caption export location is invalid.", null)
                } else {
                    runIo(result, "CAPTION_EXPORT_FAILED") {
                        discardCaptionExport(workingPath)
                        null
                    }
                }
            }
            "openMedia" -> launchPicker(
                Intent.ACTION_OPEN_DOCUMENT,
                "openMedia",
                result,
                Intent.FLAG_GRANT_READ_URI_PERMISSION,
                requiredModes = Intent.FLAG_GRANT_READ_URI_PERMISSION,
                mimeType = "*/*",
                allowMultiple = true,
            )
            "openProject" -> launchPicker(
                Intent.ACTION_OPEN_DOCUMENT,
                "openProject",
                result,
                Intent.FLAG_GRANT_READ_URI_PERMISSION or
                    Intent.FLAG_GRANT_WRITE_URI_PERMISSION,
                requiredModes = Intent.FLAG_GRANT_READ_URI_PERMISSION,
            )
            "createProject" -> {
                val suggestedName = call.argument<String>("suggestedName")
                    ?.substringAfterLast('/')
                    ?.substringAfterLast('\\')
                    ?.takeIf(String::isNotBlank)
                    ?: "project.orproj"
                launchPicker(
                    Intent.ACTION_CREATE_DOCUMENT,
                    "createProject",
                    result,
                    Intent.FLAG_GRANT_READ_URI_PERMISSION or
                        Intent.FLAG_GRANT_WRITE_URI_PERMISSION,
                    suggestedName,
                )
            }
            "createExport" -> {
                val suggestedName = call.argument<String>("suggestedName")
                    ?.substringAfterLast('/')
                    ?.substringAfterLast('\\')
                    ?.takeIf(String::isNotBlank)
                    ?.let { if (it.endsWith(".mkv", ignoreCase = true)) it else "$it.mkv" }
                    ?: "export.mkv"
                launchPicker(
                    Intent.ACTION_CREATE_DOCUMENT,
                    "createExport",
                    result,
                    Intent.FLAG_GRANT_READ_URI_PERMISSION or
                        Intent.FLAG_GRANT_WRITE_URI_PERMISSION,
                    suggestedName,
                    requiredModes = Intent.FLAG_GRANT_WRITE_URI_PERMISSION,
                    mimeType = "video/x-matroska",
                )
            }
            "synchronizeProject" -> {
                val workingPath = call.argument<String>("workingPath")
                val documentUri = call.argument<String>("documentUri")
                if (workingPath == null || documentUri == null) {
                    result.error("PROJECT_SYNC_FAILED", "Project sync request is invalid.", null)
                } else {
                    runIo(result, "PROJECT_SYNC_FAILED") {
                        synchronizeProject(workingPath, documentUri)
                    }
                }
            }
            "publishExport" -> {
                val workingPath = call.argument<String>("workingPath")
                val documentUri = call.argument<String>("documentUri")
                if (workingPath == null || documentUri == null) {
                    result.error("EXPORT_SAVE_FAILED", "The selected export location is invalid.", null)
                } else {
                    val cancelled = AtomicBoolean(false)
                    if (exportTransfers.putIfAbsent(workingPath, cancelled) != null) {
                        result.error("EXPORT_BUSY", "The export is already being saved.", null)
                    } else {
                        runIo(result, "EXPORT_SAVE_FAILED") {
                            try {
                                publishExport(workingPath, documentUri, cancelled)
                                null
                            } finally {
                                exportTransfers.remove(workingPath, cancelled)
                            }
                        }
                    }
                }
            }
            "cancelExportPublish" -> {
                val workingPath = call.argument<String>("workingPath")
                if (workingPath == null) {
                    result.error("EXPORT_CANCEL_FAILED", "The export cannot be cancelled.", null)
                } else {
                    result.success(exportTransfers[workingPath]?.let { it.set(true); true } ?: false)
                }
            }
            "discardExport" -> {
                val workingPath = call.argument<String>("workingPath")
                val documentUri = call.argument<String>("documentUri")
                if (workingPath == null || documentUri == null) {
                    result.error("EXPORT_CLEANUP_FAILED", "The export cannot be cleared.", null)
                } else {
                    exportTransfers[workingPath]?.set(true)
                    runIo(result, "EXPORT_CLEANUP_FAILED") {
                        discardExport(workingPath, documentUri)
                        null
                    }
                }
            }
            else -> result.notImplemented()
        }
    }

    private fun launchPicker(
        action: String,
        method: String,
        result: MethodChannel.Result,
        modes: Int,
        suggestedName: String? = null,
        requiredModes: Int = modes,
        mimeType: String = "*/*",
        allowMultiple: Boolean = false,
    ) {
        if (pendingPick != null) {
            result.error("PROJECT_PICKER_BUSY", "A document request is already open.", null)
            return
        }
        val intent = Intent(action).apply {
            addCategory(Intent.CATEGORY_OPENABLE)
            type = mimeType
            putExtra(
                Intent.EXTRA_MIME_TYPES,
                if (method == "openMedia") {
                    arrayOf("video/*", "audio/*")
                } else if (method == "openCaptionFile") {
                    arrayOf("application/x-subrip", "text/vtt", "text/plain")
                } else if (mimeType == "*/*") {
                    arrayOf("application/octet-stream", "application/json")
                }
                else arrayOf(mimeType),
            )
            if (suggestedName != null) putExtra(Intent.EXTRA_TITLE, suggestedName)
            if (allowMultiple) putExtra(Intent.EXTRA_ALLOW_MULTIPLE, true)
            addFlags(modes or Intent.FLAG_GRANT_PERSISTABLE_URI_PERMISSION)
        }
        try {
            pendingPick = PendingPick(method, result, requiredModes)
            startActivityForResult(intent, SAF_PICK_REQUEST)
        } catch (_: Exception) {
            pendingPick = null
            result.error("PROJECT_PICK_FAILED", "The document picker could not be opened.", null)
        }
    }

    @Deprecated("The Activity Result API is handled by FlutterActivity plugins.")
    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        super.onActivityResult(requestCode, resultCode, data)
        safDiagnostic(
            "activity result request=$requestCode result=$resultCode " +
                "pending=${pendingPick?.method ?: "none"} hasData=${data != null}",
        )
        if (requestCode != SAF_PICK_REQUEST) return
        val pending = pendingPick ?: return
        pendingPick = null
        val selectedIntent = data
        if (resultCode != RESULT_OK || selectedIntent == null) {
            pending.result.success(null)
            return
        }
        val resultFlags = selectedIntent.flags
        if (pending.method == "openMedia") {
            val uris = buildList {
                selectedIntent.clipData?.let { clips ->
                    for (index in 0 until clips.itemCount) add(clips.getItemAt(index).uri)
                }
                selectedIntent.data?.let { add(it) }
            }.distinctBy(Uri::toString)
            if (uris.isEmpty()) {
                pending.result.success(null)
                return
            }
            runIo(pending.result, "PROJECT_PICK_FAILED") {
                safDiagnostic("processing ${pending.method} result count=${uris.size}")
                uris.forEach { takePersistableGrant(it, resultFlags, pending.requiredModes) }
                mapOf("sourceUris" to uris.map(Uri::toString))
            }
            return
        }
        val uri = selectedIntent.data
        if (uri == null) {
            pending.result.success(null)
            return
        }
        runIo(pending.result, "PROJECT_PICK_FAILED") {
            safDiagnostic("processing ${pending.method} result")
            takePersistableGrant(uri, resultFlags, pending.requiredModes)
            when (pending.method) {
                "openCaptionFile" -> try {
                    captionLocation(uri)
                } finally {
                    try {
                        contentResolver.releasePersistableUriPermission(
                            uri,
                            Intent.FLAG_GRANT_READ_URI_PERMISSION,
                        )
                    } catch (_: SecurityException) {
                        // The selected file is already copied into bounded private staging.
                    }
                }
                "openProject" -> projectLocation(uri, prepareWorkingCopy(uri))
                "createProject" -> projectLocation(uri, prepareNewWorkingCopy(uri))
                "createExport" -> exportLocation(uri)
                "createCaptionExport" -> captionExportLocation(uri)
                "openMedia" -> mapOf("sourceUri" to uri.toString())
                else -> throw SafFailure("PROJECT_PICK_FAILED", "The project selection is invalid.")
            }
        }
    }

    private fun takePersistableGrant(uri: Uri, resultFlags: Int, requiredModes: Int) {
        requireDocumentUri(uri)
        val modes = resultFlags and
            (Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_WRITE_URI_PERMISSION)
        if ((modes and requiredModes) != requiredModes) {
            throw SafFailure("PROJECT_PICK_FAILED", "The provider did not grant the required access.")
        }
        try {
            contentResolver.takePersistableUriPermission(uri, modes)
        } catch (_: SecurityException) {
            throw SafFailure("PROJECT_PICK_FAILED", "The document permission could not be retained.")
        }
    }

    private fun projectLocation(uri: Uri, file: File): Map<String, String> = mapOf(
        "workingPath" to file.absolutePath,
        "documentUri" to uri.toString(),
    )

    private fun prepareNewWorkingCopy(uri: Uri): File {
        val remote = readDocument(uri)
        if (remote.isNotEmpty()) {
            throw SafFailure("PROJECT_NOT_EMPTY", "The selected document is not empty.")
        }
        val workingCopy = workingCopyFile(uri)
        val baseline = baselineFile(uri)
        if (workingCopy.exists()) {
            throw SafFailure(
                "PROJECT_ALREADY_MANAGED",
                "A managed project copy already exists.",
            )
        }
        val remoteDigest = digest(remote)
        if (readBaseline(baseline)?.let { it != remoteDigest } == true) {
            throw externalChange()
        }
        writePrivateAtomically(baseline, remoteDigest.toByteArray(Charsets.US_ASCII))
        return workingCopy
    }

    private fun prepareWorkingCopy(uri: Uri): File {
        val remote = readDocument(uri)
        val workingCopy = workingCopyFile(uri)
        val baselineFile = baselineFile(uri)
        val remoteDigest = digest(remote)
        if (!workingCopy.exists()) {
            safDiagnostic(
                "new project copy remoteBytes=${remote.size} " +
                    "remoteDigest=${remoteDigest.take(12)}",
            )
            writePrivateAtomically(workingCopy, remote)
            writePrivateAtomically(baselineFile, remoteDigest.toByteArray(Charsets.US_ASCII))
            return workingCopy
        }

        val local = readPrivateFile(workingCopy)
        val localDigest = digest(local)
        val baseline = readBaseline(baselineFile)
        safDiagnostic(
            "project copies remoteBytes=${remote.size} localBytes=${local.size} " +
                "baselinePresent=${baseline != null} " +
                "localMatchesRemote=${localDigest == remoteDigest} " +
                "baselineMatchesRemote=${baseline == remoteDigest} " +
                "localMatchesBaseline=${baseline == localDigest}",
        )
        when {
            baseline == null && localDigest == remoteDigest ->
                writePrivateAtomically(baselineFile, remoteDigest.toByteArray(Charsets.US_ASCII))
            baseline == null -> throw externalChange()
            remoteDigest == baseline -> Unit
            localDigest == remoteDigest ->
                writePrivateAtomically(baselineFile, remoteDigest.toByteArray(Charsets.US_ASCII))
            else -> throw externalChange()
        }
        return workingCopy
    }

    private fun synchronizeProject(workingPath: String, documentUri: String): Map<String, Boolean> {
        val uri = requireDocumentUri(Uri.parse(documentUri))
        val workingCopy = requireManagedPath(workingPath, uri)
        val baselineFile = baselineFile(uri)
        val baseline = readBaseline(baselineFile) ?: throw externalChange()
        val remote = readDocument(uri)
        if (digest(remote) != baseline) throw externalChange()

        val local = readPrivateFile(workingCopy)
        val localDigest = digest(local)
        if (localDigest == baseline) return mapOf("verified" to true)

        val output = contentResolver.openOutputStream(uri, "wt")
            ?: throw SafFailure("PROJECT_SYNC_FAILED", "The provider could not open the project for writing.")
        try {
            output.use {
                it.write(local)
                it.flush()
            }
        } catch (_: Exception) {
            throw SafFailure("PROJECT_SYNC_FAILED", "The project is saved locally, but provider writing failed.")
        }

        val readback = try {
            readDocument(uri)
        } catch (_: IOException) {
            writePrivateAtomically(baselineFile, localDigest.toByteArray(Charsets.US_ASCII))
            return mapOf("verified" to false)
        } catch (_: SecurityException) {
            writePrivateAtomically(baselineFile, localDigest.toByteArray(Charsets.US_ASCII))
            return mapOf("verified" to false)
        }
        if (!readback.contentEquals(local)) {
            throw SafFailure("PROJECT_SYNC_FAILED", "The provider readback did not match the saved project.")
        }
        writePrivateAtomically(baselineFile, localDigest.toByteArray(Charsets.US_ASCII))
        return mapOf("verified" to true)
    }

    private fun readDocument(uri: Uri): ByteArray {
        requireDocumentUri(uri)
        val input = contentResolver.openInputStream(uri)
            ?: throw SafFailure("PROJECT_PICK_FAILED", "The provider could not open the project for reading.")
        return input.use(::readBounded)
    }

    private fun exportLocation(uri: Uri): Map<String, String> {
        requireDocumentUri(uri)
        val file = File.createTempFile("or-export-", ".mkv", exportStagingDirectory())
        return mapOf("workingPath" to file.absolutePath, "documentUri" to uri.toString())
    }

    private fun captionExportLocation(uri: Uri): Map<String, String> {
        requireDocumentUri(uri)
        val displayName = contentResolver.query(
            uri,
            arrayOf(OpenableColumns.DISPLAY_NAME),
            null,
            null,
            null,
        )?.use { cursor -> if (cursor.moveToFirst()) cursor.getString(0) else null }
            ?: throw SafFailure("CAPTION_EXPORT_PATH_INVALID", "The caption destination is invalid.")
        val extension = displayName.substringAfterLast('.', "").lowercase()
        if (extension != "srt" && extension != "vtt") {
            throw SafFailure("CAPTION_EXPORT_PATH_INVALID", "Choose an SRT or WebVTT destination.")
        }
        val directory = captionExportStagingDirectory()
        val file = File.createTempFile("or-caption-export-", ".$extension", directory)
        captionExportUris[file.canonicalPath] = uri
        return mapOf(
            "workingPath" to file.canonicalPath,
            "documentUri" to uri.toString(),
        )
    }

    private fun publishCaptionExport(workingPath: String) {
        val source = requireManagedCaptionExportFile(workingPath)
        val uri = captionExportUris[workingPath]
            ?: throw SafFailure("CAPTION_EXPORT_FAILED", "The selected caption destination is no longer available.")
        val expectedBytes = source.length()
        if (expectedBytes <= 0L || expectedBytes > MAX_CAPTION_EXPORT_BYTES) {
            throw SafFailure("CAPTION_EXPORT_FAILED", "The caption export is empty or exceeds its size limit.")
        }
        try {
            val output = contentResolver.openOutputStream(uri, "wt")
                ?: throw SafFailure("CAPTION_EXPORT_PERMISSION_REQUIRED", "The selected location cannot be written.")
            FileInputStream(source).use { input ->
                output.use { destination ->
                    val buffer = ByteArray(32 * 1024)
                    var copied = 0L
                    while (true) {
                        val count = input.read(buffer)
                        if (count < 0) break
                        if (count == 0) throw IOException("The caption export returned an empty read.")
                        copied += count
                        if (copied > MAX_CAPTION_EXPORT_BYTES) {
                            throw SafFailure("CAPTION_EXPORT_FAILED", "The caption export exceeds its size limit.")
                        }
                        destination.write(buffer, 0, count)
                    }
                    destination.flush()
                    if (copied != expectedBytes) {
                        throw SafFailure("CAPTION_EXPORT_FAILED", "The caption export could not be fully saved.")
                    }
                }
            }
            captionExportUris.remove(workingPath)
            source.delete()
        } catch (error: Exception) {
            try {
                DocumentsContract.deleteDocument(contentResolver, uri)
            } catch (_: Exception) {
                // Some providers cannot delete a partially written item.
            }
            if (error is SafFailure) throw error
            if (error is SecurityException) {
                throw SafFailure("CAPTION_EXPORT_PERMISSION_REQUIRED", "Access to the selected location is unavailable.")
            }
            throw SafFailure("CAPTION_EXPORT_FAILED", "The caption export could not be saved.")
        }
    }

    private fun discardCaptionExport(workingPath: String) {
        val source = requireManagedCaptionExportFile(workingPath)
        val uri = captionExportUris.remove(workingPath)
        source.delete()
        if (uri != null) {
            try {
                DocumentsContract.deleteDocument(contentResolver, uri)
            } catch (_: Exception) {
                // A provider may not support deletion of a newly created item.
            }
        }
    }

    private fun captionLocation(uri: Uri): Map<String, String> {
        requireDocumentUri(uri)
        val displayName = contentResolver.query(
            uri,
            arrayOf(OpenableColumns.DISPLAY_NAME),
            null,
            null,
            null,
        )?.use { cursor ->
            if (cursor.moveToFirst()) cursor.getString(0) else null
        }
        val name = displayName ?: uri.lastPathSegment.orEmpty()
        val extension = when (name.substringAfterLast('.', "").lowercase()) {
            "srt" -> ".srt"
            "vtt" -> ".vtt"
            else -> throw SafFailure("CAPTION_PICK_FAILED", "Choose an SRT or WebVTT caption file.")
        }
        val source = contentResolver.openInputStream(uri)
            ?: throw SafFailure("CAPTION_PICK_FAILED", "The selected caption file could not be opened.")
        val directory = captionStagingDirectory()
        val destination = File.createTempFile("or-caption-", extension, directory)
        try {
            source.use { input ->
                FileOutputStream(destination).use { output ->
                    val buffer = ByteArray(32 * 1024)
                    var total = 0L
                    while (true) {
                        val count = input.read(buffer)
                        if (count < 0) break
                        if (count == 0) throw IOException("The caption provider returned an empty read.")
                        total += count
                        if (total > MAX_CAPTION_BYTES) {
                            throw SafFailure("CAPTION_FILE_TOO_LARGE", "The caption file exceeds the 8 MiB limit.")
                        }
                        output.write(buffer, 0, count)
                    }
                    output.flush()
                    output.fd.sync()
                }
            }
            return mapOf("workingPath" to destination.canonicalPath)
        } catch (error: Exception) {
            destination.delete()
            if (error is SafFailure) throw error
            throw SafFailure("CAPTION_PICK_FAILED", "The selected caption file could not be copied safely.")
        }
    }

    private fun captionStagingDirectory(): File {
        val directory = File(cacheDir, "or-captions")
        if (!directory.exists() && !directory.mkdirs()) {
            throw SafFailure("CAPTION_PICK_FAILED", "Temporary caption storage is unavailable.")
        }
        if (!directory.isDirectory) {
            throw SafFailure("CAPTION_PICK_FAILED", "Temporary caption storage is unavailable.")
        }
        return directory.canonicalFile
    }

    private fun captionExportStagingDirectory(): File {
        val directory = File(cacheDir, "or-caption-exports")
        if (!directory.exists() && !directory.mkdirs()) {
            throw SafFailure("CAPTION_EXPORT_FAILED", "Temporary caption export storage is unavailable.")
        }
        if (!directory.isDirectory) {
            throw SafFailure("CAPTION_EXPORT_FAILED", "Temporary caption export storage is unavailable.")
        }
        return directory.canonicalFile
    }

    private fun requireManagedCaptionExportFile(path: String): File {
        val directory = captionExportStagingDirectory()
        val requested = File(path).canonicalFile
        if (requested.parentFile != directory ||
            (!requested.name.endsWith(".srt", ignoreCase = true) &&
                !requested.name.endsWith(".vtt", ignoreCase = true)) ||
            !requested.isFile || requested.length() > MAX_CAPTION_EXPORT_BYTES
        ) {
            throw SafFailure("CAPTION_EXPORT_FAILED", "The temporary caption export path is invalid.")
        }
        return requested
    }

    private fun requireManagedCaptionFile(path: String): File {
        val directory = captionStagingDirectory()
        val requested = File(path).canonicalFile
        if (requested.parentFile != directory || !requested.isFile || requested.length() > MAX_CAPTION_BYTES) {
            throw SafFailure("CAPTION_PICK_FAILED", "The temporary caption file is invalid.")
        }
        return requested
    }

    private fun publishExport(workingPath: String, documentUri: String, cancelled: AtomicBoolean) {
        val source = requireManagedExportFile(workingPath)
        val uri = try {
            requireDocumentUri(Uri.parse(documentUri))
        } catch (_: Exception) {
            throw SafFailure("EXPORT_SAVE_FAILED", "The selected export location is invalid.")
        }
        if (!source.isFile || source.length() <= 0L) {
            source.delete()
            throw SafFailure("EXPORT_SAVE_FAILED", "The exported video is empty or unavailable.")
        }
        val expectedBytes = source.length()
        try {
            val output = contentResolver.openOutputStream(uri, "wt")
                ?: throw SafFailure("EXPORT_PERMISSION_REQUIRED", "The selected location cannot be written.")
            FileInputStream(source).use { input ->
                output.use { destination ->
                    val buffer = ByteArray(128 * 1024)
                    var copied = 0L
                    while (true) {
                        if (cancelled.get()) {
                            throw SafFailure("EXPORT_CANCELLED", "Export cancelled.")
                        }
                        val count = input.read(buffer)
                        if (count < 0) break
                        if (count == 0) throw IOException("The export returned an empty read.")
                        destination.write(buffer, 0, count)
                        copied += count
                    }
                    destination.flush()
                    if (copied != expectedBytes) {
                        throw SafFailure("EXPORT_SAVE_FAILED", "The exported video could not be fully saved.")
                    }
                }
            }
            if (cancelled.get()) throw SafFailure("EXPORT_CANCELLED", "Export cancelled.")
        } catch (error: Exception) {
            try {
                DocumentsContract.deleteDocument(contentResolver, uri)
            } catch (_: Exception) {
                // Some document providers cannot remove a partially written item.
            }
            if (error is SafFailure) throw error
            if (error is SecurityException) {
                throw SafFailure("EXPORT_PERMISSION_REQUIRED", "Access to the selected location is unavailable.")
            }
            throw SafFailure("EXPORT_SAVE_FAILED", "The exported video could not be saved to the selected location.")
        } finally {
            source.delete()
        }
    }

    private fun discardExport(workingPath: String, documentUri: String) {
        val source = requireManagedExportFile(workingPath)
        val uri = try {
            requireDocumentUri(Uri.parse(documentUri))
        } catch (_: Exception) {
            throw SafFailure("EXPORT_CLEANUP_FAILED", "The temporary export could not be cleared.")
        }
        source.delete()
        try {
            DocumentsContract.deleteDocument(contentResolver, uri)
        } catch (_: Exception) {
            // A provider may not support deleting a document created by this picker.
        }
    }

    private fun exportStagingDirectory(): File {
        val directory = File(cacheDir, "or-exports")
        if (!directory.exists() && !directory.mkdirs()) {
            throw SafFailure("EXPORT_SAVE_FAILED", "Temporary export storage is unavailable.")
        }
        if (!directory.isDirectory) {
            throw SafFailure("EXPORT_SAVE_FAILED", "Temporary export storage is unavailable.")
        }
        return directory.canonicalFile
    }

    private fun requireManagedExportFile(path: String): File {
        val directory = exportStagingDirectory()
        val requested = File(path).canonicalFile
        if (requested.parentFile != directory || !requested.name.endsWith(".mkv", ignoreCase = true)) {
            throw SafFailure("EXPORT_SAVE_FAILED", "The temporary export path is invalid.")
        }
        return requested
    }

    private fun readPrivateFile(file: File): ByteArray {
        if (!file.isFile || file.length() > MAX_PROJECT_BYTES) {
            throw SafFailure("PROJECT_PICK_FAILED", "The managed project copy is invalid or too large.")
        }
        return FileInputStream(file).use(::readBounded)
    }

    private fun readBounded(input: InputStream): ByteArray {
        val output = ByteArrayOutputStream()
        val buffer = ByteArray(64 * 1024)
        var total = 0L
        while (true) {
            val count = input.read(buffer)
            if (count < 0) break
            if (count == 0) throw IOException("The provider returned an empty read.")
            total += count
            if (total > MAX_PROJECT_BYTES) {
                throw SafFailure("PROJECT_PICK_FAILED", "The project exceeds the 64 MiB limit.")
            }
            output.write(buffer, 0, count)
        }
        return output.toByteArray()
    }

    private fun workingDirectory(): File {
        val directory = File(filesDir, "or-projects")
        if (!directory.exists() && !directory.mkdirs()) {
            throw SafFailure("PROJECT_PICK_FAILED", "The private project directory is unavailable.")
        }
        if (!directory.isDirectory) {
            throw SafFailure("PROJECT_PICK_FAILED", "The private project directory is unavailable.")
        }
        return directory.canonicalFile
    }

    private fun workingCopyFile(uri: Uri): File =
        File(workingDirectory(), "${uriKey(uri)}.orproj")

    private fun baselineFile(uri: Uri): File =
        File(workingDirectory(), "${uriKey(uri)}.sync-base")

    private fun requireManagedPath(path: String, uri: Uri): File {
        val requested = File(path).canonicalFile
        val expected = workingCopyFile(uri).canonicalFile
        if (requested != expected) {
            throw SafFailure("PROJECT_SYNC_FAILED", "The managed project path is invalid.")
        }
        return expected
    }

    private fun readBaseline(file: File): String? {
        if (!file.exists()) return null
        if (!file.isFile || file.length() != 64L) return null
        return file.readText(Charsets.US_ASCII).takeIf { it.matches(Regex("[0-9a-f]{64}")) }
    }

    private fun writePrivateAtomically(target: File, contents: ByteArray) {
        val temporary = File(target.parentFile, ".${target.name}.tmp-${UUID.randomUUID()}")
        try {
            FileOutputStream(temporary).use { output ->
                output.write(contents)
                output.flush()
                output.fd.sync()
            }
            Os.rename(temporary.absolutePath, target.absolutePath)
        } catch (error: Exception) {
            temporary.delete()
            throw SafFailure("PROJECT_PICK_FAILED", "The private project copy could not be stored.")
        }
    }

    private fun requireDocumentUri(uri: Uri): Uri {
        if (uri.scheme != "content" || uri.authority.isNullOrEmpty() || uri.encodedPath.isNullOrEmpty()) {
            throw SafFailure("PROJECT_PICK_FAILED", "The selected document URI is invalid.")
        }
        return uri
    }

    private fun uriKey(uri: Uri): String = digest(uri.toString().toByteArray(Charsets.UTF_8))

    private fun digest(bytes: ByteArray): String = MessageDigest.getInstance("SHA-256")
        .digest(bytes)
        .joinToString("") { byte -> "%02x".format(byte.toInt() and 0xff) }

    private fun externalChange() = SafFailure(
        "EXTERNAL_PROJECT_CHANGED",
        "The external project changed outside OR. Its changes were preserved.",
    )

    private fun runIo(
        result: MethodChannel.Result,
        failureCode: String,
        operation: () -> Any?,
    ) {
        worker.execute {
            val outcome = try {
                operation() to null
            } catch (error: SafFailure) {
                safDiagnostic("storage operation failed code=${error.code}")
                null to error
            } catch (_: Exception) {
                safDiagnostic("storage operation failed unexpectedly")
                null to SafFailure(failureCode, "The project storage operation failed.")
            }
            mainHandler.post {
                val failure = outcome.second
                safDiagnostic("storage operation complete success=${failure == null}")
                if (failure == null) result.success(outcome.first)
                else result.error(failure.code, failure.message, null)
            }
        }
    }

    private fun safDiagnostic(message: String) {
        if (applicationInfo.flags and android.content.pm.ApplicationInfo.FLAG_DEBUGGABLE != 0) {
            Log.i("OR-SAF", message)
        }
    }
}
