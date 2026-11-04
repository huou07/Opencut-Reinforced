import 'package:flutter/material.dart';

import '../design/or_colors.dart';
import '../design/or_spacing.dart';
import '../design/or_typography.dart';
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
    required this.onOpenResources,
    required this.onOpenLibrary,
    required this.onOpenGuide,
    this.showWelcome = false,
    this.onDismissWelcome,
  });

  final bool canPickProjects;
  final VoidCallback onNewProject;
  final VoidCallback onOpenProject;
  final VoidCallback onOpenProjects;
  final List<RecentProject> recentProjects;
  final bool recentProjectsLoaded;
  final bool recentProjectsUnavailable;
  final ValueChanged<RecentProject> onOpenRecentProject;
  final VoidCallback onOpenResources;
  final VoidCallback onOpenLibrary;
  final VoidCallback onOpenGuide;

  /// The first-run panel. It appears until it is dismissed once, and it explains
  /// how the app stores work instead of advertising features.
  final bool showWelcome;
  final VoidCallback? onDismissWelcome;

  @override
  Widget build(BuildContext context) {
    const androidMessage = 'Project file access is unavailable in this build.';

    return OrPageLayout(
      title: 'Home',
      subtitle: 'Projects and recent work',
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          if (showWelcome) ...[
            _WelcomePanel(
              canPickProjects: canPickProjects,
              onNewProject: onNewProject,
              onOpenProject: onOpenProject,
              onOpenLibrary: onOpenLibrary,
              onOpenGuide: onOpenGuide,
              onDismiss: onDismissWelcome,
            ),
            const SizedBox(height: OrSpacing.x8),
          ],
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
              OutlinedButton.icon(
                key: const ValueKey('home-library'),
                onPressed: onOpenLibrary,
                icon: const Icon(Icons.dashboard_customize_outlined),
                label: const Text('Library'),
              ),
              OutlinedButton.icon(
                key: const ValueKey('home-resources'),
                onPressed: onOpenResources,
                icon: const Icon(Icons.download_outlined),
                label: const Text('Resources & updates'),
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

/// The first-run explanation: three facts about where work goes, then the two
/// ways to begin.
class _WelcomePanel extends StatelessWidget {
  const _WelcomePanel({
    required this.canPickProjects,
    required this.onNewProject,
    required this.onOpenProject,
    required this.onOpenLibrary,
    required this.onOpenGuide,
    required this.onDismiss,
  });

  final bool canPickProjects;
  final VoidCallback onNewProject;
  final VoidCallback onOpenProject;
  final VoidCallback onOpenLibrary;
  final VoidCallback onOpenGuide;
  final VoidCallback? onDismiss;

  @override
  Widget build(BuildContext context) {
    return OrPanel(
      key: const ValueKey('home-welcome'),
      title: 'Start here',
      trailing: onDismiss == null
          ? null
          : TextButton(
              key: const ValueKey('home-welcome-dismiss'),
              onPressed: onDismiss,
              child: const Text('Hide this'),
            ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            'Everything happens on this machine. Nothing is uploaded, and a '
            'project opens without a network connection.',
            style: OrType.body.copyWith(color: OrColors.textSecondary),
          ),
          const SizedBox(height: OrSpacing.x4),
          const _WelcomeFact(
            title: 'Your media stays where it is',
            detail:
                'Importing references the original file. Move a file later and '
                'relink that one item; the rest of the project keeps working.',
          ),
          const _WelcomeFact(
            title: 'One project file',
            detail:
                'A .orproj file holds the edit, not the footage. Save writes '
                'the whole file at once, so an interrupted save cannot damage '
                'the previous version.',
          ),
          const _WelcomeFact(
            title: 'Export runs locally',
            detail:
                'MP4, WebM or lossless Matroska, rendered on this machine with '
                'the hardware encoder when one is available.',
          ),
          const SizedBox(height: OrSpacing.x5),
          Wrap(
            spacing: OrSpacing.x3,
            runSpacing: OrSpacing.x3,
            children: [
              if (canPickProjects)
                FilledButton(
                  key: const ValueKey('home-welcome-new'),
                  onPressed: onNewProject,
                  child: const Text('New project'),
                )
              else
                OrUnavailableButton(
                  key: const ValueKey('home-welcome-new'),
                  label: 'New project',
                  reason: 'Project file access is unavailable in this build.',
                  primary: true,
                  onPressed: onNewProject,
                ),
              TextButton(
                key: const ValueKey('home-welcome-library'),
                onPressed: onOpenLibrary,
                child: const Text('Browse templates and sound'),
              ),
              TextButton(
                key: const ValueKey('home-welcome-guide'),
                onPressed: onOpenGuide,
                child: const Text('Read the guide'),
              ),
            ],
          ),
        ],
      ),
    );
  }
}

class _WelcomeFact extends StatelessWidget {
  const _WelcomeFact({required this.title, required this.detail});

  final String title;
  final String detail;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: OrSpacing.x3),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const Padding(
            padding: EdgeInsets.only(top: 6),
            child: Icon(Icons.circle, size: 5, color: OrColors.textMuted),
          ),
          const SizedBox(width: OrSpacing.x3),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  title,
                  style: OrType.body.copyWith(
                    color: OrColors.text,
                    fontWeight: FontWeight.w600,
                  ),
                ),
                const SizedBox(height: OrSpacing.x1),
                Text(
                  detail,
                  style: OrType.metadata.copyWith(color: OrColors.textSecondary),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}
