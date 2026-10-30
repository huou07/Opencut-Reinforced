import 'package:flutter/material.dart';

import '../design/or_spacing.dart';
import '../project/project_gateway.dart';
import 'project_recovery.dart';

/// Asks what to do with a recoverable checkpoint.
Future<RecoveryChoice?> showRecoveryCheckpointDialog(
  BuildContext context,
  ProjectRecoveryInspection inspection,
) => showDialog<RecoveryChoice>(
  context: context,
  builder: (context) => AlertDialog(
    title: const Text('Recovery checkpoint found'),
    content: Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          'A recoverable project checkpoint is available${inspection.recoveryName.isEmpty ? '' : ' for “${inspection.recoveryName}”'}.',
        ),
        const SizedBox(height: OrSpacing.x3),
        Text('Saved revision: ${inspection.baseRevision}'),
        Text('Recovery revision: ${inspection.recoveryRevision}'),
      ],
    ),
    actions: [
      TextButton(
        onPressed: () => Navigator.pop(context, RecoveryChoice.cancel),
        child: const Text('Cancel'),
      ),
      TextButton(
        onPressed: () => Navigator.pop(context, RecoveryChoice.discard),
        child: const Text('Discard'),
      ),
      FilledButton(
        onPressed: () => Navigator.pop(context, RecoveryChoice.recover),
        child: const Text('Recover'),
      ),
    ],
  ),
);

/// Asks whether to discard a checkpoint from another lineage, then confirms.
Future<bool> showConflictingRecoveryDiscardDialog(
  BuildContext context,
  String reason,
) async {
  final discard = await showDialog<bool>(
    context: context,
    builder: (context) => AlertDialog(
      title: const Text('Recovery checkpoint conflict'),
      content: Text(
        'This checkpoint does not match the project lineage${reason.isEmpty ? '.' : ' ($reason).'} No recovery data will be applied.',
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context, false),
          child: const Text('Cancel'),
        ),
        FilledButton(
          onPressed: () => Navigator.pop(context, true),
          child: const Text('Discard checkpoint'),
        ),
      ],
    ),
  );
  if (discard != true) return false;
  if (!context.mounted) return false;
  return confirmRecoveryDiscardDialog(context);
}

/// Asks whether to discard a malformed checkpoint, then confirms.
Future<bool> showInvalidRecoveryDiscardDialog(BuildContext context) async {
  final discard = await showDialog<bool>(
    context: context,
    builder: (context) => AlertDialog(
      title: const Text('Invalid recovery checkpoint'),
      content: const Text(
        'This recovery checkpoint is malformed or failed validation. Its contents cannot be recovered, and the saved project will remain unchanged.',
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context, false),
          child: const Text('Cancel'),
        ),
        FilledButton(
          onPressed: () => Navigator.pop(context, true),
          child: const Text('Discard checkpoint'),
        ),
      ],
    ),
  );
  if (discard != true) return false;
  if (!context.mounted) return false;
  return confirmRecoveryDiscardDialog(context);
}

/// Confirms that the checkpoint file will be removed permanently.
Future<bool> confirmRecoveryDiscardDialog(BuildContext context) async =>
    await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Discard recovery data?'),
        content: const Text(
          'This removes the recovery checkpoint permanently.',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context, false),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(context, true),
            child: const Text('Discard'),
          ),
        ],
      ),
    ) ??
    false;
