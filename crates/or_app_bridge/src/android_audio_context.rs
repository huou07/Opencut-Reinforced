use jni::{
    EnvUnowned,
    errors::ThrowRuntimeExAndDefault,
    objects::{Global, JObject},
    sys::jboolean,
};
use std::sync::{Mutex, OnceLock};

static AUDIO_CONTEXT: OnceLock<Mutex<Option<Global<JObject<'static>>>>> = OnceLock::new();

/// Installs the app context CPAL's AAudio backend requires when Rust is loaded
/// through Flutter FFI instead of an Android Rust application runtime.
#[unsafe(no_mangle)]
pub extern "system" fn Java_io_github_huou07_or_1app_MainActivity_initializeAudioContext<'local>(
    mut env: EnvUnowned<'local>,
    _activity: JObject<'local>,
    context: JObject<'local>,
) -> jboolean {
    env.with_env(|env| -> jni::errors::Result<bool> {
        let stored_context = AUDIO_CONTEXT.get_or_init(|| Mutex::new(None));
        let mut stored_context = stored_context
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if stored_context.is_some() {
            return Ok(true);
        }

        let global_context = env.new_global_ref(&context)?;
        let java_vm = env.get_java_vm()?;
        // CPAL reads this process-lifetime context when it opens the AAudio
        // stream. Keep its global reference alive for the same lifetime.
        unsafe {
            ndk_context::initialize_android_context(
                java_vm.get_raw().cast(),
                global_context.as_obj().as_raw().cast(),
            );
        }
        *stored_context = Some(global_context);
        Ok(true)
    })
    .resolve::<ThrowRuntimeExAndDefault>() as jboolean
}
