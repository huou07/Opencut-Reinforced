#ifndef OR_VIEWER_TEXTURE_PLUGIN_C_API_H_
#define OR_VIEWER_TEXTURE_PLUGIN_C_API_H_

#include <flutter_plugin_registrar.h>

#ifdef FLUTTER_PLUGIN_IMPL
#define OR_VIEWER_TEXTURE_PLUGIN_EXPORT __declspec(dllexport)
#else
#define OR_VIEWER_TEXTURE_PLUGIN_EXPORT __declspec(dllimport)
#endif

#ifdef __cplusplus
extern "C" {
#endif

OR_VIEWER_TEXTURE_PLUGIN_EXPORT void OrViewerTexturePluginCApiRegisterWithRegistrar(
    FlutterDesktopPluginRegistrarRef registrar);

#ifdef __cplusplus
}
#endif

#endif
