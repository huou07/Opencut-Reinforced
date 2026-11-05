import 'package:flutter/material.dart';

import '../design/or_colors.dart';
import '../design/or_spacing.dart';
import '../design/or_typography.dart';
import '../project/export_capability.dart';
import '../project/export_profile.dart';
import '../widgets/or_widgets.dart';

/// What this build can actually export, measured by the linked encoder.
///
/// A format appears here only when the writer could open its encoder, so the
/// panel answers "will my MP4 export work" without the user starting one. The
/// measurement is the engine's own probe, not a list of names kept in Dart.
class ExportCapabilityPanel extends StatefulWidget {
  const ExportCapabilityPanel({super.key, this.capability});

  /// Injected in tests and by a host that already measured the build.
  final ExportCapabilityCache? capability;

  @override
  State<ExportCapabilityPanel> createState() => _ExportCapabilityPanelState();
}

class _ExportCapabilityPanelState extends State<ExportCapabilityPanel> {
  late final Future<ExportCapability> _capability =
      (widget.capability ?? ExportCapabilityCache()).load();

  @override
  Widget build(BuildContext context) {
    return OrPanel(
      key: const ValueKey('settings-export-formats'),
      title: 'Export formats',
      child: FutureBuilder<ExportCapability>(
        future: _capability,
        builder: (context, snapshot) {
          final capability = snapshot.data;
          if (capability == null) {
            return const SizedBox(
              height: 20,
              child: Align(
                alignment: Alignment.centerLeft,
                child: SizedBox.square(
                  dimension: 16,
                  child: CircularProgressIndicator(strokeWidth: 2),
                ),
              ),
            );
          }
          final caveat = capability.caveat;
          return Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              for (final profile in ExportProfile.values)
                _FormatRow(profile: profile, capability: capability),
              const SizedBox(height: OrSpacing.x2),
              Text(
                capability.summary,
                key: const ValueKey('export-capability-summary'),
                style: const TextStyle(
                  color: OrColors.textSecondary,
                  fontSize: OrType.metadataSize,
                ),
              ),
              if (caveat.isNotEmpty) ...[
                const SizedBox(height: OrSpacing.x2),
                Text(
                  caveat,
                  key: const ValueKey('export-capability-caveat'),
                  style: const TextStyle(
                    color: OrColors.warning,
                    fontSize: OrType.metadataSize,
                  ),
                ),
              ],
            ],
          );
        },
      ),
    );
  }
}

class _FormatRow extends StatelessWidget {
  const _FormatRow({required this.profile, required this.capability});

  final ExportProfile profile;
  final ExportCapability capability;

  @override
  Widget build(BuildContext context) {
    final ready = capability.supports(profile);
    final label = switch (profile.extension) {
      'mp4' => 'MP4 · H.264 and AAC',
      'webm' => 'WebM · VP9 and Opus',
      _ => 'Lossless · FFV1 and PCM',
    };
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: OrSpacing.x1),
      child: Row(
        children: [
          Icon(
            ready ? Icons.check_circle_outline : Icons.remove_circle_outline,
            size: 16,
            color: ready ? OrColors.success : OrColors.textMuted,
          ),
          const SizedBox(width: OrSpacing.x2),
          Expanded(
            child: Text(
              label,
              key: ValueKey('export-capability-${profile.extension}'),
              style: TextStyle(
                color: ready ? OrColors.text : OrColors.textMuted,
                fontSize: OrType.bodySize,
              ),
            ),
          ),
          Text(
            ready ? 'Ready' : 'Unavailable',
            style: TextStyle(
              color: ready ? OrColors.textSecondary : OrColors.textMuted,
              fontSize: OrType.tinySize,
            ),
          ),
        ],
      ),
    );
  }
}
