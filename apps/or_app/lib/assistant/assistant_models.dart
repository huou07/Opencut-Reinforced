import 'dart:ui' show Color;

import '../design/or_colors.dart';

/// One line in the assistant transcript.
enum AssistantEntryKind { user, assistant, thought, tool, error, note }

enum AssistantToolState { pending, running, done, failed }

class AssistantEntry {
  AssistantEntry({
    required this.id,
    required this.kind,
    this.text = '',
    this.title = '',
    this.toolName = '',
    this.toolKind = '',
    this.state = AssistantToolState.done,
    this.detail = '',
    this.expanded = false,
  });

  final String id;
  final AssistantEntryKind kind;
  String text;
  String title;
  String toolName;
  String toolKind;
  AssistantToolState state;
  String detail;
  bool expanded;

  bool get isStreamingText =>
      kind == AssistantEntryKind.assistant ||
      kind == AssistantEntryKind.thought;
}

class AssistantPermissionOption {
  const AssistantPermissionOption({
    required this.optionId,
    required this.name,
    required this.kind,
  });

  final String optionId;
  final String name;
  final String kind;

  bool get isAllow => kind.startsWith('allow');
}

class AssistantPermissionRequest {
  const AssistantPermissionRequest({
    required this.id,
    required this.title,
    required this.detail,
    required this.options,
  });

  final int id;
  final String title;
  final String detail;
  final List<AssistantPermissionOption> options;
}

class AssistantModelChoice {
  const AssistantModelChoice({
    required this.value,
    required this.name,
    this.group = '',
    this.description = '',
  });

  final String value;
  final String name;
  final String group;
  final String description;
}

/// What the editor is showing, sent along with a prompt so the agent can act
/// on "this clip" without guessing.
class AssistantEditorContext {
  const AssistantEditorContext({
    this.clipId,
    this.clipLabel,
    this.trackKind,
    this.playheadSeconds,
    this.selectionLabel,
  });

  final String? clipId;
  final String? clipLabel;
  final String? trackKind;
  final double? playheadSeconds;
  final String? selectionLabel;

  bool get isEmpty => clipId == null && playheadSeconds == null;

  @override
  bool operator ==(Object other) =>
      other is AssistantEditorContext &&
      other.clipId == clipId &&
      other.clipLabel == clipLabel &&
      other.trackKind == trackKind &&
      other.playheadSeconds == playheadSeconds &&
      other.selectionLabel == selectionLabel;

  @override
  int get hashCode => Object.hash(
    clipId,
    clipLabel,
    trackKind,
    playheadSeconds,
    selectionLabel,
  );

  String toPromptAppendix() {
    final parts = <String>[];
    if (clipId != null) {
      parts.add(
        'selected clip $clipId${clipLabel == null ? '' : ' ($clipLabel)'}'
        '${trackKind == null ? '' : ' on a $trackKind track'}',
      );
    }
    if (playheadSeconds != null) {
      parts.add('playhead at ${playheadSeconds!.toStringAsFixed(3)} s');
    }
    if (parts.isEmpty) return '';
    return '\n\n[Editor context: ${parts.join('; ')}.]';
  }
}

/// How often a playhead-only move reaches the assistant panel.
///
/// The panel rebuilds on every controller notification while playback reports
/// a new playhead every frame. The status row shows hundredths of a second, so
/// a quarter second keeps "this moment" meaningful without rebuilding the
/// transcript sixty times a second.
const Duration assistantPlayheadRefreshInterval = Duration(milliseconds: 250);

/// Whether [next] says anything different from [current], ignoring a playhead
/// move that arrived inside [assistantPlayheadRefreshInterval].
bool _movedOnlyPlayhead(
  AssistantEditorContext current,
  AssistantEditorContext next,
) =>
    current.clipId == next.clipId &&
    current.clipLabel == next.clipLabel &&
    current.trackKind == next.trackKind &&
    current.selectionLabel == next.selectionLabel;

/// The context to publish now, or null when this update says nothing new.
///
/// A selection change publishes immediately, and so does any move while the
/// transport is not [streaming] (a scrub or a frame step must reach the panel
/// at once). Only a playhead-only move during playback waits until
/// [sinceLastPublish] has reached [assistantPlayheadRefreshInterval].
AssistantEditorContext? assistantContextToPublish({
  required AssistantEditorContext? current,
  required AssistantEditorContext next,
  required bool streaming,
  required Duration sinceLastPublish,
}) {
  if (current == next) return null;
  if (current != null &&
      streaming &&
      _movedOnlyPlayhead(current, next) &&
      sinceLastPublish < assistantPlayheadRefreshInterval) {
    return null;
  }
  return next;
}

/// Connection state shown in the panel header and empty states.
enum AssistantConnectionState { idle, connecting, ready, error, stopped }

class AssistantHarnessFailure implements Exception {
  const AssistantHarnessFailure(this.message, {this.hint});

  final String message;
  final String? hint;

  @override
  String toString() => message;
}

Color assistantStateColor(AssistantConnectionState state) => switch (state) {
  AssistantConnectionState.ready => OrColors.success,
  AssistantConnectionState.connecting => OrColors.ai,
  AssistantConnectionState.error => OrColors.danger,
  AssistantConnectionState.stopped => OrColors.textMuted,
  AssistantConnectionState.idle => OrColors.textMuted,
};
