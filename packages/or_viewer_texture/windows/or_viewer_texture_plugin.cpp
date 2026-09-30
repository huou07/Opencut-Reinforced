#include "or_viewer_texture_plugin.h"

#include <windows.h>

#include <limits>
#include <memory>

#include "../native/viewer_texture_ffi.h"
#include <flutter/texture_registrar.h>

namespace {
constexpr size_t kMaxFrameBytes = 256 * 1024 * 1024;

struct ReleaseContext {
  FlutterDesktopPixelBuffer buffer{};
  OrViewerPixelBuffer frame{};
  OrViewerReleaseFrame release = nullptr;
};

bool load_ffi(OrViewerFfiApi* api) {
  HMODULE bridge = GetModuleHandleW(L"or_app_bridge.dll");
  if (bridge == nullptr) bridge = GetModuleHandleW(L"libor_app_bridge.dll");
  if (bridge == nullptr) return false;
  api->acquire_latest = reinterpret_cast<OrViewerAcquireLatest>(
      GetProcAddress(bridge, "or_viewer_acquire_latest"));
  api->release_frame = reinterpret_cast<OrViewerReleaseFrame>(
      GetProcAddress(bridge, "or_viewer_release_frame"));
  return api->acquire_latest != nullptr && api->release_frame != nullptr;
}

void release_pixel_buffer(void* opaque) {
  std::unique_ptr<ReleaseContext> owner(static_cast<ReleaseContext*>(opaque));
  owner->release(owner->frame.release_context);
}

const FlutterDesktopPixelBuffer* copy_pixel_buffer(size_t, size_t) {
  OrViewerFfiApi api{};
  if (!load_ffi(&api)) return nullptr;
  auto* owner = new ReleaseContext();
  if (!api.acquire_latest(&owner->frame)) {
    delete owner;
    return nullptr;
  }
  owner->release = api.release_frame;
  if (owner->frame.pixels == nullptr || owner->frame.width == 0 ||
      owner->frame.height == 0 ||
      owner->frame.width >
          (std::numeric_limits<size_t>::max)() / owner->frame.height / 4 ||
      owner->frame.width * owner->frame.height * 4 > kMaxFrameBytes) {
    api.release_frame(owner->frame.release_context);
    delete owner;
    return nullptr;
  }
  owner->buffer.buffer = owner->frame.pixels;
  owner->buffer.width = owner->frame.width;
  owner->buffer.height = owner->frame.height;
  owner->buffer.release_callback = release_pixel_buffer;
  owner->buffer.release_context = owner;
  return &owner->buffer;
}
}  // namespace

void OrViewerTexturePlugin::RegisterWithRegistrar(
    flutter::PluginRegistrarWindows* registrar) {
  registrar->AddPlugin(std::unique_ptr<flutter::Plugin>(
      new OrViewerTexturePlugin(registrar)));
}

OrViewerTexturePlugin::OrViewerTexturePlugin(
    flutter::PluginRegistrarWindows* registrar)
    : texture_registrar_(registrar->texture_registrar()) {
  texture_ = std::make_shared<flutter::TextureVariant>(
      flutter::PixelBufferTexture(copy_pixel_buffer));
  texture_id_ = texture_registrar_->RegisterTexture(texture_.get());
}

OrViewerTexturePlugin::~OrViewerTexturePlugin() {
  if (texture_id_ >= 0) {
    auto texture = texture_;
    texture_registrar_->UnregisterTexture(texture_id_, [texture] {});
  }
}
