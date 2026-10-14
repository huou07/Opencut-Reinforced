import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

/// A command activator that accepts either the control or the meta modifier.
///
/// The same command is "Ctrl+K" on Linux and Windows and "Cmd+K" on macOS,
/// so listing both as separate entries doubles every map and drifts whenever
/// one side changes. This matches either with a single entry.
class OrCommandActivator extends ShortcutActivator {
  const OrCommandActivator(this.key, {this.shift = false});

  final LogicalKeyboardKey key;
  final bool shift;

  @override
  bool accepts(KeyEvent event, HardwareKeyboard state) {
    if (event is! KeyDownEvent || event.logicalKey != key) return false;
    if (shift && !state.isShiftPressed) return false;
    if (!shift && state.isShiftPressed) return false;
    return state.isControlPressed || state.isMetaPressed;
  }

  /// Required by ShortcutActivator. Shortcuts resolves by calling
  /// [accepts], not by map-key identity, so no `==` override is needed.
  @override
  String debugDescribeKeys() => 'or-command(${key.keyLabel}, shift: $shift)';
}

/// Shell-level commands shared by every destination.
class OpenCommandPaletteIntent extends Intent {
  const OpenCommandPaletteIntent();
}

class SaveProjectIntent extends Intent {
  const SaveProjectIntent();
}

class UndoProjectIntent extends Intent {
  const UndoProjectIntent();
}

class RedoProjectIntent extends Intent {
  const RedoProjectIntent();
}

/// Editor transport.
///
/// These dispatch the same ProjectGateway preview calls as the pointer
/// transport controls, so a keyboard and a pointer can never diverge, and
/// they stay presentation-only.
class PlayPausePreviewIntent extends Intent {
  const PlayPausePreviewIntent();
}

class StepPreviewFrameIntent extends Intent {
  const StepPreviewFrameIntent(this.backwards);

  final bool backwards;
}

class SeekPreviewIntent extends Intent {
  const SeekPreviewIntent(this.seconds);

  final double seconds;
}

/// True when a text field holds focus, so command keys must go to the field
/// rather than to an editor command.
bool orTextFieldHasFocus(BuildContext context) {
  final focus = Focus.of(context);
  final widget = focus.context?.widget;
  return widget is EditableText || widget is SelectableText;
}

/// Timeline view and edit commands. Like the transport these are
/// presentation-only: none of them mutates canonical project state.
class ZoomTimelineIntent extends Intent {
  const ZoomTimelineIntent(this.factor);

  final double factor;
}

class FitTimelineIntent extends Intent {
  const FitTimelineIntent();
}

class DuplicateTimelineClipIntent extends Intent {
  const DuplicateTimelineClipIntent();
}

class DeleteTimelineClipIntent extends Intent {
  const DeleteTimelineClipIntent();
}

class ClearTimelineSelectionIntent extends Intent {
  const ClearTimelineSelectionIntent();
}
