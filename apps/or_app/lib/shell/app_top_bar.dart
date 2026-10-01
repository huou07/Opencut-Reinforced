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
    this.onExport,
    this.onCancelExport,
    this.exportIsActive = false,
    this.statusLabel,
    this.statusIsError = false,
  });

  final String title;
  final bool compact;
  final VoidCallback onOpenCommandPalette;
  final VoidCallback onHome;
  final VoidCallback? onExitEditorPreview;
  final VoidCallback? onExport;
  final VoidCallback? onCancelExport;
  final bool exportIsActive;
  final String? statusLabel;
  final bool statusIsError;

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
              if (statusLabel != null) ...[
                Icon(
                  statusIsError
                      ? Icons.warning_amber_outlined
                      : Icons.save_outlined,
                  size: 15,
                  color: statusIsError ? OrColors.warning : OrColors.textMuted,
                ),
                const SizedBox(width: OrSpacing.x1),
                Text(
                  statusLabel!,
                  style: TextStyle(
                    color: statusIsError
                        ? OrColors.warning
                        : OrColors.textMuted,
                    fontSize: 11,
                  ),
                ),
                const SizedBox(width: OrSpacing.x3),
              ],
              if (onExport != null) ...[
                if (compact)
                  Tooltip(
                    message: 'Export project',
                    child: IconButton(
                      key: const ValueKey('export-project'),
                      onPressed: onExport,
                      icon: const Icon(Icons.ios_share_outlined, size: 18),
                      color: OrColors.textSecondary,
                    ),
                  )
                else
                  OutlinedButton.icon(
                    key: const ValueKey('export-project'),
                    onPressed: onExport,
                    icon: const Icon(Icons.ios_share_outlined, size: 17),
                    label: const Text('Export'),
                    style: OutlinedButton.styleFrom(
                      minimumSize: const Size(0, 36),
                      padding: const EdgeInsets.symmetric(
                        horizontal: OrSpacing.x3,
                      ),
                      textStyle: const TextStyle(fontSize: 12),
                    ),
                  ),
                const SizedBox(width: OrSpacing.x1),
              ],
              if (exportIsActive && onCancelExport != null) ...[
                Tooltip(
                  message: 'Cancel export',
                  child: IconButton(
                    key: const ValueKey('cancel-export'),
                    onPressed: onCancelExport,
                    icon: const Icon(Icons.close_outlined, size: 17),
                    color: OrColors.textSecondary,
                  ),
                ),
                const SizedBox(width: OrSpacing.x1),
              ],
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
