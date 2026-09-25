import 'package:flutter/material.dart';

import '../design/or_colors.dart';
import '../design/or_spacing.dart';

class AppTopBar extends StatelessWidget {
  const AppTopBar({
    super.key,
    required this.title,
    required this.compact,
    required this.onOpenCommandPalette,
    required this.onHome,
    this.onExitEditorPreview,
  });

  final String title;
  final bool compact;
  final VoidCallback onOpenCommandPalette;
  final VoidCallback onHome;
  final VoidCallback? onExitEditorPreview;

  @override
  Widget build(BuildContext context) {
    return Container(
      key: const ValueKey('app-top-bar'),
      height: OrSpacing.topBarHeight,
      padding: EdgeInsets.symmetric(
        horizontal: compact ? OrSpacing.x3 : OrSpacing.x5,
      ),
      decoration: const BoxDecoration(
        color: OrColors.backgroundRaised,
        border: Border(bottom: BorderSide(color: OrColors.border)),
      ),
      child: LayoutBuilder(
        builder: (context, constraints) {
          final showFullBrand = OrBreakpoints.isWide(constraints.maxWidth);
          final showPreviewBadge = showFullBrand;

          return Row(
            children: [
              if (onExitEditorPreview != null) ...[
                Tooltip(
                  message: 'Exit Editor Shell Preview',
                  child: IconButton(
                    key: const ValueKey('exit-editor-preview'),
                    onPressed: onExitEditorPreview,
                    icon: const Icon(Icons.arrow_back_outlined, size: 19),
                    color: OrColors.textSecondary,
                  ),
                ),
                const SizedBox(width: OrSpacing.x2),
              ],
              InkWell(
                key: const ValueKey('or-brand-home'),
                onTap: onHome,
                borderRadius: BorderRadius.circular(OrRadii.small),
                child: Container(
                  width: 30,
                  height: 30,
                  alignment: Alignment.center,
                  decoration: BoxDecoration(
                    border: Border.all(color: OrColors.borderStrong),
                    borderRadius: BorderRadius.circular(7),
                  ),
                  child: const Text(
                    'OR',
                    style: TextStyle(fontSize: 12, fontWeight: FontWeight.w700),
                  ),
                ),
              ),
              if (showFullBrand) ...[
                const SizedBox(width: OrSpacing.x3),
                const Text(
                  'Opencut Reinforced',
                  style: TextStyle(fontSize: 13, fontWeight: FontWeight.w600),
                ),
              ],
              const SizedBox(width: OrSpacing.x4),
              Container(width: 1, height: 24, color: OrColors.border),
              const SizedBox(width: OrSpacing.x4),
              Expanded(
                child: Text(
                  title,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: const TextStyle(
                    color: OrColors.textSecondary,
                    fontSize: 13,
                    fontWeight: FontWeight.w500,
                  ),
                ),
              ),
              if (showPreviewBadge) ...[
                const _PreviewBadge(),
                const SizedBox(width: OrSpacing.x3),
              ],
              if (showFullBrand)
                OutlinedButton.icon(
                  key: const ValueKey('open-command-palette'),
                  onPressed: onOpenCommandPalette,
                  icon: const Icon(Icons.search_outlined, size: 17),
                  label: const Row(
                    children: [
                      Text('Search commands'),
                      SizedBox(width: OrSpacing.x2),
                      Text(
                        'Ctrl/⌘ K',
                        style: TextStyle(
                          color: OrColors.textMuted,
                          fontSize: 11,
                        ),
                      ),
                    ],
                  ),
                  style: OutlinedButton.styleFrom(
                    minimumSize: const Size(0, 36),
                    padding: const EdgeInsets.symmetric(
                      horizontal: OrSpacing.x3,
                    ),
                    textStyle: const TextStyle(fontSize: 12),
                  ),
                )
              else
                Tooltip(
                  message: 'Search commands (Ctrl/⌘ K)',
                  child: IconButton(
                    key: const ValueKey('open-command-palette'),
                    onPressed: onOpenCommandPalette,
                    icon: const Icon(Icons.search_outlined),
                    color: OrColors.textSecondary,
                  ),
                ),
            ],
          );
        },
      ),
    );
  }
}

class _PreviewBadge extends StatelessWidget {
  const _PreviewBadge();

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(
        horizontal: OrSpacing.x2,
        vertical: 5,
      ),
      decoration: BoxDecoration(
        border: Border.all(color: OrColors.border),
        borderRadius: BorderRadius.circular(OrRadii.small),
      ),
      child: const Text(
        'Developer Preview',
        style: TextStyle(color: OrColors.textSecondary, fontSize: 10),
      ),
    );
  }
}
