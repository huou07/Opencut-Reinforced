import 'package:flutter/material.dart';

import '../design/or_spacing.dart';
import '../design/or_typography.dart';

/// A tool-panel title row: one line of text with an optional trailing control.
///
/// Shared by the tool panels so every pane spells its heading the same way.
class EditorPanelHeader extends StatelessWidget {
  const EditorPanelHeader({
    super.key,
    required this.title,
    this.height = 38,
    this.trailing,
  });

  final String title;
  final double height;
  final Widget? trailing;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      height: height,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: OrSpacing.x3),
        child: Row(
          children: [
            Expanded(
              child: Text(
                title,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: const TextStyle(
                  fontSize: OrType.metadataCompactSize,
                  fontWeight: FontWeight.w600,
                ),
              ),
            ),
            ?trailing,
          ],
        ),
      ),
    );
  }
}
