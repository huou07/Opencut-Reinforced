#include <flutter_linux/flutter_linux.h>

#include <dlfcn.h>
#include <link.h>

#include <algorithm>
#include <cstdint>
#include <mutex>
#include <string>
#include <utility>
#include <vector>

#include "../native/viewer_texture_ffi.h"

namespace {
constexpr size_t kMaxRetainedFrames = 3;

struct RetainedFrame {
  OrViewerPixelBuffer lease{};
  std::vector<uint8_t> rgba;
  OrViewerReleaseFrame release_frame = nullptr;
};

typedef struct _OrViewerTexture OrViewerTexture;
typedef struct _OrViewerTextureClass OrViewerTextureClass;

struct _OrViewerTexture {
  FlPixelBufferTexture parent_instance;
  std::mutex* mutex;
  std::vector<RetainedFrame>* retained;
};

struct _OrViewerTextureClass {
  FlPixelBufferTextureClass parent_class;
};

G_DEFINE_TYPE(OrViewerTexture, or_viewer_texture, fl_pixel_buffer_texture_get_type())

struct LibrarySearch {
  void* handle = nullptr;
};

int find_bridge(struct dl_phdr_info* info, size_t, void* data) {
  if (info->dlpi_name == nullptr || std::string(info->dlpi_name).find("or_app_bridge") == std::string::npos) {
    return 0;
  }
  auto* search = static_cast<LibrarySearch*>(data);
  search->handle = dlopen(info->dlpi_name, RTLD_NOW | RTLD_NOLOAD);
  return search->handle == nullptr ? 0 : 1;
}

bool load_ffi(OrViewerFfiApi* api) {
  LibrarySearch search;
  dl_iterate_phdr(find_bridge, &search);
  if (search.handle == nullptr) return false;
  api->acquire_latest = reinterpret_cast<OrViewerAcquireLatest>(dlsym(search.handle, "or_viewer_acquire_latest"));
  api->release_frame = reinterpret_cast<OrViewerReleaseFrame>(dlsym(search.handle, "or_viewer_release_frame"));
  const bool found = api->acquire_latest != nullptr && api->release_frame != nullptr;
  dlclose(search.handle);
  return found;
}

gboolean copy_pixels(FlPixelBufferTexture* texture,
                     const uint8_t** pixels,
                     uint32_t* width,
                     uint32_t* height,
                     GError**) {
  auto* self = reinterpret_cast<_OrViewerTexture*>(texture);
  std::lock_guard<std::mutex> guard(*self->mutex);
  if (self->retained->size() >= kMaxRetainedFrames) return FALSE;

  OrViewerFfiApi api{};
  if (!load_ffi(&api)) return FALSE;
  RetainedFrame frame;
  if (!api.acquire_latest(&frame.lease)) return FALSE;
  frame.release_frame = api.release_frame;
  if (frame.lease.pixels == nullptr ||
      frame.lease.width == 0 || frame.lease.height == 0 ||
      frame.lease.width > UINT32_MAX || frame.lease.height > UINT32_MAX ||
      frame.lease.width > SIZE_MAX / frame.lease.height / 4) {
    api.release_frame(frame.lease.release_context);
    return FALSE;
  }
  const size_t byte_count = frame.lease.width * frame.lease.height * 4;
  frame.rgba.resize(byte_count);
  for (size_t i = 0; i < byte_count; i += 4) {
    frame.rgba[i] = frame.lease.pixels[i + 2];
    frame.rgba[i + 1] = frame.lease.pixels[i + 1];
    frame.rgba[i + 2] = frame.lease.pixels[i];
    frame.rgba[i + 3] = frame.lease.pixels[i + 3];
  }
  *pixels = frame.rgba.data();
  *width = static_cast<uint32_t>(frame.lease.width);
  *height = static_cast<uint32_t>(frame.lease.height);
  self->retained->push_back(std::move(frame));
  return TRUE;
}

void finalize(GObject* object) {
  auto* self = reinterpret_cast<_OrViewerTexture*>(object);
  for (const auto& frame : *self->retained) {
    if (frame.release_frame != nullptr) {
      frame.release_frame(frame.lease.release_context);
    }
  }
  delete self->retained;
  delete self->mutex;
  G_OBJECT_CLASS(or_viewer_texture_parent_class)->finalize(object);
}

void or_viewer_texture_class_init(_OrViewerTextureClass* klass) {
  FL_PIXEL_BUFFER_TEXTURE_CLASS(klass)->copy_pixels = copy_pixels;
  G_OBJECT_CLASS(klass)->finalize = finalize;
}

void or_viewer_texture_init(_OrViewerTexture* self) {
  self->mutex = new std::mutex();
  self->retained = new std::vector<RetainedFrame>();
}

struct PluginState {
  FlTextureRegistrar* registrar;
  _OrViewerTexture* texture;
};

void destroy_plugin(gpointer data) {
  auto* state = static_cast<PluginState*>(data);
  fl_texture_registrar_unregister_texture(state->registrar, FL_TEXTURE(state->texture));
  g_object_unref(state->texture);
  g_object_unref(state->registrar);
  delete state;
}
}  // namespace

extern "C" G_MODULE_EXPORT void or_viewer_texture_plugin_register_with_registrar(
    FlPluginRegistrar* registrar) {
  auto* texture_registrar = fl_plugin_registrar_get_texture_registrar(registrar);
  auto* texture = reinterpret_cast<_OrViewerTexture*>(
      g_object_new(or_viewer_texture_get_type(), nullptr));
  if (!fl_texture_registrar_register_texture(texture_registrar, FL_TEXTURE(texture))) {
    g_object_unref(texture);
    return;
  }
  auto* state = new PluginState{
      FL_TEXTURE_REGISTRAR(g_object_ref(texture_registrar)), texture};
  g_object_set_data_full(G_OBJECT(registrar), "or-viewer-texture-state", state,
                         destroy_plugin);
}
