import 'dart:async';

import 'package:flutter/material.dart';

import '../core_gateway.dart';
import '../design/or_colors.dart';
import '../design/or_typography.dart';
import '../design/or_spacing.dart';
import '../production/cloud_voice_settings_panel.dart';
import '../production/editor_settings.dart';
import '../production/narration_settings.dart';
import '../production/narration_settings_dialog.dart';
import '../project/project_gateway.dart';
import '../widgets/or_widgets.dart';

enum _SettingsSection { appearance, general, advanced }

typedef _Diagnostics = ({
  AppInfo appInfo,
  HealthStatus health,
  List<Capability> capabilities,
});

class SettingsScreen extends StatefulWidget {
  const SettingsScreen({
    super.key,
    required this.gateway,
    required this.project,
    required this.onCopyDescriptor,
    required this.onOpenEditorPreview,
  });

  final CoreGateway gateway;
  final ProjectReadModel? project;
  final VoidCallback onCopyDescriptor;
  final VoidCallback onOpenEditorPreview;

  @override
  State<SettingsScreen> createState() => _SettingsScreenState();
}

class _SettingsScreenState extends State<SettingsScreen> {
  _SettingsSection _selected = _SettingsSection.appearance;
  late Future<_Diagnostics> _diagnostics = _loadDiagnostics();
  final NarrationSettingsStore _narrationStore = NarrationSettingsStore();
  final EditorSettingsStore _editorStore = EditorSettingsStore();
  NarrationSettings? _narration;
  EditorSettings _editorSettings = const EditorSettings();

  @override
  void initState() {
    super.initState();
    _loadNarration();
    _loadEditorSettings();
  }

  Future<void> _loadEditorSettings() async {
    final settings = await _editorStore.load();
    if (mounted) setState(() => _editorSettings = settings);
  }

  Future<void> _loadNarration() async {
    final settings = await _narrationStore.load();
    if (mounted) setState(() => _narration = settings);
  }

  Future<void> _saveNarration(NarrationSettings settings) async {
    setState(() => _narration = settings);
    try {
      await _narrationStore.save(settings);
    } catch (_) {
      if (mounted) {
        OrSnackBar.show(context, 'The narration settings could not be saved.');
      }
    }
  }

  Future<void> _setShowInspector(bool show) async {
    final next = _editorSettings.copyWith(showInspector: show);
    setState(() => _editorSettings = next);
    try {
      await _editorStore.save(next);
    } catch (_) {
      if (mounted) {
        OrSnackBar.show(context, 'The editor preference could not be saved.');
      }
    }
  }

  /// The voice surface is app-level, so it is reachable without an open
  /// project; the same file is what `tools/voiceover.mjs` follows.
  Future<void> _chooseNarrationVoice() async {
    final current = _narration ?? await _narrationStore.load();
    if (!mounted) return;
    final chosen = await showDialog<NarrationSettings>(
      context: context,
      builder: (context) => NarrationSettingsDialog(initial: current),
    );
    if (!mounted || chosen == null) return;
    setState(() => _narration = chosen);
  }

  Future<_Diagnostics> _loadDiagnostics() async => (
    appInfo: await widget.gateway.appInfo(),
    health: await widget.gateway.health(),
    capabilities: await widget.gateway.capabilities(),
  );

  @override
  Widget build(BuildContext context) {
    return OrPageLayout(
      title: 'Settings',
      subtitle: 'Application preferences and developer information',
      child: LayoutBuilder(
        builder: (context, constraints) {
          final compact = OrBreakpoints.isCompact(constraints.maxWidth);
          final navigation = compact
              ? SingleChildScrollView(
                  scrollDirection: Axis.horizontal,
                  child: Row(
                    children: [
                      for (final section in _SettingsSection.values)
                        Padding(
                          padding: const EdgeInsets.only(right: OrSpacing.x2),
                          child: _SectionButton(
                            section: section,
                            selected: section == _selected,
                            onTap: () => setState(() => _selected = section),
                            compact: true,
                          ),
                        ),
                    ],
                  ),
                )
              : Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    for (final section in _SettingsSection.values)
                      Padding(
                        padding: const EdgeInsets.only(bottom: OrSpacing.x2),
                        child: _SectionButton(
                          section: section,
                          selected: section == _selected,
                          onTap: () => setState(() => _selected = section),
                        ),
                      ),
                  ],
                );

          final content = _sectionContent();
          if (compact) {
            return Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                navigation,
                const SizedBox(height: OrSpacing.x3),
                content,
              ],
            );
          }

          return Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              SizedBox(width: 184, child: navigation),
              const SizedBox(width: OrSpacing.x3),
              Expanded(child: content),
            ],
          );
        },
      ),
    );
  }

  Widget _sectionContent() => switch (_selected) {
    _SettingsSection.appearance => _appearanceSection(),
    _SettingsSection.general => _generalSection(),
    _SettingsSection.advanced => _advancedSection(),
  };

  Widget _appearanceSection() {
    const swatches = [
      OrColors.background,
      OrColors.backgroundRaised,
      OrColors.surface,
      OrColors.border,
      OrColors.text,
      OrColors.selection,
    ];

    return OrPanel(
      key: const ValueKey('settings-appearance'),
      title: 'Appearance',
      trailing: const OrBadge('Active'),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const Text(
            'Focused Monochrome',
            style: TextStyle(
              fontSize: OrType.sectionTitleSize,
              fontWeight: FontWeight.w600,
            ),
          ),
          const SizedBox(height: OrSpacing.x2),
          const Text(
            'The built-in appearance for this Developer Preview.',
            style: TextStyle(
              color: OrColors.textSecondary,
              fontSize: OrType.metadataSize,
            ),
          ),
          const SizedBox(height: OrSpacing.x4),
          Wrap(
            spacing: OrSpacing.x2,
            children: [
              for (final color in swatches)
                Tooltip(
                  message:
                      '#${color.toARGB32().toRadixString(16).substring(2).toUpperCase()}',
                  child: Container(
                    width: 28,
                    height: 28,
                    decoration: BoxDecoration(
                      color: color,
                      border: Border.all(color: OrColors.borderStrong),
                      borderRadius: BorderRadius.circular(OrRadii.small),
                    ),
                  ),
                ),
            ],
          ),
        ],
      ),
    );
  }

  Widget _generalSection() {
    final narration = _narration;
    final voice = narration == null || narration.voice.isEmpty
        ? 'Automatic: the first installed Vietnamese voice on this Mac'
        : '${narration.voice} (${narration.locale}) · ${narration.rate} wpm · ${narration.gapMs} ms gap';
    return OrPanel(
      key: const ValueKey('settings-general'),
      title: 'Narration',
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const Text(
            'The voice the Audio tool and the assistant use for offline '
            'narration. The choice is saved for this Mac; no audio or text '
            'leaves the machine.',
            style: TextStyle(
              color: OrColors.textSecondary,
              fontSize: OrType.metadataSize,
            ),
          ),
          const SizedBox(height: OrSpacing.x3),
          Text(
            voice,
            key: const ValueKey('settings-narration-voice-summary'),
            style: const TextStyle(fontSize: OrType.metadataSize),
          ),
          const SizedBox(height: OrSpacing.x2),
          OutlinedButton.icon(
            key: const ValueKey('settings-choose-narration-voice'),
            onPressed: _chooseNarrationVoice,
            icon: const Icon(Icons.record_voice_over_outlined, size: 16),
            label: const Text('Choose voice…'),
          ),
          const Divider(height: OrSpacing.x6),
          Text(
            'CLOUD VOICES',
            style: TextStyle(
              color: OrColors.textMuted,
              fontSize: OrType.tinySize,
              fontWeight: FontWeight.w600,
            ),
          ),
          const SizedBox(height: OrSpacing.x2),
          CloudVoiceSettingsPanel(
            settings: narration ?? const NarrationSettings(),
            onChanged: (settings) => unawaited(_saveNarration(settings)),
          ),
          const Divider(height: OrSpacing.x6),
          Material(
            type: MaterialType.transparency,
            child: SwitchListTile(
              key: const ValueKey('settings-show-inspector'),
              contentPadding: EdgeInsets.zero,
              title: const Text('Show inspector panel'),
              subtitle: const Text(
                'Advanced clip settings. The timeline toolbar and clip dialogs '
                'carry the frequent controls, so the tab starts hidden.',
              ),
              value: _editorSettings.showInspector,
              onChanged: _setShowInspector,
            ),
          ),
        ],
      ),
    );
  }

  Widget _advancedSection() {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        OrPanel(
          title: 'Developer Preview',
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              const Text(
                'The editor layout is available for visual evaluation only.',
                style: TextStyle(
                  color: OrColors.textSecondary,
                  fontSize: OrType.metadataSize,
                ),
              ),
              const SizedBox(height: OrSpacing.x3),
              OutlinedButton.icon(
                key: const ValueKey('settings-open-editor-preview'),
                onPressed: widget.onOpenEditorPreview,
                icon: const Icon(Icons.view_quilt_outlined),
                label: const Text('Open Editor Shell Preview'),
              ),
            ],
          ),
        ),
        if (widget.project != null &&
            widget.project!.descriptorPath.isNotEmpty) ...[
          const SizedBox(height: OrSpacing.x4),
          OrPanel(
            title: 'Local CLI session',
            trailing: const OrBadge('Available'),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                const Text(
                  'Descriptor',
                  style: TextStyle(
                    color: OrColors.textSecondary,
                    fontSize: OrType.metadataCompactSize,
                  ),
                ),
                const SizedBox(height: OrSpacing.x1),
                SelectableText(
                  widget.project!.descriptorPath,
                  key: const ValueKey('local-cli-descriptor-path'),
                  style: const TextStyle(fontSize: OrType.metadataCompactSize),
                ),
                const SizedBox(height: OrSpacing.x2),
                OutlinedButton.icon(
                  key: const ValueKey('copy-cli-descriptor-path'),
                  onPressed: widget.onCopyDescriptor,
                  icon: const Icon(Icons.copy_outlined, size: 16),
                  label: const Text('Copy Path'),
                ),
                const SizedBox(height: OrSpacing.x2),
                const Text(
                  'or session describe --attach "<descriptor-path>"',
                  key: ValueKey('attached-cli-example'),
                  style: TextStyle(
                    fontFamily: 'monospace',
                    fontSize: OrType.metadataCompactSize,
                  ),
                ),
                const SizedBox(height: OrSpacing.x1),
                const Text(
                  'The descriptor path is local to this device. Session credentials are never shown here.',
                  style: TextStyle(
                    color: OrColors.textSecondary,
                    fontSize: OrType.metadataCompactSize,
                  ),
                ),
              ],
            ),
          ),
        ],
        const SizedBox(height: OrSpacing.x4),
        OrPanel(
          title: 'Core Diagnostics',
          trailing: IconButton(
            key: const ValueKey('refresh-diagnostics'),
            tooltip: 'Refresh diagnostics',
            onPressed: () => setState(() => _diagnostics = _loadDiagnostics()),
            icon: const Icon(Icons.refresh_outlined, size: 18),
            color: OrColors.textSecondary,
          ),
          child: FutureBuilder<_Diagnostics>(
            future: _diagnostics,
            builder: (context, snapshot) {
              if (snapshot.connectionState != ConnectionState.done) {
                return const Row(
                  children: [
                    SizedBox(
                      width: 16,
                      height: 16,
                      child: CircularProgressIndicator(
                        strokeWidth: 2,
                        color: OrColors.textSecondary,
                      ),
                    ),
                    SizedBox(width: OrSpacing.x3),
                    Text('Loading Rust core diagnostics…'),
                  ],
                );
              }
              if (snapshot.hasError) {
                return const Text(
                  'Could not load Rust core diagnostics.',
                  key: ValueKey('diagnostics-error'),
                  style: TextStyle(color: OrColors.textSecondary),
                );
              }

              return _DiagnosticsView(diagnostics: snapshot.requireData);
            },
          ),
        ),
      ],
    );
  }
}

class _SectionButton extends StatelessWidget {
  const _SectionButton({
    required this.section,
    required this.selected,
    required this.onTap,
    this.compact = false,
  });

  final _SettingsSection section;
  final bool selected;
  final VoidCallback onTap;
  final bool compact;

  String get _label => switch (section) {
    _SettingsSection.appearance => 'Appearance',
    _SettingsSection.general => 'General',
    _SettingsSection.advanced => 'Advanced / Developer',
  };

  @override
  Widget build(BuildContext context) {
    return Semantics(
      selected: selected,
      button: true,
      child: InkWell(
        key: ValueKey('settings-section-${section.name}'),
        onTap: onTap,
        borderRadius: BorderRadius.circular(OrRadii.small),
        child: Container(
          constraints: BoxConstraints(minHeight: compact ? 40 : 42),
          padding: const EdgeInsets.symmetric(horizontal: OrSpacing.x3),
          decoration: BoxDecoration(
            color: selected ? OrColors.surface : Colors.transparent,
            border: Border.all(
              color: selected ? OrColors.border : Colors.transparent,
            ),
            borderRadius: BorderRadius.circular(OrRadii.small),
          ),
          alignment: Alignment.centerLeft,
          child: Text(
            _label,
            style: TextStyle(
              color: selected ? OrColors.text : OrColors.textSecondary,
              fontSize: OrType.metadataSize,
              fontWeight: selected ? FontWeight.w500 : FontWeight.w400,
            ),
          ),
        ),
      ),
    );
  }
}

class _DiagnosticsView extends StatelessWidget {
  const _DiagnosticsView({required this.diagnostics});

  final _Diagnostics diagnostics;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        _DiagnosticRow(
          label: 'Application name',
          value: diagnostics.appInfo.name,
        ),
        _DiagnosticRow(label: 'Version', value: diagnostics.appInfo.version),
        _DiagnosticRow(
          label: 'Core API version',
          value: '${diagnostics.appInfo.coreApiVersion}',
        ),
        _DiagnosticRow(label: 'Health', value: diagnostics.health.status),
        const Padding(
          padding: EdgeInsets.only(top: OrSpacing.x3, bottom: OrSpacing.x2),
          child: Text(
            'Capabilities',
            style: TextStyle(
              fontSize: OrType.metadataSize,
              fontWeight: FontWeight.w600,
            ),
          ),
        ),
        for (final capability in diagnostics.capabilities)
          Padding(
            padding: const EdgeInsets.only(bottom: OrSpacing.x2),
            child: Row(
              children: [
                const Icon(
                  Icons.circle_outlined,
                  size: 5,
                  color: OrColors.textMuted,
                ),
                const SizedBox(width: OrSpacing.x2),
                Expanded(child: Text(capability.id)),
                Text(
                  'v${capability.version}',
                  style: const TextStyle(
                    color: OrColors.textSecondary,
                    fontSize: OrType.metadataCompactSize,
                  ),
                ),
              ],
            ),
          ),
      ],
    );
  }
}

class _DiagnosticRow extends StatelessWidget {
  const _DiagnosticRow({required this.label, required this.value});

  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: OrSpacing.x3),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          SizedBox(
            width: 152,
            child: Text(
              label,
              style: const TextStyle(
                color: OrColors.textSecondary,
                fontSize: OrType.metadataSize,
              ),
            ),
          ),
          Expanded(child: Text(value)),
        ],
      ),
    );
  }
}
