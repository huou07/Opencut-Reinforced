import 'package:flutter/material.dart';

import '../design/or_colors.dart';
import '../design/or_spacing.dart';
import '../project/project_gateway.dart';

/// A compact colour row: swatch, current value and a picker.
class OrColorField extends StatelessWidget {
  const OrColorField({
    super.key,
    required this.label,
    required this.color,
    required this.onChanged,
    this.allowAlpha = true,
  });

  final String label;
  final ProjectTextColor color;
  final ValueChanged<ProjectTextColor> onChanged;
  final bool allowAlpha;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(top: OrSpacing.x2),
      child: Row(
        children: [
          _swatch(color),
          const SizedBox(width: OrSpacing.x2),
          Expanded(
            child: Text(
              '$label  ${_hex(color)}',
              style: const TextStyle(fontSize: 13),
            ),
          ),
          TextButton(
            onPressed: () async {
              final picked = await showOrColorPicker(
                context,
                initial: color,
                allowAlpha: allowAlpha,
                title: label,
              );
              if (picked != null) onChanged(picked);
            },
            child: const Text('Choose…'),
          ),
        ],
      ),
    );
  }

  static String _hex(ProjectTextColor value) {
    String two(int channel) => channel.toRadixString(16).padLeft(2, '0');
    final base =
        '#${two(value.red)}${two(value.green)}${two(value.blue)}'.toUpperCase();
    return value.alpha == 255 ? base : '$base${two(value.alpha)}'.toUpperCase();
  }

  static Widget _swatch(ProjectTextColor value) => Container(
    width: 24,
    height: 24,
    decoration: BoxDecoration(
      color: Color.fromARGB(value.alpha, value.red, value.green, value.blue),
      border: Border.all(color: OrColors.borderStrong),
      borderRadius: BorderRadius.circular(4),
    ),
  );
}

Future<ProjectTextColor?> showOrColorPicker(
  BuildContext context, {
  required ProjectTextColor initial,
  bool allowAlpha = true,
  String title = 'Color',
}) {
  var red = initial.red;
  var green = initial.green;
  var blue = initial.blue;
  var alpha = initial.alpha;
  final hexController = TextEditingController(text: _hex(initial));

  ProjectTextColor current() =>
      ProjectTextColor(red: red, green: green, blue: blue, alpha: alpha);

  void syncHex() => hexController.text = _hex(current());

  return showDialog<ProjectTextColor>(
    context: context,
    builder: (dialogContext) => StatefulBuilder(
      builder: (dialogContext, setDialogState) {
        Widget channel(
          String label,
          int value,
          ValueChanged<int> changed,
        ) => Row(
          children: [
            SizedBox(width: 18, child: Text(label)),
            Expanded(
              child: Slider(
                value: value.toDouble(),
                min: 0,
                max: 255,
                divisions: 255,
                label: '$value',
                onChanged: (next) => setDialogState(() {
                  changed(next.round());
                  syncHex();
                }),
              ),
            ),
            SizedBox(
              width: 34,
              child: Text('$value', textAlign: TextAlign.end),
            ),
          ],
        );

        return AlertDialog(
          title: Text(title),
          content: SizedBox(
            width: 380,
            child: SingleChildScrollView(
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  Container(
                    height: 56,
                    decoration: BoxDecoration(
                      color: Color.fromARGB(alpha, red, green, blue),
                      border: Border.all(color: OrColors.borderStrong),
                      borderRadius: BorderRadius.circular(6),
                    ),
                  ),
                  const SizedBox(height: OrSpacing.x2),
                  Wrap(
                    spacing: OrSpacing.x1,
                    runSpacing: OrSpacing.x1,
                    children: [
                      for (final preset in _presets)
                        InkWell(
                          onTap: () => setDialogState(() {
                            red = preset.red;
                            green = preset.green;
                            blue = preset.blue;
                            alpha = preset.alpha;
                            syncHex();
                          }),
                          child: Container(
                            width: 28,
                            height: 28,
                            decoration: BoxDecoration(
                              color: Color.fromARGB(
                                preset.alpha,
                                preset.red,
                                preset.green,
                                preset.blue,
                              ),
                              border: Border.all(color: OrColors.borderStrong),
                              borderRadius: BorderRadius.circular(4),
                            ),
                          ),
                        ),
                    ],
                  ),
                  channel('R', red, (value) => red = value),
                  channel('G', green, (value) => green = value),
                  channel('B', blue, (value) => blue = value),
                  if (allowAlpha)
                    channel('A', alpha, (value) => alpha = value),
                  TextField(
                    controller: hexController,
                    decoration: const InputDecoration(
                      labelText: 'Hex (#RRGGBB or #RRGGBBAA)',
                      isDense: true,
                    ),
                    onSubmitted: (value) {
                      final parsed = _parseHex(value);
                      if (parsed == null) return;
                      setDialogState(() {
                        red = parsed.red;
                        green = parsed.green;
                        blue = parsed.blue;
                        alpha = parsed.alpha;
                      });
                    },
                  ),
                ],
              ),
            ),
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.of(dialogContext).pop(),
              child: const Text('Cancel'),
            ),
            FilledButton(
              onPressed: () => Navigator.of(dialogContext).pop(current()),
              child: const Text('Use color'),
            ),
          ],
        );
      },
    ),
  );
}

const _presets = <ProjectTextColor>[
  ProjectTextColor.white,
  ProjectTextColor(red: 0, green: 0, blue: 0, alpha: 255),
  ProjectTextColor(red: 255, green: 220, blue: 90, alpha: 255),
  ProjectTextColor(red: 110, green: 210, blue: 255, alpha: 255),
  ProjectTextColor(red: 255, green: 120, blue: 120, alpha: 255),
  ProjectTextColor(red: 120, green: 230, blue: 160, alpha: 255),
  ProjectTextColor(red: 190, green: 140, blue: 255, alpha: 255),
  ProjectTextColor(red: 0, green: 0, blue: 0, alpha: 170),
];

String _hex(ProjectTextColor value) {
  String two(int channel) => channel.toRadixString(16).padLeft(2, '0');
  final base =
      '#${two(value.red)}${two(value.green)}${two(value.blue)}'.toUpperCase();
  return value.alpha == 255 ? base : '$base${two(value.alpha)}'.toUpperCase();
}

ProjectTextColor? _parseHex(String raw) {
  var value = raw.trim();
  if (value.startsWith('#')) value = value.substring(1);
  if (value.length != 6 && value.length != 8) return null;
  final parsed = int.tryParse(value, radix: 16);
  if (parsed == null) return null;
  if (value.length == 6) {
    return ProjectTextColor(
      red: (parsed >> 16) & 0xFF,
      green: (parsed >> 8) & 0xFF,
      blue: parsed & 0xFF,
      alpha: 255,
    );
  }
  return ProjectTextColor(
    red: (parsed >> 24) & 0xFF,
    green: (parsed >> 16) & 0xFF,
    blue: (parsed >> 8) & 0xFF,
    alpha: parsed & 0xFF,
  );
}
