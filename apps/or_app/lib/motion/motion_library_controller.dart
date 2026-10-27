import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:or_app_bridge/or_app_bridge.dart';
import 'package:path_provider/path_provider.dart';

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
}

/// Owns the motion library surface: the catalogue, the selected asset's parameter
/// values, the composition draft and the render requests.
class MotionLibraryController extends ChangeNotifier {
  MotionLibraryController({
    MotionLibraryGateway? gateway,
    Future<Directory> Function()? supportDirectory,
  }) : _gateway = gateway ?? const RustMotionLibraryGateway(),
       _supportDirectory = supportDirectory ?? getApplicationSupportDirectory;

  final MotionLibraryGateway _gateway;
  final Future<Directory> Function() _supportDirectory;

  String _root = '';
  List<MotionAssetEntryView> _entries = const [];
  MotionAssetView? _selected;
  Map<String, Object?> _values = const {};
  Uint8List? _assetPreview;
  String? _draftSceneJson;
  Uint8List? _draftPreview;
  String _search = '';
  String? _category;
  bool _busy = false;
  bool _initialized = false;
  String? _error;
  String? _notice;
  Timer? _previewTimer;
  int _appendSequence = 0;
  bool _disposed = false;

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
      var sceneJson = _draftSceneJson;
      if (sceneJson == null) {
        final created = await _gateway.newScene(
          name: 'Composition',
          width: 1920,
          height: 1080,
          seconds: 5,
        );
        if (!created.ok) {
          _error = created.message.isEmpty ? created.code : created.message;
          return;
        }
        sceneJson = created.sceneJson;
      }
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
      _notice = 'Added to the scene';
    } catch (error) {
      _error = 'The asset could not be added to the scene: $error';
    } finally {
      _busy = false;
      _notify();
    }
  }

  void clearScene() {
    _draftSceneJson = null;
    _draftPreview = null;
    _appendSequence = 0;
    _notify();
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
    super.dispose();
  }
}
