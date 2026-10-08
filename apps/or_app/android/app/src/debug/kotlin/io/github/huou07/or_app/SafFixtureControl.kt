package io.github.huou07.or_app

import android.app.Activity
import android.content.Context
import android.content.Intent
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
    override fun onAttachedToEngine(binding: FlutterPlugin.FlutterPluginBinding) {
        channel = MethodChannel(binding.binaryMessenger, "or_saf_acceptance_fixture").also { channel ->
            channel.setMethodCallHandler { call, result ->
                if (call.method != "control") result.notImplemented()
                else if (pending != null) result.error("FIXTURE_BUSY", "A fixture operation is pending.", null)
                else {
                    val activity = this.binding?.activity
                    val operation = (call.arguments as? Map<*, *>)?.get("operation")
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
                        activity.startActivityForResult(intent, REQUEST)
                    } catch (error: Exception) { pending = null; result.error("FIXTURE_UNAVAILABLE", error.toString(), null) }
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
        pending = null
        if (resultCode != Activity.RESULT_OK) result.error("FIXTURE_FAILED", data?.getStringExtra("error") ?: "Fixture cancelled.", null)
        else {
            val json = JSONObject(data!!.getStringExtra("data")!!)
            val values = json.keys().asSequence().associateWith { json.get(it) }.toMutableMap()
            values["appUid"] = Process.myUid()
            result.success(values)
        }
        return true
    }
    companion object {
        const val BACKGROUND_SIGNAL_FILE = "or-saf-background-requested"
        private const val BACKGROUND_SIGNAL = "ANDROID_SAF_BACKGROUND_RELAUNCH_REQUESTED"
        private const val REQUEST = 0x5346
    }
}
