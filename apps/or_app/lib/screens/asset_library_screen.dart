import 'package:flutter/material.dart';

import '../design/or_spacing.dart';
import '../widgets/or_widgets.dart';

class AssetLibraryScreen extends StatelessWidget {
  const AssetLibraryScreen({super.key, required this.onUnavailable});

  final ValueChanged<String> onUnavailable;

  @override
  Widget build(BuildContext context) {
    const message =
        'The asset catalog and downloads are unavailable in this Developer Preview.';

    return OrPageLayout(
      title: 'Asset Library',
      subtitle: 'Find project-ready creative resources',
      action: OrUnavailableButton(
        label: 'Downloads',
        icon: Icons.download_outlined,
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
                    OrSearchField(label: 'Search library'),
                    OrDisabledChip('License: All'),
                    OrDisabledChip('Downloads'),
                  ],
                ),
                const SizedBox(height: OrSpacing.x3),
                const Wrap(
                  spacing: OrSpacing.x2,
                  runSpacing: OrSpacing.x2,
                  children: [
                    OrDisabledChip('All'),
                    OrDisabledChip('Text Styles'),
                    OrDisabledChip('Fonts'),
                    OrDisabledChip('Music'),
                    OrDisabledChip('Sound Effects'),
                    OrDisabledChip('Stickers'),
                    OrDisabledChip('Effects'),
                    OrDisabledChip('Transitions'),
                    OrDisabledChip('Filters'),
                  ],
                ),
              ],
            ),
          ),
          const SizedBox(height: OrSpacing.x4),
          OrPanel(
            title: 'Assets',
            padding: EdgeInsets.zero,
            child: const OrEmptyState(
              title: 'No assets available',
              message: 'No asset library or project media source is connected in this Developer Preview.',
              icon: Icons.inventory_2_outlined,
            ),
          ),
        ],
      ),
    );
  }
}
