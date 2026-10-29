import 'package:flutter/material.dart';

import '../design/or_colors.dart';
import '../design/or_spacing.dart';
import '../design/or_typography.dart';

/// Ruler, grid and placeholder painting for the timeline.
///
/// These have no project state: they take the pixel geometry the timeline
/// computes and draw it. Keeping them here means the timeline panel file holds
/// interaction and layout instead of canvas code.

String _timelineTimeLabel(double seconds) {
  final value = BigInt.tryParse(seconds.toStringAsFixed(0)) ?? BigInt.zero;
  final hours = value ~/ BigInt.from(3600);
  final minutes = (value % BigInt.from(3600)) ~/ BigInt.from(60);
  final remainder = value % BigInt.from(60);
  final mm = minutes.toString().padLeft(2, '0');
  final ss = remainder.toString().padLeft(2, '0');
  return hours == BigInt.zero
      ? '$mm:$ss'
      : '${hours.toString().padLeft(2, '0')}:$mm:$ss';
}

class TimelineRulerPainter extends CustomPainter {
  const TimelineRulerPainter({
    required this.pixelsPerSecond,
    required this.tickSeconds,
    this.playheadX,
  });

  final double pixelsPerSecond;
  final double tickSeconds;

  /// Where the playhead sits, so a tick label that would sit under its cap
  /// yields instead of showing a half covered word.
  final double? playheadX;

  @override
  void paint(Canvas canvas, Size size) {
    canvas.drawRect(Offset.zero & size, Paint()..color = OrColors.surface);
    final line = Paint()
      ..color = OrColors.border
      ..strokeWidth = 1;
    final textStyle = const TextStyle(
      color: OrColors.textMuted,
      fontFamily: 'monospace',
      fontSize: OrType.tinySize,
    );
    for (var index = 0; index <= 1000; index++) {
      final seconds = index * tickSeconds;
      final x = seconds * pixelsPerSecond;
      if (!x.isFinite || x > size.width) break;
      canvas.drawLine(Offset(x, size.height - 8), Offset(x, size.height), line);
      final label = _timelineTimeLabel(seconds);
      final painter = TextPainter(
        text: TextSpan(text: label, style: textStyle),
        textDirection: TextDirection.ltr,
      )..layout();
      final labelLeft = x + 4;
      final playhead = playheadX;
      final hidden =
          playhead != null &&
          labelLeft + painter.width > playhead - 7 &&
          labelLeft < playhead + 7;
      if (!hidden) painter.paint(canvas, Offset(labelLeft, 2));
    }
    canvas.drawLine(
      Offset(0, size.height - 0.5),
      Offset(size.width, size.height - 0.5),
      line,
    );
  }

  @override
  bool shouldRepaint(TimelineRulerPainter oldDelegate) =>
      oldDelegate.pixelsPerSecond != pixelsPerSecond ||
      oldDelegate.tickSeconds != tickSeconds ||
      oldDelegate.playheadX != playheadX;
}

/// The grab notch over the ruler's playhead.
///
/// A two pixel line states a position; the notch states that the line has a
/// draggable end, which is what the pointer cursor alone cannot say.
class TimelinePlayheadHandlePainter extends CustomPainter {
  const TimelinePlayheadHandlePainter({required this.color, required this.apex});

  final Color color;

  /// Horizontal position of the cap's point in painter coordinates.
  final double apex;

  @override
  void paint(Canvas canvas, Size size) {
    final path = Path()
      ..moveTo(0, 0)
      ..lineTo(size.width, 0)
      ..lineTo(apex, size.height)
      ..close();
    canvas.drawPath(path, Paint()..color = color);
  }

  @override
  bool shouldRepaint(TimelinePlayheadHandlePainter oldDelegate) =>
      oldDelegate.color != color || oldDelegate.apex != apex;
}

class TimelineGridPainter extends CustomPainter {
  const TimelineGridPainter({
    required this.pixelsPerSecond,
    required this.tickSeconds,
  });

  final double pixelsPerSecond;
  final double tickSeconds;

  @override
  void paint(Canvas canvas, Size size) {
    final line = Paint()
      ..color = OrColors.border.withValues(alpha: 0.65)
      ..strokeWidth = 1;
    for (var index = 0; index <= 1000; index++) {
      final x = index * tickSeconds * pixelsPerSecond;
      if (!x.isFinite || x > size.width) break;
      canvas.drawLine(Offset(x, 0), Offset(x, size.height), line);
    }
    canvas.drawLine(
      Offset(0, size.height - 0.5),
      Offset(size.width, size.height - 0.5),
      Paint()
        ..color = OrColors.border
        ..strokeWidth = 1,
    );
  }

  @override
  bool shouldRepaint(TimelineGridPainter oldDelegate) =>
      oldDelegate.pixelsPerSecond != pixelsPerSecond ||
      oldDelegate.tickSeconds != tickSeconds;
}

class TimelineEmptyRuler extends StatelessWidget {
  const TimelineEmptyRuler({super.key, required this.compact});

  final bool compact;

  @override
  Widget build(BuildContext context) {
    final marks = compact
        ? const ['00:00', '00:05', '00:10']
        : const ['00:00', '00:05', '00:10', '00:15', '00:20'];

    return Container(
      height: 26,
      decoration: const BoxDecoration(
        color: OrColors.surface,
        border: Border(bottom: BorderSide(color: OrColors.border)),
      ),
      child: Row(
        children: [
          const SizedBox(width: 48),
          for (final mark in marks)
            Expanded(
              child: Container(
                alignment: Alignment.centerLeft,
                padding: const EdgeInsets.only(left: OrSpacing.x2),
                decoration: const BoxDecoration(
                  border: Border(left: BorderSide(color: OrColors.border)),
                ),
                child: Text(
                  mark,
                  style: const TextStyle(
                    color: OrColors.textMuted,
                    fontFamily: 'monospace',
                    fontSize: OrType.tinySize,
                  ),
                ),
              ),
            ),
        ],
      ),
    );
  }
}

class TimelineEmptyLane extends StatelessWidget {
  const TimelineEmptyLane({super.key});

  @override
  Widget build(BuildContext context) {
    return Semantics(
      label: 'Empty timeline lane placeholder',
      child: Row(
        children: [
          const SizedBox(
            width: 48,
            child: Center(
              child: Text('—', style: TextStyle(color: OrColors.textMuted)),
            ),
          ),
          Expanded(
            child: Row(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                for (var i = 0; i < 5; i++)
                  Expanded(
                    child: Container(
                      decoration: const BoxDecoration(
                        border: Border(
                          left: BorderSide(color: OrColors.border),
                        ),
                      ),
                    ),
                  ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}
