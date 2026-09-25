import 'package:flutter/material.dart';

import '../design/or_colors.dart';
import '../design/or_spacing.dart';
import 'app_navigation.dart';

Future<void> showOrCommandPalette(
  BuildContext context,
  ValueChanged<AppDestination> onSelected, {
  List<CommandPaletteAction> actions = const [],
}) async {
  final selection = await showDialog<_PaletteSelection>(
    context: context,
    builder: (_) => _CommandPaletteDialog(actions: actions),
  );
  if (selection == null) return;
  final destination = selection.destination;
  if (destination != null) {
    onSelected(destination);
  } else {
    selection.action?.onSelected();
  }
}

class CommandPaletteAction {
  const CommandPaletteAction({required this.label, required this.onSelected});

  final String label;
  final VoidCallback onSelected;
}

class _PaletteSelection {
  const _PaletteSelection.destination(this.destination) : action = null;
  const _PaletteSelection.action(this.action) : destination = null;

  final AppDestination? destination;
  final CommandPaletteAction? action;
}

class _PaletteItem {
  const _PaletteItem({
    required this.label,
    required this.icon,
    this.destination,
    this.action,
  });

  final String label;
  final IconData icon;
  final AppDestination? destination;
  final CommandPaletteAction? action;

  _PaletteSelection get selection => destination != null
      ? _PaletteSelection.destination(destination!)
      : _PaletteSelection.action(action!);
}

class _CommandPaletteDialog extends StatefulWidget {
  const _CommandPaletteDialog({required this.actions});

  final List<CommandPaletteAction> actions;

  @override
  State<_CommandPaletteDialog> createState() => _CommandPaletteDialogState();
}

class _CommandPaletteDialogState extends State<_CommandPaletteDialog> {
  final _query = TextEditingController();

  static const _destinations = [
    ...AppDestinationPresentation.primary,
    AppDestination.editorPreview,
  ];

  @override
  void dispose() {
    _query.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final query = _query.text.trim().toLowerCase();
    final items = [
      for (final destination in _destinations)
        _PaletteItem(
          label: destination.label,
          icon: destination.icon,
          destination: destination,
        ),
      for (final action in widget.actions)
        _PaletteItem(
          label: action.label,
          icon: Icons.bolt_outlined,
          action: action,
        ),
    ];
    final matches = items
        .where((item) => item.label.toLowerCase().contains(query))
        .toList();

    return Dialog(
      backgroundColor: OrColors.backgroundRaised,
      shape: RoundedRectangleBorder(
        side: const BorderSide(color: OrColors.borderStrong),
        borderRadius: BorderRadius.circular(OrRadii.sheet),
      ),
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 520, maxHeight: 520),
        child: Padding(
          padding: const EdgeInsets.all(OrSpacing.x4),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              const Text(
                'Search commands',
                style: TextStyle(fontSize: 16, fontWeight: FontWeight.w600),
              ),
              const SizedBox(height: OrSpacing.x3),
              TextField(
                key: const ValueKey('command-palette-query'),
                controller: _query,
                autofocus: true,
                onChanged: (_) => setState(() {}),
                decoration: const InputDecoration(
                  prefixIcon: Icon(Icons.search_outlined, size: 18),
                  hintText: 'Go to a screen',
                ),
              ),
              const SizedBox(height: OrSpacing.x2),
              Flexible(
                child: matches.isEmpty
                    ? const Padding(
                        padding: EdgeInsets.all(OrSpacing.x4),
                        child: Text(
                          'No matching commands',
                          textAlign: TextAlign.center,
                          style: TextStyle(color: OrColors.textSecondary),
                        ),
                      )
                    : ListView.separated(
                        shrinkWrap: true,
                        itemCount: matches.length,
                        separatorBuilder: (_, _) => const SizedBox(height: 2),
                        itemBuilder: (context, index) {
                          final item = matches[index];
                          return InkWell(
                            key: ValueKey(
                              'command-${item.destination?.name ?? item.label.toLowerCase().replaceAll(' ', '-')}',
                            ),
                            onTap: () =>
                                Navigator.of(context).pop(item.selection),
                            borderRadius: BorderRadius.circular(OrRadii.small),
                            child: Padding(
                              padding: const EdgeInsets.symmetric(
                                horizontal: OrSpacing.x3,
                                vertical: OrSpacing.x3,
                              ),
                              child: Row(
                                children: [
                                  Icon(
                                    item.icon,
                                    size: 18,
                                    color: OrColors.textSecondary,
                                  ),
                                  const SizedBox(width: OrSpacing.x3),
                                  Expanded(child: Text(item.label)),
                                  if (item.destination ==
                                      AppDestination.editorPreview)
                                    const OrPreviewCaption(),
                                ],
                              ),
                            ),
                          );
                        },
                      ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class OrPreviewCaption extends StatelessWidget {
  const OrPreviewCaption({super.key});

  @override
  Widget build(BuildContext context) {
    return const Text(
      'Developer Preview',
      style: TextStyle(color: OrColors.textMuted, fontSize: 11),
    );
  }
}
