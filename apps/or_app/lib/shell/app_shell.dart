import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../core_gateway.dart';
import '../design/or_colors.dart';
import '../design/or_spacing.dart';
import '../screens/asset_library_screen.dart';
import '../screens/editor_shell_preview_screen.dart';
import '../screens/home_screen.dart';
import '../screens/projects_screen.dart';
import '../screens/settings_screen.dart';
import '../screens/templates_screen.dart';
import 'app_navigation.dart';
import 'app_top_bar.dart';
import 'command_palette.dart';

class AppShell extends StatefulWidget {
  const AppShell({super.key, required this.gateway});

  final CoreGateway gateway;

  @override
  State<AppShell> createState() => _AppShellState();
}

class _AppShellState extends State<AppShell> {
  AppDestination _destination = AppDestination.home;

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        final compact = OrBreakpoints.isCompact(constraints.maxWidth);
        final editorPreview = _destination == AppDestination.editorPreview;

        return Focus(
          autofocus: true,
          child: Shortcuts(
            shortcuts: const {
              SingleActivator(LogicalKeyboardKey.keyK, control: true):
                  _OpenCommandPaletteIntent(),
              SingleActivator(LogicalKeyboardKey.keyK, meta: true):
                  _OpenCommandPaletteIntent(),
            },
            child: Actions(
              actions: {
                _OpenCommandPaletteIntent:
                    CallbackAction<_OpenCommandPaletteIntent>(
                      onInvoke: (_) {
                        _openCommandPalette();
                        return null;
                      },
                    ),
              },
              child: Scaffold(
                body: SafeArea(
                  bottom: false,
                  child: Column(
                    children: [
                      AppTopBar(
                        title: _destination.label,
                        compact: compact,
                        onHome: () => _select(AppDestination.home),
                        onOpenCommandPalette: _openCommandPalette,
                        onExitEditorPreview: editorPreview
                            ? () => _select(AppDestination.home)
                            : null,
                      ),
                      Expanded(
                        child: Row(
                          children: [
                            if (!compact && !editorPreview) ...[
                              DesktopNavigation(
                                selected: _destination,
                                onSelected: _select,
                              ),
                              const VerticalDivider(width: 1),
                            ],
                            Expanded(child: _screen()),
                          ],
                        ),
                      ),
                    ],
                  ),
                ),
                bottomNavigationBar: compact
                    ? CompactNavigation(
                        selected: _destination,
                        onSelected: _select,
                      )
                    : null,
              ),
            ),
          ),
        );
      },
    );
  }

  Widget _screen() => switch (_destination) {
    AppDestination.home => HomeScreen(
      onUnavailable: _showUnavailable,
      onOpenEditorPreview: () => _select(AppDestination.editorPreview),
      onOpenProjects: () => _select(AppDestination.projects),
    ),
    AppDestination.projects => ProjectsScreen(onUnavailable: _showUnavailable),
    AppDestination.templates => TemplatesScreen(
      onUnavailable: _showUnavailable,
    ),
    AppDestination.assets => AssetLibraryScreen(
      onUnavailable: _showUnavailable,
    ),
    AppDestination.settings => SettingsScreen(
      gateway: widget.gateway,
      onOpenEditorPreview: () => _select(AppDestination.editorPreview),
    ),
    AppDestination.editorPreview => const EditorShellPreviewScreen(),
  };

  void _select(AppDestination destination) {
    setState(() => _destination = destination);
  }

  Future<void> _openCommandPalette() => showOrCommandPalette(context, _select);

  void _showUnavailable(String message) {
    ScaffoldMessenger.of(context)
      ..hideCurrentSnackBar()
      ..showSnackBar(
        SnackBar(
          content: Text(message),
          behavior: SnackBarBehavior.floating,
          backgroundColor: OrColors.surface,
          showCloseIcon: true,
          duration: const Duration(seconds: 3),
        ),
      );
  }
}

class _OpenCommandPaletteIntent extends Intent {
  const _OpenCommandPaletteIntent();
}
