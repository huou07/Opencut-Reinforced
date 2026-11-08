import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../design/or_colors.dart';
import '../design/or_spacing.dart';
import '../design/or_typography.dart';
import '../library/agent_edit_plan.dart';
import '../widgets/or_widgets.dart';

/// Where an assistant's plan comes in: paste it, or open a file.
///
/// The dialog parses before it closes, so a malformed plan stays in the box
/// with the reason beside it instead of vanishing into a snackbar. Parsing is
/// injectable so a test can drive the failure path without a real file.
class AgentPlanImportDialog extends StatefulWidget {
  const AgentPlanImportDialog({
    super.key,
    required this.media,
    required this.frameRate,
    this.brief,
    this.parse = parseAgentEditPlan,
  });

  final List<AgentPlanMedia> media;
  final int frameRate;

  /// The prompt this dialog can copy for an assistant, if the caller has one.
  final String? brief;
  final AgentEditPlan Function(String source) parse;

  @override
  State<AgentPlanImportDialog> createState() => _AgentPlanImportDialogState();
}

class _AgentPlanImportDialogState extends State<AgentPlanImportDialog> {
  final _controller = TextEditingController();
  String? _error;

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  void _review() {
    final source = _controller.text.trim();
    if (source.isEmpty) {
      setState(() => _error = 'Paste the plan JSON, or copy the brief first.');
      return;
    }
    try {
      final plan = widget.parse(source);
      Navigator.of(context).pop(plan);
    } on AgentPlanFormatException catch (error) {
      setState(() => _error = error.message);
    }
  }

  @override
  Widget build(BuildContext context) {
    final brief = widget.brief;
    return AlertDialog(
      title: const Text('Review an assistant plan'),
      content: SizedBox(
        width: OrDialogSizes.width(context, 520),
        child: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              const Text(
                'The plan is checked against this project before anything is '
                'written. You see every shot, then choose whether to add it.',
                key: ValueKey('agent-plan-import-intro'),
                style: TextStyle(
                  color: OrColors.textSecondary,
                  fontSize: OrType.metadataSize,
                ),
              ),
              const SizedBox(height: OrSpacing.x3),
              TextField(
                key: const ValueKey('agent-plan-json'),
                controller: _controller,
                minLines: 6,
                maxLines: 12,
                style: const TextStyle(
                  fontSize: OrType.metadataSize,
                  fontFamily: 'monospace',
                ),
                decoration: const InputDecoration(
                  labelText: 'Plan JSON',
                  alignLabelWithHint: true,
                  hintText: '{"schema": "or.edit-plan/1", …}',
                ),
                onChanged: (_) {
                  if (_error != null) setState(() => _error = null);
                },
              ),
              if (_error != null) ...[
                const SizedBox(height: OrSpacing.x2),
                Text(
                  _error!,
                  key: const ValueKey('agent-plan-import-error'),
                  style: const TextStyle(
                    color: OrColors.warning,
                    fontSize: OrType.metadataSize,
                  ),
                ),
              ],
              const SizedBox(height: OrSpacing.x2),
              Text(
                '${widget.media.length} media '
                '${widget.media.length == 1 ? 'item' : 'items'} in this project '
                '· ${widget.frameRate} fps',
                key: const ValueKey('agent-plan-import-context'),
                style: const TextStyle(
                  color: OrColors.textMuted,
                  fontSize: OrType.tinySize,
                ),
              ),
            ],
          ),
        ),
      ),
      actions: [
        if (brief != null)
          TextButton(
            key: const ValueKey('agent-plan-copy-brief'),
            onPressed: () async {
              await Clipboard.setData(ClipboardData(text: brief));
              if (!context.mounted) return;
              OrSnackBar.show(
                context,
                'Brief copied. Paste it into the Assistant, then paste the '
                'JSON it returns here.',
              );
            },
            child: const Text('Copy the brief'),
          ),
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('Cancel'),
        ),
        FilledButton(
          key: const ValueKey('agent-plan-import-review'),
          onPressed: _review,
          child: const Text('Review plan'),
        ),
      ],
    );
  }
}

/// The parsed plan, shot by shot, before one clip is written.
class AgentPlanReviewDialog extends StatelessWidget {
  const AgentPlanReviewDialog({
    super.key,
    required this.plan,
    required this.labels,
    required this.appendsAfter,
  });

  final AgentEditPlan plan;
  final Map<String, String> labels;
  final bool appendsAfter;

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: const Text('Add this cut to the timeline?'),
      content: SizedBox(
        width: OrDialogSizes.width(context, 460),
        child: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                plan.summary,
                key: const ValueKey('agent-plan-summary'),
                style: const TextStyle(
                  color: OrColors.text,
                  fontSize: OrType.bodySize,
                  fontWeight: FontWeight.w600,
                ),
              ),
              const SizedBox(height: OrSpacing.x1),
              Text(
                'Planned by an assistant at ${plan.frameRate} fps',
                style: const TextStyle(
                  color: OrColors.textSecondary,
                  fontSize: OrType.metadataSize,
                ),
              ),
              const SizedBox(height: OrSpacing.x3),
              for (final (index, shot) in plan.shots.indexed) ...[
                Row(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    SizedBox(
                      width: 24,
                      child: Text(
                        '${index + 1}',
                        style: const TextStyle(
                          color: OrColors.textMuted,
                          fontSize: OrType.metadataSize,
                        ),
                      ),
                    ),
                    Expanded(
                      child: Text(
                        '${labels[shot.mediaId] ?? shot.name ?? shot.mediaId} · '
                        '${_durationLabel(shot.frameCount, plan.frameRate)}',
                        style: const TextStyle(
                          color: OrColors.text,
                          fontSize: OrType.metadataSize,
                        ),
                      ),
                    ),
                  ],
                ),
                if (shot.why.isNotEmpty)
                  Padding(
                    padding: const EdgeInsets.only(
                      left: 24,
                      bottom: OrSpacing.x2,
                    ),
                    child: Text(
                      shot.why,
                      key: ValueKey('agent-plan-shot-why-$index'),
                      style: const TextStyle(
                        color: OrColors.textMuted,
                        fontSize: OrType.metadataSize,
                      ),
                    ),
                  ),
              ],
              if (plan.notes.isNotEmpty) ...[
                const SizedBox(height: OrSpacing.x1),
                for (final (index, note) in plan.notes.indexed)
                  Padding(
                    padding: const EdgeInsets.only(bottom: OrSpacing.x1),
                    child: Text(
                      note,
                      key: ValueKey('agent-plan-note-$index'),
                      style: const TextStyle(
                        color: OrColors.warning,
                        fontSize: OrType.metadataSize,
                      ),
                    ),
                  ),
              ],
              const SizedBox(height: OrSpacing.x2),
              Text(
                [
                  if (plan.title != null)
                    'A title clip "${plan.title}" goes on a text track above '
                        'the cut.',
                  if (appendsAfter)
                    'The cut is added after the clips already on the timeline.',
                  'These are ordinary clips: trim, move or delete them like any '
                      'other. Each insert is its own step, so Undo removes '
                      'them one at a time.',
                ].join(' '),
                key: const ValueKey('agent-plan-effect'),
                style: const TextStyle(
                  color: OrColors.textSecondary,
                  fontSize: OrType.metadataSize,
                ),
              ),
            ],
          ),
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(false),
          child: const Text('Cancel'),
        ),
        FilledButton(
          key: const ValueKey('agent-plan-apply'),
          onPressed: () => Navigator.of(context).pop(true),
          child: Text(
            'Add ${plan.shots.length} '
            '${plan.shots.length == 1 ? 'clip' : 'clips'}',
          ),
        ),
      ],
    );
  }

  static String _durationLabel(int frames, int frameRate) =>
      '${(frames / frameRate).toStringAsFixed(1)} s';
}
