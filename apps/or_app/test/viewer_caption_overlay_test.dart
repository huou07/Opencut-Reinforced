import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/project/project_gateway.dart';
import 'package:or_app/screens/viewer_caption_overlay.dart';

void main() {
  ProjectTimelineClip caption() => ProjectTimelineClip(
    clipId: 'caption-1',
    contentKind: ProjectTimelineClipContentKind.caption,
    text: 'Xin chào',
    formatting: ProjectTextFormatting.captionDefaults,
    timelineStart: ProjectRationalTime(BigInt.zero, 1),
    timelineDuration: ProjectRationalTime(BigInt.from(3), 1),
  );

  Widget harness(ValueChanged<ProjectTextFormatting> onCommit) => MaterialApp(
    home: Scaffold(
      body: Center(
        child: SizedBox(
          width: 800,
          height: 600,
          child: Stack(
            children: [
              ViewerCaptionOverlay(
                canvasSize: const Size(1920, 1080),
                clip: caption(),
                onCommit: onCommit,
              ),
            ],
          ),
        ),
      ),
    ),
  );

  testWidgets('dragging the caption writes an offset and commits once', (
    tester,
  ) async {
    final committed = <ProjectTextFormatting>[];
    await tester.pumpWidget(harness(committed.add));
    await tester.drag(
      find.byKey(const ValueKey('viewer-caption-move')),
      const Offset(80, 40),
    );
    await tester.pumpAndSettle();
    expect(committed, hasLength(1));
    expect(committed.single.offsetXBasisPoints, closeTo(1000, 2));
    expect(committed.single.offsetYBasisPoints, closeTo(667, 2));
  });

  testWidgets('dragging the corner handle writes a larger font size', (
    tester,
  ) async {
    final committed = <ProjectTextFormatting>[];
    await tester.pumpWidget(harness(committed.add));
    await tester.drag(
      find.byKey(const ValueKey('viewer-caption-resize')),
      const Offset(20, 20),
    );
    await tester.pumpAndSettle();
    expect(committed, hasLength(1));
    expect(committed.single.sizeMilliPoints, greaterThan(48000));
  });
}
