#ifndef OR_VIEWER_TEXTURE_PLUGIN_H_
#define OR_VIEWER_TEXTURE_PLUGIN_H_

#include <flutter/plugin_registrar_windows.h>
#include <flutter/texture_registrar.h>

#include <cstdint>
#include <memory>

class OrViewerTexturePlugin : public flutter::Plugin {
 public:
  static void RegisterWithRegistrar(flutter::PluginRegistrarWindows* registrar);
  ~OrViewerTexturePlugin() override;

 private:
  explicit OrViewerTexturePlugin(flutter::PluginRegistrarWindows* registrar);
  flutter::TextureRegistrar* texture_registrar_;
  int64_t texture_id_ = -1;
  std::shared_ptr<flutter::TextureVariant> texture_;
};

#endif
