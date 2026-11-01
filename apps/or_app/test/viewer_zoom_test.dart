import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/screens/editor_viewer_panel.dart';

void main() {
  test('fit keeps the preview at the box it was laid out in', () {
    expect(viewerZoomScale(ViewerZoomLevel.fit, 1920, 960), 1);
  });

  test('percentage zoom maps source pixels through the fit scale', () {
    // A 1920-wide preview laid out at 960 logical pixels is 50% already.
    expect(viewerZoomScale(ViewerZoomLevel.x25, 1920, 960), 0.5);
    expect(viewerZoomScale(ViewerZoomLevel.x50, 1920, 960), 1);
    expect(viewerZoomScale(ViewerZoomLevel.x100, 1920, 960), 2);
    expect(viewerZoomScale(ViewerZoomLevel.x200, 1920, 960), 4);
  });

  test('degenerate sizes fall back to fit instead of NaN or infinity', () {
    expect(viewerZoomScale(ViewerZoomLevel.x100, 0, 960), 1);
    expect(viewerZoomScale(ViewerZoomLevel.x100, 1920, 0), 1);
  });
}
