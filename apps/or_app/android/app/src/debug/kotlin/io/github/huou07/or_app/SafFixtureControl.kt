package io.github.huou07.or_app

import android.app.Activity
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.Process
import android.util.Log
import io.flutter.embedding.engine.plugins.FlutterPlugin
import io.flutter.embedding.engine.plugins.activity.ActivityAware
import io.flutter.embedding.engine.plugins.activity.ActivityPluginBinding
import io.flutter.plugin.common.MethodChannel
import io.flutter.plugin.common.PluginRegistry
import org.json.JSONObject

/** Debug-only setup bridge to a separate-UID provider; never a preview substitute. */
class SafFixtureControl : FlutterPlugin, ActivityAware, PluginRegistry.ActivityResultListener {
    private var channel: MethodChannel? = null
    private var binding: ActivityPluginBinding? = null
    private var pending: MethodChannel.Result? = null
    private var pendingOperation: String? = null
    private var pendingUri: String? = null
    override fun onAttachedToEngine(binding: FlutterPlugin.FlutterPluginBinding) {
        channel = MethodChannel(binding.binaryMessenger, "or_saf_acceptance_fixture").also { channel ->
            channel.setMethodCallHandler { call, result ->
                if (call.method != "control") result.notImplemented()
                else if (pending != null) result.error("FIXTURE_BUSY", "A fixture operation is pending.", null)
                else {
                    val activity = this.binding?.activity
                    val operation = (call.arguments as? Map<*, *>)?.get("operation") as? String
                    val uri = (call.arguments as? Map<*, *>)?.get("uri") as? String
                    if (activity == null) result.error("FIXTURE_DETACHED", "Fixture activity unavailable.", null)
                    else if (operation == "backgroundAndResume") {
                        activity.moveTaskToBack(true)
                        activity.applicationContext.openFileOutput(
                            BACKGROUND_SIGNAL_FILE,
                            Context.MODE_PRIVATE,
                        ).use { it.write(BACKGROUND_SIGNAL.toByteArray(Charsets.UTF_8)) }
                        Log.i("OrSafFixtureControl", "ANDROID_SAF_BACKGROUND_RELAUNCH_REQUESTED")
                        result.success(mapOf("backgrounded" to true))
                    } else try {
                        val intent = Intent().setClassName("dev.opencut.saffixture", "dev.opencut.saffixture.ControlActivity")
                        (call.arguments as? Map<*, *>)?.forEach { (key, value) -> if (key is String && value is String) intent.putExtra(key, value) }
                        pending = result
                        pendingOperation = operation
                        pendingUri = uri
                        activity.startActivityForResult(intent, REQUEST)
                    } catch (error: Exception) { pending = null; pendingOperation = null; pendingUri = null; result.error("FIXTURE_UNAVAILABLE", error.toString(), null) }
                }
            }
        }
    }
    override fun onDetachedFromEngine(binding: FlutterPlugin.FlutterPluginBinding) { channel?.setMethodCallHandler(null); channel = null }
    override fun onAttachedToActivity(binding: ActivityPluginBinding) { this.binding = binding; binding.addActivityResultListener(this) }
    override fun onDetachedFromActivityForConfigChanges() { onDetachedFromActivity() }
    override fun onReattachedToActivityForConfigChanges(binding: ActivityPluginBinding) { onAttachedToActivity(binding) }
    override fun onDetachedFromActivity() { binding?.removeActivityResultListener(this); binding = null }
    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?): Boolean {
        if (requestCode != REQUEST) return false
        val result = pending ?: return true
        val operation = pendingOperation
        val uri = pendingUri
        pending = null
        pendingOperation = null
        pendingUri = null
        if (resultCode != Activity.RESULT_OK) result.error("FIXTURE_FAILED", data?.getStringExtra("error") ?: "Fixture cancelled.", null)
        else {
            try {
                val activity = binding?.activity
                    ?: throw IllegalStateException("Fixture activity detached before completion.")
                if (operation == "persistMediaGrant") persistMediaGrant(activity, uri)
                val json = JSONObject(data!!.getStringExtra("data")!!)
                val values = json.keys().asSequence().associateWith { json.get(it) }.toMutableMap()
                values["appUid"] = Process.myUid()
                values["persistedMediaUris"] = activity.contentResolver.persistedUriPermissions
                    .filter { it.isReadPermission }
                    .map { it.uri.toString() }
                result.success(values)
            } catch (_: Exception) {
                result.error("FIXTURE_PERMISSION_FAILED", "The fixture could not retain its media document grants.", null)
            }
        }
        return true
    }

    private fun persistMediaGrant(activity: Activity, value: String?) {
        val uri = Uri.parse(value ?: throw IllegalArgumentException("Media URI is required."))
        require(uri.authority == "dev.opencut.saffixture.documents")
        activity.contentResolver.takePersistableUriPermission(uri, Intent.FLAG_GRANT_READ_URI_PERMISSION)
    }
    companion object {
        const val BACKGROUND_SIGNAL_FILE = "or-saf-background-requested"
        private const val BACKGROUND_SIGNAL = "ANDROID_SAF_BACKGROUND_RELAUNCH_REQUESTED"
        private const val REQUEST = 0x5346
    }
}
