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

/// What this machine will do for each part of a job, and why.
///
/// There is one mode. The panel reads measured facts from Rust — an adapter the
/// device actually opened, encoders that answered a probe on this boot — and
/// shows the plan those facts produced: the render device, the decoder, the
/// composite path and the encoder, each with its reason. A person who knows
/// better can pin an encoder or a graphics API; nothing else is asked of them.
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

  /// Set when the machine described by the stored fingerprint is not the
  /// machine in front of us; the panel says so instead of silently re-deciding.
  String? _machineChanged;

  /// Set by Re-detect: the check ran again on request, so its timestamp has to
  /// move even when the machine is unchanged.
  bool _stampRequested = false;

  /// The fingerprint this panel instance has already handed to the store. A
  /// caller that keeps the old settings (a test double, a store that failed to
  /// save) must not make the panel offer the same fingerprint on every frame.
  String? _recordedFingerprint;

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
  /// panel never shows a plan for an override another screen already replaced.
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
    setState(() {
      _machineChanged = null;
      _stampRequested = true;
      _measure();
    });
  }

  /// Records what the plan was decided from, and notices a machine that is no
  /// longer the one described there (a new GPU, driver or OS version).
  void _recordFingerprint(PerformanceReportView report) {
    final fingerprint = _fingerprintOf(report);
    // Re-detect has to move the timestamp even when nothing about the machine
    // changed, otherwise the panel would answer a fresh check with the old time.
    final stampOnly = _stampRequested;
    if (_recordedFingerprint == fingerprint && !stampOnly) return;
    final settings = widget.settings;
    final unchanged = settings.hardwareFingerprint == fingerprint;
    if (unchanged && !stampOnly) {
      _recordedFingerprint = fingerprint;
      return;
    }
    _recordedFingerprint = fingerprint;
    final changed = settings.hardwareFingerprint != null && !unchanged;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted) return;
      _stampRequested = false;
      if (changed && _machineChanged == null) {
        setState(() {
          _machineChanged =
              'This machine changed since the last check, so the plan was '
              'decided again.';
        });
      }
      widget.onSettingsChanged(
        settings.copyWith(
          hardwareFingerprint: fingerprint,
          hardwareCheckedAt: DateTime.now().toUtc(),
        ),
      );
    });
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
          _recordFingerprint(report);
          return _body(report);
        },
      ),
    );
  }

  Widget _body(PerformanceReportView report) {
    final settings = widget.settings;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        _machineRows(report),
        const SizedBox(height: OrSpacing.x3),
        Text(
          'Automatic',
          style: TextStyle(
            color: OrColors.text,
            fontSize: OrType.metadataSize,
            fontWeight: FontWeight.w600,
          ),
        ),
        const SizedBox(height: OrSpacing.x1),
        Text(
          'Best for this machine. OR opens what it can here, picks a path for '
          'each part of the job, and records why. Nothing is benchmarked on '
          'launch or on export, so the first file is not slowed down by a '
          'measurement.',
          key: const ValueKey('performance-automatic-copy'),
          style: const TextStyle(
            color: OrColors.textSecondary,
            fontSize: OrType.metadataSize,
          ),
        ),
        const SizedBox(height: OrSpacing.x2),
        _workloadRows(report),
        if (_machineChanged != null) ...[
          const SizedBox(height: OrSpacing.x2),
          Text(
            _machineChanged!,
            key: const ValueKey('performance-machine-changed'),
            style: const TextStyle(
              color: OrColors.warning,
              fontSize: OrType.metadataSize,
            ),
          ),
        ],
        const SizedBox(height: OrSpacing.x2),
        Wrap(
          spacing: OrSpacing.x2,
          runSpacing: OrSpacing.x2,
          crossAxisAlignment: WrapCrossAlignment.center,
          children: [
            OutlinedButton(
              key: const ValueKey('performance-redetect'),
              onPressed: _refresh,
              child: const Text('Re-detect'),
            ),
            Text(
              _checkedLabel(settings),
              key: const ValueKey('performance-checked'),
              style: const TextStyle(
                color: OrColors.textMuted,
                fontSize: OrType.metadataSize,
              ),
            ),
          ],
        ),
        const SizedBox(height: OrSpacing.x3),
        _overrides(report, settings),
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

  /// One row per part of the job: who does it, on what, and why.
  Widget _workloadRows(PerformanceReportView report) {
    final gpuError = report.gpuError ?? '';
    final backend = report.gpuBackend.isEmpty
        ? 'no graphics device'
        : _backendLabel(report.gpuBackend);
    final device = report.gpuName.isEmpty
        ? 'the default device'
        : report.gpuName;
    final decoders = report.decoders
        .where((decoder) => decoder.present)
        .map((decoder) => decoder.name)
        .join(', ');
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        _WorkloadRow(
          key: const ValueKey('performance-workload-render'),
          workload: 'Preview and render',
          choice: gpuError.isEmpty ? '$backend · $device' : 'software',
          reason: gpuError.isEmpty
              ? 'the device and backend this app opened for the viewer'
              : 'the preview device did not open ($gpuError), so frames are '
                    'composited on the CPU',
        ),
        _WorkloadRow(
          key: const ValueKey('performance-workload-composite'),
          workload: 'Composite',
          choice: gpuError.isEmpty ? backend : 'software (CPU)',
          reason: gpuError.isEmpty
              ? 'same device as the viewer, so a frame is not copied between '
                    'devices to be drawn'
              : 'no device to composite on',
        ),
        _WorkloadRow(
          key: const ValueKey('performance-workload-decode'),
          workload: 'Decode',
          choice: decoders.isEmpty ? 'software' : 'software ($decoders)',
          reason:
              'this build decodes on the CPU: a hardware decoder here '
              'hands frames back through host memory, which costs more than '
              'it saves',
        ),
        FutureBuilder<EncodingPlanView>(
          future: _plan,
          builder: (context, snapshot) {
            final plan = snapshot.data;
            return _WorkloadRow(
              key: const ValueKey('performance-workload-encode'),
              workload: 'Encode',
              choice: plan == null
                  ? (snapshot.hasError ? 'unavailable' : 'measuring…')
                  : plan.summary,
              reason: plan != null
                  ? 'the first encoder that answered a probe on this boot; '
                        'starts the file on Quality, which the export dialog '
                        'can change'
                  : snapshot.hasError
                  ? 'the plan could not be resolved: ${snapshot.error}'
                  : 'waiting for the encoder probe',
            );
          },
        ),
        FutureBuilder<EncodingPlanView>(
          future: _plan,
          builder: (context, snapshot) {
            final plan = snapshot.data;
            if (plan == null) return const SizedBox.shrink();
            return Column(
              key: const ValueKey('performance-plan-notes'),
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
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
            );
          },
        ),
      ],
    );
  }

  /// Only the overrides this machine can honour, and the one button that
  /// returns every choice to the measured default.
  Widget _overrides(PerformanceReportView report, EditorSettings settings) {
    final overridden =
        settings.preferredH264Encoder != null ||
        settings.graphicsBackend != 'auto';
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Text(
          'Overrides',
          style: TextStyle(
            color: OrColors.text,
            fontSize: OrType.metadataSize,
            fontWeight: FontWeight.w600,
          ),
        ),
        const SizedBox(height: OrSpacing.x1),
        Text(
          'Only the options this machine offers. Automatic is the measured '
          'choice; something pinned here wins until it is reset.',
          style: const TextStyle(
            color: OrColors.textSecondary,
            fontSize: OrType.metadataSize,
          ),
        ),
        const SizedBox(height: OrSpacing.x2),
        _encoderRow(report, settings),
        const SizedBox(height: OrSpacing.x2),
        _backendRow(report, settings),
        const SizedBox(height: OrSpacing.x2),
        Text(
          overridden
              ? 'Pinned: ${settings.preferredH264Encoder ?? 'the encoder is measured'}, '
                    'graphics ${settings.graphicsBackend == 'auto' ? 'automatic' : settings.graphicsBackend}.'
              : 'Nothing is pinned; this machine decides every time.',
          key: const ValueKey('performance-override-state'),
          style: const TextStyle(
            color: OrColors.textSecondary,
            fontSize: OrType.metadataSize,
          ),
        ),
        const SizedBox(height: OrSpacing.x2),
        Align(
          alignment: Alignment.centerLeft,
          child: OutlinedButton(
            key: const ValueKey('performance-reset'),
            onPressed: overridden
                ? () => _save(
                    settings
                        .withPreferredH264Encoder(null)
                        .copyWith(graphicsBackend: 'auto'),
                    backend: 'auto',
                  )
                : null,
            child: const Text('Reset to Automatic'),
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
          label: 'Graphics API',
          value: current == 'auto' ? 'Automatic' : _backendLabel(current),
        ),
        const SizedBox(height: OrSpacing.x1),
        DropdownButton<String>(
          key: const ValueKey('performance-backend'),
          value: current,
          isExpanded: true,
          dropdownColor: OrColors.surface,
          style: const TextStyle(
            color: OrColors.text,
            fontSize: OrType.metadataSize,
          ),
          underline: const SizedBox.shrink(),
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
                      ? 'Automatic (the backend that opened)'
                      : '${_backendLabel(backend)} only',
                ),
              ),
          ],
        ),
        Text(
          'A backend change applies the next time the app starts.',
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
      ],
    );
  }

  Widget _encoderRow(PerformanceReportView report, EditorSettings settings) {
    final usable = report.h264Encoders
        .where((encoder) => encoder.present && encoder.opens)
        .toList(growable: false);
    final values = <String?>[null, ...usable.map((encoder) => encoder.name)];
    if (settings.preferredH264Encoder != null &&
        !values.contains(settings.preferredH264Encoder)) {
      values.add(settings.preferredH264Encoder);
    }
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        _Row(
          label: 'Encoder',
          value: settings.preferredH264Encoder == null
              ? 'Automatic'
              : _encoderLabel(report, settings.preferredH264Encoder!),
        ),
        const SizedBox(height: OrSpacing.x1),
        DropdownButton<String?>(
          key: const ValueKey('performance-encoder'),
          value: settings.preferredH264Encoder,
          isExpanded: true,
          dropdownColor: OrColors.surface,
          style: const TextStyle(
            color: OrColors.text,
            fontSize: OrType.metadataSize,
          ),
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
        Text(
          'A named encoder that cannot open here falls back and says why.',
          style: const TextStyle(
            color: OrColors.textMuted,
            fontSize: OrType.metadataSize,
          ),
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
  ) =>
      report.h264Encoders.where((encoder) => encoder.present && !encoder.opens);

  /// "Checked 4 minutes ago", so a stale measurement is visible without
  /// repeating the machine rows.
  static String _checkedLabel(EditorSettings settings) {
    final checked = settings.hardwareCheckedAt;
    if (checked == null) return 'not checked yet';
    final elapsed = DateTime.now().toUtc().difference(checked);
    if (elapsed.inSeconds < 90) return 'checked just now';
    if (elapsed.inMinutes < 60) return 'checked ${elapsed.inMinutes} min ago';
    if (elapsed.inHours < 24) return 'checked ${elapsed.inHours} h ago';
    final days = elapsed.inDays;
    return 'checked $days ${days == 1 ? 'day' : 'days'} ago';
  }

  /// Describes the machine, not the preferences: pinning an encoder changes the
  /// plan, not the hardware, and must not read as a new machine.
  static String _fingerprintOf(PerformanceReportView report) => [
    report.cpuArch,
    report.logicalCores,
    report.gpuName,
    report.gpuBackend,
    report.gpuDriver,
    report.gpuDriverInfo,
    report.gpuCompiledBackends.join(','),
    report.decoders
        .where((decoder) => decoder.present)
        .map((decoder) => decoder.name)
        .join(','),
  ].join('|');

  static String _backendLabel(String backend) => switch (backend) {
    'metal' => 'Metal',
    'vulkan' => 'Vulkan',
    'dx12' => 'Direct3D 12',
    'gles' => 'OpenGL ES',
    '' || 'other' => 'no graphics backend',
    _ => backend,
  };
}

/// One part of the job: who does it, on what, and why.
class _WorkloadRow extends StatelessWidget {
  const _WorkloadRow({
    super.key,
    required this.workload,
    required this.choice,
    required this.reason,
  });

  final String workload;
  final String choice;
  final String reason;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: OrSpacing.x1),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          SizedBox(
            width: 132,
            child: Text(
              workload,
              style: const TextStyle(
                color: OrColors.textMuted,
                fontSize: OrType.metadataSize,
              ),
            ),
          ),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  choice,
                  style: const TextStyle(
                    color: OrColors.text,
                    fontSize: OrType.metadataSize,
                    fontWeight: FontWeight.w500,
                  ),
                ),
                Text(
                  reason,
                  style: const TextStyle(
                    color: OrColors.textSecondary,
                    fontSize: OrType.metadataSize,
                  ),
                ),
              ],
            ),
          ),
        ],
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
