import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:or_app_bridge/or_app_bridge.dart';

import '../design/or_colors.dart';
import '../design/or_spacing.dart';
import '../design/or_typography.dart';
import '../widgets/or_widgets.dart';
import 'motion_library_controller.dart';
import 'motion_library_models.dart';
import 'motion_scene_editor.dart';

const List<String> motionAssetCategories = [
  'background',
  'people',
  'nature',
  'animal',
  'vehicle',
  'object',
  'text',
  'shape',
  'icon',
  'chart',
  'effect',
  'motion',
];

/// The Motion tool panel: browse the asset library, adjust an asset, compose a
/// scene and drag an asset onto the timeline.
class MotionLibraryPanel extends StatefulWidget {
  const MotionLibraryPanel({
    super.key,
    required this.controller,
    required this.onInsertClip,
    required this.onNotice,
    required this.onOpenCodeMotion,
    this.outputDirectory,
    this.pickImage,
  });

  final MotionLibraryController controller;

  /// Imports a materialised video and inserts it on the timeline.
  final Future<bool> Function(String path) onInsertClip;
  final ValueChanged<String> onNotice;
  final VoidCallback onOpenCodeMotion;

  /// A project-adjacent folder where materialised videos are written.
  final String? outputDirectory;

  /// Returns one PNG path, or null when the user cancels. The shell supplies
  /// this from the shared file picker so Android SAF and the UI harness work.
  final Future<String?> Function()? pickImage;

  @override
  State<MotionLibraryPanel> createState() => _MotionLibraryPanelState();
}

class _MotionLibraryPanelState extends State<MotionLibraryPanel> {
  final _search = TextEditingController();
  final _sceneKey = GlobalKey();
  bool _working = false;

  MotionLibraryController get controller => widget.controller;

  @override
  void initState() {
    super.initState();
    controller.addListener(_onChanged);
    unawaited(controller.initialize());
  }

  @override
  void dispose() {
    controller.removeListener(_onChanged);
    _search.dispose();
    super.dispose();
  }

  void _onChanged() {
    if (mounted) setState(() {});
  }

  Future<void> _importImage() async {
    final pick = widget.pickImage;
    if (pick == null) {
      widget.onNotice('Importing an image is unavailable here.');
      return;
    }
    final path = await pick();
    if (path == null || path.isEmpty) return;
    final name = path.split(Platform.pathSeparator).last;
    final asset = await controller.importImage(
      path,
      name: name.replaceAll(RegExp(r'\.png$'), ''),
    );
    if (!mounted) return;
    widget.onNotice(
      asset == null
          ? (controller.error ?? 'The image could not be imported.')
          : 'Imported ${asset.name}',
    );
  }

  String? _outputPath(String label) {
    final directory = widget.outputDirectory;
    if (directory == null || directory.isEmpty) return null;
    final safe = label.replaceAll(RegExp(r'[^A-Za-z0-9_-]'), '-');
    return '$directory${Platform.pathSeparator}motion-$safe-'
        '${DateTime.now().millisecondsSinceEpoch}.mkv';
  }

  Future<void> _insert({required bool draft}) async {
    final output = _outputPath(draft ? 'scene' : 'asset');
    if (output == null) {
      widget.onNotice('Open a project before adding a motion clip.');
      return;
    }
    setState(() => _working = true);
    try {
      final view = draft
          ? await controller.materializeDraft(output)
          : await controller.materializeSelected(output);
      if (!mounted) return;
      if (view == null || !view.succeeded) {
        final message = view?.message ?? '';
        widget.onNotice(
          message.isNotEmpty
              ? message
              : (controller.error ?? 'The motion clip could not be rendered.'),
        );
        return;
      }
      final added = await widget.onInsertClip(view.outputPath);
      if (!mounted) return;
      widget.onNotice(
        added
            ? 'Motion clip added to the timeline.'
            : 'The rendered clip could not be inserted.',
      );
    } finally {
      if (mounted) setState(() => _working = false);
    }
  }

  Future<void> _saveScene() async {
    final name = await _askSceneName();
    if (name == null) return;
    final saved = await controller.saveSceneToLibrary(name: name);
    if (!mounted) return;
    widget.onNotice(
      saved == null
          ? (controller.error ?? 'The scene could not be saved.')
          : 'Saved ${saved.name} to the motion library.',
    );
  }

  Future<String?> _askSceneName() async {
    final field = TextEditingController(text: 'Composition');
    final name = await showDialog<String>(
      context: context,
      builder: (context) => AlertDialog(
        backgroundColor: OrColors.surface,
        title: const Text('Save scene to library'),
        content: TextField(
          key: const ValueKey('motion-scene-name'),
          controller: field,
          autofocus: true,
          decoration: const InputDecoration(labelText: 'Name'),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(),
            child: const Text('Cancel'),
          ),
          FilledButton(
            key: const ValueKey('motion-scene-save-confirm'),
            onPressed: () => Navigator.of(context).pop(field.text.trim()),
            child: const Text('Save'),
          ),
        ],
      ),
    );
    field.dispose();
    return name == null || name.isEmpty ? null : name;
  }

  @override
  Widget build(BuildContext context) {
    final entries = controller.visibleEntries;
    return ColoredBox(
      key: const ValueKey('motion-library-panel'),
      color: OrColors.backgroundRaised,
      child: LayoutBuilder(
        builder: (context, constraints) {
          // The scene editor sits between the list and the pinned actions. Its
          // height is derived from the panel, so a short rail scrolls its controls
          // instead of overflowing, and the insert action never scrolls away.
          final available = math.max(0.0, constraints.maxHeight - 38 - 60);
          final editing = math
              .min(320.0, constraints.maxHeight * 0.55)
              .clamp(0.0, available);
          return Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              _header(),
              if (controller.busy || _working)
                const LinearProgressIndicator(minHeight: 2),
              // Messages live above the list, not inside it: an error that scrolls
              // away is an error the user never reads.
              if (controller.error != null || controller.notice != null)
                Padding(
                  padding: const EdgeInsets.fromLTRB(
                    OrSpacing.x3,
                    OrSpacing.x1,
                    OrSpacing.x3,
                    0,
                  ),
                  child: _messages(),
                ),
              Expanded(
                child: ListView(
                  padding: const EdgeInsets.fromLTRB(
                    OrSpacing.x3,
                    OrSpacing.x2,
                    OrSpacing.x3,
                    OrSpacing.x2,
                  ),
                  children: [
                    _searchRow(),
                    const SizedBox(height: OrSpacing.x2),
                    _categoryFilters(),
                    if (controller.selected != null) ...[
                      const SizedBox(height: OrSpacing.x2),
                      _selectedDetail(controller.selected!),
                    ],
                    const SizedBox(height: OrSpacing.x2),
                    if (controller.selectedAssetIds.isNotEmpty) ...[
                      _bulkBar(),
                      const SizedBox(height: OrSpacing.x2),
                    ],
                    if (entries.isEmpty)
                      _emptyLibrary()
                    else
                      for (final entry in entries) _entryTile(entry),
                    const SizedBox(height: OrSpacing.x2),
                  ],
                ),
              ),
              if (controller.hasDraft)
                SizedBox(
                  height: editing,
                  child: SingleChildScrollView(
                    padding: const EdgeInsets.symmetric(
                      horizontal: OrSpacing.x3,
                    ),
                    child: _sceneEditor(),
                  ),
                ),
              Padding(
                padding: const EdgeInsets.fromLTRB(
                  OrSpacing.x3,
                  OrSpacing.x1,
                  OrSpacing.x3,
                  OrSpacing.x2,
                ),
                child: _sceneActions(),
              ),
            ],
          );
        },
      ),
    );
  }

  Widget _header() {
    return SizedBox(
      height: 38,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: OrSpacing.x2),
        child: Row(
          children: [
            const Expanded(
              child: Text(
                'Motion library',
                style: TextStyle(
                  color: OrColors.text,
                  fontSize: OrType.metadataSize,
                  fontWeight: FontWeight.w600,
                ),
              ),
            ),
            IconButton(
              key: const ValueKey('motion-library-import-image'),
              iconSize: 18,
              tooltip: 'Import PNG cutout',
              onPressed: controller.busy ? null : _importImage,
              icon: const Icon(Icons.add_photo_alternate_outlined),
            ),
            IconButton(
              key: const ValueKey('motion-library-refresh'),
              iconSize: 18,
              tooltip: 'Refresh library',
              onPressed: controller.busy
                  ? null
                  : () => unawaited(controller.refresh()),
              icon: const Icon(Icons.refresh),
            ),
            IconButton(
              key: const ValueKey('motion-open-code'),
              iconSize: 18,
              tooltip: 'Create from code',
              onPressed: widget.onOpenCodeMotion,
              icon: const Icon(Icons.animation_outlined),
            ),
          ],
        ),
      ),
    );
  }

  Widget _searchRow() {
    return Row(
      children: [
        Expanded(
          child: TextField(
            key: const ValueKey('motion-library-search'),
            controller: _search,
            onChanged: controller.setSearch,
            style: const TextStyle(
              color: OrColors.text,
              fontSize: OrType.tinySize,
            ),
            decoration: const InputDecoration(
              isDense: true,
              hintText: 'Search assets',
              prefixIcon: Icon(Icons.search, size: 16),
            ),
          ),
        ),
        IconButton(
          key: const ValueKey('motion-library-select-all'),
          iconSize: 16,
          tooltip: 'Select every asset shown',
          onPressed: controller.visibleEntries.isEmpty
              ? null
              : controller.selectAllVisibleAssets,
          icon: const Icon(Icons.select_all),
        ),
      ],
    );
  }

  Widget _categoryFilters() {
    Widget chip(String? value, String label) {
      final selected = controller.categoryFilter == value;
      return Padding(
        padding: const EdgeInsets.only(
          right: OrSpacing.x1,
          bottom: OrSpacing.x1,
        ),
        child: InkWell(
          key: ValueKey('motion-category-${value ?? 'all'}'),
          borderRadius: BorderRadius.circular(OrRadii.small),
          onTap: () => controller.setCategoryFilter(value),
          child: Container(
            padding: const EdgeInsets.symmetric(
              horizontal: OrSpacing.x2,
              vertical: 3,
            ),
            decoration: BoxDecoration(
              color: selected ? OrColors.surfaceSelected : OrColors.surface,
              borderRadius: BorderRadius.circular(OrRadii.small),
              border: Border.all(
                color: selected ? OrColors.borderStrong : OrColors.border,
              ),
            ),
            child: Text(
              label,
              style: TextStyle(
                color: selected ? OrColors.text : OrColors.textSecondary,
                fontSize: OrType.tinySize,
              ),
            ),
          ),
        ),
      );
    }

    return Wrap(
      children: [
        chip(null, 'All'),
        for (final category in motionAssetCategories) chip(category, category),
      ],
    );
  }

  Widget _messages() {
    final message = controller.error ?? controller.notice;
    final isError = controller.error != null;
    return Container(
      padding: const EdgeInsets.all(OrSpacing.x2),
      decoration: BoxDecoration(
        color: isError
            ? OrColors.danger.withValues(alpha: 0.12)
            : OrColors.success.withValues(alpha: 0.12),
        borderRadius: BorderRadius.circular(OrRadii.small),
      ),
      child: Row(
        children: [
          Expanded(
            child: Text(
              message!,
              style: TextStyle(
                color: isError ? OrColors.danger : OrColors.success,
                fontSize: OrType.tinySize,
              ),
            ),
          ),
          InkWell(
            onTap: controller.clearMessages,
            child: const Icon(Icons.close, size: 14, color: OrColors.textMuted),
          ),
        ],
      ),
    );
  }

  Widget _emptyLibrary() {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: OrSpacing.x6),
      child: Column(
        children: [
          const Icon(
            Icons.movie_filter_outlined,
            size: 28,
            color: OrColors.textMuted,
          ),
          const SizedBox(height: OrSpacing.x2),
          Text(
            'No motion assets yet.\nImport a PNG or save a composed scene.',
            textAlign: TextAlign.center,
            style: TextStyle(
              color: OrColors.textMuted,
              fontSize: OrType.tinySize,
              height: 1.4,
            ),
          ),
        ],
      ),
    );
  }

  Widget _entryTile(MotionAssetEntryView entry) {
    final selected = controller.selected?.assetId == entry.assetId;
    final marked = controller.isAssetSelected(entry.assetId);
    final valuesJson = selected ? jsonEncode(controller.values) : '{}';
    final tile = Material(
      color: selected ? OrColors.surfaceSelected : OrColors.surface,
      borderRadius: BorderRadius.circular(OrRadii.card),
      child: InkWell(
        key: ValueKey('motion-asset-${entry.assetId}'),
        borderRadius: BorderRadius.circular(OrRadii.card),
        onTap: () => unawaited(controller.select(entry)),
        child: Padding(
          padding: const EdgeInsets.all(OrSpacing.x2),
          child: Row(
            children: [
              InkWell(
                key: ValueKey('motion-asset-select-${entry.assetId}'),
                onTap: () => controller.toggleAssetSelection(entry.assetId),
                child: Icon(
                  marked
                      ? Icons.check_box_outlined
                      : Icons.check_box_outline_blank,
                  size: 16,
                  color: marked ? OrColors.selection : OrColors.textMuted,
                ),
              ),
              const SizedBox(width: OrSpacing.x1),
              _thumbnail(entry.previewPath, 44, 44),
              const SizedBox(width: OrSpacing.x2),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      entry.name,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: const TextStyle(
                        color: OrColors.text,
                        fontSize: OrType.metadataCompactSize,
                      ),
                    ),
                    Text(
                      '${entry.category} · ${entry.kind}',
                      style: const TextStyle(
                        color: OrColors.textMuted,
                        fontSize: OrType.tinySize,
                      ),
                    ),
                  ],
                ),
              ),
              const Icon(
                Icons.drag_indicator,
                size: 14,
                color: OrColors.textMuted,
              ),
            ],
          ),
        ),
      ),
    );
    return Padding(
      padding: const EdgeInsets.only(bottom: OrSpacing.x1),
      child: Draggable<MotionAssetDragData>(
        data: MotionAssetDragData(
          assetId: entry.assetId,
          name: entry.name,
          valuesJson: valuesJson,
          previewPath: entry.previewPath,
        ),
        feedback: Material(
          color: Colors.transparent,
          child: Container(
            width: 180,
            padding: const EdgeInsets.all(OrSpacing.x2),
            decoration: BoxDecoration(
              color: OrColors.surfaceSelected,
              borderRadius: BorderRadius.circular(OrRadii.card),
              border: Border.all(color: OrColors.borderStrong),
            ),
            child: Text(
              entry.name,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: const TextStyle(
                color: OrColors.text,
                fontSize: OrType.tinySize,
              ),
            ),
          ),
        ),
        childWhenDragging: Opacity(opacity: 0.35, child: tile),
        child: tile,
      ),
    );
  }

  /// The actions a multi-selection of library assets offers: add them all to the
  /// composition, or remove them in one go.
  Widget _bulkBar() {
    final count = controller.selectedAssetIds.length;
    return Container(
      key: const ValueKey('motion-library-bulk'),
      padding: const EdgeInsets.all(OrSpacing.x2),
      decoration: BoxDecoration(
        color: OrColors.surfaceSelected,
        borderRadius: BorderRadius.circular(OrRadii.card),
        border: Border.all(color: OrColors.borderStrong),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Row(
            children: [
              const Icon(
                Icons.check_box_outlined,
                size: 14,
                color: OrColors.selection,
              ),
              const SizedBox(width: OrSpacing.x1),
              Expanded(
                child: Text(
                  '$count selected',
                  key: const ValueKey('motion-library-bulk-count'),
                  style: const TextStyle(
                    color: OrColors.text,
                    fontSize: OrType.metadataCompactSize,
                    fontWeight: FontWeight.w600,
                  ),
                ),
              ),
              IconButton(
                key: const ValueKey('motion-library-bulk-clear'),
                iconSize: 14,
                visualDensity: VisualDensity.compact,
                tooltip: 'Clear selection',
                onPressed: controller.clearAssetSelection,
                icon: const Icon(Icons.close),
              ),
            ],
          ),
          const SizedBox(height: OrSpacing.x1),
          Row(
            children: [
              Expanded(
                child: OutlinedButton.icon(
                  key: const ValueKey('motion-library-bulk-add'),
                  onPressed: controller.busy
                      ? null
                      : () => unawaited(controller.addSelectedAssetsToScene()),
                  icon: const Icon(Icons.add, size: 14),
                  label: const Text('To scene'),
                ),
              ),
              const SizedBox(width: OrSpacing.x2),
              Expanded(
                child: OutlinedButton.icon(
                  key: const ValueKey('motion-library-bulk-delete'),
                  onPressed: controller.busy
                      ? null
                      : () => unawaited(controller.deleteSelectedAssets()),
                  icon: const Icon(Icons.delete_outline, size: 14),
                  label: const Text('Delete'),
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }

  Widget _thumbnail(String path, double width, double height) {
    if (path.isEmpty || !File(path).existsSync()) {
      return Container(
        width: width,
        height: height,
        decoration: BoxDecoration(
          color: OrColors.backgroundRecessed,
          borderRadius: BorderRadius.circular(OrRadii.small),
        ),
        child: const Icon(
          Icons.image_outlined,
          size: 18,
          color: OrColors.textMuted,
        ),
      );
    }
    return ClipRRect(
      borderRadius: BorderRadius.circular(OrRadii.small),
      child: Image.file(
        File(path),
        width: width,
        height: height,
        fit: BoxFit.cover,
        cacheWidth: (width * 3).round(),
        gaplessPlayback: true,
        errorBuilder: (context, error, stack) => Container(
          width: width,
          height: height,
          color: OrColors.backgroundRecessed,
          child: const Icon(Icons.broken_image_outlined, size: 18),
        ),
      ),
    );
  }

  Widget _selectedDetail(MotionAssetView asset) {
    return OrPanel(
      title: asset.name,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Row(
            children: [
              Expanded(
                child: DropdownButtonFormField<String>(
                  key: const ValueKey('motion-asset-category'),
                  initialValue: motionAssetCategories.contains(asset.category)
                      ? asset.category
                      : null,
                  isExpanded: true,
                  dropdownColor: OrColors.surface,
                  decoration: const InputDecoration(
                    labelText: 'Category',
                    isDense: true,
                  ),
                  items: [
                    for (final category in motionAssetCategories)
                      DropdownMenuItem(value: category, child: Text(category)),
                  ],
                  onChanged: (value) {
                    if (value != null) {
                      unawaited(controller.updateSelectedCategory(value));
                    }
                  },
                ),
              ),
              const SizedBox(width: OrSpacing.x2),
              Text(
                asset.categorySource,
                style: const TextStyle(
                  color: OrColors.textMuted,
                  fontSize: OrType.tinySize,
                ),
              ),
            ],
          ),
          const SizedBox(height: OrSpacing.x2),
          if (asset.parameters.isEmpty)
            Text(
              'This asset has no adjustable parameters.',
              style: TextStyle(
                color: OrColors.textMuted,
                fontSize: OrType.tinySize,
              ),
            ),
          for (final parameter in asset.parameters)
            _parameterControl(asset, parameter),
          const SizedBox(height: OrSpacing.x2),
          Row(
            children: [
              Expanded(
                child: FilledButton(
                  key: const ValueKey('motion-insert-selected'),
                  onPressed: controller.busy
                      ? null
                      : () => _insert(draft: false),
                  child: const Text('Add to timeline'),
                ),
              ),
              const SizedBox(width: OrSpacing.x2),
              Expanded(
                child: OutlinedButton(
                  key: const ValueKey('motion-add-to-scene'),
                  onPressed: controller.busy
                      ? null
                      : () => unawaited(controller.addSelectedToScene()),
                  child: const Text('Add to scene'),
                ),
              ),
            ],
          ),
          Align(
            alignment: Alignment.centerRight,
            child: TextButton.icon(
              key: const ValueKey('motion-delete-asset'),
              onPressed: controller.busy
                  ? null
                  : () => unawaited(controller.removeSelected()),
              icon: const Icon(Icons.delete_outline, size: 16),
              label: const Text('Delete asset'),
            ),
          ),
        ],
      ),
    );
  }

  Widget _parameterControl(
    MotionAssetView asset,
    MotionAssetParameterView parameter,
  ) {
    final keySuffix = '${asset.assetId}-${parameter.id}';
    final value = controller.values[parameter.id];
    switch (parameter.kind) {
      case 'text':
      case 'choice':
        if (parameter.kind == 'choice') {
          final current = value is String ? value : null;
          return Padding(
            padding: const EdgeInsets.only(top: OrSpacing.x2),
            child: DropdownButtonFormField<String>(
              key: ValueKey('motion-param-$keySuffix'),
              initialValue: parameter.choices.contains(current)
                  ? current
                  : null,
              isExpanded: true,
              dropdownColor: OrColors.surface,
              decoration: InputDecoration(
                labelText: parameter.label,
                isDense: true,
              ),
              items: [
                for (final choice in parameter.choices)
                  DropdownMenuItem(value: choice, child: Text(choice)),
              ],
              onChanged: (value) {
                if (value != null) controller.setValue(parameter.id, value);
              },
            ),
          );
        }
        return Padding(
          padding: const EdgeInsets.only(top: OrSpacing.x2),
          child: TextFormField(
            key: ValueKey('motion-param-$keySuffix'),
            initialValue: value is String ? value : '',
            style: const TextStyle(
              color: OrColors.text,
              fontSize: OrType.tinySize,
            ),
            decoration: InputDecoration(
              labelText: parameter.label,
              isDense: true,
            ),
            onChanged: (text) => controller.setValue(parameter.id, text),
          ),
        );
      case 'color':
        final color = _colorFrom(value);
        return Padding(
          padding: const EdgeInsets.only(top: OrSpacing.x2),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                parameter.label,
                style: const TextStyle(
                  color: OrColors.textMuted,
                  fontSize: OrType.tinySize,
                ),
              ),
              const SizedBox(height: OrSpacing.x1),
              Wrap(
                spacing: OrSpacing.x1,
                runSpacing: OrSpacing.x1,
                children: [
                  for (final swatch in _swatches)
                    InkWell(
                      key: ValueKey(
                        'motion-param-${parameter.id}-${swatch.toARGB32()}',
                      ),
                      onTap: () => controller.setValue(parameter.id, {
                        'red': (swatch.r * 255).round(),
                        'green': (swatch.g * 255).round(),
                        'blue': (swatch.b * 255).round(),
                        'alpha': 255,
                      }),
                      child: Container(
                        width: 20,
                        height: 20,
                        decoration: BoxDecoration(
                          color: swatch,
                          borderRadius: BorderRadius.circular(4),
                          border: Border.all(
                            color: swatch.toARGB32() == color.toARGB32()
                                ? OrColors.focus
                                : OrColors.border,
                            width: swatch.toARGB32() == color.toARGB32()
                                ? 2
                                : 1,
                          ),
                        ),
                      ),
                    ),
                ],
              ),
            ],
          ),
        );
      default:
        final number = value is num ? value.toDouble() : 0.0;
        final min = parameter.min ?? 0.0;
        final max = parameter.max ?? (number > 100 ? number * 2 : 100.0);
        final boundedMax = max <= min ? min + 1 : max;
        return Padding(
          padding: const EdgeInsets.only(top: OrSpacing.x2),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                children: [
                  Expanded(
                    child: Text(
                      parameter.label,
                      style: const TextStyle(
                        color: OrColors.textMuted,
                        fontSize: OrType.tinySize,
                      ),
                    ),
                  ),
                  Text(
                    number.toStringAsFixed(1),
                    style: const TextStyle(
                      color: OrColors.textSecondary,
                      fontSize: OrType.tinySize,
                    ),
                  ),
                ],
              ),
              SliderTheme(
                data: SliderThemeData(
                  trackHeight: 2,
                  thumbShape: const RoundSliderThumbShape(
                    enabledThumbRadius: 6,
                  ),
                  overlayShape: const RoundSliderOverlayShape(
                    overlayRadius: 12,
                  ),
                  activeTrackColor: OrColors.textSecondary,
                  inactiveTrackColor: OrColors.border,
                  thumbColor: OrColors.text,
                ),
                child: Slider(
                  key: ValueKey('motion-param-$keySuffix'),
                  value: number.clamp(min, boundedMax),
                  min: min,
                  max: boundedMax,
                  onChanged: (value) =>
                      controller.setValue(parameter.id, value),
                ),
              ),
            ],
          ),
        );
    }
  }

  /// The composition editor: only while a draft exists, so browsing the library
  /// keeps the full height of the rail until the user starts composing.
  Widget _sceneEditor() {
    final canvas = controller.canvasSize;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Row(
          children: [
            Expanded(
              child: Text(
                controller.sceneNodes.length == 1
                    ? '1 object'
                    : '${controller.sceneNodes.length} objects',
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: const TextStyle(
                  color: OrColors.textMuted,
                  fontSize: OrType.tinySize,
                ),
              ),
            ),
            IconButton(
              key: const ValueKey('motion-undo'),
              iconSize: 16,
              tooltip: 'Undo scene edit',
              onPressed: controller.canUndo ? controller.undoScene : null,
              icon: const Icon(Icons.undo),
            ),
            IconButton(
              key: const ValueKey('motion-redo'),
              iconSize: 16,
              tooltip: 'Redo scene edit',
              onPressed: controller.canRedo ? controller.redoScene : null,
              icon: const Icon(Icons.redo),
            ),
            IconButton(
              key: const ValueKey('motion-scene-expand'),
              iconSize: 16,
              tooltip: 'Open the scene editor',
              onPressed: () =>
                  unawaited(showMotionSceneEditor(context, controller)),
              icon: const Icon(Icons.open_in_full),
            ),
            IconButton(
              key: const ValueKey('motion-scene-clear'),
              iconSize: 16,
              tooltip: 'Clear scene',
              onPressed: controller.clearScene,
              icon: const Icon(Icons.close),
            ),
          ],
        ),
        const SizedBox(height: OrSpacing.x1),
        DragTarget<MotionAssetDragData>(
          key: const ValueKey('motion-scene-drop'),
          onAcceptWithDetails: (details) {
            final box =
                _sceneKey.currentContext?.findRenderObject() as RenderBox?;
            var offsetX = 0.0;
            var offsetY = 0.0;
            if (box != null && box.hasSize && box.size.width > 0) {
              final local = box.globalToLocal(details.offset);
              offsetX = (local.dx / box.size.width * canvas.width).clamp(
                0,
                canvas.width,
              );
              offsetY = (local.dy / box.size.height * canvas.height).clamp(
                0,
                canvas.height,
              );
            }
            unawaited(
              controller.addDragToScene(
                details.data,
                offsetX: offsetX,
                offsetY: offsetY,
              ),
            );
          },
          builder: (context, candidates, rejected) {
            final highlighted = candidates.isNotEmpty;
            return Container(
              key: _sceneKey,
              foregroundDecoration: BoxDecoration(
                borderRadius: BorderRadius.circular(OrRadii.card),
                border: highlighted
                    ? Border.all(color: OrColors.focus, width: 1.5)
                    : null,
              ),
              child: MotionSceneCanvas(
                controller: controller,
                emptyLabel: highlighted
                    ? 'Drop to add'
                    : 'Preview appears here',
              ),
            );
          },
        ),
        const SizedBox(height: OrSpacing.x2),
        Row(
          children: [
            Expanded(
              child: OutlinedButton.icon(
                key: const ValueKey('motion-add-text'),
                onPressed: controller.busy
                    ? null
                    : () => unawaited(controller.addTextNode()),
                icon: const Icon(Icons.title, size: 16),
                label: const Text('Add text'),
              ),
            ),
            const SizedBox(width: OrSpacing.x2),
            Expanded(
              child: OutlinedButton.icon(
                key: const ValueKey('motion-add-asset'),
                onPressed: controller.busy
                    ? null
                    : () => unawaited(showAddAssetSheet(context, controller)),
                icon: const Icon(Icons.add_photo_alternate_outlined, size: 16),
                label: const Text('Add asset'),
              ),
            ),
          ],
        ),
        const SizedBox(height: OrSpacing.x2),
        MotionSceneInspector(controller: controller, compact: true),
      ],
    );
  }

  Widget _sceneActions() {
    if (!controller.hasDraft) {
      // No composition yet: the first gesture starts one, so a user who wants a
      // title does not have to add an asset first.
      return Row(
        children: [
          Expanded(
            child: OutlinedButton.icon(
              key: const ValueKey('motion-add-text'),
              onPressed: controller.busy
                  ? null
                  : () => unawaited(controller.addTextNode()),
              icon: const Icon(Icons.title, size: 16),
              label: const Text('Add text'),
            ),
          ),
          const SizedBox(width: OrSpacing.x2),
          Expanded(
            child: OutlinedButton.icon(
              key: const ValueKey('motion-add-asset'),
              onPressed: controller.busy
                  ? null
                  : () => unawaited(showAddAssetSheet(context, controller)),
              icon: const Icon(Icons.add_photo_alternate_outlined, size: 16),
              label: const Text('Add asset'),
            ),
          ),
        ],
      );
    }
    return Row(
      children: [
        Expanded(
          child: FilledButton(
            key: const ValueKey('motion-insert-draft'),
            onPressed: !controller.busy ? () => _insert(draft: true) : null,
            child: const Text('Add to timeline'),
          ),
        ),
        const SizedBox(width: OrSpacing.x2),
        Expanded(
          child: OutlinedButton(
            key: const ValueKey('motion-save-scene'),
            onPressed: !controller.busy ? _saveScene : null,
            child: const Text('Save to library'),
          ),
        ),
      ],
    );
  }
}

const List<Color> _swatches = motionSceneSwatches;

Color _colorFrom(Object? value) {
  if (value is Map) {
    final red = (value['red'] as num?)?.round().clamp(0, 255) ?? 255;
    final green = (value['green'] as num?)?.round().clamp(0, 255) ?? 255;
    final blue = (value['blue'] as num?)?.round().clamp(0, 255) ?? 255;
    final alpha = (value['alpha'] as num?)?.round().clamp(0, 255) ?? 255;
    return Color.fromARGB(alpha, red, green, blue);
  }
  return const Color(0xFFF5F5F5);
}
