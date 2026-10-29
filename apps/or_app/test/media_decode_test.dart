import 'dart:typed_data';

import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:or_app/screens/media_decode.dart';

void main() {
  final bytes = Uint8List.fromList([1, 2, 3]);

  test('a preview decodes at the box size times the ratio', () {
    final provider = previewImageProvider(
      bytes,
      logicalWidth: 52,
      logicalHeight: 36,
      devicePixelRatio: 2,
    ) as ResizeImage;
    expect(provider.width, 104);
    expect(provider.height, 72);
    expect(provider.allowUpscaling, isFalse);
  });

  test('a fractional ratio rounds up to whole pixels', () {
    final provider = previewImageProvider(
      bytes,
      logicalWidth: 52,
      logicalHeight: 36,
      devicePixelRatio: 1.5,
    ) as ResizeImage;
    expect(provider.width, 78);
    expect(provider.height, 54);
  });

  test('an extreme ratio is capped so memory stays bounded', () {
    final provider = previewImageProvider(
      bytes,
      logicalWidth: 500,
      logicalHeight: 300,
      devicePixelRatio: 9,
    ) as ResizeImage;
    expect(provider.width, 2000);
    expect(provider.height, 1200);
  });
}
