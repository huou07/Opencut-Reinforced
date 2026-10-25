import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/project/project_gateway.dart';
import 'package:or_app/screens/viewer_transform_overlay.dart';

/// The viewer overlay is the one place where a pointer writes model state, so
/// its geometry and its command count are both worth asserting: a wrong box
/// puts the handles somewhere the compositor does not draw, and a drag that
/// commits per sample floods the undo stack.
void main() {
  const display = Size(640, 360);

  ProjectTimelineVisualSettings settings({
    double x = 0,
    double y = 0,
    double scaleX = 1000,
    double scaleY = 1000,
    double rotation = 0,
  }) => withTransform(
    ProjectTimelineVisualSettings.identity,
    xMilliCanvas: x,
    yMilliCanvas: y,
    scaleXMilli: scaleX,
    scaleYMilli: scaleY,
    rotationMilliDegrees: rotation,
  );

  group('transformBox', () {
    test('identity fills the frame', () {
      final box = transformBox(display, settings());
      expect(box.left, closeTo(0, 0.01));
      expect(box.top, closeTo(0, 0.01));
      expect(box.width, closeTo(640, 0.01));
      expect(box.height, closeTo(360, 0.01));
    });

    test('scale shrinks around the centre', () {
      final box = transformBox(display, settings(scaleX: 500, scaleY: 500));
      expect(box.width, closeTo(320, 0.01));
      expect(box.height, closeTo(180, 0.01));
      expect(box.center.dx, closeTo(320, 0.01));
      expect(box.center.dy, closeTo(180, 0.01));
    });

    test('positive x moves right and positive y moves up', () {
      // The renderer draws y upward, so a positive model value must raise the
      // box on screen. Getting this backwards silently mirrors every drag.
      final box = transformBox(display, settings(x: 250, y: 250));
      expect(box.center.dx, closeTo(320 + 160, 0.01));
      expect(box.center.dy, closeTo(180 - 90, 0.01));
    });
  });

  group('withTransform', () {
    test('replaces only the transform fields', () {
      final base = ProjectTimelineVisualSettings(
        xMilliCanvas: 0,
        yMilliCanvas: 0,
        scaleXMilli: 1000,
        scaleYMilli: 1000,
        rotationMilliDegrees: 0,
        anchorXBasisPoints: 5000,
        anchorYBasisPoints: 5000,
        cropLeftBasisPoints: 250,
        cropTopBasisPoints: 0,
        cropRightBasisPoints: 0,
        cropBottomBasisPoints: 0,
        opacityBasisPoints: 4000,
      );
      final next = withTransform(base, xMilliCanvas: 120);
      expect(next.xMilliCanvas, 120);
      expect(next.cropLeftBasisPoints, 250);
      expect(next.opacityBasisPoints, 4000);
      expect(next.scaleXMilli, 1000);
    });
  });

  group('ViewerTransformOverlay gestures', () {
    Future<List<ProjectTimelineVisualSettings>> mount(
      WidgetTester tester, {
      ProjectTimelineVisualSettings? initial,
    }) async {
      final commits = <ProjectTimelineVisualSettings>[];
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: Center(
              child: SizedBox(
                width: display.width,
                height: display.height,
                child: ViewerTransformOverlay(
                  key: const ValueKey('viewer-transform-overlay'),
                  canvasSize: display,
                  settings: initial ?? settings(),
                  onCommit: commits.add,
                ),
              ),
            ),
          ),
        ),
      );
      return commits;
    }

    testWidgets('a move drag commits exactly once', (tester) async {
      final commits = await mount(tester);
      final centre = tester.getCenter(
        find.byKey(const ValueKey('viewer-transform-overlay')),
      );
      final gesture = await tester.startGesture(centre);
      await gesture.moveBy(const Offset(20, 0));
      await gesture.moveBy(const Offset(20, 0));
      await gesture.moveBy(const Offset(20, 0));
      expect(commits, isEmpty, reason: 'a drag in progress sends no command');
      await gesture.up();
      await tester.pumpAndSettle();
      expect(commits, hasLength(1));
      // 60 px of a 640 px canvas is +93.75 milli-canvas.
      expect(commits.single.xMilliCanvas, closeTo(93.75, 0.6));
      expect(commits.single.yMilliCanvas, 0);
    });

    testWidgets('a drag that ends near the centre snaps to it', (
      tester,
    ) async {
      final commits = await mount(tester);
      final centre = tester.getCenter(
        find.byKey(const ValueKey('viewer-transform-overlay')),
      );
      final gesture = await tester.startGesture(centre);
      await gesture.moveBy(const Offset(3, 2));
      await gesture.up();
      await tester.pumpAndSettle();
      expect(commits.single.xMilliCanvas, 0);
      expect(commits.single.yMilliCanvas, 0);
    });
  });
}
