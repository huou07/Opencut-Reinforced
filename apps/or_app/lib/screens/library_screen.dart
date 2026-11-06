import 'package:flutter/material.dart';

import '../design/or_colors.dart';
import '../design/or_spacing.dart';
import '../design/or_typography.dart';
import '../library/project_templates.dart';
import '../library/content_credits.dart';
import '../library/sound_library.dart';
import '../library/text_presets.dart';
import '../library/usage_guide.dart';
import '../project/project_gateway.dart';
import '../support/host_platform.dart';
import '../widgets/or_widgets.dart';

/// The four kinds of content the library holds.
enum LibraryTab { templates, text, sound, guide }

/// Where a reusable asset ends up: in a new project, or in the open one.
///
/// The screen never performs an edit itself. Each action hands the chosen asset
/// to the shell, which runs the ordinary project commands, so the library cannot
/// drift into a second way of changing a project.
class LibraryScreen extends StatefulWidget {
  const LibraryScreen({
    super.key,
    required this.canPickProjects,
    required this.projectOpen,
    required this.onUseTemplate,
    required this.onAddTextPreset,
    required this.onAddSoundEffect,
    required this.onOpenEditor,
    this.initialTab = LibraryTab.templates,
    this.hostPlatform,
  });

  final bool canPickProjects;
  final bool projectOpen;
  final ValueChanged<ProjectTemplate> onUseTemplate;
  final ValueChanged<TextPreset> onAddTextPreset;
  final ValueChanged<SoundEffect> onAddSoundEffect;

  /// Brings the editor forward, for the tools that only exist there.
  final VoidCallback onOpenEditor;
  final LibraryTab initialTab;

  /// Injectable so a test can render the Windows wording while running on macOS.
  final HostPlatform? hostPlatform;

  @override
  State<LibraryScreen> createState() => _LibraryScreenState();
}

class _LibraryScreenState extends State<LibraryScreen> {
  late LibraryTab _tab = widget.initialTab;
  late final HostPlatform _host = widget.hostPlatform ?? HostPlatform.current();

  @override
  Widget build(BuildContext context) {
    return OrPageLayout(
      title: 'Library',
      subtitle: 'Templates, text looks, generated sound and the guide',
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          _LibraryTabs(
            selected: _tab,
            onSelected: (tab) => setState(() => _tab = tab),
          ),
          const SizedBox(height: OrSpacing.x6),
          switch (_tab) {
            LibraryTab.templates => _TemplateGrid(
              canPickProjects: widget.canPickProjects,
              onUseTemplate: widget.onUseTemplate,
            ),
            LibraryTab.text => _TextPresetList(
              projectOpen: widget.projectOpen,
              onAddTextPreset: widget.onAddTextPreset,
              onOpenEditor: widget.onOpenEditor,
            ),
            LibraryTab.sound => _SoundList(
              projectOpen: widget.projectOpen,
              onAddSoundEffect: widget.onAddSoundEffect,
              onOpenEditor: widget.onOpenEditor,
            ),
            LibraryTab.guide => _GuideList(host: _host),
          },
        ],
      ),
    );
  }
}

class _LibraryTabs extends StatelessWidget {
  const _LibraryTabs({required this.selected, required this.onSelected});

  final LibraryTab selected;
  final ValueChanged<LibraryTab> onSelected;

  static const _labels = {
    LibraryTab.templates: 'Templates',
    LibraryTab.text: 'Text',
    LibraryTab.sound: 'Sound',
    LibraryTab.guide: 'Guide',
  };

  static const _icons = {
    LibraryTab.templates: Icons.dashboard_customize_outlined,
    LibraryTab.text: Icons.text_fields_outlined,
    LibraryTab.sound: Icons.graphic_eq_outlined,
    LibraryTab.guide: Icons.menu_book_outlined,
  };

  @override
  Widget build(BuildContext context) {
    return Container(
      key: const ValueKey('library-tabs'),
      height: 44,
      padding: const EdgeInsets.all(OrSpacing.x1),
      decoration: BoxDecoration(
        color: OrColors.backgroundRaised,
        border: Border.all(color: OrColors.border),
        borderRadius: BorderRadius.circular(OrRadii.control),
      ),
      child: Row(
        children: [
          for (final tab in LibraryTab.values)
            Expanded(
              child: Semantics(
                button: true,
                selected: selected == tab,
                label: '${_labels[tab]} library',
                child: OrFocusRing(
                  radius: OrRadii.small,
                  child: InkWell(
                    key: ValueKey('library-tab-${tab.name}'),
                    onTap: () => onSelected(tab),
                    borderRadius: BorderRadius.circular(OrRadii.small),
                    child: Container(
                      height: double.infinity,
                      alignment: Alignment.center,
                      decoration: BoxDecoration(
                        color: selected == tab
                            ? OrColors.surface
                            : Colors.transparent,
                        border: Border.all(
                          color: selected == tab
                              ? OrColors.selection
                              : Colors.transparent,
                        ),
                        borderRadius: BorderRadius.circular(OrRadii.small),
                      ),
                      child: Row(
                        mainAxisAlignment: MainAxisAlignment.center,
                        children: [
                          Icon(
                            _icons[tab],
                            size: 15,
                            color: selected == tab
                                ? OrColors.text
                                : OrColors.textMuted,
                          ),
                          const SizedBox(width: OrSpacing.x2),
                          Flexible(
                            child: Text(
                              _labels[tab]!,
                              maxLines: 1,
                              overflow: TextOverflow.ellipsis,
                              style: TextStyle(
                                color: selected == tab
                                    ? OrColors.text
                                    : OrColors.textMuted,
                                fontSize: OrType.metadataCompactSize,
                                fontWeight: selected == tab
                                    ? FontWeight.w600
                                    : FontWeight.w500,
                              ),
                            ),
                          ),
                        ],
                      ),
                    ),
                  ),
                ),
              ),
            ),
        ],
      ),
    );
  }
}

/// Two columns on a wide window, one on a narrow one, without a breakpoint
/// guess: the layout asks the width it was given.
class _TemplateGrid extends StatelessWidget {
  const _TemplateGrid({
    required this.canPickProjects,
    required this.onUseTemplate,
  });

  final bool canPickProjects;
  final ValueChanged<ProjectTemplate> onUseTemplate;

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        final columns = constraints.maxWidth >= 980 ? 2 : 1;
        final spacing = OrSpacing.x4;
        final width =
            (constraints.maxWidth - spacing * (columns - 1)) / columns;
        return Wrap(
          spacing: spacing,
          runSpacing: spacing,
          children: [
            for (final template in projectTemplates)
              SizedBox(
                width: width,
                child: _TemplateCard(
                  template: template,
                  enabled: canPickProjects,
                  onUse: () => onUseTemplate(template),
                ),
              ),
          ],
        );
      },
    );
  }
}

class _TemplateCard extends StatelessWidget {
  const _TemplateCard({
    required this.template,
    required this.enabled,
    required this.onUse,
  });

  final ProjectTemplate template;
  final bool enabled;
  final VoidCallback onUse;

  @override
  Widget build(BuildContext context) {
    return OrPanel(
      title: template.name,
      trailing: OrBadge('${template.canvasLabel} · ${template.frameRateLabel}'),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            template.summary,
            style: OrType.body.copyWith(color: OrColors.textSecondary),
          ),
          const SizedBox(height: OrSpacing.x4),
          Text(
            'Starts with',
            style: OrType.metadata.copyWith(
              color: OrColors.textMuted,
              fontWeight: FontWeight.w600,
            ),
          ),
          const SizedBox(height: OrSpacing.x2),
          for (final overlay in template.overlays)
            Padding(
              padding: const EdgeInsets.only(bottom: OrSpacing.x1),
              child: Text(
                '${overlay.label} · '
                '${_formatMilliseconds(overlay.startMilliseconds)}–'
                '${_formatMilliseconds(overlay.startMilliseconds + overlay.durationMilliseconds)}',
                style: OrType.metadata.copyWith(color: OrColors.textSecondary),
              ),
            ),
          const SizedBox(height: OrSpacing.x3),
          Text(
            'Next',
            style: OrType.metadata.copyWith(
              color: OrColors.textMuted,
              fontWeight: FontWeight.w600,
            ),
          ),
          const SizedBox(height: OrSpacing.x2),
          for (final step in template.nextSteps)
            Padding(
              padding: const EdgeInsets.only(bottom: OrSpacing.x1),
              child: Text(
                '• $step',
                style: OrType.metadata.copyWith(color: OrColors.textSecondary),
              ),
            ),
          const SizedBox(height: OrSpacing.x4),
          if (enabled)
            FilledButton.icon(
              key: ValueKey('library-template-use-${template.id}'),
              onPressed: onUse,
              icon: const Icon(Icons.add_outlined, size: 18),
              label: const Text('Use template'),
            )
          else
            OrUnavailableButton(
              key: ValueKey('library-template-use-${template.id}'),
              label: 'Use template',
              icon: Icons.add_outlined,
              reason: 'File access is unavailable in this build.',
              onPressed: onUse,
            ),
        ],
      ),
    );
  }
}

class _TextPresetList extends StatelessWidget {
  const _TextPresetList({
    required this.projectOpen,
    required this.onAddTextPreset,
    required this.onOpenEditor,
  });

  final bool projectOpen;
  final ValueChanged<TextPreset> onAddTextPreset;
  final VoidCallback onOpenEditor;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        _AvailabilityNotice(
          ready: projectOpen,
          readyText:
              'Adding a look creates a text clip at the end of the timeline. '
              'Edit or retype it like any other clip.',
          blockedText:
              'Open a project to add a text look. The values below are the '
              'exact formatting the text command uses.',
          onOpenEditor: projectOpen ? onOpenEditor : null,
        ),
        const SizedBox(height: OrSpacing.x4),
        Text(
          'Every look uses Inter or Be Vietnam Pro, both bundled under the SIL '
          'Open Font License 1.1. The Guide tab lists each licence in full.',
          key: const ValueKey('library-text-font-licence'),
          style: OrType.metadata.copyWith(color: OrColors.textMuted),
        ),
        const SizedBox(height: OrSpacing.x4),
        for (final preset in textPresets)
          Padding(
            padding: const EdgeInsets.only(bottom: OrSpacing.x3),
            child: _TextPresetCard(
              preset: preset,
              projectOpen: projectOpen,
              onAdd: () => onAddTextPreset(preset),
            ),
          ),
      ],
    );
  }
}

class _TextPresetCard extends StatelessWidget {
  const _TextPresetCard({
    required this.preset,
    required this.projectOpen,
    required this.onAdd,
  });

  final TextPreset preset;
  final bool projectOpen;
  final VoidCallback onAdd;

  @override
  Widget build(BuildContext context) {
    final formatting = preset.formatting;
    final spec = [
      _fontLabel(formatting.font),
      _weightLabel(formatting.weight),
      '${(formatting.sizeMilliPoints / 1000).toStringAsFixed(0)} pt',
      _alignmentLabel(formatting.alignment),
      _anchorLabel(formatting.anchor),
    ].join(' · ');

    return OrPanel(
      title: preset.name,
      trailing: OrBadge(preset.description),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Container(
            width: double.infinity,
            padding: const EdgeInsets.symmetric(
              horizontal: OrSpacing.x4,
              vertical: OrSpacing.x5,
            ),
            decoration: BoxDecoration(
              color: OrColors.background,
              border: Border.all(color: OrColors.border),
              borderRadius: BorderRadius.circular(OrRadii.card),
            ),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  preset.sample,
                  style: OrType.body.copyWith(
                    color: const Color(0xFFF5F5F5),
                    fontSize: 20,
                    height: 1.3,
                  ),
                ),
                const SizedBox(height: OrSpacing.x2),
                Text(
                  'Sample text, shown in the interface font. The render uses '
                  '${_fontLabel(formatting.font)}.',
                  style: OrType.tiny.copyWith(color: OrColors.textMuted),
                ),
              ],
            ),
          ),
          const SizedBox(height: OrSpacing.x3),
          Text(
            spec,
            key: ValueKey('library-text-spec-${preset.id}'),
            style: OrType.metadata.copyWith(color: OrColors.textSecondary),
          ),
          const SizedBox(height: OrSpacing.x1),
          Text(
            preset.useCase,
            style: OrType.metadata.copyWith(color: OrColors.textMuted),
          ),
          const SizedBox(height: OrSpacing.x4),
          if (projectOpen)
            OutlinedButton.icon(
              key: ValueKey('library-text-add-${preset.id}'),
              onPressed: onAdd,
              icon: const Icon(Icons.add_outlined, size: 18),
              label: const Text('Add text clip'),
            )
          else
            OrUnavailableButton(
              key: ValueKey('library-text-add-${preset.id}'),
              label: 'Add text clip',
              icon: Icons.add_outlined,
              reason: 'Open a project to add a text clip.',
              onPressed: onAdd,
            ),
        ],
      ),
    );
  }
}

class _SoundList extends StatelessWidget {
  const _SoundList({
    required this.projectOpen,
    required this.onAddSoundEffect,
    required this.onOpenEditor,
  });

  final bool projectOpen;
  final ValueChanged<SoundEffect> onAddSoundEffect;
  final VoidCallback onOpenEditor;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        _AvailabilityNotice(
          ready: projectOpen,
          readyText:
              'A sound is generated on this machine, written to the app cache '
              'and imported, so the project holds an ordinary audio file.',
          blockedText:
              'Open a project to add a sound. Generation needs no network and '
              'no download.',
          onOpenEditor: projectOpen ? onOpenEditor : null,
        ),
        const SizedBox(height: OrSpacing.x3),
        Text(
          'These effects are synthesised by the app, not recorded: no '
          'third-party audio is bundled, so nothing here needs attribution.',
          key: const ValueKey('library-sound-licence'),
          style: OrType.metadata.copyWith(color: OrColors.textMuted),
        ),
        const SizedBox(height: OrSpacing.x4),
        OrPanel(
          title: 'Music',
          trailing: const OrBadge('Generated offline'),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                'Music is longer than an effect, so it is generated in the '
                'editor: describe the bed in one sentence and the generator '
                'reads mood, tempo and length from it.',
                style: OrType.body.copyWith(color: OrColors.textSecondary),
              ),
              const SizedBox(height: OrSpacing.x4),
              if (projectOpen)
                OutlinedButton.icon(
                  key: const ValueKey('library-sound-open-music'),
                  onPressed: onOpenEditor,
                  icon: const Icon(Icons.graphic_eq_outlined, size: 18),
                  label: const Text('Open the editor'),
                )
              else
                OrUnavailableButton(
                  key: const ValueKey('library-sound-open-music'),
                  label: 'Open the editor',
                  icon: Icons.graphic_eq_outlined,
                  reason: 'Open a project to generate music.',
                  onPressed: onOpenEditor,
                ),
              const SizedBox(height: OrSpacing.x2),
              Text(
                'In the editor: Audio → Generate music…',
                style: OrType.metadata.copyWith(color: OrColors.textMuted),
              ),
            ],
          ),
        ),
        const SizedBox(height: OrSpacing.x4),
        for (final effect in soundEffects)
          Padding(
            padding: const EdgeInsets.only(bottom: OrSpacing.x3),
            child: OrPanel(
              title: effect.name,
              trailing: OrBadge(
                '${(effect.durationMilliseconds / 1000).toStringAsFixed(2)} s',
              ),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    effect.description,
                    style: OrType.body.copyWith(color: OrColors.textSecondary),
                  ),
                  const SizedBox(height: OrSpacing.x3),
                  if (projectOpen)
                    OutlinedButton.icon(
                      key: ValueKey('library-sound-add-${effect.id}'),
                      onPressed: () => onAddSoundEffect(effect),
                      icon: const Icon(Icons.add_outlined, size: 18),
                      label: const Text('Add to project'),
                    )
                  else
                    OrUnavailableButton(
                      key: ValueKey('library-sound-add-${effect.id}'),
                      label: 'Add to project',
                      icon: Icons.add_outlined,
                      reason: 'Open a project to add a sound.',
                      onPressed: () => onAddSoundEffect(effect),
                    ),
                ],
              ),
            ),
          ),
      ],
    );
  }
}

class _GuideList extends StatelessWidget {
  const _GuideList({required this.host});

  final HostPlatform host;

  @override
  Widget build(BuildContext context) {
    final commandKey = host.isMacOS ? 'Cmd' : 'Ctrl';
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        for (final section in usageGuide)
          Padding(
            padding: const EdgeInsets.only(bottom: OrSpacing.x4),
            child: OrPanel(
              key: ValueKey('library-guide-${section.id}'),
              title: section.title,
              trailing: OrBadge(section.summary),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  for (final step in section.steps)
                    Padding(
                      padding: const EdgeInsets.only(bottom: OrSpacing.x3),
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(
                            step.title,
                            style: OrType.body.copyWith(
                              color: OrColors.text,
                              fontWeight: FontWeight.w600,
                            ),
                          ),
                          const SizedBox(height: OrSpacing.x1),
                          Text(
                            step.detail,
                            style: OrType.metadata.copyWith(
                              color: OrColors.textSecondary,
                            ),
                          ),
                        ],
                      ),
                    ),
                  if (section.shortcuts.isNotEmpty) ...[
                    const SizedBox(height: OrSpacing.x2),
                    for (final shortcut in section.shortcuts)
                      Padding(
                        padding: const EdgeInsets.only(bottom: OrSpacing.x1),
                        child: Row(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            SizedBox(
                              width: 168,
                              child: Text(
                                shortcut.command
                                    ? '$commandKey+${shortcut.keys}'
                                    : shortcut.keys,
                                style: OrType.metadata.copyWith(
                                  color: OrColors.text,
                                  fontWeight: FontWeight.w600,
                                ),
                              ),
                            ),
                            Expanded(
                              child: Text(
                                shortcut.action,
                                style: OrType.metadata.copyWith(
                                  color: OrColors.textSecondary,
                                ),
                              ),
                            ),
                          ],
                        ),
                      ),
                  ],
                ],
              ),
            ),
          ),
        Padding(
          key: const ValueKey('library-shortcut-hint'),
          padding: const EdgeInsets.only(top: OrSpacing.x3),
          child: Text(
            'Press ? anywhere for every keyboard shortcut, or Ctrl/⌘ K for the '
            'command palette.',
            style: OrType.metadata.copyWith(color: OrColors.textSecondary),
          ),
        ),
        const ContentCreditsPanel(),
      ],
    );
  }
}

class _AvailabilityNotice extends StatelessWidget {
  const _AvailabilityNotice({
    required this.ready,
    required this.readyText,
    required this.blockedText,
    required this.onOpenEditor,
  });

  final bool ready;
  final String readyText;
  final String blockedText;
  final VoidCallback? onOpenEditor;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.all(OrSpacing.x4),
      decoration: BoxDecoration(
        color: OrColors.backgroundRaised,
        border: Border.all(color: OrColors.border),
        borderRadius: BorderRadius.circular(OrRadii.card),
      ),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Icon(
            ready ? Icons.check_circle_outline : Icons.info_outline,
            size: 18,
            color: ready ? OrColors.success : OrColors.textMuted,
          ),
          const SizedBox(width: OrSpacing.x3),
          Expanded(
            child: Text(
              ready ? readyText : blockedText,
              style: OrType.metadata.copyWith(color: OrColors.textSecondary),
            ),
          ),
          if (onOpenEditor != null)
            TextButton(
              key: const ValueKey('library-open-editor'),
              onPressed: onOpenEditor,
              child: const Text('Open editor'),
            ),
        ],
      ),
    );
  }
}

String _formatMilliseconds(int milliseconds) {
  final seconds = milliseconds ~/ 1000;
  final minutes = seconds ~/ 60;
  final remaining = (seconds % 60).toString().padLeft(2, '0');
  return '$minutes:$remaining';
}

String _fontLabel(ProjectFontIdentity font) => switch (font) {
  ProjectFontIdentity.bundledInter => 'Inter',
  ProjectFontIdentity.beVietnamPro => 'Be Vietnam Pro',
};

String _weightLabel(ProjectTextWeight weight) => switch (weight) {
  ProjectTextWeight.regular => 'Regular',
  ProjectTextWeight.medium => 'Medium',
  ProjectTextWeight.semibold => 'Semibold',
  ProjectTextWeight.bold => 'Bold',
};

String _alignmentLabel(ProjectTextAlignment alignment) => switch (alignment) {
  ProjectTextAlignment.start => 'left',
  ProjectTextAlignment.center => 'centre',
  ProjectTextAlignment.end => 'right',
};

String _anchorLabel(ProjectTextVerticalAnchor anchor) => switch (anchor) {
  ProjectTextVerticalAnchor.top => 'top',
  ProjectTextVerticalAnchor.center => 'middle',
  ProjectTextVerticalAnchor.bottom => 'bottom',
};
