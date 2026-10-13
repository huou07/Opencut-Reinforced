import 'package:flutter/material.dart';

import '../design/or_spacing.dart';
import '../project/recent_projects.dart';
import 'recent_projects_list.dart';
import '../widgets/or_widgets.dart';

class HomeScreen extends StatelessWidget {
  const HomeScreen({
    super.key,
    required this.canPickProjects,
    required this.onNewProject,
    required this.onOpenProject,
    required this.onOpenProjects,
    required this.recentProjects,
    required this.recentProjectsLoaded,
    required this.recentProjectsUnavailable,
    required this.onOpenRecentProject,
  });

  final bool canPickProjects;
  final VoidCallback onNewProject;
  final VoidCallback onOpenProject;
  final VoidCallback onOpenProjects;
  final List<RecentProject> recentProjects;
  final bool recentProjectsLoaded;
  final bool recentProjectsUnavailable;
  final ValueChanged<RecentProject> onOpenRecentProject;

  @override
  Widget build(BuildContext context) {
    const androidMessage = 'Project file access is unavailable in this build.';

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
            child: RecentProjectsList(
              projects: recentProjects.take(5).toList(growable: false),
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
