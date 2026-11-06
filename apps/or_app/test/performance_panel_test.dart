import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/core_gateway.dart';
import 'package:or_app_bridge/or_app_bridge.dart';
import 'package:or_app/production/editor_settings.dart';
import 'package:or_app/updates/performance_panel.dart';

void main() {
  late _FakeGateway gateway;
  late EditorSettings settings;
  var saves = 0;

  setUp(() {
    gateway = _FakeGateway();
    settings = const EditorSettings();
    saves = 0;
  });

  Future<void> pumpPanel(WidgetTester tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: SingleChildScrollView(
            child: PerformancePanel(
              gateway: gateway,
              settings: settings,
              onSettingsChanged: (next) {
                saves += 1;
                settings = next;
              },
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
  }

  testWidgets('the panel names the machine and the planned encoder', (
    tester,
  ) async {
    await pumpPanel(tester);

    expect(
      find.byKey(const ValueKey('settings-performance')),
      findsOneWidget,
    );
    expect(find.textContaining('Fixture GPU'), findsOneWidget);
    expect(find.textContaining('Metal'), findsOneWidget);
    expect(
      find.byKey(const ValueKey('performance-plan-summary')),
      findsOneWidget,
    );
    expect(find.textContaining('h264_videotoolbox'), findsOneWidget);
    for (final wire in [
      'recommended',
      'maximum_performance',
      'balanced',
      'compatibility',
    ]) {
      expect(
        find.byKey(ValueKey('performance-profile-$wire')),
        findsOneWidget,
      );
    }
  });

  testWidgets('choosing a profile saves it and asks for the new plan', (
    tester,
  ) async {
    await pumpPanel(tester);

    await tester.tap(
      find.byKey(const ValueKey('performance-profile-compatibility')),
    );
    await tester.pumpAndSettle();

    expect(saves, 1);
    expect(settings.performanceProfile, 'compatibility');
    expect(gateway.planCalls, greaterThanOrEqualTo(2));
    expect(gateway.lastProfile, 'compatibility');
    expect(find.textContaining('libx264'), findsWidgets);
  });

  testWidgets('a plan with several notes builds without duplicate keys', (
    tester,
  ) async {
    gateway.planNotes = const [
      'h264_nvenc: the device this encoder needs did not open',
      'no software H.264 encoder opened',
    ];
    await pumpPanel(tester);

    expect(
      find.byKey(const ValueKey('performance-plan-note-0')),
      findsOneWidget,
    );
    expect(
      find.byKey(const ValueKey('performance-plan-note-1')),
      findsOneWidget,
    );
    expect(
      find.byKey(const ValueKey('performance-plan-notes')),
      findsOneWidget,
    );
    expect(tester.takeException(), isNull);
  });

  testWidgets('choosing a graphics backend stores it for the next start', (
    tester,
  ) async {
    await pumpPanel(tester);

    final dropdown = find.byKey(const ValueKey('performance-backend'));
    await tester.ensureVisible(dropdown);
    await tester.pumpAndSettle();
    await tester.tap(dropdown);
    await tester.pumpAndSettle();
    await tester.tap(find.text('Vulkan').last);
    await tester.pumpAndSettle();

    expect(saves, 1);
    expect(settings.graphicsBackend, 'vulkan');
    expect(gateway.graphicsCalls, 1);
    expect(gateway.lastGraphicsBackend, 'vulkan');
  });

  testWidgets('reset clears the encoder override and returns to recommended', (
    tester,
  ) async {
    settings = const EditorSettings(
      performanceProfile: 'balanced',
      preferredH264Encoder: 'h264_videotoolbox',
    );
    await pumpPanel(tester);

    final reset = find.byKey(const ValueKey('performance-reset'));
    await tester.ensureVisible(reset);
    await tester.pumpAndSettle();
    await tester.tap(reset);
    await tester.pumpAndSettle();

    expect(saves, 1);
    expect(settings.performanceProfile, 'recommended');
    expect(settings.preferredH264Encoder, isNull);
    expect(gateway.lastProfile, 'recommended');
    expect(gateway.lastEncoder, isNull);
  });
}

class _FakeGateway implements CoreGateway {
  int planCalls = 0;
  String? lastProfile;
  String? lastEncoder;
  List<String> planNotes = const [];

  @override
  Future<AppInfo> appInfo() async =>
      const AppInfo(name: 'Fixture', version: '0', coreApiVersion: 1);

  @override
  Future<HealthStatus> health() async => const HealthStatus(status: 'ok');

  @override
  Future<List<Capability>> capabilities() async => const [];

  @override
  Future<PerformanceReportView> performanceReport() async =>
      PerformanceReportView(
        cpuArch: 'arm64',
        logicalCores: 10,
        gpuName: 'Fixture GPU',
        gpuBackend: 'Metal',
        gpuDeviceType: 'integrated',
        gpuDriver: 'fixture-driver',
        gpuDriverInfo: '',
        preferredBackend: 'auto',
        gpuCompiledBackends: ['metal', 'vulkan'],
        h264Encoders: [
          EncoderProbeView(
            name: 'h264_videotoolbox',
            present: true,
            opens: true,
            hardware: true,
          ),
          EncoderProbeView(
            name: 'libx264',
            present: true,
            opens: true,
            hardware: false,
          ),
          EncoderProbeView(
            name: 'h264_nvenc',
            present: true,
            opens: false,
            hardware: true,
            note: 'the device this encoder needs did not open',
          ),
        ],
        decoders: [
          DecoderProbeView(name: 'h264', present: true),
          DecoderProbeView(name: 'aac', present: true),
        ],
        plans: [],
      );

  @override
  Future<void> setGraphicsPreference(String backend) async {
    graphicsCalls += 1;
    lastGraphicsBackend = backend;
  }

  int graphicsCalls = 0;
  String? lastGraphicsBackend;

  @override
  Future<EncodingPlanView> encodingPlan({
    required String profile,
    String? preferredH264Encoder,
  }) async {
    planCalls += 1;
    lastProfile = profile;
    lastEncoder = preferredH264Encoder;
    final hardware =
        preferredH264Encoder == 'h264_videotoolbox' ||
        (preferredH264Encoder == null && profile != 'compatibility');
    return EncodingPlanView(
      profile: profile,
      quality: profile == 'maximum_performance' ? 'speed' : 'quality',
      h264Encoder: hardware ? 'h264_videotoolbox' : 'libx264',
      hardware: hardware,
      software: !hardware,
      notes: planNotes,
      summary: hardware
          ? 'h264_videotoolbox (hardware)'
          : 'libx264 (software)',
    );
  }
}
