import 'package:flutter/material.dart';

import '../design/or_colors.dart';
import '../design/or_spacing.dart';
import '../widgets/or_widgets.dart';

class HomeScreen extends StatelessWidget {
  const HomeScreen({
    super.key,
    required this.canPickProjects,
    required this.onNewProject,
    required this.onOpenProject,
    required this.onOpenEditorPreview,
    required this.onOpenProjects,
  });

  final bool canPickProjects;
  final VoidCallback onNewProject;
  final VoidCallback onOpenProject;
  final VoidCallback onOpenEditorPreview;
  final VoidCallback onOpenProjects;

  @override
  Widget build(BuildContext context) {
    const androidMessage =
        'Project file access on Android requires Storage Access Framework integration and is not available in this Developer Preview.';

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
              if (canPickProjects)
                FilledButton.icon(
                  key: const ValueKey('home-new-project'),
                  onPressed: onNewProject,
                  icon: const Icon(Icons.add_outlined),
                  label: const Text('New Project'),
                )
              else
                OrUnavailableButton(
                  key: const ValueKey('home-new-project'),
                  label: 'New Project',
                  icon: Icons.add_outlined,
                  reason: androidMessage,
                  primary: true,
                  onPressed: onNewProject,
                ),
              if (canPickProjects)
                OutlinedButton.icon(
                  key: const ValueKey('home-open-project'),
                  onPressed: onOpenProject,
                  icon: const Icon(Icons.folder_open_outlined),
                  label: const Text('Open Project'),
                )
              else
                OrUnavailableButton(
                  key: const ValueKey('home-open-project'),
                  label: 'Open Project',
                  icon: Icons.folder_open_outlined,
                  reason: androidMessage,
                  onPressed: onOpenProject,
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
              message: 'Opened projects appear in the Projects workspace. Recent project history is not stored in this preview.',
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
