import 'package:flutter/material.dart';

import '../design/or_colors.dart';
import '../design/or_spacing.dart';
import '../widgets/or_widgets.dart';

class HomeScreen extends StatelessWidget {
  const HomeScreen({
    super.key,
    required this.onUnavailable,
    required this.onOpenEditorPreview,
    required this.onOpenProjects,
  });

  final ValueChanged<String> onUnavailable;
  final VoidCallback onOpenEditorPreview;
  final VoidCallback onOpenProjects;

  @override
  Widget build(BuildContext context) {
    const projectActionReason =
        'Project creation is unavailable in this Developer Preview.';
    const openActionReason =
        'Opening projects from the production UI is not available in this Developer Preview.';

    return OrPageLayout(
      title: 'Home',
      subtitle: 'Projects and recent work',
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          const OrSectionHeader('Quick actions'),
          Wrap(
            spacing: OrSpacing.x3,
            runSpacing: OrSpacing.x3,
            children: [
              OrUnavailableButton(
                key: const ValueKey('home-new-project'),
                label: 'New Project',
                icon: Icons.add_outlined,
                reason: projectActionReason,
                primary: true,
                onPressed: () => onUnavailable(projectActionReason),
              ),
              OrUnavailableButton(
                key: const ValueKey('home-open-project'),
                label: 'Open Project',
                icon: Icons.folder_open_outlined,
                reason: openActionReason,
                onPressed: () => onUnavailable(openActionReason),
              ),
            ],
          ),
          const SizedBox(height: OrSpacing.x8),
          OrPanel(
            title: 'Recent Projects',
            trailing: TextButton(
              key: const ValueKey('home-view-projects'),
              onPressed: onOpenProjects,
              child: const Text('View all'),
            ),
            padding: EdgeInsets.zero,
            child: const OrEmptyState(
              title: 'No recent projects yet',
              message: 'Project open and create UI wiring will arrive in a later foundation step.',
            ),
          ),
          const SizedBox(height: OrSpacing.x6),
          OrPanel(
            title: 'Developer Preview',
            trailing: const OrBadge('Pre-MVP'),
            child: LayoutBuilder(
              builder: (context, constraints) {
                final compact = OrBreakpoints.isCompact(constraints.maxWidth);
                final copy = const Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      'The production visual shell is ready for evaluation.',
                      style: TextStyle(
                        color: OrColors.text,
                        fontSize: 14,
                        fontWeight: FontWeight.w500,
                      ),
                    ),
                    SizedBox(height: OrSpacing.x2),
                    Text(
                      'Project and core foundations exist. Editing, media, playback, and export are not connected.',
                      style: TextStyle(
                        color: OrColors.textSecondary,
                        fontSize: 13,
                        height: 1.45,
                      ),
                    ),
                  ],
                );

                if (compact) {
                  return Column(
                    crossAxisAlignment: CrossAxisAlignment.stretch,
                    children: [
                      copy,
                      const SizedBox(height: OrSpacing.x4),
                      OutlinedButton.icon(
                        key: const ValueKey('home-editor-preview'),
                        onPressed: onOpenEditorPreview,
                        icon: const Icon(Icons.view_quilt_outlined),
                        label: const Text('Open Editor Shell Preview'),
                      ),
                    ],
                  );
                }

                return Row(
                  children: [
                    Expanded(child: copy),
                    const SizedBox(width: OrSpacing.x6),
                    OutlinedButton.icon(
                      key: const ValueKey('home-editor-preview'),
                      onPressed: onOpenEditorPreview,
                      icon: const Icon(Icons.view_quilt_outlined),
                      label: const Text('Open Editor Shell Preview'),
                    ),
                  ],
                );
              },
            ),
          ),
        ],
      ),
    );
  }
}
