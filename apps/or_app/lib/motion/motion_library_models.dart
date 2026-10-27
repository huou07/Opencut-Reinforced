import 'package:flutter/foundation.dart';

/// The data a motion asset card carries while it is dragged onto the timeline or
/// into the composition preview.
@immutable
class MotionAssetDragData {
  const MotionAssetDragData({
    required this.assetId,
    required this.name,
    required this.valuesJson,
    this.previewPath = '',
  });

  final String assetId;
  final String name;
  final String valuesJson;
  final String previewPath;
}
