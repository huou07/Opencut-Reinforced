#define FLUTTER_PLUGIN_IMPL
#include "or_viewer_texture/or_viewer_texture_plugin_c_api.h"

#include "or_viewer_texture_plugin.h"

void OrViewerTexturePluginCApiRegisterWithRegistrar(
    FlutterDesktopPluginRegistrarRef registrar) {
  OrViewerTexturePlugin::RegisterWithRegistrar(
      flutter::PluginRegistrarManager::GetInstance()
          ->GetRegistrar<flutter::PluginRegistrarWindows>(registrar));
}
