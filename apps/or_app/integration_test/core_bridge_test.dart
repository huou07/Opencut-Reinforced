import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';
import 'dart:ui' show Size;

import 'package:flutter/foundation.dart' show ValueKey;
import 'package:flutter/material.dart' show SnackBar, Text;
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:or_app/main.dart';
import 'package:or_app/project/project_file_picker.dart';
import 'package:or_app/project/project_gateway.dart';
import 'package:or_app/project/rust_project_gateway.dart';
import 'package:or_app/rust_core_gateway.dart';
import 'package:or_app_bridge/or_app_bridge.dart' show RustLib;
import 'package:or_viewer_texture/or_viewer_texture.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  setUpAll(RustLib.init);

  testWidgets(
    'Rust project bridge stores editable titles and manual captions',
    (tester) async {
      final directory = Directory.systemTemp.createTempSync('or-text-bridge-');
      addTearDown(() => directory.deleteSync(recursive: true));
      const gateway = RustProjectGateway();
      final session = await gateway.createProject(
        '${directory.path}/text-project.orproj',
        'Text bridge',
      );
      var current = await gateway.summary(session);
      final textTrackResult = await gateway.addTimelineTrack(
        session,
        current,
        ProjectTimelineTrackKind.text,
      );
      expect(textTrackResult.succeeded, isTrue);
      current = textTrackResult.view!;
      final textTrack = (await gateway.listTimelineTracks(session))
          .items
          .single;
      const titleFormatting = ProjectTextFormatting(
        font: ProjectFontIdentity.bundledInter,
        sizeMilliPoints: 36000,
        weight: ProjectTextWeight.semibold,
        alignment: ProjectTextAlignment.center,
        color: ProjectTextColor.white,
      );
      final insertTitle = await gateway.insertTimelineTextClip(
        session,
        current,
        trackId: textTrack.trackId,
        timelineStart: ProjectRationalTime(BigInt.one, 3),
        timelineDuration: ProjectRationalTime(BigInt.from(5), 2),
        content: const ProjectTimelineTextContent(
          kind: ProjectTimelineClipContentKind.text,
          text: 'Opening title',
          formatting: titleFormatting,
        ),
      );
      expect(insertTitle.succeeded, isTrue);
      current = insertTitle.view!;
      var page = await gateway.listTimelineClips(
        session,
        trackId: textTrack.trackId,
        offset: 0,
        limit: 10,
      );
      final title = page.items.single;
      expect(title.contentKind, ProjectTimelineClipContentKind.text);
      expect(title.text, 'Opening title');
      expect(title.timelineStart.canonical, '1/3');
      expect(title.timelineDuration.canonical, '5/2');
      expect(title.formatting?.weight, ProjectTextWeight.semibold);

      final updateTitle = await gateway.updateTimelineTextClip(
        session,
        current,
        trackId: textTrack.trackId,
        clipId: title.clipId,
        timelineDuration: ProjectRationalTime(BigInt.from(3), 1),
        content: const ProjectTimelineTextContent(
          kind: ProjectTimelineClipContentKind.text,
          text: 'Updated title',
          formatting: titleFormatting,
        ),
      );
      expect(updateTitle.succeeded, isTrue);
      current = updateTitle.view!;
      page = await gateway.listTimelineClips(
        session,
        trackId: textTrack.trackId,
        offset: 0,
        limit: 10,
      );
      expect(page.items.single.text, 'Updated title');
      expect(page.items.single.clipId, title.clipId);
      expect(page.items.single.timelineStart.canonical, '1/3');
      expect(page.items.single.timelineDuration.canonical, '3/1');

      final captionTrackResult = await gateway.addTimelineTrack(
        session,
        current,
        ProjectTimelineTrackKind.caption,
      );
      expect(captionTrackResult.succeeded, isTrue);
      current = captionTrackResult.view!;
      final captionTrack = (await gateway.listTimelineTracks(session))
          .items
          .last;
      final insertCaption = await gateway.insertTimelineTextClip(
        session,
        current,
        trackId: captionTrack.trackId,
        timelineStart: ProjectRationalTime(BigInt.from(4), 1),
        timelineDuration: ProjectRationalTime(BigInt.one, 1),
        content: const ProjectTimelineTextContent(
          kind: ProjectTimelineClipContentKind.caption,
          text: 'A manual caption',
          formatting: ProjectTextFormatting.defaults,
        ),
      );
      expect(insertCaption.succeeded, isTrue);
      final captions = await gateway.listTimelineClips(
        session,
        trackId: captionTrack.trackId,
        offset: 0,
        limit: 10,
      );
      expect(
        captions.items.single.contentKind,
        ProjectTimelineClipContentKind.caption,
      );
      expect(captions.items.single.text, 'A manual caption');
      expect(captions.items.single.timelineStart.canonical, '4/1');
      expect(captions.items.single.timelineDuration.canonical, '1/1');
      await gateway.close(session, discardUnsaved: true);
    },
  );

  testWidgets('Rust project bridge saves and reopens audio clip settings', (
    tester,
  ) async {
    final directory = Directory.systemTemp.createTempSync('or-audio-bridge-');
    const gateway = RustProjectGateway();
    ProjectSessionHandle? activeSession;
    addTearDown(() async {
      if (activeSession != null) {
        await gateway.close(activeSession, discardUnsaved: true);
      }
      directory.deleteSync(recursive: true);
    });

    final audioPath = '${directory.path}/silent-audio.wav';
    await File(audioPath)
        .writeAsBytes(_silentWave(sampleRate: 48000, frames: 12000));
    var session = await gateway.createProject(
      '${directory.path}/audio-project.orproj',
      'Audio bridge',
    );
    activeSession = session;
    var current = await gateway.summary(session);
    final imported = await gateway.importMedia(session, current, audioPath);
    expect(imported.succeeded, isTrue);
    current = imported.view!;
    final media = (await gateway.listMediaPage(
      session,
      offset: 0,
      limit: 10,
    )).items.single;

    final addedTrack = await gateway.addTimelineTrack(
      session,
      current,
      ProjectTimelineTrackKind.audio,
    );
    expect(addedTrack.succeeded, isTrue);
    current = addedTrack.view!;
    final track = (await gateway.listTimelineTracks(session)).items.single;
    final inserted = await gateway.insertTimelineClip(
      session,
      current,
      trackId: track.trackId,
      mediaId: media.mediaId,
      timelineStart: ProjectRationalTime(BigInt.zero, 1),
      sourceStart: ProjectRationalTime(BigInt.zero, 1),
      duration: ProjectRationalTime(BigInt.one, 4),
    );
    expect(inserted.succeeded, isTrue);
    current = inserted.view!;
    final clip = (await gateway.listTimelineClips(
      session,
      trackId: track.trackId,
      offset: 0,
      limit: 10,
    )).items.single;

    final initial = await gateway.getTimelineClipAudioSettings(
      session,
      current,
      trackId: track.trackId,
      clipId: clip.clipId,
    );
    expect(initial.gainMilliDecibels, 0);
    final settings = ProjectTimelineAudioSettings(
      gainMilliDecibels: -6250,
      panBasisPoints: -3750,
      fadeIn: ProjectRationalTime(BigInt.one, 8),
      fadeOut: ProjectRationalTime(BigInt.one, 16),
    );
    final updated = await gateway.updateTimelineClipAudioSettings(
      session,
      current,
      trackId: track.trackId,
      clipId: clip.clipId,
      settings: settings,
    );
    expect(updated.succeeded, isTrue);
    current = updated.view!;
    expect((await gateway.save(session)).succeeded, isTrue);
    await gateway.close(session, discardUnsaved: false);
    activeSession = null;

    session = await gateway.openProject(
      '${directory.path}/audio-project.orproj',
    );
    activeSession = session;
    current = await gateway.summary(session);
    final reopenedTrack = (await gateway.listTimelineTracks(session))
        .items
        .single;
    final reopenedClip = (await gateway.listTimelineClips(
      session,
      trackId: reopenedTrack.trackId,
      offset: 0,
      limit: 10,
    )).items.single;
    final reopened = await gateway.getTimelineClipAudioSettings(
      session,
      current,
      trackId: reopenedTrack.trackId,
      clipId: reopenedClip.clipId,
    );
    expect(reopened.gainMilliDecibels, -6250);
    expect(reopened.panBasisPoints, -3750);
    expect(reopened.fadeIn.canonical, '1/8');
    expect(reopened.fadeOut.canonical, '1/16');
    expect(tester.takeException(), isNull);
  });

  testWidgets('native Flutter project lifecycle uses one Rust host', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1280, 900);
    addTearDown(tester.view.resetPhysicalSize);
    final directory = Directory.systemTemp.createTempSync('or-flutter-cli-');
    addTearDown(() => directory.deleteSync(recursive: true));
    final projectPath = '${directory.path}/native-project.orproj';
    final picker = _NativeProjectPicker(projectPath);
    final gateway = _ObservedRustProjectGateway();
    final creation = gateway.projectCreationCompletion;
    const coreGateway = RustCoreGateway();

    await tester.pumpWidget(
      OrApp(
        gateway: coreGateway,
        projectGateway: gateway,
        projectFilePicker: picker,
      ),
    );
    await tester.tap(find.byKey(const ValueKey('home-new-project')));
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byKey(const ValueKey('new-project-name')),
      'Native Project',
    );
    await tester.tap(find.byKey(const ValueKey('confirm-new-project')));
    await tester.pumpAndSettle();
    final creationCompleted = await tester.runAsync(
      () => creation
          .then((_) => true)
          .timeout(const Duration(seconds: 30), onTimeout: () => false),
    );
    expect(
      creationCompleted,
      isTrue,
      reason:
          'Native project creation did not complete within 30 seconds at '
          '$projectPath; native bridge error: '
          '${gateway.lastError ?? "none recorded"}',
    );

    final session = gateway.activeSession;
    final errorMessage = tester
        .widgetList<Text>(
          find.descendant(
            of: find.byType(SnackBar),
            matching: find.byType(Text),
          ),
        )
        .map((text) => text.data)
        .whereType<String>()
        .join(' | ');
    expect(
      session,
      isNotNull,
      reason: '$errorMessage; native bridge error: ${gateway.lastError}',
    );
    final initial = await gateway.summary(session!);
    expect(initial.name, 'Native Project');
    expect(initial.revision, BigInt.zero);
    expect(initial.dirty, isFalse);
    final descriptorPath = initial.descriptorPath;

    await tester.tap(find.byKey(const ValueKey('or-brand-home')));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('nav-settings')));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('settings-section-advanced')));
    await tester.pumpAndSettle();
    expect(find.text('Local CLI session'), findsOneWidget);
    expect(find.text('Available'), findsOneWidget);
    expect(find.text(descriptorPath), findsOneWidget);
    expect(find.textContaining('token'), findsNothing);
    expect(find.textContaining('--attach "<descriptor-path>"'), findsOneWidget);
    await tester.tap(
      find.byKey(const ValueKey('settings-open-editor-preview')),
    );
    await tester.pumpAndSettle();

    final events = <ProjectHostEvent>[];
    final eventSubscription = gateway.watch(session).listen(events.add);
    addTearDown(eventSubscription.cancel);

    await _renameProject(tester, 'From Flutter');
    expect((await gateway.summary(session)).revision, BigInt.one);
    expect(find.text('Unsaved changes'), findsOneWidget);

    await tester.tap(find.byKey(const ValueKey('workspace-undo')));
    await tester.pumpAndSettle();
    expect((await gateway.summary(session)).name, 'Native Project');
    expect((await gateway.summary(session)).revision, BigInt.two);

    await tester.tap(find.byKey(const ValueKey('workspace-redo')));
    await tester.pumpAndSettle();
    expect((await gateway.summary(session)).name, 'From Flutter');
    expect((await gateway.summary(session)).revision, BigInt.from(3));

    await tester.tap(find.byKey(const ValueKey('workspace-save')));
    await tester.pumpAndSettle();
    expect((await gateway.summary(session)).dirty, isFalse);
    expect((await gateway.summary(session)).revision, BigInt.from(3));

    await tester.tap(find.byKey(const ValueKey('workspace-close')));
    await tester.pumpAndSettle();
    expect(File(descriptorPath).existsSync(), isFalse);

    picker.openPath = projectPath;
    await tester.tap(find.byKey(const ValueKey('home-open-project')));
    await tester.pumpAndSettle();
    final reopened = await gateway.summary(gateway.activeSession!);
    expect(reopened.projectId, initial.projectId);
    expect(reopened.projectInstanceId, isNot(initial.projectInstanceId));
    expect(reopened.name, 'From Flutter');
    expect(reopened.revision, BigInt.from(3));
    expect(reopened.dirty, isFalse);
    expect(find.text('No timeline tracks'), findsOneWidget);

    await tester.tap(find.byKey(const ValueKey('workspace-close')));
    await tester.pumpAndSettle();
    await tester.runAsync(
      () => Future<void>.delayed(const Duration(milliseconds: 150)),
    );
    expect(events.map((event) => event.sequence).toList(), [
      BigInt.one,
      BigInt.two,
      BigInt.from(3),
      BigInt.from(4),
      BigInt.from(5),
    ]);
    expect(events.map((event) => event.kind).toList(), [
      'project_changed',
      'project_changed',
      'project_changed',
      'project_saved',
      'session_closing',
    ]);
  });

  testWidgets('native preview transport keeps exact time and explicit rate', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1280, 900);
    addTearDown(tester.view.resetPhysicalSize);
    final directory = Directory.systemTemp.createTempSync('or-preview-');
    addTearDown(() => directory.deleteSync(recursive: true));
    final gateway = _ObservedRustProjectGateway();
    const coreGateway = RustCoreGateway();
    await tester.pumpWidget(
      OrApp(
        gateway: coreGateway,
        projectGateway: gateway,
        projectFilePicker: _NativeProjectPicker(
          '${directory.path}/preview.orproj',
        ),
      ),
    );
    await tester.tap(find.byKey(const ValueKey('home-new-project')));
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byKey(const ValueKey('new-project-name')),
      'Preview Project',
    );
    await tester.tap(find.byKey(const ValueKey('confirm-new-project')));
    await tester.pumpAndSettle();

    final session = gateway.activeSession!;
    expect(await OrViewerTexture.textureId(), isNotNull);
    final project = await gateway.summary(session);
    final exactTime = ProjectRationalTime(BigInt.from(7), 15);
    final preview = await gateway.previewSeek(session, exactTime);
    expect(preview.position.canonical, '7/15');
    expect(preview.presentedTime?.canonical, '7/15');
    expect(preview.frameSequence, greaterThan(BigInt.zero));
    expect(preview.width, greaterThan(0));
    expect(preview.height, greaterThan(0));
    expect((await gateway.summary(session)).revision, project.revision);
    await expectLater(
      gateway.previewPlay(session),
      throwsA(
        isA<ProjectGatewayException>().having(
          (error) => error.code,
          'code',
          'SEQUENCE_FRAME_RATE_REQUIRED',
        ),
      ),
    );

    final configured = await gateway.setTimelineSequenceFrameRate(
      session,
      project,
      const ProjectRationalRate(24, 1),
    );
    expect(configured.succeeded, isTrue);
    expect(configured.view?.revision, project.revision + BigInt.one);
    final configuredPreview = await gateway.previewState(session);
    expect(configuredPreview.sequenceFrameRate?.canonical, '24/1');
    expect(configuredPreview.contentEnd, isNull);
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    'native timeline bridge edits, history, and save reopen use one Rust host',
    (tester) async {
      final directory = Directory.systemTemp.createTempSync(
        'or-timeline-bridge-',
      );
      final projectPath = '${directory.path}/timeline-project.orproj';
      final missingSourcePath = '${directory.path}/offline-video.mov';
      await File(projectPath).writeAsString(
        jsonEncode(
          _offlineVideoTimelineProject(Uri.file(missingSourcePath).toString()),
        ),
      );
      final gateway = _ObservedRustProjectGateway();
      addTearDown(() async {
        final session = gateway.activeSession;
        if (session != null) {
          await gateway.close(session, discardUnsaved: true);
        }
        directory.deleteSync(recursive: true);
      });

      final session = await gateway.openProject(projectPath);
      var current = await gateway.summary(session);
      final originalProjectId = current.projectId;
      final originalInstanceId = current.projectInstanceId;
      final emptyTracks = await gateway.listTimelineTracks(session);
      expect(emptyTracks.items, isEmpty);
      expect(emptyTracks.projectId, originalProjectId);
      expect(emptyTracks.projectInstanceId, originalInstanceId);
      expect(emptyTracks.projectRevision, BigInt.zero);

      final mediaPage = await gateway.listMediaPage(
        session,
        offset: 0,
        limit: 10,
      );
      final media = mediaPage.items.single;
      expect(media.containerDuration?.canonical, '4/1');
      expect(media.firstVideoDuration?.canonical, '4/1');
      expect(media.firstAudioDuration, isNull);

      final added = await gateway.addTimelineTrack(
        session,
        current,
        ProjectTimelineTrackKind.video,
      );
      expect(added.succeeded, isTrue, reason: gateway.lastError?.toString());
      current = added.view!;
      expect(current.revision, BigInt.one);
      expect(current.projectId, originalProjectId);
      expect(current.projectInstanceId, originalInstanceId);
      final tracks = await gateway.listTimelineTracks(session);
      final track = tracks.items.single;
      expect(
        track.trackId,
        matches(
          RegExp(
            r'^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$',
          ),
        ),
      );
      expect(track.kind, ProjectTimelineTrackKind.video);
      expect(track.clipCount, 0);
      expect(tracks.projectRevision, current.revision);

      final inserted = await gateway.insertTimelineClip(
        session,
        current,
        trackId: track.trackId,
        mediaId: media.mediaId,
        timelineStart: ProjectRationalTime(BigInt.zero, 1),
        sourceStart: ProjectRationalTime(BigInt.one, 2),
        duration: ProjectRationalTime(BigInt.from(2), 1),
      );
      expect(inserted.succeeded, isTrue);
      current = inserted.view!;
      expect(current.revision, BigInt.from(2));
      var page = await gateway.listTimelineClips(
        session,
        trackId: track.trackId,
        offset: 0,
        limit: 100,
      );
      final clip = page.items.single;
      expect(
        clip.clipId,
        matches(
          RegExp(
            r'^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$',
          ),
        ),
      );
      expect(clip.mediaId, media.mediaId);
      expect(clip.timelineStart.canonical, '0/1');
      expect(clip.sourceStart!.canonical, '1/2');
      expect(clip.timelineDuration.canonical, '2/1');
      expect(page.projectRevision, current.revision);

      final undoneInsert = await gateway.undo(session, current);
      expect(undoneInsert.succeeded, isTrue);
      current = undoneInsert.view!;
      expect(current.revision, BigInt.from(3));
      expect(
        (await gateway.listTimelineClips(
          session,
          trackId: track.trackId,
          offset: 0,
          limit: 100,
        )).items,
        isEmpty,
      );

      final redoneInsert = await gateway.redo(session, current);
      expect(redoneInsert.succeeded, isTrue);
      current = redoneInsert.view!;
      expect(current.revision, BigInt.from(4));
      page = await gateway.listTimelineClips(
        session,
        trackId: track.trackId,
        offset: 0,
        limit: 100,
      );
      expect(page.items.single.clipId, clip.clipId);

      final snap = await gateway.resolveTimelineSnap(
        session,
        current,
        operation: ProjectTimelineSnapOperation.move,
        clipId: clip.clipId,
        targetTrackId: track.trackId,
        targetTime: ProjectRationalTime(BigInt.one, 16),
      );
      expect(snap.projectId, originalProjectId);
      expect(snap.projectInstanceId, originalInstanceId);
      expect(snap.projectRevision, current.revision);
      expect(snap.rawTargetTime.canonical, '1/16');
      expect(snap.resolvedTargetTime.canonical, '0/1');
      expect(snap.snapped, isTrue);
      expect(snap.targetKind, ProjectTimelineSnapTargetKind.timelineZero);

      final moved = await gateway.moveTimelineClip(
        session,
        current,
        clipId: clip.clipId,
        trackId: track.trackId,
        timelineStart: ProjectRationalTime(BigInt.from(3), 1),
      );
      expect(moved.succeeded, isTrue);
      current = moved.view!;
      expect(current.revision, BigInt.from(5));
      page = await gateway.listTimelineClips(
        session,
        trackId: track.trackId,
        offset: 0,
        limit: 100,
      );
      expect(page.items.single.clipId, clip.clipId);
      expect(page.items.single.mediaId, clip.mediaId);
      expect(page.items.single.sourceStart!.canonical, '1/2');
      expect(page.items.single.timelineDuration.canonical, '2/1');
      expect(page.items.single.timelineStart.canonical, '3/1');

      final noOpMove = await gateway.moveTimelineClip(
        session,
        current,
        clipId: clip.clipId,
        trackId: track.trackId,
        timelineStart: ProjectRationalTime(BigInt.from(3), 1),
      );
      expect(noOpMove.succeeded, isTrue);
      current = noOpMove.view!;
      expect(current.revision, BigInt.from(5));

      final trimmedStart = await gateway.trimTimelineClip(
        session,
        current,
        clipId: clip.clipId,
        edge: ProjectTimelineTrimEdge.start,
        timelineTime: ProjectRationalTime(BigInt.from(7), 2),
      );
      expect(trimmedStart.succeeded, isTrue);
      current = trimmedStart.view!;
      expect(current.revision, BigInt.from(6));
      page = await gateway.listTimelineClips(
        session,
        trackId: track.trackId,
        offset: 0,
        limit: 100,
      );
      expect(page.items.single.timelineStart.canonical, '7/2');
      expect(page.items.single.sourceStart!.canonical, '1/1');
      expect(page.items.single.timelineDuration.canonical, '3/2');

      final trimmedEnd = await gateway.trimTimelineClip(
        session,
        current,
        clipId: clip.clipId,
        edge: ProjectTimelineTrimEdge.end,
        timelineTime: ProjectRationalTime(BigInt.from(9), 2),
      );
      expect(trimmedEnd.succeeded, isTrue);
      current = trimmedEnd.view!;
      expect(current.revision, BigInt.from(7));
      page = await gateway.listTimelineClips(
        session,
        trackId: track.trackId,
        offset: 0,
        limit: 100,
      );
      expect(
        page.items.single.timelineStart
            .add(page.items.single.timelineDuration)
            .canonical,
        '9/2',
      );
      expect(page.items.single.timelineDuration.canonical, '1/1');

      final noOpTrim = await gateway.trimTimelineClip(
        session,
        current,
        clipId: clip.clipId,
        edge: ProjectTimelineTrimEdge.end,
        timelineTime: ProjectRationalTime(BigInt.from(9), 2),
      );
      expect(noOpTrim.succeeded, isTrue);
      current = noOpTrim.view!;
      expect(current.revision, BigInt.from(7));

      final split = await gateway.splitTimelineClip(
        session,
        current,
        clipId: clip.clipId,
        timelineTime: ProjectRationalTime(BigInt.from(4), 1),
      );
      expect(split.succeeded, isTrue);
      current = split.view!;
      expect(current.revision, BigInt.from(8));
      page = await gateway.listTimelineClips(
        session,
        trackId: track.trackId,
        offset: 0,
        limit: 100,
      );
      expect(page.items, hasLength(2));
      final rightClip = page.items.singleWhere(
        (candidate) => candidate.clipId != clip.clipId,
      );
      expect(rightClip.timelineStart.canonical, '4/1');
      expect(rightClip.sourceStart!.canonical, '3/2');
      expect(rightClip.timelineDuration.canonical, '1/2');

      final insertedLater = await gateway.insertTimelineClip(
        session,
        current,
        trackId: track.trackId,
        mediaId: media.mediaId,
        timelineStart: ProjectRationalTime(BigInt.from(6), 1),
        sourceStart: ProjectRationalTime(BigInt.zero, 1),
        duration: ProjectRationalTime(BigInt.one, 1),
      );
      expect(insertedLater.succeeded, isTrue);
      current = insertedLater.view!;
      expect(current.revision, BigInt.from(9));
      page = await gateway.listTimelineClips(
        session,
        trackId: track.trackId,
        offset: 0,
        limit: 100,
      );
      expect(page.items, hasLength(3));
      final laterClip = page.items.singleWhere(
        (candidate) =>
            candidate.clipId != clip.clipId &&
            candidate.clipId != rightClip.clipId,
      );
      expect(laterClip.timelineStart.canonical, '6/1');

      final rippleDeleted = await gateway.rippleDeleteTimelineClip(
        session,
        current,
        rightClip.clipId,
      );
      expect(rippleDeleted.succeeded, isTrue);
      current = rippleDeleted.view!;
      expect(current.revision, BigInt.from(10));
      page = await gateway.listTimelineClips(
        session,
        trackId: track.trackId,
        offset: 0,
        limit: 100,
      );
      expect(page.items, hasLength(2));
      expect(page.items[0].clipId, clip.clipId);
      expect(page.items[0].timelineStart.canonical, '7/2');
      expect(page.items[1].clipId, laterClip.clipId);
      expect(page.items[1].timelineStart.canonical, '11/2');

      final undoneRipple = await gateway.undo(session, current);
      expect(undoneRipple.succeeded, isTrue);
      current = undoneRipple.view!;
      expect(current.revision, BigInt.from(11));
      page = await gateway.listTimelineClips(
        session,
        trackId: track.trackId,
        offset: 0,
        limit: 100,
      );
      expect(page.items, hasLength(3));
      expect(page.items[1].clipId, rightClip.clipId);
      expect(page.items[2].timelineStart.canonical, '6/1');

      final redoneRipple = await gateway.redo(session, current);
      expect(redoneRipple.succeeded, isTrue);
      current = redoneRipple.view!;
      expect(current.revision, BigInt.from(12));
      expect(
        (await gateway.listTimelineClips(
          session,
          trackId: track.trackId,
          offset: 0,
          limit: 100,
        )).items,
        hasLength(2),
      );
      final initialVisual = await gateway.getTimelineClipVisualSettings(
        session,
        current,
        trackId: track.trackId,
        clipId: clip.clipId,
      );
      expect(initialVisual.xMilliCanvas, 0);
      expect(initialVisual.scaleXMilli, 1000);
      expect(initialVisual.opacityBasisPoints, 10000);
      final visualSettings = ProjectTimelineVisualSettings(
        xMilliCanvas: 250,
        yMilliCanvas: -125,
        scaleXMilli: 1500,
        scaleYMilli: 750,
        rotationMilliDegrees: 15000,
        anchorXBasisPoints: 2500,
        anchorYBasisPoints: 7500,
        cropLeftBasisPoints: 1000,
        cropTopBasisPoints: 2000,
        cropRightBasisPoints: 500,
        cropBottomBasisPoints: 1500,
        opacityBasisPoints: 6250,
        brightnessAmountMilli: 500,
        contrastAmountMilli: 1250,
        saturationAmountMilli: 750,
        gaussianBlurRadiusMilli: 1000,
        transitionIn: ProjectTimelineTransition.crossDissolve,
        transitionInDuration: ProjectRationalTime(BigInt.one, 2),
        transitionOut: ProjectTimelineTransition.wipe,
        transitionOutDuration: ProjectRationalTime(BigInt.one, 4),
        modifiedEffects: {
          ProjectTimelineEffectKind.brightness,
          ProjectTimelineEffectKind.contrast,
          ProjectTimelineEffectKind.saturation,
          ProjectTimelineEffectKind.gaussianBlur,
        },
        updateTransitionIn: true,
        updateTransitionOut: true,
      );
      final visualUpdate = await gateway.updateTimelineClipVisualSettings(
        session,
        current,
        trackId: track.trackId,
        clipId: clip.clipId,
        settings: visualSettings,
      );
      expect(visualUpdate.succeeded, isTrue);
      current = visualUpdate.view!;
      expect(current.revision, BigInt.from(13));
      final updatedVisual = await gateway.getTimelineClipVisualSettings(
        session,
        current,
        trackId: track.trackId,
        clipId: clip.clipId,
      );
      expect(updatedVisual.xMilliCanvas, visualSettings.xMilliCanvas);
      expect(updatedVisual.yMilliCanvas, visualSettings.yMilliCanvas);
      expect(updatedVisual.scaleXMilli, visualSettings.scaleXMilli);
      expect(updatedVisual.scaleYMilli, visualSettings.scaleYMilli);
      expect(
        updatedVisual.rotationMilliDegrees,
        visualSettings.rotationMilliDegrees,
      );
      expect(
        updatedVisual.anchorXBasisPoints,
        visualSettings.anchorXBasisPoints,
      );
      expect(
        updatedVisual.anchorYBasisPoints,
        visualSettings.anchorYBasisPoints,
      );
      expect(
        updatedVisual.cropLeftBasisPoints,
        visualSettings.cropLeftBasisPoints,
      );
      expect(
        updatedVisual.cropTopBasisPoints,
        visualSettings.cropTopBasisPoints,
      );
      expect(
        updatedVisual.cropRightBasisPoints,
        visualSettings.cropRightBasisPoints,
      );
      expect(
        updatedVisual.cropBottomBasisPoints,
        visualSettings.cropBottomBasisPoints,
      );
      expect(
        updatedVisual.opacityBasisPoints,
        visualSettings.opacityBasisPoints,
      );
      expect(updatedVisual.brightnessAmountMilli, 500);
      expect(updatedVisual.contrastAmountMilli, 1250);
      expect(updatedVisual.saturationAmountMilli, 750);
      expect(updatedVisual.gaussianBlurRadiusMilli, 1000);
      expect(
        updatedVisual.transitionIn,
        ProjectTimelineTransition.crossDissolve,
      );
      expect(updatedVisual.transitionInDuration!.canonical, '1/2');
      expect(updatedVisual.transitionOut, ProjectTimelineTransition.wipe);
      expect(updatedVisual.transitionOutDuration!.canonical, '1/4');
      final trackState = await gateway.setTimelineTrackState(
        session,
        current,
        trackId: track.trackId,
        state: const ProjectTimelineTrackState(
          locked: true,
          visible: false,
          muted: false,
          solo: true,
        ),
      );
      expect(trackState.succeeded, isTrue);
      current = trackState.view!;
      expect(current.revision, BigInt.from(14));
      final updatedTrack = (await gateway.listTimelineTracks(session))
          .items
          .single;
      expect(updatedTrack.state.locked, isTrue);
      expect(updatedTrack.state.visible, isFalse);
      expect(updatedTrack.state.solo, isTrue);
      expect(
        (await gateway.listMediaPage(session, offset: 0, limit: 10)).items,
        hasLength(1),
      );

      final saved = await gateway.save(session);
      expect(saved.succeeded, isTrue);
      expect(saved.view?.revision, BigInt.from(14));
      expect(saved.view?.dirty, isFalse);
      await gateway.close(session, discardUnsaved: false);

      final reopenedSession = await gateway.openProject(projectPath);
      final reopened = await gateway.summary(reopenedSession);
      expect(reopened.projectId, originalProjectId);
      expect(reopened.projectInstanceId, isNot(originalInstanceId));
      expect(reopened.revision, BigInt.from(14));
      expect(reopened.dirty, isFalse);
      final reopenedTracks = await gateway.listTimelineTracks(reopenedSession);
      expect(reopenedTracks.items.single.trackId, track.trackId);
      expect(reopenedTracks.items.single.kind, ProjectTimelineTrackKind.video);
      expect(reopenedTracks.items.single.state.locked, isTrue);
      expect(reopenedTracks.items.single.state.visible, isFalse);
      expect(reopenedTracks.items.single.state.solo, isTrue);
      final reopenedVisual = await gateway.getTimelineClipVisualSettings(
        reopenedSession,
        reopened,
        trackId: track.trackId,
        clipId: clip.clipId,
      );
      expect(reopenedVisual.xMilliCanvas, visualSettings.xMilliCanvas);
      expect(reopenedVisual.yMilliCanvas, visualSettings.yMilliCanvas);
      expect(reopenedVisual.scaleXMilli, visualSettings.scaleXMilli);
      expect(reopenedVisual.scaleYMilli, visualSettings.scaleYMilli);
      expect(
        reopenedVisual.rotationMilliDegrees,
        visualSettings.rotationMilliDegrees,
      );
      expect(
        reopenedVisual.anchorXBasisPoints,
        visualSettings.anchorXBasisPoints,
      );
      expect(
        reopenedVisual.anchorYBasisPoints,
        visualSettings.anchorYBasisPoints,
      );
      expect(
        reopenedVisual.cropLeftBasisPoints,
        visualSettings.cropLeftBasisPoints,
      );
      expect(
        reopenedVisual.cropTopBasisPoints,
        visualSettings.cropTopBasisPoints,
      );
      expect(
        reopenedVisual.cropRightBasisPoints,
        visualSettings.cropRightBasisPoints,
      );
      expect(
        reopenedVisual.cropBottomBasisPoints,
        visualSettings.cropBottomBasisPoints,
      );
      expect(
        reopenedVisual.opacityBasisPoints,
        visualSettings.opacityBasisPoints,
      );
      expect(reopenedVisual.brightnessAmountMilli, 500);
      expect(reopenedVisual.contrastAmountMilli, 1250);
      expect(reopenedVisual.saturationAmountMilli, 750);
      expect(reopenedVisual.gaussianBlurRadiusMilli, 1000);
      expect(
        reopenedVisual.transitionIn,
        ProjectTimelineTransition.crossDissolve,
      );
      expect(reopenedVisual.transitionInDuration!.canonical, '1/2');
      expect(reopenedVisual.transitionOut, ProjectTimelineTransition.wipe);
      expect(reopenedVisual.transitionOutDuration!.canonical, '1/4');
      final reopenedClips = (await gateway.listTimelineClips(
        reopenedSession,
        trackId: track.trackId,
        offset: 0,
        limit: 100,
      )).items;
      expect(reopenedClips, hasLength(2));
      expect(reopenedClips[0].clipId, clip.clipId);
      expect(reopenedClips[0].timelineStart.canonical, '7/2');
      expect(reopenedClips[1].clipId, laterClip.clipId);
      expect(reopenedClips[1].timelineStart.canonical, '11/2');
      expect(
        (await gateway.listMediaPage(
          reopenedSession,
          offset: 0,
          limit: 10,
        )).items.single.mediaId,
        media.mediaId,
      );
      final historyAfterReopen = await gateway.undo(reopenedSession, reopened);
      expect(historyAfterReopen.succeeded, isFalse);
      expect(historyAfterReopen.errorCode, 'NOTHING_TO_UNDO');
    },
  );

  testWidgets(
    'native media bridge persists offline media through undo and save',
    (tester) async {
      final directory = Directory.systemTemp.createTempSync('or-media-bridge-');
      final projectPath = '${directory.path}/media-project.orproj';
      final missingSourcePath = '${directory.path}/offline-source.mkv';
      await File(projectPath).writeAsString(
        jsonEncode(
          _offlineMediaProject(Uri.file(missingSourcePath).toString()),
        ),
      );
      final gateway = _ObservedRustProjectGateway();
      addTearDown(() async {
        final session = gateway.activeSession;
        if (session != null) {
          await gateway.close(session, discardUnsaved: true);
        }
        directory.deleteSync(recursive: true);
      });

      final session = await gateway.openProject(projectPath);
      final original = await gateway.listMediaPage(
        session,
        offset: 0,
        limit: 50,
      );
      expect(original.items, hasLength(1));
      expect(
        original.items.single.mediaId,
        '22222222-2222-4222-8222-222222222222',
      );
      expect(original.items.single.formatNames, ['matroska']);
      expect(
        original.items.single.sourceUri,
        Uri.file(missingSourcePath).toString(),
      );
      expect(File(missingSourcePath).existsSync(), isFalse);

      final thumbnailRequest = await gateway.requestMediaThumbnail(
        session,
        original.items.single.mediaId,
      );
      expect(thumbnailRequest.kind, ProjectMediaArtifactKind.thumbnail);
      expect(
        thumbnailRequest.state,
        ProjectMediaArtifactRequestState.notApplicable,
      );
      expect(thumbnailRequest.cacheKey, isNull);
      final waveformRequest = await gateway.requestMediaWaveform(
        session,
        original.items.single.mediaId,
      );
      expect(waveformRequest.kind, ProjectMediaArtifactKind.waveform);
      expect(
        waveformRequest.state,
        ProjectMediaArtifactRequestState.notApplicable,
      );
      expect((await gateway.summary(session)).revision, BigInt.zero);
      await expectLater(
        gateway.readMediaArtifact(
          session,
          kind: ProjectMediaArtifactKind.thumbnail,
          cacheKey: 'A' * 64,
        ),
        throwsA(
          isA<ProjectGatewayException>().having(
            (error) => error.code,
            'code',
            'INVALID_CACHE_KEY',
          ),
        ),
      );

      final removed = await gateway.removeMedia(
        session,
        await gateway.summary(session),
        original.items.single.mediaId,
      );
      expect(removed.succeeded, isTrue);
      expect(
        (await gateway.listMediaPage(session, offset: 0, limit: 50)).items,
        isEmpty,
      );

      final undone = await gateway.undo(session, removed.view!);
      expect(undone.succeeded, isTrue);
      final restored = await gateway.listMediaPage(
        session,
        offset: 0,
        limit: 50,
      );
      expect(restored.items.single.mediaId, original.items.single.mediaId);
      expect(restored.items.single.sourceUri, original.items.single.sourceUri);
      expect(
        restored.items.single.formatNames,
        original.items.single.formatNames,
      );

      final saved = await gateway.save(session);
      expect(saved.succeeded, isTrue);
      expect(saved.view?.dirty, isFalse);
      await gateway.close(session, discardUnsaved: false);

      final reopenedSession = await gateway.openProject(projectPath);
      final reopened = await gateway.listMediaPage(
        reopenedSession,
        offset: 0,
        limit: 50,
      );
      expect(reopened.items, hasLength(1));
      expect(reopened.items.single.mediaId, original.items.single.mediaId);
      expect(reopened.items.single.sourceUri, original.items.single.sourceUri);
      expect(reopened.items.single.formatNames, ['matroska']);
      expect((await gateway.summary(reopenedSession)).dirty, isFalse);
      expect(File(missingSourcePath).existsSync(), isFalse);
    },
  );
}

Map<String, Object?> _offlineMediaProject(String sourceUri) => {
  'format': 'opencut-reinforced-project',
  'schema_version': 2,
  'project': {
    'id': '01234567-89ab-4def-8123-456789abcdef',
    'revision': 0,
    'name': 'Offline media fixture',
    'media': [
      {
        'id': '22222222-2222-4222-8222-222222222222',
        'source': {'kind': 'local_file', 'uri': sourceUri},
        'metadata': {
          'format_names': ['matroska'],
          'duration': null,
          'file_size_bytes': 0,
          'streams': [],
        },
      },
    ],
  },
};

Map<String, Object?> _offlineVideoTimelineProject(String sourceUri) => {
  'format': 'opencut-reinforced-project',
  'schema_version': 3,
  'project': {
    'id': '01234567-89ab-4def-8123-456789abcdef',
    'revision': 0,
    'name': 'Offline video timeline fixture',
    'media': [
      {
        'id': '22222222-2222-4222-8222-222222222222',
        'source': {'kind': 'local_file', 'uri': sourceUri},
        'metadata': {
          'format_names': ['mov'],
          'duration': {'numerator': 4, 'denominator': 1},
          'file_size_bytes': 0,
          'streams': [
            {
              'kind': 'video',
              'metadata': {
                'index': 0,
                'codec_name': 'h264',
                'width': 1920,
                'height': 1080,
                'pixel_format': 'yuv420p',
                'average_frame_rate': {'numerator': 24, 'denominator': 1},
                'duration': {'numerator': 4, 'denominator': 1},
              },
            },
          ],
        },
      },
    ],
    'timeline': {'tracks': []},
  },
};

Future<void> _renameProject(WidgetTester tester, String name) async {
  await tester.tap(find.byKey(const ValueKey('workspace-rename')));
  await tester.pumpAndSettle();
  await tester.enterText(
    find.byKey(const ValueKey('rename-project-name')),
    name,
  );
  await tester.tap(find.byKey(const ValueKey('confirm-rename-project')));
  await tester.pumpAndSettle();
  expect(
    tester
        .widget<Text>(find.byKey(const ValueKey('workspace-project-name')))
        .data,
    name,
  );
}

class _NativeProjectPicker implements ProjectFilePicker {
  _NativeProjectPicker(this.projectPath);

  final String projectPath;
  String? openPath;

  @override
  bool get isSupported => true;

  @override
  bool get supportsMediaImport => true;

  @override
  bool get supportsExport => true;

  @override
  Future<String?> openProjectPath() async => openPath;

  @override
  Future<String?> openMediaPath() async => null;

  @override
  Future<String?> saveProjectPath({required String suggestedName}) async =>
      projectPath;

  @override
  Future<String?> saveExportPath({required String suggestedName}) async => null;

  @override
  Future<ProjectFileSyncResult?> synchronizeProjectPath(String path) async =>
      null;
}

Uint8List _silentWave({required int sampleRate, required int frames}) {
  final sampleBytes = frames * 2;
  final bytes = Uint8List(44 + sampleBytes);
  void writeAscii(int offset, String value) {
    bytes.setRange(offset, offset + value.length, value.codeUnits);
  }

  void writeU16(int offset, int value) {
    bytes[offset] = value & 0xff;
    bytes[offset + 1] = (value >> 8) & 0xff;
  }

  void writeU32(int offset, int value) {
    for (var byte = 0; byte < 4; byte++) {
      bytes[offset + byte] = (value >> (8 * byte)) & 0xff;
    }
  }

  writeAscii(0, 'RIFF');
  writeU32(4, 36 + sampleBytes);
  writeAscii(8, 'WAVE');
  writeAscii(12, 'fmt ');
  writeU32(16, 16);
  writeU16(20, 1);
  writeU16(22, 1);
  writeU32(24, sampleRate);
  writeU32(28, sampleRate * 2);
  writeU16(32, 2);
  writeU16(34, 16);
  writeAscii(36, 'data');
  writeU32(40, sampleBytes);
  return bytes;
}

class _ObservedRustProjectGateway implements ProjectGateway {
  ProjectSessionHandle? activeSession;
  Object? lastError;
  final Completer<void> _projectCreationCompletion = Completer<void>();
  Future<void> get projectCreationCompletion =>
      _projectCreationCompletion.future;
  static const _gateway = RustProjectGateway();

  @override
  Future<ProjectSessionHandle> createProject(String path, String name) async {
    try {
      return activeSession = await _gateway.createProject(path, name);
    } catch (error) {
      lastError = error;
      rethrow;
    } finally {
      _projectCreationCompletion.complete();
    }
  }

  @override
  Future<ProjectSessionHandle> openProject(String path) async =>
      activeSession = await _gateway.openProject(path);

  @override
  Future<ProjectReadModel> summary(ProjectSessionHandle session) =>
      _gateway.summary(session);

  @override
  Future<ProjectPreviewState> previewState(ProjectSessionHandle session) =>
      _gateway.previewState(session);

  @override
  Future<ProjectPreviewState> previewSeek(
    ProjectSessionHandle session,
    ProjectRationalTime position,
  ) => _gateway.previewSeek(session, position);

  @override
  Future<ProjectPreviewState> previewPlay(ProjectSessionHandle session) =>
      _gateway.previewPlay(session);

  @override
  Future<ProjectPreviewState> previewPause(ProjectSessionHandle session) =>
      _gateway.previewPause(session);

  @override
  Future<ProjectPreviewState> previewStep(
    ProjectSessionHandle session,
    ProjectPreviewFrameStep direction,
  ) => _gateway.previewStep(session, direction);

  @override
  Future<ProjectPreviewState> previewTick(ProjectSessionHandle session) =>
      _gateway.previewTick(session);

  @override
  Future<ProjectActionResult> setTimelineSequenceFrameRate(
    ProjectSessionHandle session,
    ProjectReadModel current,
    ProjectRationalRate? sequenceFrameRate,
  ) => _gateway.setTimelineSequenceFrameRate(
    session,
    current,
    sequenceFrameRate,
  );

  @override
  Future<ProjectActionResult> rename(
    ProjectSessionHandle session,
    ProjectReadModel current,
    String name,
  ) => _gateway.rename(session, current, name);

  @override
  Future<ProjectActionResult> undo(
    ProjectSessionHandle session,
    ProjectReadModel current,
  ) => _gateway.undo(session, current);

  @override
  Future<ProjectActionResult> redo(
    ProjectSessionHandle session,
    ProjectReadModel current,
  ) => _gateway.redo(session, current);

  @override
  Future<ProjectMediaPage> listMediaPage(
    ProjectSessionHandle session, {
    required int offset,
    required int limit,
  }) => _gateway.listMediaPage(session, offset: offset, limit: limit);

  @override
  Future<ProjectTimelineTracks> listTimelineTracks(
    ProjectSessionHandle session,
  ) => _gateway.listTimelineTracks(session);

  @override
  Future<ProjectTimelineClipPage> listTimelineClips(
    ProjectSessionHandle session, {
    required String trackId,
    required int offset,
    required int limit,
  }) => _gateway.listTimelineClips(
    session,
    trackId: trackId,
    offset: offset,
    limit: limit,
  );

  @override
  Future<ProjectTimelineVisualSettings> getTimelineClipVisualSettings(
    ProjectSessionHandle session,
    ProjectReadModel current, {
    required String trackId,
    required String clipId,
  }) => _gateway.getTimelineClipVisualSettings(
    session,
    current,
    trackId: trackId,
    clipId: clipId,
  );

  @override
  Future<ProjectActionResult> updateTimelineClipVisualSettings(
    ProjectSessionHandle session,
    ProjectReadModel current, {
    required String trackId,
    required String clipId,
    required ProjectTimelineVisualSettings settings,
  }) => _gateway.updateTimelineClipVisualSettings(
    session,
    current,
    trackId: trackId,
    clipId: clipId,
    settings: settings,
  );

  @override
  Future<ProjectTimelineAudioSettings> getTimelineClipAudioSettings(
    ProjectSessionHandle session,
    ProjectReadModel current, {
    required String trackId,
    required String clipId,
  }) => _gateway.getTimelineClipAudioSettings(
    session,
    current,
    trackId: trackId,
    clipId: clipId,
  );

  @override
  Future<ProjectActionResult> updateTimelineClipAudioSettings(
    ProjectSessionHandle session,
    ProjectReadModel current, {
    required String trackId,
    required String clipId,
    required ProjectTimelineAudioSettings settings,
  }) => _gateway.updateTimelineClipAudioSettings(
    session,
    current,
    trackId: trackId,
    clipId: clipId,
    settings: settings,
  );

  @override
  Future<ProjectTimelineMarkerPage> listTimelineMarkers(
    ProjectSessionHandle session, {
    required int offset,
    required int limit,
  }) => _gateway.listTimelineMarkers(session, offset: offset, limit: limit);

  @override
  Future<ProjectTimelineSnapResult> resolveTimelineSnap(
    ProjectSessionHandle session,
    ProjectReadModel current, {
    required ProjectTimelineSnapOperation operation,
    required String clipId,
    String? targetTrackId,
    required ProjectRationalTime targetTime,
  }) => _gateway.resolveTimelineSnap(
    session,
    current,
    operation: operation,
    clipId: clipId,
    targetTrackId: targetTrackId,
    targetTime: targetTime,
  );

  @override
  Future<ProjectActionResult> addTimelineTrack(
    ProjectSessionHandle session,
    ProjectReadModel current,
    ProjectTimelineTrackKind kind,
  ) => _gateway.addTimelineTrack(session, current, kind);

  @override
  Future<ProjectActionResult> removeTimelineTrack(
    ProjectSessionHandle session,
    ProjectReadModel current,
    String trackId,
  ) => _gateway.removeTimelineTrack(session, current, trackId);

  @override
  Future<ProjectActionResult> setTimelineTrackState(
    ProjectSessionHandle session,
    ProjectReadModel current, {
    required String trackId,
    required ProjectTimelineTrackState state,
  }) => _gateway.setTimelineTrackState(
    session,
    current,
    trackId: trackId,
    state: state,
  );

  @override
  Future<ProjectActionResult> insertTimelineClip(
    ProjectSessionHandle session,
    ProjectReadModel current, {
    required String trackId,
    required String mediaId,
    required ProjectRationalTime timelineStart,
    required ProjectRationalTime sourceStart,
    required ProjectRationalTime duration,
  }) => _gateway.insertTimelineClip(
    session,
    current,
    trackId: trackId,
    mediaId: mediaId,
    timelineStart: timelineStart,
    sourceStart: sourceStart,
    duration: duration,
  );

  @override
  Future<ProjectActionResult> insertTimelineTextClip(
    ProjectSessionHandle session,
    ProjectReadModel current, {
    required String trackId,
    required ProjectRationalTime timelineStart,
    required ProjectRationalTime timelineDuration,
    required ProjectTimelineTextContent content,
  }) => _gateway.insertTimelineTextClip(
    session,
    current,
    trackId: trackId,
    timelineStart: timelineStart,
    timelineDuration: timelineDuration,
    content: content,
  );

  @override
  Future<ProjectActionResult> updateTimelineTextClip(
    ProjectSessionHandle session,
    ProjectReadModel current, {
    required String trackId,
    required String clipId,
    required ProjectRationalTime timelineDuration,
    required ProjectTimelineTextContent content,
  }) => _gateway.updateTimelineTextClip(
    session,
    current,
    trackId: trackId,
    clipId: clipId,
    timelineDuration: timelineDuration,
    content: content,
  );

  @override
  Future<ProjectActionResult> moveTimelineClip(
    ProjectSessionHandle session,
    ProjectReadModel current, {
    required String clipId,
    required String trackId,
    required ProjectRationalTime timelineStart,
  }) => _gateway.moveTimelineClip(
    session,
    current,
    clipId: clipId,
    trackId: trackId,
    timelineStart: timelineStart,
  );

  @override
  Future<ProjectActionResult> deleteTimelineClip(
    ProjectSessionHandle session,
    ProjectReadModel current,
    String clipId,
  ) => _gateway.deleteTimelineClip(session, current, clipId);

  @override
  Future<ProjectActionResult> trimTimelineClip(
    ProjectSessionHandle session,
    ProjectReadModel current, {
    required String clipId,
    required ProjectTimelineTrimEdge edge,
    required ProjectRationalTime timelineTime,
  }) => _gateway.trimTimelineClip(
    session,
    current,
    clipId: clipId,
    edge: edge,
    timelineTime: timelineTime,
  );

  @override
  Future<ProjectActionResult> splitTimelineClip(
    ProjectSessionHandle session,
    ProjectReadModel current, {
    required String clipId,
    required ProjectRationalTime timelineTime,
  }) => _gateway.splitTimelineClip(
    session,
    current,
    clipId: clipId,
    timelineTime: timelineTime,
  );

  @override
  Future<ProjectActionResult> rippleDeleteTimelineClip(
    ProjectSessionHandle session,
    ProjectReadModel current,
    String clipId,
  ) => _gateway.rippleDeleteTimelineClip(session, current, clipId);

  @override
  Future<ProjectActionResult> addTimelineMarker(
    ProjectSessionHandle session,
    ProjectReadModel current, {
    required ProjectRationalTime timelineTime,
    required String label,
  }) => _gateway.addTimelineMarker(
    session,
    current,
    timelineTime: timelineTime,
    label: label,
  );

  @override
  Future<ProjectActionResult> moveTimelineMarker(
    ProjectSessionHandle session,
    ProjectReadModel current, {
    required String markerId,
    required ProjectRationalTime timelineTime,
  }) => _gateway.moveTimelineMarker(
    session,
    current,
    markerId: markerId,
    timelineTime: timelineTime,
  );

  @override
  Future<ProjectActionResult> renameTimelineMarker(
    ProjectSessionHandle session,
    ProjectReadModel current, {
    required String markerId,
    required String label,
  }) => _gateway.renameTimelineMarker(
    session,
    current,
    markerId: markerId,
    label: label,
  );

  @override
  Future<ProjectActionResult> deleteTimelineMarker(
    ProjectSessionHandle session,
    ProjectReadModel current,
    String markerId,
  ) => _gateway.deleteTimelineMarker(session, current, markerId);

  @override
  Future<ProjectMediaArtifactRequest> requestMediaThumbnail(
    ProjectSessionHandle session,
    String mediaId,
  ) => _gateway.requestMediaThumbnail(session, mediaId);

  @override
  Future<ProjectMediaArtifactRequest> requestMediaWaveform(
    ProjectSessionHandle session,
    String mediaId,
  ) => _gateway.requestMediaWaveform(session, mediaId);

  @override
  Future<ProjectMediaArtifact?> readMediaArtifact(
    ProjectSessionHandle session, {
    required ProjectMediaArtifactKind kind,
    required String cacheKey,
  }) => _gateway.readMediaArtifact(session, kind: kind, cacheKey: cacheKey);

  @override
  Stream<ProjectMediaArtifactEvent> watchMediaArtifacts(
    ProjectSessionHandle session,
  ) => _gateway.watchMediaArtifacts(session);

  @override
  Future<ProjectActionResult> importMedia(
    ProjectSessionHandle session,
    ProjectReadModel current,
    String path,
  ) => _gateway.importMedia(session, current, path);

  @override
  Future<ProjectActionResult> removeMedia(
    ProjectSessionHandle session,
    ProjectReadModel current,
    String mediaId,
  ) => _gateway.removeMedia(session, current, mediaId);

  @override
  Future<ProjectActionResult> save(ProjectSessionHandle session) =>
      _gateway.save(session);

  @override
  Future<ProjectActionResult> autosaveCheckpoint(
    ProjectSessionHandle session,
  ) => _gateway.autosaveCheckpoint(session);

  @override
  Future<ProjectExportJob> startExport(
    ProjectSessionHandle session,
    ProjectReadModel current,
    String destination,
  ) => _gateway.startExport(session, current, destination);

  @override
  Future<ProjectExportJob> exportStatus(
    ProjectSessionHandle session,
    ProjectReadModel current,
    String jobId,
  ) => _gateway.exportStatus(session, current, jobId);

  @override
  Future<ProjectExportJob> cancelExport(
    ProjectSessionHandle session,
    ProjectReadModel current,
    String jobId,
  ) => _gateway.cancelExport(session, current, jobId);

  @override
  Future<void> close(
    ProjectSessionHandle session, {
    required bool discardUnsaved,
  }) async {
    await _gateway.close(session, discardUnsaved: discardUnsaved);
    if (identical(activeSession, session)) activeSession = null;
  }

  @override
  Stream<ProjectHostEvent> watch(ProjectSessionHandle session) =>
      _gateway.watch(session);

  @override
  Future<ProjectRecoveryInspection> inspectRecovery(String path) =>
      _gateway.inspectRecovery(path);

  @override
  Future<ProjectRecoveryActionResult> applyRecovery(String path) =>
      _gateway.applyRecovery(path);

  @override
  Future<ProjectRecoveryActionResult> discardRecovery(String path) =>
      _gateway.discardRecovery(path);
}
