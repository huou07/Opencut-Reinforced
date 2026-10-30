import 'package:flutter/material.dart';

import '../design/or_colors.dart';
import '../design/or_spacing.dart';
import '../design/or_typography.dart';
import '../project/export_profile.dart';
import '../project/project_gateway.dart';

/// What a user asked to materialise.
class MotionMaterializeRequest {
  const MotionMaterializeRequest({
    required this.sourcePath,
    required this.destinationPath,
    required this.frameRate,
    required this.width,
    required this.height,
    required this.title,
  });

  final String sourcePath;
  final String destinationPath;
  final int frameRate;
  final int width;
  final int height;
  final String title;
}

/// Opens the code-to-illustration surface.
///
/// The dialog owns the fields, the bounds and the feedback; the caller owns the work,
/// through `render`, so the surface can be driven by a test without a Rust library and
/// the shell can pass the real gateway call. The bounds are the ones the encoder and the
/// rasteriser actually accept, refused here with a message rather than after a render.
Future<void> showMotionMaterializeDialog({
  required BuildContext context,
  required Future<String?> Function() chooseSource,
  required Future<String?> Function(String suggestedName) chooseDestination,
  required Future<MotionMaterializationOutcome> Function(
    MotionMaterializeRequest,
  )
  render,
  required Future<bool> Function(MotionMaterializationOutcome outcome)
  addToTimeline,
}) {
  return showDialog<void>(
    context: context,
    builder: (dialogContext) => _MotionMaterializeDialog(
      chooseSource: chooseSource,
      chooseDestination: chooseDestination,
      render: render,
      addToTimeline: addToTimeline,
    ),
  );
}

class _MotionMaterializeDialog extends StatefulWidget {
  const _MotionMaterializeDialog({
    required this.chooseSource,
    required this.chooseDestination,
    required this.render,
    required this.addToTimeline,
  });

  final Future<String?> Function() chooseSource;
  final Future<String?> Function(String suggestedName) chooseDestination;
  final Future<MotionMaterializationOutcome> Function(MotionMaterializeRequest)
  render;
  final Future<bool> Function(MotionMaterializationOutcome outcome)
  addToTimeline;

  @override
  State<_MotionMaterializeDialog> createState() =>
      _MotionMaterializeDialogState();
}

class _MotionMaterializeDialogState extends State<_MotionMaterializeDialog> {
  late final TextEditingController _title = TextEditingController(
    text: 'Illustration',
  );
  late final TextEditingController _frameRate = TextEditingController(
    text: '10',
  );
  late final TextEditingController _width = TextEditingController(text: '640');
  late final TextEditingController _height = TextEditingController(text: '360');

  bool _busy = false;
  String? _status;
  bool _failed = false;
  String? _sourcePath;
  String? _destinationPath;
  MotionMaterializationOutcome? _outcome;

  @override
  void dispose() {
    _title.dispose();
    _frameRate.dispose();
    _width.dispose();
    _height.dispose();
    super.dispose();
  }

  int? _positive(TextEditingController controller) {
    final value = int.tryParse(controller.text.trim());
    if (value == null || value <= 0) return null;
    return value;
  }

  Future<void> _chooseSource() async {
    setState(() => _busy = true);
    try {
      final path = await widget.chooseSource();
      if (!mounted || path == null) return;
      setState(() {
        _sourcePath = path;
        _destinationPath = null;
        _outcome = null;
        _status = null;
        _failed = false;
      });
    } catch (_) {
      if (mounted) {
        setState(() {
          _failed = true;
          _status = 'A code file could not be selected.';
        });
      }
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _chooseDestination() async {
    setState(() => _busy = true);
    try {
      final title = _title.text.trim().isEmpty ? 'Illustration' : _title.text;
      final timestamp = DateTime.now().millisecondsSinceEpoch;
      final suggestedName = ExportProfile.matroskaFfv1PcmS16le.suggestedName(
        '$title-$timestamp',
      );
      final path = await widget.chooseDestination(suggestedName);
      if (!mounted || path == null) return;
      setState(() {
        _destinationPath = path;
        _outcome = null;
        _status = null;
        _failed = false;
      });
    } catch (_) {
      if (mounted) {
        setState(() {
          _failed = true;
          _status = 'An output location could not be selected.';
        });
      }
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _submit() async {
    final source = _sourcePath;
    if (source == null) {
      setState(() {
        _failed = true;
        _status = 'Choose a code file to illustrate.';
      });
      return;
    }
    final destination = _destinationPath;
    if (destination == null) {
      setState(() {
        _failed = true;
        _status = 'Choose where the video should be written.';
      });
      return;
    }
    final frameRate = _positive(_frameRate);
    final width = _positive(_width);
    final height = _positive(_height);
    if (frameRate == null || width == null || height == null) {
      setState(() {
        _failed = true;
        _status = 'Frame rate, width and height must be positive numbers.';
      });
      return;
    }
    if (width > 4096 || height > 4096) {
      setState(() {
        _failed = true;
        _status = 'The canvas is bounded at 4096 by 4096.';
      });
      return;
    }
    setState(() {
      _busy = true;
      _failed = false;
      _status = 'Rendering…';
    });
    MotionMaterializationOutcome outcome;
    try {
      outcome = await widget.render(
        MotionMaterializeRequest(
          sourcePath: source,
          destinationPath: destination,
          frameRate: frameRate,
          width: width,
          height: height,
          title: _title.text.trim(),
        ),
      );
    } catch (_) {
      outcome = MotionMaterializationOutcome(
        succeeded: false,
        destinationPath: destination,
        frameCount: 0,
        width: width,
        height: height,
        frameRateNumerator: frameRate,
        frameRateDenominator: 1,
        bytesWritten: 0,
        errorCode: 'MOTION_RENDER_FAILED',
        message: 'The illustration video could not be rendered.',
      );
    }
    if (!mounted) return;
    setState(() {
      _busy = false;
      _failed = !outcome.succeeded;
      _outcome = outcome.succeeded ? outcome : null;
      _status = outcome.succeeded
          ? 'Rendered ${outcome.frameCount} frames at ${outcome.width}×'
                '${outcome.height}.'
          : '${outcome.errorCode}: ${outcome.message}';
    });
  }

  Future<void> _addToTimeline() async {
    if (_outcome == null) return;
    setState(() {
      _busy = true;
      _failed = false;
      _status = 'Adding to the project and opening the preview…';
    });
    var added = false;
    try {
      added = await widget.addToTimeline(_outcome!);
    } catch (_) {
      // The rendered file stays at its selected destination for manual import.
    }
    if (!mounted) return;
    if (added) {
      Navigator.of(context).pop();
      return;
    }
    setState(() {
      _busy = false;
      _failed = true;
      _status =
          'The video is rendered, but could not be added to the timeline.';
    });
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      key: const ValueKey('motion-materialize-dialog'),
      scrollable: true,
      title: const Text('Illustration Video'),
      content: SizedBox(
        width: 380,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            const Text(
              'Turn a code file into a video: the source becomes a declarative '
              'MotionScene and renders through the editor’s own encoder.',
              style: TextStyle(
                color: OrColors.textMuted,
                fontSize: OrType.metadataSize,
              ),
            ),
            const SizedBox(height: OrSpacing.x3),
            _fileChoice(
              key: 'motion-materialize-source-picker',
              label: 'Code file',
              path: _sourcePath,
              emptyLabel: 'No code file selected',
              buttonLabel: 'Choose file',
              onPressed: _chooseSource,
            ),
            _fileChoice(
              key: 'motion-materialize-destination-picker',
              label: 'Save video',
              path: _destinationPath,
              emptyLabel: 'No save location selected',
              buttonLabel: 'Choose location',
              onPressed: _chooseDestination,
            ),
            _field('motion-materialize-title', 'Title', _title),
            Row(
              children: [
                Expanded(
                  child: _field(
                    'motion-materialize-frame-rate',
                    'Frames/second',
                    _frameRate,
                  ),
                ),
                const SizedBox(width: OrSpacing.x2),
                Expanded(
                  child: _field('motion-materialize-width', 'Width', _width),
                ),
                const SizedBox(width: OrSpacing.x2),
                Expanded(
                  child: _field('motion-materialize-height', 'Height', _height),
                ),
              ],
            ),
            if (_status != null) ...[
              const SizedBox(height: OrSpacing.x3),
              Text(
                _status!,
                key: const ValueKey('motion-materialize-status'),
                style: TextStyle(
                  color: _failed ? OrColors.warning : OrColors.textSecondary,
                  fontSize: OrType.metadataSize,
                ),
              ),
            ],
          ],
        ),
      ),
      actions: [
        TextButton(
          key: const ValueKey('motion-materialize-cancel'),
          onPressed: _busy ? null : () => Navigator.of(context).pop(),
          child: const Text('Close'),
        ),
        if (_outcome != null)
          FilledButton.tonal(
            key: const ValueKey('motion-materialize-add-to-timeline'),
            onPressed: _busy ? null : _addToTimeline,
            child: const Text('Add to timeline'),
          ),
        FilledButton(
          key: const ValueKey('motion-materialize-render'),
          onPressed:
              _busy ||
                  _sourcePath == null ||
                  _destinationPath == null ||
                  _outcome != null
              ? null
              : _submit,
          child: Text(
            _busy
                ? 'Rendering…'
                : _outcome == null
                ? 'Render'
                : 'Rendered',
          ),
        ),
      ],
    );
  }

  Widget _fileChoice({
    required String key,
    required String label,
    required String? path,
    required String emptyLabel,
    required String buttonLabel,
    required VoidCallback onPressed,
  }) {
    return Padding(
      padding: const EdgeInsets.only(bottom: OrSpacing.x2),
      child: Row(
        children: [
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(label, style: const TextStyle(fontSize: 13)),
                const SizedBox(height: OrSpacing.x1),
                Tooltip(
                  message: path ?? 'No location selected',
                  child: Text(
                    path == null ? emptyLabel : _fileName(path),
                    key: ValueKey(key),
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: const TextStyle(
                      color: OrColors.textSecondary,
                      fontSize: OrType.metadataSize,
                    ),
                  ),
                ),
              ],
            ),
          ),
          const SizedBox(width: OrSpacing.x2),
          OutlinedButton(
            onPressed: _busy ? null : onPressed,
            child: Text(buttonLabel),
          ),
        ],
      ),
    );
  }

  Widget _field(String key, String label, TextEditingController controller) {
    return Padding(
      padding: const EdgeInsets.only(bottom: OrSpacing.x2),
      child: TextField(
        key: ValueKey(key),
        controller: controller,
        enabled: !_busy,
        decoration: InputDecoration(labelText: label),
      ),
    );
  }

  String _fileName(String path) => path.replaceAll('\\', '/').split('/').last;
}
