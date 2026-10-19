import 'package:flutter/material.dart';

import '../design/or_colors.dart';
import '../design/or_spacing.dart';
import '../design/or_typography.dart';
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
  required String initialDestinationPath,
  required Future<MotionMaterializationOutcome> Function(
    MotionMaterializeRequest,
  )
  render,
}) {
  return showDialog<void>(
    context: context,
    builder: (dialogContext) => _MotionMaterializeDialog(
      initialDestinationPath: initialDestinationPath,
      render: render,
    ),
  );
}

class _MotionMaterializeDialog extends StatefulWidget {
  const _MotionMaterializeDialog({
    required this.initialDestinationPath,
    required this.render,
  });

  final String initialDestinationPath;
  final Future<MotionMaterializationOutcome> Function(MotionMaterializeRequest)
  render;

  @override
  State<_MotionMaterializeDialog> createState() =>
      _MotionMaterializeDialogState();
}

class _MotionMaterializeDialogState extends State<_MotionMaterializeDialog> {
  late final TextEditingController _source = TextEditingController();
  late final TextEditingController _destination = TextEditingController(
    text: widget.initialDestinationPath,
  );
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

  @override
  void dispose() {
    _source.dispose();
    _destination.dispose();
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

  Future<void> _submit() async {
    final source = _source.text.trim();
    if (source.isEmpty) {
      setState(() {
        _failed = true;
        _status = 'Choose a code file to illustrate.';
      });
      return;
    }
    final destination = _destination.text.trim();
    if (destination.isEmpty) {
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
    final outcome = await widget.render(
      MotionMaterializeRequest(
        sourcePath: source,
        destinationPath: destination,
        frameRate: frameRate,
        width: width,
        height: height,
        title: _title.text.trim(),
      ),
    );
    if (!mounted) return;
    setState(() {
      _busy = false;
      _failed = !outcome.succeeded;
      _status = outcome.succeeded
          ? 'Wrote ${outcome.frameCount} frames at ${outcome.width}×'
                '${outcome.height} to ${outcome.destinationPath}'
          : '${outcome.errorCode}: ${outcome.message}';
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
            _field('motion-materialize-source', 'Code file', _source),
            _field(
              'motion-materialize-destination',
              'Output video',
              _destination,
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
        FilledButton(
          key: const ValueKey('motion-materialize-render'),
          onPressed: _busy ? null : () => _submit(),
          child: Text(_busy ? 'Rendering…' : 'Render'),
        ),
      ],
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
}
