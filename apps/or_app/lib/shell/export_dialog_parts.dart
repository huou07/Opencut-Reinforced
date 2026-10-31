import 'package:flutter/material.dart';

import '../design/or_colors.dart';
import '../design/or_spacing.dart';
import '../design/or_typography.dart';

/// Export heights offered in the dialog, as the short side in pixels.
///
/// A short side keeps the canvas shape: a 9:16 canvas exported at 1080p is
/// 1080 × 1920, not letterboxed 1920 × 1080. `null` keeps the project canvas
/// size exactly.
const exportResolutions = <(int?, String)>[
  (null, 'Match project'),
  (2160, '2160p (4K)'),
  (1440, '1440p'),
  (1080, '1080p'),
  (720, '720p'),
  (480, '480p'),
];

class ExportDialogSection extends StatelessWidget {
  const ExportDialogSection(this.label, {super.key});

  final String label;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(top: OrSpacing.x2, bottom: OrSpacing.x1),
      child: Text(
        label.toUpperCase(),
        style: const TextStyle(
          color: OrColors.textMuted,
          fontSize: OrType.tinySize,
          fontWeight: FontWeight.w600,
          letterSpacing: 0.6,
        ),
      ),
    );
  }
}

class ExportChoiceTile extends StatelessWidget {
  const ExportChoiceTile({
    super.key,
    required this.title,
    required this.subtitle,
    required this.selected,
    required this.onTap,
  });

  final String title;
  final String subtitle;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return Semantics(
      selected: selected,
      button: true,
      child: InkWell(
        onTap: onTap,
        borderRadius: BorderRadius.circular(OrRadii.small),
        child: Padding(
          padding: const EdgeInsets.symmetric(
            vertical: OrSpacing.x1,
            horizontal: OrSpacing.x1,
          ),
          child: Row(
            children: [
              SizedBox(
                width: 20,
                child: selected
                    ? const Icon(
                        Icons.check,
                        size: 16,
                        color: OrColors.selection,
                      )
                    : null,
              ),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      title,
                      style: const TextStyle(fontSize: OrType.bodySize),
                    ),
                    Text(
                      subtitle,
                      style: const TextStyle(
                        color: OrColors.textMuted,
                        fontSize: OrType.tinySize,
                      ),
                    ),
                  ],
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
