import 'package:flutter/material.dart';

import '../design/or_spacing.dart';
import '../widgets/or_widgets.dart';

class ProjectsScreen extends StatelessWidget {
  const ProjectsScreen({super.key, required this.onUnavailable});

  final ValueChanged<String> onUnavailable;

  @override
  Widget build(BuildContext context) {
    const message =
        'Project open and create UI wiring is not available in this Developer Preview.';

    return OrPageLayout(
      title: 'Projects',
      subtitle: 'Browse and organize project files',
      action: OrUnavailableButton(
        label: 'New Project',
        icon: Icons.add_outlined,
        primary: true,
        reason: message,
        onPressed: () => onUnavailable(message),
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
          OrPanel(
            title: 'Projects',
            padding: EdgeInsets.zero,
            child: const OrEmptyState(
              title: 'No projects opened from the production UI yet',
              message: 'Project browsing and file picker wiring are not available in this Developer Preview.',
            ),
          ),
        ],
      ),
    );
  }
}
