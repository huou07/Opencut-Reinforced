import 'package:flutter/material.dart';

import '../design/or_colors.dart';
import '../design/or_spacing.dart';

enum AppDestination {
  home,
  projects,
  templates,
  assets,
  settings,
  editorPreview,
}

extension AppDestinationPresentation on AppDestination {
  String get label => switch (this) {
    AppDestination.home => 'Home',
    AppDestination.projects => 'Projects',
    AppDestination.templates => 'Templates',
    AppDestination.assets => 'Asset Library',
    AppDestination.settings => 'Settings',
    AppDestination.editorPreview => 'Editor Shell Preview',
  };

  IconData get icon => switch (this) {
    AppDestination.home => Icons.home_outlined,
    AppDestination.projects => Icons.folder_outlined,
    AppDestination.templates => Icons.dashboard_outlined,
    AppDestination.assets => Icons.video_library_outlined,
    AppDestination.settings => Icons.settings_outlined,
    AppDestination.editorPreview => Icons.view_quilt_outlined,
  };

  bool get isPrimary => this != AppDestination.editorPreview;

  static const primary = [
    AppDestination.home,
    AppDestination.projects,
    AppDestination.templates,
    AppDestination.assets,
    AppDestination.settings,
  ];
}

class DesktopNavigation extends StatelessWidget {
  const DesktopNavigation({
    super.key,
    required this.selected,
    required this.onSelected,
  });

  final AppDestination selected;
  final ValueChanged<AppDestination> onSelected;

  @override
  Widget build(BuildContext context) {
    final current = selected == AppDestination.editorPreview
        ? AppDestination.settings
        : selected;

    return SizedBox(
      width: OrSpacing.sidebarWidth,
      child: Material(
        color: OrColors.backgroundRaised,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            const Padding(
              padding: EdgeInsets.fromLTRB(
                OrSpacing.x3,
                OrSpacing.x4,
                OrSpacing.x3,
                OrSpacing.x2,
              ),
              child: _NavigationHeading('WORKSPACE'),
            ),
            for (final destination in AppDestinationPresentation.primary.take(
              4,
            ))
              _DesktopDestination(
                destination: destination,
                selected: current == destination,
                onTap: () => onSelected(destination),
              ),
            const Padding(
              padding: EdgeInsets.fromLTRB(
                OrSpacing.x3,
                OrSpacing.x6,
                OrSpacing.x3,
                OrSpacing.x2,
              ),
              child: _NavigationHeading('APPLICATION'),
            ),
            _DesktopDestination(
              destination: AppDestination.settings,
              selected: current == AppDestination.settings,
              onTap: () => onSelected(AppDestination.settings),
            ),
            const Spacer(),
            const Divider(height: 1),
            const Padding(
              padding: EdgeInsets.all(OrSpacing.x4),
              child: Row(
                children: [
                  Icon(
                    Icons.circle_outlined,
                    size: 7,
                    color: OrColors.textMuted,
                  ),
                  SizedBox(width: OrSpacing.x2),
                  Text(
                    'Developer Preview',
                    style: TextStyle(
                      color: OrColors.textSecondary,
                      fontSize: 12,
                    ),
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _NavigationHeading extends StatelessWidget {
  const _NavigationHeading(this.label);

  final String label;

  @override
  Widget build(BuildContext context) {
    return Text(
      label,
      style: const TextStyle(
        color: OrColors.textMuted,
        fontSize: 10,
        fontWeight: FontWeight.w600,
        letterSpacing: 0.7,
      ),
    );
  }
}

class _DesktopDestination extends StatelessWidget {
  const _DesktopDestination({
    required this.destination,
    required this.selected,
    required this.onTap,
  });

  final AppDestination destination;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(
        horizontal: OrSpacing.x2,
        vertical: 2,
      ),
      child: Semantics(
        selected: selected,
        button: true,
        child: InkWell(
          key: ValueKey('nav-${destination.name}'),
          onTap: onTap,
          borderRadius: BorderRadius.circular(7),
          child: Container(
            constraints: const BoxConstraints(minHeight: 40),
            padding: const EdgeInsets.symmetric(horizontal: OrSpacing.x3),
            decoration: BoxDecoration(
              color: selected ? const Color(0xFF202022) : Colors.transparent,
              borderRadius: BorderRadius.circular(7),
            ),
            child: Row(
              children: [
                Icon(
                  destination.icon,
                  size: 19,
                  color: selected ? OrColors.text : OrColors.textSecondary,
                ),
                const SizedBox(width: OrSpacing.x3),
                Text(
                  destination.label,
                  style: TextStyle(
                    color: selected ? OrColors.text : OrColors.textSecondary,
                    fontSize: 13,
                    fontWeight: selected ? FontWeight.w500 : FontWeight.w400,
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class CompactNavigation extends StatelessWidget {
  const CompactNavigation({
    super.key,
    required this.selected,
    required this.onSelected,
  });

  final AppDestination selected;
  final ValueChanged<AppDestination> onSelected;

  @override
  Widget build(BuildContext context) {
    final current = selected == AppDestination.editorPreview
        ? AppDestination.settings
        : selected;

    return Container(
      key: const ValueKey('compact-navigation'),
      decoration: const BoxDecoration(
        color: OrColors.backgroundRaised,
        border: Border(top: BorderSide(color: OrColors.border)),
      ),
      child: SafeArea(
        top: false,
        child: SizedBox(
          height: 60,
          child: Row(
            children: [
              for (final destination in AppDestinationPresentation.primary)
                Expanded(
                  child: _CompactDestination(
                    destination: destination,
                    selected: current == destination,
                    onTap: () => onSelected(destination),
                  ),
                ),
            ],
          ),
        ),
      ),
    );
  }
}

class _CompactDestination extends StatelessWidget {
  const _CompactDestination({
    required this.destination,
    required this.selected,
    required this.onTap,
  });

  final AppDestination destination;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return Semantics(
      selected: selected,
      button: true,
      child: InkWell(
        key: ValueKey('nav-${destination.name}'),
        onTap: onTap,
        child: SizedBox(
          height: 60,
          child: Column(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              Icon(
                destination.icon,
                size: 20,
                color: selected ? OrColors.text : OrColors.textMuted,
              ),
              const SizedBox(height: 3),
              Padding(
                padding: const EdgeInsets.symmetric(horizontal: 1),
                child: Text(
                  destination.label,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: TextStyle(
                    color: selected ? OrColors.text : OrColors.textMuted,
                    fontSize: 10,
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
