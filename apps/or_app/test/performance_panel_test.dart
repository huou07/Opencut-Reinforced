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

  /// Mirrors the Settings screen: the owner of the settings rebuilds the panel
  /// with the value it saved, which is what makes an override appear at once.
  Future<void> pumpPanel(WidgetTester tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: StatefulBuilder(
            builder: (context, setHostState) => SingleChildScrollView(
              child: PerformancePanel(
                gateway: gateway,
                settings: settings,
                onSettingsChanged: (next) {
                  saves += 1;
                  setHostState(() => settings = next);
                },
              ),
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
  }

  testWidgets(
    'the panel names the machine, the plan, and no performance tiers',
    (tester) async {
      await pumpPanel(tester);

      expect(
        find.byKey(const ValueKey('settings-performance')),
        findsOneWidget,
      );
      expect(find.textContaining('Fixture GPU'), findsWidgets);
      expect(
        find.byKey(const ValueKey('performance-automatic-copy')),
        findsOneWidget,
      );
      expect(find.textContaining('h264_videotoolbox'), findsWidgets);
      for (final retired in [
        'recommended',
        'maximum_performance',
        'balanced',
        'compatibility',
      ]) {
        expect(
          find.byKey(ValueKey('performance-profile-$retired')),
          findsNothing,
          reason: 'the retired tier $retired must not be offered',
        );
      }
      expect(find.text('Maximum performance'), findsNothing);
      expect(find.text('Compatibility'), findsNothing);
    },
  );

  testWidgets('each part of the job says who does it and why', (tester) async {
    await pumpPanel(tester);

    for (final part in ['render', 'composite', 'decode', 'encode']) {
      expect(
        find.byKey(ValueKey('performance-workload-$part')),
        findsOneWidget,
      );
    }
    expect(find.textContaining('software (h264, aac)'), findsOneWidget);
    expect(
      find.textContaining('costs more than it saves'),
      findsOneWidget,
      reason: 'the decode row must say why hardware is not used',
    );
  });

  testWidgets('the first look records what the plan was decided from', (
    tester,
  ) async {
    await pumpPanel(tester);

    expect(saves, 1);
    expect(settings.hardwareFingerprint, isNotNull);
    expect(settings.hardwareFingerprint, contains('Fixture GPU'));
    expect(settings.hardwareCheckedAt, isNotNull);
    expect(
      find.byKey(const ValueKey('performance-machine-changed')),
      findsNothing,
      reason: 'a first look is not a change',
    );
    expect(find.byKey(const ValueKey('performance-checked')), findsOneWidget);
  });

  testWidgets(
    'a machine that no longer matches is reported and decided again',
    (tester) async {
      settings = const EditorSettings(
        hardwareFingerprint: 'a-different-machine',
      );
      await pumpPanel(tester);

      expect(
        find.byKey(const ValueKey('performance-machine-changed')),
        findsOneWidget,
      );
      expect(settings.hardwareFingerprint, contains('Fixture GPU'));
    },
  );

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

  testWidgets('an encoder the person pins wins and Reset clears it', (
    tester,
  ) async {
    await pumpPanel(tester);

    final dropdown = find.byKey(const ValueKey('performance-encoder'));
    await tester.ensureVisible(dropdown);
    await tester.pumpAndSettle();
    await tester.tap(dropdown);
    await tester.pumpAndSettle();
    await tester.tap(find.text('libx264').last);
    await tester.pumpAndSettle();

    expect(saves, greaterThanOrEqualTo(1));
    expect(settings.preferredH264Encoder, 'libx264');
    expect(gateway.lastEncoder, 'libx264');
    expect(find.textContaining('libx264'), findsWidgets);

    final reset = find.byKey(const ValueKey('performance-reset'));
    await tester.ensureVisible(reset);
    await tester.pumpAndSettle();
    await tester.tap(reset);
    await tester.pumpAndSettle();

    expect(settings.preferredH264Encoder, isNull);
    expect(settings.graphicsBackend, 'auto');
    expect(settings.performanceProfile, 'automatic');
  });

  testWidgets('Reset is unavailable until something is overridden', (
    tester,
  ) async {
    await pumpPanel(tester);

    final reset = tester.widget<OutlinedButton>(
      find.byKey(const ValueKey('performance-reset')),
    );
    expect(reset.onPressed, isNull);
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
    await tester.tap(find.text('Vulkan only').last);
    await tester.pumpAndSettle();

    expect(settings.graphicsBackend, 'vulkan');
    expect(gateway.graphicsCalls, 1);
    expect(gateway.lastGraphicsBackend, 'vulkan');
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
        preferredH264Encoder == null ||
        preferredH264Encoder == 'h264_videotoolbox';
    return EncodingPlanView(
      profile: profile,
      quality: 'quality',
      h264Encoder: hardware ? 'h264_videotoolbox' : 'libx264',
      hardware: hardware,
      software: !hardware,
      notes: planNotes,
      summary: hardware ? 'h264_videotoolbox (hardware)' : 'libx264 (software)',
    );
  }
}
