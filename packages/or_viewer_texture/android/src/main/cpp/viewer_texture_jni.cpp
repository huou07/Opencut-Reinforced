#include <jni.h>

#include <dlfcn.h>

#include <cstddef>
#include <cstdint>
#include <mutex>

namespace {
constexpr size_t kMaxFrameBytes = 16 * 1024 * 1024;
constexpr size_t kMaxWidth = 1920;
constexpr size_t kMaxHeight = 1080;

struct OrViewerPixelBuffer {
  const uint8_t* pixels;
  size_t width;
  size_t height;
  void* release_context;
};

using OrViewerAcquireLatest = bool (*)(OrViewerPixelBuffer*);
using OrViewerReleaseFrame = void (*)(void*);
using OrMediaRegisterSeekableFd = bool (*)(const char*, int32_t);
using OrMediaClearSeekableFds = void (*)();
using OrViewerFrameGeneration = uint64_t (*)(const void*);
using OrViewerCurrentGeneration = uint64_t (*)();
using OrResourceCount = size_t (*)();

struct BridgeApi {
  OrViewerAcquireLatest acquire_latest = nullptr;
  OrViewerReleaseFrame release_frame = nullptr;
  OrMediaRegisterSeekableFd register_media_fd = nullptr;
  OrMediaClearSeekableFds clear_media_fds = nullptr;
  OrViewerFrameGeneration frame_generation = nullptr;
  OrViewerCurrentGeneration current_generation = nullptr;
  OrResourceCount in_flight_frames = nullptr;
  OrResourceCount latest_frame_bytes = nullptr;
  OrResourceCount media_fd_count = nullptr;
};

bool resolve_api(BridgeApi* api) {
  static std::mutex mutex;
  static void* bridge = nullptr;
  std::lock_guard<std::mutex> lock(mutex);
  if (bridge == nullptr) {
    bridge = dlopen("libor_app_bridge.so", RTLD_NOW);
    if (bridge == nullptr) return false;
  }
  api->acquire_latest = reinterpret_cast<OrViewerAcquireLatest>(
      dlsym(bridge, "or_viewer_acquire_latest"));
  api->release_frame = reinterpret_cast<OrViewerReleaseFrame>(
      dlsym(bridge, "or_viewer_release_frame"));
  api->register_media_fd = reinterpret_cast<OrMediaRegisterSeekableFd>(
      dlsym(bridge, "or_media_register_seekable_fd"));
  api->clear_media_fds = reinterpret_cast<OrMediaClearSeekableFds>(
      dlsym(bridge, "or_media_clear_seekable_fds"));
  api->frame_generation = reinterpret_cast<OrViewerFrameGeneration>(dlsym(bridge, "or_viewer_frame_generation"));
  api->current_generation = reinterpret_cast<OrViewerCurrentGeneration>(dlsym(bridge, "or_viewer_current_generation"));
  api->in_flight_frames = reinterpret_cast<OrResourceCount>(dlsym(bridge, "or_viewer_in_flight_frames"));
  api->latest_frame_bytes = reinterpret_cast<OrResourceCount>(dlsym(bridge, "or_viewer_latest_frame_bytes"));
  api->media_fd_count = reinterpret_cast<OrResourceCount>(dlsym(bridge, "or_media_seekable_fd_count"));
  return api->acquire_latest != nullptr && api->release_frame != nullptr &&
      api->register_media_fd != nullptr && api->clear_media_fds != nullptr &&
      api->frame_generation != nullptr && api->current_generation != nullptr &&
      api->in_flight_frames != nullptr && api->latest_frame_bytes != nullptr && api->media_fd_count != nullptr;
}

jobject acquire_latest(JNIEnv* env, jobject) {
  BridgeApi api;
  if (!resolve_api(&api)) return nullptr;

  OrViewerPixelBuffer frame{};
  if (!api.acquire_latest(&frame)) return nullptr;

  const bool size_valid = frame.width > 0 && frame.height > 0 &&
                          frame.width <= kMaxWidth && frame.height <= kMaxHeight &&
                          frame.width <= SIZE_MAX / frame.height / 4 &&
                          frame.width * frame.height * 4 <= kMaxFrameBytes;
  if (!size_valid || frame.pixels == nullptr || frame.release_context == nullptr) {
    api.release_frame(frame.release_context);
    return nullptr;
  }

  auto byte_count = static_cast<jlong>(frame.width * frame.height * 4);
  jobject pixels = env->NewDirectByteBuffer(
      const_cast<uint8_t*>(frame.pixels), byte_count);
  if (pixels == nullptr) {
    api.release_frame(frame.release_context);
    return nullptr;
  }

  jclass frame_class = env->FindClass("dev/opencut/viewertexture/AndroidViewerFrame");
  if (frame_class == nullptr) {
    api.release_frame(frame.release_context);
    return nullptr;
  }
  jmethodID constructor = env->GetMethodID(
      frame_class, "<init>", "(Ljava/nio/ByteBuffer;IIJJ)V");
  if (constructor == nullptr) {
    api.release_frame(frame.release_context);
    return nullptr;
  }
  jobject result = env->NewObject(
      frame_class, constructor, pixels, static_cast<jint>(frame.width),
      static_cast<jint>(frame.height),
      static_cast<jlong>(reinterpret_cast<uintptr_t>(frame.release_context)),
      static_cast<jlong>(api.frame_generation(frame.release_context)));
  env->DeleteLocalRef(pixels);
  env->DeleteLocalRef(frame_class);
  if (result == nullptr) api.release_frame(frame.release_context);
  return result;
}

void release_frame(JNIEnv* env, jobject, jlong context) {
  BridgeApi api;
  if (context != 0 && resolve_api(&api)) {
    api.release_frame(reinterpret_cast<void*>(static_cast<uintptr_t>(context)));
  }
}

jboolean register_media_fd(JNIEnv* env, jobject, jstring uri, jint fd) {
  BridgeApi api;
  if (uri == nullptr || fd < 0 || !resolve_api(&api) || api.register_media_fd == nullptr) {
    return JNI_FALSE;
  }
  const char* chars = env->GetStringUTFChars(uri, nullptr);
  if (chars == nullptr) return JNI_FALSE;
  const bool registered = api.register_media_fd(chars, fd);
  env->ReleaseStringUTFChars(uri, chars);
  return registered ? JNI_TRUE : JNI_FALSE;
}

jboolean clear_media_fds(JNIEnv*, jobject) {
  BridgeApi api;
  if (resolve_api(&api) && api.clear_media_fds != nullptr) {
    api.clear_media_fds();
    return JNI_TRUE;
  }
  return JNI_FALSE;
}

}  // namespace

extern "C" JNIEXPORT jobject JNICALL
Java_dev_opencut_viewertexture_OrViewerTexturePlugin_nativeAcquireLatest(
    JNIEnv* env, jobject object) {
  return acquire_latest(env, object);
}

extern "C" JNIEXPORT void JNICALL
Java_dev_opencut_viewertexture_OrViewerTexturePlugin_nativeReleaseFrame(
    JNIEnv* env, jobject object, jlong context) {
  release_frame(env, object, context);
}

extern "C" JNIEXPORT jboolean JNICALL
Java_dev_opencut_viewertexture_OrViewerTexturePlugin_nativeRegisterMediaFd(
    JNIEnv* env, jobject object, jstring uri, jint fd) {
  return register_media_fd(env, object, uri, fd);
}

extern "C" JNIEXPORT jboolean JNICALL
Java_dev_opencut_viewertexture_OrViewerTexturePlugin_nativeClearMediaFds(
    JNIEnv* env, jobject object) {
  return clear_media_fds(env, object);
}

extern "C" JNIEXPORT jlongArray JNICALL
Java_dev_opencut_viewertexture_OrViewerTexturePlugin_nativeResourceSnapshot(JNIEnv* env, jobject) {
  BridgeApi api;
  if (!resolve_api(&api)) return nullptr;
  jlong values[] = {static_cast<jlong>(api.current_generation()),
      static_cast<jlong>(api.in_flight_frames()), static_cast<jlong>(api.latest_frame_bytes()),
      static_cast<jlong>(api.media_fd_count())};
  jlongArray result = env->NewLongArray(4);
  if (result != nullptr) env->SetLongArrayRegion(result, 0, 4, values);
  return result;
}
