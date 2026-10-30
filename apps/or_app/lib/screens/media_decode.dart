import 'dart:typed_data';

import 'package:flutter/widgets.dart';

/// The image provider for a media preview of a known on-screen box.
///
/// Decoding a stored thumbnail at its source resolution for a small tile costs
/// tens of megabytes per library page; the provider is bounded by the box size
/// times the device pixel ratio instead.
ImageProvider previewImageProvider(
  Uint8List bytes, {
  required double logicalWidth,
  required double logicalHeight,
  required double devicePixelRatio,
  double maxScale = 4,
}) {
  final scale = devicePixelRatio.clamp(1, maxScale);
  return ResizeImage(
    MemoryImage(bytes),
    width: (logicalWidth * scale).round(),
    height: (logicalHeight * scale).round(),
    allowUpscaling: false,
  );
}
