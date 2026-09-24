import 'package:flutter/material.dart';
import 'package:or_app/core_gateway.dart';
import 'package:or_app/rust_core_gateway.dart';
import 'package:or_app/src/rust/frb_generated.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await RustLib.init();
  runApp(const OrApp(gateway: RustCoreGateway()));
}

class OrApp extends StatelessWidget {
  const OrApp({super.key, required this.gateway});

  final CoreGateway gateway;

  @override
  Widget build(BuildContext context) {
    const background = Color(0xFF101010);
    const surface = Color(0xFF181818);
    const border = Color(0xFF303030);

    return MaterialApp(
      title: 'Opencut Reinforced',
      debugShowCheckedModeBanner: false,
      theme: ThemeData(
        useMaterial3: true,
        brightness: Brightness.dark,
        scaffoldBackgroundColor: background,
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
      home: _Workspace(gateway: gateway),
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
  const _Workspace({required this.gateway});

  final CoreGateway gateway;

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
              Expanded(
                child: _AreaPage(area: _selected, gateway: widget.gateway),
              ),
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
  const _AreaPage({required this.area, required this.gateway});

  final _Area area;
  final CoreGateway gateway;

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
            _Area.settings => _SettingsPage(gateway: gateway),
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

typedef _Diagnostics = ({
  AppInfo appInfo,
  HealthStatus health,
  List<Capability> capabilities,
});

class _SettingsPage extends StatefulWidget {
  const _SettingsPage({required this.gateway});

  final CoreGateway gateway;

  @override
  State<_SettingsPage> createState() => _SettingsPageState();
}

class _SettingsPageState extends State<_SettingsPage> {
  late final Future<_Diagnostics> _diagnostics = _loadDiagnostics();

  Future<_Diagnostics> _loadDiagnostics() async => (
    appInfo: await widget.gateway.appInfo(),
    health: await widget.gateway.health(),
    capabilities: await widget.gateway.capabilities(),
  );

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        const _PageTitle('Settings'),
        const SizedBox(height: 28),
        const Text(
          'About / Developer Diagnostics',
          style: TextStyle(fontSize: 16, fontWeight: FontWeight.w600),
        ),
        const SizedBox(height: 16),
        FutureBuilder<_Diagnostics>(
          future: _diagnostics,
          builder: (context, snapshot) {
            if (snapshot.connectionState != ConnectionState.done) {
              return const Text('Loading Rust core diagnostics...');
            }
            if (snapshot.hasError) {
              return const Text('Could not load Rust core diagnostics.');
            }

            final diagnostics = snapshot.requireData;
            return _DiagnosticsView(diagnostics: diagnostics);
          },
        ),
      ],
    );
  }
}

class _DiagnosticsView extends StatelessWidget {
  const _DiagnosticsView({required this.diagnostics});

  final _Diagnostics diagnostics;

  @override
  Widget build(BuildContext context) {
    return Container(
      width: double.infinity,
      padding: const EdgeInsets.all(20),
      decoration: BoxDecoration(
        color: Theme.of(context).colorScheme.surface,
        border: Border.all(color: Theme.of(context).dividerColor),
        borderRadius: BorderRadius.circular(8),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          _DiagnosticRow('Application name', diagnostics.appInfo.name),
          _DiagnosticRow('Version', diagnostics.appInfo.version),
          _DiagnosticRow(
            'Core API version',
            '${diagnostics.appInfo.coreApiVersion}',
          ),
          _DiagnosticRow('Health', diagnostics.health.status),
          const SizedBox(height: 16),
          const Text(
            'Capabilities',
            style: TextStyle(fontWeight: FontWeight.w600),
          ),
          const SizedBox(height: 8),
          for (final capability in diagnostics.capabilities)
            Padding(
              padding: const EdgeInsets.only(bottom: 6),
              child: Text('${capability.id} · v${capability.version}'),
            ),
        ],
      ),
    );
  }
}

class _DiagnosticRow extends StatelessWidget {
  const _DiagnosticRow(this.label, this.value);

  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 12),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          SizedBox(
            width: 144,
            child: Text(
              label,
              style: const TextStyle(color: Color(0xFFAAAAAA)),
            ),
          ),
          Expanded(child: Text(value)),
        ],
      ),
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
