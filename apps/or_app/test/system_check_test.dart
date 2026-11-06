import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/support/host_platform.dart';
import 'package:or_app/updates/system_check.dart';
import 'package:or_app/updates/system_check_panel.dart';

void main() {
  group('host facts', () {
    test('bytes are shown in the unit a person compares', () {
      expect(formatBytes(0), '0 B');
      expect(formatBytes(512), '512 B');
      expect(formatBytes(1024), '1.0 KB');
      expect(formatBytes(1536), '1.5 KB');
      expect(formatBytes(16 * 1024 * 1024 * 1024), '16 GB');
      expect(formatBytes(1500 * 1024 * 1024 * 1024), '1.5 TB');
    });

    test('the summary names each measured value once', () {
      const facts = HostFacts(
        platformName: 'macOS',
        osVersion: 'Version 15.0',
        cpuCount: 10,
        memoryBytes: 16 * 1024 * 1024 * 1024,
        freeDiskBytes: 120 * 1024 * 1024 * 1024,
      );

      expect(
        facts.summary,
        'macOS Version 15.0 · 10 cores · 16 GB RAM · 120 GB free',
      );
      expect(facts.hasExportHeadroom, isTrue);
    });

    test('a missing measurement is unknown, not zero', () {
      const facts = HostFacts(
        platformName: 'Linux',
        osVersion: 'Linux 6.1',
        cpuCount: 1,
      );

      expect(facts.summary, 'Linux Linux 6.1 · 1 core');
      expect(facts.hasExportHeadroom, isTrue);
    });

    test('less than the export headroom is a warning', () {
      const facts = HostFacts(
        platformName: 'Windows',
        osVersion: 'Windows 11',
        cpuCount: 8,
        freeDiskBytes: 1024 * 1024 * 1024,
      );

      expect(facts.hasExportHeadroom, isFalse);
    });

    test(
      'a POSIX host reads free space from df and memory from the kernel',
      () async {
        final calls = <String>[];
        Future<String?> run(String executable, List<String> arguments) async {
          calls.add('$executable ${arguments.join(' ')}');
          if (executable == 'df') {
            return 'Filesystem 1024-blocks Used Available Capacity Mounted\n'
                '/dev/disk3s1 1000000 500000 400000 56% /\n';
          }
          return null;
        }

        final facts = await readHostFacts(
          volume: Directory.systemTemp,
          platform: const HostPlatform(
            isWindows: false,
            isMacOS: false,
            isLinux: true,
            environment: {'HOME': '/home/test'},
            executablePath: '/app/or_app',
          ),
          run: run,
        );

        expect(facts.freeDiskBytes, 400000 * 1024);
        expect(facts.cpuCount, greaterThan(0));
        expect(calls.any((call) => call.startsWith('df -k ')), isTrue);
      },
    );

    test('a Windows host asks PowerShell for the free bytes', () async {
      Future<String?> run(String executable, List<String> arguments) async {
        if (executable == 'powershell') {
          return arguments.last.contains('Free') ? '123456789\n' : null;
        }
        return null;
      }

      final facts = await readHostFacts(
        volume: Directory(r'C:\Users\test\AppData'),
        platform: const HostPlatform(
          isWindows: true,
          isMacOS: false,
          isLinux: false,
          environment: {'USERPROFILE': r'C:\Users\test'},
          executablePath: r'C:\Program Files\or_app\or_app.exe',
        ),
        run: run,
      );

      expect(facts.freeDiskBytes, 123456789);
    });

    test('a refusing host reports unknown instead of throwing', () async {
      final facts = await readHostFacts(
        volume: Directory.systemTemp,
        platform: const HostPlatform(
          isWindows: false,
          isMacOS: true,
          isLinux: false,
          environment: {'HOME': '/Users/test'},
          executablePath: '/Applications/or_app.app/or_app',
        ),
        run: (_, _) async => null,
      );

      expect(facts.memoryBytes, isNull);
      expect(facts.freeDiskBytes, isNull);
    });
  });

  group('system check panel', () {
    Future<void> pump(WidgetTester tester, HostFacts facts) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: SingleChildScrollView(
              child: SystemCheckPanel(facts: () async => facts),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
    }

    testWidgets('a roomy machine shows each measured row and no warning', (
      tester,
    ) async {
      await pump(
        tester,
        const HostFacts(
          platformName: 'macOS',
          osVersion: 'Version 15.0',
          cpuCount: 10,
          memoryBytes: 16 * 1024 * 1024 * 1024,
          freeDiskBytes: 120 * 1024 * 1024 * 1024,
        ),
      );

      expect(
        find.byKey(const ValueKey('settings-system-check')),
        findsOneWidget,
      );
      expect(
        find.byKey(const ValueKey('system-check-summary')),
        findsOneWidget,
      );
      expect(find.text('10'), findsOneWidget);
      expect(find.text('16 GB'), findsOneWidget);
      expect(
        find.byKey(const ValueKey('system-check-disk-warning')),
        findsNothing,
      );
    });

    testWidgets('a nearly full disk says what will happen and how to fix it', (
      tester,
    ) async {
      await pump(
        tester,
        const HostFacts(
          platformName: 'Linux',
          osVersion: 'Linux 6.1',
          cpuCount: 4,
          freeDiskBytes: 512 * 1024 * 1024,
        ),
      );

      final warning = find.byKey(const ValueKey('system-check-disk-warning'));
      expect(warning, findsOneWidget);
      expect(
        tester.widget<Text>(warning).data,
        contains('Free some space, or export a shorter range.'),
      );
    });

    testWidgets('a machine that cannot be read says so instead of spinning', (
      tester,
    ) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: SystemCheckPanel(
              facts: () async => throw StateError('no machine data'),
            ),
          ),
        ),
      );
      await tester.pump();

      expect(find.byKey(const ValueKey('system-check-error')), findsOneWidget);
      expect(find.byKey(const ValueKey('system-check-summary')), findsNothing);
      expect(find.byType(CircularProgressIndicator), findsNothing);
    });
  });
}
