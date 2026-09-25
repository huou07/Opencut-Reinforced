import 'package:flutter/material.dart';

import '../design/or_spacing.dart';
import '../widgets/or_widgets.dart';

class TemplatesScreen extends StatelessWidget {
  const TemplatesScreen({super.key, required this.onUnavailable});

  final ValueChanged<String> onUnavailable;

  @override
  Widget build(BuildContext context) {
    const message =
        'Template browsing and creation are unavailable in this Developer Preview.';

    return OrPageLayout(
      title: 'Templates',
      subtitle: 'Browse reusable project structures',
      action: OrUnavailableButton(
        label: 'Create Template',
        icon: Icons.add_outlined,
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
                  children: const [
                    OrSearchField(label: 'Search templates'),
                    OrDisabledChip('Category: All'),
                    OrDisabledChip('Aspect ratio: Any'),
                    OrDisabledChip('License: Any'),
                  ],
                ),
                const SizedBox(height: OrSpacing.x3),
                const Wrap(
                  spacing: OrSpacing.x2,
                  runSpacing: OrSpacing.x2,
                  children: [
                    OrDisabledChip('Browse'),
                    OrDisabledChip('Community'),
                    OrDisabledChip('Downloaded'),
                    OrDisabledChip('My Templates'),
                  ],
                ),
              ],
            ),
          ),
          const SizedBox(height: OrSpacing.x4),
          OrPanel(
            title: 'Template Library',
            padding: EdgeInsets.zero,
            child: const OrEmptyState(
              title: 'No templates available',
              message: 'A template catalog is not connected in this Developer Preview.',
              icon: Icons.dashboard_outlined,
            ),
          ),
        ],
      ),
    );
  }
}
