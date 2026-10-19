import 'dart:io';
import 'dart:typed_data';
import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:or_app/main.dart';
import 'package:or_app/project/project_file_picker.dart';
import 'package:or_app/project/project_gateway.dart';
import 'package:or_app/project/rust_project_gateway.dart';
import 'package:or_app/rust_core_gateway.dart';
import 'package:or_app_bridge/or_app_bridge.dart' show RustLib;

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  setUpAll(RustLib.init);

  testWidgets(
    'Linux editor preflight uses real Rust state and semantic actions',
    (tester) async {
      expect(Platform.isLinux, isTrue);
      final directory = Directory.systemTemp.createTempSync('or-ui-preflight-');
      final picker = _FixturePicker(directory);
      final gateway = _ObservedGateway();
      final artifacts = Directory(
        Platform.environment['OR_UI_ARTIFACT_DIR'] ??
            '${directory.path}/evidence',
      )..createSync(recursive: true);
      final boundary = GlobalKey();
      final wav = ByteData(16044);
      void ascii(int offset, String value) {
        for (var i = 0; i < value.length; i++) {
          wav.setUint8(offset + i, value.codeUnitAt(i));
        }
      }

      ascii(0, 'RIFF');
      wav.setUint32(4, 16036, Endian.little);
      ascii(8, 'WAVEfmt ');
      wav.setUint32(16, 16, Endian.little);
      wav.setUint16(20, 1, Endian.little);
      wav.setUint16(22, 1, Endian.little);
      wav.setUint32(24, 8000, Endian.little);
      wav.setUint32(28, 16000, Endian.little);
      wav.setUint16(32, 2, Endian.little);
      wav.setUint16(34, 16, Endian.little);
      ascii(36, 'data');
      wav.setUint32(40, 16000, Endian.little);
      File(picker.media).writeAsBytesSync(wav.buffer.asUint8List());
      File(picker.badMedia).writeAsStringSync('synthetic invalid media');

      await tester.pumpWidget(
        RepaintBoundary(
          key: boundary,
          child: OrApp(
            gateway: const RustCoreGateway(),
            projectGateway: gateway,
            projectFilePicker: picker,
          ),
        ),
      );
      Future<void> tap(String key) async {
        final finder = find.byKey(ValueKey(key));
        await tester.ensureVisible(finder);
        await tester.tap(finder);
        await tester.pumpAndSettle();
      }

      Future<void> waitFor(bool Function() condition, String operation) async {
        for (var i = 0; i < 200; i++) {
          await tester.pump(const Duration(milliseconds: 50));
          if (condition()) return;
          await tester.runAsync(
            () => Future<void>.delayed(const Duration(milliseconds: 20)),
          );
        }
        fail('Timed out: $operation');
      }

      Future<void> capture(String state) async {
        await tester.pumpAndSettle();
        final render =
            boundary.currentContext!.findRenderObject()!
                as RenderRepaintBoundary;
        final image = await render.toImage(pixelRatio: 1);
        final bytes = await image.toByteData(format: ui.ImageByteFormat.png);
        File('${artifacts.path}/$state.png')
            .writeAsBytesSync(bytes!.buffer.asUint8List());
        image.dispose();
      }

      try {
        await tap('home-new-project');
        await tester.enterText(
          find.byKey(const ValueKey('new-project-name')),
          'Linux preflight',
        );
        await tap('confirm-new-project');
        await waitFor(
          () =>
              gateway.session != null &&
              find.text('No timeline tracks').evaluate().isNotEmpty,
          'new project',
        );
        final session = gateway.session!;
        expect((await gateway.summary(session)).name, 'Linux preflight');
        final semantics = tester.ensureSemantics();
        try {
          for (final key in [
            'timeline-empty-import-media',
            'timeline-empty-add-audio',
          ]) {
            final node = tester.getSemantics(find.byKey(ValueKey(key)));
            expect(node.flagsCollection.isButton, isTrue);
            expect(node.label, isNotEmpty);
            expect(
              node.getSemanticsData().hasAction(ui.SemanticsAction.tap),
              isTrue,
            );
          }
          picker.nextMedia = picker.badMedia;
          await tap('timeline-empty-import-media');
          await waitFor(
            () => find.byType(SnackBar).evaluate().isNotEmpty,
            'invalid media feedback',
          );
          expect(
            (await gateway.listMediaPage(session, offset: 0, limit: 10)).items,
            isEmpty,
          );
          picker.nextMedia = picker.media;
          await tap('timeline-empty-import-media');
          await waitFor(
            () => find.text('fixture.wav').evaluate().isNotEmpty,
            'retry import',
          );
          final media = (await gateway.listMediaPage(
            session,
            offset: 0,
            limit: 10,
          )).items.single;
          await tap('timeline-add-audio-track');
          final track = (await gateway.listTimelineTracks(session))
              .items
              .single;
          expect(track.kind, ProjectTimelineTrackKind.audio);
          await tap('media-add-timeline-${media.mediaId}');
          await tap('timeline-confirm-insert');
          await waitFor(
            () => find
                .byKey(ValueKey('timeline-track-lock-${track.trackId}'))
                .evaluate()
                .isNotEmpty,
            'timeline insertion',
          );
          var clips = (await gateway.listTimelineClips(
            session,
            trackId: track.trackId,
            offset: 0,
            limit: 10,
          )).items;
          expect(clips, hasLength(1));
          expect(clips.single.mediaId, media.mediaId);
          expect(clips.single.timelineDuration.canonical, '1/1');
          final clip = find.bySemanticsLabel(
            RegExp('fixture.wav, starts 0/1, duration 1/1'),
          );
          final node = tester.getSemantics(clip);
          File('${artifacts.path}/clip-semantics.txt')
              .writeAsStringSync(node.toStringDeep());
          expect(node.flagsCollection.isButton, isTrue);
          expect(
            node.getSemanticsData().hasAction(ui.SemanticsAction.tap),
            isTrue,
          );
          await tester.tap(
            find.byKey(ValueKey('timeline-clip-${clips.single.clipId}')),
          );
          await tester.pumpAndSettle();
          await tester.tap(
            find.descendant(
              of: find.byType(AlertDialog),
              matching: find.widgetWithText(TextButton, 'Close'),
            ),
          );
          await tester.pumpAndSettle();
          await tester.sendKeyDownEvent(LogicalKeyboardKey.controlLeft);
          await tester.sendKeyEvent(LogicalKeyboardKey.keyD);
          await tester.sendKeyUpEvent(LogicalKeyboardKey.controlLeft);
          await tester.pumpAndSettle();
          clips = (await gateway.listTimelineClips(
            session,
            trackId: track.trackId,
            offset: 0,
            limit: 10,
          )).items;
          expect(clips, hasLength(2));
          await tap('timeline-undo');
          clips = (await gateway.listTimelineClips(
            session,
            trackId: track.trackId,
            offset: 0,
            limit: 10,
          )).items;
          expect(clips, hasLength(1));
          await capture('editor-selected');
          final ruler = find.byKey(const ValueKey('preview-scrub-ruler'));
          await tester.tapAt(tester.getCenter(ruler));
          await tester.pumpAndSettle();
          expect(
            (await gateway.previewState(session)).position.secondsForDisplay,
            greaterThan(0),
          );
          await tap('export-project');
          expect(find.text('Export video'), findsOneWidget);
          await capture('export-setup');
          await tester.tap(find.widgetWithText(TextButton, 'Cancel'));
          await tester.pumpAndSettle();
          expect(find.text('Export video'), findsNothing);
          expect(tester.takeException(), isNull);
        } finally {
          semantics.dispose();
        }
        expect((await gateway.save(session)).succeeded, isTrue);
        await gateway.close(session, discardUnsaved: false);
        gateway.session = null;
        final reopened = await gateway.openProject(picker.project);
        expect(
          (await gateway.listMediaPage(reopened, offset: 0, limit: 10)).items,
          hasLength(1),
        );
        expect(
          (await gateway.listTimelineTracks(reopened)).items.single.clipCount,
          1,
        );
      } finally {
        await tester.pumpWidget(const SizedBox.shrink());
        if (gateway.session != null) {
          await gateway.close(gateway.session!, discardUnsaved: true);
        }
        directory.deleteSync(recursive: true);
      }
    },
  );
}

class _ObservedGateway extends RustProjectGateway {
  ProjectSessionHandle? session;
  @override
  Future<ProjectSessionHandle> createProject(String path, String name) async =>
      session = await super.createProject(path, name);
  @override
  Future<ProjectSessionHandle> openProject(String path) async =>
      session = await super.openProject(path);
}

class _FixturePicker extends FileSelectorProjectPicker {
  _FixturePicker(this.directory);
  final Directory directory;
  String get project => '${directory.path}/fixture.orproj';
  String get media => '${directory.path}/fixture.wav';
  String get badMedia => '${directory.path}/invalid.wav';
  String? nextMedia;
  @override
  Future<String?> saveProjectPath({required String suggestedName}) async =>
      project;
  @override
  Future<String?> openProjectPath() async => project;
  @override
  Future<List<String>> openMediaSources() async => [nextMedia ?? media];
}
