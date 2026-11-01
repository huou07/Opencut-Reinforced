import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:or_app_bridge/or_app_bridge.dart';
import 'package:path_provider/path_provider.dart';

import 'motion_insert_log.dart';
import 'motion_library_models.dart';

/// The bridge calls the library surface needs. A fake implements this in tests.
abstract class MotionLibraryGateway {
  Future<MotionAssetLibraryView> library(String root);
  Future<MotionAssetView> load(String root, String assetId);
  Future<MotionAssetView> save(
    String root,
    String documentJson,
    String? category,
  );
  Future<MotionAssetMutationView> remove(String root, String assetId);
  Future<MotionAssetView> importImage({
    required String root,
    required String sourcePath,
    required String name,
    String? category,
    required List<String> tags,
    required int width,
    required int height,
    required int seconds,
  });
  Future<MotionSceneView> assetScene({
    required String root,
    required String assetId,
    required String valuesJson,
    String? sceneId,
    String? name,
  });
  Future<MotionSceneView> newScene({
    required String name,
    required int width,
    required int height,
    required int seconds,
  });
  Future<MotionSceneView> append({
    required String root,
    required String sceneJson,
    required String assetId,
    required String valuesJson,
    required String prefix,
    required double offsetX,
    required double offsetY,
  });
  Future<MotionScenePreviewView> preview({
    required String root,
    required String sceneJson,
    required int atMilliseconds,
  });
  Future<MotionSceneView> editScene({
    required String root,
    required String sceneJson,
    required String editJson,
  });
  Future<MotionMaterializationView> materializeScene({
    required String root,
    required String sceneJson,
    required String outputPath,
    required int frameRateNumerator,
    required int frameRateDenominator,
    required int width,
    required int height,
  });
  Future<MotionMaterializationView> materializeAsset({
    required String root,
    required String assetId,
    required String valuesJson,
    required String outputPath,
    required int frameRateNumerator,
    required int frameRateDenominator,
    required int width,
    required int height,
  });
  Future<MotionAssetView> saveScene({
    required String root,
    required String sceneJson,
    required String name,
    String? category,
  });

  /// Opens an untrusted `.ormotion.json` file into an editable scene.
  Future<MotionSceneView> readScene({
    required String root,
    required String path,
  });

  /// Reopens a scene that was saved to this library as an editable draft.
  Future<MotionSceneView> sceneFromAsset({
    required String root,
    required String assetId,
  });
}

/// The real gateway: every call is a Rust bridge function.
class RustMotionLibraryGateway implements MotionLibraryGateway {
  const RustMotionLibraryGateway();

  @override
  Future<MotionAssetLibraryView> library(String root) =>
      motionLibrary(root: root);

  @override
  Future<MotionAssetView> load(String root, String assetId) =>
      motionAssetLoad(root: root, assetId: assetId);

  @override
  Future<MotionAssetView> save(
    String root,
    String documentJson,
    String? category,
  ) => motionAssetSave(
    root: root,
    documentJson: documentJson,
    category: category,
  );

  @override
  Future<MotionAssetMutationView> remove(String root, String assetId) =>
      motionAssetRemove(root: root, assetId: assetId);

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
  }) => motionAssetImportImage(
    root: root,
    sourcePath: sourcePath,
    name: name,
    category: category,
    tags: tags,
    width: width,
    height: height,
    seconds: seconds,
  );

  @override
  Future<MotionSceneView> assetScene({
    required String root,
    required String assetId,
    required String valuesJson,
    String? sceneId,
    String? name,
  }) => motionAssetScene(
    root: root,
    assetId: assetId,
    valuesJson: valuesJson,
    sceneId: sceneId,
    name: name,
  );

  @override
  Future<MotionSceneView> newScene({
    required String name,
    required int width,
    required int height,
    required int seconds,
  }) => motionSceneNew(
    name: name,
    width: width,
    height: height,
    seconds: seconds,
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
  }) => motionSceneAppend(
    root: root,
    sceneJson: sceneJson,
    assetId: assetId,
    valuesJson: valuesJson,
    prefix: prefix,
    offsetX: offsetX,
    offsetY: offsetY,
  );

  @override
  Future<MotionScenePreviewView> preview({
    required String root,
    required String sceneJson,
    required int atMilliseconds,
  }) => motionScenePreview(
    root: root,
    sceneJson: sceneJson,
    atMilliseconds: atMilliseconds,
  );

  @override
  Future<MotionSceneView> editScene({
    required String root,
    required String sceneJson,
    required String editJson,
  }) => motionSceneEdit(root: root, sceneJson: sceneJson, editJson: editJson);

  @override
  Future<MotionMaterializationView> materializeScene({
    required String root,
    required String sceneJson,
    required String outputPath,
    required int frameRateNumerator,
    required int frameRateDenominator,
    required int width,
    required int height,
  }) => motionSceneMaterialize(
    root: root,
    sceneJson: sceneJson,
    outputPath: outputPath,
    frameRateNumerator: frameRateNumerator,
    frameRateDenominator: frameRateDenominator,
    width: width,
    height: height,
  );

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
  }) => motionAssetMaterialize(
    root: root,
    assetId: assetId,
    valuesJson: valuesJson,
    outputPath: outputPath,
    frameRateNumerator: frameRateNumerator,
    frameRateDenominator: frameRateDenominator,
    width: width,
    height: height,
  );

  @override
  Future<MotionAssetView> saveScene({
    required String root,
    required String sceneJson,
    required String name,
    String? category,
  }) => motionSceneSave(
    root: root,
    sceneJson: sceneJson,
    name: name,
    category: category,
  );

  @override
  Future<MotionSceneView> readScene({
    required String root,
    required String path,
  }) => motionSceneRead(root: root, path: path);

  @override
  Future<MotionSceneView> sceneFromAsset({
    required String root,
    required String assetId,
  }) => motionSceneFromAsset(root: root, assetId: assetId);
}

/// Owns the motion library surface: the catalogue, the selected asset's parameter
/// values, the composition draft and the render requests.
class MotionLibraryController extends ChangeNotifier {
  MotionLibraryController({
    MotionLibraryGateway? gateway,
    Future<Directory> Function()? supportDirectory,
    this.insertLog,
  }) : _gateway = gateway ?? const RustMotionLibraryGateway(),
       _supportDirectory = supportDirectory ?? getApplicationSupportDirectory;

  final MotionLibraryGateway _gateway;
  final Future<Directory> Function() _supportDirectory;

  /// Records clips this controller puts on the timeline, for the panel's
  /// "on timeline" row. Optional so tests and other surfaces can omit it.
  final MotionInsertLog? insertLog;

  String _root = '';
  List<MotionAssetEntryView> _entries = const [];
  MotionAssetView? _selected;
  Map<String, Object?> _values = const {};
  Uint8List? _assetPreview;
  String? _draftSceneJson;
  Uint8List? _draftPreview;
  List<MotionSceneNodeView> _sceneNodes = const [];
  final Set<String> _selectedNodeIds = <String>{};
  final Set<String> _selectedAssetIds = <String>{};
  final List<_SceneSnapshot> _undoStack = <_SceneSnapshot>[];
  final List<_SceneSnapshot> _redoStack = <_SceneSnapshot>[];
  String _search = '';
  String? _category;
  bool _busy = false;
  bool _initialized = false;
  String? _error;
  String? _notice;
  Timer? _previewTimer;
  Timer? _textTimer;
  int _appendSequence = 0;
  bool _disposed = false;

  /// How many scene states the draft keeps for undo. A snapshot is a scene string,
  /// a PNG reference and its node boxes, so this is bounded in memory as well as in
  /// steps.
  static const int _maxHistory = 32;

  String get root => _root;
  List<MotionAssetEntryView> get entries => _entries;
  MotionAssetView? get selected => _selected;
  Map<String, Object?> get values => Map.unmodifiable(_values);
  Uint8List? get assetPreview => _assetPreview;
  String? get draftSceneJson => _draftSceneJson;
  Uint8List? get draftPreview => _draftPreview;
  String get search => _search;
  String? get categoryFilter => _category;
  bool get busy => _busy;
  bool get initialized => _initialized;
  String? get error => _error;
  String? get notice => _notice;
  bool get hasDraft => _draftSceneJson != null;
  List<MotionSceneNodeView> get sceneNodes => _sceneNodes;
  Set<String> get selectedNodeIds => Set.unmodifiable(_selectedNodeIds);
  bool get hasNodeSelection => _selectedNodeIds.isNotEmpty;
  int get selectionCount => _selectedNodeIds.length;

  /// The one selected node, or null for none and for a multiple selection.
  String? get selectedNodeId =>
      _selectedNodeIds.length == 1 ? _selectedNodeIds.first : null;

  bool isNodeSelected(String nodeId) => _selectedNodeIds.contains(nodeId);

  MotionSceneNodeView? nodeById(String nodeId) {
    for (final node in _sceneNodes) {
      if (node.nodeId == nodeId) return node;
    }
    return null;
  }

  MotionSceneNodeView? get selectedNode {
    final nodeId = selectedNodeId;
    return nodeId == null ? null : nodeById(nodeId);
  }

  List<MotionSceneNodeView> get selectedNodes => [
    for (final node in _sceneNodes)
      if (_selectedNodeIds.contains(node.nodeId)) node,
  ];

  bool get canUndo => _undoStack.isNotEmpty;
  bool get canRedo => _redoStack.isNotEmpty;

  Set<String> get selectedAssetIds => Set.unmodifiable(_selectedAssetIds);
  bool isAssetSelected(String assetId) => _selectedAssetIds.contains(assetId);

  /// The draft canvas, or a 1920x1080 default before a scene exists.
  ({double width, double height}) get canvasSize {
    final json = _draftSceneJson;
    if (json != null) {
      try {
        final decoded = jsonDecode(json);
        if (decoded is Map) {
          final canvas = decoded['canvas'];
          if (canvas is Map) {
            final width = (canvas['width'] as num?)?.toDouble();
            final height = (canvas['height'] as num?)?.toDouble();
            if (width != null && height != null && width > 0 && height > 0) {
              return (width: width, height: height);
            }
          }
        }
      } catch (_) {
        // A draft that cannot be read falls back to the default canvas.
      }
    }
    return (width: 1920, height: 1080);
  }

  int get draftLayerCount {
    final json = _draftSceneJson;
    if (json == null) return 0;
    try {
      final decoded = jsonDecode(json);
      final nodes = decoded is Map ? decoded['nodes'] : null;
      return nodes is List ? nodes.length : 0;
    } catch (_) {
      return 0;
    }
  }

  List<MotionAssetEntryView> get visibleEntries {
    final query = _search.trim().toLowerCase();
    return _entries.where((entry) {
      if (_category != null && entry.category != _category) return false;
      if (query.isEmpty) return true;
      return entry.name.toLowerCase().contains(query) ||
          entry.tags.any((tag) => tag.toLowerCase().contains(query));
    }).toList();
  }

  Future<void> initialize() async {
    if (_initialized) return;
    try {
      final support = await _supportDirectory();
      _root = '${support.path}${Platform.pathSeparator}motion-library';
      _initialized = true;
      await refresh();
    } catch (error) {
      _error = 'The motion library folder is unavailable: $error';
      _notify();
    }
  }

  Future<void> refresh() async {
    if (_root.isEmpty) return;
    _busy = true;
    _error = null;
    _notify();
    try {
      final view = await _gateway.library(_root);
      if (_disposed) return;
      if (!view.ok) {
        _error = view.message.isEmpty ? view.code : view.message;
      } else {
        _entries = view.entries;
        _selectedAssetIds.removeWhere(
          (id) => !_entries.any((entry) => entry.assetId == id),
        );
      }
    } catch (error) {
      _error = 'The motion library could not be read: $error';
    } finally {
      _busy = false;
      _notify();
    }
  }

  void setSearch(String value) {
    _search = value;
    _notify();
  }

  void setCategoryFilter(String? value) {
    _category = value;
    _notify();
  }

  void clearMessages() {
    _error = null;
    _notice = null;
    _notify();
  }

  Future<void> select(MotionAssetEntryView entry) async {
    if (_root.isEmpty) return;
    _busy = true;
    _error = null;
    _notify();
    try {
      final view = await _gateway.load(_root, entry.assetId);
      if (_disposed) return;
      if (!view.ok) {
        _error = view.message.isEmpty ? view.code : view.message;
        return;
      }
      _selected = view;
      _values = _defaultValues(view);
      _assetPreview = null;
      await _renderAssetPreview();
    } catch (error) {
      _error = 'The asset could not be opened: $error';
    } finally {
      _busy = false;
      _notify();
    }
  }

  void clearSelection() {
    _selected = null;
    _values = const {};
    _assetPreview = null;
    _notify();
  }

  void setValue(String id, Object? value) {
    final next = Map<String, Object?>.from(_values)..[id] = value;
    _values = next;
    _notify();
    _previewTimer?.cancel();
    _previewTimer = Timer(const Duration(milliseconds: 250), () {
      unawaited(_renderAssetPreview(refresh: true));
    });
  }

  /// Renders the selected asset's preview now, bypassing the edit debounce.
  Future<void> refreshPreview() => _renderAssetPreview(refresh: true);

  Future<MotionAssetView?> importImage(
    String sourcePath, {
    String name = '',
    String? category,
    List<String> tags = const [],
  }) async {
    if (_root.isEmpty) return null;
    _busy = true;
    _notify();
    try {
      final view = await _gateway.importImage(
        root: _root,
        sourcePath: sourcePath,
        name: name,
        category: category,
        tags: tags,
        width: 1920,
        height: 1080,
        seconds: 5,
      );
      if (_disposed) return null;
      if (!view.ok) {
        _error = view.message.isEmpty ? view.code : view.message;
        return null;
      }
      await refresh();
      await selectEntryById(view.assetId);
      return view;
    } catch (error) {
      _error = 'The image could not be imported: $error';
      return null;
    } finally {
      _busy = false;
      _notify();
    }
  }

  Future<void> selectEntryById(String assetId) async {
    for (final entry in _entries) {
      if (entry.assetId == assetId) {
        await select(entry);
        return;
      }
    }
  }

  Future<void> updateSelectedCategory(String category) async {
    final selected = _selected;
    if (selected == null || _root.isEmpty) return;
    _busy = true;
    _notify();
    try {
      final decoded = jsonDecode(selected.documentJson);
      if (decoded is! Map<String, dynamic>) return;
      decoded['category'] = category;
      decoded['category_source'] = 'explicit';
      final view = await _gateway.save(_root, jsonEncode(decoded), category);
      if (_disposed) return;
      if (!view.ok) {
        _error = view.message.isEmpty ? view.code : view.message;
      } else {
        _selected = view;
        _notice = 'Category set to $category';
        await refresh();
      }
    } catch (error) {
      _error = 'The category could not be changed: $error';
    } finally {
      _busy = false;
      _notify();
    }
  }

  Future<void> removeSelected() async {
    final selected = _selected;
    if (selected == null || _root.isEmpty) return;
    _busy = true;
    _notify();
    try {
      final view = await _gateway.remove(_root, selected.assetId);
      if (_disposed) return;
      if (!view.ok) {
        _error = view.message.isEmpty ? view.code : view.message;
      } else {
        _notice = 'Removed ${selected.name}';
        clearSelection();
        await refresh();
      }
    } catch (error) {
      _error = 'The asset could not be removed: $error';
    } finally {
      _busy = false;
      _notify();
    }
  }

  /// Adds the selected asset to the composition at a canvas offset.
  Future<void> addSelectedToScene({
    double offsetX = 0,
    double offsetY = 0,
  }) async {
    final selected = _selected;
    if (selected == null) return;
    await _appendToScene(
      assetId: selected.assetId,
      valuesJson: jsonEncode(_values),
      offsetX: offsetX,
      offsetY: offsetY,
    );
  }

  /// Adds a dragged card to the composition, using the values captured at drag
  /// start rather than whatever is selected now.
  Future<void> addDragToScene(
    MotionAssetDragData data, {
    double offsetX = 0,
    double offsetY = 0,
  }) => _appendToScene(
    assetId: data.assetId,
    valuesJson: data.valuesJson,
    offsetX: offsetX,
    offsetY: offsetY,
  );

  /// Applies the selected motion-kind asset to the one selected canvas node.
  ///
  /// A preset is a property track, not a scene body, so it cannot be appended;
  /// it replaces the node's existing track for that property through the same
  /// validated scene-edit path every other gesture uses.
  Future<void> applySelectedPresetToNode() async {
    final asset = _selected;
    final node = selectedNode;
    if (asset == null || asset.kind != 'motion') return;
    if (_draftSceneJson == null) {
      _error = 'Add an object to the composition before applying a preset.';
      _notify();
      return;
    }
    if (node == null) {
      _error = 'Select exactly one object on the canvas first.';
      _notify();
      return;
    }
    Object? preset;
    try {
      final document = jsonDecode(asset.documentJson);
      if (document is Map) preset = document['preset'];
    } on FormatException {
      preset = null;
    }
    if (preset is! Map) {
      _error = 'This motion asset carries no preset.';
      _notify();
      return;
    }
    final keyframes = preset['keyframes'];
    final property = preset['property'];
    if (keyframes is! List || property is! String) {
      _error = 'This motion preset is incomplete.';
      _notify();
      return;
    }
    await _applyEdit({
      'op': 'apply_preset',
      'node_id': node.nodeId,
      'property': property,
      'keyframes': keyframes,
      'easing': preset['easing'] ?? 'linear',
    });
  }

  Future<void> _appendToScene({
    required String assetId,
    required String valuesJson,
    required double offsetX,
    required double offsetY,
  }) async {
    if (_root.isEmpty) return;
    _busy = true;
    _error = null;
    _notify();
    try {
      final before = _snapshot();
      final sceneJson = await _ensureDraftScene();
      if (sceneJson == null) return;
      final view = await _gateway.append(
        root: _root,
        sceneJson: sceneJson,
        assetId: assetId,
        valuesJson: valuesJson,
        prefix: 'a${_appendSequence++}',
        offsetX: offsetX,
        offsetY: offsetY,
      );
      if (_disposed) return;
      if (!view.ok) {
        _error = view.message.isEmpty ? view.code : view.message;
        return;
      }
      _draftSceneJson = view.sceneJson;
      _draftPreview = view.previewPng;
      _sceneNodes = view.nodes;
      _remember(before);
      _notice = 'Added to the scene';
    } catch (error) {
      _error = 'The asset could not be added to the scene: $error';
    } finally {
      _busy = false;
      _notify();
    }
  }

  void clearScene() {
    final before = _snapshot();
    _draftSceneJson = null;
    _draftPreview = null;
    _sceneNodes = const [];
    _selectedNodeIds.clear();
    _appendSequence = 0;
    _remember(before);
    _notify();
  }

  /// Opens a `.ormotion.json` file from disk into the editable draft.
  ///
  /// Rust parses, validates and adopts the scene's images into this library, so
  /// the returned draft edits and renders exactly like one composed in the app.
  Future<MotionSceneView?> importSceneFile(String path) async {
    if (_root.isEmpty || path.trim().isEmpty) return null;
    _busy = true;
    _error = null;
    _notice = null;
    _notify();
    final before = _snapshot();
    try {
      final view = await _gateway.readScene(root: _root, path: path);
      if (_disposed) return null;
      if (!view.ok) {
        _error = view.message.isEmpty ? view.code : view.message;
        return null;
      }
      _seedDraft(view, before);
      _notice = 'Imported ${view.name}';
      return view;
    } catch (error) {
      _error = 'The scene could not be imported: $error';
      return null;
    } finally {
      _busy = false;
      _notify();
    }
  }

  /// Reopens a scene saved to this library so it can be edited again.
  Future<MotionSceneView?> openSceneAsset(String assetId) async {
    if (_root.isEmpty) return null;
    _busy = true;
    _error = null;
    _notice = null;
    _notify();
    final before = _snapshot();
    try {
      final view = await _gateway.sceneFromAsset(root: _root, assetId: assetId);
      if (_disposed) return null;
      if (!view.ok) {
        _error = view.message.isEmpty ? view.code : view.message;
        return null;
      }
      _seedDraft(view, before);
      _notice = 'Opened ${view.name}';
      return view;
    } catch (error) {
      _error = 'The saved scene could not be opened: $error';
      return null;
    } finally {
      _busy = false;
      _notify();
    }
  }

  void _seedDraft(MotionSceneView view, _SceneSnapshot before) {
    _draftSceneJson = view.sceneJson;
    _draftPreview = view.previewPng.isEmpty ? null : view.previewPng;
    _sceneNodes = view.nodes;
    _selectedNodeIds.clear();
    _appendSequence = 0;
    _remember(before);
  }

  // -- node selection ---------------------------------------------------

  /// Selects exactly one node, or clears the selection when `nodeId` is null.
  void selectNode(String? nodeId) {
    if (nodeId == null) {
      if (_selectedNodeIds.isEmpty) return;
      _selectedNodeIds.clear();
      _notify();
      return;
    }
    if (_selectedNodeIds.length == 1 && _selectedNodeIds.contains(nodeId)) {
      return;
    }
    _selectedNodeIds
      ..clear()
      ..add(nodeId);
    _notify();
  }

  /// Adds or removes one node from the selection, for shift-click.
  void toggleNodeSelection(String nodeId) {
    if (!_selectedNodeIds.remove(nodeId)) _selectedNodeIds.add(nodeId);
    _notify();
  }

  /// Replaces (or extends) the selection with a set of ids.
  void selectNodes(Iterable<String> nodeIds, {bool additive = false}) {
    if (!additive) _selectedNodeIds.clear();
    _selectedNodeIds.addAll(nodeIds);
    _notify();
  }

  /// Selects every node that draws something; a group draws nothing, so it is not
  /// part of a canvas selection.
  void selectAllNodes() {
    _selectedNodeIds
      ..clear()
      ..addAll(
        _sceneNodes
            .where((node) => node.kind != 'group')
            .map((node) => node.nodeId),
      );
    _notify();
  }

  void clearNodeSelection() => selectNode(null);

  // -- node edits -------------------------------------------------------

  /// Moves one node to an evaluated canvas position.
  Future<void> moveNode(String nodeId, double x, double y) =>
      _applyEdit({'op': 'move', 'node_id': nodeId, 'x': x, 'y': y});

  /// Moves a whole selection in one step; each target is an evaluated position.
  Future<void> moveNodes(Map<String, ({double x, double y})> targets) {
    if (targets.isEmpty) return Future<void>.value();
    return _applyEdit({
      'op': 'move_many',
      'nodes': [
        for (final entry in targets.entries)
          {'node_id': entry.key, 'x': entry.value.x, 'y': entry.value.y},
      ],
    });
  }

  /// Sets a shape's evaluated box: its top-left corner and its size.
  Future<void> resizeNode(
    String nodeId, {
    required double x,
    required double y,
    required double width,
    required double height,
  }) => _applyEdit({
    'op': 'resize',
    'node_id': nodeId,
    'x': x,
    'y': y,
    'width': width,
    'height': height,
  });

  /// Sets a node's rotation in degrees, counter-clockwise as the scene stores it.
  Future<void> rotateNode(String nodeId, double degrees) =>
      _applyEdit({'op': 'rotate', 'node_id': nodeId, 'degrees': degrees});

  /// Replaces one text node's content, debounced so typing does not render a
  /// frame per keystroke.
  void queueNodeText(String nodeId, String text) {
    _textTimer?.cancel();
    _textTimer = Timer(const Duration(milliseconds: 450), () {
      unawaited(setNodeText(nodeId, text));
    });
  }

  Future<void> setNodeText(String nodeId, String text) {
    _textTimer?.cancel();
    final node = nodeById(nodeId);
    if (node != null && node.text == text) {
      return Future<void>.value();
    }
    return _applyEdit({'op': 'set_text', 'node_id': nodeId, 'text': text});
  }

  /// Sets one node's fill colour.
  Future<void> setNodeColor(
    String nodeId, {
    required int red,
    required int green,
    required int blue,
    int alpha = 255,
  }) => _applyEdit({
    'op': 'set_fill',
    'node_id': nodeId,
    'color': {'red': red, 'green': green, 'blue': blue, 'alpha': alpha},
  });

  /// Sets the fill colour of the whole selection in one step.
  Future<void> setSelectionColor({
    required int red,
    required int green,
    required int blue,
    int alpha = 255,
  }) {
    final ids = [
      for (final node in selectedNodes)
        if (node.kind != 'group') node.nodeId,
    ];
    if (ids.isEmpty) return Future<void>.value();
    if (ids.length == 1) {
      return setNodeColor(
        ids.single,
        red: red,
        green: green,
        blue: blue,
        alpha: alpha,
      );
    }
    return _applyEdit({
      'op': 'set_fill_many',
      'node_ids': ids,
      'color': {'red': red, 'green': green, 'blue': blue, 'alpha': alpha},
    });
  }

  /// Sets an outline on every strokeable node in the selection.
  Future<void> setSelectionStroke({
    required int red,
    required int green,
    required int blue,
    int alpha = 255,
    double width = 4,
  }) {
    final ids = [
      for (final node in selectedNodes)
        if (node.strokeable) node.nodeId,
    ];
    if (ids.isEmpty) return Future<void>.value();
    return _applyEdits([
      for (final id in ids)
        {
          'op': 'set_stroke',
          'node_id': id,
          'color': {'red': red, 'green': green, 'blue': blue, 'alpha': alpha},
          'width': width,
        },
    ]);
  }

  /// Removes the outline from every outlined node in the selection.
  Future<void> clearSelectionStroke() {
    final ids = [
      for (final node in selectedNodes)
        if (node.hasStroke) node.nodeId,
    ];
    if (ids.isEmpty) return Future<void>.value();
    return _applyEdits([
      for (final id in ids) {'op': 'set_stroke', 'node_id': id, 'color': null},
    ]);
  }

  /// Sets a text node's type size in canvas pixels.
  Future<void> setNodeFontSize(String nodeId, double size) => _applyEdit({
    'op': 'set_font_size',
    'node_id': nodeId,
    'font_size': size.clamp(1, 2000),
  });

  /// Removes one node, its children and the animations that targeted it.
  Future<void> deleteNode(String nodeId) {
    _selectedNodeIds.remove(nodeId);
    return _applyEdit({'op': 'delete', 'node_id': nodeId});
  }

  /// Removes the whole selection in one step.
  Future<void> deleteSelectedNodes() {
    final ids = _selectedNodeIds.toList();
    if (ids.isEmpty) return Future<void>.value();
    _selectedNodeIds.clear();
    return _applyEdit({'op': 'delete_many', 'node_ids': ids});
  }

  /// Appends a new text node at a canvas position and selects it.
  Future<void> addTextNode({
    String text = 'New text',
    double? x,
    double? y,
    double fontSize = 72,
  }) async {
    final canvas = canvasSize;
    final nodeId = _nextTextNodeId();
    if (_draftSceneJson == null) {
      final sceneJson = await _ensureDraftScene();
      if (sceneJson == null) return;
      _notify();
    }
    await _applyEdit(
      {
        'op': 'add_text',
        'node_id': nodeId,
        'text': text,
        'x': x ?? canvas.width * 0.12,
        'y': y ?? canvas.height * 0.42,
        'font_size': fontSize,
      },
      selectNodeIds: {nodeId},
    );
  }

  /// Appends one library asset to the composition at a canvas position.
  Future<void> addAssetFromLibrary(String assetId, {double? x, double? y}) {
    final canvas = canvasSize;
    return _appendToScene(
      assetId: assetId,
      valuesJson: '{}',
      offsetX: x ?? canvas.width * 0.25,
      offsetY: y ?? canvas.height * 0.25,
    );
  }

  // -- draft history ----------------------------------------------------

  /// Undoes the last change to the composition draft. Project history is separate:
  /// this never touches the project revision.
  /// Restores the previous draft state, or the next one after an undo.
  void _applySnapshot(_SceneSnapshot snapshot) {
    _draftSceneJson = snapshot.sceneJson;
    _draftPreview = snapshot.preview;
    _sceneNodes = snapshot.nodes;
    _selectedNodeIds
      ..clear()
      ..addAll(
        snapshot.selected.where(
          (id) => snapshot.nodes.any((node) => node.nodeId == id),
        ),
      );
    _notify();
  }

  void undoScene() {
    if (_undoStack.isEmpty) return;
    _redoStack.add(_snapshot());
    if (_redoStack.length > _maxHistory) _redoStack.removeAt(0);
    _applySnapshot(_undoStack.removeLast());
  }

  void redoScene() {
    if (_redoStack.isEmpty) return;
    _undoStack.add(_snapshot());
    if (_undoStack.length > _maxHistory) _undoStack.removeAt(0);
    _applySnapshot(_redoStack.removeLast());
  }

  _SceneSnapshot _snapshot() => _SceneSnapshot(
    sceneJson: _draftSceneJson,
    preview: _draftPreview,
    nodes: _sceneNodes,
    selected: Set<String>.of(_selectedNodeIds),
  );

  /// Records a state that undo can return to, and drops the redo branch.
  void _remember(_SceneSnapshot before) {
    _undoStack.add(before);
    if (_undoStack.length > _maxHistory) _undoStack.removeAt(0);
    _redoStack.clear();
  }

  // -- library selection ------------------------------------------------

  void toggleAssetSelection(String assetId) {
    if (!_selectedAssetIds.remove(assetId)) _selectedAssetIds.add(assetId);
    _notify();
  }

  void clearAssetSelection() {
    if (_selectedAssetIds.isEmpty) return;
    _selectedAssetIds.clear();
    _notify();
  }

  /// Selects every asset the current filter shows, for a bulk action.
  void selectAllVisibleAssets() {
    _selectedAssetIds
      ..clear()
      ..addAll(visibleEntries.map((entry) => entry.assetId));
    _notify();
  }

  /// Removes every selected library asset. Each removal is its own atomic write;
  /// a failure names the first refusal and leaves the rest removed.
  Future<void> deleteSelectedAssets() async {
    if (_root.isEmpty || _selectedAssetIds.isEmpty) return;
    final ids = _selectedAssetIds.toList();
    _busy = true;
    _error = null;
    _notify();
    var removed = 0;
    String? failure;
    try {
      for (final assetId in ids) {
        final view = await _gateway.remove(_root, assetId);
        if (_disposed) return;
        if (!view.ok) {
          failure ??= view.message.isEmpty ? view.code : view.message;
          continue;
        }
        if (view.removed) removed += 1;
        _selectedAssetIds.remove(assetId);
        if (_selected?.assetId == assetId) {
          _selected = null;
          _values = const {};
          _assetPreview = null;
        }
      }
      _selectedAssetIds.clear();
      if (failure != null) {
        _error = failure;
      } else {
        _notice = removed == 1 ? 'Removed 1 asset' : 'Removed $removed assets';
      }
    } catch (error) {
      _error = 'The assets could not be removed: $error';
    } finally {
      _busy = false;
      await refresh();
      _notify();
    }
  }

  /// Appends every selected library asset to the composition, stepping each one so
  /// they do not land on top of each other.
  Future<void> addSelectedAssetsToScene() async {
    if (_selectedAssetIds.isEmpty) return;
    final ids = _selectedAssetIds
        .where(
          (id) => entries.any(
            (entry) => entry.assetId == id && entry.kind != 'motion',
          ),
        )
        .toList();
    if (ids.isEmpty) {
      _error = 'A motion preset applies to a selected object, not to the composition.';
      _notify();
      return;
    }
    final canvas = canvasSize;
    for (var index = 0; index < ids.length; index++) {
      await addAssetFromLibrary(
        ids[index],
        x: canvas.width * 0.2 + index * 32,
        y: canvas.height * 0.2 + index * 32,
      );
    }
    _notice = ids.length == 1
        ? 'Added to the scene'
        : 'Added ${ids.length} assets to the scene';
    _notify();
  }

  /// The draft scene, created on demand so a gesture on an empty panel starts a
  /// composition instead of doing nothing.
  Future<String?> _ensureDraftScene() async {
    final existing = _draftSceneJson;
    if (existing != null) return existing;
    final created = await _gateway.newScene(
      name: 'Composition',
      width: 1920,
      height: 1080,
      seconds: 5,
    );
    if (_disposed) return null;
    if (!created.ok) {
      _error = created.message.isEmpty ? created.code : created.message;
      return null;
    }
    _adoptScene(created);
    return created.sceneJson;
  }

  /// Nudges every selected node by a canvas-space delta.
  Future<void> nudgeSelectedNodes(double dx, double dy) {
    final targets = <String, ({double x, double y})>{
      for (final node in selectedNodes)
        node.nodeId: (x: node.x + dx, y: node.y + dy),
    };
    return moveNodes(targets);
  }

  /// Nudges the one selected node by a canvas-space delta.
  Future<void> nudgeSelectedNode(double dx, double dy) {
    final node = selectedNode;
    if (node == null) return Future<void>.value();
    return moveNode(node.nodeId, node.x + dx, node.y + dy);
  }

  String _nextTextNodeId() {
    final used = _sceneNodes.map((node) => node.nodeId).toSet();
    var index = 1;
    while (used.contains('text-$index')) {
      index += 1;
    }
    return 'text-$index';
  }

  Future<void> _applyEdit(
    Map<String, Object?> edit, {
    Set<String>? selectNodeIds,
  }) => _applyEdits([edit], selectNodeIds: selectNodeIds);

  /// Applies edits in order, as one undo step.
  Future<void> _applyEdits(
    List<Map<String, Object?>> edits, {
    Set<String>? selectNodeIds,
  }) async {
    final sceneJson = _draftSceneJson;
    if (sceneJson == null || _root.isEmpty || edits.isEmpty) return;
    final before = _snapshot();
    _busy = true;
    _error = null;
    _notify();
    MotionSceneView? last;
    try {
      var current = sceneJson;
      for (final edit in edits) {
        final view = await _gateway.editScene(
          root: _root,
          sceneJson: current,
          editJson: jsonEncode(edit),
        );
        if (_disposed) return;
        if (!view.ok) {
          _error = view.message.isEmpty ? view.code : view.message;
          break;
        }
        current = view.sceneJson;
        last = view;
      }
      if (last != null && !identical(last.sceneJson, sceneJson)) {
        _adoptScene(last);
        if (selectNodeIds != null) {
          _selectedNodeIds
            ..clear()
            ..addAll(selectNodeIds);
        }
        _remember(before);
      }
    } catch (error) {
      _error = 'The edit could not be applied: $error';
    } finally {
      _busy = false;
      _notify();
    }
  }

  /// Takes a scene the bridge returned as the new draft, keeping a selection that
  /// still names a node.
  void _adoptScene(MotionSceneView view) {
    _draftSceneJson = view.sceneJson;
    _draftPreview = view.previewPng;
    _sceneNodes = view.nodes;
    _selectedNodeIds.removeWhere(
      (id) => !_sceneNodes.any((node) => node.nodeId == id),
    );
  }

  Future<void> previewDraft({int atMilliseconds = 0}) async {
    final sceneJson = _draftSceneJson;
    if (sceneJson == null || _root.isEmpty) return;
    try {
      final view = await _gateway.preview(
        root: _root,
        sceneJson: sceneJson,
        atMilliseconds: atMilliseconds,
      );
      if (!_disposed && view.ok) {
        _draftPreview = view.png;
        _notify();
      }
    } catch (error) {
      _error = 'The scene preview could not be rendered: $error';
      _notify();
    }
  }

  Future<MotionAssetView?> saveSceneToLibrary({
    String name = '',
    String? category,
  }) async {
    final sceneJson = _draftSceneJson;
    if (sceneJson == null || _root.isEmpty) return null;
    _busy = true;
    _notify();
    try {
      final view = await _gateway.saveScene(
        root: _root,
        sceneJson: sceneJson,
        name: name,
        category: category,
      );
      if (_disposed) return null;
      if (!view.ok) {
        _error = view.message.isEmpty ? view.code : view.message;
        return null;
      }
      _notice = 'Saved ${view.name} to the library';
      await refresh();
      await selectEntryById(view.assetId);
      return view;
    } catch (error) {
      _error = 'The scene could not be saved: $error';
      return null;
    } finally {
      _busy = false;
      _notify();
    }
  }

  Future<MotionMaterializationView?> materializeSelected(
    String outputPath, {
    int width = 1920,
    int height = 1080,
  }) async {
    final selected = _selected;
    if (selected == null || _root.isEmpty) return null;
    return _materializeAsset(
      selected.assetId,
      jsonEncode(_values),
      outputPath,
      width,
      height,
    );
  }

  Future<MotionMaterializationView?> materializeDrag(
    MotionAssetDragData data,
    String outputPath, {
    int width = 1920,
    int height = 1080,
  }) => _materializeAsset(
    data.assetId,
    data.valuesJson,
    outputPath,
    width,
    height,
  );

  Future<MotionMaterializationView?> _materializeAsset(
    String assetId,
    String valuesJson,
    String outputPath,
    int width,
    int height,
  ) async {
    _busy = true;
    _error = null;
    _notify();
    try {
      final view = await _gateway.materializeAsset(
        root: _root,
        assetId: assetId,
        valuesJson: valuesJson,
        outputPath: outputPath,
        frameRateNumerator: 30,
        frameRateDenominator: 1,
        width: width,
        height: height,
      );
      if (!_disposed && !view.succeeded) {
        _error = view.message.isEmpty ? view.code : view.message;
      }
      return view;
    } catch (error) {
      _error = 'The asset could not be rendered: $error';
      return null;
    } finally {
      _busy = false;
      _notify();
    }
  }

  Future<MotionMaterializationView?> materializeDraft(
    String outputPath, {
    int width = 1920,
    int height = 1080,
  }) async {
    final sceneJson = _draftSceneJson;
    if (sceneJson == null || _root.isEmpty) return null;
    _busy = true;
    _error = null;
    _notify();
    try {
      final view = await _gateway.materializeScene(
        root: _root,
        sceneJson: sceneJson,
        outputPath: outputPath,
        frameRateNumerator: 30,
        frameRateDenominator: 1,
        width: width,
        height: height,
      );
      if (!_disposed && !view.succeeded) {
        _error = view.message.isEmpty ? view.code : view.message;
      }
      return view;
    } catch (error) {
      _error = 'The scene could not be rendered: $error';
      return null;
    } finally {
      _busy = false;
      _notify();
    }
  }

  /// Records a materialisation this controller put on the timeline. The
  /// panel's "on timeline" row reads it; a failed render records nothing.
  void recordInsert(MotionMaterializationView view, MotionInsertSource source) {
    if (!view.succeeded) return;
    insertLog?.record(
      MotionInsertRecord(
        outputPath: view.outputPath,
        sceneName: view.sceneName,
        width: view.width,
        height: view.height,
        frameCount: view.frameCount.toInt(),
        source: source,
      ),
    );
  }

  Future<void> _renderAssetPreview({bool refresh = false}) async {
    final selected = _selected;
    if (selected == null || _root.isEmpty || _disposed) return;
    try {
      final view = await _gateway.assetScene(
        root: _root,
        assetId: selected.assetId,
        valuesJson: jsonEncode(_values),
      );
      if (_disposed) return;
      if (!view.ok) {
        _error = view.message.isEmpty ? view.code : view.message;
      } else {
        _assetPreview = view.previewPng;
      }
      if (refresh) _notify();
    } catch (error) {
      if (!_disposed) {
        _error = 'The preview could not be rendered: $error';
        _notify();
      }
    }
  }

  Map<String, Object?> _defaultValues(MotionAssetView view) {
    final values = <String, Object?>{};
    for (final parameter in view.parameters) {
      try {
        values[parameter.id] = jsonDecode(parameter.defaultJson);
      } catch (_) {
        values[parameter.id] = switch (parameter.kind) {
          'number' => 0.0,
          'color' => {'red': 255, 'green': 255, 'blue': 255, 'alpha': 255},
          _ => '',
        };
      }
    }
    return values;
  }

  void _notify() {
    if (!_disposed) notifyListeners();
  }

  @override
  void dispose() {
    _disposed = true;
    _previewTimer?.cancel();
    _textTimer?.cancel();
    super.dispose();
  }
}

/// One reversible state of the composition draft.
///
/// The preview bytes are shared, not copied: a `Uint8List` the bridge returned is
/// immutable here, so a snapshot costs one scene string plus references.
class _SceneSnapshot {
  const _SceneSnapshot({
    required this.sceneJson,
    required this.preview,
    required this.nodes,
    required this.selected,
  });

  final String? sceneJson;
  final Uint8List? preview;
  final List<MotionSceneNodeView> nodes;
  final Set<String> selected;
}
