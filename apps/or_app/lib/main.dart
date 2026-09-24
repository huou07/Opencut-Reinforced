import 'package:flutter/material.dart';

void main() {
  runApp(const OrApp());
}

class OrApp extends StatelessWidget {
  const OrApp({super.key});

  @override
  Widget build(BuildContext context) {
    const surface = Color(0xFF181818);
    const border = Color(0xFF303030);

    return MaterialApp(
      title: 'Opencut Reinforced',
      debugShowCheckedModeBanner: false,
      theme: ThemeData(
        useMaterial3: true,
        brightness: Brightness.dark,
        scaffoldBackgroundColor: const Color(0xFF101010),
        colorScheme: const ColorScheme.dark(
          primary: Color(0xFFE5E5E5),
          onPrimary: Color(0xFF101010),
          secondary: Color(0xFFB8B8B8),
          surface: surface,
          onSurface: Color(0xFFE5E5E5),
          outline: border,
        ),
        dividerColor: border,
        navigationBarTheme: const NavigationBarThemeData(
          backgroundColor: surface,
          indicatorColor: Color(0xFF303030),
          labelTextStyle: WidgetStatePropertyAll(
            TextStyle(fontSize: 11, fontWeight: FontWeight.w500),
          ),
        ),
      ),
      home: const _Workspace(),
    );
  }
}

enum _Area { home, projects, templates, assets, settings }

extension on _Area {
  String get label => switch (this) {
    _Area.home => 'Home',
    _Area.projects => 'Projects',
    _Area.templates => 'Templates',
    _Area.assets => 'Asset Library',
    _Area.settings => 'Settings',
  };

  IconData get icon => switch (this) {
    _Area.home => Icons.home_outlined,
    _Area.projects => Icons.folder_outlined,
    _Area.templates => Icons.dashboard_outlined,
    _Area.assets => Icons.video_library_outlined,
    _Area.settings => Icons.settings_outlined,
  };
}

class _Workspace extends StatefulWidget {
  const _Workspace();

  @override
  State<_Workspace> createState() => _WorkspaceState();
}

class _WorkspaceState extends State<_Workspace> {
  _Area _selected = _Area.home;

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        final compact = constraints.maxWidth < 720;
        final areas = _Area.values;

        return Scaffold(
          body: Row(
            children: [
              if (!compact) ...[
                SizedBox(
                  width: 232,
                  child: _SideNavigation(
                    selected: _selected,
                    onSelected: _select,
                  ),
                ),
                const VerticalDivider(width: 1, thickness: 1),
              ],
              Expanded(child: _AreaPage(area: _selected)),
            ],
          ),
          bottomNavigationBar: compact
              ? NavigationBar(
                  selectedIndex: _selected.index,
                  onDestinationSelected: (index) => _select(areas[index]),
                  destinations: [
                    for (final area in areas)
                      NavigationDestination(
                        icon: Icon(area.icon),
                        label: area.label,
                      ),
                  ],
                )
              : null,
        );
      },
    );
  }

  void _select(_Area area) => setState(() => _selected = area);
}

class _SideNavigation extends StatelessWidget {
  const _SideNavigation({required this.selected, required this.onSelected});

  final _Area selected;
  final ValueChanged<_Area> onSelected;

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;

    return Material(
      color: colors.surface,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          const Padding(
            padding: EdgeInsets.fromLTRB(20, 24, 16, 24),
            child: Text(
              'Opencut Reinforced',
              style: TextStyle(fontSize: 15, fontWeight: FontWeight.w600),
            ),
          ),
          const Divider(height: 1),
          const SizedBox(height: 12),
          for (final area in _Area.values)
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 2),
              child: ListTile(
                leading: Icon(area.icon, size: 20),
                title: Text(area.label),
                selected: selected == area,
                selectedTileColor: const Color(0xFF303030),
                shape: RoundedRectangleBorder(
                  borderRadius: BorderRadius.circular(6),
                ),
                dense: true,
                onTap: () => onSelected(area),
              ),
            ),
        ],
      ),
    );
  }
}

class _AreaPage extends StatelessWidget {
  const _AreaPage({required this.area});

  final _Area area;

  @override
  Widget build(BuildContext context) {
    return SafeArea(
      child: SingleChildScrollView(
        padding: const EdgeInsets.fromLTRB(36, 32, 36, 36),
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 900),
          child: switch (area) {
            _Area.home => const _HomePage(),
            _Area.projects ||
            _Area.templates ||
            _Area.assets => _PlaceholderPage(title: area.label),
            _Area.settings => const _SettingsPage(),
          },
        ),
      ),
    );
  }
}

class _HomePage extends StatelessWidget {
  const _HomePage();

  @override
  Widget build(BuildContext context) {
    return const Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        _PageTitle('Home'),
        SizedBox(height: 28),
        Text(
          'Opencut Reinforced',
          style: TextStyle(fontSize: 24, fontWeight: FontWeight.w600),
        ),
        SizedBox(height: 8),
        Text(
          'Pre-MVP architecture foundation',
          style: TextStyle(color: Color(0xFFAAAAAA), fontSize: 14),
        ),
      ],
    );
  }
}

class _PlaceholderPage extends StatelessWidget {
  const _PlaceholderPage({required this.title});

  final String title;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        _PageTitle(title),
        const SizedBox(height: 28),
        const Text(
          'Not implemented in Phase 3.',
          style: TextStyle(color: Color(0xFFAAAAAA)),
        ),
      ],
    );
  }
}

class _SettingsPage extends StatelessWidget {
  const _SettingsPage();

  @override
  Widget build(BuildContext context) {
    return const Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        _PageTitle('Settings'),
        SizedBox(height: 28),
        Text(
          'About / Developer Diagnostics',
          style: TextStyle(fontSize: 16, fontWeight: FontWeight.w600),
        ),
        SizedBox(height: 8),
        Text(
          'Core diagnostics will be connected in Phase 3.',
          style: TextStyle(color: Color(0xFFAAAAAA)),
        ),
      ],
    );
  }
}

class _PageTitle extends StatelessWidget {
  const _PageTitle(this.title);

  final String title;

  @override
  Widget build(BuildContext context) {
    return Text(
      title,
      style: const TextStyle(fontSize: 22, fontWeight: FontWeight.w600),
    );
  }
}
