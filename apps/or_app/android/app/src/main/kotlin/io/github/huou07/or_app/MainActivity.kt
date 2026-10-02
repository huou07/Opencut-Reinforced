package io.github.huou07.or_app

import android.content.Intent
import android.net.Uri
import android.os.Handler
import android.os.Looper
import android.system.Os
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
import java.util.concurrent.Executors

private const val SAF_CHANNEL = "io.github.huou07.or_app/saf_storage"
private const val SAF_PICK_REQUEST = 0x4f52
private const val MAX_PROJECT_BYTES = 64L * 1024 * 1024

class MainActivity : FlutterActivity() {
    private data class PendingPick(
        val method: String,
        val result: MethodChannel.Result,
        val requiredModes: Int,
    )

    private class SafFailure(val code: String, message: String) : Exception(message)

    private val worker = Executors.newSingleThreadExecutor()
    private val mainHandler = Handler(Looper.getMainLooper())
    private var pendingPick: PendingPick? = null

    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)
        MethodChannel(flutterEngine.dartExecutor.binaryMessenger, SAF_CHANNEL)
            .setMethodCallHandler(::handleSafCall)
    }

    private fun handleSafCall(call: MethodCall, result: MethodChannel.Result) {
        when (call.method) {
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
    ) {
        if (pendingPick != null) {
            result.error("PROJECT_PICKER_BUSY", "A document request is already open.", null)
            return
        }
        val intent = Intent(action).apply {
            addCategory(Intent.CATEGORY_OPENABLE)
            type = "*/*"
            putExtra(
                Intent.EXTRA_MIME_TYPES,
                arrayOf("application/octet-stream", "application/json"),
            )
            if (suggestedName != null) putExtra(Intent.EXTRA_TITLE, suggestedName)
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
        if (requestCode != SAF_PICK_REQUEST) return
        val pending = pendingPick ?: return
        pendingPick = null
        val selectedIntent = data
        val uri = selectedIntent?.data
        if (resultCode != RESULT_OK || selectedIntent == null || uri == null) {
            pending.result.success(null)
            return
        }
        val resultFlags = selectedIntent.flags
        runIo(pending.result, "PROJECT_PICK_FAILED") {
            takePersistableGrant(uri, resultFlags, pending.requiredModes)
            when (pending.method) {
                "openProject" -> projectLocation(uri, prepareWorkingCopy(uri))
                "createProject" -> projectLocation(uri, prepareNewWorkingCopy(uri))
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
            writePrivateAtomically(workingCopy, remote)
            writePrivateAtomically(baselineFile, remoteDigest.toByteArray(Charsets.US_ASCII))
            return workingCopy
        }

        val local = readPrivateFile(workingCopy)
        val localDigest = digest(local)
        val baseline = readBaseline(baselineFile)
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
                null to error
            } catch (_: Exception) {
                null to SafFailure(failureCode, "The project storage operation failed.")
            }
            mainHandler.post {
                val failure = outcome.second
                if (failure == null) result.success(outcome.first)
                else result.error(failure.code, failure.message, null)
            }
        }
    }
}
