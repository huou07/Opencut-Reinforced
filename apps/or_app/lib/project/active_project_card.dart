import 'package:flutter/material.dart';

import '../design/or_colors.dart';
import '../design/or_spacing.dart';
import '../widgets/or_widgets.dart';
import 'project_gateway.dart';

class ActiveProjectCard extends StatelessWidget {
  const ActiveProjectCard({
    super.key,
    required this.project,
    required this.onOpen,
    required this.onSave,
    required this.onRename,
    required this.onClose,
  });

  final ProjectReadModel project;
  final VoidCallback onOpen;
  final VoidCallback onSave;
  final VoidCallback onRename;
  final VoidCallback onClose;

  @override
  Widget build(BuildContext context) {
    return OrPanel(
      key: const ValueKey('active-project-card'),
      title: 'Active Project',
      trailing: OrBadge(project.dirty ? 'Unsaved changes' : 'Saved'),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            project.name,
            key: const ValueKey('active-project-name'),
            style: const TextStyle(fontSize: 15, fontWeight: FontWeight.w600),
          ),
          const SizedBox(height: OrSpacing.x1),
          Text(
            'Revision ${project.revision}',
            key: const ValueKey('active-project-revision'),
            style: const TextStyle(fontSize: 12, color: OrColors.textSecondary),
          ),
          const SizedBox(height: OrSpacing.x3),
          Wrap(
            spacing: OrSpacing.x2,
            runSpacing: OrSpacing.x2,
            children: [
              FilledButton.icon(
                key: const ValueKey('active-project-open'),
                onPressed: onOpen,
                icon: const Icon(Icons.open_in_new_outlined, size: 17),
                label: const Text('Open Workspace'),
              ),
              if (project.dirty)
                OutlinedButton.icon(
                  key: const ValueKey('active-project-save'),
                  onPressed: onSave,
                  icon: const Icon(Icons.save_outlined, size: 17),
                  label: const Text('Save'),
                ),
              OutlinedButton.icon(
                key: const ValueKey('active-project-rename'),
                onPressed: onRename,
                icon: const Icon(Icons.edit_outlined, size: 17),
                label: const Text('Rename'),
              ),
              TextButton.icon(
                key: const ValueKey('active-project-close'),
                onPressed: onClose,
                icon: const Icon(Icons.close_outlined, size: 17),
                label: const Text('Close Project'),
              ),
            ],
          ),
        ],
      ),
    );
  }
}
