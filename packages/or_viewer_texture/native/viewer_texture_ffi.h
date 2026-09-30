#ifndef OR_VIEWER_TEXTURE_FFI_H_
#define OR_VIEWER_TEXTURE_FFI_H_

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

typedef struct {
  const uint8_t* pixels;
  size_t width;
  size_t height;
  void* release_context;
} OrViewerPixelBuffer;

typedef bool (*OrViewerAcquireLatest)(OrViewerPixelBuffer* output);
typedef void (*OrViewerReleaseFrame)(void* release_context);

typedef struct {
  OrViewerAcquireLatest acquire_latest;
  OrViewerReleaseFrame release_frame;
} OrViewerFfiApi;

#endif
