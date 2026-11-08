import 'dart:typed_data';

import 'package:flutter/material.dart';

import '../design/or_colors.dart';
import '../design/or_spacing.dart';
import '../design/or_typography.dart';
import '../library/creative_directions.dart';
import '../library/edit_styles.dart';
import '../library/project_templates.dart';
import '../library/template_previews.dart';
import '../motion/motion_starters.dart';
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
    required this.onApplyEditStyle,
    required this.onReviewAgentPlan,
    required this.onReadDirection,
    required this.onAddTextPreset,
    required this.onAddSoundEffect,
    required this.onOpenEditor,
    this.initialTab = LibraryTab.templates,
    this.hostPlatform,
    this.templatePreviews,
    this.myDirections = const [],
  });

  final bool canPickProjects;
  final bool projectOpen;
  final ValueChanged<ProjectTemplate> onUseTemplate;

  /// Cuts the open project's media with a style's pacing rules.
  final ValueChanged<EditStyle> onApplyEditStyle;

  /// Opens the review for a plan an assistant wrote.
  final VoidCallback onReviewAgentPlan;

  /// Opens a creative direction as a brief.
  final ValueChanged<CreativeDirection> onReadDirection;
  final ValueChanged<TextPreset> onAddTextPreset;
  final ValueChanged<SoundEffect> onAddSoundEffect;

  /// Brings the editor forward, for the tools that only exist there.
  final VoidCallback onOpenEditor;
  final LibraryTab initialTab;

  /// Injectable so a test can render the Windows wording while running on macOS.
  final HostPlatform? hostPlatform;

  /// Renders the opening still of each template; null keeps the text-only card.
  final TemplatePreviews? templatePreviews;

  /// Directions the person kept themselves, shown with the bundled ones.
  final List<CreativeDirection> myDirections;

  @override
  State<LibraryScreen> createState() => _LibraryScreenState();
}

class _LibraryScreenState extends State<LibraryScreen> {
  late LibraryTab _tab = widget.initialTab;
  late final HostPlatform _host = widget.hostPlatform ?? HostPlatform.current();

  @override
  Widget build(BuildContext context) {
    return OrPageLayout(
      scrollKey: const ValueKey('library-scroll'),
      title: 'Library',
      subtitle:
          'Project formats, edit styles, creative directions, text looks, '
          'sound and the guide',
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          _LibraryTabs(
            selected: _tab,
            onSelected: (tab) => setState(() => _tab = tab),
          ),
          const SizedBox(height: OrSpacing.x6),
          switch (_tab) {
            LibraryTab.templates => _TemplatesAndStyles(
              canPickProjects: widget.canPickProjects,
              projectOpen: widget.projectOpen,
              onUseTemplate: widget.onUseTemplate,
              onApplyEditStyle: widget.onApplyEditStyle,
              onReviewAgentPlan: widget.onReviewAgentPlan,
              onReadDirection: widget.onReadDirection,
              onOpenEditor: widget.onOpenEditor,
              templatePreviews: widget.templatePreviews,
              myDirections: widget.myDirections,
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
/// The three kinds of template, in one place, each saying what it is for.
///
/// A project format is a starter layout; an edit style decides how a cut is
/// paced; a creative direction is the brief an agent works from. They are kept
/// apart because they answer different questions and are used at different
/// moments.
class _TemplatesAndStyles extends StatelessWidget {
  const _TemplatesAndStyles({
    required this.canPickProjects,
    required this.projectOpen,
    required this.onUseTemplate,
    required this.onApplyEditStyle,
    required this.onReviewAgentPlan,
    required this.onReadDirection,
    required this.onOpenEditor,
    this.templatePreviews,
    this.myDirections = const [],
  });

  final bool canPickProjects;
  final bool projectOpen;
  final ValueChanged<ProjectTemplate> onUseTemplate;
  final ValueChanged<EditStyle> onApplyEditStyle;
  final VoidCallback onReviewAgentPlan;
  final ValueChanged<CreativeDirection> onReadDirection;
  final VoidCallback onOpenEditor;
  final TemplatePreviews? templatePreviews;
  final List<CreativeDirection> myDirections;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        const _LibrarySectionHeader(
          title: 'Project formats',
          detail:
              'Canvas and frame rate, with starter titles you can retype. No '
              'media is added and nothing is pre-cut.',
        ),
        const SizedBox(height: OrSpacing.x3),
        _TemplateGrid(
          canPickProjects: canPickProjects,
          onUseTemplate: onUseTemplate,
          templatePreviews: templatePreviews,
        ),
        const SizedBox(height: OrSpacing.x6),
        const _LibrarySectionHeader(
          title: 'Edit styles',
          detail:
              'Not a project: a way of cutting. Pick one and OR turns the media '
              'already in your project into a first cut you can keep editing.',
        ),
        const SizedBox(height: OrSpacing.x3),
        _CardGrid(
          children: [
            for (final style in bundledEditStyles)
              _StyleCard(
                style: style,
                projectOpen: projectOpen,
                onUse: () => onApplyEditStyle(style),
                onOpenEditor: onOpenEditor,
              ),
            _AgentPlanCard(
              projectOpen: projectOpen,
              onReview: onReviewAgentPlan,
              onOpenEditor: onOpenEditor,
            ),
          ],
        ),
        const SizedBox(height: OrSpacing.x6),
        const _LibrarySectionHeader(
          title: 'Creative directions',
          detail:
              'A brief, not a script: voice, story, graphics, captions, editing '
              'and sound described as a method. Copy it for an agent or keep '
              'your own version.',
        ),
        const SizedBox(height: OrSpacing.x3),
        _CardGrid(
          children: [
            for (final direction in bundledCreativeDirections)
              _DirectionCard(
                direction: direction,
                onRead: () => onReadDirection(direction),
              ),
          ],
        ),
        if (myDirections.isNotEmpty) ...[
          const SizedBox(height: OrSpacing.x6),
          const _LibrarySectionHeader(
            title: 'My directions',
            detail: 'Copies you kept and can edit.',
          ),
          const SizedBox(height: OrSpacing.x3),
          _CardGrid(
            children: [
              for (final direction in myDirections)
                _DirectionCard(
                  direction: direction,
                  onRead: () => onReadDirection(direction),
                ),
            ],
          ),
        ],
      ],
    );
  }
}

class _LibrarySectionHeader extends StatelessWidget {
  const _LibrarySectionHeader({required this.title, required this.detail});

  final String title;
  final String detail;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          title,
          style: const TextStyle(
            color: OrColors.text,
            fontSize: OrType.bodySize,
            fontWeight: FontWeight.w600,
          ),
        ),
        const SizedBox(height: OrSpacing.x1),
        Text(
          detail,
          style: const TextStyle(
            color: OrColors.textSecondary,
            fontSize: OrType.metadataSize,
          ),
        ),
      ],
    );
  }
}

/// Two columns when there is room, one when there is not.
class _CardGrid extends StatelessWidget {
  const _CardGrid({required this.children});

  final List<Widget> children;

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        final columns = constraints.maxWidth >= 980 ? 2 : 1;
        const spacing = OrSpacing.x4;
        final width =
            (constraints.maxWidth - spacing * (columns - 1)) / columns;
        return Wrap(
          spacing: spacing,
          runSpacing: spacing,
          children: [
            for (final child in children) SizedBox(width: width, child: child),
          ],
        );
      },
    );
  }
}

class _StyleCard extends StatelessWidget {
  const _StyleCard({
    required this.style,
    required this.projectOpen,
    required this.onUse,
    required this.onOpenEditor,
  });

  final EditStyle style;
  final bool projectOpen;
  final VoidCallback onUse;
  final VoidCallback onOpenEditor;

  @override
  Widget build(BuildContext context) {
    return OrPanel(
      title: style.name,
      trailing: OrBadge(style.pacingLabel),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            style.summary,
            style: const TextStyle(
              color: OrColors.text,
              fontSize: OrType.metadataSize,
            ),
          ),
          const SizedBox(height: OrSpacing.x2),
          Text(
            style.description,
            style: const TextStyle(
              color: OrColors.textSecondary,
              fontSize: OrType.metadataSize,
            ),
          ),
          const SizedBox(height: OrSpacing.x2),
          Text(
            'Shots · ${style.maxShots} at most, ${style.shotSeconds.toStringAsFixed(1)} s each'
            '${style.captions ? ' · captions on' : ''}'
            '${style.title ? ' · a title clip' : ''}',
            style: const TextStyle(
              color: OrColors.textMuted,
              fontSize: OrType.metadataSize,
            ),
          ),
          const SizedBox(height: OrSpacing.x3),
          Wrap(
            spacing: OrSpacing.x2,
            runSpacing: OrSpacing.x2,
            crossAxisAlignment: WrapCrossAlignment.center,
            children: [
              FilledButton.tonal(
                key: ValueKey('edit-style-use-${style.id}'),
                onPressed: projectOpen ? onUse : null,
                child: const Text('Build the cut'),
              ),
              if (!projectOpen)
                TextButton(
                  key: ValueKey('edit-style-need-project-${style.id}'),
                  onPressed: onOpenEditor,
                  child: const Text('Open a project first'),
                ),
            ],
          ),
          if (!projectOpen) ...[
            const SizedBox(height: OrSpacing.x1),
            const Text(
              'A style cuts the media already in an open project. Create or '
              'open one, import your footage, then come back.',
              style: TextStyle(
                color: OrColors.textMuted,
                fontSize: OrType.metadataSize,
              ),
            ),
          ],
        ],
      ),
    );
  }
}

/// The way a real model's plan enters: it is reviewed, then written as clips.
class _AgentPlanCard extends StatelessWidget {
  const _AgentPlanCard({
    required this.projectOpen,
    required this.onReview,
    required this.onOpenEditor,
  });

  final bool projectOpen;
  final VoidCallback onReview;
  final VoidCallback onOpenEditor;

  @override
  Widget build(BuildContext context) {
    return OrPanel(
      title: 'From an assistant',
      trailing: const OrBadge('AI plan'),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const Text(
            'A style is a rule; a plan is a decision about your footage.',
            style: TextStyle(
              color: OrColors.text,
              fontSize: OrType.metadataSize,
            ),
          ),
          const SizedBox(height: OrSpacing.x2),
          const Text(
            'Copy the brief into the Assistant, ask for a cut, then paste the '
            'JSON it returns. OR checks every shot against the project and '
            'shows it before one clip is added. The plan is the model\'s; the '
            'editing stays yours.',
            style: TextStyle(
              color: OrColors.textSecondary,
              fontSize: OrType.metadataSize,
            ),
          ),
          const SizedBox(height: OrSpacing.x3),
          Wrap(
            spacing: OrSpacing.x2,
            runSpacing: OrSpacing.x2,
            crossAxisAlignment: WrapCrossAlignment.center,
            children: [
              FilledButton.tonal(
                key: const ValueKey('library-agent-plan'),
                onPressed: projectOpen ? onReview : null,
                child: const Text('Review a plan…'),
              ),
              if (!projectOpen)
                TextButton(
                  key: const ValueKey('library-agent-plan-need-project'),
                  onPressed: onOpenEditor,
                  child: const Text('Open a project first'),
                ),
            ],
          ),
        ],
      ),
    );
  }
}

class _DirectionCard extends StatelessWidget {
  const _DirectionCard({required this.direction, required this.onRead});

  final CreativeDirection direction;
  final VoidCallback onRead;

  @override
  Widget build(BuildContext context) {
    return OrPanel(
      title: direction.name,
      trailing: direction.isMine ? const OrBadge('Mine') : null,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            direction.summary,
            style: const TextStyle(
              color: OrColors.text,
              fontSize: OrType.metadataSize,
            ),
          ),
          const SizedBox(height: OrSpacing.x2),
          Text(
            'Fits ${direction.bestFor.join(', ')}',
            style: const TextStyle(
              color: OrColors.textMuted,
              fontSize: OrType.metadataSize,
            ),
          ),
          const SizedBox(height: OrSpacing.x3),
          FilledButton.tonal(
            key: ValueKey('creative-direction-read-${direction.id}'),
            onPressed: onRead,
            child: const Text('Read the brief'),
          ),
        ],
      ),
    );
  }
}

class _TemplateGrid extends StatelessWidget {
  const _TemplateGrid({
    required this.canPickProjects,
    required this.onUseTemplate,
    this.templatePreviews,
  });

  final bool canPickProjects;
  final ValueChanged<ProjectTemplate> onUseTemplate;
  final TemplatePreviews? templatePreviews;

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
                  previews: templatePreviews,
                ),
              ),
          ],
        );
      },
    );
  }
}

class _TemplateCard extends StatefulWidget {
  const _TemplateCard({
    required this.template,
    required this.enabled,
    required this.onUse,
    this.previews,
  });

  final ProjectTemplate template;
  final bool enabled;
  final VoidCallback onUse;
  final TemplatePreviews? previews;

  @override
  State<_TemplateCard> createState() => _TemplateCardState();
}

class _TemplateCardState extends State<_TemplateCard> {
  Future<Uint8List?>? _still;

  @override
  void initState() {
    super.initState();
    _still = widget.previews?.still(widget.template);
  }

  @override
  Widget build(BuildContext context) {
    final template = widget.template;
    final enabled = widget.enabled;
    final onUse = widget.onUse;
    final still = _still;
    return OrPanel(
      title: template.name,
      trailing: OrBadge('${template.canvasLabel} · ${template.frameRateLabel}'),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          if (still != null) ...[
            _TemplateStill(future: still, name: template.name),
            const SizedBox(height: OrSpacing.x3),
          ],
          Text(
            template.summary,
            style: OrType.body.copyWith(color: OrColors.textSecondary),
          ),
          if (template.startsWithMotionScene) ...[
            const SizedBox(height: OrSpacing.x3),
            Text(
              'Opens with ${_starterName(template.motionStarterId!)} in the '
              'Motion panel, ready to place.',
              style: OrType.metadata.copyWith(color: OrColors.textSecondary),
            ),
          ],
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

/// The opening still of a template's scene, or nothing while it renders.
class _TemplateStill extends StatelessWidget {
  const _TemplateStill({required this.future, required this.name});

  final Future<Uint8List?> future;
  final String name;

  @override
  Widget build(BuildContext context) {
    return FutureBuilder<Uint8List?>(
      future: future,
      builder: (context, snapshot) {
        final png = snapshot.data;
        if (png == null || png.isEmpty) return const SizedBox.shrink();
        return ClipRRect(
          borderRadius: BorderRadius.circular(6),
          child: AspectRatio(
            aspectRatio: 16 / 9,
            child: Image.memory(
              png,
              fit: BoxFit.cover,
              gaplessPlayback: true,
              filterQuality: FilterQuality.medium,
              semanticLabel: '$name opening frame',
              // A still that cannot decode (a rasterizer that has no GPU
              // context, a truncated render) must cost the card its picture,
              // never the whole grid: the text card is the fallback state.
              errorBuilder: (context, error, stack) => const SizedBox.shrink(),
            ),
          ),
        );
      },
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

/// The composition name a template opens with, falling back to the raw id so a
/// renamed starter still names something.
String _starterName(String id) {
  for (final starter in motionStarters) {
    if (starter.id == id) return starter.name;
  }
  return id;
}
