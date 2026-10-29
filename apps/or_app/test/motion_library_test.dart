import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:or_app_bridge/or_app_bridge.dart';
import 'package:or_app/design/or_theme.dart';
import 'package:or_app/motion/motion_insert_log.dart';
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
  "nodes": [
    {
      "node_id": "title",
      "kind": "text",
      "text": "Hello",
      "position": {"x": 100, "y": 200},
      "font_size": 72,
      "fill": {"red": 245, "green": 245, "blue": 245, "alpha": 255}
    }
  ],
  "animations": []
}
''';

/// A test double for the scene editor: the fake applies the same closed operation
/// set the Rust bridge does, so a widget test can prove a gesture reached a real
/// scene change rather than only that a call was made.
Map<String, dynamic> _applyFakeEdit(
  Map<String, dynamic> scene,
  Map<String, dynamic> edit,
) {
  final nodes = (scene['nodes'] as List).cast<Map<String, dynamic>>();
  Map<String, dynamic>? find(String id) {
    for (final node in nodes) {
      if (node['node_id'] == id) return node;
    }
    return null;
  }

  final op = edit['op'] as String;
  final id = edit['node_id'] as String?;
  switch (op) {
    case 'move':
      final node = find(id!)!;
      node['position'] = {'x': edit['x'], 'y': edit['y']};
    case 'move_many':
      for (final target
          in (edit['nodes'] as List).cast<Map<String, dynamic>>()) {
        final node = find(target['node_id'] as String)!;
        node['position'] = {'x': target['x'], 'y': target['y']};
      }
    case 'resize':
      final node = find(id!)!;
      node['size'] = {'width': edit['width'], 'height': edit['height']};
      if (edit['x'] != null && edit['y'] != null) {
        node['position'] = {'x': edit['x'], 'y': edit['y']};
      }
    case 'rotate':
      find(id!)!['rotation_degrees'] = edit['degrees'];
    case 'set_stroke':
      final node = find(id!)!;
      if (edit['color'] == null) {
        node.remove('stroke');
        node.remove('stroke_width');
      } else {
        node['stroke'] = edit['color'];
        node['stroke_width'] = edit['width'] ?? 4;
      }
    case 'set_fill_many':
      for (final target in (edit['node_ids'] as List).cast<String>()) {
        find(target)!['fill'] = edit['color'];
      }
    case 'set_text':
      find(id!)!['text'] = edit['text'];
    case 'set_fill':
      find(id!)!['fill'] = edit['color'];
    case 'set_font_size':
      find(id!)!['font_size'] = edit['font_size'];
    case 'delete':
      nodes.removeWhere((node) => node['node_id'] == id);
    case 'delete_many':
      final ids = (edit['node_ids'] as List).cast<String>();
      nodes.removeWhere((node) => ids.contains(node['node_id']));
    case 'add_text':
      nodes.add({
        'node_id': id,
        'kind': 'text',
        'text': edit['text'],
        'position': {'x': edit['x'], 'y': edit['y']},
        'font_size': edit['font_size'] ?? 64,
        'fill':
            edit['fill'] ??
            {'red': 245, 'green': 245, 'blue': 245, 'alpha': 255},
      });
  }
  return scene;
}

List<MotionSceneNodeView> _fakeNodes(Map<String, dynamic> scene) {
  final nodes = (scene['nodes'] as List).cast<Map<String, dynamic>>();
  return [
    for (final node in nodes)
      () {
        final position = node['position'] as Map<String, dynamic>? ?? const {};
        final fill = node['fill'] as Map<String, dynamic>? ?? const {};
        final fontSize = (node['font_size'] as num?)?.toDouble() ?? 0;
        final text = node['text'] as String? ?? '';
        final kind = node['kind'] as String? ?? 'text';
        final isText = kind == 'text';
        final size = node['size'] as Map<String, dynamic>? ?? const {};
        final stroke = node['stroke'] as Map<String, dynamic>? ?? const {};
        final strokeWidth = (node['stroke_width'] as num?)?.toDouble() ?? 0;
        return MotionSceneNodeView(
          nodeId: node['node_id'] as String,
          kind: kind,
          text: text,
          x: (position['x'] as num?)?.toDouble() ?? 0,
          y: (position['y'] as num?)?.toDouble() ?? 0,
          width: isText
              ? text.length * fontSize * 0.56
              : (size['width'] as num?)?.toDouble() ?? 0,
          height: isText
              ? fontSize * 1.25
              : (size['height'] as num?)?.toDouble() ?? 0,
          fontSize: fontSize,
          opacity: 1,
          hasFill: fill.isNotEmpty,
          fillRed: (fill['red'] as num?)?.toInt() ?? 255,
          fillGreen: (fill['green'] as num?)?.toInt() ?? 255,
          fillBlue: (fill['blue'] as num?)?.toInt() ?? 255,
          fillAlpha: (fill['alpha'] as num?)?.toInt() ?? 255,
          hasStroke: stroke.isNotEmpty,
          strokeRed: (stroke['red'] as num?)?.toInt() ?? 255,
          strokeGreen: (stroke['green'] as num?)?.toInt() ?? 255,
          strokeBlue: (stroke['blue'] as num?)?.toInt() ?? 255,
          strokeAlpha: (stroke['alpha'] as num?)?.toInt() ?? 255,
          strokeWidth: strokeWidth,
          rotationDegrees: (node['rotation_degrees'] as num?)?.toDouble() ?? 0,
          resizable:
              kind == 'rectangle' || kind == 'ellipse' || kind == 'image',
          strokeable:
              kind == 'rectangle' || kind == 'ellipse' || kind == 'arrow',
          depth: 0,
          animated: false,
        );
      }(),
  ];
}

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
  final List<Map<String, dynamic>> edits = [];
  Map<String, dynamic> scene = jsonDecode(_sceneJson) as Map<String, dynamic>;

  /// Adds a resizable, strokeable rectangle to the fake composition so a test can
  /// drive grips and outlines without an editor-only fixture scene.
  void seedRectangle({
    String nodeId = 'panel',
    double x = 900,
    double y = 600,
    double width = 400,
    double height = 200,
  }) {
    (scene['nodes'] as List).add({
      'node_id': nodeId,
      'kind': 'rectangle',
      'position': {'x': x, 'y': y},
      'size': {'width': width, 'height': height},
      'fill': {'red': 30, 'green': 30, 'blue': 30, 'alpha': 255},
    });
  }

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
    nodes: _fakeNodes(scene),
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
    nodes: _fakeNodes(scene),
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
      nodes: _fakeNodes(scene),
    );
  }

  @override
  Future<MotionSceneView> editScene({
    required String root,
    required String sceneJson,
    required String editJson,
  }) async {
    final edit = jsonDecode(editJson) as Map<String, dynamic>;
    edits.add(edit);
    final next = jsonDecode(jsonEncode(scene)) as Map<String, dynamic>;
    _applyFakeEdit(next, edit);
    scene = next;
    return MotionSceneView(
      ok: true,
      code: 'OK',
      message: '',
      sceneJson: jsonEncode(next),
      sceneId: '7a2b3c4d-5e6f-4a1b-8c9d-0e1f2a3b4c5d',
      name: 'Composition',
      width: 1920,
      height: 1080,
      durationMilliseconds: 5000,
      previewPng: _png,
      nodes: _fakeNodes(next),
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

  @override
  Future<MotionSceneView> readScene({
    required String root,
    required String path,
  }) async => MotionSceneView(
    ok: true,
    code: 'OK',
    message: '',
    sceneJson: jsonEncode(scene),
    sceneId: '7a2b3c4d-5e6f-4a1b-8c9d-0e1f2a3b4c5d',
    name: 'Imported demo',
    width: 1920,
    height: 1080,
    durationMilliseconds: 5000,
    previewPng: _png,
    nodes: _fakeNodes(scene),
  );

  @override
  Future<MotionSceneView> sceneFromAsset({
    required String root,
    required String assetId,
  }) async => readScene(root: root, path: assetId);
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

  test(
    'scene edits move, retitle, recolour, add and delete through the bridge',
    () async {
      final (controller, gateway, _) = await buildController();
      await controller.select(controller.entries.first);
      await controller.addSelectedToScene();
      expect(controller.sceneNodes, hasLength(1));
      expect(controller.sceneNodes.single.nodeId, 'title');

      await controller.moveNode('title', 640, 360);
      expect(gateway.edits.last['op'], 'move');
      expect(controller.sceneNodes.single.x, 640);
      expect(controller.sceneNodes.single.y, 360);

      await controller.setNodeColor('title', red: 200, green: 40, blue: 90);
      expect(gateway.edits.last['op'], 'set_fill');
      expect(controller.sceneNodes.single.fillRed, 200);
      expect(controller.sceneNodes.single.fillGreen, 40);
      expect(controller.sceneNodes.single.fillBlue, 90);

      await controller.setNodeFontSize('title', 96);
      expect(controller.sceneNodes.single.fontSize, 96);

      await controller.setNodeText('title', 'Changed');
      expect(gateway.edits.last['op'], 'set_text');
      expect(controller.sceneNodes.single.text, 'Changed');

      await controller.addTextNode(text: 'Second line');
      expect(gateway.edits.last['op'], 'add_text');
      expect(controller.sceneNodes, hasLength(2));
      expect(controller.selectedNodeId, 'text-1');
      expect(controller.selectedNode?.text, 'Second line');
      // The new node lands on the canvas, not on top of the existing title.
      expect(controller.selectedNode?.x, greaterThan(0));

      await controller.nudgeSelectedNode(8, 0);
      expect(gateway.edits.last['op'], 'move');

      await controller.deleteNode('text-1');
      expect(gateway.edits.last['op'], 'delete');
      expect(controller.sceneNodes, hasLength(1));
      expect(controller.selectedNodeId, isNull);
    },
  );

  test('add text starts a composition when no draft exists yet', () async {
    final (controller, gateway, _) = await buildController();
    expect(controller.hasDraft, isFalse);

    await controller.addTextNode(text: 'First object');

    expect(controller.hasDraft, isTrue);
    expect(controller.sceneNodes, isNotEmpty);
    expect(controller.selectedNode?.text, 'First object');
    expect(gateway.edits.last['op'], 'add_text');
  });

  test('importing a scene file seeds the editable canvas', () async {
    final (controller, _, _) = await buildController();
    expect(controller.hasDraft, isFalse);

    final view = await controller.importSceneFile('/tmp/demo.ormotion.json');

    expect(view, isNotNull);
    expect(controller.hasDraft, isTrue);
    expect(controller.sceneNodes, isNotEmpty);
    expect(controller.draftPreview, isNotNull);
    expect(controller.notice, contains('Imported'));
    expect(controller.canUndo, isTrue);
  });

  test('a saved scene can be reopened as an editable draft', () async {
    final (controller, _, _) = await buildController();

    final view = await controller.openSceneAsset(
      '1c8e3a7f-6d2b-4a4c-8e9f-0b1c2d3e4f5a',
    );

    expect(view, isNotNull);
    expect(controller.hasDraft, isTrue);
    expect(controller.sceneNodes, isNotEmpty);
    expect(controller.notice, contains('Opened'));
  });

  testWidgets('dragging a handle on the scene canvas moves the object', (
    tester,
  ) async {
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

    final handle = find.byKey(const ValueKey('motion-node-title'));
    expect(handle, findsOneWidget);
    final before = tester.getTopLeft(handle);

    await tester.drag(handle, const Offset(40, 20));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 300));

    expect(gateway.edits.last['op'], 'move_many');
    expect(controller.selectedNodeId, 'title');
    expect(controller.sceneNodes.single.x, greaterThan(100));
    expect(controller.sceneNodes.single.y, greaterThan(200));
    expect(tester.getTopLeft(handle).dx, greaterThan(before.dx));

    // Tapping the canvas background clears the selection.
    await tester.tap(find.byKey(const ValueKey('motion-scene-surface')));
    await tester.pump();
    expect(controller.selectedNodeId, isNull);
  });

  testWidgets('the inline inspector binds to the selected object', (
    tester,
  ) async {
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

    expect(
      find.byKey(const ValueKey('motion-node-inspector')),
      findsNothing,
      reason: 'nothing is selected yet',
    );

    await tester.tap(find.byKey(const ValueKey('motion-node-title')));
    await tester.pump();
    expect(find.byKey(const ValueKey('motion-node-inspector')), findsOneWidget);
    expect(find.byKey(const ValueKey('motion-node-text')), findsOneWidget);

    await tester.enterText(
      find.byKey(const ValueKey('motion-node-text')),
      'Inline edit',
    );
    await tester.pump(const Duration(milliseconds: 600));
    expect(gateway.edits.last['op'], 'set_text');
    expect(controller.sceneNodes.single.text, 'Inline edit');
  });

  testWidgets('the expanded editor retitles, recolours, resizes and deletes', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1200, 900);
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

    await tester.pumpWidget(
      MaterialApp(
        theme: OrTheme.dark,
        home: Scaffold(
          body: SizedBox(
            width: 1200,
            height: 900,
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

    await tester.tap(find.byKey(const ValueKey('motion-scene-expand')));
    await tester.pumpAndSettle();
    expect(find.text('Edit scene'), findsOneWidget);

    await tester.tap(find.byKey(const ValueKey('motion-editor-layer-title')));
    await tester.pump();
    expect(
      find.byKey(const ValueKey('motion-editor-node-inspector')),
      findsOneWidget,
    );

    await tester.enterText(
      find.byKey(const ValueKey('motion-editor-node-text')),
      'Edited title',
    );
    await tester.pump(const Duration(milliseconds: 600));
    expect(gateway.edits.last['op'], 'set_text');
    expect(controller.sceneNodes.single.text, 'Edited title');

    await tester.tap(
      find.byKey(const ValueKey('motion-editor-node-swatch-224-90-90')),
    );
    await tester.pump();
    expect(gateway.edits.last['op'], 'set_fill');
    expect(controller.sceneNodes.single.fillRed, 224);

    await tester.tap(find.byKey(const ValueKey('motion-editor-node-size-up')));
    await tester.pump();
    expect(gateway.edits.last['op'], 'set_font_size');
    expect(controller.sceneNodes.single.fontSize, greaterThan(72));

    await tester.tap(
      find.byKey(const ValueKey('motion-editor-node-nudge-right')),
    );
    await tester.pump();
    expect(gateway.edits.last['op'], 'move');
    expect(controller.sceneNodes.single.x, greaterThan(100));

    await tester.tap(find.byKey(const ValueKey('motion-editor-node-delete')));
    await tester.pump();
    expect(gateway.edits.last['op'], 'delete');
    expect(controller.sceneNodes, isEmpty);
  });

  testWidgets('the expanded editor adds text and an asset from the library', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1200, 900);
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
    final appendsBefore = gateway.appends;

    await tester.pumpWidget(
      MaterialApp(
        theme: OrTheme.dark,
        home: Scaffold(
          body: SizedBox(
            width: 1200,
            height: 900,
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

    await tester.tap(find.byKey(const ValueKey('motion-scene-expand')));
    await tester.pumpAndSettle();
    expect(find.text('Edit scene'), findsOneWidget);
    expect(find.byKey(const ValueKey('motion-node-title')), findsWidgets);

    await tester.tap(find.byKey(const ValueKey('motion-editor-add-text')));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 300));
    expect(controller.sceneNodes, hasLength(2));
    expect(controller.selectedNodeId, 'text-1');

    await tester.tap(find.byKey(const ValueKey('motion-editor-add-asset')));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('motion-add-asset-0')));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 300));
    expect(gateway.appends, appendsBefore + 1);

    await tester.tap(find.byKey(const ValueKey('motion-editor-close')));
    await tester.pumpAndSettle();
    expect(find.text('Edit scene'), findsNothing);
  });

  test(
    'a selection moves, recolours and deletes as one undoable step',
    () async {
      final (controller, gateway, _) = await buildController();
      await gateway.scene['nodes']
          as List; // keep the analyzer quiet about the field
      gateway.seedRectangle();
      await controller.select(controller.entries.first);
      await controller.addSelectedToScene();
      expect(controller.sceneNodes, hasLength(2));

      controller.selectAllNodes();
      expect(controller.selectionCount, 2);
      expect(
        controller.selectedNodeId,
        isNull,
        reason: 'a multiple selection has no single node',
      );

      final title = controller.nodeById('title')!;
      await controller.moveNodes({
        for (final node in controller.selectedNodes)
          node.nodeId: (x: node.x + 40, y: node.y + 20),
      });
      expect(gateway.edits.last['op'], 'move_many');
      expect(controller.nodeById('title')!.x, closeTo(title.x + 40, 0.001));

      await controller.setSelectionColor(red: 63, green: 199, blue: 255);
      expect(gateway.edits.last['op'], 'set_fill_many');
      expect(controller.nodeById('title')!.fillRed, 63);
      expect(controller.nodeById('panel')!.fillRed, 63);

      // One gesture is one history step, not one per bridge call.
      controller.undoScene();
      expect(controller.nodeById('title')!.fillRed, isNot(63));
      expect(controller.nodeById('panel')!.fillRed, 30);
      expect(controller.canRedo, isTrue);

      controller.redoScene();
      expect(controller.nodeById('title')!.fillRed, 63);

      await controller.deleteSelectedNodes();
      expect(controller.sceneNodes, isEmpty);
      expect(controller.hasNodeSelection, isFalse);

      controller.undoScene();
      expect(controller.sceneNodes, hasLength(2));
    },
  );

  test(
    'resize, rotate and outline reach the bridge and are undoable',
    () async {
      final (controller, gateway, _) = await buildController();
      gateway.seedRectangle();
      await controller.select(controller.entries.first);
      await controller.addSelectedToScene();

      await controller.resizeNode(
        'panel',
        x: 900,
        y: 600,
        width: 640,
        height: 360,
      );
      expect(gateway.edits.last['op'], 'resize');
      expect(controller.nodeById('panel')!.width, 640);

      await controller.rotateNode('panel', 45);
      expect(gateway.edits.last['op'], 'rotate');
      expect(controller.nodeById('panel')!.rotationDegrees, 45);

      controller.selectNode('panel');
      await controller.setSelectionStroke(red: 224, green: 90, blue: 90);
      expect(gateway.edits.last['op'], 'set_stroke');
      expect(controller.nodeById('panel')!.hasStroke, isTrue);
      expect(controller.nodeById('panel')!.strokeRed, 224);

      await controller.clearSelectionStroke();
      expect(controller.nodeById('panel')!.hasStroke, isFalse);

      // Undo walks back through each of those edits.
      expect(controller.canUndo, isTrue);
      controller.undoScene();
      expect(controller.nodeById('panel')!.hasStroke, isTrue);
    },
  );

  test(
    'the library deletes and adds a multi-selection in one action',
    () async {
      final (controller, gateway, _) = await buildController();
      expect(controller.entries, hasLength(2));

      controller.toggleAssetSelection(controller.entries.first.assetId);
      controller.toggleAssetSelection(controller.entries.last.assetId);
      expect(controller.selectedAssetIds, hasLength(2));

      await controller.deleteSelectedAssets();
      expect(gateway.removes, 2);
      expect(controller.selectedAssetIds, isEmpty);
      expect(controller.notice, contains('2 assets'));

      controller.toggleAssetSelection(controller.entries.first.assetId);
      controller.toggleAssetSelection(controller.entries.last.assetId);
      await controller.addSelectedAssetsToScene();
      expect(gateway.appends, 2);
      expect(controller.hasDraft, isTrue);
    },
  );

  testWidgets('the library bar appears for a ticked or select-all row', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(900, 2400);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    late MotionLibraryController controller;
    late Directory support;
    await tester.runAsync(() async {
      (controller, _, support) = await buildController();
    });

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
    expect(find.byKey(const ValueKey('motion-library-bulk')), findsNothing);

    await tester.tap(find.byKey(const ValueKey('motion-library-select-all')));
    await tester.pump();
    expect(find.byKey(const ValueKey('motion-library-bulk')), findsOneWidget);
    expect(controller.selectedAssetIds, hasLength(2));

    await tester.tap(find.byKey(const ValueKey('motion-library-bulk-clear')));
    await tester.pump();
    expect(find.byKey(const ValueKey('motion-library-bulk')), findsNothing);
  });

  testWidgets('a marquee selects several objects and drags them together', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(900, 2400);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    late MotionLibraryController controller;
    late _FakeMotionGateway gateway;
    late Directory support;
    await tester.runAsync(() async {
      (controller, gateway, support) = await buildController();
    });
    gateway.seedRectangle();
    await controller.select(controller.entries.first);
    await controller.addSelectedToScene();

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

    final surface = find.byKey(const ValueKey('motion-scene-surface'));
    await tester.dragFrom(
      tester.getTopLeft(surface) + const Offset(4, 4),
      const Offset(260, 140),
    );
    await tester.pump();
    expect(controller.selectionCount, 2, reason: 'the marquee covers both');

    final before = controller.nodeById('title')!.x;
    await tester.drag(
      find.byKey(const ValueKey('motion-node-title')),
      const Offset(24, 12),
    );
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 300));

    expect(gateway.edits.last['op'], 'move_many');
    expect(
      (gateway.edits.last['nodes'] as List),
      hasLength(2),
      reason: 'one gesture moves the whole selection',
    );
    expect(controller.nodeById('title')!.x, greaterThan(before));
    expect(controller.nodeById('panel')!.x, greaterThan(900));
  });

  testWidgets('corner grips resize and the rotate grip turns a shape', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(900, 2400);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    late MotionLibraryController controller;
    late _FakeMotionGateway gateway;
    late Directory support;
    await tester.runAsync(() async {
      (controller, gateway, support) = await buildController();
    });
    gateway.seedRectangle();
    await controller.select(controller.entries.first);
    await controller.addSelectedToScene();

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

    await tester.tap(find.byKey(const ValueKey('motion-node-panel')));
    await tester.pump();
    final resized = controller.nodeById('panel')!;
    expect(resized.width, 400);

    await tester.drag(
      find.byKey(const ValueKey('motion-resize-panel-0')),
      const Offset(20, 10),
    );
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 300));

    expect(gateway.edits.last['op'], 'resize');
    expect(controller.nodeById('panel')!.width, greaterThan(400));
    expect(controller.nodeById('panel')!.height, greaterThan(200));

    await tester.drag(
      find.byKey(const ValueKey('motion-rotate-panel')),
      const Offset(18, 18),
    );
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 300));

    expect(gateway.edits.last['op'], 'rotate');
    expect(controller.nodeById('panel')!.rotationDegrees, isNot(0));
  });

  testWidgets('the outline control adds and clears a stroke', (tester) async {
    tester.view.physicalSize = const Size(1200, 900);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    late MotionLibraryController controller;
    late _FakeMotionGateway gateway;
    late Directory support;
    await tester.runAsync(() async {
      (controller, gateway, support) = await buildController();
    });
    gateway.seedRectangle();
    await controller.select(controller.entries.first);
    await controller.addSelectedToScene();

    await tester.pumpWidget(
      MaterialApp(
        theme: OrTheme.dark,
        home: Scaffold(
          body: SizedBox(
            width: 1200,
            height: 900,
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

    await tester.tap(find.byKey(const ValueKey('motion-scene-expand')));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('motion-editor-layer-panel')));
    await tester.pump();

    await tester.tap(
      find.byKey(const ValueKey('motion-editor-node-outline-224-90-90')),
    );
    await tester.pump();
    expect(gateway.edits.last['op'], 'set_stroke');
    expect(controller.nodeById('panel')!.hasStroke, isTrue);

    await tester.tap(
      find.byKey(const ValueKey('motion-editor-node-outline-none')),
    );
    await tester.pump();
    expect(controller.nodeById('panel')!.hasStroke, isFalse);
  });

  testWidgets('the panel undo and redo buttons walk the draft history', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(900, 2400);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    late MotionLibraryController controller;
    late _FakeMotionGateway gateway;
    late Directory support;
    await tester.runAsync(() async {
      (controller, gateway, support) = await buildController();
    });
    gateway.seedRectangle();
    await controller.select(controller.entries.first);
    await controller.addSelectedToScene();

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

    await tester.tap(find.byKey(const ValueKey('motion-node-panel')));
    await tester.pump();
    final before = controller.nodeById('panel')!.x;
    await tester.drag(
      find.byKey(const ValueKey('motion-node-panel')),
      const Offset(30, 0),
    );
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 300));
    expect(controller.nodeById('panel')!.x, greaterThan(before));

    await tester.tap(find.byKey(const ValueKey('motion-undo')));
    await tester.pump();
    expect(controller.nodeById('panel')!.x, closeTo(before, 0.001));

    await tester.tap(find.byKey(const ValueKey('motion-redo')));
    await tester.pump();
    expect(controller.nodeById('panel')!.x, greaterThan(before));
  });

  testWidgets('a rendered clip stays visible and can be rendered again', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(900, 2400);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    final log = MotionInsertLog();
    late MotionLibraryController controller;
    late _FakeMotionGateway gateway;
    late Directory supportDir;
    await tester.runAsync(() async {
      supportDir = await Directory.systemTemp.createTemp('or-motion-row-');
      gateway = _FakeMotionGateway();
      controller = MotionLibraryController(
        gateway: gateway,
        supportDirectory: () async => supportDir,
        insertLog: log,
      );
      await controller.initialize();
    });
    addTearDown(() async {
      controller.dispose();
      await supportDir.delete(recursive: true);
    });
    await controller.select(controller.entries.first);
    await controller.addSelectedToScene();

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
              outputDirectory: supportDir.path,
              onInsertClip: (_) async {
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
    expect(
      find.byKey(const ValueKey('motion-scene-on-timeline')),
      findsNothing,
    );

    await tester.tap(find.byKey(const ValueKey('motion-insert-draft')));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 800));
    expect(inserted, 1);
    expect(
      find.byKey(const ValueKey('motion-scene-on-timeline')),
      findsOneWidget,
    );
    expect(
      find.textContaining(
        'On timeline \u00b7 1920\u00d71080 \u00b7 150 frames',
      ),
      findsOneWidget,
    );

    final firstOutput = log.last!.outputPath;
    await tester.tap(find.byKey(const ValueKey('motion-scene-rerender')));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 800));
    expect(inserted, 2);
    expect(log.last!.outputPath, isNot(firstOutput));
    expect(log.last!.frameCount, 150);
  });

}
