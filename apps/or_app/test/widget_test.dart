import 'dart:async';
import 'dart:convert';
import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/core_gateway.dart';
import 'package:or_app/design/or_colors.dart';
import 'package:or_app/design/or_spacing.dart';
import 'package:or_app/main.dart';
import 'package:or_app/project/project_file_picker.dart';
import 'package:or_app/project/project_gateway.dart';
import 'package:or_app/screens/settings_screen.dart';
import 'package:or_app/shell/app_navigation.dart';
import 'package:or_app/shell/app_top_bar.dart';

void main() {
  testWidgets('wide shell shows OR navigation and project destinations', (
    tester,
  ) async {
    _setViewport(tester, const Size(1440, 900));
    await _mount(tester);

    expect(find.byKey(const ValueKey('app-top-bar')), findsOneWidget);
    expect(find.byKey(const ValueKey('compact-navigation')), findsNothing);
    expect(find.byKey(const ValueKey('nav-home')), findsOneWidget);
    expect(find.text('Opencut Reinforced'), findsOneWidget);
    expect(find.text('Home'), findsWidgets);
    expect(find.text('New Project'), findsOneWidget);
    expect(find.text('Open Project'), findsOneWidget);
    expect(find.text('No recent projects yet'), findsOneWidget);
    expect(
      find.textContaining('import media, edit a timeline'),
      findsOneWidget,
    );
    expect(find.textContaining('Matroska export workflows'), findsOneWidget);

    await tester.tap(find.byKey(const ValueKey('nav-projects')));
    await tester.pumpAndSettle();
    expect(find.text('No active project'), findsOneWidget);
  });

  testWidgets('compact shell reaches all five primary destinations', (
    tester,
  ) async {
    _setViewport(tester, const Size(390, 844));
    await _mount(tester);
    expect(find.byKey(const ValueKey('compact-navigation')), findsOneWidget);

    final destinations = [
      (AppDestination.projects, 'No active project'),
      (AppDestination.templates, 'No templates available'),
      (AppDestination.assets, 'No assets available'),
      (AppDestination.settings, 'Focused Monochrome'),
      (AppDestination.home, 'No recent projects yet'),
    ];
    for (final (destination, expectedText) in destinations) {
      await tester.tap(find.byKey(ValueKey('nav-${destination.name}')));
      await tester.pumpAndSettle();
      expect(find.text(expectedText), findsOneWidget);
      expect(tester.takeException(), isNull);
    }
  });

  testWidgets('medium desktop page header fits beside the navigation rail', (
    tester,
  ) async {
    _setViewport(tester, const Size(760, 900));
    await _mount(tester);
    await tester.tap(find.byKey(const ValueKey('nav-projects')));
    await tester.pumpAndSettle();
    expect(find.text('Projects'), findsWidgets);
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    'compact top bar keeps export status accessible without overflow',
    (tester) async {
      _setViewport(tester, const Size(320, 640));
      const status = 'Export complete';
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: Column(
              children: [
                AppTopBar(
                  title: 'SAF preview acceptance',
                  compact: true,
                  onOpenCommandPalette: () {},
                  onHome: () {},
                  onExport: () {},
                  onCancelExport: () {},
                  exportIsActive: true,
                  statusLabel: status,
                ),
                const Expanded(child: SizedBox()),
              ],
            ),
          ),
        ),
      );

      expect(find.byTooltip(status), findsOneWidget);
      expect(find.byKey(const ValueKey('export-status')), findsOneWidget);
      expect(find.text(status), findsNothing);
      expect(tester.takeException(), isNull);
    },
  );

  test('Focused Monochrome tokens and responsive policy are centralized', () {
    expect(OrColors.background.toARGB32(), 0xFF090909);
    expect(OrColors.surface.toARGB32(), 0xFF151516);
    expect(OrColors.border.toARGB32(), 0xFF2A2A2D);
    expect(OrColors.text.toARGB32(), 0xFFF5F5F5);
    expect(OrColors.selection.toARGB32(), 0xFF3FC7FF);
    expect(OrColors.ai.toARGB32(), 0xFF9B7CFF);
    expect(OrSpacing.x1, 4);
    expect(OrSpacing.x12, 48);
    expect(OrRadii.control, 8);
    expect(OrRadii.panel, 12);
    expect(OrBreakpoints.isCompact(759), isTrue);
    expect(OrBreakpoints.isCompact(760), isFalse);
    expect(OrBreakpoints.isWide(1179), isFalse);
    expect(OrBreakpoints.isWide(1180), isTrue);
  });

  testWidgets('settings diagnostics remain sourced from CoreGateway', (
    tester,
  ) async {
    _setViewport(tester, const Size(1440, 900));
    await _mount(tester);
    await tester.tap(find.byKey(const ValueKey('nav-settings')));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('settings-section-advanced')));
    await tester.pumpAndSettle();

    expect(find.text('Fixture Core App'), findsOneWidget);
    expect(find.text('9.8.7-test'), findsOneWidget);
    expect(find.text('healthy-from-gateway'), findsOneWidget);
    expect(find.text('core.app_info'), findsOneWidget);
    expect(find.text('core.health'), findsOneWidget);
    expect(find.text('core.capabilities'), findsOneWidget);
    expect(find.text('Hardcoded Flutter diagnostics'), findsNothing);
  });

  testWidgets(
    'settings shows local CLI details only when a descriptor exists',
    (tester) async {
      _setViewport(tester, const Size(1440, 900));
      ProjectReadModel project(String descriptorPath) => ProjectReadModel(
        projectId: 'project-id',
        projectInstanceId: 'project-instance-id',
        revision: BigInt.zero,
        name: 'Preview project',
        dirty: false,
        descriptorPath: descriptorPath,
      );
      Widget settings(ProjectReadModel project) => MaterialApp(
        home: Scaffold(
          body: SettingsScreen(
            gateway: _FakeCoreGateway(),
            project: project,
            onCopyDescriptor: () {},
            onOpenEditorPreview: () {},
          ),
        ),
      );

      await tester.pumpWidget(settings(project('/tmp/or-session.json')));
      await tester.tap(find.byKey(const ValueKey('settings-section-advanced')));
      await tester.pumpAndSettle();
      expect(find.text('Local CLI session'), findsOneWidget);
      expect(find.text('/tmp/or-session.json'), findsOneWidget);

      await tester.pumpWidget(settings(project('')));
      await tester.pumpAndSettle();
      expect(find.text('Local CLI session'), findsNothing);
      expect(
        find.byKey(const ValueKey('local-cli-descriptor-path')),
        findsNothing,
      );
    },
  );

  testWidgets('new project asks for a name and opens the created workspace', (
    tester,
  ) async {
    _setViewport(tester, const Size(1440, 900));
    final gateway = _FakeProjectGateway();
    final picker = _FakeProjectPicker()..savePath = '/tmp/or-widget-demo';
    await _mount(tester, gateway: gateway, picker: picker);

    await tester.tap(find.byKey(const ValueKey('home-new-project')));
    await tester.pumpAndSettle();
    expect(find.byKey(const ValueKey('new-project-name')), findsOneWidget);
    expect(gateway.createCalls, 0);
    await tester.enterText(
      find.byKey(const ValueKey('new-project-name')),
      'Demo Project ',
    );
    await tester.tap(find.byKey(const ValueKey('confirm-new-project')));
    await tester.pumpAndSettle();

    expect(picker.saveCalls, 1);
    expect(gateway.lastCreatePath, '/tmp/or-widget-demo.orproj');
    expect(gateway.lastCreateName, 'Demo Project ');
    expect(gateway.inspectCalls, 1);
    expect(gateway.createCalls, 1);
    expect(
      find.byKey(const ValueKey('workspace-project-name')),
      findsOneWidget,
    );
    expect(find.text('Revision 0'), findsOneWidget);
    expect(find.text('Saved'), findsOneWidget);
    expect(find.text('No timeline tracks'), findsOneWidget);
    expect(find.text('No media loaded'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets('canceling the new-project name or location creates nothing', (
    tester,
  ) async {
    _setViewport(tester, const Size(1440, 900));
    final gateway = _FakeProjectGateway();
    final picker = _FakeProjectPicker()..savePath = null;
    await _mount(tester, gateway: gateway, picker: picker);

    await tester.tap(find.byKey(const ValueKey('home-new-project')));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Cancel'));
    await tester.pumpAndSettle();
    expect(picker.saveCalls, 0);
    expect(gateway.createCalls, 0);

    await tester.tap(find.byKey(const ValueKey('home-new-project')));
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byKey(const ValueKey('new-project-name')),
      'Cancelled at picker',
    );
    await tester.tap(find.byKey(const ValueKey('confirm-new-project')));
    await tester.pumpAndSettle();
    expect(picker.saveCalls, 1);
    expect(gateway.createCalls, 0);
    expect(find.text('Editor layout preview'), findsNothing);
  });

  testWidgets('open project uses the picker and Rust read model', (
    tester,
  ) async {
    _setViewport(tester, const Size(1440, 900));
    final gateway = _FakeProjectGateway();
    final picker = _FakeProjectPicker()..openPath = '/tmp/opened.orproj';
    await _mount(tester, gateway: gateway, picker: picker);

    await tester.tap(find.byKey(const ValueKey('home-open-project')));
    await tester.pumpAndSettle();
    expect(picker.openCalls, 1);
    expect(gateway.lastOpenPath, '/tmp/opened.orproj');
    expect(gateway.openCalls, 1);
    expect(
      find.byKey(const ValueKey('workspace-project-name')),
      findsOneWidget,
    );
    expect(
      find.byKey(const ValueKey('workspace-project-revision')),
      findsOneWidget,
    );
    expect(tester.takeException(), isNull);
  });

  testWidgets('active project requests previews for supported media only', (
    tester,
  ) async {
    _setViewport(tester, const Size(1440, 900));
    final video = _mediaFixture('media-video', '/tmp/source clip.mp4');
    final audio = _audioMediaFixture('media-audio', '/tmp/music.wav');
    final unavailable = _mediaFixture(
      'media-unavailable',
      '/tmp/unavailable.mp4',
    );
    final unsupported = _unsupportedMediaFixture(
      'media-other',
      '/tmp/other.dat',
    );
    final png = await _tinyPngBytes();
    final gateway = _FakeProjectGateway()
      ..initialMedia = [video, audio, unavailable, unsupported]
      ..artifactRequestResponses['media-video:thumbnail'] =
          const ProjectMediaArtifactRequest(
            mediaId: 'media-video',
            kind: ProjectMediaArtifactKind.thumbnail,
            state: ProjectMediaArtifactRequestState.queued,
            cacheKey: 'thumbnail-cache',
            jobId: 'thumbnail-job',
          )
      ..artifactRequestResponses['media-audio:waveform'] =
          const ProjectMediaArtifactRequest(
            mediaId: 'media-audio',
            kind: ProjectMediaArtifactKind.waveform,
            state: ProjectMediaArtifactRequestState.queued,
            cacheKey: 'waveform-cache',
            jobId: 'waveform-job',
          )
      ..artifactResponses['thumbnail-cache'] = ProjectMediaArtifact(
        bytes: png,
        mimeType: 'image/png',
      )
      ..artifactResponses['waveform-cache'] = ProjectMediaArtifact(
        bytes: png,
        mimeType: 'image/png',
      );
    final picker = _FakeProjectPicker()..savePath = '/tmp/media-library.orproj';
    await _mount(tester, gateway: gateway, picker: picker);
    await _createProject(tester, 'Media Library');

    expect(find.byKey(const ValueKey('project-media-panel')), findsOneWidget);
    expect(find.text('source clip.mp4'), findsOneWidget);
    expect(find.text('music.wav'), findsOneWidget);
    expect(
      find.text('mp4 · 10 s · 1920×1080 h264 · Audio · AAC'),
      findsNWidgets(2),
    );
    expect(gateway.thumbnailRequests, 2);
    expect(gateway.waveformRequests, 1);
    expect(gateway.mediaPreviewReads, 0);
    expect(
      find.byKey(const ValueKey('media-preview-media-unavailable')),
      findsOneWidget,
    );
    expect(find.text('CACHE_UNAVAILABLE'), findsNothing);
    expect(find.text('Preview unavailable'), findsNothing);
    expect(
      find.byKey(const ValueKey('media-preview-image-media-video')),
      findsNothing,
    );
    expect(
      find.byKey(const ValueKey('media-preview-image-media-audio')),
      findsNothing,
    );
    expect(gateway.lastSession!.view.revision, BigInt.zero);

    gateway.emitMediaArtifact(
      gateway.lastSession!,
      ProjectMediaArtifactEvent(
        sequence: BigInt.one,
        mediaId: 'media-video',
        kind: ProjectMediaArtifactKind.thumbnail,
        cacheKey: 'thumbnail-cache',
        jobId: 'thumbnail-job',
        state: ProjectMediaArtifactEventState.succeeded,
      ),
    );
    gateway.emitMediaArtifact(
      gateway.lastSession!,
      ProjectMediaArtifactEvent(
        sequence: BigInt.from(2),
        mediaId: 'media-audio',
        kind: ProjectMediaArtifactKind.waveform,
        cacheKey: 'waveform-cache',
        jobId: 'waveform-job',
        state: ProjectMediaArtifactEventState.succeeded,
      ),
    );
    await tester.pumpAndSettle();

    expect(gateway.mediaPreviewReads, 2);
    expect(
      find.byKey(const ValueKey('media-preview-image-media-video')),
      findsOneWidget,
    );
    expect(
      find.byKey(const ValueKey('media-preview-image-media-audio')),
      findsOneWidget,
    );
    expect(gateway.thumbnailRequests, 2);
    expect(gateway.waveformRequests, 1);
    expect(gateway.lastSession!.view.revision, BigInt.zero);

    await tester.tap(find.byKey(const ValueKey('media-actions-media-video')));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('media-remove-media-video')));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('confirm-remove-media')));
    await tester.pumpAndSettle();
    expect(find.byKey(const ValueKey('media-name-media-video')), findsNothing);
    expect(
      find.byKey(const ValueKey('media-preview-image-media-video')),
      findsNothing,
    );
    expect(find.text('No media loaded'), findsOneWidget);
    expect(find.text('No timeline tracks'), findsOneWidget);
    expect(find.byKey(const ValueKey('media-load-more')), findsNothing);
    expect(tester.takeException(), isNull);
  });

  testWidgets('cached media preview is read without waiting for an event', (
    tester,
  ) async {
    _setViewport(tester, const Size(1440, 900));
    final png = await _tinyPngBytes();
    final gateway = _FakeProjectGateway()
      ..initialMedia = [_mediaFixture('cached-video', '/tmp/cached.mp4')]
      ..artifactRequestResponses['cached-video:thumbnail'] =
          const ProjectMediaArtifactRequest(
            mediaId: 'cached-video',
            kind: ProjectMediaArtifactKind.thumbnail,
            state: ProjectMediaArtifactRequestState.ready,
            cacheKey: 'cached-thumbnail',
          )
      ..artifactResponses['cached-thumbnail'] = ProjectMediaArtifact(
        bytes: png,
        mimeType: 'image/png',
      );
    final picker = _FakeProjectPicker()
      ..savePath = '/tmp/cached-preview.orproj';
    await _mount(tester, gateway: gateway, picker: picker);
    await _createProject(tester, 'Cached Preview');

    expect(gateway.thumbnailRequests, 1);
    expect(gateway.mediaPreviewReads, 1);
    expect(
      find.byKey(const ValueKey('media-preview-image-cached-video')),
      findsOneWidget,
    );
    expect(gateway.lastSession!.view.revision, BigInt.zero);
    expect(tester.takeException(), isNull);
  });

  testWidgets('Import Media calls the gateway and refreshes the page', (
    tester,
  ) async {
    _setViewport(tester, const Size(1440, 900));
    final gateway = _FakeProjectGateway();
    final picker = _FakeProjectPicker()
      ..savePath = '/tmp/media-import.orproj'
      ..mediaPath = '/tmp/imported.mov';
    await _mount(tester, gateway: gateway, picker: picker);
    await _createProject(tester, 'Import Media');

    final queriesBeforeImport = gateway.mediaListCalls;
    await tester.tap(find.byKey(const ValueKey('media-import')));
    await tester.pumpAndSettle();

    expect(picker.mediaOpenCalls, 1);
    expect(gateway.importMediaCalls, 1);
    expect(gateway.lastImportSource, '/tmp/imported.mov');
    expect(gateway.mediaListCalls, greaterThan(queriesBeforeImport));
    expect(find.text('imported.mov'), findsOneWidget);
    expect(find.text('Unsaved changes'), findsOneWidget);
    expect(find.text('No media loaded'), findsOneWidget);
  });

  testWidgets('Import Media adds each selected source through the gateway', (
    tester,
  ) async {
    _setViewport(tester, const Size(1440, 900));
    final gateway = _FakeProjectGateway();
    final picker = _FakeProjectPicker()
      ..savePath = '/tmp/media-batch.orproj'
      ..mediaPaths = ['/tmp/first.mkv', '/tmp/second.mkv'];
    await _mount(tester, gateway: gateway, picker: picker);
    await _createProject(tester, 'Import Media Batch');

    await tester.tap(find.byKey(const ValueKey('media-import')));
    await tester.pumpAndSettle();

    expect(picker.mediaOpenCalls, 1);
    expect(gateway.importMediaCalls, 2);
    expect(gateway.importMediaSources, ['/tmp/first.mkv', '/tmp/second.mkv']);
    expect(gateway.lastSession!.view.revision, BigInt.from(2));
    expect(find.text('first.mkv'), findsOneWidget);
    expect(find.text('second.mkv'), findsOneWidget);
  });

  testWidgets('media library loads later items through bounded pages', (
    tester,
  ) async {
    _setViewport(tester, const Size(1440, 900));
    final gateway = _FakeProjectGateway()
      ..initialMedia = List.generate(
        55,
        (index) => _mediaFixture('media-$index', '/tmp/clip-$index.mp4'),
      );
    final picker = _FakeProjectPicker()..savePath = '/tmp/paginated.orproj';
    await _mount(tester, gateway: gateway, picker: picker);
    await _createProject(tester, 'Paginated Media');

    final panelScroll = find.descendant(
      of: find.byKey(const ValueKey('project-media-panel')),
      matching: find.byType(Scrollable),
    );
    await tester.scrollUntilVisible(
      find.byKey(const ValueKey('media-load-more')),
      160,
      scrollable: panelScroll.first,
    );
    await tester.tap(find.byKey(const ValueKey('media-load-more')));
    await tester.pumpAndSettle();

    expect(gateway.mediaListOffsets, [0, 50]);
    expect(find.byKey(const ValueKey('media-load-more')), findsNothing);
    await tester.scrollUntilVisible(
      find.byKey(const ValueKey('media-name-media-54')),
      160,
      scrollable: panelScroll.first,
    );
    expect(find.text('clip-54.mp4'), findsOneWidget);
  });

  testWidgets('probe backend unavailability is shown honestly', (tester) async {
    _setViewport(tester, const Size(1440, 900));
    final gateway = _FakeProjectGateway()..nextImportBackendUnavailable = true;
    final picker = _FakeProjectPicker()
      ..savePath = '/tmp/media-backend.orproj'
      ..mediaPath = '/tmp/clip.mp4';
    await _mount(tester, gateway: gateway, picker: picker);
    await _createProject(tester, 'Backend');

    await tester.tap(find.byKey(const ValueKey('media-import')));
    await tester.pumpAndSettle();

    expect(gateway.importMediaCalls, 1);
    expect(
      find.text(
        'Imported 0 of 1 selected files. 1 could not be imported. '
        'The packaged media inspector could not start. Check the app installation and try again.',
      ),
      findsOneWidget,
    );
    expect(find.text('clip.mp4'), findsNothing);
  });

  testWidgets('media import continues after one selected source fails', (
    tester,
  ) async {
    _setViewport(tester, const Size(1440, 900));
    final gateway = _FakeProjectGateway()..failedImportCalls = {2};
    final picker = _FakeProjectPicker()
      ..savePath = '/tmp/partial-media.orproj'
      ..mediaPaths = ['/tmp/first.mkv', '/tmp/bad.mkv', '/tmp/last.mkv'];
    await _mount(tester, gateway: gateway, picker: picker);
    await _createProject(tester, 'Partial import');

    await tester.tap(find.byKey(const ValueKey('media-import')));
    await tester.pumpAndSettle();

    expect(gateway.importMediaCalls, 3);
    expect(gateway.importMediaSources, [
      '/tmp/first.mkv',
      '/tmp/bad.mkv',
      '/tmp/last.mkv',
    ]);
    expect(find.text('first.mkv'), findsOneWidget);
    expect(find.text('last.mkv'), findsOneWidget);
    expect(find.text('bad.mkv'), findsNothing);
    expect(
      find.textContaining(
        'Imported 2 of 3 selected files. 1 could not be imported.',
      ),
      findsOneWidget,
    );
  });

  testWidgets('missing media feedback explains the recovery action', (
    tester,
  ) async {
    _setViewport(tester, const Size(1440, 900));
    final gateway = _FakeProjectGateway()..nextImportSourceNotFound = true;
    final picker = _FakeProjectPicker()
      ..savePath = '/tmp/missing-media.orproj'
      ..mediaPath = '/tmp/missing.mkv';
    await _mount(tester, gateway: gateway, picker: picker);
    await _createProject(tester, 'Missing media');

    await tester.tap(find.byKey(const ValueKey('media-import')));
    await tester.pumpAndSettle();

    expect(
      find.text(
        'Imported 0 of 1 selected files. 1 could not be imported. '
        'The selected media file could not be found. Check that the source still exists.',
      ),
      findsOneWidget,
    );
  });

  testWidgets('relinking replaces a source while keeping the media identity', (
    tester,
  ) async {
    _setViewport(tester, const Size(1440, 900));
    final media = _mediaFixture('media-relink', '/tmp/missing/original.wav');
    final gateway = _FakeProjectGateway()..initialMedia = [media];
    final picker = _FakeProjectPicker()
      ..savePath = '/tmp/relink.orproj'
      ..mediaPath = '/tmp/recovered/replacement.wav';
    await _mount(tester, gateway: gateway, picker: picker);
    await _createProject(tester, 'Relink Media');

    await tester.tap(find.byKey(const ValueKey('media-actions-media-relink')));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('media-relink-media-relink')));
    await tester.pumpAndSettle();

    expect(gateway.relinkMediaCalls, 1);
    expect(gateway.lastRelinkMediaId, media.mediaId);
    expect(gateway.lastRelinkSource, '/tmp/recovered/replacement.wav');
    expect(gateway.lastSession!.media.single.mediaId, media.mediaId);
    expect(
      gateway.lastSession!.media.single.sourceUri,
      '/tmp/recovered/replacement.wav',
    );
    expect(find.text('replacement.wav'), findsOneWidget);
    expect(find.text('original.wav'), findsNothing);
  });

  testWidgets('relink requires exactly one replacement source', (tester) async {
    _setViewport(tester, const Size(1440, 900));
    final gateway = _FakeProjectGateway()
      ..initialMedia = [_mediaFixture('media-relink-many', '/tmp/offline.wav')];
    final picker = _FakeProjectPicker()
      ..savePath = '/tmp/relink-many.orproj'
      ..mediaPaths = ['/tmp/one.wav', '/tmp/two.wav'];
    await _mount(tester, gateway: gateway, picker: picker);
    await _createProject(tester, 'Relink Many');

    await tester.tap(
      find.byKey(const ValueKey('media-actions-media-relink-many')),
    );
    await tester.pumpAndSettle();
    await tester.tap(
      find.byKey(const ValueKey('media-relink-media-relink-many')),
    );
    await tester.pumpAndSettle();

    expect(gateway.relinkMediaCalls, 0);
    expect(
      find.text('Select exactly one replacement media file.'),
      findsOneWidget,
    );
    expect(
      gateway.lastSession!.media.single.sourceUri,
      'file:///tmp/offline.wav',
    );
  });

  testWidgets('removing media requires confirmation and updates the library', (
    tester,
  ) async {
    _setViewport(tester, const Size(1440, 900));
    final gateway = _FakeProjectGateway()
      ..initialMedia = [_mediaFixture('media-remove', '/tmp/remove me.wav')];
    final picker = _FakeProjectPicker()..savePath = '/tmp/media-remove.orproj';
    await _mount(tester, gateway: gateway, picker: picker);
    await _createProject(tester, 'Remove Media');

    await tester.tap(find.byKey(const ValueKey('media-actions-media-remove')));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('media-remove-media-remove')));
    await tester.pumpAndSettle();
    expect(
      find.textContaining('source file will not be deleted'),
      findsOneWidget,
    );
    expect(gateway.removeMediaCalls, 0);

    await tester.tap(find.byKey(const ValueKey('confirm-remove-media')));
    await tester.pumpAndSettle();

    expect(gateway.removeMediaCalls, 1);
    expect(gateway.lastRemovedMediaId, 'media-remove');
    expect(find.text('remove me.wav'), findsNothing);
    expect(find.text('No media imported'), findsOneWidget);
    expect(find.text('No media loaded'), findsOneWidget);
  });

  testWidgets(
    'Projects shows one active project row and only saves when dirty',
    (tester) async {
      _setViewport(tester, const Size(1440, 900));
      final gateway = _FakeProjectGateway();
      final picker = _FakeProjectPicker()..savePath = '/tmp/active.orproj';
      await _mount(tester, gateway: gateway, picker: picker);
      await _createProject(tester, 'Active');
      await tester.tap(find.byKey(const ValueKey('or-brand-home')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('nav-projects')));
      await tester.pumpAndSettle();

      expect(find.byKey(const ValueKey('active-project-card')), findsOneWidget);
      expect(find.text('Active Project'), findsOneWidget);
      expect(find.text('Revision 0'), findsOneWidget);
      expect(find.byKey(const ValueKey('active-project-save')), findsNothing);
      expect(find.text('Recent Project'), findsNothing);
    },
  );

  testWidgets('timeline clip pages load in bounded, revision-safe batches', (
    tester,
  ) async {
    _setViewport(tester, const Size(1280, 800));
    final clips = [
      for (var index = 0; index < 101; index++)
        ProjectTimelineClip(
          clipId: 'bulk-clip-$index',
          mediaId: 'bulk-media',
          timelineStart: ProjectRationalTime(BigInt.from(index * 2), 1),
          sourceStart: ProjectRationalTime(BigInt.zero, 1),
          timelineDuration: ProjectRationalTime(BigInt.one, 1),
        ),
    ];
    final gateway = _FakeProjectGateway()
      ..initialTimelineTracks = [
        const ProjectTimelineTrack(
          trackId: 'bulk-track',
          kind: ProjectTimelineTrackKind.video,
          clipCount: 101,
        ),
      ]
      ..initialTimelineClips = {'bulk-track': clips};
    final picker = _FakeProjectPicker()
      ..savePath = '/tmp/paged-timeline.orproj';
    await _mount(tester, gateway: gateway, picker: picker);
    await _createProject(tester, 'Paged timeline');

    expect(gateway.timelineClipOffsets, [0]);
    expect(find.text('100 / 101'), findsOneWidget);
    final loadMore = find.byKey(
      const ValueKey('timeline-load-more-bulk-track'),
    );
    await tester.ensureVisible(loadMore);
    await tester.tap(loadMore);
    await tester.pumpAndSettle();
    expect(gateway.timelineClipOffsets, [0, 100]);
    expect(find.text('101'), findsOneWidget);
    expect(loadMore, findsNothing);
  });

  testWidgets('timeline marker pages load in bounded, revision-safe batches', (
    tester,
  ) async {
    _setViewport(tester, const Size(1280, 800));
    final markers = [
      for (var index = 0; index < 101; index++)
        ProjectTimelineMarker(
          markerId: 'marker-$index',
          timelineTime: ProjectRationalTime(BigInt.from(index), 1),
          label: 'Marker $index',
        ),
    ];
    final gateway = _FakeProjectGateway()
      ..initialTimelineTracks = const [
        ProjectTimelineTrack(
          trackId: 'marker-track',
          kind: ProjectTimelineTrackKind.video,
          clipCount: 0,
        ),
      ]
      ..initialTimelineMarkers = markers;
    await _mount(
      tester,
      gateway: gateway,
      picker: _FakeProjectPicker()..savePath = '/tmp/paged-markers.orproj',
    );
    await _createProject(tester, 'Paged markers');

    expect(gateway.timelineMarkerOffsets, [0]);
    expect(find.text('Marker 0'), findsOneWidget);
    final loadMore = find.byKey(const ValueKey('timeline-marker-load-more'));
    await tester.ensureVisible(loadMore);
    await tester.tap(loadMore);
    await tester.pumpAndSettle();
    expect(gateway.timelineMarkerOffsets, [0, 100]);
    expect(find.text('Marker 100'), findsOneWidget);
    expect(loadMore, findsNothing);
    expect(tester.takeException(), isNull);
  });

  testWidgets('marker ruler uses canonical marker commands for all actions', (
    tester,
  ) async {
    _setViewport(tester, const Size(1280, 800));
    final gateway = _FakeProjectGateway()
      ..initialTimelineTracks = const [
        ProjectTimelineTrack(
          trackId: 'marker-actions-track',
          kind: ProjectTimelineTrackKind.video,
          clipCount: 0,
        ),
      ];
    await _mount(
      tester,
      gateway: gateway,
      picker: _FakeProjectPicker()..savePath = '/tmp/marker-actions.orproj',
    );
    await _createProject(tester, 'Marker actions');

    await tester.tap(find.byKey(const ValueKey('timeline-add-marker')));
    await tester.pumpAndSettle();
    expect(find.byKey(const ValueKey('timeline-marker-label')), findsOneWidget);
    await tester.enterText(
      find.byKey(const ValueKey('timeline-marker-time')),
      '3/2',
    );
    await tester.enterText(
      find.byKey(const ValueKey('timeline-marker-label')),
      'First cut',
    );
    await tester.pump();
    await tester.tap(find.byKey(const ValueKey('timeline-confirm-add-marker')));
    await tester.pumpAndSettle();
    expect(gateway.addTimelineMarkerCalls, 1);
    expect(gateway.lastMarkerTime?.canonical, '3/2');
    expect(find.text('First cut'), findsOneWidget);

    const markerId = 'marker-1';
    await tester.tap(find.byKey(const ValueKey('timeline-marker-marker-1')));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(ValueKey('timeline-marker-rename-$markerId')));
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byKey(ValueKey('timeline-marker-rename-label-$markerId')),
      'Opening cut',
    );
    await tester.pump();
    await tester.tap(find.byKey(ValueKey('timeline-confirm-rename-$markerId')));
    await tester.pumpAndSettle();
    expect(gateway.renameTimelineMarkerCalls, 1);
    expect(gateway.lastMarkerLabel, 'Opening cut');

    await tester.tap(find.byKey(const ValueKey('timeline-marker-marker-1')));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(ValueKey('timeline-marker-move-$markerId')));
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byKey(const ValueKey('timeline-marker-move-time')),
      '5/2',
    );
    await tester.pump();
    await tester.tap(find.byKey(ValueKey('timeline-confirm-move-$markerId')));
    await tester.pumpAndSettle();
    expect(gateway.moveTimelineMarkerCalls, 1);
    expect(gateway.lastMarkerTime?.canonical, '5/2');

    await tester.tap(find.byKey(const ValueKey('timeline-marker-marker-1')));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(ValueKey('timeline-marker-delete-$markerId')));
    await tester.pumpAndSettle();
    await tester.tap(
      find.byKey(ValueKey('timeline-confirm-delete-marker-$markerId')),
    );
    await tester.pumpAndSettle();
    expect(gateway.deleteTimelineMarkerCalls, 1);
    expect(
      find.byKey(const ValueKey('timeline-marker-marker-1')),
      findsNothing,
    );
    expect(tester.takeException(), isNull);
  });

  testWidgets('stale timeline page is discarded and refreshed coherently', (
    tester,
  ) async {
    _setViewport(tester, const Size(1280, 800));
    final clips = [
      for (var index = 0; index < 101; index++)
        ProjectTimelineClip(
          clipId: 'stale-clip-$index',
          mediaId: 'stale-media',
          timelineStart: ProjectRationalTime(BigInt.from(index * 2), 1),
          sourceStart: ProjectRationalTime(BigInt.zero, 1),
          timelineDuration: ProjectRationalTime(BigInt.one, 1),
        ),
    ];
    final gateway = _FakeProjectGateway()
      ..initialTimelineTracks = [
        const ProjectTimelineTrack(
          trackId: 'stale-track',
          kind: ProjectTimelineTrackKind.video,
          clipCount: 101,
        ),
      ]
      ..initialTimelineClips = {'stale-track': clips};
    final picker = _FakeProjectPicker()
      ..savePath = '/tmp/stale-timeline.orproj';
    await _mount(tester, gateway: gateway, picker: picker);
    await _createProject(tester, 'Stale timeline page');
    gateway.revisionChangeOnNextTimelinePage = true;

    final loadMore = find.byKey(
      const ValueKey('timeline-load-more-stale-track'),
    );
    await tester.ensureVisible(loadMore);
    await tester.tap(loadMore);
    await tester.pumpAndSettle();
    expect(gateway.timelineClipOffsets, [0, 100, 0]);
    expect(find.text('100 / 101'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets('timeline track controls dispatch typed persistent state', (
    tester,
  ) async {
    _setViewport(tester, const Size(1280, 800));
    final gateway = _FakeProjectGateway()
      ..initialTimelineTracks = const [
        ProjectTimelineTrack(
          trackId: 'state-video',
          kind: ProjectTimelineTrackKind.video,
          clipCount: 0,
        ),
        ProjectTimelineTrack(
          trackId: 'state-audio',
          kind: ProjectTimelineTrackKind.audio,
          clipCount: 0,
        ),
      ];
    final picker = _FakeProjectPicker()
      ..savePath = '/tmp/timeline-track-state.orproj';
    await _mount(tester, gateway: gateway, picker: picker);
    await _createProject(tester, 'Track state');

    await tester.tap(
      find.byKey(const ValueKey('timeline-track-enabled-state-video')),
    );
    await tester.pumpAndSettle();
    expect(gateway.lastTrackState?.visible, isFalse);
    expect(gateway.setTimelineTrackStateCalls, 1);

    await tester.tap(
      find.byKey(const ValueKey('timeline-track-lock-state-video')),
    );
    await tester.pumpAndSettle();
    expect(gateway.lastTrackState?.locked, isTrue);
    expect(gateway.lastTrackState?.visible, isFalse);

    await tester.tap(
      find.byKey(const ValueKey('timeline-track-solo-state-video')),
    );
    await tester.pumpAndSettle();
    expect(gateway.lastTrackState?.solo, isTrue);

    await tester.tap(
      find.byKey(const ValueKey('timeline-track-enabled-state-audio')),
    );
    await tester.pumpAndSettle();
    expect(gateway.lastTrackStateId, 'state-audio');
    expect(gateway.lastTrackState?.muted, isTrue);
    expect(gateway.lastSession!.view.revision, BigInt.from(4));
    expect(tester.takeException(), isNull);
  });

  testWidgets('titles and manual captions can be added and edited', (
    tester,
  ) async {
    _setViewport(tester, const Size(1280, 800));
    final gateway = _FakeProjectGateway();
    final picker = _FakeProjectPicker()
      ..savePath = '/tmp/timeline-text-captions.orproj';
    await _mount(tester, gateway: gateway, picker: picker);
    await _createProject(tester, 'Text and captions');

    await tester.ensureVisible(
      find.byKey(const ValueKey('timeline-add-text-track')),
    );
    await tester.tap(find.byKey(const ValueKey('timeline-add-text-track')));
    await tester.pumpAndSettle();
    await tester.ensureVisible(
      find.byKey(const ValueKey('timeline-add-title')),
    );
    await tester.tap(find.byKey(const ValueKey('timeline-add-title')));
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byKey(const ValueKey('timeline-text-content')),
      'Opening title',
    );
    await tester.enterText(
      find.byKey(const ValueKey('timeline-text-duration')),
      '7/2',
    );
    await tester.tap(find.byKey(const ValueKey('timeline-save-text')));
    await tester.pumpAndSettle();

    final title = gateway.lastSession!.clips['track-1']!.single;
    expect(title.contentKind, ProjectTimelineClipContentKind.text);
    expect(title.text, 'Opening title');
    expect(title.timelineStart.canonical, '0/1');
    expect(title.timelineDuration.canonical, '7/2');
    expect(title.formatting?.font, ProjectFontIdentity.bundledInter);
    expect(title.formatting?.sizeMilliPoints, 48000);
    expect(title.formatting?.weight, ProjectTextWeight.regular);
    expect(title.formatting?.alignment, ProjectTextAlignment.center);
    expect(title.formatting?.color.red, 255);
    expect(title.formatting?.color.green, 255);
    expect(title.formatting?.color.blue, 255);
    expect(title.formatting?.color.alpha, 255);
    expect(gateway.insertTimelineTextClipCalls, 1);

    await tester.tap(find.byKey(ValueKey('timeline-clip-${title.clipId}')));
    await tester.pumpAndSettle();
    await tester.tap(
      find.byKey(ValueKey('timeline-edit-text-${title.clipId}')),
    );
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byKey(const ValueKey('timeline-text-content')),
      'Updated title',
    );
    await tester.enterText(
      find.byKey(const ValueKey('timeline-text-duration')),
      '9/2',
    );
    await tester.tap(find.byKey(const ValueKey('timeline-save-text')));
    await tester.pumpAndSettle();

    final updatedTitle = gateway.lastSession!.clips['track-1']!.single;
    expect(gateway.updateTimelineTextClipCalls, 1);
    expect(gateway.lastTextClipId, title.clipId);
    expect(updatedTitle.text, 'Updated title');
    expect(updatedTitle.timelineDuration.canonical, '9/2');

    await tester.ensureVisible(
      find.byKey(const ValueKey('timeline-add-caption-track')),
    );
    await tester.tap(find.byKey(const ValueKey('timeline-add-caption-track')));
    await tester.pumpAndSettle();
    expect(gateway.lastSession!.tracks.map((track) => track.kind), [
      ProjectTimelineTrackKind.text,
      ProjectTimelineTrackKind.caption,
    ]);
    await tester.ensureVisible(
      find.byKey(const ValueKey('timeline-add-manual-caption')),
    );
    await tester.tap(find.byKey(const ValueKey('timeline-add-manual-caption')));
    await tester.pumpAndSettle();
    expect(find.text('Add Manual Caption'), findsOneWidget);
    await tester.enterText(
      find.byKey(const ValueKey('timeline-text-content')),
      'A manual caption',
    );
    await tester.pumpAndSettle();
    await tester.ensureVisible(
      find.byKey(const ValueKey('timeline-save-text')),
    );
    expect(
      tester
          .widget<FilledButton>(
            find.byKey(const ValueKey('timeline-save-text')),
          )
          .onPressed,
      isNotNull,
    );
    await tester.tap(find.byKey(const ValueKey('timeline-save-text')));
    await tester.pumpAndSettle();

    expect(gateway.insertTimelineTextClipCalls, 2);
    final caption =
        gateway.lastSession!.clips[gateway.lastTextTrackId!]!.single;
    expect(caption.contentKind, ProjectTimelineClipContentKind.caption);
    expect(caption.text, 'A manual caption');
    expect(tester.takeException(), isNull);
  });

  testWidgets('caption import explains formatting loss before applying', (
    tester,
  ) async {
    _setViewport(tester, const Size(1280, 800));
    final gateway = _FakeProjectGateway()
      ..captionImportPreview = ProjectCaptionImportPreview(
        formatName: 'WebVTT',
        cueCount: BigInt.from(2),
        formattingLossCount: BigInt.one,
        emptyCuesSkipped: BigInt.one,
      );
    final picker = _FakeProjectPicker()
      ..savePath = '/tmp/caption-import.orproj'
      ..captionPath = '/tmp/captions.vtt';
    await _mount(tester, gateway: gateway, picker: picker);
    await _createProject(tester, 'Caption import');

    final importButton = find.byKey(const ValueKey('timeline-import-captions'));
    await tester.ensureVisible(importButton);
    await tester.tap(importButton);
    await tester.pumpAndSettle();

    expect(find.text('Import WebVTT captions?'), findsOneWidget);
    expect(
      find.textContaining('styling or placement that will be simplified'),
      findsOneWidget,
    );
    expect(find.textContaining('empty cues will be skipped'), findsOneWidget);
    expect(gateway.captionImportCalls, 0);
    await tester.tap(find.text('Cancel'));
    await tester.pumpAndSettle();
    expect(gateway.captionImportCalls, 0);
    expect(picker.captionCleanupCalls, 1);

    await tester.ensureVisible(importButton);
    await tester.tap(importButton);
    await tester.pumpAndSettle();
    await tester.tap(find.text('Import').last);
    await tester.pumpAndSettle();
    expect(gateway.captionImportCalls, 1);
    expect(gateway.importedCaptionPath, '/tmp/captions.vtt');
    expect(picker.captionCleanupCalls, 2);
    expect(find.textContaining('Imported captions from'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets('timeline duplicate preserves exact source range', (
    tester,
  ) async {
    _setViewport(tester, const Size(1280, 800));
    final clip = ProjectTimelineClip(
      clipId: 'duplicate-clip',
      mediaId: 'duplicate-media',
      timelineStart: ProjectRationalTime(BigInt.one, 3),
      sourceStart: ProjectRationalTime(BigInt.one, 5),
      timelineDuration: ProjectRationalTime(BigInt.from(7), 3),
    );
    final gateway = _FakeProjectGateway()
      ..initialTimelineTracks = const [
        ProjectTimelineTrack(
          trackId: 'duplicate-track',
          kind: ProjectTimelineTrackKind.video,
          clipCount: 1,
        ),
      ]
      ..initialTimelineClips = {
        'duplicate-track': [clip],
      };
    final picker = _FakeProjectPicker()
      ..savePath = '/tmp/timeline-duplicate.orproj';
    await _mount(tester, gateway: gateway, picker: picker);
    await _createProject(tester, 'Duplicate clip');

    await tester.tap(
      find.byKey(const ValueKey('timeline-clip-duplicate-clip')),
    );
    await tester.pumpAndSettle();
    await tester.tap(
      find.byKey(const ValueKey('timeline-duplicate-duplicate-clip')),
    );
    await tester.pumpAndSettle();

    expect(gateway.insertTimelineClipCalls, 1);
    expect(gateway.lastInsertTrackId, 'duplicate-track');
    expect(
      _sameRational(
        gateway.lastInsertTimelineStart!,
        ProjectRationalTime(BigInt.from(8), 3),
      ),
      isTrue,
    );
    expect(
      _sameRational(gateway.lastInsertSourceStart!, clip.sourceStart!),
      isTrue,
    );
    expect(
      _sameRational(gateway.lastInsertDuration!, clip.timelineDuration),
      isTrue,
    );
    expect(gateway.lastSession!.clips['duplicate-track'], hasLength(2));
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    'timeline selection supports Ctrl+D and zoom stays presentation-only',
    (tester) async {
      _setViewport(tester, const Size(1280, 800));
      final clip = ProjectTimelineClip(
        clipId: 'keyboard-clip',
        mediaId: 'keyboard-media',
        timelineStart: ProjectRationalTime(BigInt.zero, 1),
        sourceStart: ProjectRationalTime(BigInt.zero, 1),
        timelineDuration: ProjectRationalTime(BigInt.from(2), 1),
      );
      final gateway = _FakeProjectGateway()
        ..initialTimelineTracks = const [
          ProjectTimelineTrack(
            trackId: 'keyboard-track',
            kind: ProjectTimelineTrackKind.video,
            clipCount: 1,
          ),
        ]
        ..initialTimelineClips = {
          'keyboard-track': [clip],
        };
      final picker = _FakeProjectPicker()
        ..savePath = '/tmp/timeline-keyboard.orproj';
      await _mount(tester, gateway: gateway, picker: picker);
      await _createProject(tester, 'Timeline keyboard');

      final revision = gateway.lastSession!.view.revision;
      await tester.tap(find.byKey(const ValueKey('timeline-zoom-in')));
      await tester.pumpAndSettle();
      expect(find.text('125%'), findsOneWidget);
      await tester.tap(find.byKey(const ValueKey('timeline-fit-zoom')));
      await tester.pumpAndSettle();
      expect(
        find.descendant(
          of: find.byKey(const ValueKey('timeline-zoom-label')),
          matching: find.text('Fit'),
        ),
        findsOneWidget,
      );
      expect(gateway.lastSession!.view.revision, revision);

      await tester.tap(
        find.byKey(const ValueKey('timeline-clip-keyboard-clip')),
      );
      await tester.pumpAndSettle();
      await tester.tap(find.text('Close').last);
      await tester.pumpAndSettle();
      expect(
        find.byKey(const ValueKey('timeline-selection-duplicate')),
        findsOneWidget,
      );
      final inspectorScrollable = find
          .descendant(
            of: find.byKey(const ValueKey('inspector-scroll')),
            matching: find.byType(Scrollable),
          )
          .first;
      expect(find.text('TRANSFORM'), findsOneWidget);
      await tester.enterText(
        find.byKey(const ValueKey('inspector-visual-x')),
        '250',
      );
      await tester.enterText(
        find.byKey(const ValueKey('inspector-visual-brightness')),
        '250',
      );
      await tester.enterText(
        find.byKey(const ValueKey('inspector-visual-contrast')),
        '1250',
      );
      await tester.scrollUntilVisible(
        find.byKey(const ValueKey('inspector-visual-transition_in_kind')),
        200,
        scrollable: inspectorScrollable,
      );
      await tester.tap(
        find.byKey(const ValueKey('inspector-visual-transition_in_kind')),
      );
      await tester.pumpAndSettle();
      await tester.tap(find.text('Cross dissolve').last);
      await tester.pumpAndSettle();
      await tester.enterText(
        find.byKey(const ValueKey('inspector-visual-transition_in_duration')),
        '1/2',
      );
      await tester.scrollUntilVisible(
        find.byKey(const ValueKey('inspector-visual-apply')),
        200,
        scrollable: inspectorScrollable,
      );
      await tester.tap(find.byKey(const ValueKey('inspector-visual-apply')));
      await tester.pumpAndSettle();
      expect(gateway.updateVisualSettingsCalls, 1);
      expect(gateway.lastVisualSettings?.xMilliCanvas, 250);
      expect(gateway.lastVisualSettings?.brightnessAmountMilli, 250);
      expect(gateway.lastVisualSettings?.contrastAmountMilli, 1250);
      expect(
        gateway.lastVisualSettings?.transitionIn,
        ProjectTimelineTransition.crossDissolve,
      );
      expect(
        gateway.lastVisualSettings?.transitionInDuration?.canonical,
        '1/2',
      );
      await tester.scrollUntilVisible(
        find.byKey(const ValueKey('inspector-visual-reset')),
        200,
        scrollable: inspectorScrollable,
      );
      await tester.tap(find.byKey(const ValueKey('inspector-visual-reset')));
      await tester.pumpAndSettle();
      expect(gateway.updateVisualSettingsCalls, 2);
      expect(gateway.lastVisualSettings?.xMilliCanvas, 0);
      expect(gateway.lastVisualSettings?.scaleXMilli, 1000);
      expect(gateway.lastVisualSettings?.opacityBasisPoints, 10000);
      expect(gateway.lastVisualSettings?.brightnessAmountMilli, 0);
      expect(
        gateway.lastVisualSettings?.transitionIn,
        ProjectTimelineTransition.none,
      );
      await tester.sendKeyDownEvent(LogicalKeyboardKey.controlLeft);
      await tester.sendKeyDownEvent(LogicalKeyboardKey.keyD);
      await tester.sendKeyUpEvent(LogicalKeyboardKey.keyD);
      await tester.sendKeyUpEvent(LogicalKeyboardKey.controlLeft);
      await tester.pumpAndSettle();
      expect(gateway.insertTimelineClipCalls, 1);
      expect(gateway.lastInsertTrackId, 'keyboard-track');

      await tester.sendKeyEvent(LogicalKeyboardKey.escape);
      await tester.pumpAndSettle();
      expect(
        find.byKey(const ValueKey('timeline-selection-duplicate')),
        findsNothing,
      );

      await tester.tap(
        find.byKey(const ValueKey('timeline-clip-keyboard-clip')),
      );
      await tester.pumpAndSettle();
      await tester.tap(find.text('Close').last);
      await tester.pumpAndSettle();
      await tester.sendKeyEvent(LogicalKeyboardKey.delete);
      await tester.pumpAndSettle();
      expect(find.text('Delete clip?'), findsOneWidget);
      await tester.tap(find.text('Cancel').last);
      await tester.pumpAndSettle();
      expect(gateway.deleteTimelineClipCalls, 0);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('media drag inserts on a compatible timeline track', (
    tester,
  ) async {
    _setViewport(tester, const Size(1280, 800));
    final gateway = _FakeProjectGateway()
      ..initialMedia = [
        ProjectMediaItem(
          mediaId: 'audio-only-media',
          sourceUri: Uri.file('/tmp/audio-only.wav').toString(),
          formatNames: const ['wav'],
          duration: '9 s',
          videoDetails: null,
          audioDetails: 'pcm',
          firstAudioDuration: ProjectRationalTime(BigInt.from(9), 1),
        ),
        ProjectMediaItem(
          mediaId: 'av-media',
          sourceUri: Uri.file('/tmp/av-media.mov').toString(),
          formatNames: const ['mov'],
          duration: '5 s',
          videoDetails: '1920×1080 h264',
          audioDetails: 'aac',
          firstVideoDuration: ProjectRationalTime(BigInt.from(3), 2),
          firstAudioDuration: ProjectRationalTime(BigInt.from(5), 2),
        ),
      ]
      ..initialTimelineTracks = const [
        ProjectTimelineTrack(
          trackId: 'drop-video',
          kind: ProjectTimelineTrackKind.video,
          clipCount: 0,
        ),
        ProjectTimelineTrack(
          trackId: 'drop-audio',
          kind: ProjectTimelineTrackKind.audio,
          clipCount: 0,
        ),
      ];
    final picker = _FakeProjectPicker()
      ..savePath = '/tmp/timeline-drag-insert.orproj';
    await _mount(tester, gateway: gateway, picker: picker);
    await _createProject(tester, 'Timeline drag insert');

    Future<void> dragFromHandle(Offset start, Offset target) async {
      final gesture = await tester.startGesture(start);
      await gesture.moveBy(const Offset(20, 0));
      await tester.pump();
      await gesture.moveTo(target);
      await gesture.up();
      await tester.pumpAndSettle();
    }

    final videoLane = find.byKey(
      const ValueKey('timeline-drop-target-drop-video'),
    );
    await tester.ensureVisible(videoLane);
    final invalidStart = tester.getCenter(
      find.byKey(const ValueKey('media-drag-handle-audio-only-media')),
    );
    final videoRect = tester.getRect(videoLane);
    final invalidTarget = Offset(videoRect.left + 64, videoRect.center.dy);
    await dragFromHandle(invalidStart, invalidTarget);
    expect(gateway.insertTimelineClipCalls, 0);

    final audioLane = find.byKey(
      const ValueKey('timeline-drop-target-drop-audio'),
    );
    await tester.ensureVisible(audioLane);
    final validStart = tester.getCenter(
      find.byKey(const ValueKey('media-drag-handle-audio-only-media')),
    );
    final audioRect = tester.getRect(audioLane);
    final validTarget = Offset(audioRect.left + 64, audioRect.center.dy);
    await dragFromHandle(validStart, validTarget);

    expect(gateway.insertTimelineClipCalls, 1);
    expect(gateway.lastInsertTrackId, 'drop-audio');
    expect(
      _sameRational(
        gateway.lastInsertTimelineStart!,
        ProjectRationalTime(BigInt.one, 1),
      ),
      isTrue,
    );
    expect(
      _sameRational(
        gateway.lastInsertDuration!,
        ProjectRationalTime(BigInt.from(9), 1),
      ),
      isTrue,
    );
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    'attached timeline event refreshes tracks and rejects stale insert',
    (tester) async {
      _setViewport(tester, const Size(1280, 800));
      final gateway = _FakeProjectGateway()
        ..initialMedia = [
          ProjectMediaItem(
            mediaId: 'event-media',
            sourceUri: Uri.file('/tmp/event.mp4').toString(),
            formatNames: const ['mp4'],
            duration: '2 s',
            videoDetails: '1920×1080 h264',
            audioDetails: null,
            firstVideoDuration: ProjectRationalTime(BigInt.from(2), 1),
          ),
        ];
      final picker = _FakeProjectPicker()
        ..savePath = '/tmp/event-timeline.orproj';
      await _mount(tester, gateway: gateway, picker: picker);
      await _createProject(tester, 'Event timeline');
      gateway.externalAddTimelineTrack(ProjectTimelineTrackKind.video);
      await tester.pumpAndSettle();
      expect(find.text('V1'), findsOneWidget);
      expect(find.text('Revision 1'), findsOneWidget);

      await tester.tap(
        find.byKey(const ValueKey('media-add-timeline-event-media')),
      );
      await tester.pumpAndSettle();
      gateway.externalRename('Changed by attached CLI');
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('timeline-confirm-insert')));
      await tester.pumpAndSettle();

      expect(gateway.insertTimelineClipCalls, 1);
      expect(gateway.lastSession!.clips['external-track-1'], isEmpty);
      expect(
        find.textContaining('this action was not retried'),
        findsOneWidget,
      );
      expect(find.text('Revision 2'), findsOneWidget);
    },
  );

  testWidgets(
    'timeline insert failure leaves canonical presentation unchanged',
    (tester) async {
      _setViewport(tester, const Size(1280, 800));
      final gateway = _FakeProjectGateway()
        ..initialMedia = [
          ProjectMediaItem(
            mediaId: 'failure-media',
            sourceUri: Uri.file('/tmp/failure.mp4').toString(),
            formatNames: const ['mp4'],
            duration: '2 s',
            videoDetails: '1920×1080 h264',
            audioDetails: null,
            firstVideoDuration: ProjectRationalTime(BigInt.from(2), 1),
          ),
        ];
      final picker = _FakeProjectPicker()
        ..savePath = '/tmp/failure-timeline.orproj';
      await _mount(tester, gateway: gateway, picker: picker);
      await _createProject(tester, 'Failed timeline insert');
      await tester.tap(find.byKey(const ValueKey('timeline-add-video-track')));
      await tester.pumpAndSettle();
      gateway.nextTimelineFailure = 'TIMELINE_OVERLAP';

      await tester.tap(
        find.byKey(const ValueKey('media-add-timeline-failure-media')),
      );
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('timeline-confirm-insert')));
      await tester.pumpAndSettle();
      expect(gateway.insertTimelineClipCalls, 1);
      expect(gateway.lastSession!.clips['track-1'], isEmpty);
      expect(gateway.lastSession!.view.revision, BigInt.one);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('rename marks dirty and explicit save keeps the revision', (
    tester,
  ) async {
    _setViewport(tester, const Size(1440, 900));
    final gateway = _FakeProjectGateway();
    final picker = _FakeProjectPicker()..savePath = '/tmp/save.orproj';
    await _mount(tester, gateway: gateway, picker: picker);
    await _createProject(tester, 'Before');

    await tester.tap(find.byKey(const ValueKey('workspace-rename')));
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byKey(const ValueKey('rename-project-name')),
      'After',
    );
    await tester.tap(find.byKey(const ValueKey('confirm-rename-project')));
    await tester.pumpAndSettle();
    expect(
      find.byKey(const ValueKey('workspace-project-name')),
      findsOneWidget,
    );
    expect(find.text('Unsaved changes'), findsOneWidget);
    expect(find.text('Revision 1'), findsOneWidget);
    final revisionBeforeSave = gateway.lastSession!.view.revision;

    await tester.tap(find.byKey(const ValueKey('workspace-save')));
    await tester.pumpAndSettle();
    expect(gateway.saveCalls, 1);
    expect(gateway.lastSession!.view.dirty, isFalse);
    expect(gateway.lastSession!.view.revision, revisionBeforeSave);
    expect(find.text('Saved'), findsOneWidget);
  });

  testWidgets('dirty projects periodically autosave to recovery', (
    tester,
  ) async {
    _setViewport(tester, const Size(1440, 900));
    final gateway = _FakeProjectGateway();
    final picker = _FakeProjectPicker()..savePath = '/tmp/autosave.orproj';
    await _mount(tester, gateway: gateway, picker: picker);
    await _createProject(tester, 'Autosave');

    await tester.pump(const Duration(seconds: 30));
    await tester.pumpAndSettle();
    expect(gateway.autosaveCalls, 0);

    await _renameActiveProject(tester, 'Autosaved edit');
    await tester.pump(const Duration(seconds: 30));
    await tester.pumpAndSettle();
    expect(gateway.autosaveCalls, 1);
    expect(find.text('Recovery saved'), findsOneWidget);
    expect(gateway.lastSession!.view.dirty, isTrue);
    expect(tester.takeException(), isNull);
  });

  testWidgets('export starts through the gateway and can be cancelled', (
    tester,
  ) async {
    _setViewport(tester, const Size(1440, 900));
    final gateway = _FakeProjectGateway();
    final picker = _FakeProjectPicker()
      ..savePath = '/tmp/export-project.orproj'
      ..exportPath = '/tmp/export-project.mkv';
    await _mount(tester, gateway: gateway, picker: picker);
    await _createProject(tester, 'Export project');

    await tester.tap(find.byKey(const ValueKey('export-project')));
    await tester.pump();
    await tester.pump();
    expect(picker.exportPathCalls, 1);
    expect(gateway.exportCalls, 1);
    expect(gateway.lastExportDestination, '/tmp/export-project.mkv');
    expect(find.byKey(const ValueKey('cancel-export')), findsOneWidget);

    await tester.tap(find.byKey(const ValueKey('open-command-palette')));
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byKey(const ValueKey('command-palette-query')),
      'Close Project',
    );
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('command-close-project')));
    await tester.pumpAndSettle();
    expect(gateway.closeCalls, 0);
    expect(
      find.text(
        'Wait for the export to finish or cancel it before closing the project.',
      ),
      findsOneWidget,
    );

    await tester.tap(find.byKey(const ValueKey('cancel-export')));
    await tester.pump();
    await tester.pump();
    expect(gateway.exportCancelCalls, 1);
    expect(find.text('Export cancelled'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    'completed export is published through the selected storage boundary',
    (tester) async {
      _setViewport(tester, const Size(1440, 900));
      final gateway = _FakeProjectGateway();
      final picker = _FakeProjectPicker()
        ..savePath = '/tmp/export-project.orproj'
        ..exportPath = '/tmp/export-project.mkv';
      await _mount(tester, gateway: gateway, picker: picker);
      await _createProject(tester, 'Export project');

      await tester.tap(find.byKey(const ValueKey('export-project')));
      await tester.pump();
      await tester.pump();
      gateway.completeExport();
      await tester.pump(const Duration(milliseconds: 450));
      await tester.pump();

      expect(picker.publishExportCalls, 1);
      expect(find.text('Export complete'), findsOneWidget);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('save failure leaves the project open and dirty', (tester) async {
    _setViewport(tester, const Size(1440, 900));
    final gateway = _FakeProjectGateway()
      ..nextSaveFailure = 'PROJECT_FILE_CHANGED';
    final picker = _FakeProjectPicker()..savePath = '/tmp/fail-save.orproj';
    await _mount(tester, gateway: gateway, picker: picker);
    await _createProject(tester, 'Dirty');
    await tester.tap(find.byKey(const ValueKey('workspace-rename')));
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byKey(const ValueKey('rename-project-name')),
      'Changed',
    );
    await tester.tap(find.byKey(const ValueKey('confirm-rename-project')));
    await tester.pumpAndSettle();

    await tester.tap(find.byKey(const ValueKey('workspace-save')));
    await tester.pumpAndSettle();
    expect(gateway.lastSession!.view.dirty, isTrue);
    expect(gateway.closeCalls, 0);
    expect(find.text('Unsaved changes'), findsOneWidget);
    expect(find.textContaining('save was blocked'), findsOneWidget);
  });

  testWidgets('ZZ probe create at 320x640', (tester) async {
    _setViewport(tester, const Size(320, 640));
    final gateway = _FakeProjectGateway();
    final picker = _FakeProjectPicker()..savePath = '/tmp/zz-ovf.orproj';
    await _mount(tester, gateway: gateway, picker: picker);
    FlutterError.onError = (details) {
      FlutterError.dumpErrorToConsole(details);
    };
    await tester.tap(find.byKey(const ValueKey('home-new-project')));
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byKey(const ValueKey('new-project-name')),
      'Ov',
    );
    await tester.tap(find.byKey(const ValueKey('confirm-new-project')));
    await tester.pumpAndSettle();
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    'compact SAF-style journey closes an open project from Projects',
    (tester) async {
      // The Android SAF journey ends by opening the Projects workspace and
      // tapping its close control at the emulator's 320x640 viewport. Neither
      // that sequence nor the compact layout was covered before.
      _setViewport(tester, const Size(320, 640));
      final gateway = _FakeProjectGateway();
      final picker = _FakeProjectPicker()
        ..savePath = '/tmp/compact-close.orproj';
      await _mount(tester, gateway: gateway, picker: picker);
      await _createProject(tester, 'Compact close');

      await tester.tap(find.byKey(const ValueKey('nav-projects')));
      await tester.pumpAndSettle();

      expect(
        find.byKey(const ValueKey('active-project-close')),
        findsOneWidget,
        reason: 'the Projects screen must offer Close for the open project',
      );

      // The action row sits below the fold at this height, so a real user must
      // scroll to it. The journey does the same.
      await tester.ensureVisible(
        find.byKey(const ValueKey('active-project-close')),
      );
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('active-project-close')));
      await tester.pumpAndSettle();
      expect(gateway.closeCalls, 1);
      expect(find.text('No recent projects yet'), findsOneWidget);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('close guard supports cancel, discard, and save', (tester) async {
    _setViewport(tester, const Size(1440, 900));
    final gateway = _FakeProjectGateway();
    final picker = _FakeProjectPicker()..savePath = '/tmp/close.orproj';
    await _mount(tester, gateway: gateway, picker: picker);
    await _createProject(tester, 'Close me');
    await _renameActiveProject(tester, 'Unsaved');

    await tester.tap(find.byKey(const ValueKey('workspace-close')));
    await tester.pumpAndSettle();
    expect(find.text('Save your changes before continuing?'), findsOneWidget);
    await tester.tap(find.text('Cancel'));
    await tester.pumpAndSettle();
    expect(gateway.closeCalls, 0);
    expect(
      find.byKey(const ValueKey('workspace-project-name')),
      findsOneWidget,
    );

    await tester.tap(find.byKey(const ValueKey('workspace-close')));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Discard'));
    await tester.pumpAndSettle();
    expect(gateway.closeCalls, 1);
    expect(gateway.lastCloseDiscard, isTrue);
    expect(gateway.lastCloseDiscardRecoveryCheckpoint, isTrue);
    expect(find.text('No recent projects yet'), findsOneWidget);

    await tester.tap(find.byKey(const ValueKey('home-new-project')));
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byKey(const ValueKey('new-project-name')),
      'Save to close',
    );
    await tester.tap(find.byKey(const ValueKey('confirm-new-project')));
    await tester.pumpAndSettle();
    await _renameActiveProject(tester, 'Save me');
    await tester.tap(find.byKey(const ValueKey('workspace-close')));
    await tester.pumpAndSettle();
    await tester.tap(find.widgetWithText(FilledButton, 'Save'));
    await tester.pumpAndSettle();
    expect(gateway.saveCalls, 1);
    expect(gateway.closeCalls, 2);
    expect(gateway.lastCloseDiscard, isFalse);
    expect(gateway.lastCloseDiscardRecoveryCheckpoint, isFalse);
    expect(find.text('No recent projects yet'), findsOneWidget);
  });

  testWidgets('clean close retries a failed SAF synchronization', (
    tester,
  ) async {
    _setViewport(tester, const Size(1440, 900));
    final gateway = _FakeProjectGateway();
    final picker = _FakeProjectPicker()..savePath = '/tmp/retry-sync.orproj';
    await _mount(tester, gateway: gateway, picker: picker);
    await _createProject(tester, 'Retry sync');
    await _renameActiveProject(tester, 'Saved locally');
    picker.syncFailureMessage =
        'The external project could not be synchronized.';

    await tester.tap(find.byKey(const ValueKey('workspace-save')));
    await tester.pumpAndSettle();
    expect(gateway.lastSession!.view.dirty, isFalse);
    expect(picker.syncCalls, 2);

    await tester.tap(find.byKey(const ValueKey('workspace-close')));
    await tester.pumpAndSettle();
    expect(gateway.closeCalls, 0);
    expect(picker.syncCalls, 3);

    picker.syncFailureMessage = null;
    await tester.tap(find.byKey(const ValueKey('workspace-close')));
    await tester.pumpAndSettle();
    expect(gateway.closeCalls, 1);
    expect(picker.syncCalls, 4);
  });

  testWidgets('revision conflict refreshes once and does not retry rename', (
    tester,
  ) async {
    _setViewport(tester, const Size(1440, 900));
    final gateway = _FakeProjectGateway()..nextRenameConflict = true;
    final picker = _FakeProjectPicker()..savePath = '/tmp/conflict.orproj';
    await _mount(tester, gateway: gateway, picker: picker);
    await _createProject(tester, 'Flutter name');

    await tester.tap(find.byKey(const ValueKey('workspace-rename')));
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byKey(const ValueKey('rename-project-name')),
      'Unsent rename',
    );
    await tester.tap(find.byKey(const ValueKey('confirm-rename-project')));
    await tester.pumpAndSettle();

    expect(gateway.renameCalls, 1);
    expect(gateway.summaryCalls, greaterThanOrEqualTo(2));
    expect(
      find.byKey(const ValueKey('workspace-project-name')),
      findsOneWidget,
    );
    expect(find.textContaining('this action was not retried'), findsOneWidget);
  });

  testWidgets(
    'dirty project switch can cancel or save before opening another',
    (tester) async {
      _setViewport(tester, const Size(1440, 900));
      final gateway = _FakeProjectGateway();
      final picker = _FakeProjectPicker()..savePath = '/tmp/switch.orproj';
      await _mount(tester, gateway: gateway, picker: picker);
      await _createProject(tester, 'Current project');
      await _renameActiveProject(tester, 'Current dirty project');
      picker.openPath = '/tmp/next.orproj';
      await tester.tap(find.byKey(const ValueKey('or-brand-home')));
      await tester.pumpAndSettle();

      await tester.tap(find.byKey(const ValueKey('home-open-project')));
      await tester.pumpAndSettle();
      expect(find.text('Save your changes before continuing?'), findsOneWidget);
      await tester.tap(find.text('Cancel'));
      await tester.pumpAndSettle();
      expect(gateway.openCalls, 0);
      expect(gateway.closeCalls, 0);
      expect(gateway.lastSession!.closed, isFalse);

      await tester.tap(find.byKey(const ValueKey('home-open-project')));
      await tester.pumpAndSettle();
      await tester.tap(find.widgetWithText(FilledButton, 'Save'));
      await tester.pumpAndSettle();
      expect(gateway.saveCalls, 1);
      expect(gateway.closeCalls, 1);
      expect(gateway.lastCloseDiscard, isFalse);
      expect(gateway.openCalls, 1);
      expect(
        tester
            .widget<Text>(find.byKey(const ValueKey('workspace-project-name')))
            .data,
        'Opened Project',
      );
    },
  );

  testWidgets('attached-session invalidations refresh from summary', (
    tester,
  ) async {
    _setViewport(tester, const Size(1440, 900));
    final gateway = _FakeProjectGateway();
    final picker = _FakeProjectPicker()..savePath = '/tmp/events.orproj';
    await _mount(tester, gateway: gateway, picker: picker);
    await _createProject(tester, 'Initial');
    final before = gateway.lastSession!.view.revision;
    gateway.externalRename('From attached CLI');
    await tester.pumpAndSettle();

    expect(gateway.lastSession!.view.revision, before + BigInt.one);
    expect(
      find.byKey(const ValueKey('workspace-project-name')),
      findsOneWidget,
    );
    expect(find.text('Revision 1'), findsOneWidget);
    expect(find.text('Unsaved changes'), findsOneWidget);
  });

  test('timeline rational input remains exact and rejects inexact forms', () {
    expect(ProjectRationalTime.tryParse('0/1')!.canonical, '0/1');
    expect(ProjectRationalTime.tryParse('1/2')!.canonical, '1/2');
    expect(ProjectRationalTime.tryParse('3003/1001')!.canonical, '3/1');
    expect(ProjectRationalTime.tryParse('2/4')!.canonical, '1/2');
    expect(ProjectRationalTime.tryParse('-2/4')!.canonical, '-1/2');
    expect(ProjectRationalTime.tryParse('0/11')!.canonical, '0/1');
    expect(ProjectRationalTime.tryParse('+5/2')!.canonical, '5/2');
    for (final invalid in [
      '1.5',
      '1e2/1',
      '00:00:01',
      '1/0',
      'abc',
      '1/',
      ' /1',
      '9223372036854775808/1',
      '1/4294967296',
    ]) {
      expect(ProjectRationalTime.tryParse(invalid), isNull, reason: invalid);
    }
  });

  testWidgets('audio inspector writes gain, pan, and exact fades', (
    tester,
  ) async {
    _setViewport(tester, const Size(1280, 800));
    final clip = ProjectTimelineClip(
      clipId: 'audio-inspector-clip',
      mediaId: 'audio-inspector-media',
      timelineStart: ProjectRationalTime(BigInt.zero, 1),
      sourceStart: ProjectRationalTime(BigInt.zero, 1),
      timelineDuration: ProjectRationalTime(BigInt.from(4), 1),
    );
    final gateway = _FakeProjectGateway()
      ..initialTimelineTracks = const [
        ProjectTimelineTrack(
          trackId: 'audio-inspector-track',
          kind: ProjectTimelineTrackKind.audio,
          clipCount: 1,
        ),
      ]
      ..initialTimelineClips = {
        'audio-inspector-track': [clip],
      };
    final picker = _FakeProjectPicker()
      ..savePath = '/tmp/audio-inspector.orproj';
    await _mount(tester, gateway: gateway, picker: picker);
    await _createProject(tester, 'Audio inspector');

    await tester.tap(
      find.byKey(const ValueKey('timeline-clip-audio-inspector-clip')),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.text('Close').last);
    await tester.pumpAndSettle();
    expect(find.text('AUDIO'), findsOneWidget);
    expect(gateway.getAudioSettingsCalls, 1);
    await tester.enterText(
      find.byKey(const ValueKey('inspector-audio-gain_db')),
      '-6.25',
    );
    await tester.enterText(
      find.byKey(const ValueKey('inspector-audio-pan_percent')),
      '-37.5',
    );
    await tester.enterText(
      find.byKey(const ValueKey('inspector-audio-fade_in')),
      '1/2',
    );
    await tester.enterText(
      find.byKey(const ValueKey('inspector-audio-fade_out')),
      '1/3',
    );
    await tester.ensureVisible(
      find.byKey(const ValueKey('inspector-audio-apply')),
    );
    await tester.tap(find.byKey(const ValueKey('inspector-audio-apply')));
    await tester.pumpAndSettle();

    expect(gateway.updateAudioSettingsCalls, 1);
    expect(gateway.lastAudioSettings?.gainMilliDecibels, -6250);
    expect(gateway.lastAudioSettings?.panBasisPoints, -3750);
    expect(gateway.lastAudioSettings?.fadeIn.canonical, '1/2');
    expect(gateway.lastAudioSettings?.fadeOut.canonical, '1/3');
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    'real project timeline adds tracks in canonical order and removes only empty tracks',
    (tester) async {
      _setViewport(tester, const Size(1280, 800));
      final gateway = _FakeProjectGateway();
      final picker = _FakeProjectPicker()
        ..savePath = '/tmp/timeline-tracks.orproj';
      await _mount(tester, gateway: gateway, picker: picker);
      await _createProject(tester, 'Timeline tracks');

      await tester.tap(find.byKey(const ValueKey('timeline-add-video-track')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('timeline-add-audio-track')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('timeline-add-video-track')));
      await tester.pumpAndSettle();

      expect(find.text('V1'), findsOneWidget);
      expect(find.text('A1'), findsOneWidget);
      expect(find.text('V2'), findsOneWidget);
      expect(gateway.lastSession!.tracks.map((track) => track.kind).toList(), [
        ProjectTimelineTrackKind.video,
        ProjectTimelineTrackKind.audio,
        ProjectTimelineTrackKind.video,
      ]);

      final middleTrack = gateway.lastSession!.tracks[1];
      await tester.tap(
        find.byKey(ValueKey('timeline-remove-track-${middleTrack.trackId}')),
      );
      await tester.pumpAndSettle();
      expect(gateway.lastSession!.tracks.map((track) => track.trackId), [
        'track-1',
        'track-3',
      ]);
    },
  );

  testWidgets('project switch clears the previous timeline read snapshot', (
    tester,
  ) async {
    _setViewport(tester, const Size(1440, 900));
    final gateway = _FakeProjectGateway();
    final picker = _FakeProjectPicker()
      ..savePath = '/tmp/timeline-first-project.orproj';
    await _mount(tester, gateway: gateway, picker: picker);
    await _createProject(tester, 'First timeline project');
    await tester.tap(find.byKey(const ValueKey('timeline-add-video-track')));
    await tester.pumpAndSettle();
    expect(find.text('V1'), findsOneWidget);

    await tester.tap(find.byKey(const ValueKey('workspace-close')));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Discard'));
    await tester.pumpAndSettle();
    picker.savePath = '/tmp/timeline-second-project.orproj';
    await _createProject(tester, 'Second timeline project');

    expect(find.text('No timeline tracks'), findsOneWidget);
    expect(find.text('V1'), findsNothing);
    expect(tester.takeException(), isNull);
  });

  testWidgets('pointer move resolves once on drop and snap can be disabled', (
    tester,
  ) async {
    _setViewport(tester, const Size(1280, 800));
    const trackId = 'pointer-video';
    final clip = ProjectTimelineClip(
      clipId: 'pointer-clip',
      mediaId: 'pointer-media',
      timelineStart: ProjectRationalTime(BigInt.from(2), 1),
      sourceStart: ProjectRationalTime(BigInt.zero, 1),
      timelineDuration: ProjectRationalTime(BigInt.from(4), 1),
    );
    final gateway = _FakeProjectGateway()
      ..initialTimelineTracks = const [
        ProjectTimelineTrack(
          trackId: trackId,
          kind: ProjectTimelineTrackKind.video,
          clipCount: 1,
        ),
      ]
      ..initialTimelineClips = {
        trackId: [clip],
      };
    final picker = _FakeProjectPicker()..savePath = '/tmp/pointer-move.orproj';
    await _mount(tester, gateway: gateway, picker: picker);
    await _createProject(tester, 'Pointer move');

    final initial = gateway.lastSession!.view;
    gateway.nextTimelineSnapResult = ProjectTimelineSnapResult(
      projectId: initial.projectId,
      projectInstanceId: initial.projectInstanceId,
      projectRevision: initial.revision,
      rawTargetTime: ProjectRationalTime(BigInt.from(5), 2),
      resolvedTargetTime: ProjectRationalTime(BigInt.from(3), 1),
      snapped: true,
      movingAnchor: ProjectTimelineSnapMovingAnchor.start,
      targetKind: ProjectTimelineSnapTargetKind.clipStart,
      targetTime: ProjectRationalTime(BigInt.from(3), 1),
      targetTrackId: trackId,
      targetClipId: 'other-clip',
    );
    expect(
      tester
          .widget<FilterChip>(
            find.byKey(const ValueKey('timeline-snap-toggle')),
          )
          .selected,
      isTrue,
    );

    await tester.drag(
      find.byKey(const ValueKey('timeline-clip-pointer-clip')),
      const Offset(32, 0),
    );
    await tester.pumpAndSettle();

    expect(gateway.resolveTimelineSnapCalls, 1);
    expect(gateway.lastSnapOperation, ProjectTimelineSnapOperation.move);
    expect(gateway.lastSnapTrackId, trackId);
    expect(
      _sameRational(
        gateway.lastSnapTargetTime!,
        ProjectRationalTime(BigInt.from(5), 2),
      ),
      isTrue,
    );
    expect(gateway.moveTimelineClipCalls, 1);
    expect(gateway.lastMoveTimelineStart!.canonical, '3/1');

    await tester.tap(find.byKey(const ValueKey('timeline-snap-toggle')));
    await tester.pump();
    expect(
      tester
          .widget<FilterChip>(
            find.byKey(const ValueKey('timeline-snap-toggle')),
          )
          .selected,
      isFalse,
    );
    await tester.drag(
      find.byKey(const ValueKey('timeline-clip-pointer-clip')),
      const Offset(32, 0),
    );
    await tester.pumpAndSettle();

    expect(gateway.resolveTimelineSnapCalls, 1);
    expect(gateway.moveTimelineClipCalls, 2);
    expect(gateway.lastMoveTimelineStart!.canonical, '7/2');
  });

  testWidgets('pointer deltas round once from the original exact position', (
    tester,
  ) async {
    _setViewport(tester, const Size(1280, 800));
    const trackId = 'quantized-video';
    final gateway = _FakeProjectGateway()
      ..initialTimelineTracks = const [
        ProjectTimelineTrack(
          trackId: trackId,
          kind: ProjectTimelineTrackKind.video,
          clipCount: 1,
        ),
      ]
      ..initialTimelineClips = {
        trackId: [
          ProjectTimelineClip(
            clipId: 'quantized-clip',
            mediaId: 'quantized-media',
            timelineStart: ProjectRationalTime(BigInt.from(2), 1),
            sourceStart: ProjectRationalTime(BigInt.zero, 1),
            timelineDuration: ProjectRationalTime(BigInt.from(4), 1),
          ),
        ],
      };
    await _mount(
      tester,
      gateway: gateway,
      picker: _FakeProjectPicker()..savePath = '/tmp/quantized.orproj',
    );
    await _createProject(tester, 'Quantized pointer');

    final gesture = await tester.startGesture(
      tester.getCenter(
        find.byKey(const ValueKey('timeline-clip-quantized-clip')),
      ),
    );
    await gesture.moveBy(const Offset(20, 0));
    await tester.pump();
    await gesture.moveBy(const Offset(1, 0));
    await tester.pump();
    await gesture.up();
    await tester.pumpAndSettle();

    expect(gateway.resolveTimelineSnapCalls, 1);
    expect(
      _sameRational(
        gateway.lastSnapTargetTime!,
        ProjectRationalTime(BigInt.from(2328), 1000),
      ),
      isTrue,
    );
    expect(
      _sameRational(
        gateway.lastMoveTimelineStart!,
        ProjectRationalTime(BigInt.from(2328), 1000),
      ),
      isTrue,
    );
  });

  testWidgets('trim handles have priority and send exact edge operations', (
    tester,
  ) async {
    _setViewport(tester, const Size(1280, 800));
    const trackId = 'trim-video';
    final gateway = _FakeProjectGateway()
      ..initialTimelineTracks = const [
        ProjectTimelineTrack(
          trackId: trackId,
          kind: ProjectTimelineTrackKind.video,
          clipCount: 1,
        ),
      ]
      ..initialTimelineClips = {
        trackId: [
          ProjectTimelineClip(
            clipId: 'trim-pointer-clip',
            mediaId: 'trim-pointer-media',
            timelineStart: ProjectRationalTime(BigInt.from(2), 1),
            sourceStart: ProjectRationalTime(BigInt.zero, 1),
            timelineDuration: ProjectRationalTime(BigInt.from(6), 1),
          ),
        ],
      };
    await _mount(
      tester,
      gateway: gateway,
      picker: _FakeProjectPicker()..savePath = '/tmp/trim-pointer.orproj',
    );
    await _createProject(tester, 'Pointer trim');

    await tester.drag(
      find.byKey(
        const ValueKey('timeline-trim-handle-start-trim-pointer-clip'),
      ),
      const Offset(32, 0),
    );
    await tester.pumpAndSettle();
    expect(gateway.lastSnapOperation, ProjectTimelineSnapOperation.trimStart);
    expect(gateway.lastTrimEdge, ProjectTimelineTrimEdge.start);
    expect(
      _sameRational(
        gateway.lastTrimTimelineTime!,
        ProjectRationalTime(BigInt.from(5), 2),
      ),
      isTrue,
    );
    expect(gateway.moveTimelineClipCalls, 0);

    await tester.drag(
      find.byKey(const ValueKey('timeline-trim-handle-end-trim-pointer-clip')),
      const Offset(32, 0),
    );
    await tester.pumpAndSettle();
    expect(gateway.lastSnapOperation, ProjectTimelineSnapOperation.trimEnd);
    expect(gateway.lastTrimEdge, ProjectTimelineTrimEdge.end);
    expect(
      _sameRational(
        gateway.lastTrimTimelineTime!,
        ProjectRationalTime(BigInt.from(17), 2),
      ),
      isTrue,
    );
    expect(gateway.trimTimelineClipCalls, 2);
  });

  testWidgets('move stays on same-kind lanes and rejects opposite-kind drops', (
    tester,
  ) async {
    _setViewport(tester, const Size(1280, 800));
    final gateway = _FakeProjectGateway()
      ..initialTimelineTracks = const [
        ProjectTimelineTrack(
          trackId: 'video-one',
          kind: ProjectTimelineTrackKind.video,
          clipCount: 1,
        ),
        ProjectTimelineTrack(
          trackId: 'video-two',
          kind: ProjectTimelineTrackKind.video,
          clipCount: 0,
        ),
        ProjectTimelineTrack(
          trackId: 'audio-one',
          kind: ProjectTimelineTrackKind.audio,
          clipCount: 0,
        ),
      ]
      ..initialTimelineClips = {
        'video-one': [
          ProjectTimelineClip(
            clipId: 'lane-clip',
            mediaId: 'lane-media',
            timelineStart: ProjectRationalTime(BigInt.from(2), 1),
            sourceStart: ProjectRationalTime(BigInt.zero, 1),
            timelineDuration: ProjectRationalTime(BigInt.from(4), 1),
          ),
        ],
        'video-two': [],
        'audio-one': [],
      };
    await _mount(
      tester,
      gateway: gateway,
      picker: _FakeProjectPicker()..savePath = '/tmp/lane-pointer.orproj',
    );
    await _createProject(tester, 'Pointer lanes');

    await tester.drag(
      find.byKey(const ValueKey('timeline-clip-lane-clip')),
      const Offset(0, 58),
    );
    await tester.pumpAndSettle();
    expect(gateway.moveTimelineClipCalls, 1);
    expect(gateway.lastMoveTrackId, 'video-two');

    await tester.drag(
      find.byKey(const ValueKey('timeline-clip-lane-clip')),
      const Offset(0, 58),
    );
    await tester.pumpAndSettle();
    expect(gateway.moveTimelineClipCalls, 1);
    expect(gateway.resolveTimelineSnapCalls, 1);
  });

  testWidgets('attached project changes invalidate an in-flight pointer snap', (
    tester,
  ) async {
    _setViewport(tester, const Size(1280, 800));
    const trackId = 'stale-pointer-video';
    final gateway = _FakeProjectGateway()
      ..initialTimelineTracks = const [
        ProjectTimelineTrack(
          trackId: trackId,
          kind: ProjectTimelineTrackKind.video,
          clipCount: 1,
        ),
      ]
      ..initialTimelineClips = {
        trackId: [
          ProjectTimelineClip(
            clipId: 'stale-pointer-clip',
            mediaId: 'stale-pointer-media',
            timelineStart: ProjectRationalTime(BigInt.from(2), 1),
            sourceStart: ProjectRationalTime(BigInt.zero, 1),
            timelineDuration: ProjectRationalTime(BigInt.from(4), 1),
          ),
        ],
      };
    await _mount(
      tester,
      gateway: gateway,
      picker: _FakeProjectPicker()..savePath = '/tmp/stale-pointer.orproj',
    );
    await _createProject(tester, 'Stale pointer');

    final snapCompleter = Completer<ProjectTimelineSnapResult>();
    gateway.nextTimelineSnapCompleter = snapCompleter;
    await tester.drag(
      find.byKey(const ValueKey('timeline-clip-stale-pointer-clip')),
      const Offset(32, 0),
    );
    await tester.pump();
    expect(gateway.resolveTimelineSnapCalls, 1);

    gateway.externalRename('Changed externally');
    await tester.pumpAndSettle();
    final raw = gateway.lastSnapTargetTime!;
    final session = gateway.lastSession!.view;
    snapCompleter.complete(
      ProjectTimelineSnapResult(
        projectId: session.projectId,
        projectInstanceId: session.projectInstanceId,
        projectRevision: BigInt.zero,
        rawTargetTime: raw,
        resolvedTargetTime: raw,
        snapped: false,
        movingAnchor: ProjectTimelineSnapMovingAnchor.start,
        targetKind: ProjectTimelineSnapTargetKind.none,
        targetTime: raw,
      ),
    );
    await tester.pumpAndSettle();

    expect(gateway.moveTimelineClipCalls, 0);
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    'project timeline inserts exact clip and shows proportional block',
    (tester) async {
      _setViewport(tester, const Size(1280, 800));
      final gateway = _FakeProjectGateway()
        ..initialMedia = [
          ProjectMediaItem(
            mediaId: 'media-video',
            sourceUri: Uri.file('/tmp/scene.mp4').toString(),
            formatNames: const ['mp4'],
            duration: '4 s',
            videoDetails: '1920×1080 h264',
            audioDetails: null,
            firstVideoDuration: ProjectRationalTime(BigInt.from(4), 1),
          ),
        ];
      final picker = _FakeProjectPicker()
        ..savePath = '/tmp/timeline-clip.orproj';
      await _mount(tester, gateway: gateway, picker: picker);
      await _createProject(tester, 'Timeline clip');
      await tester.tap(find.byKey(const ValueKey('timeline-add-video-track')));
      await tester.pumpAndSettle();

      await tester.tap(
        find.byKey(const ValueKey('media-add-timeline-media-video')),
      );
      await tester.pumpAndSettle();
      expect(find.text('Insert Clip'), findsOneWidget);
      expect(find.text('4/1'), findsOneWidget);
      await tester.enterText(
        find.byKey(const ValueKey('timeline-insert-start')),
        '2/1',
      );
      await tester.tap(find.byKey(const ValueKey('timeline-confirm-insert')));
      await tester.pumpAndSettle();

      expect(gateway.lastSession!.view.revision, BigInt.from(2));
      final clip = gateway.lastSession!.clips.values.single.single;
      expect(clip.timelineStart.canonical, '2/1');
      expect(clip.sourceStart!.canonical, '0/1');
      expect(clip.timelineDuration.canonical, '4/1');
      final tooltip = tester.widget<Tooltip>(
        find.byKey(ValueKey('timeline-clip-tooltip-${clip.clipId}')),
      );
      expect(tooltip.message, contains('Timeline start: 2/1'));
      expect(tooltip.message, contains('Source range: 0/1 + 4/1'));
      expect(
        find.byKey(ValueKey('timeline-clip-${clip.clipId}')),
        findsOneWidget,
      );
      expect(find.text('scene.mp4'), findsNWidgets(2));
      expect(find.byKey(const ValueKey('timeline-trim')), findsNothing);
      expect(find.byKey(const ValueKey('timeline-split')), findsNothing);
      expect(find.byKey(const ValueKey('timeline-ripple')), findsNothing);
    },
  );

  testWidgets(
    'insert dialog explains missing compatible tracks and unknown duration stays blank',
    (tester) async {
      _setViewport(tester, const Size(1280, 800));
      final gateway = _FakeProjectGateway()
        ..initialMedia = [_mediaFixture('media-unknown', '/tmp/unknown.mp4')];
      final picker = _FakeProjectPicker()
        ..savePath = '/tmp/timeline-empty.orproj';
      await _mount(tester, gateway: gateway, picker: picker);
      await _createProject(tester, 'Empty tracks');
      await tester.tap(
        find.byKey(const ValueKey('media-add-timeline-media-unknown')),
      );
      await tester.pumpAndSettle();
      expect(
        find.text('Add a compatible Video or Audio track first.'),
        findsOneWidget,
      );
      await tester.tap(find.text('Close').last);
      await tester.pumpAndSettle();

      await tester.tap(find.byKey(const ValueKey('timeline-empty-add-video')));
      await tester.pumpAndSettle();
      await tester.tap(
        find.byKey(const ValueKey('media-add-timeline-media-unknown')),
      );
      await tester.pumpAndSettle();
      expect(
        find.byKey(const ValueKey('timeline-insert-duration')),
        findsOneWidget,
      );
      expect(
        tester
            .widget<TextField>(
              find.byKey(const ValueKey('timeline-insert-duration')),
            )
            .controller!
            .text,
        isEmpty,
      );
      expect(
        tester
            .widget<FilledButton>(
              find.byKey(const ValueKey('timeline-confirm-insert')),
            )
            .onPressed,
        isNull,
      );
      await tester.enterText(
        find.byKey(const ValueKey('timeline-insert-duration')),
        '5/2',
      );
      await tester.pump();
      expect(
        tester
            .widget<FilledButton>(
              find.byKey(const ValueKey('timeline-confirm-insert')),
            )
            .onPressed,
        isNotNull,
      );
      await tester.tap(find.byKey(const ValueKey('timeline-confirm-insert')));
      await tester.pumpAndSettle();
      expect(
        gateway
            .lastSession!
            .clips
            .values
            .single
            .single
            .timelineDuration
            .canonical,
        '5/2',
      );
    },
  );

  testWidgets(
    'timeline insert chooses exact duration for the selected stream',
    (tester) async {
      _setViewport(tester, const Size(1280, 800));
      final gateway = _FakeProjectGateway()
        ..initialMedia = [
          ProjectMediaItem(
            mediaId: 'media-multistream',
            sourceUri: Uri.file('/tmp/multistream.mov').toString(),
            formatNames: const ['mov'],
            duration: '5 s',
            videoDetails: '1920×1080 h264',
            audioDetails: 'AAC',
            containerDuration: ProjectRationalTime(BigInt.from(5), 1),
            firstVideoDuration: ProjectRationalTime(BigInt.from(4), 1),
            firstAudioDuration: ProjectRationalTime(BigInt.from(3), 1),
          ),
        ];
      final picker = _FakeProjectPicker()
        ..savePath = '/tmp/multistream-timeline.orproj';
      await _mount(tester, gateway: gateway, picker: picker);
      await _createProject(tester, 'Multistream timeline');
      await tester.tap(find.byKey(const ValueKey('timeline-add-video-track')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('timeline-add-audio-track')));
      await tester.pumpAndSettle();
      await tester.tap(
        find.byKey(const ValueKey('media-add-timeline-media-multistream')),
      );
      await tester.pumpAndSettle();
      expect(
        tester
            .widget<TextField>(
              find.byKey(const ValueKey('timeline-insert-duration')),
            )
            .controller!
            .text,
        '4/1',
      );
      await tester.tap(find.byKey(const ValueKey('timeline-insert-track')));
      await tester.pumpAndSettle();
      await tester.tap(find.text('A1 · audio').last);
      await tester.pumpAndSettle();
      expect(
        tester
            .widget<TextField>(
              find.byKey(const ValueKey('timeline-insert-duration')),
            )
            .controller!
            .text,
        '3/1',
      );
      await tester.tap(find.byKey(const ValueKey('timeline-confirm-insert')));
      await tester.pumpAndSettle();
      expect(gateway.insertTimelineClipCalls, 1);
      expect(gateway.lastSession!.clips['track-1'], isEmpty);
      expect(
        gateway
            .lastSession!
            .clips['track-2']!
            .single
            .timelineDuration
            .canonical,
        '3/1',
      );
    },
  );

  testWidgets(
    'timeline clip move and delete use explicit dialogs and keep media',
    (tester) async {
      _setViewport(tester, const Size(1280, 800));
      final gateway = _FakeProjectGateway()
        ..initialMedia = [_mediaFixture('media-move', '/tmp/move.mp4')];
      final picker = _FakeProjectPicker()
        ..savePath = '/tmp/timeline-move.orproj';
      await _mount(tester, gateway: gateway, picker: picker);
      await _createProject(tester, 'Move timeline');
      await tester.tap(find.byKey(const ValueKey('timeline-add-video-track')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('timeline-add-video-track')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('timeline-add-audio-track')));
      await tester.pumpAndSettle();
      await tester.tap(
        find.byKey(const ValueKey('media-add-timeline-media-move')),
      );
      await tester.pumpAndSettle();
      await tester.enterText(
        find.byKey(const ValueKey('timeline-insert-duration')),
        '2/1',
      );
      await tester.pump();
      await tester.tap(find.byKey(const ValueKey('timeline-confirm-insert')));
      await tester.pumpAndSettle();
      final clip = gateway.lastSession!.clips['track-1']!.single;
      expect(
        tester
            .widget<IconButton>(
              find.byKey(const ValueKey('timeline-remove-track-track-1')),
            )
            .onPressed,
        isNull,
      );

      await tester.tap(find.byKey(ValueKey('timeline-clip-${clip.clipId}')));
      await tester.pumpAndSettle();
      expect(find.textContaining('Timeline start: 0/1'), findsOneWidget);
      await tester.tap(find.byKey(ValueKey('timeline-move-${clip.clipId}')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('timeline-move-track')));
      await tester.pumpAndSettle();
      expect(find.text('A1 · audio'), findsNothing);
      await tester.tap(find.text('V2').last);
      await tester.enterText(
        find.byKey(const ValueKey('timeline-move-start')),
        '3/1',
      );
      await tester.pump();
      await tester.tap(find.byKey(const ValueKey('timeline-confirm-move')));
      await tester.pumpAndSettle();
      expect(
        gateway.lastSession!.clips['track-2']!.single.timelineStart.canonical,
        '3/1',
      );

      await tester.tap(find.byKey(ValueKey('timeline-clip-${clip.clipId}')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(ValueKey('timeline-delete-${clip.clipId}')));
      await tester.pumpAndSettle();
      expect(find.text('Delete clip?'), findsOneWidget);
      await tester.tap(
        find.byKey(ValueKey('timeline-confirm-delete-${clip.clipId}')),
      );
      await tester.pumpAndSettle();
      expect(
        gateway.lastSession!.clips.values.every((clips) => clips.isEmpty),
        isTrue,
      );
      expect(gateway.lastSession!.media, hasLength(1));
    },
  );

  testWidgets(
    'timeline clip trim split and ripple delete use exact action dialogs',
    (tester) async {
      _setViewport(tester, const Size(1280, 800));
      final clip = ProjectTimelineClip(
        clipId: 'edit-clip',
        mediaId: 'missing-media',
        timelineStart: ProjectRationalTime(BigInt.from(2), 1),
        sourceStart: ProjectRationalTime(BigInt.zero, 1),
        timelineDuration: ProjectRationalTime(BigInt.from(6), 1),
      );
      final laterClip = ProjectTimelineClip(
        clipId: 'later-clip',
        mediaId: 'missing-media',
        timelineStart: ProjectRationalTime(BigInt.from(10), 1),
        sourceStart: ProjectRationalTime(BigInt.from(6), 1),
        timelineDuration: ProjectRationalTime(BigInt.one, 1),
      );
      final otherTrackClip = ProjectTimelineClip(
        clipId: 'other-track-clip',
        mediaId: 'missing-media',
        timelineStart: ProjectRationalTime(BigInt.from(1), 1),
        sourceStart: ProjectRationalTime(BigInt.zero, 1),
        timelineDuration: ProjectRationalTime(BigInt.one, 1),
      );
      final gateway = _FakeProjectGateway()
        ..initialTimelineTracks = const [
          ProjectTimelineTrack(
            trackId: 'edit-track',
            kind: ProjectTimelineTrackKind.video,
            clipCount: 2,
          ),
          ProjectTimelineTrack(
            trackId: 'other-track',
            kind: ProjectTimelineTrackKind.video,
            clipCount: 1,
          ),
        ]
        ..initialTimelineClips = {
          'edit-track': [clip, laterClip],
          'other-track': [otherTrackClip],
        };
      final picker = _FakeProjectPicker()
        ..savePath = '/tmp/timeline-advanced-dialogs.orproj';
      await _mount(tester, gateway: gateway, picker: picker);
      await _createProject(tester, 'Advanced timeline dialogs');

      await tester.tap(find.byKey(const ValueKey('timeline-clip-edit-clip')));
      await tester.pumpAndSettle();
      expect(find.text('Move'), findsOneWidget);
      expect(find.text('Trim'), findsOneWidget);
      expect(find.text('Split'), findsOneWidget);
      expect(find.text('Delete'), findsOneWidget);
      expect(find.text('Ripple Delete'), findsOneWidget);
      await tester.tap(find.byKey(const ValueKey('timeline-trim-edit-clip')));
      await tester.pumpAndSettle();
      expect(find.text('Current timing: 2/1 – 8/1'), findsOneWidget);
      expect(find.text('Edge'), findsOneWidget);
      expect(find.text('Timeline edge'), findsOneWidget);
      expect(find.byKey(const ValueKey('timeline-trim-source')), findsNothing);
      expect(
        find.byKey(const ValueKey('timeline-trim-duration')),
        findsNothing,
      );
      expect(
        tester
            .widget<TextField>(find.byKey(const ValueKey('timeline-trim-time')))
            .controller!
            .text,
        '2/1',
      );
      await tester.tap(find.byKey(const ValueKey('timeline-trim-edge')));
      await tester.pumpAndSettle();
      await tester.tap(find.text('End').last);
      await tester.pumpAndSettle();
      expect(
        tester
            .widget<TextField>(find.byKey(const ValueKey('timeline-trim-time')))
            .controller!
            .text,
        '8/1',
      );
      await tester.enterText(
        find.byKey(const ValueKey('timeline-trim-time')),
        '6/1',
      );
      await tester.pump();
      await tester.tap(find.byKey(const ValueKey('timeline-trim-edge')));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Start').last);
      await tester.pumpAndSettle();
      expect(
        tester
            .widget<TextField>(find.byKey(const ValueKey('timeline-trim-time')))
            .controller!
            .text,
        '6/1',
      );
      await tester.enterText(
        find.byKey(const ValueKey('timeline-trim-time')),
        '3/1',
      );
      await tester.tap(find.byKey(const ValueKey('timeline-confirm-trim')));
      await tester.pumpAndSettle();
      expect(gateway.trimTimelineClipCalls, 1);
      expect(
        gateway.lastSession!.clips['edit-track']!.first.timelineStart.canonical,
        '3/1',
      );
      expect(
        gateway.lastSession!.clips['edit-track']!.first.sourceStart!.canonical,
        '1/1',
      );
      expect(
        gateway
            .lastSession!
            .clips['edit-track']!
            .first
            .timelineDuration
            .canonical,
        '5/1',
      );

      await tester.tap(find.byKey(const ValueKey('timeline-clip-edit-clip')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('timeline-split-edit-clip')));
      await tester.pumpAndSettle();
      expect(find.text('Split at:'), findsOneWidget);
      expect(find.text('Current timing: 3/1 – 8/1'), findsOneWidget);
      expect(
        tester
            .widget<TextField>(
              find.byKey(const ValueKey('timeline-split-time')),
            )
            .controller!
            .text,
        isEmpty,
      );
      await tester.enterText(
        find.byKey(const ValueKey('timeline-split-time')),
        '5/1',
      );
      await tester.pump();
      expect(
        tester
            .widget<TextField>(
              find.byKey(const ValueKey('timeline-split-time')),
            )
            .controller!
            .text,
        '5/1',
      );
      expect(
        tester
            .widget<FilledButton>(
              find.byKey(const ValueKey('timeline-confirm-split')),
            )
            .onPressed,
        isNotNull,
      );
      await tester.tap(find.byKey(const ValueKey('timeline-confirm-split')));
      await tester.pumpAndSettle();
      expect(gateway.splitTimelineClipCalls, 1);
      expect(gateway.lastSession!.clips['edit-track'], hasLength(3));
      expect(gateway.lastSession!.clips['edit-track']![1].clipId, 'clip-1');
      expect(
        gateway.lastSession!.clips['edit-track']![1].timelineStart.canonical,
        '5/1',
      );

      await tester.tap(find.byKey(const ValueKey('timeline-clip-edit-clip')));
      await tester.pumpAndSettle();
      await tester.tap(
        find.byKey(const ValueKey('timeline-ripple-delete-edit-clip')),
      );
      await tester.pumpAndSettle();
      expect(find.text('Ripple delete clip?'), findsOneWidget);
      expect(
        find.text(
          'Delete this clip and shift later clips on this track left by its duration? Other tracks will not move.',
        ),
        findsOneWidget,
      );
      await tester.tap(
        find.byKey(const ValueKey('timeline-confirm-ripple-delete-edit-clip')),
      );
      await tester.pumpAndSettle();
      expect(gateway.rippleDeleteTimelineClipCalls, 1);
      expect(gateway.lastSession!.clips['edit-track']![0].clipId, 'clip-1');
      expect(
        gateway.lastSession!.clips['edit-track']![0].timelineStart.canonical,
        '3/1',
      );
      expect(
        gateway.lastSession!.clips['edit-track']![1].timelineStart.canonical,
        '8/1',
      );
      expect(
        gateway
            .lastSession!
            .clips['other-track']!
            .single
            .timelineStart
            .canonical,
        '1/1',
      );
    },
  );

  testWidgets('compact real timeline does not overflow', (tester) async {
    _setViewport(tester, const Size(390, 844));
    final clip = ProjectTimelineClip(
      clipId: 'compact-clip',
      mediaId: 'missing-media',
      timelineStart: ProjectRationalTime(BigInt.one, 2),
      sourceStart: ProjectRationalTime(BigInt.zero, 1),
      timelineDuration: ProjectRationalTime(BigInt.from(2), 1),
    );
    final gateway = _FakeProjectGateway()
      ..initialTimelineTracks = const [
        ProjectTimelineTrack(
          trackId: 'compact-track',
          kind: ProjectTimelineTrackKind.video,
          clipCount: 1,
        ),
      ]
      ..initialTimelineClips = {
        'compact-track': [clip],
      };
    final picker = _FakeProjectPicker()
      ..savePath = '/tmp/timeline-compact.orproj';
    await _mount(tester, gateway: gateway, picker: picker);
    await _createProject(tester, 'Compact timeline');
    expect(find.text('V1'), findsOneWidget);
    expect(
      find.byKey(const ValueKey('timeline-clip-compact-clip')),
      findsOneWidget,
    );
    await tester.tap(find.byKey(const ValueKey('mobile-editor-tool-media')));
    await tester.pumpAndSettle();
    expect(find.byKey(const ValueKey('project-media-panel')), findsOneWidget);
    expect(find.byKey(const ValueKey('media-import')), findsOneWidget);
    picker.mediaPath = '/tmp/compact-import.mkv';
    await tester.tap(find.byKey(const ValueKey('media-import')));
    await tester.pumpAndSettle();
    expect(gateway.importMediaCalls, 1);
    expect(find.text('compact-import.mkv'), findsOneWidget);
    expect(tester.takeException(), isNull);
    await tester.tap(find.byKey(const ValueKey('mobile-tool-sheet-close')));
    await tester.pumpAndSettle();
    expect(find.byTooltip('Close tool panel'), findsNothing);

    await tester.tap(find.byKey(const ValueKey('timeline-clip-compact-clip')));
    await tester.pumpAndSettle();
    expect(find.text('Clip'), findsOneWidget);
    await tester.tap(
      find.descendant(
        of: find.byType(AlertDialog).last,
        matching: find.text('Close'),
      ),
    );
    await tester.pumpAndSettle();
    await tester.tap(
      find.byKey(const ValueKey('mobile-editor-tool-inspector')),
    );
    await tester.pumpAndSettle();
    expect(find.byKey(const ValueKey('inspector-visual-x')), findsOneWidget);
    expect(gateway.getVisualSettingsCalls, 1);
    await tester.tap(find.byKey(const ValueKey('mobile-tool-sheet-close')));
    await tester.pumpAndSettle();
    expect(tester.takeException(), isNull);
  });

  testWidgets('candidate recovery offers recover discard and cancel', (
    tester,
  ) async {
    _setViewport(tester, const Size(1440, 900));
    final gateway = _FakeProjectGateway()
      ..recoveryInspection = _inspection(ProjectRecoveryKind.candidate);
    final picker = _FakeProjectPicker()..openPath = '/tmp/candidate.orproj';
    await _mount(tester, gateway: gateway, picker: picker);

    await tester.tap(find.byKey(const ValueKey('home-open-project')));
    await tester.pumpAndSettle();
    expect(find.text('Saved revision: 4'), findsOneWidget);
    expect(find.text('Recovery revision: 6'), findsOneWidget);
    expect(find.text('Recover'), findsOneWidget);
    expect(find.text('Discard'), findsOneWidget);
    await tester.tap(find.text('Cancel'));
    await tester.pumpAndSettle();
    expect(gateway.applyRecoveryCalls, 0);
    expect(gateway.discardRecoveryCalls, 0);
    expect(gateway.openCalls, 0);

    await tester.tap(find.byKey(const ValueKey('home-open-project')));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Discard'));
    await tester.pumpAndSettle();
    expect(gateway.discardRecoveryCalls, 1);
    expect(gateway.openCalls, 1);
    expect(
      find.byKey(const ValueKey('workspace-project-name')),
      findsOneWidget,
    );
  });

  testWidgets('compact recovery opens with a readable unavailable viewer', (
    tester,
  ) async {
    _setViewport(tester, const Size(320, 640));
    final gateway = _FakeProjectGateway()
      ..recoveryInspection = _inspection(ProjectRecoveryKind.candidate)
      ..preview = ProjectPreviewState(
        position: ProjectRationalTime(BigInt.zero, 1),
        contentEnd: ProjectRationalTime(BigInt.from(4), 1),
        playing: false,
        generation: BigInt.zero,
        frameSequence: BigInt.zero,
        width: 0,
        height: 0,
      );
    final picker = _FakeProjectPicker()
      ..openPath = '/tmp/compact-recovery.orproj'
      ..syncFailureMessage =
          'The project is saved on this device, but could not be synchronized.';
    await _mount(tester, gateway: gateway, picker: picker);

    await tester.tap(find.byKey(const ValueKey('home-open-project')));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Recover'));
    await tester.pumpAndSettle();

    expect(gateway.applyRecoveryCalls, 1);
    expect(gateway.openCalls, 1);
    expect(
      find.byKey(const ValueKey('workspace-project-name')),
      findsOneWidget,
    );
    expect(find.byKey(const ValueKey('editor-shell-preview')), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    'recovery conflict needs confirmed discard and never offers recover',
    (tester) async {
      _setViewport(tester, const Size(1440, 900));
      final gateway = _FakeProjectGateway()
        ..recoveryInspection = _inspection(ProjectRecoveryKind.conflict);
      final picker = _FakeProjectPicker()..openPath = '/tmp/conflicting.orproj';
      await _mount(tester, gateway: gateway, picker: picker);

      await tester.tap(find.byKey(const ValueKey('home-open-project')));
      await tester.pumpAndSettle();
      expect(find.text('Recovery checkpoint conflict'), findsOneWidget);
      expect(find.text('Recover'), findsNothing);
      await tester.tap(find.text('Discard checkpoint'));
      await tester.pumpAndSettle();
      expect(find.text('Discard recovery data?'), findsOneWidget);
      await tester.tap(find.text('Cancel'));
      await tester.pumpAndSettle();
      expect(gateway.discardRecoveryCalls, 0);
      expect(gateway.openCalls, 0);

      await tester.tap(find.byKey(const ValueKey('home-open-project')));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Discard checkpoint'));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Discard'));
      await tester.pumpAndSettle();
      expect(gateway.discardRecoveryCalls, 1);
      expect(gateway.openCalls, 1);
    },
  );

  testWidgets('invalid recovery requires explicit confirmation', (
    tester,
  ) async {
    _setViewport(tester, const Size(1440, 900));
    final gateway = _FakeProjectGateway()
      ..recoveryInspection = _inspection(ProjectRecoveryKind.invalid);
    final picker = _FakeProjectPicker()..openPath = '/tmp/invalid.orproj';
    await _mount(tester, gateway: gateway, picker: picker);

    await tester.tap(find.byKey(const ValueKey('home-open-project')));
    await tester.pumpAndSettle();
    expect(find.text('Invalid recovery checkpoint'), findsOneWidget);
    expect(
      find.textContaining('malformed or failed validation'),
      findsOneWidget,
    );
    await tester.tap(find.text('Discard checkpoint'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Discard'));
    await tester.pumpAndSettle();
    expect(gateway.discardRecoveryCalls, 1);
    expect(gateway.openCalls, 1);
  });

  testWidgets('stale recovery opens canonical project and can be discarded', (
    tester,
  ) async {
    _setViewport(tester, const Size(1440, 900));
    final gateway = _FakeProjectGateway()
      ..recoveryInspection = _inspection(ProjectRecoveryKind.stale);
    final picker = _FakeProjectPicker()..openPath = '/tmp/stale.orproj';
    await _mount(tester, gateway: gateway, picker: picker);

    await tester.tap(find.byKey(const ValueKey('home-open-project')));
    await tester.pumpAndSettle();
    expect(gateway.openCalls, 1);
    expect(gateway.applyRecoveryCalls, 0);
    expect(gateway.discardRecoveryCalls, 0);
    expect(
      find.textContaining('stale recovery checkpoint is present'),
      findsOneWidget,
    );
    await tester.tap(find.byKey(const ValueKey('discard-stale-recovery')));
    await tester.pumpAndSettle();
    expect(gateway.discardRecoveryCalls, 1);
    expect(find.byKey(const ValueKey('project-notice')), findsNothing);
  });

  testWidgets('unsupported platform lifecycle shows storage unavailable', (
    tester,
  ) async {
    _setViewport(tester, const Size(390, 844));
    final gateway = _FakeProjectGateway();
    final picker = _FakeProjectPicker(supported: false);
    await _mount(tester, gateway: gateway, picker: picker);

    await tester.tap(find.byKey(const ValueKey('home-new-project')));
    await tester.pumpAndSettle();
    expect(
      find.text('Project storage is not available on this platform.'),
      findsOneWidget,
    );
    await tester.tap(find.byKey(const ValueKey('home-open-project')));
    await tester.pumpAndSettle();
    expect(picker.saveCalls, 0);
    expect(picker.openCalls, 0);
    expect(gateway.createCalls, 0);
    expect(gateway.openCalls, 0);
  });

  testWidgets('keyboard shortcuts use the same live gateway operations', (
    tester,
  ) async {
    _setViewport(tester, const Size(1440, 900));
    final gateway = _FakeProjectGateway();
    final picker = _FakeProjectPicker()..savePath = '/tmp/keys.orproj';
    await _mount(tester, gateway: gateway, picker: picker);
    await _createProject(tester, 'Key project');
    await _renameActiveProject(tester, 'Renamed');

    await _sendShortcut(tester, LogicalKeyboardKey.keyZ);
    expect(gateway.lastSession!.view.name, 'Key project');
    await _sendShortcut(tester, LogicalKeyboardKey.keyZ, shift: true);
    expect(gateway.lastSession!.view.name, 'Renamed');
    await _sendShortcut(tester, LogicalKeyboardKey.keyS);
    expect(gateway.saveCalls, 1);
    expect(gateway.lastSession!.view.dirty, isFalse);
  });

  testWidgets('dirty application exit can cancel and clean exit closes host', (
    tester,
  ) async {
    _setViewport(tester, const Size(1440, 900));
    final gateway = _FakeProjectGateway();
    final picker = _FakeProjectPicker()..savePath = '/tmp/exit.orproj';
    await _mount(tester, gateway: gateway, picker: picker);
    await _createProject(tester, 'Exit project');
    await _renameActiveProject(tester, 'Exit dirty');

    final cancelledExit = WidgetsBinding.instance.handleRequestAppExit();
    await tester.pumpAndSettle();
    expect(find.text('Unsaved project changes'), findsOneWidget);
    await tester.tap(find.text('Cancel'));
    await tester.pumpAndSettle();
    expect(await cancelledExit, ui.AppExitResponse.cancel);
    expect(gateway.closeCalls, 0);

    await tester.tap(find.byKey(const ValueKey('workspace-save')));
    await tester.pumpAndSettle();
    expect(
      await WidgetsBinding.instance.handleRequestAppExit(),
      ui.AppExitResponse.exit,
    );
    expect(gateway.closeCalls, 1);
    expect(gateway.lastCloseDiscard, isFalse);
  });

  testWidgets('command palette exposes lifecycle operations while active', (
    tester,
  ) async {
    _setViewport(tester, const Size(1280, 800));
    final gateway = _FakeProjectGateway();
    final picker = _FakeProjectPicker()..savePath = '/tmp/palette.orproj';
    await _mount(tester, gateway: gateway, picker: picker);
    await _createProject(tester, 'Palette');

    await _sendShortcut(tester, LogicalKeyboardKey.keyK);
    await tester.enterText(
      find.byKey(const ValueKey('command-palette-query')),
      'Close Project',
    );
    await tester.pumpAndSettle();
    expect(find.byKey(const ValueKey('command-close-project')), findsOneWidget);
  });

  testWidgets('desktop Editor Shell Preview stays distinct without a project', (
    tester,
  ) async {
    _setViewport(tester, const Size(1440, 900));
    await _mount(tester);
    await tester.tap(find.byKey(const ValueKey('home-editor-preview')));
    await tester.pumpAndSettle();

    expect(find.text('Editor layout preview'), findsOneWidget);
    expect(find.text('Viewer'), findsOneWidget);
    expect(find.text('Timeline'), findsOneWidget);
    expect(find.text('Inspector'), findsOneWidget);
    expect(find.text('No media loaded'), findsOneWidget);
    expect(find.text('No selection'), findsOneWidget);
    expect(find.text('Timeline engine not implemented'), findsOneWidget);
    expect(find.textContaining('.mp4'), findsNothing);
    expect(find.byKey(const ValueKey('nav-editorPreview')), findsNothing);
  });

  testWidgets('compact Editor Shell Preview uses a tool sheet', (tester) async {
    _setViewport(tester, const Size(390, 844));
    await _mount(tester);
    await tester.ensureVisible(
      find.byKey(const ValueKey('home-editor-preview')),
    );
    await tester.tap(find.byKey(const ValueKey('home-editor-preview')));
    await tester.pumpAndSettle();

    expect(find.text('No media loaded'), findsOneWidget);
    expect(find.text('Timeline engine not implemented'), findsOneWidget);
    expect(find.byKey(const ValueKey('preview-play')), findsOneWidget);
    expect(
      find.byKey(const ValueKey('mobile-editor-tool-dock')),
      findsOneWidget,
    );
    expect(find.byKey(const ValueKey('editor-tool-media')), findsNothing);
    expect(
      find.byKey(const ValueKey('mobile-editor-tool-media')),
      findsOneWidget,
    );
    expect(tester.takeException(), isNull);

    await tester.tap(find.byKey(const ValueKey('mobile-editor-tool-media')));
    await tester.pumpAndSettle();
    expect(find.text('Unavailable in this Developer Preview'), findsOneWidget);
    await tester.tap(find.byTooltip('Close tool panel'));
    await tester.pumpAndSettle();
    expect(find.text('Timeline engine not implemented'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets('viewer uses exact scrub and explicit sequence rate controls', (
    tester,
  ) async {
    _setViewport(tester, const Size(1280, 800));
    final gateway = _FakeProjectGateway()
      ..preview = ProjectPreviewState(
        position: ProjectRationalTime(BigInt.zero, 1),
        contentEnd: ProjectRationalTime(BigInt.from(4), 1),
        playing: false,
        generation: BigInt.zero,
        frameSequence: BigInt.zero,
        width: 0,
        height: 0,
      );
    final picker = _FakeProjectPicker()..savePath = '/tmp/preview.orproj';
    await _mount(tester, gateway: gateway, picker: picker);
    await _createProject(tester, 'Preview');
    await tester.pumpAndSettle();

    expect(gateway.previewStateCalls, greaterThan(0));
    expect(gateway.preview.contentEnd?.secondsForDisplay, 4);
    expect(find.byKey(const ValueKey('preview-scrub-ruler')), findsOneWidget);
    expect(
      tester
          .widget<IconButton>(find.byKey(const ValueKey('preview-play')))
          .onPressed,
      isNull,
    );

    await tester.tap(find.byKey(const ValueKey('preview-frame-rate')));
    await tester.pumpAndSettle();
    await tester.tap(find.text('24 fps'));
    await tester.pumpAndSettle();
    expect(gateway.lastPreviewRate, const ProjectRationalRate(24, 1));
    expect(
      tester
          .widget<IconButton>(find.byKey(const ValueKey('preview-play')))
          .onPressed,
      isNotNull,
    );

    await tester.tap(find.byKey(const ValueKey('preview-play')));
    await tester.pumpAndSettle();
    expect(gateway.preview.playing, isTrue);
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.paused);
    await tester.pumpAndSettle();
    expect(gateway.preview.playing, isFalse);
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.hidden);
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
    await tester.pumpAndSettle();
    expect(gateway.preview.playing, isFalse);

    await tester.tap(find.byKey(const ValueKey('preview-next-frame')));
    await tester.pumpAndSettle();
    expect(gateway.previewStepCalls, 1);
    expect(gateway.lastPreviewStep, ProjectPreviewFrameStep.next);

    final previousSeekCalls = gateway.previewSeekCalls;
    await tester.drag(
      find.byKey(const ValueKey('preview-scrub-ruler')),
      const Offset(90, 0),
    );
    await tester.pumpAndSettle();
    expect(gateway.previewSeekCalls, greaterThan(previousSeekCalls));
    expect(gateway.lastPreviewSeek, isNotNull);
    expect(gateway.lastPreviewSeek!.numerator, greaterThan(BigInt.zero));
    expect(tester.takeException(), isNull);
  });

  testWidgets('empty preview keeps play and frame step unavailable', (
    tester,
  ) async {
    _setViewport(tester, const Size(1280, 800));
    final gateway = _FakeProjectGateway()
      ..preview = ProjectPreviewState(
        position: ProjectRationalTime(BigInt.zero, 1),
        sequenceFrameRate: const ProjectRationalRate(24, 1),
        playing: false,
        generation: BigInt.zero,
        frameSequence: BigInt.zero,
        width: 0,
        height: 0,
      );
    final picker = _FakeProjectPicker()..savePath = '/tmp/empty-preview.orproj';
    await _mount(tester, gateway: gateway, picker: picker);
    await _createProject(tester, 'Empty preview');
    await tester.pumpAndSettle();

    expect(find.byKey(const ValueKey('preview-scrub-ruler')), findsNothing);
    for (final key in const [
      ValueKey('preview-previous-frame'),
      ValueKey('preview-play'),
      ValueKey('preview-next-frame'),
    ]) {
      expect(tester.widget<IconButton>(find.byKey(key)).onPressed, isNull);
    }
    expect(find.text('No media loaded'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });
}

ProjectRecoveryInspection _inspection(ProjectRecoveryKind kind) =>
    ProjectRecoveryInspection(
      kind: kind,
      projectId: 'project-id',
      baseRevision: BigInt.from(4),
      recoveryRevision: BigInt.from(6),
      recoveryName: 'Recovered Demo',
      conflictReason: 'project_id_mismatch',
      message: 'internal fixture details are not shown',
    );

Future<void> _mount(
  WidgetTester tester, {
  _FakeProjectGateway? gateway,
  _FakeProjectPicker? picker,
}) async {
  await tester.pumpWidget(
    OrApp(
      gateway: _FakeCoreGateway(),
      projectGateway: gateway ?? _FakeProjectGateway(),
      projectFilePicker: picker ?? _FakeProjectPicker(),
    ),
  );
  await tester.pumpAndSettle();
}

Future<void> _createProject(WidgetTester tester, String name) async {
  await tester.tap(find.byKey(const ValueKey('home-new-project')));
  await tester.pumpAndSettle();
  await tester.enterText(find.byKey(const ValueKey('new-project-name')), name);
  await tester.tap(find.byKey(const ValueKey('confirm-new-project')));
  await tester.pumpAndSettle();
}

Future<void> _renameActiveProject(WidgetTester tester, String name) async {
  await tester.tap(find.byKey(const ValueKey('workspace-rename')));
  await tester.pumpAndSettle();
  await tester.enterText(
    find.byKey(const ValueKey('rename-project-name')),
    name,
  );
  await tester.tap(find.byKey(const ValueKey('confirm-rename-project')));
  await tester.pumpAndSettle();
}

Future<void> _sendShortcut(
  WidgetTester tester,
  LogicalKeyboardKey key, {
  bool shift = false,
}) async {
  await tester.sendKeyDownEvent(LogicalKeyboardKey.controlLeft);
  if (shift) await tester.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft);
  await tester.sendKeyDownEvent(key);
  await tester.sendKeyUpEvent(key);
  if (shift) await tester.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft);
  await tester.sendKeyUpEvent(LogicalKeyboardKey.controlLeft);
  await tester.pumpAndSettle();
}

void _setViewport(WidgetTester tester, Size size) {
  tester.view.physicalSize = size;
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.resetPhysicalSize);
  addTearDown(tester.view.resetDevicePixelRatio);
}

class _FakeCoreGateway implements CoreGateway {
  @override
  Future<AppInfo> appInfo() async => const AppInfo(
    name: 'Fixture Core App',
    version: '9.8.7-test',
    coreApiVersion: 4,
  );

  @override
  Future<HealthStatus> health() async =>
      const HealthStatus(status: 'healthy-from-gateway');

  @override
  Future<List<Capability>> capabilities() async => const [
    Capability(id: 'core.app_info', version: 2),
    Capability(id: 'core.health', version: 3),
    Capability(id: 'core.capabilities', version: 4),
  ];
}

class _FakeProjectPicker implements ProjectFilePicker {
  _FakeProjectPicker({this.supported = true});

  final bool supported;
  String? openPath;
  String? mediaPath;
  List<String>? mediaPaths;
  String? savePath;
  String? exportPath;
  String? captionPath;
  String? syncFailureMessage;
  int openCalls = 0;
  int mediaOpenCalls = 0;
  int saveCalls = 0;
  int exportPathCalls = 0;
  int captionOpenCalls = 0;
  int captionCleanupCalls = 0;
  int publishExportCalls = 0;
  int discardExportCalls = 0;
  int syncCalls = 0;

  @override
  bool get isSupported => supported;

  @override
  bool get supportsMediaImport => supported;

  @override
  bool get supportsExport => supported;

  @override
  Future<String?> openProjectPath() async {
    openCalls++;
    return openPath;
  }

  @override
  Future<List<String>> openMediaSources() async {
    mediaOpenCalls++;
    return mediaPaths ?? [?mediaPath];
  }

  @override
  Future<String?> saveProjectPath({required String suggestedName}) async {
    saveCalls++;
    return savePath;
  }

  @override
  Future<String?> saveExportPath({required String suggestedName}) async {
    exportPathCalls++;
    return exportPath;
  }

  @override
  Future<String?> openCaptionFile() async {
    captionOpenCalls++;
    return captionPath;
  }

  @override
  Future<void> cleanupCaptionFile(String path) async {
    captionCleanupCalls++;
  }

  @override
  Future<String?> saveCaptionPath({required String suggestedName}) async =>
      null;

  @override
  Future<void> publishCaptionPath(String path) async {}

  @override
  Future<void> discardCaptionPath(String path) async {}

  @override
  Future<void> publishExportPath(String path) async {
    publishExportCalls++;
  }

  @override
  Future<void> discardExportPath(String path) async {
    discardExportCalls++;
  }

  @override
  Future<void> cancelExportPublish(String path) async {}

  @override
  Future<ProjectFileSyncResult?> synchronizeProjectPath(String path) async {
    syncCalls++;
    final failure = syncFailureMessage;
    if (failure != null) throw ProjectSafStorageException(failure);
    return null;
  }
}

class _FakeProjectGateway implements ProjectGateway {
  final Map<String, _FakeSession> _sessionsByPath = {};
  final Map<String, ProjectReadModel> _savedByPath = {};
  _FakeSession? lastSession;
  ProjectRecoveryInspection recoveryInspection = _inspection(
    ProjectRecoveryKind.none,
  );
  ProjectPreviewState preview = ProjectPreviewState(
    position: ProjectRationalTime(BigInt.zero, 1),
    playing: false,
    generation: BigInt.zero,
    frameSequence: BigInt.zero,
    width: 0,
    height: 0,
  );
  int captionImportCalls = 0;
  String? importedCaptionPath;
  ProjectCaptionImportPreview captionImportPreview =
      ProjectCaptionImportPreview(
        formatName: 'SubRip',
        cueCount: BigInt.one,
        formattingLossCount: BigInt.zero,
        emptyCuesSkipped: BigInt.zero,
      );
  int previewSeekCalls = 0;
  int previewStateCalls = 0;
  int previewStepCalls = 0;
  ProjectRationalTime? lastPreviewSeek;
  ProjectPreviewFrameStep? lastPreviewStep;
  ProjectRationalRate? lastPreviewRate;
  List<ProjectMediaItem> initialMedia = [];
  List<ProjectTimelineTrack> initialTimelineTracks = [];
  Map<String, List<ProjectTimelineClip>> initialTimelineClips = {};
  List<ProjectTimelineMarker> initialTimelineMarkers = [];
  final Map<String, ProjectMediaArtifactRequest> artifactRequestResponses = {};
  final Map<String, ProjectMediaArtifact> artifactResponses = {};
  String? nextSaveFailure;
  bool nextRenameConflict = false;
  bool nextImportBackendUnavailable = false;
  bool nextImportSourceNotFound = false;
  Set<int> failedImportCalls = {};
  String? lastCreatePath;
  String? lastCreateName;
  String? lastOpenPath;
  int createCalls = 0;
  int openCalls = 0;
  int summaryCalls = 0;
  int renameCalls = 0;
  int mediaListCalls = 0;
  int timelineTracksCalls = 0;
  int timelineClipsCalls = 0;
  int getVisualSettingsCalls = 0;
  int updateVisualSettingsCalls = 0;
  ProjectTimelineVisualSettings? lastVisualSettings;
  int getAudioSettingsCalls = 0;
  int updateAudioSettingsCalls = 0;
  ProjectTimelineAudioSettings? lastAudioSettings;
  int timelineMarkersCalls = 0;
  bool revisionChangeOnNextTimelinePage = false;
  int resolveTimelineSnapCalls = 0;
  bool revisionChangeOnNextTimelineSnap = false;
  ProjectTimelineSnapResult? nextTimelineSnapResult;
  Completer<ProjectTimelineSnapResult>? nextTimelineSnapCompleter;
  ProjectRationalTime? lastSnapTargetTime;
  ProjectTimelineSnapOperation? lastSnapOperation;
  String? lastSnapClipId;
  String? lastSnapTrackId;
  int addTimelineTrackCalls = 0;
  int removeTimelineTrackCalls = 0;
  int setTimelineTrackStateCalls = 0;
  String? lastTrackStateId;
  ProjectTimelineTrackState? lastTrackState;
  int insertTimelineClipCalls = 0;
  String? lastInsertTrackId;
  ProjectRationalTime? lastInsertTimelineStart;
  ProjectRationalTime? lastInsertSourceStart;
  ProjectRationalTime? lastInsertDuration;
  int insertTimelineTextClipCalls = 0;
  int updateTimelineTextClipCalls = 0;
  String? lastTextTrackId;
  String? lastTextClipId;
  ProjectRationalTime? lastTextTimelineStart;
  ProjectRationalTime? lastTextTimelineDuration;
  ProjectTimelineTextContent? lastTextContent;
  int moveTimelineClipCalls = 0;
  String? lastMoveClipId;
  String? lastMoveTrackId;
  ProjectRationalTime? lastMoveTimelineStart;
  int deleteTimelineClipCalls = 0;
  int trimTimelineClipCalls = 0;
  String? lastTrimClipId;
  ProjectTimelineTrimEdge? lastTrimEdge;
  ProjectRationalTime? lastTrimTimelineTime;
  int splitTimelineClipCalls = 0;
  int rippleDeleteTimelineClipCalls = 0;
  int addTimelineMarkerCalls = 0;
  int moveTimelineMarkerCalls = 0;
  int renameTimelineMarkerCalls = 0;
  int deleteTimelineMarkerCalls = 0;
  ProjectRationalTime? lastMarkerTime;
  String? lastMarkerLabel;
  String? lastMarkerId;
  String? nextTimelineFailure;
  int thumbnailRequests = 0;
  int waveformRequests = 0;
  int mediaPreviewReads = 0;
  final List<int> mediaListOffsets = [];
  final List<int> timelineClipOffsets = [];
  final List<int> timelineMarkerOffsets = [];
  int importMediaCalls = 0;
  int relinkMediaCalls = 0;
  int removeMediaCalls = 0;
  String? lastImportSource;
  String? lastRelinkMediaId;
  String? lastRelinkSource;
  final List<String> importMediaSources = [];
  String? lastRemovedMediaId;
  int saveCalls = 0;
  int autosaveCalls = 0;
  int exportCalls = 0;
  int exportStatusCalls = 0;
  int exportCancelCalls = 0;
  String? lastExportDestination;
  ProjectExportJob? _exportJob;

  void completeExport() {
    final current = _exportJob!;
    _exportJob = ProjectExportJob(
      succeeded: true,
      errorCode: '',
      message: 'Export complete.',
      jobId: current.jobId,
      state: 'succeeded',
      progressCompleted: current.progressTotal,
      progressTotal: current.progressTotal,
    );
  }

  int closeCalls = 0;
  bool? lastCloseDiscard;
  bool? lastCloseDiscardRecoveryCheckpoint;
  int inspectCalls = 0;
  int applyRecoveryCalls = 0;
  int discardRecoveryCalls = 0;

  @override
  Future<ProjectSessionHandle> createProject(String path, String name) async {
    createCalls++;
    lastCreatePath = path;
    lastCreateName = name;
    final view = ProjectReadModel(
      projectId: 'project-id',
      projectInstanceId: 'instance-$createCalls',
      revision: BigInt.zero,
      name: name,
      dirty: false,
      descriptorPath: '/tmp/or-session-$createCalls.json',
    );
    final session = _FakeSession(path, view)
      ..media.addAll(initialMedia)
      ..tracks = List.of(initialTimelineTracks)
      ..markers.addAll(initialTimelineMarkers)
      ..clips.addAll({
        for (final entry in initialTimelineClips.entries)
          entry.key: List.of(entry.value),
      });
    _sessionsByPath[path] = session;
    _savedByPath[path] = view;
    lastSession = session;
    return session;
  }

  @override
  Future<ProjectSessionHandle> openProject(String path) async {
    openCalls++;
    lastOpenPath = path;
    final saved =
        _savedByPath[path] ??
        ProjectReadModel(
          projectId: 'project-id',
          projectInstanceId: 'instance-open-$openCalls',
          revision: BigInt.zero,
          name: 'Opened Project',
          dirty: false,
          descriptorPath: '/tmp/or-session-open-$openCalls.json',
        );
    final session =
        _FakeSession(
            path,
            ProjectReadModel(
              projectId: saved.projectId,
              projectInstanceId: 'instance-open-$openCalls',
              revision: saved.revision,
              name: saved.name,
              dirty: false,
              descriptorPath: '/tmp/or-session-open-$openCalls.json',
            ),
          )
          ..media.addAll(initialMedia)
          ..tracks = List.of(initialTimelineTracks)
          ..markers.addAll(initialTimelineMarkers)
          ..clips.addAll({
            for (final entry in initialTimelineClips.entries)
              entry.key: List.of(entry.value),
          });
    _sessionsByPath[path] = session;
    lastSession = session;
    return session;
  }

  @override
  Future<ProjectReadModel> summary(ProjectSessionHandle handle) async {
    summaryCalls++;
    return _session(handle).view;
  }

  @override
  Future<ProjectPreviewState> previewState(ProjectSessionHandle session) async {
    previewStateCalls++;
    return preview;
  }

  @override
  Future<ProjectPreviewState> previewSeek(
    ProjectSessionHandle session,
    ProjectRationalTime position,
  ) async {
    previewSeekCalls++;
    lastPreviewSeek = position;
    return _copyPreview(
      position: position,
      presentedTime: position,
      advanceGeneration: true,
      advanceFrame: true,
    );
  }

  @override
  Future<ProjectPreviewState> previewPlay(ProjectSessionHandle session) async =>
      _copyPreview(playing: true, advanceGeneration: true);

  @override
  Future<ProjectPreviewState> previewPause(
    ProjectSessionHandle session,
  ) async => _copyPreview(playing: false);

  @override
  Future<ProjectPreviewState> previewStep(
    ProjectSessionHandle session,
    ProjectPreviewFrameStep direction,
  ) async {
    previewStepCalls++;
    lastPreviewStep = direction;
    return _copyPreview(advanceGeneration: true, advanceFrame: true);
  }

  @override
  Future<ProjectPreviewState> previewTick(ProjectSessionHandle session) async =>
      preview;

  @override
  Future<ProjectActionResult> setTimelineSequenceFrameRate(
    ProjectSessionHandle session,
    ProjectReadModel current,
    ProjectRationalRate? sequenceFrameRate,
  ) async {
    lastPreviewRate = sequenceFrameRate;
    _copyPreview(
      sequenceFrameRate: sequenceFrameRate,
      updateFrameRate: true,
      advanceGeneration: true,
    );
    return ProjectActionResult(succeeded: true, view: current);
  }

  ProjectPreviewState _copyPreview({
    ProjectRationalTime? position,
    ProjectRationalTime? presentedTime,
    ProjectRationalRate? sequenceFrameRate,
    bool updateFrameRate = false,
    bool? playing,
    bool advanceGeneration = false,
    bool advanceFrame = false,
  }) {
    final current = preview;
    return preview = ProjectPreviewState(
      position: position ?? current.position,
      presentedTime: presentedTime ?? current.presentedTime,
      sequenceFrameRate: updateFrameRate
          ? sequenceFrameRate
          : current.sequenceFrameRate,
      contentEnd: current.contentEnd,
      playing: playing ?? current.playing,
      generation:
          current.generation + (advanceGeneration ? BigInt.one : BigInt.zero),
      frameSequence:
          current.frameSequence + (advanceFrame ? BigInt.one : BigInt.zero),
      width: current.width,
      height: current.height,
      errorCode: current.errorCode,
      errorMessage: current.errorMessage,
    );
  }

  @override
  Future<ProjectActionResult> rename(
    ProjectSessionHandle handle,
    ProjectReadModel current,
    String name,
  ) async {
    renameCalls++;
    final session = _session(handle);
    if (nextRenameConflict) {
      nextRenameConflict = false;
      session.view = _copyView(
        session.view,
        name: 'Changed in CLI',
        revision: session.view.revision + BigInt.one,
        dirty: true,
      );
      session.emit('project_changed');
      return const ProjectActionResult(
        succeeded: false,
        errorCode: 'REVISION_CONFLICT',
        message: 'revision changed',
      );
    }
    if (current.revision != session.view.revision) {
      return const ProjectActionResult(
        succeeded: false,
        errorCode: 'REVISION_CONFLICT',
        message: 'revision changed',
      );
    }
    if (name == session.view.name) {
      return ProjectActionResult(succeeded: true, view: session.view);
    }
    session.undo.add(session.view.name);
    session.redo.clear();
    session.view = _copyView(
      session.view,
      name: name,
      revision: session.view.revision + BigInt.one,
      dirty: true,
    );
    session.emit('project_changed');
    return ProjectActionResult(succeeded: true, view: session.view);
  }

  @override
  Future<ProjectActionResult> undo(
    ProjectSessionHandle handle,
    ProjectReadModel current,
  ) async {
    final session = _session(handle);
    if (current.revision != session.view.revision) {
      return const ProjectActionResult(
        succeeded: false,
        errorCode: 'REVISION_CONFLICT',
      );
    }
    if (session.undo.isEmpty) {
      return const ProjectActionResult(
        succeeded: false,
        errorCode: 'NOTHING_TO_UNDO',
      );
    }
    session.redo.add(session.view.name);
    final name = session.undo.removeLast();
    session.view = _copyView(
      session.view,
      name: name,
      revision: session.view.revision + BigInt.one,
      dirty: true,
    );
    session.emit('project_changed');
    return ProjectActionResult(succeeded: true, view: session.view);
  }

  @override
  Future<ProjectActionResult> redo(
    ProjectSessionHandle handle,
    ProjectReadModel current,
  ) async {
    final session = _session(handle);
    if (current.revision != session.view.revision) {
      return const ProjectActionResult(
        succeeded: false,
        errorCode: 'REVISION_CONFLICT',
      );
    }
    if (session.redo.isEmpty) {
      return const ProjectActionResult(
        succeeded: false,
        errorCode: 'NOTHING_TO_REDO',
      );
    }
    session.undo.add(session.view.name);
    final name = session.redo.removeLast();
    session.view = _copyView(
      session.view,
      name: name,
      revision: session.view.revision + BigInt.one,
      dirty: true,
    );
    session.emit('project_changed');
    return ProjectActionResult(succeeded: true, view: session.view);
  }

  @override
  Future<ProjectMediaPage> listMediaPage(
    ProjectSessionHandle handle, {
    required int offset,
    required int limit,
  }) async {
    mediaListCalls++;
    mediaListOffsets.add(offset);
    final session = _session(handle);
    final start = offset.clamp(0, session.media.length).toInt();
    final end = (start + limit).clamp(start, session.media.length).toInt();
    return ProjectMediaPage(
      projectId: session.view.projectId,
      projectInstanceId: session.view.projectInstanceId,
      projectRevision: session.view.revision,
      items: List.unmodifiable(session.media.sublist(start, end)),
      totalCount: session.media.length,
      offset: start,
      limit: limit,
      nextOffset: end < session.media.length ? end : null,
    );
  }

  @override
  Future<ProjectTimelineTracks> listTimelineTracks(
    ProjectSessionHandle handle,
  ) async {
    timelineTracksCalls++;
    final session = _session(handle);
    return ProjectTimelineTracks(
      projectId: session.view.projectId,
      projectInstanceId: session.view.projectInstanceId,
      projectRevision: session.view.revision,
      items: session.tracks,
    );
  }

  @override
  Future<ProjectTimelineClipPage> listTimelineClips(
    ProjectSessionHandle handle, {
    required String trackId,
    required int offset,
    required int limit,
  }) async {
    timelineClipsCalls++;
    timelineClipOffsets.add(offset);
    final session = _session(handle);
    if (offset > 0 && revisionChangeOnNextTimelinePage) {
      revisionChangeOnNextTimelinePage = false;
      _timelineChanged(session);
    }
    final clips = session.clips[trackId] ?? const <ProjectTimelineClip>[];
    final start = offset.clamp(0, clips.length).toInt();
    final end = (start + limit).clamp(start, clips.length).toInt();
    return ProjectTimelineClipPage(
      projectId: session.view.projectId,
      projectInstanceId: session.view.projectInstanceId,
      projectRevision: session.view.revision,
      trackId: trackId,
      items: clips.sublist(start, end),
      totalCount: clips.length,
      offset: offset,
      limit: limit,
      nextOffset: end < clips.length ? end : null,
    );
  }

  @override
  Future<ProjectTimelineVisualSettings> getTimelineClipVisualSettings(
    ProjectSessionHandle handle,
    ProjectReadModel current, {
    required String trackId,
    required String clipId,
  }) async {
    getVisualSettingsCalls++;
    final session = _session(handle);
    return session.visualSettings[clipId] ??
        ProjectTimelineVisualSettings.identity;
  }

  @override
  Future<ProjectActionResult> updateTimelineClipVisualSettings(
    ProjectSessionHandle handle,
    ProjectReadModel current, {
    required String trackId,
    required String clipId,
    required ProjectTimelineVisualSettings settings,
  }) async {
    updateVisualSettingsCalls++;
    lastVisualSettings = settings;
    final session = _session(handle);
    if (current.revision != session.view.revision) return _revisionConflict();
    final trackExists = session.tracks.any(
      (track) =>
          track.trackId == trackId &&
          track.kind == ProjectTimelineTrackKind.video,
    );
    final clipExists = (session.clips[trackId] ?? const <ProjectTimelineClip>[])
        .any((clip) => clip.clipId == clipId);
    if (!trackExists || !clipExists) {
      return _timelineOperationFailure('TIMELINE_CLIP_NOT_FOUND');
    }
    session.visualSettings[clipId] = settings;
    _timelineChanged(session);
    return ProjectActionResult(succeeded: true, view: session.view);
  }

  @override
  Future<ProjectTimelineAudioSettings> getTimelineClipAudioSettings(
    ProjectSessionHandle handle,
    ProjectReadModel current, {
    required String trackId,
    required String clipId,
  }) async {
    getAudioSettingsCalls++;
    final session = _session(handle);
    return session.audioSettings[clipId] ??
        ProjectTimelineAudioSettings.identity;
  }

  @override
  Future<ProjectActionResult> updateTimelineClipAudioSettings(
    ProjectSessionHandle handle,
    ProjectReadModel current, {
    required String trackId,
    required String clipId,
    required ProjectTimelineAudioSettings settings,
  }) async {
    updateAudioSettingsCalls++;
    lastAudioSettings = settings;
    final session = _session(handle);
    if (current.revision != session.view.revision) return _revisionConflict();
    final trackExists = session.tracks.any(
      (track) =>
          track.trackId == trackId &&
          track.kind == ProjectTimelineTrackKind.audio,
    );
    final clipExists = (session.clips[trackId] ?? const <ProjectTimelineClip>[])
        .any((clip) => clip.clipId == clipId);
    if (!trackExists || !clipExists) {
      return _timelineOperationFailure('TIMELINE_CLIP_NOT_FOUND');
    }
    session.audioSettings[clipId] = settings;
    _timelineChanged(session);
    return ProjectActionResult(succeeded: true, view: session.view);
  }

  @override
  Future<ProjectTimelineMarkerPage> listTimelineMarkers(
    ProjectSessionHandle handle, {
    required int offset,
    required int limit,
  }) async {
    timelineMarkersCalls++;
    timelineMarkerOffsets.add(offset);
    final session = _session(handle);
    final markers = session.markers;
    final start = offset.clamp(0, markers.length).toInt();
    final end = (start + limit).clamp(start, markers.length).toInt();
    return ProjectTimelineMarkerPage(
      projectId: session.view.projectId,
      projectInstanceId: session.view.projectInstanceId,
      projectRevision: session.view.revision,
      items: markers.sublist(start, end),
      totalCount: markers.length,
      offset: offset,
      limit: limit,
      nextOffset: end < markers.length ? end : null,
    );
  }

  @override
  Future<ProjectTimelineSnapResult> resolveTimelineSnap(
    ProjectSessionHandle handle,
    ProjectReadModel current, {
    required ProjectTimelineSnapOperation operation,
    required String clipId,
    String? targetTrackId,
    required ProjectRationalTime targetTime,
  }) async {
    resolveTimelineSnapCalls++;
    lastSnapOperation = operation;
    lastSnapClipId = clipId;
    lastSnapTrackId = targetTrackId;
    lastSnapTargetTime = targetTime;
    final session = _session(handle);
    if (revisionChangeOnNextTimelineSnap) {
      revisionChangeOnNextTimelineSnap = false;
      _timelineChanged(session);
    }
    if (current.revision != session.view.revision) {
      throw ProjectGatewayException('REVISION_CONFLICT', 'revision changed');
    }
    final completer = nextTimelineSnapCompleter;
    nextTimelineSnapCompleter = null;
    if (completer != null) return completer.future;
    final result = nextTimelineSnapResult;
    nextTimelineSnapResult = null;
    return result ??
        ProjectTimelineSnapResult(
          projectId: current.projectId,
          projectInstanceId: current.projectInstanceId,
          projectRevision: current.revision,
          rawTargetTime: targetTime,
          resolvedTargetTime: targetTime,
          snapped: false,
          movingAnchor: ProjectTimelineSnapMovingAnchor.none,
          targetKind: ProjectTimelineSnapTargetKind.none,
          targetTime: targetTime,
        );
  }

  @override
  Future<ProjectActionResult> addTimelineTrack(
    ProjectSessionHandle handle,
    ProjectReadModel current,
    ProjectTimelineTrackKind kind,
  ) async {
    addTimelineTrackCalls++;
    final session = _session(handle);
    final failure = _timelineFailure();
    if (failure != null) return failure;
    if (current.revision != session.view.revision) return _revisionConflict();
    final id = 'track-${session.nextTrackId++}';
    session.tracks.add(
      ProjectTimelineTrack(trackId: id, kind: kind, clipCount: 0),
    );
    session.clips[id] = [];
    _timelineChanged(session);
    return ProjectActionResult(succeeded: true, view: session.view);
  }

  @override
  Future<ProjectActionResult> importTimelineCaptions(
    ProjectSessionHandle handle,
    ProjectReadModel current,
    String path,
  ) async {
    captionImportCalls++;
    importedCaptionPath = path;
    return ProjectActionResult(
      succeeded: true,
      message: 'Imported captions from $path.',
      view: _session(handle).view,
    );
  }

  @override
  Future<ProjectCaptionImportPreview> previewTimelineCaptions(
    ProjectSessionHandle handle,
    String path,
  ) async => captionImportPreview;

  @override
  Future<ProjectActionResult> exportTimelineCaptions(
    ProjectSessionHandle handle,
    ProjectReadModel current, {
    required String path,
    required ProjectCaptionFileFormat format,
  }) async => ProjectActionResult(
    succeeded: true,
    message: 'Exported captions.',
    view: _session(handle).view,
  );

  @override
  Future<ProjectActionResult> removeTimelineTrack(
    ProjectSessionHandle handle,
    ProjectReadModel current,
    String trackId,
  ) async {
    removeTimelineTrackCalls++;
    final session = _session(handle);
    final failure = _timelineFailure();
    if (failure != null) return failure;
    if (current.revision != session.view.revision) return _revisionConflict();
    session.tracks.removeWhere((track) => track.trackId == trackId);
    session.clips.remove(trackId);
    _timelineChanged(session);
    return ProjectActionResult(succeeded: true, view: session.view);
  }

  @override
  Future<ProjectActionResult> setTimelineTrackState(
    ProjectSessionHandle handle,
    ProjectReadModel current, {
    required String trackId,
    required ProjectTimelineTrackState state,
  }) async {
    setTimelineTrackStateCalls++;
    lastTrackStateId = trackId;
    lastTrackState = state;
    final session = _session(handle);
    final failure = _timelineFailure();
    if (failure != null) return failure;
    if (current.revision != session.view.revision) return _revisionConflict();
    final index = session.tracks.indexWhere(
      (track) => track.trackId == trackId,
    );
    if (index < 0) return _timelineOperationFailure('TIMELINE_TRACK_NOT_FOUND');
    final track = session.tracks[index];
    session.tracks[index] = ProjectTimelineTrack(
      trackId: track.trackId,
      kind: track.kind,
      clipCount: track.clipCount,
      state: state,
    );
    _timelineChanged(session);
    return ProjectActionResult(succeeded: true, view: session.view);
  }

  @override
  Future<ProjectActionResult> insertTimelineClip(
    ProjectSessionHandle handle,
    ProjectReadModel current, {
    required String trackId,
    required String mediaId,
    required ProjectRationalTime timelineStart,
    required ProjectRationalTime sourceStart,
    required ProjectRationalTime duration,
  }) async {
    insertTimelineClipCalls++;
    lastInsertTrackId = trackId;
    lastInsertTimelineStart = timelineStart;
    lastInsertSourceStart = sourceStart;
    lastInsertDuration = duration;
    final session = _session(handle);
    final failure = _timelineFailure();
    if (failure != null) return failure;
    if (current.revision != session.view.revision) return _revisionConflict();
    final clipId = 'clip-${session.nextClipId++}';
    final clips = session.clips[trackId]!;
    clips.add(
      ProjectTimelineClip(
        clipId: clipId,
        mediaId: mediaId,
        timelineStart: timelineStart,
        sourceStart: sourceStart,
        timelineDuration: duration,
      ),
    );
    clips.sort((a, b) => _compareRational(a.timelineStart, b.timelineStart));
    session.tracks = [
      for (final track in session.tracks)
        if (track.trackId == trackId)
          ProjectTimelineTrack(
            trackId: track.trackId,
            kind: track.kind,
            clipCount: clips.length,
            state: track.state,
          )
        else
          track,
    ];
    _timelineChanged(session);
    return ProjectActionResult(succeeded: true, view: session.view);
  }

  @override
  Future<ProjectActionResult> insertTimelineTextClip(
    ProjectSessionHandle handle,
    ProjectReadModel current, {
    required String trackId,
    required ProjectRationalTime timelineStart,
    required ProjectRationalTime timelineDuration,
    required ProjectTimelineTextContent content,
  }) async {
    insertTimelineTextClipCalls++;
    lastTextTrackId = trackId;
    lastTextTimelineStart = timelineStart;
    lastTextTimelineDuration = timelineDuration;
    lastTextContent = content;
    final session = _session(handle);
    final failure = _timelineFailure();
    if (failure != null) return failure;
    if (current.revision != session.view.revision) return _revisionConflict();
    final track = session.tracks.where((item) => item.trackId == trackId);
    if (track.isEmpty ||
        (content.kind == ProjectTimelineClipContentKind.text &&
            track.first.kind != ProjectTimelineTrackKind.text) ||
        (content.kind == ProjectTimelineClipContentKind.caption &&
            track.first.kind != ProjectTimelineTrackKind.caption)) {
      return _timelineOperationFailure('TIMELINE_TRACK_NOT_FOUND');
    }
    final clipId = 'clip-${session.nextClipId++}';
    final clips = session.clips.putIfAbsent(trackId, () => []);
    clips.add(
      ProjectTimelineClip(
        clipId: clipId,
        contentKind: content.kind,
        text: content.text,
        formatting: content.formatting,
        timelineStart: timelineStart,
        timelineDuration: timelineDuration,
      ),
    );
    clips.sort((a, b) => _compareRational(a.timelineStart, b.timelineStart));
    session.tracks = [
      for (final item in session.tracks)
        if (item.trackId == trackId)
          ProjectTimelineTrack(
            trackId: item.trackId,
            kind: item.kind,
            clipCount: clips.length,
            state: item.state,
          )
        else
          item,
    ];
    _timelineChanged(session);
    return ProjectActionResult(succeeded: true, view: session.view);
  }

  @override
  Future<ProjectActionResult> updateTimelineTextClip(
    ProjectSessionHandle handle,
    ProjectReadModel current, {
    required String trackId,
    required String clipId,
    required ProjectRationalTime timelineDuration,
    required ProjectTimelineTextContent content,
  }) async {
    updateTimelineTextClipCalls++;
    lastTextTrackId = trackId;
    lastTextClipId = clipId;
    lastTextTimelineDuration = timelineDuration;
    lastTextContent = content;
    final session = _session(handle);
    final failure = _timelineFailure();
    if (failure != null) return failure;
    if (current.revision != session.view.revision) return _revisionConflict();
    final clips = session.clips[trackId];
    if (clips == null) {
      return _timelineOperationFailure('TIMELINE_CLIP_NOT_FOUND');
    }
    final index = clips.indexWhere((clip) => clip.clipId == clipId);
    if (index < 0) return _timelineOperationFailure('TIMELINE_CLIP_NOT_FOUND');
    final clip = clips[index];
    clips[index] = ProjectTimelineClip(
      clipId: clip.clipId,
      contentKind: content.kind,
      text: content.text,
      formatting: content.formatting,
      timelineStart: clip.timelineStart,
      timelineDuration: timelineDuration,
    );
    _timelineChanged(session);
    return ProjectActionResult(succeeded: true, view: session.view);
  }

  @override
  Future<ProjectActionResult> moveTimelineClip(
    ProjectSessionHandle handle,
    ProjectReadModel current, {
    required String clipId,
    required String trackId,
    required ProjectRationalTime timelineStart,
  }) async {
    moveTimelineClipCalls++;
    lastMoveClipId = clipId;
    lastMoveTrackId = trackId;
    lastMoveTimelineStart = timelineStart;
    final session = _session(handle);
    final failure = _timelineFailure();
    if (failure != null) return failure;
    if (current.revision != session.view.revision) return _revisionConflict();
    ProjectTimelineClip? clip;
    for (final clips in session.clips.values) {
      final index = clips.indexWhere((candidate) => candidate.clipId == clipId);
      if (index >= 0) {
        clip = clips.removeAt(index);
        break;
      }
    }
    if (clip == null) {
      return _timelineOperationFailure('TIMELINE_CLIP_NOT_FOUND');
    }
    final moved = ProjectTimelineClip(
      clipId: clip.clipId,
      mediaId: clip.mediaId,
      timelineStart: timelineStart,
      sourceStart: clip.sourceStart,
      timelineDuration: clip.timelineDuration,
    );
    final clips = session.clips[trackId]!;
    clips.add(moved);
    clips.sort((a, b) => _compareRational(a.timelineStart, b.timelineStart));
    session.tracks = [
      for (final track in session.tracks)
        ProjectTimelineTrack(
          trackId: track.trackId,
          kind: track.kind,
          clipCount: session.clips[track.trackId]!.length,
          state: track.state,
        ),
    ];
    _timelineChanged(session);
    return ProjectActionResult(succeeded: true, view: session.view);
  }

  @override
  Future<ProjectActionResult> deleteTimelineClip(
    ProjectSessionHandle handle,
    ProjectReadModel current,
    String clipId,
  ) async {
    deleteTimelineClipCalls++;
    final session = _session(handle);
    final failure = _timelineFailure();
    if (failure != null) return failure;
    if (current.revision != session.view.revision) return _revisionConflict();
    var removedTrackId = '';
    for (final entry in session.clips.entries) {
      final index = entry.value.indexWhere((clip) => clip.clipId == clipId);
      if (index >= 0) {
        entry.value.removeAt(index);
        removedTrackId = entry.key;
        break;
      }
    }
    session.tracks = [
      for (final track in session.tracks)
        if (track.trackId == removedTrackId)
          ProjectTimelineTrack(
            trackId: track.trackId,
            kind: track.kind,
            clipCount: session.clips[track.trackId]!.length,
            state: track.state,
          )
        else
          track,
    ];
    _timelineChanged(session);
    return ProjectActionResult(succeeded: true, view: session.view);
  }

  @override
  Future<ProjectActionResult> trimTimelineClip(
    ProjectSessionHandle handle,
    ProjectReadModel current, {
    required String clipId,
    required ProjectTimelineTrimEdge edge,
    required ProjectRationalTime timelineTime,
  }) async {
    trimTimelineClipCalls++;
    lastTrimClipId = clipId;
    lastTrimEdge = edge;
    lastTrimTimelineTime = timelineTime;
    final session = _session(handle);
    final failure = _timelineFailure();
    if (failure != null) return failure;
    if (current.revision != session.view.revision) return _revisionConflict();
    for (final entry in session.clips.entries) {
      final index = entry.value.indexWhere((clip) => clip.clipId == clipId);
      if (index < 0) continue;
      final clip = entry.value[index];
      final currentEdge = edge == ProjectTimelineTrimEdge.start
          ? clip.timelineStart
          : clip.timelineEnd;
      if (_sameRational(currentEdge, timelineTime)) {
        return ProjectActionResult(succeeded: true, view: session.view);
      }
      final duration = edge == ProjectTimelineTrimEdge.start
          ? _subtractRational(clip.timelineEnd, timelineTime)
          : _subtractRational(timelineTime, clip.timelineStart);
      final sourceStart = edge == ProjectTimelineTrimEdge.start
          ? clip.sourceStart!.add(
              _subtractRational(timelineTime, clip.timelineStart),
            )
          : clip.sourceStart;
      entry.value[index] = ProjectTimelineClip(
        clipId: clip.clipId,
        mediaId: clip.mediaId,
        timelineStart: edge == ProjectTimelineTrimEdge.start
            ? timelineTime
            : clip.timelineStart,
        sourceStart: sourceStart,
        timelineDuration: duration,
      );
      _timelineChanged(session);
      return ProjectActionResult(succeeded: true, view: session.view);
    }
    return _timelineOperationFailure('TIMELINE_CLIP_NOT_FOUND');
  }

  @override
  Future<ProjectActionResult> splitTimelineClip(
    ProjectSessionHandle handle,
    ProjectReadModel current, {
    required String clipId,
    required ProjectRationalTime timelineTime,
  }) async {
    splitTimelineClipCalls++;
    final session = _session(handle);
    final failure = _timelineFailure();
    if (failure != null) return failure;
    if (current.revision != session.view.revision) return _revisionConflict();
    for (final entry in session.clips.entries) {
      final index = entry.value.indexWhere((clip) => clip.clipId == clipId);
      if (index < 0) continue;
      final clip = entry.value[index];
      final leftDuration = _subtractRational(timelineTime, clip.timelineStart);
      final rightDuration = _subtractRational(clip.timelineEnd, timelineTime);
      final right = ProjectTimelineClip(
        clipId: 'clip-${session.nextClipId++}',
        mediaId: clip.mediaId,
        timelineStart: timelineTime,
        sourceStart: clip.sourceStart!.add(leftDuration),
        timelineDuration: rightDuration,
      );
      entry.value[index] = ProjectTimelineClip(
        clipId: clip.clipId,
        mediaId: clip.mediaId,
        timelineStart: clip.timelineStart,
        sourceStart: clip.sourceStart,
        timelineDuration: leftDuration,
      );
      entry.value.insert(index + 1, right);
      session.tracks = [
        for (final track in session.tracks)
          ProjectTimelineTrack(
            trackId: track.trackId,
            kind: track.kind,
            clipCount: session.clips[track.trackId]!.length,
            state: track.state,
          ),
      ];
      _timelineChanged(session);
      return ProjectActionResult(succeeded: true, view: session.view);
    }
    return _timelineOperationFailure('TIMELINE_CLIP_NOT_FOUND');
  }

  @override
  Future<ProjectActionResult> rippleDeleteTimelineClip(
    ProjectSessionHandle handle,
    ProjectReadModel current,
    String clipId,
  ) async {
    rippleDeleteTimelineClipCalls++;
    final session = _session(handle);
    final failure = _timelineFailure();
    if (failure != null) return failure;
    if (current.revision != session.view.revision) return _revisionConflict();
    for (final entry in session.clips.entries) {
      final index = entry.value.indexWhere((clip) => clip.clipId == clipId);
      if (index < 0) continue;
      final duration = entry.value[index].timelineDuration;
      entry.value.removeAt(index);
      for (var suffix = index; suffix < entry.value.length; suffix++) {
        final clip = entry.value[suffix];
        entry.value[suffix] = ProjectTimelineClip(
          clipId: clip.clipId,
          mediaId: clip.mediaId,
          timelineStart: _subtractRational(clip.timelineStart, duration),
          sourceStart: clip.sourceStart,
          timelineDuration: clip.timelineDuration,
        );
      }
      session.tracks = [
        for (final track in session.tracks)
          ProjectTimelineTrack(
            trackId: track.trackId,
            kind: track.kind,
            clipCount: session.clips[track.trackId]!.length,
            state: track.state,
          ),
      ];
      _timelineChanged(session);
      return ProjectActionResult(succeeded: true, view: session.view);
    }
    return _timelineOperationFailure('TIMELINE_CLIP_NOT_FOUND');
  }

  @override
  Future<ProjectActionResult> addTimelineMarker(
    ProjectSessionHandle handle,
    ProjectReadModel current, {
    required ProjectRationalTime timelineTime,
    required String label,
  }) async {
    addTimelineMarkerCalls++;
    lastMarkerTime = timelineTime;
    lastMarkerLabel = label;
    final session = _session(handle);
    final failure = _timelineFailure();
    if (failure != null) return failure;
    if (current.revision != session.view.revision) return _revisionConflict();
    session.markers.add(
      ProjectTimelineMarker(
        markerId: 'marker-${session.nextMarkerId++}',
        timelineTime: timelineTime,
        label: label,
      ),
    );
    session.markers.sort(
      (left, right) => _compareRational(left.timelineTime, right.timelineTime),
    );
    _timelineChanged(session);
    return ProjectActionResult(succeeded: true, view: session.view);
  }

  @override
  Future<ProjectActionResult> moveTimelineMarker(
    ProjectSessionHandle handle,
    ProjectReadModel current, {
    required String markerId,
    required ProjectRationalTime timelineTime,
  }) async {
    moveTimelineMarkerCalls++;
    lastMarkerId = markerId;
    lastMarkerTime = timelineTime;
    final session = _session(handle);
    final failure = _timelineFailure();
    if (failure != null) return failure;
    if (current.revision != session.view.revision) return _revisionConflict();
    final index = session.markers.indexWhere(
      (marker) => marker.markerId == markerId,
    );
    if (index < 0) {
      return _timelineOperationFailure('TIMELINE_MARKER_NOT_FOUND');
    }
    final marker = session.markers[index];
    session.markers[index] = ProjectTimelineMarker(
      markerId: marker.markerId,
      timelineTime: timelineTime,
      label: marker.label,
    );
    session.markers.sort(
      (left, right) => _compareRational(left.timelineTime, right.timelineTime),
    );
    _timelineChanged(session);
    return ProjectActionResult(succeeded: true, view: session.view);
  }

  @override
  Future<ProjectActionResult> renameTimelineMarker(
    ProjectSessionHandle handle,
    ProjectReadModel current, {
    required String markerId,
    required String label,
  }) async {
    renameTimelineMarkerCalls++;
    lastMarkerId = markerId;
    lastMarkerLabel = label;
    final session = _session(handle);
    final failure = _timelineFailure();
    if (failure != null) return failure;
    if (current.revision != session.view.revision) return _revisionConflict();
    final index = session.markers.indexWhere(
      (marker) => marker.markerId == markerId,
    );
    if (index < 0) {
      return _timelineOperationFailure('TIMELINE_MARKER_NOT_FOUND');
    }
    final marker = session.markers[index];
    session.markers[index] = ProjectTimelineMarker(
      markerId: marker.markerId,
      timelineTime: marker.timelineTime,
      label: label,
    );
    _timelineChanged(session);
    return ProjectActionResult(succeeded: true, view: session.view);
  }

  @override
  Future<ProjectActionResult> deleteTimelineMarker(
    ProjectSessionHandle handle,
    ProjectReadModel current,
    String markerId,
  ) async {
    deleteTimelineMarkerCalls++;
    lastMarkerId = markerId;
    final session = _session(handle);
    final failure = _timelineFailure();
    if (failure != null) return failure;
    if (current.revision != session.view.revision) return _revisionConflict();
    final index = session.markers.indexWhere(
      (marker) => marker.markerId == markerId,
    );
    if (index < 0) {
      return _timelineOperationFailure('TIMELINE_MARKER_NOT_FOUND');
    }
    session.markers.removeAt(index);
    _timelineChanged(session);
    return ProjectActionResult(succeeded: true, view: session.view);
  }

  ProjectActionResult? _timelineFailure() {
    final code = nextTimelineFailure;
    nextTimelineFailure = null;
    return code == null ? null : _timelineOperationFailure(code);
  }

  ProjectActionResult _timelineOperationFailure(String code) =>
      ProjectActionResult(succeeded: false, errorCode: code, message: code);

  ProjectActionResult _revisionConflict() => const ProjectActionResult(
    succeeded: false,
    errorCode: 'REVISION_CONFLICT',
    message: 'revision changed',
  );

  void _timelineChanged(_FakeSession session) {
    session.view = _copyView(
      session.view,
      revision: session.view.revision + BigInt.one,
      dirty: true,
    );
    session.emit('project_changed');
  }

  @override
  Future<ProjectMediaArtifactRequest> requestMediaThumbnail(
    ProjectSessionHandle handle,
    String mediaId,
  ) async {
    thumbnailRequests++;
    return artifactRequestResponses['$mediaId:thumbnail'] ??
        ProjectMediaArtifactRequest(
          mediaId: mediaId,
          kind: ProjectMediaArtifactKind.thumbnail,
          state: ProjectMediaArtifactRequestState.failed,
          errorCode: 'CACHE_UNAVAILABLE',
          message: 'Preview unavailable',
        );
  }

  @override
  Future<ProjectMediaArtifactRequest> requestMediaWaveform(
    ProjectSessionHandle handle,
    String mediaId,
  ) async {
    waveformRequests++;
    return artifactRequestResponses['$mediaId:waveform'] ??
        ProjectMediaArtifactRequest(
          mediaId: mediaId,
          kind: ProjectMediaArtifactKind.waveform,
          state: ProjectMediaArtifactRequestState.failed,
          errorCode: 'CACHE_UNAVAILABLE',
          message: 'Preview unavailable',
        );
  }

  @override
  Future<ProjectMediaArtifact?> readMediaArtifact(
    ProjectSessionHandle handle, {
    required ProjectMediaArtifactKind kind,
    required String cacheKey,
  }) async {
    mediaPreviewReads++;
    return artifactResponses[cacheKey];
  }

  @override
  Stream<ProjectMediaArtifactEvent> watchMediaArtifacts(
    ProjectSessionHandle handle,
  ) => _session(handle).mediaArtifactEvents.stream;

  @override
  Future<ProjectActionResult> importMedia(
    ProjectSessionHandle handle,
    ProjectReadModel current,
    String source,
  ) async {
    importMediaCalls++;
    lastImportSource = source;
    importMediaSources.add(source);
    if (nextImportBackendUnavailable ||
        failedImportCalls.contains(importMediaCalls)) {
      nextImportBackendUnavailable = false;
      return const ProjectActionResult(
        succeeded: false,
        errorCode: 'PROBE_BACKEND_UNAVAILABLE',
        message: 'probe unavailable',
      );
    }
    if (nextImportSourceNotFound) {
      nextImportSourceNotFound = false;
      return const ProjectActionResult(
        succeeded: false,
        errorCode: 'SOURCE_NOT_FOUND',
        message: 'media file was not found',
      );
    }
    final session = _session(handle);
    if (current.revision != session.view.revision) {
      return const ProjectActionResult(
        succeeded: false,
        errorCode: 'REVISION_CONFLICT',
        message: 'revision changed',
      );
    }
    final item = _mediaFixture('media-imported-$importMediaCalls', source);
    session.media.add(item);
    session.view = _copyView(
      session.view,
      revision: session.view.revision + BigInt.one,
      dirty: true,
    );
    session.emit('project_changed');
    return ProjectActionResult(succeeded: true, view: session.view);
  }

  @override
  Future<ProjectActionResult> relinkMedia(
    ProjectSessionHandle handle,
    ProjectReadModel current,
    String mediaId,
    String source,
  ) async {
    relinkMediaCalls++;
    lastRelinkMediaId = mediaId;
    lastRelinkSource = source;
    final session = _session(handle);
    if (current.revision != session.view.revision) {
      return const ProjectActionResult(
        succeeded: false,
        errorCode: 'REVISION_CONFLICT',
        message: 'revision changed',
      );
    }
    final index = session.media.indexWhere((item) => item.mediaId == mediaId);
    if (index < 0) {
      return const ProjectActionResult(
        succeeded: false,
        errorCode: 'MEDIA_NOT_FOUND',
        message: 'media not found',
      );
    }
    final previous = session.media[index];
    session.media[index] = ProjectMediaItem(
      mediaId: previous.mediaId,
      sourceUri: source,
      formatNames: previous.formatNames,
      duration: previous.duration,
      videoDetails: previous.videoDetails,
      audioDetails: previous.audioDetails,
      containerDuration: previous.containerDuration,
      firstVideoDuration: previous.firstVideoDuration,
      firstAudioDuration: previous.firstAudioDuration,
    );
    session.view = _copyView(
      session.view,
      revision: session.view.revision + BigInt.one,
      dirty: true,
    );
    session.emit('project_changed');
    return ProjectActionResult(succeeded: true, view: session.view);
  }

  @override
  Future<ProjectActionResult> removeMedia(
    ProjectSessionHandle handle,
    ProjectReadModel current,
    String mediaId,
  ) async {
    removeMediaCalls++;
    lastRemovedMediaId = mediaId;
    final session = _session(handle);
    if (current.revision != session.view.revision) {
      return const ProjectActionResult(
        succeeded: false,
        errorCode: 'REVISION_CONFLICT',
        message: 'revision changed',
      );
    }
    session.media.removeWhere((item) => item.mediaId == mediaId);
    session.view = _copyView(
      session.view,
      revision: session.view.revision + BigInt.one,
      dirty: true,
    );
    session.emit('project_changed');
    return ProjectActionResult(succeeded: true, view: session.view);
  }

  @override
  Future<ProjectActionResult> save(ProjectSessionHandle handle) async {
    saveCalls++;
    final session = _session(handle);
    final failure = nextSaveFailure;
    nextSaveFailure = null;
    if (failure != null) {
      return ProjectActionResult(
        succeeded: false,
        errorCode: failure,
        message: 'external file changed',
      );
    }
    session.view = _copyView(session.view, dirty: false);
    _savedByPath[session.path] = session.view;
    session.emit('project_saved');
    return ProjectActionResult(succeeded: true, view: session.view);
  }

  @override
  Future<ProjectActionResult> autosaveCheckpoint(
    ProjectSessionHandle handle,
  ) async {
    autosaveCalls++;
    return const ProjectActionResult(
      succeeded: true,
      message: 'Recovery checkpoint updated.',
    );
  }

  @override
  Future<ProjectExportJob> startExport(
    ProjectSessionHandle handle,
    ProjectReadModel current,
    String destination,
  ) async {
    exportCalls++;
    lastExportDestination = destination;
    _exportJob = ProjectExportJob(
      succeeded: true,
      errorCode: '',
      message: 'Export queued.',
      jobId: '11111111-1111-4111-8111-111111111111',
      state: 'running',
      progressCompleted: BigInt.zero,
      progressTotal: BigInt.from(30),
    );
    return _exportJob!;
  }

  @override
  Future<ProjectExportJob> exportStatus(
    ProjectSessionHandle handle,
    ProjectReadModel current,
    String jobId,
  ) async {
    exportStatusCalls++;
    return _exportJob ??
        const ProjectExportJob(
          succeeded: false,
          errorCode: 'EXPORT_JOB_NOT_FOUND',
          message: 'not found',
          jobId: '',
          state: '',
        );
  }

  @override
  Future<ProjectExportJob> cancelExport(
    ProjectSessionHandle handle,
    ProjectReadModel current,
    String jobId,
  ) async {
    exportCancelCalls++;
    _exportJob = ProjectExportJob(
      succeeded: true,
      errorCode: '',
      message: 'Export cancelled.',
      jobId: jobId,
      state: 'cancelled',
      progressCompleted: _exportJob?.progressCompleted,
      progressTotal: _exportJob?.progressTotal,
    );
    return _exportJob!;
  }

  @override
  Future<void> close(
    ProjectSessionHandle handle, {
    required bool discardUnsaved,
    bool discardRecoveryCheckpoint = false,
  }) async {
    final session = _session(handle);
    if (session.view.dirty && !discardUnsaved) {
      throw const ProjectGatewayException('UNSAVED_CHANGES', 'dirty');
    }
    closeCalls++;
    lastCloseDiscard = discardUnsaved;
    lastCloseDiscardRecoveryCheckpoint = discardRecoveryCheckpoint;
    session.closed = true;
    session.emit('session_closing');
  }

  @override
  Stream<ProjectHostEvent> watch(ProjectSessionHandle handle) =>
      _session(handle).events.stream;

  void emitMediaArtifact(
    ProjectSessionHandle handle,
    ProjectMediaArtifactEvent event,
  ) => _session(handle).mediaArtifactEvents.add(event);

  @override
  Future<ProjectRecoveryInspection> inspectRecovery(String path) async {
    inspectCalls++;
    return recoveryInspection;
  }

  @override
  Future<ProjectRecoveryActionResult> applyRecovery(String path) async {
    applyRecoveryCalls++;
    return const ProjectRecoveryActionResult(
      succeeded: true,
      changed: true,
      message: 'Recovery applied.',
    );
  }

  @override
  Future<ProjectRecoveryActionResult> discardRecovery(String path) async {
    discardRecoveryCalls++;
    return const ProjectRecoveryActionResult(
      succeeded: true,
      changed: true,
      message: 'Recovery checkpoint discarded.',
    );
  }

  void externalRename(String name) {
    final session = lastSession!;
    session.view = _copyView(
      session.view,
      name: name,
      revision: session.view.revision + BigInt.one,
      dirty: true,
    );
    session.emit('project_changed');
  }

  void externalAddTimelineTrack(ProjectTimelineTrackKind kind) {
    final session = lastSession!;
    final trackId = 'external-track-${session.tracks.length + 1}';
    session.tracks.add(
      ProjectTimelineTrack(trackId: trackId, kind: kind, clipCount: 0),
    );
    session.clips[trackId] = [];
    _timelineChanged(session);
  }

  _FakeSession _session(ProjectSessionHandle handle) {
    if (handle is _FakeSession) return handle;
    throw ArgumentError.value(handle);
  }
}

ProjectReadModel _copyView(
  ProjectReadModel view, {
  String? name,
  BigInt? revision,
  bool? dirty,
}) => ProjectReadModel(
  projectId: view.projectId,
  projectInstanceId: view.projectInstanceId,
  revision: revision ?? view.revision,
  name: name ?? view.name,
  dirty: dirty ?? view.dirty,
  descriptorPath: view.descriptorPath,
);

bool _sameRational(ProjectRationalTime left, ProjectRationalTime right) =>
    left.numerator * BigInt.from(right.denominator) ==
    right.numerator * BigInt.from(left.denominator);

ProjectRationalTime _subtractRational(
  ProjectRationalTime left,
  ProjectRationalTime right,
) {
  final numerator =
      left.numerator * BigInt.from(right.denominator) -
      right.numerator * BigInt.from(left.denominator);
  final denominator = left.denominator * right.denominator;
  return ProjectRationalTime.tryParse('$numerator/$denominator')!;
}

int _compareRational(ProjectRationalTime left, ProjectRationalTime right) =>
    (left.numerator * BigInt.from(right.denominator)).compareTo(
      right.numerator * BigInt.from(left.denominator),
    );

class _FakeSession implements ProjectSessionHandle {
  _FakeSession(this.path, this.view);

  final String path;
  ProjectReadModel view;
  final List<ProjectMediaItem> media = [];
  List<ProjectTimelineTrack> tracks = [];
  final List<ProjectTimelineMarker> markers = [];
  final Map<String, List<ProjectTimelineClip>> clips = {};
  final Map<String, ProjectTimelineVisualSettings> visualSettings = {};
  final Map<String, ProjectTimelineAudioSettings> audioSettings = {};
  int nextTrackId = 1;
  int nextClipId = 1;
  int nextMarkerId = 1;
  final List<String> undo = [];
  final List<String> redo = [];
  final StreamController<ProjectHostEvent> events =
      StreamController<ProjectHostEvent>.broadcast(sync: true);
  final StreamController<ProjectMediaArtifactEvent> mediaArtifactEvents =
      StreamController<ProjectMediaArtifactEvent>.broadcast(sync: true);
  int sequence = 0;
  bool closed = false;

  void emit(String kind) {
    events.add(
      ProjectHostEvent(
        sequence: BigInt.from(++sequence),
        kind: kind,
        projectId: view.projectId,
        projectInstanceId: view.projectInstanceId,
        revision: view.revision,
        dirty: view.dirty,
      ),
    );
  }
}

ProjectMediaItem _mediaFixture(String id, String path) => ProjectMediaItem(
  mediaId: id,
  sourceUri: Uri.file(path).toString(),
  formatNames: const ['mp4'],
  duration: '10 s',
  videoDetails: '1920×1080 h264',
  audioDetails: 'AAC',
);

ProjectMediaItem _audioMediaFixture(String id, String path) => ProjectMediaItem(
  mediaId: id,
  sourceUri: Uri.file(path).toString(),
  formatNames: const ['wav'],
  duration: '10 s',
  videoDetails: null,
  audioDetails: '48000 Hz 2 ch stereo AAC',
);

ProjectMediaItem _unsupportedMediaFixture(String id, String path) =>
    ProjectMediaItem(
      mediaId: id,
      sourceUri: Uri.file(path).toString(),
      formatNames: const ['dat'],
      duration: null,
      videoDetails: null,
      audioDetails: null,
    );

Future<Uint8List> _tinyPngBytes() async => base64Decode(
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=',
);
