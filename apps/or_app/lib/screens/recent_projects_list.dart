import 'dart:io';

import 'package:flutter/material.dart';

import '../design/or_colors.dart';
import '../project/recent_projects.dart';
import '../widgets/or_widgets.dart';

class RecentProjectsList extends StatelessWidget {
  const RecentProjectsList({
    super.key,
    required this.projects,
    required this.loaded,
    required this.unavailable,
    required this.onOpen,
  });

  final List<RecentProject> projects;
  final bool loaded;
  final bool unavailable;
  final ValueChanged<RecentProject> onOpen;

  @override
  Widget build(BuildContext context) {
    if (!loaded) {
      return const Padding(
        padding: EdgeInsets.all(20),
        child: Center(child: CircularProgressIndicator()),
      );
    }
    if (unavailable) {
      return const OrEmptyState(
        title: 'Recent projects are unavailable',
        message: 'Use Open Project to choose a project file.',
      );
    }
    if (projects.isEmpty) {
      return const OrEmptyState(
        title: 'No recent projects yet',
        message: 'Projects you open will appear here for quick access.',
      );
    }

    return Column(
      children: [
        for (final project in projects)
          Material(
            color: Colors.transparent,
            child: ListTile(
              key: ValueKey('recent-project-${project.identity}'),
              minTileHeight: 64,
              leading: const Icon(
                Icons.video_file_outlined,
                color: OrColors.textSecondary,
              ),
              title: Text(
                project.name,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
              ),
              subtitle: Tooltip(
                message: project.path,
                child: Text(
                  '${recentProjectPathLabel(project.path)} · Last opened '
                  '${MaterialLocalizations.of(context).formatMediumDate(project.lastOpened.toLocal())}',
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
              ),
              trailing: const Icon(Icons.chevron_right),
              onTap: () => onOpen(project),
            ),
          ),
      ],
    );
  }
}

/// Where a recent project lives, with the home directory shortened.
///
/// The file name is part of the label on purpose: two projects called "Repro"
/// in the same folder differ only by their file, and a folder-only subtitle
/// made them look like a duplicate row.
String recentProjectPathLabel(String path) {
  final home =
      Platform.environment['HOME'] ?? Platform.environment['USERPROFILE'];
  if (home != null && home.isNotEmpty && path.startsWith(home)) {
    final rest = path.substring(home.length);
    return rest.isEmpty ? '~' : '~$rest';
  }
  return path;
}
