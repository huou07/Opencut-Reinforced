import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:or_app_bridge/or_app_bridge.dart';

import '../design/or_colors.dart';
import '../design/or_spacing.dart';
import '../design/or_typography.dart';
import '../widgets/or_widgets.dart';
import 'motion_library_controller.dart';
import 'motion_library_models.dart';

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
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          _header(),
          if (controller.busy || _working)
            const LinearProgressIndicator(minHeight: 2),
          Expanded(
            child: ListView(
              padding: const EdgeInsets.fromLTRB(
                OrSpacing.x3,
                OrSpacing.x2,
                OrSpacing.x3,
                OrSpacing.x4,
              ),
              children: [
                _searchRow(),
                const SizedBox(height: OrSpacing.x2),
                _categoryFilters(),
                if (controller.error != null || controller.notice != null) ...[
                  const SizedBox(height: OrSpacing.x2),
                  _messages(),
                ],
                if (controller.selected != null) ...[
                  const SizedBox(height: OrSpacing.x2),
                  _selectedDetail(controller.selected!),
                ],
                const SizedBox(height: OrSpacing.x2),
                if (entries.isEmpty)
                  _emptyLibrary()
                else
                  for (final entry in entries) _entryTile(entry),
                const SizedBox(height: OrSpacing.x3),
              ],
            ),
          ),
          if (controller.hasDraft)
            Padding(
              padding: const EdgeInsets.fromLTRB(
                OrSpacing.x3,
                0,
                OrSpacing.x3,
                OrSpacing.x3,
              ),
              child: _composer(),
            ),
        ],
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
    return TextField(
      key: const ValueKey('motion-library-search'),
      controller: _search,
      onChanged: controller.setSearch,
      style: const TextStyle(color: OrColors.text, fontSize: OrType.tinySize),
      decoration: const InputDecoration(
        isDense: true,
        hintText: 'Search assets',
        prefixIcon: Icon(Icons.search, size: 16),
      ),
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

  Widget _composer() {
    final hasDraft = controller.hasDraft;
    return OrPanel(
      title: 'Scene',
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Row(
            children: [
              Expanded(
                child: Text(
                  hasDraft
                      ? '${controller.draftLayerCount} layers'
                      : 'Drop an asset below to start a composition',
                  style: const TextStyle(
                    color: OrColors.textMuted,
                    fontSize: OrType.tinySize,
                  ),
                ),
              ),
              if (hasDraft)
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
                offsetX = (local.dx / box.size.width * 1920).clamp(0, 1920);
                offsetY = (local.dy / box.size.height * 1080).clamp(0, 1080);
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
              final bytes = controller.draftPreview ?? controller.assetPreview;
              return Container(
                key: _sceneKey,
                decoration: BoxDecoration(
                  color: OrColors.backgroundRecessed,
                  borderRadius: BorderRadius.circular(OrRadii.card),
                  border: Border.all(
                    color: highlighted ? OrColors.focus : OrColors.border,
                  ),
                ),
                child: AspectRatio(
                  aspectRatio: 16 / 9,
                  child: bytes == null
                      ? Center(
                          child: Text(
                            highlighted
                                ? 'Drop to add'
                                : 'Preview appears here',
                            style: const TextStyle(
                              color: OrColors.textMuted,
                              fontSize: OrType.tinySize,
                            ),
                          ),
                        )
                      : Image.memory(
                          bytes,
                          fit: BoxFit.contain,
                          gaplessPlayback: true,
                        ),
                ),
              );
            },
          ),
          const SizedBox(height: OrSpacing.x2),
          Row(
            children: [
              Expanded(
                child: FilledButton(
                  key: const ValueKey('motion-insert-draft'),
                  onPressed: hasDraft && !controller.busy
                      ? () => _insert(draft: true)
                      : null,
                  child: const Text('Add scene to timeline'),
                ),
              ),
              const SizedBox(width: OrSpacing.x2),
              Expanded(
                child: OutlinedButton(
                  key: const ValueKey('motion-save-scene'),
                  onPressed: hasDraft && !controller.busy ? _saveScene : null,
                  child: const Text('Save to library'),
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }
}

const List<Color> _swatches = [
  Color(0xFFF5F5F5),
  Color(0xFFA6A6AB),
  Color(0xFF3FC7FF),
  Color(0xFF47B881),
  Color(0xFFD8A542),
  Color(0xFFE05A5A),
  Color(0xFF9B7CFF),
  Color(0xFFF28C38),
  Color(0xFF31C48D),
  Color(0xFF2D9CDB),
  Color(0xFFEB5757),
  Color(0xFF090909),
];

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
