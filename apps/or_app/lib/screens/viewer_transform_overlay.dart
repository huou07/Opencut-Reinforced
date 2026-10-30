import 'dart:math' as math;

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../design/or_colors.dart';
import '../design/or_spacing.dart';
import '../project/project_gateway.dart';

/// Direct manipulation of the selected clip inside the viewer.
///
/// The box shows where the compositor will draw the clip: the canvas rect
/// scaled and rotated by the clip transform. Dragging the interior moves it,
/// dragging a corner scales it, and dragging the handle above the top edge
/// rotates it. Every gesture ends with a single committed settings value, so a
/// drag produces one command rather than one command per pointer sample.
class ViewerTransformOverlay extends StatefulWidget {
  const ViewerTransformOverlay({
    super.key,
    required this.canvasSize,
    required this.settings,
    required this.onCommit,
    this.onPreview,
    this.enabled = true,
  });

  /// Sequence resolution. The overlay itself is laid over the drawn frame, so
  /// this is the unit the stored transform is expressed in.
  final Size canvasSize;
  final ProjectTimelineVisualSettings settings;

  /// Called once per gesture, on release.
  final ValueChanged<ProjectTimelineVisualSettings> onCommit;

  /// Called during the gesture so the box follows the pointer without sending
  /// commands; a host that ignores it still gets the committed value.
  final ValueChanged<ProjectTimelineVisualSettings>? onPreview;
  final bool enabled;

  @override
  State<ViewerTransformOverlay> createState() => _ViewerTransformOverlayState();
}

enum _DragKind { move, scale, rotate }

class _ViewerTransformOverlayState extends State<ViewerTransformOverlay> {
  static const double _handleRadius = 6;

  /// How close, in display pixels, the box edge must be to the canvas centre
  /// before it snaps. Eight pixels is about one text line of travel.
  static const double _snapDistance = 8;

  _DragKind? _drag;
  ProjectTimelineVisualSettings? _draft;
  Offset? _dragStart;
  ProjectTimelineVisualSettings? _dragStartSettings;
  double _startAngle = 0;
  bool _snappedX = false;
  bool _snappedY = false;

  ProjectTimelineVisualSettings get _current => _draft ?? widget.settings;

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        final display = Size(constraints.maxWidth, constraints.maxHeight);
        final box = transformBox(display, _current);
        final centre = Offset(display.width / 2, display.height / 2);
        final rotated = _current.rotationMilliDegrees / 1000 * math.pi / 180;
        return Stack(
          clipBehavior: Clip.none,
          children: [
            // Every interactive child stays mounted while `enabled` is false.
            // A rebuild that removed them mid-gesture disposed the recognizer
            // and a drag died after its first sample; the overlay only has to
            // ignore input, not disappear.
            if (_snappedX)
              _guide(left: centre.dx, top: 0, width: 1, height: display.height),
            if (_snappedY)
              _guide(left: 0, top: centre.dy, width: display.width, height: 1),
            // The frame outline and its own centre cross.
            Positioned.fromRect(
              rect: box,
              child: IgnorePointer(
                child: Transform.rotate(
                  angle: rotated,
                  child: DecoratedBox(
                    decoration: BoxDecoration(
                      border: Border.all(color: OrColors.selection, width: 1),
                    ),
                    child: Center(
                      child: SizedBox(
                        width: 10,
                        height: 10,
                        child: CustomPaint(painter: _CrossPainter()),
                      ),
                    ),
                  ),
                ),
              ),
            ),
            _interior(display, box, centre, rotated),
            for (final (index, corner) in _corners(
              box,
              rotated,
              display,
            ).indexed)
              _handle(
                handleKey: const [
                  'viewer-transform-handle-tl',
                  'viewer-transform-handle-tr',
                  'viewer-transform-handle-bl',
                  'viewer-transform-handle-br',
                ][index],
                centre: corner,
                cursor: corner.dx < centre.dx
                    ? (corner.dy < centre.dy
                          ? SystemMouseCursors.resizeUpLeft
                          : SystemMouseCursors.resizeDownLeft)
                    : (corner.dy < centre.dy
                          ? SystemMouseCursors.resizeUpRight
                          : SystemMouseCursors.resizeDownRight),
                kind: _DragKind.scale,
              ),
            _handle(
              handleKey: 'viewer-transform-handle-rotate',
              centre: _rotateHandleCentre(box, rotated, display),
              cursor: SystemMouseCursors.grab,
              kind: _DragKind.rotate,
              rotate: true,
            ),
          ],
        );
      },
    );
  }

  Widget _guide({
    required double left,
    required double top,
    required double width,
    required double height,
  }) => Positioned(
    left: left,
    top: top,
    width: width,
    height: height,
    child: IgnorePointer(
      child: ColoredBox(color: OrColors.selection.withValues(alpha: 0.55)),
    ),
  );

  Widget _handle({
    required Offset centre,
    required MouseCursor cursor,
    required _DragKind kind,
    bool rotate = false,
    String? handleKey,
  }) => Positioned(
    key: handleKey == null ? null : ValueKey(handleKey),
    left: centre.dx - (rotate ? _handleRadius + 2 : _handleRadius),
    top: centre.dy - (rotate ? _handleRadius + 2 : _handleRadius),
    width: rotate ? (_handleRadius + 2) * 2 : _handleRadius * 2,
    height: rotate ? (_handleRadius + 2) * 2 : _handleRadius * 2,
    child: MouseRegion(
      cursor: cursor,
      child: GestureDetector(
        behavior: HitTestBehavior.opaque,
        dragStartBehavior: DragStartBehavior.down,
        onPanStart: (details) => _begin(kind, details.globalPosition),
        onPanUpdate: _update,
        onPanEnd: (_) => _finish(),
        onPanCancel: _finish,
        child: DecoratedBox(
          decoration: BoxDecoration(
            color: rotate ? OrColors.surface : OrColors.background,
            shape: rotate ? BoxShape.circle : BoxShape.rectangle,
            border: Border.all(color: OrColors.selection, width: 1.5),
            borderRadius: rotate
                ? null
                : BorderRadius.circular(OrRadii.small / 2),
          ),
          child: rotate
              ? const Icon(
                  Icons.rotate_right,
                  size: 11,
                  color: OrColors.selection,
                )
              : null,
        ),
      ),
    ),
  );

  Widget _interior(Size display, Rect box, Offset centre, double rotation) =>
      Positioned.fromRect(
        rect: box,
        child: MouseRegion(
          cursor: SystemMouseCursors.move,
          child: GestureDetector(
            behavior: HitTestBehavior.translucent,
            dragStartBehavior: DragStartBehavior.down,
            onPanStart: (details) =>
                _begin(_DragKind.move, details.globalPosition),
            onPanUpdate: _update,
            onPanEnd: (_) => _finish(),
            child: const SizedBox.expand(),
          ),
        ),
      );

  /// A pan reported from the slop-crossing point would lose the first ~36 px of
  /// the gesture, so every detector starts the drag at the pointer-down point
  /// and the box follows the hand from where it landed.
  void _begin(_DragKind kind, Offset global) {
    if (!widget.enabled) return;
    setState(() {
      _drag = kind;
      _dragStart = global;
      _dragStartSettings = widget.settings;
      _draft = widget.settings;
      _snappedX = false;
      _snappedY = false;
      final box = transformBox(_displaySize(), widget.settings);
      _startAngle = math.atan2(
        global.dy - _displaySize().height / 2,
        global.dx - _displaySize().width / 2,
      );
      if (kind == _DragKind.rotate && box.isEmpty) _startAngle = 0;
    });
  }

  Size _displaySize() {
    final renderBox = context.findRenderObject() as RenderBox?;
    return renderBox?.size ?? Size.zero;
  }

  void _update(DragUpdateDetails details) {
    final start = _dragStart;
    final base = _dragStartSettings;
    if (start == null || base == null) return;
    final display = _displaySize();
    final offset = details.globalPosition - start;
    final shift = HardwareKeyboard.instance.isShiftPressed;
    switch (_drag) {
      case _DragKind.move:
        final next = _move(base, offset, display, bypassSnap: shift);
        _publish(next);
      case _DragKind.scale:
        final next = _scale(base, offset, display);
        _publish(next);
      case _DragKind.rotate:
        final next = _rotate(base, details.globalPosition, display, shift);
        _publish(next);
      case null:
        break;
    }
  }

  ProjectTimelineVisualSettings _move(
    ProjectTimelineVisualSettings base,
    Offset offset,
    Size display, {
    required bool bypassSnap,
  }) {
    var x = base.xMilliCanvas + offset.dx / display.width * 1000;
    var y = base.yMilliCanvas - offset.dy / display.height * 1000;
    var snappedX = false;
    var snappedY = false;
    if (!bypassSnap) {
      final thresholdX = _snapDistance / display.width * 1000;
      final thresholdY = _snapDistance / display.height * 1000;
      if (x.abs() <= thresholdX) {
        x = 0;
        snappedX = true;
      }
      if (y.abs() <= thresholdY) {
        y = 0;
        snappedY = true;
      }
    }
    _snappedX = snappedX;
    _snappedY = snappedY;
    return withTransform(base, xMilliCanvas: x, yMilliCanvas: y);
  }

  ProjectTimelineVisualSettings _scale(
    ProjectTimelineVisualSettings base,
    Offset offset,
    Size display,
  ) {
    final box = transformBox(display, base);
    final halfWidth = math.max(box.width / 2, 1);
    final halfHeight = math.max(box.height / 2, 1);
    // A corner drag grows the box by how far the pointer moved away from the
    // centre along either axis; the larger axis wins so the gesture feels the
    // same on every corner.
    final factor = math.max(
      (halfWidth + offset.dx.abs()) / halfWidth,
      (halfHeight + offset.dy.abs()) / halfHeight,
    );
    final scaleX = (base.scaleXMilli * factor).clamp(1.0, 8000.0);
    final scaleY = (base.scaleYMilli * factor).clamp(1.0, 8000.0);
    return withTransform(base, scaleXMilli: scaleX, scaleYMilli: scaleY);
  }

  ProjectTimelineVisualSettings _rotate(
    ProjectTimelineVisualSettings base,
    Offset global,
    Size display,
    bool snap,
  ) {
    final centre = Offset(display.width / 2, display.height / 2);
    final angle = math.atan2(global.dy - centre.dy, global.dx - centre.dx);
    var degrees =
        base.rotationMilliDegrees / 1000 +
        (angle - _startAngle) * 180 / math.pi;
    if (snap) degrees = (degrees / 15).round() * 15;
    final milli = (degrees * 1000).clamp(-360000.0, 360000.0);
    return withTransform(base, rotationMilliDegrees: milli);
  }

  void _publish(ProjectTimelineVisualSettings next) {
    setState(() => _draft = next);
    widget.onPreview?.call(next);
  }

  void _finish() {
    final committed = _draft;
    setState(() {
      _drag = null;
      _draft = null;
      _dragStart = null;
      _dragStartSettings = null;
      _snappedX = false;
      _snappedY = false;
    });
    if (committed != null && widget.enabled) widget.onCommit(committed);
  }

  /// Handles sit just inside the box instead of centred on its edge: the box
  /// at identity fills the frame exactly, and a handle on the boundary is half
  /// outside the stage, where the clip region makes it impossible to grab.
  List<Offset> _corners(Rect box, double rotation, Size display) {
    final centre = box.center;
    return [box.topLeft, box.topRight, box.bottomLeft, box.bottomRight]
        .map((corner) => _rotatePoint(corner, centre, rotation))
        .map((corner) => _inset(corner, centre, _handleRadius + 4))
        // A clip larger than the frame puts its corners outside the stage,
        // where the clip region makes them unreachable. Clamping keeps every
        // corner grabbable even when the box overflows.
        .map((corner) => _clampTo(display, corner))
        .toList();
  }

  Offset _rotateHandleCentre(Rect box, double rotation, Size display) {
    final centre = box.center;
    final top = Offset(box.center.dx, box.top + _handleRadius + 8);
    return _clampTo(display, _rotatePoint(top, centre, rotation));
  }

  Offset _clampTo(Size display, Offset point) {
    final inset = _handleRadius + 2;
    return Offset(
      point.dx.clamp(inset, math.max(inset, display.width - inset)),
      point.dy.clamp(inset, math.max(inset, display.height - inset)),
    );
  }

  Offset _inset(Offset point, Offset towards, double distance) {
    final delta = towards - point;
    final length = delta.distance;
    if (length <= distance) return point;
    return point + delta / length * distance;
  }

  Offset _rotatePoint(Offset point, Offset centre, double rotation) {
    final dx = point.dx - centre.dx;
    final dy = point.dy - centre.dy;
    final cos = math.cos(rotation);
    final sin = math.sin(rotation);
    return Offset(
      centre.dx + dx * cos - dy * sin,
      centre.dy + dx * sin + dy * cos,
    );
  }
}

class _CrossPainter extends CustomPainter {
  @override
  void paint(Canvas canvas, Size size) {
    final paint = Paint()
      ..color = OrColors.selection.withValues(alpha: 0.9)
      ..strokeWidth = 1;
    canvas.drawLine(Offset.zero, Offset(size.width, size.height), paint);
    canvas.drawLine(Offset(size.width, 0), Offset(0, size.height), paint);
  }

  @override
  bool shouldRepaint(covariant _CrossPainter oldDelegate) => false;
}

/// Where the clip box lands inside the drawn frame.
///
/// The renderer treats `x_milli_canvas = 1000` as one full canvas width and
/// draws `y` upward, so the vertical axis is inverted here as well.
Rect transformBox(Size display, ProjectTimelineVisualSettings settings) {
  final width = display.width * settings.scaleXMilli / 1000;
  final height = display.height * settings.scaleYMilli / 1000;
  final centre = Offset(
    display.width / 2 + settings.xMilliCanvas / 1000 * display.width,
    display.height / 2 - settings.yMilliCanvas / 1000 * display.height,
  );
  return Rect.fromCenter(center: centre, width: width, height: height);
}

/// A visual settings value with only the transform fields replaced.
ProjectTimelineVisualSettings withTransform(
  ProjectTimelineVisualSettings base, {
  double? xMilliCanvas,
  double? yMilliCanvas,
  double? scaleXMilli,
  double? scaleYMilli,
  double? rotationMilliDegrees,
}) => ProjectTimelineVisualSettings(
  xMilliCanvas: (xMilliCanvas ?? base.xMilliCanvas).round(),
  yMilliCanvas: (yMilliCanvas ?? base.yMilliCanvas).round(),
  scaleXMilli: (scaleXMilli ?? base.scaleXMilli).round(),
  scaleYMilli: (scaleYMilli ?? base.scaleYMilli).round(),
  rotationMilliDegrees: (rotationMilliDegrees ?? base.rotationMilliDegrees)
      .round(),
  anchorXBasisPoints: base.anchorXBasisPoints,
  anchorYBasisPoints: base.anchorYBasisPoints,
  cropLeftBasisPoints: base.cropLeftBasisPoints,
  cropTopBasisPoints: base.cropTopBasisPoints,
  cropRightBasisPoints: base.cropRightBasisPoints,
  cropBottomBasisPoints: base.cropBottomBasisPoints,
  opacityBasisPoints: base.opacityBasisPoints,
  brightnessAmountMilli: base.brightnessAmountMilli,
  contrastAmountMilli: base.contrastAmountMilli,
  saturationAmountMilli: base.saturationAmountMilli,
  gaussianBlurRadiusMilli: base.gaussianBlurRadiusMilli,
  transitionIn: base.transitionIn,
  transitionInDuration: base.transitionInDuration,
  transitionOut: base.transitionOut,
  transitionOutDuration: base.transitionOutDuration,
  modifiedEffects: base.modifiedEffects,
  updateTransitionIn: base.updateTransitionIn,
  updateTransitionOut: base.updateTransitionOut,
);
