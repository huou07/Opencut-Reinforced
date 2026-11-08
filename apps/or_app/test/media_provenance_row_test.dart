import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:or_app/project/project_gateway.dart';
import 'package:or_app/screens/editor_media_library.dart';

ProjectMediaItem _item({String? tool, String? version}) => ProjectMediaItem(
  mediaId: 'm1',
  sourceUri: 'file:///tmp/shot.mp4',
  formatNames: const ['mp4'],
  duration: '4 s',
  videoDetails: '1920×1080 h264',
  audioDetails: null,
  provenanceTool: tool,
  provenanceVersion: version,
);

Widget _wrap(ProjectMediaItem item) => MaterialApp(
  home: Scaffold(
    body: EditorMediaLibraryItem(
      item: item,
      preview: null,
      onAddToTimeline: () {},
      onRelink: () {},
      onRemove: () {},
      onGenerateCaptions: () {},
      onCleanUpAudio: null,
    ),
  ),
);

void main() {
  testWidgets('a companion result says which tool produced it', (tester) async {
    await tester.pumpWidget(_wrap(_item(tool: 'Blender', version: '4.3.2')));
    expect(find.textContaining('From Blender 4.3.2'), findsOneWidget);
  });

  testWidgets('ordinary media carries no provenance line', (tester) async {
    await tester.pumpWidget(_wrap(_item()));
    expect(find.textContaining('From '), findsNothing);
  });
}
