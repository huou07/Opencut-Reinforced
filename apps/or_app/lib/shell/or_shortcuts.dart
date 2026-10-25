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
///
/// EditableText's focus node is attached to a descendant Focus widget, so the
/// primary focus context is not itself an EditableText.
bool orTextFieldHasFocus() {
  final context = FocusManager.instance.primaryFocus?.context;
  if (context == null) return false;

  final widget = context.widget;
  return widget is EditableText ||
      widget is SelectableText ||
      context.findAncestorWidgetOfExactType<EditableText>() != null ||
      context.findAncestorWidgetOfExactType<SelectableText>() != null;
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

class SplitClipIntent extends Intent {
  const SplitClipIntent();
}

class RippleDeleteClipIntent extends Intent {
  const RippleDeleteClipIntent();
}

class AddMarkerIntent extends Intent {
  const AddMarkerIntent();
}

class ExportProjectIntent extends Intent {
  const ExportProjectIntent();
}

/// Selects one of the editor's tool categories. The number keys and the tab
/// strip therefore drive the same state.
class SelectToolIntent extends Intent {
  const SelectToolIntent(this.tool);

  final String tool;
}

/// Moves the selected clip inside the frame by a fixed step. The pointer
/// produces the same value through the viewer handles, so a keyboard user and
/// a pointer user change the same field.
class NudgeSelectedClipIntent extends Intent {
  const NudgeSelectedClipIntent(this.dxMilliCanvas, this.dyMilliCanvas);

  final double dxMilliCanvas;
  final double dyMilliCanvas;
}

/// Blade mode is a mode, not a command: `C` keeps the pointer cutting until
/// `V` returns it to selection.
class SetBladeModeIntent extends Intent {
  const SetBladeModeIntent(this.enabled);

  final bool enabled;
}

/// Copy the selected clip into the editor's clipboard.
class CopyClipIntent extends Intent {
  const CopyClipIntent();
}

/// Paste the clipboard at the playhead on the selected track.
class PasteClipIntent extends Intent {
  const PasteClipIntent();
}
