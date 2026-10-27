import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:or_app_bridge/or_app_bridge.dart';
import 'package:or_app/design/or_theme.dart';
import 'package:or_app/motion/motion_library_controller.dart';
import 'package:or_app/motion/motion_library_models.dart';
import 'package:or_app/motion/motion_library_panel.dart';

final Uint8List _png = base64Decode(
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg==',
);

const _sceneJson = '''
{
  "format": "opencut-reinforced-motion-scene",
  "schema_version": 1,
  "scene_id": "7a2b3c4d-5e6f-4a1b-8c9d-0e1f2a3b4c5d",
  "name": "Composition",
  "duration": {"numerator": 5, "denominator": 1},
  "canvas": {"width": 1920, "height": 1080},
  "assets": [],
  "nodes": [],
  "animations": []
}
''';

MotionAssetEntryView _entry(String id, String name, String category) =>
    MotionAssetEntryView(
      assetId: id,
      name: name,
      category: category,
      categorySource: 'filename',
      kind: 'scene',
      tags: const [],
      canvasWidth: 1920,
      canvasHeight: 1080,
      durationMilliseconds: 4000,
      previewPath: '',
    );

MotionAssetView _asset(String id, String name, String category) =>
    MotionAssetView(
      ok: true,
      code: 'OK',
      message: '',
      assetId: id,
      name: name,
      category: category,
      categorySource: 'filename',
      kind: 'scene',
      tags: const [],
      canvasWidth: 1920,
      canvasHeight: 1080,
      durationMilliseconds: 4000,
      previewPath: '',
      documentJson: jsonEncode({
        'format': 'opencut-reinforced-motion-asset',
        'schema_version': 1,
        'asset_id': id,
        'name': name,
        'category': category,
        'category_source': 'filename',
        'kind': 'scene',
        'canvas': {'width': 1920, 'height': 1080},
        'duration': {'numerator': 4, 'denominator': 1},
        'parameters': [],
        'scene': {'assets': [], 'nodes': [], 'animations': []},
      }),
      parameters: [
        MotionAssetParameterView(
          id: 'title',
          label: 'Title',
          kind: 'text',
          nodeId: 'title',
          field: 'text',
          defaultJson: '"Hello"',
          min: null,
          max: null,
          choices: const [],
        ),
      ],
    );

class _FakeMotionGateway implements MotionLibraryGateway {
  final List<String> insertedCategories = [];
  int removes = 0;
  int appends = 0;
  String? lastMaterializedOutput;
  String? lastAppendedAsset;

  @override
  Future<MotionAssetLibraryView> library(String root) async =>
      MotionAssetLibraryView(
        ok: true,
        code: 'OK',
        message: '',
        root: root,
        entries: [
          _entry(
            '6f1c2f5e-3a4b-4c5d-8e9f-0a1b2c3d4e5f',
            'Mountain background',
            'background',
          ),
          _entry('0b9d2f6e-5c1a-4f3b-9d8e-7a6b5c4d3e2f', 'Car', 'vehicle'),
        ],
      );

  @override
  Future<MotionAssetView> load(String root, String assetId) async => _asset(
    assetId,
    assetId == '0b9d2f6e-5c1a-4f3b-9d8e-7a6b5c4d3e2f'
        ? 'Car'
        : 'Mountain background',
    assetId == '0b9d2f6e-5c1a-4f3b-9d8e-7a6b5c4d3e2f'
        ? 'vehicle'
        : 'background',
  );

  @override
  Future<MotionAssetView> save(
    String root,
    String documentJson,
    String? category,
  ) async {
    insertedCategories.add(category ?? '');
    final decoded = jsonDecode(documentJson) as Map<String, dynamic>;
    return _asset(
      decoded['asset_id'] as String,
      decoded['name'] as String,
      category ?? decoded['category'] as String,
    );
  }

  @override
  Future<MotionAssetMutationView> remove(String root, String assetId) async {
    removes += 1;
    return const MotionAssetMutationView(
      ok: true,
      code: 'OK',
      message: '',
      removed: true,
    );
  }

  @override
  Future<MotionAssetView> importImage({
    required String root,
    required String sourcePath,
    required String name,
    String? category,
    required List<String> tags,
    required int width,
    required int height,
    required int seconds,
  }) async =>
      _asset('5d0c3b2a-1f4e-4a6b-9c8d-7e6f5a4b3c2d', name, 'background');

  @override
  Future<MotionSceneView> assetScene({
    required String root,
    required String assetId,
    required String valuesJson,
    String? sceneId,
    String? name,
  }) async => MotionSceneView(
    ok: true,
    code: 'OK',
    message: '',
    sceneJson: _sceneJson,
    sceneId: '7a2b3c4d-5e6f-4a1b-8c9d-0e1f2a3b4c5d',
    name: 'Preview',
    width: 1920,
    height: 1080,
    durationMilliseconds: 4000,
    previewPng: _png,
  );

  @override
  Future<MotionSceneView> newScene({
    required String name,
    required int width,
    required int height,
    required int seconds,
  }) async => MotionSceneView(
    ok: true,
    code: 'OK',
    message: '',
    sceneJson: _sceneJson,
    sceneId: '7a2b3c4d-5e6f-4a1b-8c9d-0e1f2a3b4c5d',
    name: name,
    width: width,
    height: height,
    durationMilliseconds: seconds * 1000,
    previewPng: _png,
  );

  @override
  Future<MotionSceneView> append({
    required String root,
    required String sceneJson,
    required String assetId,
    required String valuesJson,
    required String prefix,
    required double offsetX,
    required double offsetY,
  }) async {
    lastAppendedAsset = assetId;
    appends += 1;
    return MotionSceneView(
      ok: true,
      code: 'OK',
      message: '',
      sceneJson: _sceneJson,
      sceneId: '7a2b3c4d-5e6f-4a1b-8c9d-0e1f2a3b4c5d',
      name: 'Composition',
      width: 1920,
      height: 1080,
      durationMilliseconds: 5000,
      previewPng: _png,
    );
  }

  @override
  Future<MotionScenePreviewView> preview({
    required String root,
    required String sceneJson,
    required int atMilliseconds,
  }) async => MotionScenePreviewView(
    ok: true,
    code: 'OK',
    message: '',
    png: _png,
    width: 1920,
    height: 1080,
  );

  @override
  Future<MotionMaterializationView> materializeScene({
    required String root,
    required String sceneJson,
    required String outputPath,
    required int frameRateNumerator,
    required int frameRateDenominator,
    required int width,
    required int height,
  }) async {
    lastMaterializedOutput = outputPath;
    return MotionMaterializationView(
      succeeded: true,
      code: 'OK',
      message: '',
      outputPath: outputPath,
      sceneId: '7a2b3c4d-5e6f-4a1b-8c9d-0e1f2a3b4c5d',
      sceneName: 'Composition',
      width: width,
      height: height,
      frameRateNumerator: frameRateNumerator,
      frameRateDenominator: frameRateDenominator,
      frameCount: BigInt.from(150),
      durationNumerator: 5,
      durationDenominator: 1,
      bytesWritten: BigInt.from(1024),
    );
  }

  @override
  Future<MotionMaterializationView> materializeAsset({
    required String root,
    required String assetId,
    required String valuesJson,
    required String outputPath,
    required int frameRateNumerator,
    required int frameRateDenominator,
    required int width,
    required int height,
  }) async {
    lastMaterializedOutput = outputPath;
    return MotionMaterializationView(
      succeeded: true,
      code: 'OK',
      message: '',
      outputPath: outputPath,
      sceneId: '7a2b3c4d-5e6f-4a1b-8c9d-0e1f2a3b4c5d',
      sceneName: 'Asset',
      width: width,
      height: height,
      frameRateNumerator: frameRateNumerator,
      frameRateDenominator: frameRateDenominator,
      frameCount: BigInt.from(150),
      durationNumerator: 5,
      durationDenominator: 1,
      bytesWritten: BigInt.from(1024),
    );
  }

  @override
  Future<MotionAssetView> saveScene({
    required String root,
    required String sceneJson,
    required String name,
    String? category,
  }) async => _asset(
    '1c8e3a7f-6d2b-4a4c-8e9f-0b1c2d3e4f5a',
    name,
    category ?? 'object',
  );
}

void main() {
  Future<(MotionLibraryController, _FakeMotionGateway, Directory)>
  buildController() async {
    final support = await Directory.systemTemp.createTemp('or-motion-ui-');
    final gateway = _FakeMotionGateway();
    final controller = MotionLibraryController(
      gateway: gateway,
      supportDirectory: () async => support,
    );
    addTearDown(() async {
      controller.dispose();
      await support.delete(recursive: true);
    });
    await controller.initialize();
    return (controller, gateway, support);
  }

  test(
    'the catalogue, values and composition are driven by the bridge',
    () async {
      final (controller, gateway, _) = await buildController();
      expect(controller.entries, hasLength(2));
      expect(controller.root, endsWith('motion-library'));

      await controller.select(controller.entries.first);
      expect(controller.selected?.name, 'Mountain background');
      expect(controller.values['title'], 'Hello');
      expect(controller.assetPreview, isNotNull);

      controller.setValue('title', 'Changed');
      await controller.refreshPreview();
      expect(controller.values['title'], 'Changed');

      await controller.addSelectedToScene(offsetX: 40, offsetY: 20);
      expect(gateway.lastAppendedAsset, controller.selected?.assetId);
      expect(controller.hasDraft, isTrue);
      expect(controller.draftPreview, isNotNull);

      final output = await controller.materializeDraft('/tmp/out.mkv');
      expect(output?.succeeded, isTrue);
      expect(gateway.lastMaterializedOutput, '/tmp/out.mkv');

      await controller.updateSelectedCategory('nature');
      expect(gateway.insertedCategories.last, 'nature');
    },
  );

  test(
    'a dragged card materialises the values captured at drag start',
    () async {
      final (controller, gateway, _) = await buildController();
      await controller.materializeDrag(
        const MotionAssetDragData(
          assetId: '6f1c2f5e-3a4b-4c5d-8e9f-0a1b2c3d4e5f',
          name: 'Mountain background',
          valuesJson: '{"title":"Dragged"}',
        ),
        '/tmp/drag.mkv',
      );
      expect(gateway.lastMaterializedOutput, '/tmp/drag.mkv');
    },
  );

  testWidgets('the panel browses, edits, composes and inserts', (tester) async {
    tester.view.physicalSize = const Size(900, 2400);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    late MotionLibraryController controller;
    late _FakeMotionGateway gateway;
    late Directory support;
    // testWidgets runs in a fake-async zone; the temporary support directory is
    // real filesystem work and belongs in runAsync.
    await tester.runAsync(() async {
      (controller, gateway, support) = await buildController();
    });
    var inserted = 0;
    await tester.pumpWidget(
      MaterialApp(
        theme: OrTheme.dark,
        home: Scaffold(
          body: SizedBox(
            width: 340,
            height: 2000,
            child: MotionLibraryPanel(
              controller: controller,
              outputDirectory: support.path,
              onInsertClip: (path) async {
                inserted += 1;
                return true;
              },
              onNotice: (_) {},
              onOpenCodeMotion: () {},
            ),
          ),
        ),
      ),
    );
    await tester.pump();
    expect(find.text('Mountain background'), findsOneWidget);

    await tester.tap(
      find.byKey(
        const ValueKey('motion-asset-6f1c2f5e-3a4b-4c5d-8e9f-0a1b2c3d4e5f'),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.byKey(const ValueKey('motion-param-title')), findsNothing);
    expect(
      find.byKey(
        const ValueKey(
          'motion-param-6f1c2f5e-3a4b-4c5d-8e9f-0a1b2c3d4e5f-title',
        ),
      ),
      findsOneWidget,
    );

    await tester.tap(find.byKey(const ValueKey('motion-add-to-scene')));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 500));
    expect(controller.hasDraft, isTrue);

    await tester.tap(find.byKey(const ValueKey('motion-insert-draft')));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 800));
    expect(inserted, 1);
    expect(gateway.lastMaterializedOutput, isNotNull);
  });

  testWidgets('a dragged card adds to the composition preview', (tester) async {
    tester.view.physicalSize = const Size(900, 2400);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    late MotionLibraryController controller;
    late _FakeMotionGateway gateway;
    late Directory support;
    await tester.runAsync(() async {
      (controller, gateway, support) = await buildController();
    });
    await controller.select(controller.entries.first);
    await controller.addSelectedToScene();
    final before = gateway.appends;

    await tester.pumpWidget(
      MaterialApp(
        theme: OrTheme.dark,
        home: Scaffold(
          body: SizedBox(
            width: 340,
            height: 2000,
            child: MotionLibraryPanel(
              controller: controller,
              outputDirectory: support.path,
              onInsertClip: (_) async => true,
              onNotice: (_) {},
              onOpenCodeMotion: () {},
            ),
          ),
        ),
      ),
    );
    await tester.pump();

    final start = tester.getCenter(
      find.byKey(
        const ValueKey('motion-asset-6f1c2f5e-3a4b-4c5d-8e9f-0a1b2c3d4e5f'),
      ),
    );
    final end = tester.getCenter(
      find.byKey(const ValueKey('motion-scene-drop')),
    );
    final gesture = await tester.startGesture(start);
    await tester.pump(const Duration(milliseconds: 100));
    await gesture.moveTo(end, timeStamp: const Duration(milliseconds: 200));
    await tester.pump(const Duration(milliseconds: 100));
    await gesture.up();
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 500));

    expect(gateway.appends, before + 1);
  });
}
