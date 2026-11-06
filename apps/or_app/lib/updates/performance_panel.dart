import 'dart:async';

import 'package:flutter/material.dart';
import 'package:or_app_bridge/or_app_bridge.dart'
    show EncodingPlanView, EncoderProbeView, PerformanceReportView;

import '../core_gateway.dart';
import '../design/or_colors.dart';
import '../design/or_spacing.dart';
import '../design/or_typography.dart';
import '../production/editor_settings.dart';
import '../widgets/or_widgets.dart';
import 'performance_options.dart';

/// How this machine will encode, and the one setting that changes it.
///
/// The panel reads measured facts from Rust — an adapter the device actually
/// opened, encoders that answered a probe on this boot — instead of guessing
/// from the platform name. A profile is a preference: the engine still refuses
/// an encoder that does not open and says which one it used instead.
class PerformancePanel extends StatefulWidget {
  const PerformancePanel({
    super.key,
    required this.gateway,
    required this.settings,
    required this.onSettingsChanged,
  });

  final CoreGateway gateway;
  final EditorSettings settings;
  final ValueChanged<EditorSettings> onSettingsChanged;

  @override
  State<PerformancePanel> createState() => _PerformancePanelState();
}

class _PerformancePanelState extends State<PerformancePanel> {
  late Future<PerformanceReportView> _report;
  late Future<EncodingPlanView> _plan;

  @override
  void initState() {
    super.initState();
    _measure();
  }

  void _measure() {
    _report = widget.gateway.performanceReport();
    _plan = _planFor(widget.settings);
  }

  Future<EncodingPlanView> _planFor(EditorSettings settings) =>
      widget.gateway.encodingPlan(
        profile: settings.performanceProfile,
        preferredH264Encoder: settings.preferredH264Encoder,
      );

  /// Saves first and resolves the plan from the value that was saved, so the
  /// panel never shows a plan for a profile the other screen already replaced.
  void _save(EditorSettings next, {String? backend}) {
    widget.onSettingsChanged(next);
    if (backend != null) {
      // Stored for the next start: the render device is opened once per
      // process, so the panel does not claim a live switch.
      unawaited(widget.gateway.setGraphicsPreference(backend));
    }
    setState(() {
      _plan = _planFor(next);
    });
  }

  void _refresh() {
    setState(_measure);
  }

  @override
  Widget build(BuildContext context) {
    return OrPanel(
      key: const ValueKey('settings-performance'),
      title: 'Performance',
      child: FutureBuilder<PerformanceReportView>(
        future: _report,
        builder: (context, snapshot) {
          if (snapshot.hasError) {
            return Text(
              'This machine could not be measured: ${snapshot.error}. Exports '
              'still work; they use the default encoder for the file type.',
              key: const ValueKey('performance-error'),
              style: const TextStyle(
                color: OrColors.textSecondary,
                fontSize: OrType.metadataSize,
              ),
            );
          }
          final report = snapshot.data;
          if (report == null) {
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
          return _body(report);
        },
      ),
    );
  }

  Widget _body(PerformanceReportView report) {
    final settings = widget.settings;
    final profile = settings.performanceProfile;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        _machineRows(report),
        const SizedBox(height: OrSpacing.x3),
        Text(
          'Export profile',
          style: TextStyle(
            color: OrColors.text,
            fontSize: OrType.metadataSize,
            fontWeight: FontWeight.w600,
          ),
        ),
        const SizedBox(height: OrSpacing.x1),
        for (final option in performanceProfileOptions)
          _ProfileTile(
            key: ValueKey('performance-profile-${option.wire}'),
            title: option.title,
            description: option.description,
            selected: option.wire == profile,
            onTap: () => _save(settings.copyWith(performanceProfile: option.wire)),
          ),
        const SizedBox(height: OrSpacing.x2),
        FutureBuilder<EncodingPlanView>(
          future: _plan,
          builder: (context, snapshot) {
            final plan = snapshot.data;
            if (plan == null) {
              return Text(
                snapshot.hasError
                    ? 'The encoder plan could not be resolved.'
                    : 'Resolving the encoder plan…',
                style: const TextStyle(
                  color: OrColors.textSecondary,
                  fontSize: OrType.metadataSize,
                ),
              );
            }
            return _planBlock(plan);
          },
        ),
        const SizedBox(height: OrSpacing.x3),
        _encoderRow(report, settings),
        const SizedBox(height: OrSpacing.x2),
        _backendRow(report, settings),
        const SizedBox(height: OrSpacing.x2),
        Align(
          alignment: Alignment.centerLeft,
          child: Wrap(
            spacing: OrSpacing.x2,
            runSpacing: OrSpacing.x2,
            children: [
              OutlinedButton(
                key: const ValueKey('performance-reset'),
                onPressed: () => _save(
                  settings
                      .copyWith(performanceProfile: 'recommended')
                      .withPreferredH264Encoder(null)
                  .copyWith(graphicsBackend: 'auto'),
                  backend: 'auto',
                ),
                child: const Text('Reset to recommended'),
              ),
              TextButton(
                key: const ValueKey('performance-refresh'),
                onPressed: _refresh,
                child: const Text('Measure again'),
              ),
            ],
          ),
        ),
      ],
    );
  }

  Widget _backendRow(PerformanceReportView report, EditorSettings settings) {
    final options = ['auto', ...report.gpuCompiledBackends];
    final current = options.contains(settings.graphicsBackend)
        ? settings.graphicsBackend
        : 'auto';
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        _Row(
          label: 'Graphics backend',
          value: current == 'auto'
              ? 'Automatic'
              : _backendLabel(current),
        ),
        const SizedBox(height: OrSpacing.x1),
        DropdownButton<String>(
          key: const ValueKey('performance-backend'),
          value: current,
          isExpanded: true,
          onChanged: (value) {
            if (value == null || value == current) return;
            _save(settings.copyWith(graphicsBackend: value), backend: value);
          },
          items: [
            for (final backend in options)
              DropdownMenuItem(
                value: backend,
                child: Text(
                  backend == 'auto'
                      ? 'Automatic (all compiled backends)'
                      : _backendLabel(backend),
                ),
              ),
          ],
        ),
        Text(
          'Changes apply the next time the app starts.',
          style: const TextStyle(
            color: OrColors.textMuted,
            fontSize: OrType.metadataSize,
          ),
        ),
      ],
    );
  }

  Widget _machineRows(PerformanceReportView report) {
    final gpuError = report.gpuError ?? '';
    final gpu = report.gpuName.isEmpty
        ? (gpuError.isEmpty ? 'unknown' : 'unavailable')
        : '${report.gpuName} · ${_backendLabel(report.gpuBackend)}'
              '${report.gpuDeviceType.isEmpty ? '' : ' · ${report.gpuDeviceType}'}';
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Text(
          'Measured on this machine, not assumed from the platform.',
          key: const ValueKey('performance-summary'),
          style: const TextStyle(
            color: OrColors.textSecondary,
            fontSize: OrType.metadataSize,
          ),
        ),
        const SizedBox(height: OrSpacing.x2),
        _Row(
          label: 'Processor',
          value: '${report.cpuArch} · ${report.logicalCores} threads',
        ),
        _Row(label: 'Graphics', value: gpu),
        if (report.gpuDriver.isNotEmpty)
          _Row(
            label: 'Driver',
            value: report.gpuDriverInfo.isEmpty
                ? report.gpuDriver
                : '${report.gpuDriver} · ${report.gpuDriverInfo}',
          ),
        _Row(
          label: 'Graphics APIs',
          value: report.gpuCompiledBackends.isEmpty
              ? 'none compiled'
              : report.gpuCompiledBackends.join(', '),
        ),
        _Row(
          label: 'Decoders',
          value: report.decoders.isEmpty
              ? 'none probed'
              : report.decoders
                    .where((decoder) => decoder.present)
                    .map((decoder) => decoder.name)
                    .join(', '),
        ),
        if (gpuError.isNotEmpty) ...[
          const SizedBox(height: OrSpacing.x2),
          Text(
            'The preview device did not open: $gpuError. Exports fall '
            'back to software rendering.',
            key: const ValueKey('performance-gpu-error'),
            style: const TextStyle(
              color: OrColors.warning,
              fontSize: OrType.metadataSize,
            ),
          ),
        ],
        if (_unopenedEncoders(report).isNotEmpty) ...[
          const SizedBox(height: OrSpacing.x2),
          Text(
            _unopenedEncoders(report)
                .map(
                  (encoder) =>
                      '${encoder.name}: ${encoder.note ?? 'did not open'}',
                )
                .join('\n'),
            key: const ValueKey('performance-unavailable'),
            style: const TextStyle(
              color: OrColors.textMuted,
              fontSize: OrType.metadataSize,
            ),
          ),
        ],
      ],
    );
  }

  Widget _planBlock(EncodingPlanView plan) {
    return Container(
      padding: const EdgeInsets.all(OrSpacing.x3),
      decoration: BoxDecoration(
        color: OrColors.surface,
        borderRadius: BorderRadius.circular(OrRadii.small),
      ),
      child: Column(
        key: const ValueKey('performance-plan-notes'),
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(
            'This machine will encode with ${plan.summary}.',
            key: const ValueKey('performance-plan-summary'),
            style: const TextStyle(color: OrColors.text, fontSize: OrType.metadataSize),
          ),
          Text(
            'Starts the file on ${plan.quality == 'speed' ? 'Speed' : 'Quality'}.',
            style: const TextStyle(
              color: OrColors.textSecondary,
              fontSize: OrType.metadataSize,
            ),
          ),
          if (plan.fallback != null) ...[
            const SizedBox(height: OrSpacing.x1),
            Text(
              'Fell back: ${plan.fallback}.',
              key: const ValueKey('performance-plan-fallback'),
              style: const TextStyle(
                color: OrColors.warning,
                fontSize: OrType.metadataSize,
              ),
            ),
          ],
          for (final (index, note) in plan.notes.indexed) ...[
            const SizedBox(height: OrSpacing.x1),
            Text(
              note,
              key: ValueKey('performance-plan-note-$index'),
              style: const TextStyle(
                color: OrColors.textSecondary,
                fontSize: OrType.metadataSize,
              ),
            ),
          ],
        ],
      ),
    );
  }

  Widget _encoderRow(
    PerformanceReportView report,
    EditorSettings settings,
  ) {
    final usable = report.h264Encoders
        .where((encoder) => encoder.present && encoder.opens)
        .toList(growable: false);
    final values = <String?>[null, ...usable.map((encoder) => encoder.name)];
    if (settings.preferredH264Encoder != null &&
        !values.contains(settings.preferredH264Encoder)) {
      values.add(settings.preferredH264Encoder);
    }
    return Row(
      crossAxisAlignment: CrossAxisAlignment.center,
      children: [
        Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                'Encoder',
                style: TextStyle(
                  color: OrColors.text,
                  fontSize: OrType.metadataSize,
                  fontWeight: FontWeight.w600,
                ),
              ),
              Text(
                'Automatic lets the profile decide. A named encoder that '
                'cannot open falls back and says why.',
                style: const TextStyle(
                  color: OrColors.textSecondary,
                  fontSize: OrType.metadataSize,
                ),
              ),
            ],
          ),
        ),
        const SizedBox(width: OrSpacing.x3),
        DropdownButton<String?>(
          key: const ValueKey('performance-encoder'),
          value: settings.preferredH264Encoder,
          dropdownColor: OrColors.surface,
          style: const TextStyle(color: OrColors.text, fontSize: OrType.metadataSize),
          underline: const SizedBox.shrink(),
          items: [
            for (final value in values)
              DropdownMenuItem<String?>(
                value: value,
                child: Text(
                  value == null ? 'Automatic' : _encoderLabel(report, value),
                ),
              ),
          ],
          onChanged: (value) => _save(settings.withPreferredH264Encoder(value)),
        ),
      ],
    );
  }

  String _encoderLabel(PerformanceReportView report, String name) {
    final encoder = report.h264Encoders.where((item) => item.name == name);
    if (encoder.isNotEmpty && encoder.first.hardware) return '$name (hardware)';
    return name;
  }

  static Iterable<EncoderProbeView> _unopenedEncoders(
    PerformanceReportView report,
  ) => report.h264Encoders.where((encoder) => encoder.present && !encoder.opens);

  static String _backendLabel(String backend) => switch (backend) {
    'metal' => 'Metal',
    'vulkan' => 'Vulkan',
    'dx12' => 'Direct3D 12',
    'gles' => 'OpenGL ES',
    '' || 'other' => 'no graphics backend',
    _ => backend,
  };
}


class _ProfileTile extends StatelessWidget {
  const _ProfileTile({
    super.key,
    required this.title,
    required this.description,
    required this.selected,
    required this.onTap,
  });

  final String title;
  final String description;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return Semantics(
      selected: selected,
      button: true,
      child: InkWell(
        onTap: onTap,
        borderRadius: BorderRadius.circular(OrRadii.small),
        child: Container(
          margin: const EdgeInsets.only(bottom: OrSpacing.x1),
          padding: const EdgeInsets.symmetric(
            horizontal: OrSpacing.x3,
            vertical: OrSpacing.x2,
          ),
          decoration: BoxDecoration(
            color: selected ? OrColors.surfaceSelected : Colors.transparent,
            border: Border.all(
              color: selected ? OrColors.borderStrong : OrColors.border,
            ),
            borderRadius: BorderRadius.circular(OrRadii.small),
          ),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                title,
                style: TextStyle(
                  color: OrColors.text,
                  fontSize: OrType.metadataSize,
                  fontWeight: selected ? FontWeight.w600 : FontWeight.w500,
                ),
              ),
              const SizedBox(height: 2),
              Text(
                description,
                style: const TextStyle(
                  color: OrColors.textSecondary,
                  fontSize: OrType.metadataSize,
                ),
              ),
            ],
          ),
        ),
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
      padding: const EdgeInsets.symmetric(vertical: 2),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          SizedBox(
            width: 132,
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
              style: const TextStyle(
                color: OrColors.text,
                fontSize: OrType.metadataSize,
              ),
            ),
          ),
        ],
      ),
    );
  }
}
