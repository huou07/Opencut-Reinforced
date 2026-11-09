import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'package:or_app/design/or_colors.dart';
import 'package:or_app/design/or_spacing.dart';
import 'package:or_app/design/or_typography.dart';
import 'package:or_app/project/project_gateway.dart';
import 'package:or_app/widgets/or_widgets.dart';

/// Shows the record a companion tool left beside one library item.
///
/// A rendered result is not an editable composition, and the person can only
/// trust that if the record says which tool, which command, which source
/// project and which file it describes. Everything here is read from the
/// sidecar; nothing is re-probed and nothing is uploaded.
Future<void> showMediaProvenanceDialog(
  BuildContext context,
  ProjectMediaItem item,
) {
  return showDialog<void>(
    context: context,
    builder: (context) => _MediaProvenanceDialog(item: item),
  );
}

class _MediaProvenanceDialog extends StatelessWidget {
  const _MediaProvenanceDialog({required this.item});

  final ProjectMediaItem item;

  @override
  Widget build(BuildContext context) {
    final tool = item.provenanceTool ?? 'a companion tool';
    final version = item.provenanceVersion;
    final command = item.provenanceCommand;
    return AlertDialog(
      key: const ValueKey('media-provenance-dialog'),
      title: Text(version == null ? 'Made by $tool' : 'Made by $tool $version'),
      content: SizedBox(
        width: OrDialogSizes.width(context, 460),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            const Text(
              'This file was rendered outside Opencut Reinforced. The record '
              'below keeps the source and the command, so the result can be '
              'reproduced instead of guessed at.',
              style: TextStyle(
                color: OrColors.textMuted,
                fontSize: OrType.tinySize,
                height: 1.4,
              ),
            ),
            const SizedBox(height: OrSpacing.x3),
            if (item.provenanceSourceProject != null)
              _row(
                'source-project',
                'Source project',
                item.provenanceSourceProject!,
              ),
            if (item.provenanceManifest != null)
              _row('manifest', 'Scene or manifest', item.provenanceManifest!),
            _row(
              'output',
              'Result file',
              item.provenanceOutput ?? item.sourceUri,
            ),
            if (command != null) _row('command', 'Command', command),
            if (item.provenanceRecord != null)
              _row('record', 'Record', item.provenanceRecord!),
          ],
        ),
      ),
      actions: [
        if (command != null)
          TextButton(
            key: const ValueKey('media-provenance-copy-command'),
            onPressed: () async {
              await Clipboard.setData(ClipboardData(text: command));
              if (!context.mounted) return;
              OrSnackBar.show(context, 'Command copied');
            },
            child: const Text('Copy command'),
          ),
        TextButton(
          key: const ValueKey('media-provenance-close'),
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('Close'),
        ),
      ],
    );
  }

  Widget _row(String key, String label, String value) {
    return Padding(
      padding: const EdgeInsets.only(bottom: OrSpacing.x3),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            label,
            style: const TextStyle(
              color: OrColors.textMuted,
              fontSize: OrType.tinySize,
            ),
          ),
          const SizedBox(height: OrSpacing.x1),
          SelectableText(
            value,
            key: ValueKey('media-provenance-$key'),
            style: const TextStyle(
              color: OrColors.text,
              fontSize: OrType.tinySize,
              height: 1.4,
            ),
          ),
        ],
      ),
    );
  }
}
