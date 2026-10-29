import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:or_app_bridge/or_app_bridge.dart';

import '../screens/media_decode.dart';
import '../design/or_colors.dart';
import '../design/or_spacing.dart';
import '../design/or_typography.dart';
import 'motion_library_controller.dart';

/// The palette the motion surfaces offer. A closed list rather than a picker so a
/// scene keeps to one look, and so every colour the editor can write is a value the
/// scene schema already stores.
const List<Color> motionSceneSwatches = [
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

/// The smallest pointer target a move handle gets, in logical pixels. A 6 px title
/// is still grabbable on a small preview without misreporting the node's real box in
/// the scene: the handle grows for the pointer, the edit keeps canvas coordinates.
const double _minimumHandle = 16;

/// The painted size of a resize grip, and the size of its hit area.
const double _gripSize = 9;
const double _gripHitSize = 18;

/// How far a pointer must travel before a background drag becomes a marquee rather
/// than a click that clears the selection.
const double _marqueeThreshold = 4;

/// A drag that always wins the gesture arena.
///
/// A handle can sit inside a scrollable panel, and a vertical pan there would
/// otherwise be claimed by the scroll view: the handle would look draggable and
/// move nothing. An immediate multi-drag recognizer claims the pointer on the down
/// event, so a drag that begins on an object is always an object move.
class ObjectDragDetector extends StatelessWidget {
  const ObjectDragDetector({
    super.key,
    required this.onStart,
    required this.onUpdate,
    required this.onEnd,
    required this.child,
    this.onTap,
  });

  /// Called with the pointer's global position when the drag begins, so a rotate
  /// gesture can measure an angle instead of only a distance.
  final ValueChanged<Offset> onStart;
  final ValueChanged<Offset> onUpdate;
  final VoidCallback onEnd;

  /// A click that never becomes a drag. The immediate drag recognizer only accepts
  /// once the pointer passes the hit slop, so a plain click needs its own
  /// recognizer or it would do nothing at all.
  final VoidCallback? onTap;

  final Widget child;

  @override
  Widget build(BuildContext context) {
    return RawGestureDetector(
      behavior: HitTestBehavior.opaque,
      gestures: {
        ImmediateMultiDragGestureRecognizer:
            GestureRecognizerFactoryWithHandlers<
              ImmediateMultiDragGestureRecognizer
            >(() => ImmediateMultiDragGestureRecognizer(debugOwner: this), (
              recognizer,
            ) {
              recognizer.onStart = (position) {
                onStart(position);
                return _ObjectDrag(
                  owner: this,
                  onUpdate: onUpdate,
                  onEnd: onEnd,
                );
              };
            }),
      },
      child: onTap == null
          ? child
          : GestureDetector(
              behavior: HitTestBehavior.opaque,
              onTap: onTap,
              child: child,
            ),
    );
  }
}

class _ObjectDrag implements Drag {
  _ObjectDrag({
    required this.owner,
    required this.onUpdate,
    required this.onEnd,
  });

  final Object owner;
  final ValueChanged<Offset> onUpdate;
  final VoidCallback onEnd;
  bool _finished = false;

  @override
  void update(DragUpdateDetails details) => onUpdate(details.delta);

  @override
  void end(DragEndDetails details) {
    if (_finished) return;
    _finished = true;
    onEnd();
  }

  @override
  void cancel() {
    if (_finished) return;
    _finished = true;
    onEnd();
  }
}

/// What a drag on the canvas is doing.
enum _DragMode { none, move, resize, rotate, marquee }

/// The interactive composition: the rendered still with a handle per node, a
/// marquee for a multi-selection, corner grips that resize a shape and a grip that
/// rotates it. A gesture is committed once, on release, so the bridge validates and
/// re-renders one edit rather than one per pointer sample.
class MotionSceneCanvas extends StatefulWidget {
  const MotionSceneCanvas({
    super.key,
    required this.controller,
    this.emptyLabel = 'Preview appears here',
    this.maxHeight = 160,
    this.scope = 'motion',
  });

  final MotionLibraryController controller;
  final String emptyLabel;

  /// How tall the canvas may grow. The panel is a column beside the timeline, so
  /// the editor keeps the picture within reach instead of pushing the controls off
  /// the bottom of the sidebar.
  final double maxHeight;

  /// The key namespace for this canvas. The panel and the expanded editor can be
  /// on screen at the same time, so each gets its own keys rather than duplicating
  /// `motion-node-*` in the tree.
  final String scope;

  @override
  State<MotionSceneCanvas> createState() => _MotionSceneCanvasState();
}

class _MotionSceneCanvasState extends State<MotionSceneCanvas> {
  final GlobalKey _surfaceKey = GlobalKey();

  _DragMode _mode = _DragMode.none;
  Offset _dragDelta = Offset.zero;
  Offset _startPointer = Offset.zero;
  Offset _marqueeCurrent = Offset.zero;
  Map<String, ({double x, double y})> _dragOrigins = const {};
  int _corner = 0;
  MotionSceneNodeView? _dragNode;
  Rect _resizeStart = Rect.zero;
  double _rotateStartDegrees = 0;

  MotionLibraryController get controller => widget.controller;

  /// Groups draw nothing, so they have no handle: their children carry their own
  /// anchors and the layer list still offers the group for deletion.
  bool _hasHandle(MotionSceneNodeView node) => node.kind != 'group';

  /// The pointer position in canvas coordinates, or null before the first frame.
  Offset? _pointerInCanvas(Offset globalPosition) {
    final box = _surfaceKey.currentContext?.findRenderObject() as RenderBox?;
    if (box == null || !box.hasSize || box.size.width <= 0) return null;
    final local = box.globalToLocal(globalPosition);
    final canvas = controller.canvasSize;
    final scale = box.size.width / canvas.width;
    return local / scale;
  }

  double _scaleOf() {
    final box = _surfaceKey.currentContext?.findRenderObject() as RenderBox?;
    if (box == null || !box.hasSize || box.size.width <= 0) return 1;
    return box.size.width / controller.canvasSize.width;
  }

  @override
  Widget build(BuildContext context) {
    final canvas = controller.canvasSize;
    final preview = controller.draftPreview;
    return LayoutBuilder(
      builder: (context, constraints) {
        var width = constraints.maxWidth;
        if (!width.isFinite || width <= 0) return const SizedBox.shrink();
        var height = width * canvas.height / canvas.width;
        if (height > widget.maxHeight) {
          height = widget.maxHeight;
          width = height * canvas.width / canvas.height;
        }
        final scale = width / canvas.width;
        return SizedBox(
          width: width,
          height: height,
          child: DecoratedBox(
            key: _surfaceKey,
            decoration: BoxDecoration(
              color: OrColors.backgroundRecessed,
              borderRadius: BorderRadius.circular(OrRadii.card),
              border: Border.all(color: OrColors.border),
            ),
            child: ClipRRect(
              borderRadius: BorderRadius.circular(OrRadii.card),
              child: Stack(
                clipBehavior: Clip.none,
                children: [
                  if (preview != null)
                    Positioned.fill(
                      child: IgnorePointer(
                        child: Image(
                          image: previewImageProvider(
                            preview,
                            logicalWidth: width,
                            logicalHeight: height,
                            devicePixelRatio:
                                MediaQuery.devicePixelRatioOf(context),
                          ),
                          fit: BoxFit.fill,
                          gaplessPlayback: true,
                        ),
                      ),
                    )
                  else
                    Positioned.fill(
                      child: IgnorePointer(
                        child: Center(
                          child: Text(
                            widget.emptyLabel,
                            style: const TextStyle(
                              color: OrColors.textMuted,
                              fontSize: OrType.tinySize,
                            ),
                          ),
                        ),
                      ),
                    ),
                  // The surface under the objects: a drag here draws a marquee, a
                  // click clears the selection.
                  Positioned.fill(
                    child: ObjectDragDetector(
                      key: ValueKey('${widget.scope}-scene-surface'),
                      onStart: _marqueeStart,
                      onUpdate: _marqueeUpdate,
                      onEnd: _marqueeEnd,
                      onTap: _surfaceTap,
                      child: const ColoredBox(color: Color(0x00000000)),
                    ),
                  ),
                  for (final node in controller.sceneNodes)
                    if (_hasHandle(node)) _objectHandle(node, scale),
                  ..._selectionChrome(scale),
                  if (_mode == _DragMode.marquee)
                    _marqueeOverlay(constraints.maxWidth),
                ],
              ),
            ),
          ),
        );
      },
    );
  }

  // -- object handles ---------------------------------------------------

  Widget _objectHandle(MotionSceneNodeView node, double scale) {
    final selected = controller.isNodeSelected(node.nodeId);
    final moving = _mode == _DragMode.move && selected;
    final left = node.x * scale + (moving ? _dragDelta.dx : 0);
    final top = node.y * scale + (moving ? _dragDelta.dy : 0);
    final width = math.max(node.width * scale, _minimumHandle);
    final height = math.max(node.height * scale, _minimumHandle);
    return Positioned(
      left: left,
      top: top,
      width: width,
      height: height,
      child: Transform.rotate(
        angle: _screenAngle(node),
        child: MouseRegion(
          cursor: SystemMouseCursors.move,
          child: Semantics(
            label: '${_kindLabel(node.kind)} object',
            selected: selected,
            child: ObjectDragDetector(
              key: ValueKey('${widget.scope}-node-${node.nodeId}'),
              onStart: (position) => _moveStart(node, position),
              onUpdate: (delta) => setState(() => _dragDelta += delta),
              onEnd: _moveEnd,
              onTap: () => _selectOnTap(node),
              child: DecoratedBox(
                decoration: BoxDecoration(
                  color: selected
                      ? OrColors.selection.withValues(alpha: 0.10)
                      : null,
                  border: Border.all(
                    color: selected ? OrColors.selection : OrColors.textMuted,
                    width: selected ? 1.5 : 1,
                  ),
                  borderRadius: BorderRadius.circular(2),
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }

  void _selectOnTap(MotionSceneNodeView node) {
    if (HardwareKeyboard.instance.isShiftPressed) {
      controller.toggleNodeSelection(node.nodeId);
    } else {
      controller.selectNode(node.nodeId);
    }
  }

  void _surfaceTap() {
    if (!HardwareKeyboard.instance.isShiftPressed) {
      controller.clearNodeSelection();
    }
  }

  void _moveStart(MotionSceneNodeView node, Offset globalPosition) {
    final pointer = _pointerInCanvas(globalPosition);
    if (pointer == null) return;
    _startPointer = pointer;
    final shift = HardwareKeyboard.instance.isShiftPressed;
    if (!controller.isNodeSelected(node.nodeId)) {
      if (shift) {
        controller.toggleNodeSelection(node.nodeId);
      } else {
        controller.selectNode(node.nodeId);
      }
    }
    setState(() {
      _mode = _DragMode.move;
      _dragDelta = Offset.zero;
      _dragOrigins = {
        for (final selected in controller.selectedNodes)
          selected.nodeId: (x: selected.x, y: selected.y),
      };
    });
  }

  void _moveEnd() {
    final delta = _dragDelta;
    final origins = _dragOrigins;
    setState(() {
      _mode = _DragMode.none;
      _dragDelta = Offset.zero;
      _dragOrigins = const {};
    });
    if (delta == Offset.zero || origins.isEmpty) return;
    final scale = _scaleOf();
    if (scale <= 0) return;
    unawaited(
      controller.moveNodes({
        for (final entry in origins.entries)
          entry.key: (
            x: entry.value.x + delta.dx / scale,
            y: entry.value.y + delta.dy / scale,
          ),
      }),
    );
  }

  // -- resize and rotate ------------------------------------------------

  List<Widget> _selectionChrome(double scale) {
    final nodes = controller.selectedNodes;
    if (nodes.isEmpty) return const [];
    final chrome = <Widget>[];
    final single = nodes.length == 1 ? nodes.single : null;
    if (single != null) {
      if (single.resizable) chrome.addAll(_resizeGrips(single, scale));
      if (single.width > 0 || single.height > 0) {
        chrome.add(_rotateGrip(single, scale));
      }
    }
    return chrome;
  }

  /// The four corner grips, placed on the rotated box so they follow the object.
  List<Widget> _resizeGrips(MotionSceneNodeView node, double scale) {
    final width = math.max(node.width * scale, _minimumHandle);
    final height = math.max(node.height * scale, _minimumHandle);
    final left = node.x * scale;
    final top = node.y * scale;
    final corners = <Offset>[
      Offset(left + width, top + height),
      Offset(left, top + height),
      Offset(left, top),
      Offset(left + width, top),
    ];
    final labels = ['bottom right', 'bottom left', 'top left', 'top right'];
    return [
      for (var index = 0; index < corners.length; index++)
        Positioned(
          left: corners[index].dx - _gripHitSize / 2,
          top: corners[index].dy - _gripHitSize / 2,
          width: _gripHitSize,
          height: _gripHitSize,
          child: MouseRegion(
            cursor: index.isEven
                ? SystemMouseCursors.resizeUpLeftDownRight
                : SystemMouseCursors.resizeUpRightDownLeft,
            child: ObjectDragDetector(
              key: ValueKey('${widget.scope}-resize-${node.nodeId}-$index'),
              onStart: (position) => _resizeStartDrag(node, index, position),
              onUpdate: (delta) => setState(() => _dragDelta += delta),
              onEnd: _resizeEnd,
              child: Center(
                child: Semantics(
                  label: 'Resize ${labels[index]}',
                  child: Container(
                    width: _gripSize,
                    height: _gripSize,
                    decoration: BoxDecoration(
                      color: OrColors.selection,
                      border: Border.all(color: OrColors.background, width: 1),
                      borderRadius: BorderRadius.circular(1),
                    ),
                  ),
                ),
              ),
            ),
          ),
        ),
    ];
  }

  void _resizeStartDrag(
    MotionSceneNodeView node,
    int corner,
    Offset globalPosition,
  ) {
    final pointer = _pointerInCanvas(globalPosition);
    if (pointer == null) return;
    _startPointer = pointer;
    setState(() {
      _mode = _DragMode.resize;
      _dragNode = node;
      _resizeStart = Rect.fromLTWH(node.x, node.y, node.width, node.height);
      _corner = corner;
      _dragDelta = Offset.zero;
    });
  }

  void _resizeEnd() {
    final node = _dragNode;
    final start = _resizeStart;
    final delta = _dragDelta;
    final corner = _corner;
    setState(() {
      _mode = _DragMode.none;
      _dragNode = null;
      _dragDelta = Offset.zero;
    });
    if (node == null || delta == Offset.zero) return;
    final scale = _scaleOf();
    if (scale <= 0) return;
    final local = _rotateOffset(delta / scale, -_screenAngle(node));
    // Which way the dragged corner points, so the opposite corner can stay fixed.
    final signX = corner == 0 || corner == 3 ? 1.0 : -1.0;
    final signY = corner == 0 || corner == 1 ? 1.0 : -1.0;
    final width = math.max(8.0, start.width + signX * local.dx);
    final height = math.max(8.0, start.height + signY * local.dy);
    // The renderer rotates around the box centre, so keeping the opposite corner
    // still means moving the centre by the rotated change in that corner's offset.
    final fixedBefore = Offset(
      -signX * start.width / 2,
      -signY * start.height / 2,
    );
    final fixedAfter = Offset(-signX * width / 2, -signY * height / 2);
    final centre =
        start.center +
        _rotateOffset(fixedBefore - fixedAfter, _screenAngle(node));
    final position = centre - Offset(width / 2, height / 2);
    unawaited(
      controller.resizeNode(
        node.nodeId,
        x: position.dx,
        y: position.dy,
        width: width,
        height: height,
      ),
    );
  }

  Widget _rotateGrip(MotionSceneNodeView node, double scale) {
    final width = math.max(node.width * scale, _minimumHandle);
    final left = node.x * scale;
    final top = node.y * scale;
    return Positioned(
      left: left + width / 2 - _gripHitSize / 2,
      top: top - _gripHitSize - 6,
      width: _gripHitSize,
      height: _gripHitSize,
      child: MouseRegion(
        cursor: SystemMouseCursors.grab,
        child: ObjectDragDetector(
          key: ValueKey('${widget.scope}-rotate-${node.nodeId}'),
          onStart: (position) => _rotateStartDrag(node, position),
          onUpdate: (delta) => setState(() => _dragDelta += delta),
          onEnd: _rotateEnd,
          child: Center(
            child: Semantics(
              label: 'Rotate object',
              child: Container(
                width: _gripSize + 2,
                height: _gripSize + 2,
                decoration: BoxDecoration(
                  color: OrColors.background,
                  border: Border.all(color: OrColors.selection, width: 1.5),
                  shape: BoxShape.circle,
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }

  void _rotateStartDrag(MotionSceneNodeView node, Offset globalPosition) {
    final pointer = _pointerInCanvas(globalPosition);
    if (pointer == null) return;
    _startPointer = pointer;
    setState(() {
      _mode = _DragMode.rotate;
      _dragNode = node;
      _rotateStartDegrees = node.rotationDegrees;
      _dragDelta = Offset.zero;
    });
  }

  void _rotateEnd() {
    final node = _dragNode;
    final delta = _dragDelta;
    setState(() {
      _mode = _DragMode.none;
      _dragNode = null;
      _dragDelta = Offset.zero;
    });
    if (node == null || delta == Offset.zero) return;
    final scale = _scaleOf();
    if (scale <= 0) return;
    final centre = Offset(node.x + node.width / 2, node.y + node.height / 2);
    final startVector = _startPointer - centre;
    final endVector = startVector + delta / scale;
    final turn =
        math.atan2(endVector.dy, endVector.dx) -
        math.atan2(startVector.dy, startVector.dx);
    final degrees = _rotateStartDegrees - turn * 180 / math.pi;
    unawaited(controller.rotateNode(node.nodeId, normalizeDegrees(degrees)));
  }

  // -- marquee ----------------------------------------------------------

  void _marqueeStart(Offset globalPosition) {
    final pointer = _pointerInCanvas(globalPosition);
    if (pointer == null) return;
    _startPointer = pointer;
    setState(() {
      _mode = _DragMode.marquee;
      _dragDelta = Offset.zero;
      _marqueeCurrent = pointer;
    });
  }

  void _marqueeUpdate(Offset delta) {
    final scale = _scaleOf();
    setState(() {
      _dragDelta += delta;
      _marqueeCurrent = _startPointer + delta / (scale == 0 ? 1 : scale);
    });
  }

  void _marqueeEnd() {
    final moved = _dragDelta;
    final start = _startPointer;
    setState(() {
      _mode = _DragMode.none;
      _dragDelta = Offset.zero;
    });
    final scale = _scaleOf();
    if (scale <= 0) return;
    final shift = HardwareKeyboard.instance.isShiftPressed;
    if (moved.distance / scale < _marqueeThreshold) {
      if (!shift) controller.clearNodeSelection();
      return;
    }
    final current = _marqueeCurrent;
    final rect = Rect.fromPoints(start, current);
    final hits = [
      for (final node in controller.sceneNodes)
        if (_hasHandle(node) &&
            rect.overlaps(
              Rect.fromLTWH(
                node.x,
                node.y,
                math.max(node.width, 1),
                math.max(node.height, 1),
              ),
            ))
          node.nodeId,
    ];
    controller.selectNodes(hits, additive: shift);
  }

  Widget _marqueeOverlay(double surfaceWidth) {
    final rect = Rect.fromPoints(_startPointer, _marqueeCurrent);
    return Positioned.fromRect(
      rect: rect,
      child: IgnorePointer(
        child: DecoratedBox(
          decoration: BoxDecoration(
            color: OrColors.selection.withValues(alpha: 0.10),
            border: Border.all(color: OrColors.selection),
          ),
        ),
      ),
    );
  }
}

/// The controls for the selection: text, type size, colour, outline, position and
/// deletion — or the bulk actions when more than one object is selected.
class MotionSceneInspector extends StatefulWidget {
  const MotionSceneInspector({
    super.key,
    required this.controller,
    this.compact = false,
    this.scope = 'motion',
  });

  final MotionLibraryController controller;

  /// The panel's copy drops the position row: the canvas is the position control
  /// there, and the rail has no room for both.
  final bool compact;

  /// The key namespace for this inspector, so the panel and the expanded editor
  /// never publish the same key twice.
  final String scope;

  @override
  State<MotionSceneInspector> createState() => _MotionSceneInspectorState();
}

class _MotionSceneInspectorState extends State<MotionSceneInspector> {
  final _text = TextEditingController();
  String? _boundNodeId;

  MotionLibraryController get controller => widget.controller;

  @override
  void initState() {
    super.initState();
    _syncText();
  }

  @override
  void didUpdateWidget(covariant MotionSceneInspector oldWidget) {
    super.didUpdateWidget(oldWidget);
    _syncText();
  }

  /// Binds the field to a node id, never to a fresh value: rewriting the field on
  /// every controller notification would fight the keyboard while a debounced edit
  /// is in flight.
  void _syncText() {
    final node = controller.selectedNode;
    if (node == null || node.nodeId != _boundNodeId) {
      _boundNodeId = node?.nodeId;
      _text.text = node?.text ?? '';
    }
  }

  @override
  void dispose() {
    _text.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final nodes = controller.selectedNodes;
    if (nodes.isEmpty) {
      return Padding(
        padding: const EdgeInsets.symmetric(vertical: OrSpacing.x1),
        child: Text(
          'Select an object on the canvas to edit it.',
          style: const TextStyle(
            color: OrColors.textMuted,
            fontSize: OrType.tinySize,
          ),
        ),
      );
    }
    if (nodes.length > 1) return _bulkInspector(nodes);
    return _singleInspector(nodes.single);
  }

  Widget _singleInspector(MotionSceneNodeView node) {
    final isText = node.kind == 'text';
    return Column(
      key: ValueKey('${widget.scope}-node-inspector'),
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Row(
          children: [
            Icon(_kindIcon(node.kind), size: 14, color: OrColors.textSecondary),
            const SizedBox(width: OrSpacing.x1),
            Expanded(
              child: Text(
                _kindLabel(node.kind),
                overflow: TextOverflow.ellipsis,
                style: const TextStyle(
                  color: OrColors.text,
                  fontSize: OrType.metadataSize,
                  fontWeight: FontWeight.w600,
                ),
              ),
            ),
            if (node.animated)
              const Padding(
                padding: EdgeInsets.only(right: OrSpacing.x1),
                child: Icon(Icons.bolt, size: 10, color: OrColors.textMuted),
              ),
            IconButton(
              key: ValueKey('${widget.scope}-node-delete'),
              iconSize: 16,
              tooltip: 'Delete object',
              onPressed: () => unawaited(controller.deleteNode(node.nodeId)),
              icon: const Icon(Icons.delete_outline),
            ),
          ],
        ),
        if (isText) ...[
          TextField(
            key: ValueKey('${widget.scope}-node-text'),
            controller: _text,
            style: const TextStyle(
              color: OrColors.text,
              fontSize: OrType.metadataSize,
            ),
            decoration: const InputDecoration(isDense: true, labelText: 'Text'),
            onChanged: (value) => controller.queueNodeText(node.nodeId, value),
            onSubmitted: (value) =>
                unawaited(controller.setNodeText(node.nodeId, value)),
          ),
          const SizedBox(height: OrSpacing.x1),
          _fontSizeRow(node),
        ],
        const SizedBox(height: OrSpacing.x1),
        _colorRow(node),
        if (node.strokeable) ...[
          const SizedBox(height: OrSpacing.x1),
          _outlineRow(node),
        ],
        if (!widget.compact) ...[
          const SizedBox(height: OrSpacing.x1),
          _positionRow(node),
        ],
      ],
    );
  }

  Widget _bulkInspector(List<MotionSceneNodeView> nodes) {
    final drawable = nodes.where((node) => node.kind != 'group').length;
    final strokeable = nodes.where((node) => node.strokeable).length;
    return Column(
      key: ValueKey('${widget.scope}-bulk-inspector'),
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Row(
          children: [
            const Icon(Icons.select_all, size: 14, color: OrColors.selection),
            const SizedBox(width: OrSpacing.x1),
            Expanded(
              child: Text(
                '${nodes.length} objects selected',
                key: ValueKey('${widget.scope}-selection-count'),
                style: const TextStyle(
                  color: OrColors.text,
                  fontSize: OrType.metadataSize,
                  fontWeight: FontWeight.w600,
                ),
              ),
            ),
            IconButton(
              key: ValueKey('${widget.scope}-selection-delete'),
              iconSize: 16,
              tooltip: 'Delete selection',
              onPressed: () => unawaited(controller.deleteSelectedNodes()),
              icon: const Icon(Icons.delete_sweep_outlined),
            ),
          ],
        ),
        if (drawable > 0) ...[
          const Text(
            'Colour',
            style: TextStyle(
              color: OrColors.textMuted,
              fontSize: OrType.tinySize,
            ),
          ),
          Wrap(
            spacing: OrSpacing.x1,
            runSpacing: OrSpacing.x1,
            children: [
              for (final swatch in motionSceneSwatches) _bulkSwatch(swatch),
            ],
          ),
        ],
        if (strokeable > 0) ...[
          const SizedBox(height: OrSpacing.x1),
          Row(
            children: [
              const Text(
                'Outline',
                style: TextStyle(
                  color: OrColors.textMuted,
                  fontSize: OrType.tinySize,
                ),
              ),
              const SizedBox(width: OrSpacing.x1),
              _outlineButton('none', null),
              for (final swatch in motionSceneSwatches.take(6))
                _outlineButton(_swatchName(swatch), swatch),
            ],
          ),
        ],
      ],
    );
  }

  Widget _fontSizeRow(MotionSceneNodeView node) {
    final size = node.fontSize <= 0 ? 32.0 : node.fontSize;
    return Row(
      children: [
        const Text(
          'Size',
          style: TextStyle(
            color: OrColors.textMuted,
            fontSize: OrType.tinySize,
          ),
        ),
        const SizedBox(width: OrSpacing.x2),
        Expanded(
          child: Text(
            '${size.round()} px',
            style: const TextStyle(
              color: OrColors.text,
              fontSize: OrType.tinySize,
            ),
          ),
        ),
        IconButton(
          key: ValueKey('${widget.scope}-node-size-down'),
          iconSize: 14,
          visualDensity: VisualDensity.compact,
          tooltip: 'Smaller type',
          onPressed: () => unawaited(
            controller.setNodeFontSize(node.nodeId, math.max(8, size - 8)),
          ),
          icon: const Icon(Icons.remove),
        ),
        IconButton(
          key: ValueKey('${widget.scope}-node-size-up'),
          iconSize: 14,
          visualDensity: VisualDensity.compact,
          tooltip: 'Larger type',
          onPressed: () => unawaited(
            controller.setNodeFontSize(node.nodeId, math.min(400, size + 8)),
          ),
          icon: const Icon(Icons.add),
        ),
      ],
    );
  }

  Widget _colorRow(MotionSceneNodeView node) {
    return Wrap(
      spacing: OrSpacing.x1,
      runSpacing: OrSpacing.x1,
      children: [
        for (final swatch in motionSceneSwatches)
          GestureDetector(
            key: ValueKey(
              '${widget.scope}-node-swatch-${(swatch.r * 255).round()}-'
              '${(swatch.g * 255).round()}-${(swatch.b * 255).round()}',
            ),
            onTap: () => unawaited(
              controller.setNodeColor(
                node.nodeId,
                red: (swatch.r * 255).round(),
                green: (swatch.g * 255).round(),
                blue: (swatch.b * 255).round(),
              ),
            ),
            child: Container(
              width: 18,
              height: 18,
              decoration: BoxDecoration(
                color: swatch,
                borderRadius: BorderRadius.circular(OrRadii.small),
                border: Border.all(
                  color:
                      node.hasFill &&
                          _sameRgb(
                            swatch,
                            node.fillRed,
                            node.fillGreen,
                            node.fillBlue,
                          )
                      ? OrColors.selection
                      : OrColors.border,
                  width: 2,
                ),
              ),
            ),
          ),
      ],
    );
  }

  Widget _outlineRow(MotionSceneNodeView node) {
    return Row(
      children: [
        const Text(
          'Outline',
          style: TextStyle(
            color: OrColors.textMuted,
            fontSize: OrType.tinySize,
          ),
        ),
        const SizedBox(width: OrSpacing.x1),
        _outlineButton('none', null, selected: !node.hasStroke),
        for (final swatch in motionSceneSwatches.take(6))
          _outlineButton(
            _swatchName(swatch),
            swatch,
            selected:
                node.hasStroke &&
                _sameRgb(
                  swatch,
                  node.strokeRed,
                  node.strokeGreen,
                  node.strokeBlue,
                ),
          ),
      ],
    );
  }

  Widget _outlineButton(String name, Color? color, {bool selected = false}) {
    return Padding(
      padding: const EdgeInsets.only(right: OrSpacing.x1),
      child: GestureDetector(
        key: ValueKey('${widget.scope}-node-outline-$name'),
        onTap: () {
          if (color == null) {
            unawaited(controller.clearSelectionStroke());
            return;
          }
          unawaited(
            controller.setSelectionStroke(
              red: (color.r * 255).round(),
              green: (color.g * 255).round(),
              blue: (color.b * 255).round(),
            ),
          );
        },
        child: Container(
          width: 18,
          height: 18,
          decoration: BoxDecoration(
            color: color ?? OrColors.background,
            borderRadius: BorderRadius.circular(OrRadii.small),
            border: Border.all(
              color: selected ? OrColors.selection : OrColors.border,
              width: selected ? 2 : 1,
            ),
          ),
          child: color == null
              ? const Icon(Icons.block, size: 10, color: OrColors.textMuted)
              : null,
        ),
      ),
    );
  }

  Widget _bulkSwatch(Color swatch) {
    return GestureDetector(
      key: ValueKey(
        '${widget.scope}-bulk-swatch-${(swatch.r * 255).round()}-'
        '${(swatch.g * 255).round()}-${(swatch.b * 255).round()}',
      ),
      onTap: () => unawaited(
        controller.setSelectionColor(
          red: (swatch.r * 255).round(),
          green: (swatch.g * 255).round(),
          blue: (swatch.b * 255).round(),
        ),
      ),
      child: Container(
        width: 18,
        height: 18,
        decoration: BoxDecoration(
          color: swatch,
          borderRadius: BorderRadius.circular(OrRadii.small),
          border: Border.all(color: OrColors.border),
        ),
      ),
    );
  }

  Widget _positionRow(MotionSceneNodeView node) {
    return Row(
      children: [
        const Text(
          'Position',
          style: TextStyle(
            color: OrColors.textMuted,
            fontSize: OrType.tinySize,
          ),
        ),
        const SizedBox(width: OrSpacing.x2),
        Expanded(
          child: Text(
            '${node.x.round()}, ${node.y.round()}',
            style: const TextStyle(
              color: OrColors.text,
              fontSize: OrType.tinySize,
            ),
          ),
        ),
        _nudgeButton(node, 'left', Icons.chevron_left, -8, 0),
        _nudgeButton(node, 'up', Icons.expand_less, 0, -8),
        _nudgeButton(node, 'down', Icons.expand_more, 0, 8),
        _nudgeButton(node, 'right', Icons.chevron_right, 8, 0),
      ],
    );
  }

  Widget _nudgeButton(
    MotionSceneNodeView node,
    String name,
    IconData icon,
    double dx,
    double dy,
  ) => IconButton(
    key: ValueKey('${widget.scope}-node-nudge-$name'),
    iconSize: 14,
    visualDensity: VisualDensity.compact,
    tooltip: 'Nudge $name',
    onPressed: () =>
        unawaited(controller.moveNode(node.nodeId, node.x + dx, node.y + dy)),
    icon: Icon(icon),
  );
}

/// The node list: one row per object, in document order, with the same selection the
/// canvas uses. A checkbox toggles an object into the selection, so a hidden object
/// can be reached without hunting for it on the canvas.
class MotionSceneLayerList extends StatelessWidget {
  const MotionSceneLayerList({
    super.key,
    required this.controller,
    this.maxRows = 8,
    this.scope = 'motion',
  });

  final MotionLibraryController controller;
  final int maxRows;
  final String scope;

  @override
  Widget build(BuildContext context) {
    final nodes = controller.sceneNodes;
    if (nodes.isEmpty) {
      return Text(
        'No objects yet. Add text or pick an asset.',
        style: const TextStyle(
          color: OrColors.textMuted,
          fontSize: OrType.tinySize,
        ),
      );
    }
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        for (final node in nodes.take(maxRows)) _row(node),
        if (nodes.length > maxRows)
          Padding(
            padding: const EdgeInsets.only(top: OrSpacing.x1),
            child: Text(
              'and ${nodes.length - maxRows} more',
              style: const TextStyle(
                color: OrColors.textMuted,
                fontSize: OrType.tinySize,
              ),
            ),
          ),
      ],
    );
  }

  Widget _row(MotionSceneNodeView node) {
    final selected = controller.isNodeSelected(node.nodeId);
    final label = node.kind == 'text' && node.text.isNotEmpty
        ? node.text
        : _kindLabel(node.kind);
    return Padding(
      padding: EdgeInsets.only(left: node.depth * OrSpacing.x2),
      child: InkWell(
        key: ValueKey('$scope-layer-${node.nodeId}'),
        onTap: () => HardwareKeyboard.instance.isShiftPressed
            ? controller.toggleNodeSelection(node.nodeId)
            : controller.selectNode(node.nodeId),
        child: Container(
          padding: const EdgeInsets.symmetric(
            horizontal: OrSpacing.x1,
            vertical: 3,
          ),
          decoration: BoxDecoration(
            color: selected
                ? OrColors.selection.withValues(alpha: 0.16)
                : Colors.transparent,
            borderRadius: BorderRadius.circular(OrRadii.small),
          ),
          child: Row(
            children: [
              InkWell(
                key: ValueKey('$scope-layer-check-${node.nodeId}'),
                onTap: () => controller.toggleNodeSelection(node.nodeId),
                child: Icon(
                  selected
                      ? Icons.check_box_outlined
                      : Icons.check_box_outline_blank,
                  size: 14,
                  color: selected ? OrColors.selection : OrColors.textMuted,
                ),
              ),
              const SizedBox(width: OrSpacing.x1),
              Icon(
                _kindIcon(node.kind),
                size: 13,
                color: selected ? OrColors.selection : OrColors.textMuted,
              ),
              const SizedBox(width: OrSpacing.x1),
              Expanded(
                child: Text(
                  label,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: TextStyle(
                    color: selected ? OrColors.text : OrColors.textSecondary,
                    fontSize: OrType.tinySize,
                  ),
                ),
              ),
              if (node.animated)
                const Icon(Icons.bolt, size: 10, color: OrColors.textMuted),
              IconButton(
                key: ValueKey('$scope-layer-delete-${node.nodeId}'),
                iconSize: 13,
                visualDensity: VisualDensity.compact,
                padding: EdgeInsets.zero,
                constraints: const BoxConstraints(minWidth: 22, minHeight: 22),
                tooltip: 'Delete $label',
                onPressed: () => unawaited(controller.deleteNode(node.nodeId)),
                icon: const Icon(Icons.close),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

/// Opens the library so a card can be appended to the composition.
Future<void> showAddAssetSheet(
  BuildContext context,
  MotionLibraryController controller,
) async {
  final entries = controller.entries;
  final assetId = await showDialog<String>(
    context: context,
    builder: (context) => AlertDialog(
      backgroundColor: OrColors.surface,
      title: const Text('Add an asset to the scene'),
      content: SizedBox(
        width: 420,
        height: 420,
        child: entries.isEmpty
            ? const Align(
                alignment: Alignment.topLeft,
                child: Text(
                  'The motion library is empty. Import a PNG or save a scene first.',
                  style: TextStyle(
                    color: OrColors.textMuted,
                    fontSize: OrType.metadataSize,
                  ),
                ),
              )
            : ListView.builder(
                itemCount: entries.length,
                itemBuilder: (context, index) {
                  final entry = entries[index];
                  // Keyed by position: a name can repeat (importing the same PNG
                  // twice keeps its name), and a duplicate key is not allowed
                  // among siblings.
                  return ListTile(
                    key: ValueKey('motion-add-asset-$index'),
                    dense: true,
                    title: Text(
                      entry.name,
                      style: const TextStyle(
                        color: OrColors.text,
                        fontSize: OrType.metadataSize,
                      ),
                    ),
                    subtitle: Text(
                      '${entry.category} · ${entry.kind}',
                      style: const TextStyle(
                        color: OrColors.textMuted,
                        fontSize: OrType.tinySize,
                      ),
                    ),
                    onTap: () => Navigator.of(context).pop(entry.assetId),
                  );
                },
              ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('Cancel'),
        ),
      ],
    ),
  );
  if (assetId == null) return;
  await controller.addAssetFromLibrary(assetId);
}

/// The expanded editor: a large canvas next to the inspector and the layer list.
Future<void> showMotionSceneEditor(
  BuildContext context,
  MotionLibraryController controller,
) => showDialog<void>(
  context: context,
  builder: (context) => _MotionSceneEditorDialog(controller: controller),
);

class _MotionSceneEditorDialog extends StatefulWidget {
  const _MotionSceneEditorDialog({required this.controller});

  final MotionLibraryController controller;

  @override
  State<_MotionSceneEditorDialog> createState() =>
      _MotionSceneEditorDialogState();
}

class _MotionSceneEditorDialogState extends State<_MotionSceneEditorDialog> {
  final FocusNode _focus = FocusNode(debugLabel: 'motion-scene-editor');

  MotionLibraryController get controller => widget.controller;

  @override
  void dispose() {
    _focus.dispose();
    super.dispose();
  }

  KeyEventResult _onKey(FocusNode node, KeyEvent event) {
    if (event is! KeyDownEvent && event is! KeyRepeatEvent) {
      return KeyEventResult.ignored;
    }
    final keys = HardwareKeyboard.instance;
    final command = keys.isMetaPressed || keys.isControlPressed;
    if (command && event.logicalKey == LogicalKeyboardKey.keyZ) {
      keys.isShiftPressed ? controller.redoScene() : controller.undoScene();
      return KeyEventResult.handled;
    }
    if (command && event.logicalKey == LogicalKeyboardKey.keyY) {
      controller.redoScene();
      return KeyEventResult.handled;
    }
    if (event.logicalKey == LogicalKeyboardKey.delete ||
        event.logicalKey == LogicalKeyboardKey.backspace) {
      if (controller.hasNodeSelection) {
        unawaited(controller.deleteSelectedNodes());
        return KeyEventResult.handled;
      }
    }
    if (event.logicalKey == LogicalKeyboardKey.escape) {
      Navigator.of(context).maybePop();
      return KeyEventResult.handled;
    }
    return KeyEventResult.ignored;
  }

  @override
  Widget build(BuildContext context) {
    return Dialog(
      backgroundColor: OrColors.backgroundRaised,
      insetPadding: const EdgeInsets.all(OrSpacing.x4),
      child: Focus(
        focusNode: _focus,
        autofocus: true,
        onKeyEvent: _onKey,
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 1180, maxHeight: 760),
          child: AnimatedBuilder(
            animation: controller,
            builder: (context, _) => Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                Padding(
                  padding: const EdgeInsets.fromLTRB(
                    OrSpacing.x4,
                    OrSpacing.x3,
                    OrSpacing.x2,
                    OrSpacing.x2,
                  ),
                  child: Row(
                    children: [
                      const Expanded(
                        child: Text(
                          'Edit scene',
                          style: TextStyle(
                            color: OrColors.text,
                            fontSize: OrType.sectionTitleSize,
                            fontWeight: FontWeight.w600,
                          ),
                        ),
                      ),
                      IconButton(
                        key: const ValueKey('motion-editor-undo'),
                        iconSize: 18,
                        tooltip: 'Undo scene edit',
                        onPressed: controller.canUndo
                            ? controller.undoScene
                            : null,
                        icon: const Icon(Icons.undo),
                      ),
                      IconButton(
                        key: const ValueKey('motion-editor-redo'),
                        iconSize: 18,
                        tooltip: 'Redo scene edit',
                        onPressed: controller.canRedo
                            ? controller.redoScene
                            : null,
                        icon: const Icon(Icons.redo),
                      ),
                      IconButton(
                        key: const ValueKey('motion-editor-close'),
                        iconSize: 18,
                        tooltip: 'Close',
                        onPressed: () => Navigator.of(context).pop(),
                        icon: const Icon(Icons.close),
                      ),
                    ],
                  ),
                ),
                Flexible(
                  child: Padding(
                    padding: const EdgeInsets.fromLTRB(
                      OrSpacing.x4,
                      0,
                      OrSpacing.x4,
                      OrSpacing.x4,
                    ),
                    child: Row(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Expanded(
                          child: Column(
                            crossAxisAlignment: CrossAxisAlignment.stretch,
                            children: [
                              MotionSceneCanvas(
                                controller: controller,
                                maxHeight: 520,
                                scope: 'motion-editor',
                              ),
                              const SizedBox(height: OrSpacing.x2),
                              Row(
                                children: [
                                  OutlinedButton.icon(
                                    key: const ValueKey(
                                      'motion-editor-add-text',
                                    ),
                                    onPressed: controller.busy
                                        ? null
                                        : () => unawaited(
                                            controller.addTextNode(),
                                          ),
                                    icon: const Icon(Icons.title, size: 16),
                                    label: const Text('Add text'),
                                  ),
                                  const SizedBox(width: OrSpacing.x2),
                                  OutlinedButton.icon(
                                    key: const ValueKey(
                                      'motion-editor-add-asset',
                                    ),
                                    onPressed: controller.busy
                                        ? null
                                        : () => unawaited(
                                            showAddAssetSheet(
                                              context,
                                              controller,
                                            ),
                                          ),
                                    icon: const Icon(
                                      Icons.add_photo_alternate_outlined,
                                      size: 16,
                                    ),
                                    label: const Text('Add from library'),
                                  ),
                                  const Spacer(),
                                  IconButton(
                                    key: const ValueKey(
                                      'motion-editor-select-all',
                                    ),
                                    iconSize: 18,
                                    tooltip: 'Select every object',
                                    onPressed: controller.sceneNodes.isEmpty
                                        ? null
                                        : controller.selectAllNodes,
                                    icon: const Icon(Icons.select_all),
                                  ),
                                ],
                              ),
                            ],
                          ),
                        ),
                        const SizedBox(width: OrSpacing.x3),
                        SizedBox(
                          width: 260,
                          child: SingleChildScrollView(
                            child: Column(
                              crossAxisAlignment: CrossAxisAlignment.stretch,
                              children: [
                                MotionSceneInspector(
                                  controller: controller,
                                  scope: 'motion-editor',
                                ),
                                const Divider(height: OrSpacing.x4),
                                Text(
                                  'Layers (${controller.sceneNodes.length})',
                                  style: const TextStyle(
                                    color: OrColors.textMuted,
                                    fontSize: OrType.tinySize,
                                  ),
                                ),
                                const SizedBox(height: OrSpacing.x1),
                                MotionSceneLayerList(
                                  controller: controller,
                                  maxRows: 24,
                                  scope: 'motion-editor',
                                ),
                              ],
                            ),
                          ),
                        ),
                      ],
                    ),
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

/// Rotates an offset by an angle in radians, in screen coordinates.
Offset _rotateOffset(Offset value, double radians) {
  final cos = math.cos(radians);
  final sin = math.sin(radians);
  return Offset(
    value.dx * cos - value.dy * sin,
    value.dx * sin + value.dy * cos,
  );
}

/// The clockwise screen angle the renderer's counter-clockwise degrees describe.
double _screenAngle(MotionSceneNodeView node) =>
    -node.rotationDegrees * math.pi / 180;

/// Wraps an angle into `(-180, 180]`, matching the scene's rotation rule.
double normalizeDegrees(double degrees) {
  final wrapped = degrees % 360;
  final positive = wrapped < 0 ? wrapped + 360 : wrapped;
  return positive > 180 ? positive - 360 : positive;
}

bool _sameRgb(Color swatch, int red, int green, int blue) =>
    (swatch.r * 255).round() == red &&
    (swatch.g * 255).round() == green &&
    (swatch.b * 255).round() == blue;

String _swatchName(Color swatch) =>
    '${(swatch.r * 255).round()}-${(swatch.g * 255).round()}-'
    '${(swatch.b * 255).round()}';

IconData _kindIcon(String kind) => switch (kind) {
  'text' => Icons.title,
  'rectangle' => Icons.crop_square,
  'ellipse' => Icons.circle_outlined,
  'arrow' => Icons.arrow_right_alt,
  'image' => Icons.image_outlined,
  'group' => Icons.folder_outlined,
  _ => Icons.category_outlined,
};

String _kindLabel(String kind) => switch (kind) {
  'text' => 'Text',
  'rectangle' => 'Rectangle',
  'ellipse' => 'Ellipse',
  'arrow' => 'Arrow',
  'image' => 'Image',
  'group' => 'Group',
  _ => 'Object',
};
