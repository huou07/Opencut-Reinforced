import 'package:flutter/material.dart';

import '../design/or_spacing.dart';
import '../project/active_project_card.dart';
import '../project/project_gateway.dart';
import '../widgets/or_widgets.dart';

class ProjectsScreen extends StatelessWidget {
  const ProjectsScreen({
    super.key,
    required this.project,
    required this.canPickProjects,
    required this.onNewProject,
    required this.onOpenProject,
    required this.onOpenWorkspace,
    required this.onRenameProject,
    required this.onSaveProject,
    required this.onCloseProject,
  });

  final ProjectReadModel? project;
  final bool canPickProjects;
  final VoidCallback onNewProject;
  final VoidCallback onOpenProject;
  final VoidCallback onOpenWorkspace;
  final VoidCallback onRenameProject;
  final VoidCallback onSaveProject;
  final VoidCallback onCloseProject;

  @override
  Widget build(BuildContext context) {
    const androidMessage =
        'Project file access on Android requires Storage Access Framework integration and is not available in this Developer Preview.';

    return OrPageLayout(
      title: 'Projects',
      subtitle: 'Browse and organize project files',
      action: canPickProjects
          ? FilledButton.icon(
              key: const ValueKey('projects-new-project'),
              onPressed: onNewProject,
              icon: const Icon(Icons.add_outlined),
              label: const Text('New Project'),
            )
          : OrUnavailableButton(
              key: const ValueKey('projects-new-project'),
              label: 'New Project',
              icon: Icons.add_outlined,
              primary: true,
              reason: androidMessage,
              onPressed: onNewProject,
            ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          OrPanel(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Wrap(
                  spacing: OrSpacing.x3,
                  runSpacing: OrSpacing.x3,
                  crossAxisAlignment: WrapCrossAlignment.center,
                  children: const [
                    OrSearchField(label: 'Search projects'),
                    OrDisabledChip('Sort: Edited'),
                    OrDisabledChip('Grid view'),
                    OrDisabledChip('List view'),
                  ],
                ),
                const SizedBox(height: OrSpacing.x3),
                const Wrap(
                  spacing: OrSpacing.x2,
                  runSpacing: OrSpacing.x2,
                  children: [
                    OrDisabledChip('All'),
                    OrDisabledChip('Recent'),
                    OrDisabledChip('Pinned'),
                    OrDisabledChip('Archived'),
                  ],
                ),
              ],
            ),
          ),
          const SizedBox(height: OrSpacing.x4),
          if (project case final active?)
            ActiveProjectCard(
              project: active,
              onOpen: onOpenWorkspace,
              onSave: onSaveProject,
              onRename: onRenameProject,
              onClose: onCloseProject,
            )
          else
            OrPanel(
              title: 'Projects',
              padding: EdgeInsets.zero,
              child: Column(
                children: [
                  const OrEmptyState(
                    title: 'No active project',
                    message: 'Create a project or open an existing .orproj file to begin.',
                  ),
                  Padding(
                    padding: const EdgeInsets.only(bottom: OrSpacing.x4),
                    child: canPickProjects
                        ? OutlinedButton.icon(
                            key: const ValueKey('projects-open-project'),
                            onPressed: onOpenProject,
                            icon: const Icon(Icons.folder_open_outlined),
                            label: const Text('Open Project'),
                          )
                        : OrUnavailableButton(
                            key: const ValueKey('projects-open-project'),
                            label: 'Open Project',
                            icon: Icons.folder_open_outlined,
                            reason: androidMessage,
                            onPressed: onOpenProject,
                          ),
                  ),
                ],
              ),
            ),
        ],
      ),
    );
  }
}
