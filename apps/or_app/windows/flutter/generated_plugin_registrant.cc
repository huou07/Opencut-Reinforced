//
//  Generated file. Do not edit.
//

// clang-format off

#include "generated_plugin_registrant.h"

#include <file_selector_windows/file_selector_windows.h>
#include <or_viewer_texture/or_viewer_texture_plugin_c_api.h>

void RegisterPlugins(flutter::PluginRegistry* registry) {
  FileSelectorWindowsRegisterWithRegistrar(
      registry->GetRegistrarForPlugin("FileSelectorWindows"));
  OrViewerTexturePluginCApiRegisterWithRegistrar(
      registry->GetRegistrarForPlugin("OrViewerTexturePluginCApi"));
}
