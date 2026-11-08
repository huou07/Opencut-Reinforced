import 'package:flutter/material.dart';

import '../design/or_colors.dart';
import '../design/or_spacing.dart';
import '../design/or_typography.dart';
import '../library/creative_directions.dart';

/// Edits one of the person's own creative directions.
///
/// A kept copy is only useful if its brief can be rewritten: the bundled
/// directions describe a method the product chose, while this dialog edits the
/// copy the person keeps. Every section is a short paragraph, so one field per
/// section with the same names the read-only brief uses is the smallest honest
/// form — no section may be blanked, because a brief with a missing part is
/// worse than the bundled one it came from.
class CreativeDirectionEditorDialog extends StatefulWidget {
  const CreativeDirectionEditorDialog({super.key, required this.direction});

  final CreativeDirection direction;

  @override
  State<CreativeDirectionEditorDialog> createState() =>
      _CreativeDirectionEditorDialogState();
}

class _CreativeDirectionEditorDialogState
    extends State<CreativeDirectionEditorDialog> {
  late final Map<String, TextEditingController> _fields = {
    'name': TextEditingController(text: widget.direction.name),
    'summary': TextEditingController(text: widget.direction.summary),
    'best-for': TextEditingController(
      text: widget.direction.bestFor.join('; '),
    ),
    'voice': TextEditingController(text: widget.direction.voice),
    'script': TextEditingController(text: widget.direction.script),
    'storytelling': TextEditingController(text: widget.direction.storytelling),
    'graphics': TextEditingController(text: widget.direction.graphics),
    'captions': TextEditingController(text: widget.direction.captions),
    'editing': TextEditingController(text: widget.direction.editing),
    'audio': TextEditingController(text: widget.direction.audio),
  };

  static const _labels = <String, String>{
    'name': 'Name',
    'summary': 'One line about the style',
    'best-for': 'Best for',
    'voice': 'Voice and narration',
    'script': 'Script shape',
    'storytelling': 'Storytelling',
    'graphics': 'Graphics',
    'captions': 'Captions',
    'editing': 'Editing',
    'audio': 'Audio',
  };

  bool _filled(String key) => _fields[key]!.text.trim().isNotEmpty;

  bool get _complete => _fields.keys.every(_filled);

  @override
  void dispose() {
    for (final controller in _fields.values) {
      controller.dispose();
    }
    super.dispose();
  }

  CreativeDirection _edited() => widget.direction.copyWith(
    name: _fields['name']!.text.trim(),
    summary: _fields['summary']!.text.trim(),
    bestFor: _fields['best-for']!
        .text
        .split(';')
        .map((item) => item.trim())
        .where((item) => item.isNotEmpty)
        .toList(growable: false),
    voice: _fields['voice']!.text.trim(),
    script: _fields['script']!.text.trim(),
    storytelling: _fields['storytelling']!.text.trim(),
    graphics: _fields['graphics']!.text.trim(),
    captions: _fields['captions']!.text.trim(),
    editing: _fields['editing']!.text.trim(),
    audio: _fields['audio']!.text.trim(),
    isMine: true,
  );

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: const Text('Edit your creative direction'),
      content: SizedBox(
        width: OrDialogSizes.width(context, 460),
        child: SingleChildScrollView(
          key: const ValueKey('direction-editor'),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              const Text(
                'This brief is context for you and for the assistant. Describe '
                'the method, not a subject: a direction has to work for the '
                'next video too.',
                style: TextStyle(
                  color: OrColors.textSecondary,
                  fontSize: OrType.metadataSize,
                ),
              ),
              const SizedBox(height: OrSpacing.x3),
              for (final entry in _labels.entries) ...[
                TextField(
                  key: ValueKey('direction-edit-${entry.key}'),
                  controller: _fields[entry.key],
                  maxLines: entry.key == 'name' ? 1 : null,
                  minLines: entry.key == 'name' ? 1 : 2,
                  onChanged: (_) => setState(() {}),
                  decoration: InputDecoration(labelText: entry.value),
                ),
                const SizedBox(height: OrSpacing.x2),
              ],
            ],
          ),
        ),
      ),
      actions: [
        TextButton(
          key: const ValueKey('direction-edit-cancel'),
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('Cancel'),
        ),
        FilledButton(
          key: const ValueKey('direction-edit-save'),
          onPressed: _complete
              ? () => Navigator.of(context).pop(_edited())
              : null,
          child: const Text('Save'),
        ),
      ],
    );
  }
}
