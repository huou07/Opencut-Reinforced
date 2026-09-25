import 'package:flutter/material.dart';

import '../design/or_colors.dart';
import '../design/or_spacing.dart';
import '../project/project_gateway.dart';
import '../widgets/or_widgets.dart';

class EditorShellPreviewScreen extends StatefulWidget {
  const EditorShellPreviewScreen({
    super.key,
    this.isProjectWorkspace = false,
    this.project,
    this.notice,
    this.busy = false,
    this.onSave,
    this.onRename,
    this.onUndo,
    this.onRedo,
    this.onClose,
    this.onDiscardRecovery,
  });

  final bool isProjectWorkspace;
  final ProjectReadModel? project;
  final String? notice;
  final bool busy;
  final VoidCallback? onSave;
  final VoidCallback? onRename;
  final VoidCallback? onUndo;
  final VoidCallback? onRedo;
  final VoidCallback? onClose;
  final VoidCallback? onDiscardRecovery;

  @override
  State<EditorShellPreviewScreen> createState() =>
      _EditorShellPreviewScreenState();
}

class _EditorTool {
  const _EditorTool(this.label, this.icon);

  final String label;
  final IconData icon;
}

const _editorTools = [
  _EditorTool('Media', Icons.perm_media_outlined),
  _EditorTool('Templates', Icons.dashboard_outlined),
  _EditorTool('Text', Icons.text_fields_outlined),
  _EditorTool('Captions', Icons.subtitles_outlined),
  _EditorTool('Stickers / Shapes', Icons.category_outlined),
  _EditorTool('Audio', Icons.graphic_eq_outlined),
  _EditorTool('Effects', Icons.auto_fix_high_outlined),
  _EditorTool('Transitions', Icons.compare_arrows_outlined),
  _EditorTool('Filters / Color', Icons.tune_outlined),
  _EditorTool('AI', Icons.auto_awesome_outlined),
  _EditorTool('More', Icons.more_horiz_outlined),
];

class _EditorShellPreviewScreenState extends State<EditorShellPreviewScreen> {
  String _selectedTool = 'Media';

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      key: const ValueKey('editor-shell-preview'),
      builder: (context, constraints) {
        final compact = OrBreakpoints.isCompact(constraints.maxWidth);
        return ColoredBox(
          color: OrColors.background,
          child: Column(
            children: [
              if (widget.isProjectWorkspace)
                _ProjectWorkspaceHeader(
                  project: widget.project,
                  busy: widget.busy,
                  onSave: widget.onSave,
                  onRename: widget.onRename,
                  onUndo: widget.onUndo,
                  onRedo: widget.onRedo,
                  onClose: widget.onClose,
                )
              else
                const _EditorPreviewHeader(),
              if (widget.notice != null)
                _ProjectNotice(
                  message: widget.notice!,
                  onDiscardRecovery: widget.onDiscardRecovery,
                ),
              Expanded(
                child: compact
                    ? _compactLayout()
                    : _desktopLayout(
                        OrBreakpoints.isWide(constraints.maxWidth),
                      ),
              ),
            ],
          ),
        );
      },
    );
  }

  Widget _desktopLayout(bool wide) {
    return Column(
      children: [
        Expanded(
          flex: 3,
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              _EditorToolRail(
                selected: _selectedTool,
                onSelected: (tool) => setState(() => _selectedTool = tool),
              ),
              const VerticalDivider(width: 1),
              SizedBox(
                width: wide ? 224 : 172,
                child: _EditorToolPanel(selectedTool: _selectedTool),
              ),
              const VerticalDivider(width: 1),
              const Expanded(child: _ViewerPanel(compact: false)),
              const VerticalDivider(width: 1),
              SizedBox(width: wide ? 236 : 188, child: const _InspectorPanel()),
            ],
          ),
        ),
        const _ResizeDivider(horizontal: true),
        const _TimelineToolbar(compact: false),
        const Expanded(flex: 2, child: _TimelinePanel(compact: false)),
      ],
    );
  }

  Widget _compactLayout() {
    return Column(
      children: [
        const Expanded(flex: 4, child: _ViewerPanel(compact: true)),
        const _TimelineToolbar(compact: true),
        const Expanded(flex: 2, child: _TimelinePanel(compact: true)),
        _MobileToolDock(
          selected: _selectedTool,
          onSelected: _showMobileToolSheet,
        ),
      ],
    );
  }

  Future<void> _showMobileToolSheet(String tool) async {
    setState(() => _selectedTool = tool);
    await showModalBottomSheet<void>(
      context: context,
      useSafeArea: true,
      backgroundColor: Colors.transparent,
      builder: (context) => _UnavailableToolSheet(tool: tool),
    );
  }
}

class _ProjectWorkspaceHeader extends StatelessWidget {
  const _ProjectWorkspaceHeader({
    required this.project,
    required this.busy,
    required this.onSave,
    required this.onRename,
    required this.onUndo,
    required this.onRedo,
    required this.onClose,
  });

  final ProjectReadModel? project;
  final bool busy;
  final VoidCallback? onSave;
  final VoidCallback? onRename;
  final VoidCallback? onUndo;
  final VoidCallback? onRedo;
  final VoidCallback? onClose;

  @override
  Widget build(BuildContext context) {
    final current = project;
    return Container(
      height: 52,
      padding: const EdgeInsets.symmetric(horizontal: OrSpacing.x3),
      decoration: const BoxDecoration(
        color: OrColors.backgroundRaised,
        border: Border(bottom: BorderSide(color: OrColors.border)),
      ),
      child: Row(
        children: [
          const Icon(
            Icons.folder_open_outlined,
            size: 17,
            color: OrColors.textSecondary,
          ),
          const SizedBox(width: OrSpacing.x2),
          Expanded(
            child: Text(
              current?.name ?? 'Opening project…',
              key: const ValueKey('workspace-project-name'),
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: const TextStyle(fontSize: 12, fontWeight: FontWeight.w600),
            ),
          ),
          if (current != null) ...[
            Text(
              'Revision ${current.revision}',
              key: const ValueKey('workspace-project-revision'),
              style: const TextStyle(
                fontSize: 11,
                color: OrColors.textSecondary,
              ),
            ),
            const SizedBox(width: OrSpacing.x2),
            OrBadge(current.dirty ? 'Unsaved changes' : 'Saved'),
          ],
          const SizedBox(width: OrSpacing.x2),
          Flexible(
            child: SingleChildScrollView(
              scrollDirection: Axis.horizontal,
              child: Row(
                children: [
                  _WorkspaceAction(
                    'Rename',
                    Icons.edit_outlined,
                    onRename,
                    busy,
                  ),
                  _WorkspaceAction(
                    'Save',
                    Icons.save_outlined,
                    onSave,
                    busy || current?.dirty != true,
                  ),
                  _WorkspaceAction('Undo', Icons.undo_outlined, onUndo, busy),
                  _WorkspaceAction('Redo', Icons.redo_outlined, onRedo, busy),
                  _WorkspaceAction(
                    'Close',
                    Icons.close_outlined,
                    onClose,
                    busy,
                  ),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _WorkspaceAction extends StatelessWidget {
  const _WorkspaceAction(this.label, this.icon, this.onPressed, this.disabled);

  final String label;
  final IconData icon;
  final VoidCallback? onPressed;
  final bool disabled;

  @override
  Widget build(BuildContext context) => TextButton.icon(
    key: ValueKey('workspace-$label'.toLowerCase()),
    onPressed: disabled ? null : onPressed,
    icon: Icon(icon, size: 16),
    label: Text(label),
  );
}

class _ProjectNotice extends StatelessWidget {
  const _ProjectNotice({required this.message, this.onDiscardRecovery});

  final String message;
  final VoidCallback? onDiscardRecovery;

  @override
  Widget build(BuildContext context) {
    final staleCheckpoint = message.toLowerCase().startsWith(
      'a stale recovery checkpoint',
    );
    return Container(
      key: const ValueKey('project-notice'),
      width: double.infinity,
      padding: const EdgeInsets.symmetric(
        horizontal: OrSpacing.x3,
        vertical: OrSpacing.x2,
      ),
      color: OrColors.surface,
      child: Row(
        children: [
          const Icon(
            Icons.info_outline,
            size: 16,
            color: OrColors.textSecondary,
          ),
          const SizedBox(width: OrSpacing.x2),
          Expanded(child: Text(message, style: const TextStyle(fontSize: 12))),
          if (staleCheckpoint && onDiscardRecovery != null)
            TextButton(
              key: const ValueKey('discard-stale-recovery'),
              onPressed: onDiscardRecovery,
              child: const Text('Discard stale checkpoint'),
            ),
        ],
      ),
    );
  }
}

class _EditorPreviewHeader extends StatelessWidget {
  const _EditorPreviewHeader();

  @override
  Widget build(BuildContext context) {
    return Container(
      height: 40,
      padding: const EdgeInsets.symmetric(horizontal: OrSpacing.x3),
      decoration: const BoxDecoration(
        color: OrColors.backgroundRaised,
        border: Border(bottom: BorderSide(color: OrColors.border)),
      ),
      child: const Row(
        children: [
          Icon(
            Icons.view_quilt_outlined,
            size: 17,
            color: OrColors.textSecondary,
          ),
          SizedBox(width: OrSpacing.x2),
          Expanded(
            child: Text(
              'Editor layout preview',
              style: TextStyle(fontSize: 12, fontWeight: FontWeight.w500),
            ),
          ),
          OrBadge('Non-functional'),
        ],
      ),
    );
  }
}

class _EditorToolRail extends StatelessWidget {
  const _EditorToolRail({required this.selected, required this.onSelected});

  final String selected;
  final ValueChanged<String> onSelected;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: 52,
      child: ColoredBox(
        color: OrColors.backgroundRaised,
        child: SingleChildScrollView(
          child: Column(
            children: [
              for (final tool in _editorTools)
                Tooltip(
                  message:
                      '${tool.label} — unavailable in this Developer Preview',
                  child: Semantics(
                    button: true,
                    label:
                        '${tool.label}, unavailable in this Developer Preview',
                    child: InkWell(
                      key: ValueKey('editor-tool-${_toolKey(tool.label)}'),
                      onTap: () => onSelected(tool.label),
                      child: Container(
                        width: 48,
                        height: 48,
                        margin: const EdgeInsets.symmetric(vertical: 1),
                        decoration: BoxDecoration(
                          color: selected == tool.label
                              ? OrColors.surface
                              : Colors.transparent,
                          borderRadius: BorderRadius.circular(OrRadii.small),
                        ),
                        child: Column(
                          mainAxisAlignment: MainAxisAlignment.center,
                          children: [
                            Icon(
                              tool.icon,
                              size: 17,
                              color: OrColors.textMuted,
                            ),
                            const SizedBox(height: 3),
                            Text(
                              _shortToolLabel(tool.label),
                              maxLines: 1,
                              overflow: TextOverflow.clip,
                              style: const TextStyle(
                                color: OrColors.textMuted,
                                fontSize: 8,
                              ),
                            ),
                          ],
                        ),
                      ),
                    ),
                  ),
                ),
            ],
          ),
        ),
      ),
    );
  }
}

class _EditorToolPanel extends StatelessWidget {
  const _EditorToolPanel({required this.selectedTool});

  final String selectedTool;

  @override
  Widget build(BuildContext context) {
    final message = selectedTool == 'Media'
        ? 'Media tools unavailable in this Developer Preview'
        : 'Unavailable in this Developer Preview';

    return ColoredBox(
      color: OrColors.backgroundRaised,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          _PanelHeader(title: selectedTool),
          const Divider(height: 1),
          Expanded(
            child: Padding(
              padding: const EdgeInsets.all(OrSpacing.x4),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  const OrBadge('Unavailable'),
                  const SizedBox(height: OrSpacing.x3),
                  Text(
                    message,
                    style: const TextStyle(
                      color: OrColors.textSecondary,
                      fontSize: 13,
                      height: 1.4,
                    ),
                  ),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _ViewerPanel extends StatelessWidget {
  const _ViewerPanel({required this.compact});

  final bool compact;

  @override
  Widget build(BuildContext context) {
    return ColoredBox(
      color: OrColors.background,
      child: Column(
        children: [
          _PanelHeader(
            title: 'Viewer',
            height: compact ? 34 : 38,
            trailing: const OrBadge('Preview surface'),
          ),
          Expanded(
            child: Container(
              key: const ValueKey('editor-viewer-surface'),
              alignment: Alignment.center,
              color: OrColors.background,
              child: ConstrainedBox(
                constraints: BoxConstraints(
                  maxWidth: compact ? 520 : 660,
                  maxHeight: compact ? 360 : 420,
                ),
                child: AspectRatio(
                  aspectRatio: 16 / 9,
                  child: Container(
                    decoration: BoxDecoration(
                      color: const Color(0xFF0C0C0D),
                      border: Border.all(color: OrColors.borderStrong),
                      borderRadius: BorderRadius.circular(OrRadii.small),
                    ),
                    alignment: Alignment.center,
                    child: const Column(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        Icon(
                          Icons.movie_outlined,
                          size: 22,
                          color: OrColors.textMuted,
                        ),
                        SizedBox(height: OrSpacing.x2),
                        Text(
                          'No media loaded',
                          style: TextStyle(
                            color: OrColors.textSecondary,
                            fontSize: 13,
                          ),
                        ),
                      ],
                    ),
                  ),
                ),
              ),
            ),
          ),
          _PreviewTransport(compact: compact),
        ],
      ),
    );
  }
}

class _PreviewTransport extends StatelessWidget {
  const _PreviewTransport({required this.compact});

  final bool compact;

  @override
  Widget build(BuildContext context) {
    return Container(
      height: compact ? 42 : 44,
      decoration: const BoxDecoration(
        color: OrColors.backgroundRaised,
        border: Border(top: BorderSide(color: OrColors.border)),
      ),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Tooltip(
            message: 'Playback is unavailable in this Developer Preview',
            child: IconButton(
              key: const ValueKey('preview-previous-frame'),
              onPressed: null,
              icon: const Icon(Icons.skip_previous_outlined, size: 18),
            ),
          ),
          Tooltip(
            message: 'Playback is unavailable in this Developer Preview',
            child: IconButton(
              key: const ValueKey('preview-play'),
              onPressed: null,
              icon: const Icon(Icons.play_arrow_outlined, size: 20),
            ),
          ),
          Tooltip(
            message: 'Playback is unavailable in this Developer Preview',
            child: IconButton(
              key: const ValueKey('preview-next-frame'),
              onPressed: null,
              icon: const Icon(Icons.skip_next_outlined, size: 18),
            ),
          ),
          if (!compact) ...[
            const SizedBox(width: OrSpacing.x2),
            const Text(
              'Playback unavailable',
              style: TextStyle(color: OrColors.textMuted, fontSize: 11),
            ),
          ],
        ],
      ),
    );
  }
}

class _InspectorPanel extends StatelessWidget {
  const _InspectorPanel();

  @override
  Widget build(BuildContext context) {
    return const ColoredBox(
      color: OrColors.backgroundRaised,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          _PanelHeader(title: 'Inspector'),
          Divider(height: 1),
          Expanded(
            child: OrEmptyState(
              title: 'No selection',
              message: 'Inspector controls are unavailable in this preview.',
              icon: Icons.tune_outlined,
            ),
          ),
        ],
      ),
    );
  }
}

class _TimelineToolbar extends StatelessWidget {
  const _TimelineToolbar({required this.compact});

  final bool compact;

  @override
  Widget build(BuildContext context) {
    return Container(
      height: 40,
      padding: EdgeInsets.symmetric(
        horizontal: compact ? OrSpacing.x2 : OrSpacing.x3,
      ),
      decoration: const BoxDecoration(
        color: OrColors.backgroundRaised,
        border: Border.symmetric(
          horizontal: BorderSide(color: OrColors.border),
        ),
      ),
      child: Row(
        children: [
          Expanded(
            child: Text(
              compact ? 'Timeline preview' : 'Timeline tools unavailable',
              style: const TextStyle(
                color: OrColors.textSecondary,
                fontSize: 12,
                fontWeight: FontWeight.w500,
              ),
            ),
          ),
          _UnavailableTimelineAction(
            tooltip: 'Undo is unavailable in this Developer Preview',
            icon: Icons.undo_outlined,
          ),
          _UnavailableTimelineAction(
            tooltip: 'Split is unavailable in this Developer Preview',
            icon: Icons.content_cut_outlined,
          ),
          _UnavailableTimelineAction(
            tooltip: 'Timeline zoom is unavailable in this Developer Preview',
            icon: Icons.zoom_in_outlined,
          ),
        ],
      ),
    );
  }
}

class _UnavailableTimelineAction extends StatelessWidget {
  const _UnavailableTimelineAction({required this.tooltip, required this.icon});

  final String tooltip;
  final IconData icon;

  @override
  Widget build(BuildContext context) {
    return Tooltip(
      message: tooltip,
      child: IconButton(
        onPressed: null,
        icon: Icon(icon, size: 17),
        visualDensity: VisualDensity.compact,
      ),
    );
  }
}

class _TimelinePanel extends StatelessWidget {
  const _TimelinePanel({required this.compact});

  final bool compact;

  @override
  Widget build(BuildContext context) {
    return Container(
      key: const ValueKey('editor-timeline-region'),
      decoration: const BoxDecoration(
        color: Color(0xFF101011),
        border: Border(bottom: BorderSide(color: OrColors.border)),
      ),
      child: Column(
        children: [
          _PanelHeader(
            title: 'Timeline',
            height: compact ? 34 : 38,
            trailing: const OrBadge('Developer Preview'),
          ),
          _TimelineRuler(compact: compact),
          const Expanded(
            child: Column(
              children: [
                Expanded(child: _EmptyTimelineLane()),
                Divider(height: 1),
                Expanded(child: _EmptyTimelineLane()),
                Divider(height: 1),
                Expanded(child: _EmptyTimelineLane()),
              ],
            ),
          ),
          Padding(
            padding: const EdgeInsets.symmetric(vertical: OrSpacing.x2),
            child: Text(
              'Timeline engine not implemented',
              style: TextStyle(
                color: OrColors.textMuted,
                fontSize: compact ? 10 : 11,
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _TimelineRuler extends StatelessWidget {
  const _TimelineRuler({required this.compact});

  final bool compact;

  @override
  Widget build(BuildContext context) {
    final marks = compact
        ? const ['00:00', '00:05', '00:10']
        : const ['00:00', '00:05', '00:10', '00:15', '00:20'];

    return Container(
      height: 26,
      decoration: const BoxDecoration(
        color: OrColors.surface,
        border: Border(bottom: BorderSide(color: OrColors.border)),
      ),
      child: Row(
        children: [
          const SizedBox(width: 48),
          for (final mark in marks)
            Expanded(
              child: Container(
                alignment: Alignment.centerLeft,
                padding: const EdgeInsets.only(left: OrSpacing.x2),
                decoration: const BoxDecoration(
                  border: Border(left: BorderSide(color: OrColors.border)),
                ),
                child: Text(
                  mark,
                  style: const TextStyle(
                    color: OrColors.textMuted,
                    fontFamily: 'monospace',
                    fontSize: 10,
                  ),
                ),
              ),
            ),
        ],
      ),
    );
  }
}

class _EmptyTimelineLane extends StatelessWidget {
  const _EmptyTimelineLane();

  @override
  Widget build(BuildContext context) {
    return Semantics(
      label: 'Empty timeline lane placeholder',
      child: Row(
        children: [
          const SizedBox(
            width: 48,
            child: Center(
              child: Text('—', style: TextStyle(color: OrColors.textMuted)),
            ),
          ),
          Expanded(
            child: Row(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                for (var i = 0; i < 5; i++)
                  Expanded(
                    child: Container(
                      decoration: const BoxDecoration(
                        border: Border(
                          left: BorderSide(color: OrColors.border),
                        ),
                      ),
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

class _MobileToolDock extends StatelessWidget {
  const _MobileToolDock({required this.selected, required this.onSelected});

  final String selected;
  final ValueChanged<String> onSelected;

  @override
  Widget build(BuildContext context) {
    return Container(
      key: const ValueKey('mobile-editor-tool-dock'),
      height: 58,
      decoration: const BoxDecoration(
        color: OrColors.backgroundRaised,
        border: Border(top: BorderSide(color: OrColors.border)),
      ),
      child: SingleChildScrollView(
        scrollDirection: Axis.horizontal,
        child: Row(
          children: [
            for (final tool in _editorTools)
              Tooltip(
                message:
                    '${tool.label} — unavailable in this Developer Preview',
                child: InkWell(
                  key: ValueKey('mobile-editor-tool-${_toolKey(tool.label)}'),
                  onTap: () => onSelected(tool.label),
                  child: SizedBox(
                    width: 62,
                    height: 56,
                    child: Column(
                      mainAxisAlignment: MainAxisAlignment.center,
                      children: [
                        Icon(
                          tool.icon,
                          size: 18,
                          color: selected == tool.label
                              ? OrColors.textSecondary
                              : OrColors.textMuted,
                        ),
                        const SizedBox(height: 3),
                        Text(
                          _shortToolLabel(tool.label),
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          style: const TextStyle(
                            color: OrColors.textMuted,
                            fontSize: 9,
                          ),
                        ),
                      ],
                    ),
                  ),
                ),
              ),
          ],
        ),
      ),
    );
  }
}

class _UnavailableToolSheet extends StatelessWidget {
  const _UnavailableToolSheet({required this.tool});

  final String tool;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.all(OrSpacing.x4),
      decoration: BoxDecoration(
        color: OrColors.backgroundRaised,
        border: Border.all(color: OrColors.borderStrong),
        borderRadius: const BorderRadius.vertical(
          top: Radius.circular(OrRadii.sheet),
        ),
      ),
      child: SafeArea(
        top: false,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Row(
              children: [
                Expanded(
                  child: Text(
                    tool,
                    style: const TextStyle(
                      fontSize: 16,
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                ),
                IconButton(
                  tooltip: 'Close tool panel',
                  onPressed: () => Navigator.of(context).pop(),
                  icon: const Icon(Icons.close_outlined),
                ),
              ],
            ),
            const SizedBox(height: OrSpacing.x2),
            const Text(
              'Unavailable in this Developer Preview',
              style: TextStyle(color: OrColors.textSecondary, fontSize: 13),
            ),
            const SizedBox(height: OrSpacing.x4),
          ],
        ),
      ),
    );
  }
}

class _PanelHeader extends StatelessWidget {
  const _PanelHeader({required this.title, this.height = 38, this.trailing});

  final String title;
  final double height;
  final Widget? trailing;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      height: height,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: OrSpacing.x3),
        child: Row(
          children: [
            Expanded(
              child: Text(
                title,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: const TextStyle(
                  fontSize: 12,
                  fontWeight: FontWeight.w600,
                ),
              ),
            ),
            ?trailing,
          ],
        ),
      ),
    );
  }
}

class _ResizeDivider extends StatelessWidget {
  const _ResizeDivider({required this.horizontal});

  final bool horizontal;

  @override
  Widget build(BuildContext context) {
    return Container(
      height: horizontal ? 7 : double.infinity,
      alignment: Alignment.center,
      color: OrColors.background,
      child: Container(
        width: horizontal ? 36 : 1,
        height: horizontal ? 2 : double.infinity,
        decoration: BoxDecoration(
          color: OrColors.borderStrong,
          borderRadius: BorderRadius.circular(2),
        ),
      ),
    );
  }
}

String _shortToolLabel(String label) => switch (label) {
  'Stickers / Shapes' => 'Shapes',
  'Filters / Color' => 'Color',
  _ => label,
};

String _toolKey(String label) =>
    label.toLowerCase().replaceAll(' ', '-').replaceAll('/', '-');
