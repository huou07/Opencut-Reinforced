import 'package:flutter/material.dart';

import '../design/or_spacing.dart';
import '../project/active_project_card.dart';
import '../project/project_gateway.dart';
import '../project/recent_projects.dart';
import '../widgets/or_widgets.dart';
import 'recent_projects_list.dart';

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
    required this.recentProjects,
    required this.recentProjectsLoaded,
    required this.recentProjectsUnavailable,
    required this.onOpenRecentProject,
  });

  final ProjectReadModel? project;
  final bool canPickProjects;
  final VoidCallback onNewProject;
  final VoidCallback onOpenProject;
  final VoidCallback onOpenWorkspace;
  final VoidCallback onRenameProject;
  final VoidCallback onSaveProject;
  final VoidCallback onCloseProject;
  final List<RecentProject> recentProjects;
  final bool recentProjectsLoaded;
  final bool recentProjectsUnavailable;
  final ValueChanged<RecentProject> onOpenRecentProject;

  @override
  Widget build(BuildContext context) {
    const androidMessage = 'Project file access is unavailable in this build.';

    return OrPageLayout(
      title: 'Projects',
      subtitle: 'Create or open an .orproj project',
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
          const SizedBox(height: OrSpacing.x6),
          OrPanel(
            title: 'Recent Projects',
            padding: EdgeInsets.zero,
            child: RecentProjectsList(
              projects: recentProjects,
              loaded: recentProjectsLoaded,
              unavailable: recentProjectsUnavailable,
              onOpen: onOpenRecentProject,
            ),
          ),
        ],
      ),
    );
  }
}
