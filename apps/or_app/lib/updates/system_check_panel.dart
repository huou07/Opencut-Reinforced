import 'dart:io';

import 'package:flutter/material.dart';
import 'package:path_provider/path_provider.dart';

import '../design/or_colors.dart';
import '../design/or_spacing.dart';
import '../design/or_typography.dart';
import '../widgets/or_widgets.dart';
import 'system_check.dart';

/// The machine the user is about to render on.
///
/// Setup used to be a sentence like "a recent computer is recommended"; this
/// panel answers the questions that decide whether an export finishes — how
/// many cores, how much memory, and whether the disk that will hold the file
/// has room for it. Everything is measured; a value the host will not report is
/// shown as unknown rather than filled in.
class SystemCheckPanel extends StatefulWidget {
  const SystemCheckPanel({super.key, this.facts});

  /// Injected in tests so a small disk can be described without one.
  final Future<HostFacts> Function()? facts;

  @override
  State<SystemCheckPanel> createState() => _SystemCheckPanelState();
}

class _SystemCheckPanelState extends State<SystemCheckPanel> {
  late final Future<HostFacts> _facts = (widget.facts ?? _measure)();

  Future<HostFacts> _measure() async {
    Directory volume;
    try {
      volume = await getApplicationSupportDirectory();
    } catch (_) {
      volume = Directory.current;
    }
    return readHostFacts(volume: volume);
  }

  @override
  Widget build(BuildContext context) {
    return OrPanel(
      key: const ValueKey('settings-system-check'),
      title: 'This machine',
      child: FutureBuilder<HostFacts>(
        future: _facts,
        builder: (context, snapshot) {
          if (snapshot.hasError) {
            return Text(
              'This machine could not be read: ${snapshot.error}. The editor '
              'still works; the rows below that need it stay hidden.',
              key: const ValueKey('system-check-error'),
              style: const TextStyle(
                color: OrColors.textSecondary,
                fontSize: OrType.metadataSize,
              ),
            );
          }
          final facts = snapshot.data;
          if (facts == null) {
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
          return Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Text(
                facts.summary,
                key: const ValueKey('system-check-summary'),
                style: const TextStyle(
                  color: OrColors.textSecondary,
                  fontSize: OrType.metadataSize,
                ),
              ),
              const SizedBox(height: OrSpacing.x2),
              _Row(
                label: 'System',
                value: '${facts.platformName} · ${facts.osVersion}',
              ),
              _Row(label: 'Processors', value: '${facts.cpuCount}'),
              _Row(
                label: 'Memory',
                value: facts.memoryBytes == null
                    ? 'unknown'
                    : formatBytes(facts.memoryBytes!),
              ),
              _Row(
                label: 'Free space',
                value: facts.freeDiskBytes == null
                    ? 'unknown'
                    : formatBytes(facts.freeDiskBytes!),
              ),
              if (!facts.hasExportHeadroom) ...[
                const SizedBox(height: OrSpacing.x2),
                Text(
                  'An export stages a copy beside the file. With less than '
                  '${formatBytes(HostFacts.exportHeadroomBytes)} free, a long '
                  'export may stop before it publishes. Free some space, or '
                  'export a shorter range.',
                  key: const ValueKey('system-check-disk-warning'),
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

class _Row extends StatelessWidget {
  const _Row({required this.label, required this.value});

  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: OrSpacing.x1),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          SizedBox(
            width: 96,
            child: Text(
              label,
              style: const TextStyle(
                color: OrColors.textMuted,
                fontSize: OrType.metadataSize,
              ),
            ),
          ),
          Expanded(
            child: Text(
              value,
              key: ValueKey('system-check-${label.toLowerCase()}'),
              style: const TextStyle(
                color: OrColors.text,
                fontSize: OrType.bodySize,
              ),
            ),
          ),
        ],
      ),
    );
  }
}
