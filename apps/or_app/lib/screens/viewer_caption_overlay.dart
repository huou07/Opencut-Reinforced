import 'dart:math' as math;

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';

import '../design/or_colors.dart';
import '../project/project_gateway.dart';

/// Direct manipulation of the selected caption or title inside the viewer.
///
/// The box follows the same anchor and offset the Rust compositor uses, so the
/// outline sits over the rendered text. Dragging the interior writes the two
/// offset fields; dragging the corner handle writes the font size. One gesture
/// commits one formatting value, never one command per pointer sample.
class ViewerCaptionOverlay extends StatefulWidget {
  const ViewerCaptionOverlay({
    super.key,
    required this.canvasSize,
    required this.clip,
    required this.onCommit,
    this.enabled = true,
  });

  /// Sequence resolution, so stored milli-points map to the drawn frame.
  final Size canvasSize;
  final ProjectTimelineClip clip;
  final ValueChanged<ProjectTextFormatting> onCommit;
  final bool enabled;

  @override
  State<ViewerCaptionOverlay> createState() => _ViewerCaptionOverlayState();
}

class _ViewerCaptionOverlayState extends State<ViewerCaptionOverlay> {
  static const double _handle = 12;
  static const double _minSizeMilli = 4000;
  static const double _maxSizeMilli = 256000;

  ProjectTextFormatting? _draft;
  ProjectTextFormatting? _startFormatting;
  Offset? _startPointer;
  bool _resizing = false;

  ProjectTextFormatting get _formatting =>
      _draft ?? widget.clip.formatting ?? ProjectTextFormatting.captionDefaults;

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        final display = Size(constraints.maxWidth, constraints.maxHeight);
        final box = _box(display, _formatting);
        return Stack(
          clipBehavior: Clip.none,
          children: [
            Positioned.fromRect(
              rect: box,
              child: IgnorePointer(
                child: DecoratedBox(
                  decoration: BoxDecoration(
                    border: Border.all(color: OrColors.selection, width: 1),
                  ),
                ),
              ),
            ),
            Positioned.fromRect(
              rect: box,
              child: MouseRegion(
                cursor: SystemMouseCursors.move,
                child: GestureDetector(
                  key: const ValueKey('viewer-caption-move'),
                  behavior: HitTestBehavior.translucent,
                  dragStartBehavior: DragStartBehavior.down,
                  onPanStart: (details) =>
                      _begin(details.globalPosition, resizing: false),
                  onPanUpdate: (details) =>
                      _update(details.globalPosition, display),
                  onPanEnd: (_) => _finish(),
                  onPanCancel: _finish,
                  child: const SizedBox.expand(),
                ),
              ),
            ),
            Positioned(
              key: const ValueKey('viewer-caption-resize'),
              left: box.right - _handle / 2,
              top: box.bottom - _handle / 2,
              width: _handle,
              height: _handle,
              child: MouseRegion(
                cursor: SystemMouseCursors.resizeDownRight,
                child: GestureDetector(
                  behavior: HitTestBehavior.opaque,
                  dragStartBehavior: DragStartBehavior.down,
                  onPanStart: (details) =>
                      _begin(details.globalPosition, resizing: true),
                  onPanUpdate: (details) =>
                      _update(details.globalPosition, display),
                  onPanEnd: (_) => _finish(),
                  onPanCancel: _finish,
                  child: DecoratedBox(
                    decoration: BoxDecoration(
                      color: OrColors.background,
                      border: Border.all(color: OrColors.selection, width: 1.5),
                      borderRadius: BorderRadius.circular(2),
                    ),
                  ),
                ),
              ),
            ),
          ],
        );
      },
    );
  }

  Rect _box(Size display, ProjectTextFormatting formatting) {
    final canvasWidth = math.max(widget.canvasSize.width, 1);
    final scale = display.width / canvasWidth;
    final fontPx = formatting.sizeMilliPoints / 1000 * (4 / 3) * scale;
    final painter = TextPainter(
      text: TextSpan(
        text: widget.clip.text ?? '',
        style: TextStyle(
          color: Colors.white,
          fontSize: fontPx,
          fontFamily: _family(formatting.font),
          fontWeight: _weight(formatting.weight),
        ),
      ),
      textAlign: _align(formatting.alignment),
      textDirection: TextDirection.ltr,
      maxLines: null,
    )..layout(maxWidth: math.max(display.width * 0.9, 1));
    final textSize = painter.size;
    final textWidth = math.max(textSize.width, 24.0);
    final textHeight = math.max(textSize.height, fontPx * 1.2);
    final marginY = display.height * 0.05;
    final top = switch (formatting.anchor) {
      ProjectTextVerticalAnchor.top => marginY,
      ProjectTextVerticalAnchor.center => (display.height - textHeight) / 2,
      ProjectTextVerticalAnchor.bottom => display.height - marginY - textHeight,
    };
    final left = (display.width - textWidth) / 2;
    final shiftX = display.width * formatting.offsetXBasisPoints / 10000;
    final shiftY = display.height * formatting.offsetYBasisPoints / 10000;
    return Rect.fromLTWH(
      (left + shiftX).clamp(0, math.max(display.width - textWidth, 0)),
      (top + shiftY).clamp(0, math.max(display.height - textHeight, 0)),
      textWidth,
      textHeight,
    );
  }

  void _begin(Offset global, {required bool resizing}) {
    if (!widget.enabled) return;
    setState(() {
      _startPointer = global;
      _startFormatting = _formatting;
      _draft = _formatting;
      _resizing = resizing;
    });
  }

  void _update(Offset global, Size display) {
    if (!widget.enabled || _startPointer == null) return;
    final start = _startFormatting;
    if (start == null) return;
    final delta = global - _startPointer!;
    if (_resizing) {
      final canvasWidth = math.max(widget.canvasSize.width, 1);
      final scale = display.width / canvasWidth;
      final points = delta.dy / (scale * (4 / 3));
      final size = (start.sizeMilliPoints + points * 1000).clamp(
        _minSizeMilli,
        _maxSizeMilli,
      );
      setState(() => _draft = start.copyWith(sizeMilliPoints: size.round()));
      return;
    }
    final dx = (offsetDelta(delta.dx, display.width) + start.offsetXBasisPoints)
        .clamp(-10000, 10000);
    final dy =
        (offsetDelta(delta.dy, display.height) + start.offsetYBasisPoints)
            .clamp(-10000, 10000);
    setState(() {
      _draft = start.copyWith(
        offsetXBasisPoints: dx.round(),
        offsetYBasisPoints: dy.round(),
      );
    });
  }

  static int offsetDelta(double pixels, double span) =>
      span <= 0 ? 0 : (pixels / span * 10000).round();

  void _finish() {
    final next = _draft;
    setState(() {
      _draft = null;
      _startPointer = null;
      _startFormatting = null;
      _resizing = false;
    });
    if (next != null) widget.onCommit(next);
  }

  static String _family(ProjectFontIdentity font) => switch (font) {
    ProjectFontIdentity.bundledInter => 'Inter',
    ProjectFontIdentity.beVietnamPro => 'Be Vietnam Pro',
  };

  static FontWeight _weight(ProjectTextWeight weight) => switch (weight) {
    ProjectTextWeight.regular => FontWeight.w400,
    ProjectTextWeight.medium => FontWeight.w500,
    ProjectTextWeight.semibold => FontWeight.w600,
    ProjectTextWeight.bold => FontWeight.w700,
  };

  static TextAlign _align(ProjectTextAlignment alignment) =>
      switch (alignment) {
        ProjectTextAlignment.start => TextAlign.left,
        ProjectTextAlignment.center => TextAlign.center,
        ProjectTextAlignment.end => TextAlign.right,
      };
}
