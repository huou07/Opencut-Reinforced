import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:or_app/project/project_gateway.dart';
import 'package:or_app/screens/editor_media_library.dart';

ProjectMediaItem _item({
  String? tool,
  String? version,
  String? sourceProject,
  String? manifest,
  String? command,
  String? output,
  String? record,
}) => ProjectMediaItem(
  mediaId: 'm1',
  sourceUri: 'file:///tmp/shot.mp4',
  formatNames: const ['mp4'],
  duration: '4 s',
  videoDetails: '1920×1080 h264',
  audioDetails: null,
  provenanceTool: tool,
  provenanceVersion: version,
  provenanceSourceProject: sourceProject,
  provenanceManifest: manifest,
  provenanceCommand: command,
  provenanceOutput: output,
  provenanceRecord: record,
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

  testWidgets('ordinary media offers no provenance action', (tester) async {
    await tester.pumpWidget(_wrap(_item()));
    expect(
      find.byKey(const ValueKey('media-provenance-m1')),
      findsNothing,
      reason: 'a plain import must not claim an origin it does not have',
    );
  });

  testWidgets('the origin dialog keeps source, command and record', (
    tester,
  ) async {
    final item = _item(
      tool: 'Blender',
      version: '4.3.2',
      sourceProject: '/work/shot.blend',
      manifest: '/work/shot.manifest.json',
      command: 'blender -b scene.blend -o //shot -F PNG -f 1',
      output: '/work/shot.mp4',
      record: '/work/shot.mp4.orprovenance.json',
    );
    await tester.pumpWidget(_wrap(item));
    await tester.tap(find.byKey(const ValueKey('media-actions-m1')));
    await tester.pumpAndSettle();
    expect(find.text('Where This Came From…'), findsOneWidget);
    await tester.tap(find.byKey(const ValueKey('media-provenance-m1')));
    await tester.pumpAndSettle();

    expect(find.text('Made by Blender 4.3.2'), findsOneWidget);
    expect(
      find.byKey(const ValueKey('media-provenance-source-project')),
      findsOneWidget,
    );
    expect(
      tester
          .widget<SelectableText>(
            find.byKey(const ValueKey('media-provenance-source-project')),
          )
          .data,
      '/work/shot.blend',
    );
    expect(
      tester
          .widget<SelectableText>(
            find.byKey(const ValueKey('media-provenance-manifest')),
          )
          .data,
      '/work/shot.manifest.json',
    );
    expect(
      tester
          .widget<SelectableText>(
            find.byKey(const ValueKey('media-provenance-command')),
          )
          .data,
      'blender -b scene.blend -o //shot -F PNG -f 1',
    );
    expect(
      tester
          .widget<SelectableText>(
            find.byKey(const ValueKey('media-provenance-record')),
          )
          .data,
      '/work/shot.mp4.orprovenance.json',
    );
    await tester.tap(find.byKey(const ValueKey('media-provenance-close')));
    await tester.pumpAndSettle();
    expect(find.byKey(const ValueKey('media-provenance-dialog')), findsNothing);
  });

  testWidgets('a record without a source project still shows its command', (
    tester,
  ) async {
    await tester.pumpWidget(
      _wrap(_item(tool: 'FFmpeg', version: '7.1.5', command: 'ffmpeg -i a.mov b.mp4')),
    );
    await tester.tap(find.byKey(const ValueKey('media-actions-m1')));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('media-provenance-m1')));
    await tester.pumpAndSettle();
    expect(
      find.byKey(const ValueKey('media-provenance-source-project')),
      findsNothing,
    );
    expect(
      find.byKey(const ValueKey('media-provenance-command')),
      findsOneWidget,
    );
  });
}
