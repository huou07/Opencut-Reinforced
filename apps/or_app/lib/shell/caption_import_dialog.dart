import 'dart:convert';

import 'package:flutter/material.dart';

import '../design/or_spacing.dart';
import '../project/project_gateway.dart';

/// Reviews an imported caption file before it becomes a caption track.
///
/// Blank cues are skipped, oversized cues stay editable until the text fits
/// and nothing is imported without an explicit accept.
Future<List<ProjectCaptionCue>?> showCaptionImportDialog(
  BuildContext context,
  ProjectCaptionImportPreview preview,
) => showDialog<List<ProjectCaptionCue>>(
    context: context,
    builder: (context) {
      final reviewed = List<ProjectCaptionCue>.of(preview.cues);
      final invalidCueIndexes = <int>{};
      var blankCueCount = reviewed
          .where((cue) => cue.text.trim().isEmpty)
          .length;
      final height = (MediaQuery.sizeOf(context).height * .65)
          .clamp(280.0, 580.0)
          .toDouble();
      return StatefulBuilder(
        builder: (context, setDialogState) => AlertDialog(
          title: Text('Review ${preview.formatName} captions'),
          content: SizedBox(
            width: 620,
            height: height,
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                Text(
                  [
                    '${reviewed.length - blankCueCount} ${reviewed.length - blankCueCount == 1 ? 'caption' : 'captions'} will be added to a new caption track. Edit text below; timing stays as imported.',
                    if (blankCueCount > 0)
                      '$blankCueCount blank ${blankCueCount == 1 ? 'cue' : 'cues'} will be skipped.',
                    if (preview.formattingLossCount > BigInt.zero)
                      '${preview.formattingLossCount} cues contain styling or placement that will be simplified to plain text.',
                    if (preview.emptyCuesSkipped > BigInt.zero)
                      '${preview.emptyCuesSkipped} empty cues were skipped.',
                  ].join('\n\n'),
                ),
                const SizedBox(height: 12),
                Expanded(
                  child: ListView.separated(
                    key: const ValueKey('caption-import-review-list'),
                    itemCount: reviewed.length,
                    separatorBuilder: (context, index) =>
                        const Divider(height: 16),
                    itemBuilder: (context, index) {
                      final cue = reviewed[index];
                      return Column(
                        crossAxisAlignment: CrossAxisAlignment.stretch,
                        children: [
                          Text(
                            '${captionTimeLabel(cue.startMs)} – ${captionTimeLabel(cue.endMs)}',
                            style: Theme.of(context).textTheme.labelSmall,
                          ),
                          // The field's label floats above its box, so the timing
                          // line needs its own gap instead of being overlapped.
                          const SizedBox(height: OrSpacing.x3),
                          TextFormField(
                            key: ValueKey('caption-import-text-$index'),
                            initialValue: cue.text,
                            minLines: 1,
                            maxLines: 4,
                            maxLength: preview.maxCaptionTextBytes.toInt(),
                            decoration: InputDecoration(
                              labelText: 'Caption ${index + 1} text',
                              counterText: '',
                              errorText: invalidCueIndexes.contains(index)
                                  ? 'Caption exceeds the ${preview.maxCaptionTextBytes}-byte project limit.'
                                  : null,
                            ),
                            onChanged: (text) {
                              final previousWasBlank = reviewed[index].text
                                  .trim()
                                  .isEmpty;
                              final isBlank = text.trim().isEmpty;
                              final isTooLarge =
                                  utf8.encode(text).length >
                                  preview.maxCaptionTextBytes.toInt();
                              setDialogState(() {
                                reviewed[index] = ProjectCaptionCue(
                                  startMs: cue.startMs,
                                  endMs: cue.endMs,
                                  text: text,
                                );
                                if (previousWasBlank != isBlank) {
                                  blankCueCount += isBlank ? 1 : -1;
                                }
                                if (isTooLarge) {
                                  invalidCueIndexes.add(index);
                                } else {
                                  invalidCueIndexes.remove(index);
                                }
                              });
                            },
                          ),
                        ],
                      );
                    },
                  ),
                ),
              ],
            ),
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.of(context).pop(),
              child: const Text('Cancel'),
            ),
            FilledButton(
              onPressed:
                  invalidCueIndexes.isNotEmpty ||
                      reviewed.length == blankCueCount
                  ? null
                  : () => Navigator.of(context)
                        .pop(List<ProjectCaptionCue>.unmodifiable(reviewed)),
              child: const Text('Import captions'),
            ),
          ],
        ),
      );
    },
  );

String captionTimeLabel(BigInt milliseconds) {
  final safeMilliseconds = milliseconds.isNegative ? BigInt.zero : milliseconds;
  final hours = safeMilliseconds ~/ BigInt.from(3_600_000);
  final afterHours = safeMilliseconds % BigInt.from(3_600_000);
  final minutes = afterHours ~/ BigInt.from(60_000);
  final afterMinutes = afterHours % BigInt.from(60_000);
  final seconds = afterMinutes ~/ BigInt.from(1_000);
  final millis = afterMinutes % BigInt.from(1_000);
  return '${hours.toString().padLeft(2, '0')}:${minutes.toString().padLeft(2, '0')}:${seconds.toString().padLeft(2, '0')}.${millis.toString().padLeft(3, '0')}';
}
