// The OR agent-driven UI scenario runner.
//
// One entrypoint for `scripts/or_ui.py`. It reads a JSON scenario, drives the
// real app through `WidgetTester` on whichever target is running (Linux under
// Xvfb, or the Android emulator), and writes a step-by-step transcript plus
// screenshots and semantics snapshots. It never injects canonical project state
// into the app: every mutation goes through a real UI action, and the only host
// integration it replaces is the OS file picker, which a test cannot drive.
//
// Selectors are semantic, never positional: a `key`, a semantics `label`
// (exact or regex), visible `text`, or a `role` filter on a semantics label.
// The one exception is a role-only selector, which taps the centre of the
// semantics node it named — the coordinate is derived from the semantics tree at
// observation time, never authored in a scenario.
//
// Scenario file: `OR_UI_SCENARIO_FILE` from the environment (Linux) or the
// app-private `files/or-ui-scenario.json` (Android, pushed by the host).
// Artifacts: `OR_UI_ARTIFACT_DIR` (Linux) or the app-private
// `files/or-ui-artifacts` (Android), plus an `index.json` the host uses to pull
// them off a guest.
import 'dart:convert';
import 'dart:io';
import 'dart:math' as math;
import 'dart:typed_data';
import 'dart:ui' as ui;

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/scheduler.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:or_app/main.dart';
import 'package:or_app/project/export_profile.dart';
import 'package:or_app/project/project_file_picker.dart';
import 'package:or_app/project/recent_projects.dart';
import 'package:or_app/project/rust_project_gateway.dart';
import 'package:or_app/rust_core_gateway.dart';
import 'package:or_app_bridge/or_app_bridge.dart' show RustLib;

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  setUpAll(RustLib.init);

  testWidgets('OR UI scenario', (tester) async {
    final spec = _Spec.load();
    final artifacts = _Artifacts(spec);
    final runner = _Runner(tester, spec, artifacts);
    try {
      await runner.run();
    } finally {
      await runner.dispose();
    }
    if (!runner.ok) {
      fail('OR UI scenario "${spec.name}" failed: ${runner.firstFailure}');
    }
  });
}

/// Where the scenario, its fixtures and its evidence live on this target.
class _Locations {
  static bool get onAndroid => Platform.isAndroid;

  /// The app-private files directory. On Android `systemTemp` is the app cache
  /// directory, so its parent is the app's data directory and `files/` survives
  /// an APK reinstall; elsewhere this is never used for evidence.
  static String get appFiles => '${Directory.systemTemp.parent.path}/files';

  static String _firstNonEmpty(List<String> candidates, String fallback) {
    for (final candidate in candidates) {
      if (candidate.isNotEmpty) return candidate;
    }
    return fallback;
  }

  static String scenarioPath() => _firstNonEmpty(
    [
      Platform.environment['OR_UI_SCENARIO_FILE'] ?? '',
      const String.fromEnvironment('OR_UI_SCENARIO_FILE'),
    ],
    onAndroid
        ? '$appFiles/or-ui-scenario.json'
        : '${Directory.systemTemp.path}/or-ui-scenario.json',
  );

  static String artifactPath() => _firstNonEmpty(
    [
      Platform.environment['OR_UI_ARTIFACT_DIR'] ?? '',
      const String.fromEnvironment('OR_UI_ARTIFACT_DIR'),
    ],
    onAndroid
        ? '$appFiles/or-ui-artifacts'
        : '${Directory.systemTemp.path}/or-ui-artifacts',
  );
}

class _Spec {
  _Spec(this.name, this.viewport, this.steps, this.raw, this.pickList);

  final String name;
  final ({double width, double height})? viewport;
  final List<Map<String, Object?>> steps;
  final Map<String, Object?> raw;
  final List<String> pickList;

  static _Spec load() {
    final path = _Locations.scenarioPath();
    final file = File(path);
    if (!file.existsSync()) {
      throw StateError('the scenario file $path does not exist');
    }
    final decoded = jsonDecode(file.readAsStringSync());
    if (decoded is! Map<String, Object?>) {
      throw StateError(
        'a scenario must be a JSON object, not ${decoded.runtimeType}',
      );
    }
    final viewport = decoded['viewport'];
    ({double width, double height})? size;
    if (viewport is Map) {
      size = (
        width: (viewport['width'] as num).toDouble(),
        height: (viewport['height'] as num).toDouble(),
      );
    }
    final steps = <Map<String, Object?>>[];
    for (final step in (decoded['steps'] as List? ?? const [])) {
      if (step is Map<String, Object?>) steps.add(step);
    }
    final picks = <String>[];
    for (final pick in (decoded['fixtures'] as List? ?? const [])) {
      picks.add('$pick');
    }
    return _Spec(
      '${decoded['name'] ?? 'scenario'}',
      size,
      steps,
      decoded,
      picks,
    );
  }
}

/// Owns the artifact directory, the index the host reads, and every write.
class _Artifacts {
  _Artifacts(this.spec) {
    directory = Directory(_Locations.artifactPath());
    if (directory.existsSync()) directory.deleteSync(recursive: true);
    directory.createSync(recursive: true);
    replacements = Directory('${directory.path}/replacements')..createSync();
  }

  final _Spec spec;
  late final Directory directory;
  late final Directory replacements;
  final List<Map<String, Object?>> index = [];

  String relative(String name) => name;

  void writeJson(String name, Object? payload) {
    File('${directory.path}/$name')
        .writeAsStringSync(const JsonEncoder.withIndent('  ').convert(payload));
    index.add({
      'name': name,
      'bytes': File('${directory.path}/$name').lengthSync(),
    });
  }

  void writeText(String name, String text) {
    File('${directory.path}/$name').writeAsStringSync(text);
    index.add({
      'name': name,
      'bytes': File('${directory.path}/$name').lengthSync(),
    });
  }

  void writeBytes(String name, List<int> bytes) {
    File('${directory.path}/$name').writeAsBytesSync(bytes);
    index.add({'name': name, 'bytes': bytes.length});
  }

  void close(String platform, String transcriptName) {
    writeJson('index.json', {
      'scenario': spec.name,
      'platform': platform,
      'transcript': transcriptName,
      'files': index,
    });
  }
}

typedef _Selector = Map<String, Object?>;

class _StepResult {
  _StepResult(this.index, this.op, this.selector);

  final int index;
  final String op;
  final _Selector? selector;
  String status = 'ok';
  String? detail;
  double elapsedMs = 0;
  String? screenshot;
  String? semantics;
  Map<String, Object?>? measurement;

  Map<String, Object?> toJson() => {
    'index': index,
    'op': op,
    if (selector != null) 'selector': selector,
    'status': status,
    'elapsed_ms': double.parse(elapsedMs.toStringAsFixed(2)),
    if (detail != null) 'detail': detail,
    if (screenshot != null) 'screenshot': screenshot,
    if (semantics != null) 'semantics': semantics,
    if (measurement != null) 'measurement': measurement,
  };
}

class _Runner {
  _Runner(this.tester, this.spec, this.artifacts);

  final WidgetTester tester;
  final _Spec spec;
  final _Artifacts artifacts;

  final GlobalKey boundary = GlobalKey();
  final List<_StepResult> results = [];
  late final _FixturePicker picker;
  List<String> screenInventory = const [];
  bool ok = true;
  String? firstFailure;
  SemanticsHandle? _semantics;
  final List<String> frameworkErrors = [];
  void Function(FlutterErrorDetails)? _previousOnError;

  String get platform => Platform.operatingSystem;

  Future<void> run() async {
    final viewport = spec.viewport ?? (width: 1280, height: 800);
    tester.view.devicePixelRatio = 1.0;
    tester.view.physicalSize = Size(viewport.width, viewport.height);
    addTearDown(tester.view.reset);

    picker = _FixturePicker(
      Directory(artifacts.replacements.path),
      spec.pickList,
    );
    await tester.pumpWidget(
      RepaintBoundary(
        key: boundary,
        child: OrApp(
          gateway: const RustCoreGateway(),
          projectGateway: const RustProjectGateway(),
          projectFilePicker: picker,
          recentProjectStore: InMemoryRecentProjectStore(),
        ),
      ),
    );
    // Record every framework error with the widget that caused it. The
    // binding's own record is a bare summary ("A RenderFlex overflowed by 17
    // pixels on the bottom."), which does not say where; the details do, and
    // losing them costs another run to find the surface.
    _previousOnError = FlutterError.onError;
    FlutterError.onError = (details) {
      final text = details.toString().replaceAll(RegExp(r'\s+'), ' ').trim();
      // ignore: avoid_print
      print('OR_FRAMEWORK_ERROR $text');
      frameworkErrors.add(
        text.length > 400 ? '${text.substring(0, 400)}…' : text,
      );
      _previousOnError?.call(details);
    };
    await _settle();
    _semantics = tester.ensureSemantics();
    if (spec.raw['screens'] == true) {
      await _captureScreenInventory();
    }
    for (final step in spec.steps) {
      if (!ok && spec.raw['continue_on_failure'] != true) break;
      await _executeStep(step);
    }
  }

  Future<void> dispose() async {
    if (_previousOnError != null) {
      FlutterError.onError = _previousOnError;
      _previousOnError = null;
    }
    _semantics?.dispose();
    final transcript = {
      'scenario': spec.name,
      'platform': platform,
      'viewport': spec.viewport == null
          ? null
          : {'width': spec.viewport!.width, 'height': spec.viewport!.height},
      'result': ok ? 'PASS' : 'FAIL',
      'steps': results.map((result) => result.toJson()).toList(),
    };
    final name = '${spec.name}.json';
    artifacts.writeJson(name, transcript);
    artifacts.close(platform, name);
    try {
      await tester.pumpWidget(const SizedBox.shrink());
    } catch (_) {
      // A disposal failure is not worth masking the scenario result.
    }
  }

  // -- execution ---------------------------------------------------------
  Future<void> _executeStep(Map<String, Object?> step) async {
    final op = '${step['op']}';
    final selector = _selector(step);
    final result = _StepResult(results.length, op, selector);
    results.add(result);
    final started = DateTime.now();
    try {
      await _dispatch(op, step, result);
      result.status = 'ok';
    } on _StepFailure catch (failure) {
      result.status = failure.status;
      result.detail = failure.message;
      ok = false;
      firstFailure ??= 'step ${result.index} ($op): ${failure.message}';
    } catch (error) {
      result.status = 'error';
      result.detail = '$error';
      ok = false;
      firstFailure ??= 'step ${result.index} ($op): $error';
    } finally {
      result.elapsedMs =
          DateTime.now().difference(started).inMicroseconds / 1000.0;
      // A RenderFlex overflow, a failed assertion or a thrown build error is
      // reported through the binding, not through the step's return value. The
      // harness must not let one pass silently: a clipped or overflowing layout
      // is exactly what the UX thresholds forbid.
      final exception = tester.takeException();
      if (exception != null && result.status == 'ok') {
        result.status = 'exception';
        // Keep more than the first line: the framework's overflow message names
        // the error-causing widget after the summary, and a transcript that drops
        // it costs another run to find.
        // A FlutterError carries its location in `diagnostics`; `toString()`
        // is only the summary.
        final raw = exception is FlutterError
            ? exception.diagnostics.map((node) => node.toString()).join(' | ')
            : '$exception';
        final text = raw.replaceAll(RegExp(r'\s+'), ' ').trim();
        result.detail = text.length > 400 ? '${text.substring(0, 400)}…' : text;
        ok = false;
        firstFailure ??= 'step ${result.index} ($op) raised: ${result.detail}';
      }
    }
  }

  Future<void> _dispatch(
    String op,
    Map<String, Object?> step,
    _StepResult result,
  ) async {
    switch (op) {
      case 'resize':
        // Breakpoint review needs a real layout change at a stated size. The
        // view size is the same thing the window would do, without depending
        // on the window manager.
        final width = (step['width'] as num?)?.toDouble();
        final height = (step['height'] as num?)?.toDouble();
        if (width == null || height == null) {
          throw FormatException('resize needs numeric width and height');
        }
        tester.view.devicePixelRatio = 1.0;
        tester.view.physicalSize = Size(width, height);
        await tester.pumpAndSettle();
        result.detail = 'resized to ${width.toInt()}x${height.toInt()}';
        return;
      case 'pump':
        await _pumpFor(_milliseconds(step['ms'] ?? 100));
        return;
      case 'wait':
        await _waitForText(
          '${step['text']}',
          _milliseconds(step['budget_ms'] ?? 8000),
        );
        return;
      case 'wait_absent':
        await _waitForText(
          '${step['text']}',
          _milliseconds(step['budget_ms'] ?? 8000),
          absent: true,
        );
        return;
      case 'wait_label':
        await _waitForLabel(
          '${step['label_regex'] ?? step['label']}',
          _milliseconds(step['budget_ms'] ?? 8000),
        );
        return;
      case 'wait_label_absent':
        await _waitForLabel(
          '${step['label_regex'] ?? step['label']}',
          _milliseconds(step['budget_ms'] ?? 8000),
          absent: true,
        );
        return;
      case 'shot':
        result.screenshot = await _shot(
          '${step['name'] ?? 'shot'}-${result.index}',
        );
        return;
      case 'tree':
        result.semantics = await _tree(
          '${step['name'] ?? 'tree'}-${result.index}',
        );
        return;
      case 'screens':
        await _captureScreenInventory();
        result.semantics = 'screen-inventory.json';
        return;
      case 'tap':
        await _act(step, (finder) async {
          await _scrollIntoView(finder, settle: step['settle'] != false);
          await tester.tap(finder);
        });
        // `settle: false` keeps a measured interaction honest: a fixed settle
        // budget inside the timed block measures the harness, not the app. The
        // following wait_* step observes the state instead, so the measured time
        // is the latency to the visible result (plus at most one 16 ms poll
        // interval).
        if (step['settle'] != false) await _settle();
        return;
      case 'secondary_tap':
        // A right click. Clip dialogs (trim, slip, roll, move) live behind the clip
        // context menu, and a pointer owner reaches them that way; without this op
        // the harness can only assert those dialogs through the widget suite.
        await _act(step, (finder) async {
          await _scrollIntoView(finder, settle: step['settle'] != false);
          await tester.tap(finder, buttons: kSecondaryMouseButton);
        });
        if (step['settle'] != false) await _settle();
        return;
      case 'shift_tap':
        // Shift-click extends a selection. The app reads the hardware keyboard
        // state during the tap, so the modifier is held across the gesture.
        await _act(step, (finder) async {
          await _scrollIntoView(finder, settle: step['settle'] != false);
          await tester.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft);
          try {
            await tester.tap(finder);
          } finally {
            await tester.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft);
          }
        });
        if (step['settle'] != false) await _settle();
        return;
      case 'long_press':
        await _act(step, (finder) async {
          await _scrollIntoView(finder, settle: step['settle'] != false);
          await tester.longPress(finder);
        });
        if (step['settle'] != false) await _settle();
        return;
      case 'drag':
        // A pointer drag over a widget: `dx`/`dy` are logical pixels from the
        // widget centre, and `steps` splits the move into samples so a widget
        // that follows the pointer is exercised the way a hand moves it.
        final dx = _num(step['dx']);
        final dy = _num(step['dy']);
        // Where inside the widget the pointer lands. A rubber band has to start
        // where the clips are not, so the default centre is not always right.
        final fromDx = _num(step['from_dx']);
        final fromDy = _num(step['from_dy']);
        final steps = (step['steps'] as num?)?.toInt() ?? 8;
        await _act(step, (finder) async {
          final origin = step['from'] == 'topLeft'
              ? tester.getTopLeft(finder)
              : tester.getCenter(finder);
          final centre = origin + Offset(fromDx, fromDy);
          final gesture = await tester.startGesture(centre);
          // Samples without an intervening frame: a pump between moves lets a
          // host rebuild replace the recognizer mid-gesture, which is a harness
          // artifact rather than something the app does to a real pointer.
          for (var i = 0; i < steps; i++) {
            await gesture.moveBy(Offset(dx / steps, dy / steps));
          }
          await gesture.up();
        });
        if (step['settle'] != false) await _settle();
        return;
      case 'focus':
        // Focus an editable without tapping it: a tap also moves the caret to
        // the touched offset, which would make a Backspace assertion depend on
        // geometry instead of the shortcut layer.
        await tester.showKeyboard(_requireFinder(step));
        await _settle();
        return;
      case 'enter_text':
        final finder = _requireFinder(step);
        await _scrollIntoView(finder);
        await tester.enterText(finder, _expandRoot('${step['value'] ?? ''}'));
        await _settle();
        return;
      case 'key':
        await _pressChord('${step['chord']}');
        await _settle();
        return;
      case 'expect':
        final finder = _requireFinder(step);
        if (finder.evaluate().isEmpty) {
          throw _StepFailure(
            'not_found',
            'no widget matched ${_selector(step)}',
          );
        }
        final role = step['role'];
        if (role != null) {
          final node = tester.getSemantics(finder.first);
          final actual = _roleOf(node);
          if (actual != '$role') {
            throw _StepFailure(
              'wrong_role',
              'expected role $role but the node reports $actual',
            );
          }
          result.detail = 'role=$actual label=${node.label}';
        }
        return;
      case 'expect_text':
        if (find.text('${step['text']}').evaluate().isEmpty) {
          throw _StepFailure('not_found', 'no visible text "${step['text']}"');
        }
        return;
      case 'expect_value':
        // Read the live controller of a text field. Typing behaviour (IME,
        // Backspace, copy/paste) cannot be asserted through a rendered label,
        // because a text field's value lives in its editor state.
        final finder = _requireFinder(step);
        final editable = find.descendant(
          of: finder.first,
          matching: find.byType(EditableText),
        );
        if (editable.evaluate().isEmpty) {
          throw _StepFailure(
            'not_found',
            'no text field matched ${_selector(step)}',
          );
        }
        final actual = tester
            .widget<EditableText>(editable.first)
            .controller
            .text;
        final expected = '${step['value']}';
        if (actual != expected) {
          throw _StepFailure(
            'wrong_value',
            'expected "$expected" but the field holds "$actual"',
          );
        }
        result.detail = 'value="$actual"';
        return;
      case 'expect_absent':
        if (_anyFinder(step).evaluate().isNotEmpty) {
          throw _StepFailure(
            'still_present',
            'a matching widget is still on screen',
          );
        }
        return;
      case 'expect_state':
        // Assert a semantics state on a named control, e.g. a toggle's checked
        // or an enabled flag. The tree is the authority, not a widget field.
        final data = tester
            .getSemantics(_requireFinder(step).first)
            .getSemanticsData();
        final expected = '${step['state']}';
        if (!_matchesState(data, expected)) {
          throw _StepFailure(
            'wrong_state',
            'expected state $expected; observed ${_describe(data)}',
          );
        }
        result.detail = _describe(data);
        return;
      case 'wait_state':
        // Readiness by semantics rather than by sleeping: a control that is
        // disabled while the previous command settles must not be tapped, and a
        // fixed delay is how a scenario becomes flaky. Measured 2026-10-18: the
        // compact journey tapped the timeline's Add Audio while the import was
        // still settling, the disabled control swallowed the tap, and the later
        // insert reported "Add a compatible Video or Audio track first."
        await _waitForState(step);
        return;
      case 'picker':
        picker.next = '${step['next']}';
        result.detail = 'next pick: ${picker.next}';
        return;
      case 'scroll':
        final finder = _requireFinder(step);
        await tester.drag(
          finder,
          Offset(0, (step['dy'] as num?)?.toDouble() ?? -200),
        );
        await _settle();
        return;
      case 'measure':
        result.measurement = await _measure(step);
        return;
      case 'frames':
        result.measurement = await _measureFrames(step);
        return;
      default:
        throw _StepFailure('bad_scenario', 'unknown op "$op"');
    }
  }

  Future<void> _act(
    Map<String, Object?> step,
    Future<void> Function(Finder finder) action,
  ) async {
    final selector = _selector(step);
    // The same narrowing the other steps use: an `index` picks the nth match
    // instead of making the step ambiguous.
    final finder = _requireFinder(step);
    if (finder.evaluate().isEmpty) {
      throw _StepFailure('not_found', 'no widget matched $selector');
    }
    final role = step['role'];
    if (role != null) {
      final node = tester.getSemantics(finder.first);
      final actual = _roleOf(node);
      if (actual != '$role') {
        throw _StepFailure(
          'wrong_role',
          'expected role $role but the node reports $actual',
        );
      }
    }
    final count = finder.evaluate().length;
    if (count > 1 && step['allow_multiple'] != true) {
      throw _StepFailure(
        'ambiguous',
        '$count widgets matched $selector; make the selector narrower',
      );
    }
    await action(finder.first);
  }

  Future<void> _pressChord(String chord) async {
    final parts = chord.toLowerCase().split('+');
    final modifiers = <LogicalKeyboardKey>[];
    for (final part in parts.take(parts.length - 1)) {
      modifiers.add(switch (part) {
        'ctrl' || 'control' => LogicalKeyboardKey.controlLeft,
        'shift' => LogicalKeyboardKey.shiftLeft,
        'alt' => LogicalKeyboardKey.altLeft,
        'meta' || 'cmd' || 'super' => LogicalKeyboardKey.metaLeft,
        _ => throw _StepFailure('bad_scenario', 'unknown modifier "$part"'),
      });
    }
    final key = _logicalKey(parts.last);
    for (final modifier in modifiers) {
      await tester.sendKeyDownEvent(modifier);
    }
    await tester.sendKeyEvent(key);
    for (final modifier in modifiers.reversed) {
      await tester.sendKeyUpEvent(modifier);
    }
  }

  /// Step numbers arrive as JSON numbers and must stay finite.
  double _num(Object? value, {double fallback = 0}) {
    if (value is num && value.isFinite) return value.toDouble();
    return double.tryParse('${value ?? ''}') ?? fallback;
  }

  LogicalKeyboardKey _logicalKey(String name) {
    const named = {
      'escape': LogicalKeyboardKey.escape,
      'enter': LogicalKeyboardKey.enter,
      'tab': LogicalKeyboardKey.tab,
      'space': LogicalKeyboardKey.space,
      'delete': LogicalKeyboardKey.delete,
      'backspace': LogicalKeyboardKey.backspace,
      'arrowleft': LogicalKeyboardKey.arrowLeft,
      'arrowright': LogicalKeyboardKey.arrowRight,
      'arrowup': LogicalKeyboardKey.arrowUp,
      'arrowdown': LogicalKeyboardKey.arrowDown,
      'home': LogicalKeyboardKey.home,
      'end': LogicalKeyboardKey.end,
      'period': LogicalKeyboardKey.period,
      'comma': LogicalKeyboardKey.comma,
      'equal': LogicalKeyboardKey.equal,
      'minus': LogicalKeyboardKey.minus,
      'f5': LogicalKeyboardKey.f5,
      'plus': LogicalKeyboardKey.numpadAdd,
    };
    final direct = named[name];
    if (direct != null) return direct;
    final letter = RegExp(r'^[a-z]$').firstMatch(name);
    if (letter != null) {
      return LogicalKeyboardKey(
        LogicalKeyboardKey.keyA.keyId + (name.codeUnitAt(0) - 97),
      );
    }
    final digit = RegExp(r'^[0-9]$').firstMatch(name);
    if (digit != null) {
      return LogicalKeyboardKey(
        LogicalKeyboardKey.digit0.keyId + int.parse(name),
      );
    }
    throw _StepFailure('bad_scenario', 'unknown key "$name"');
  }

  /// A measured interaction: run the sub-steps [repeat] times and report the
  /// distribution. Wall-clock on this host only; software-rendered Linux and a
  /// SwiftShader guest never become device performance claims.
  /// Frame pacing while the preview plays: every presented frame's build and
  /// raster cost, and the interval between presentations.
  ///
  /// This reads the engine's own timings rather than a widget's clock, so a
  /// stutter shows up as a long interval instead of being averaged away.
  Future<Map<String, Object?>> _measureFrames(Map<String, Object?> step) async {
    final durationMs = (step['duration_ms'] as num?)?.toInt() ?? 3000;
    final submitted = <FrameTiming>[];
    void collect(List<FrameTiming> timings) => submitted.addAll(timings);
    SchedulerBinding.instance.addTimingsCallback(collect);
    try {
      // The harness drives frames itself: nothing pumps unless the test does,
      // so a bare delay would record the one frame this loop started on.
      final deadline = DateTime.now().add(Duration(milliseconds: durationMs));
      while (DateTime.now().isBefore(deadline)) {
        await tester.pump(const Duration(milliseconds: 16));
        await tester.runAsync(
          () => Future<void>.delayed(const Duration(milliseconds: 6)),
        );
      }
    } finally {
      SchedulerBinding.instance.removeTimingsCallback(collect);
    }

    final frames = submitted
        .where((t) => t.totalSpan.inMicroseconds > 0)
        .toList();
    if (frames.isEmpty) {
      return {
        'name': '${step['name'] ?? 'frames'}',
        'frames': 0,
        'limits': 'no frame timings were reported for this window',
      };
    }
    final builds =
        frames.map((t) => t.buildDuration.inMicroseconds / 1000.0).toList()
          ..sort();
    final rasters =
        frames.map((t) => t.rasterDuration.inMicroseconds / 1000.0).toList()
          ..sort();
    final spans =
        frames.map((t) => t.totalSpan.inMicroseconds / 1000.0).toList()..sort();
    final intervals = <double>[];
    for (var index = 1; index < frames.length; index += 1) {
      final gap =
          frames[index].timestampInMicroseconds(ui.FramePhase.vsyncStart) -
          frames[index - 1].timestampInMicroseconds(ui.FramePhase.vsyncStart);
      if (gap > 0) intervals.add(gap / 1000.0);
    }
    intervals.sort();
    double percentile(List<double> sorted, double fraction) =>
        sorted[((sorted.length - 1) * fraction).round()];
    final firstStamp = frames.first.timestampInMicroseconds(
      ui.FramePhase.vsyncStart,
    );
    final lastStamp = frames.last.timestampInMicroseconds(
      ui.FramePhase.vsyncStart,
    );
    final elapsedSeconds = lastStamp > firstStamp
        ? (lastStamp - firstStamp) / 1000000.0
        : 0.0;
    // A scenario can turn the numbers into a gate: crossing the budget fails
    // the step instead of leaving pacing as something a human compares by eye.
    void requireBudget(String key, double actual, String label) {
      final budget = (step[key] as num?)?.toDouble();
      if (budget != null && actual > budget) {
        throw _StepFailure(
          'frame_budget',
          '$label ${actual.toStringAsFixed(2)} ms exceeded the '
              '${budget.toStringAsFixed(2)} ms budget',
        );
      }
    }

    final buildP95 = percentile(builds, 0.95);
    final spanP95 = percentile(spans, 0.95);
    requireBudget('max_build_p95_ms', buildP95, 'build p95');
    requireBudget('max_span_p95_ms', spanP95, 'span p95');

    return {
      'name': '${step['name'] ?? 'frames'}',
      'frames': frames.length,
      'fps': elapsedSeconds > 0
          ? double.parse(
              ((frames.length - 1) / elapsedSeconds).toStringAsFixed(2),
            )
          : 0,
      'build_p50_ms': double.parse(percentile(builds, 0.50).toStringAsFixed(2)),
      'build_p95_ms': double.parse(percentile(builds, 0.95).toStringAsFixed(2)),
      'raster_p95_ms': double.parse(
        percentile(rasters, 0.95).toStringAsFixed(2),
      ),
      'span_p95_ms': double.parse(percentile(spans, 0.95).toStringAsFixed(2)),
      'gap_p95_ms': intervals.isEmpty
          ? 0
          : double.parse(percentile(intervals, 0.95).toStringAsFixed(2)),
      'gap_max_ms': intervals.isEmpty
          ? 0
          : double.parse(intervals.last.toStringAsFixed(2)),
      'limits':
          'engine FrameTiming on ${Platform.operatingSystem} in ${const bool.fromEnvironment('dart.vm.product') ? 'release' : 'a non-release build'}; '
          'timings describe this run only',
    };
  }

  Future<Map<String, Object?>> _measure(Map<String, Object?> step) async {
    final repeat = (step['repeat'] as num?)?.toInt() ?? 10;
    final samples = <double>[];
    for (var i = 0; i < repeat; i++) {
      final started = DateTime.now();
      for (final sub in (step['steps'] as List? ?? const [])) {
        if (sub is Map<String, Object?>) {
          await _dispatch(
            '${sub['op']}',
            sub,
            _StepResult(-1, '${sub['op']}', null),
          );
        }
      }
      samples.add(DateTime.now().difference(started).inMicroseconds / 1000.0);
    }
    samples.sort();
    double percentile(double fraction) {
      final index = ((samples.length - 1) * fraction).round();
      return samples[index];
    }

    return {
      'name': '${step['name'] ?? 'measurement'}',
      'repeat': repeat,
      'p50_ms': double.parse(percentile(0.50).toStringAsFixed(2)),
      'p95_ms': double.parse(percentile(0.95).toStringAsFixed(2)),
      'min_ms': double.parse(samples.first.toStringAsFixed(2)),
      'max_ms': double.parse(samples.last.toStringAsFixed(2)),
      'limits':
          'wall clock on ${Platform.operatingSystem} on this host through the '
          'Flutter test binding; software rendering is not device performance',
    };
  }

  // -- selectors ---------------------------------------------------------
  _Selector? _selector(Map<String, Object?> step) {
    final selector = step['selector'];
    if (selector is Map<String, Object?>) return selector;
    final inline = <String, Object?>{};
    for (final entry in step.entries) {
      if (const {
        'key',
        'key_regex',
        'label',
        'label_regex',
        'text',
        'role',
      }.contains(entry.key)) {
        inline[entry.key] = entry.value;
      }
    }
    if (inline.containsKey('role') && !inline.containsKey('label')) {
      // A role-only selector is allowed, but it must resolve through the tree.
      return {...inline};
    }
    return inline.isEmpty ? null : inline;
  }

  Finder _anyFinder(Map<String, Object?> step) {
    final selector = _selector(step);
    if (selector == null) {
      throw _StepFailure(
        'bad_scenario',
        'this step needs key, label, text or role',
      );
    }
    final scoped = _scopeFor(selector);
    final base = _baseFinder(selector);
    return scoped == null ? base : find.descendant(of: scoped, matching: base);
  }

  /// Restrict a selector to one container: `{"label": "Close", "within":
  /// {"type": "AlertDialog"}}` names the Close button *in the dialog*, which is
  /// the same control a user sees there. Scoping is still semantic; it never
  /// introduces a coordinate.
  Finder? _scopeFor(Map<String, Object?> selector) {
    final within = selector['within'];
    if (within is! Map) return null;
    final key = within['key'];
    if (key != null) return find.byKey(ValueKey('$key'));
    final type = within['type'];
    if (type != null) {
      final known = <String, Type>{
        'AlertDialog': AlertDialog,
        'Dialog': Dialog,
        'SnackBar': SnackBar,
        'Drawer': Drawer,
        'Scaffold': Scaffold,
        'NavigationBar': NavigationBar,
      };
      final resolved = known['$type'];
      if (resolved == null) {
        throw _StepFailure('bad_scenario', 'unknown container type "$type"');
      }
      return find.byType(resolved);
    }
    throw _StepFailure('bad_scenario', 'a within scope needs key or type');
  }

  Finder _baseFinder(Map<String, Object?> selector) {
    final key = selector['key'];
    if (key != null) return find.byKey(ValueKey('$key'));
    final keyRegex = selector['key_regex'];
    if (keyRegex != null) {
      final pattern = RegExp('$keyRegex');
      return find.byWidgetPredicate((widget) {
        final key = widget.key;
        return key is ValueKey<String> && pattern.hasMatch(key.value);
      });
    }
    final labelRegex = selector['label_regex'];
    if (labelRegex != null) return find.bySemanticsLabel(RegExp('$labelRegex'));
    final label = selector['label'];
    if (label != null) return find.bySemanticsLabel('$label');
    final text = selector['text'];
    if (text != null) return find.text('$text');
    final role = selector['role'];
    if (role != null) {
      final node = _semanticsNodeForRole('$role', index: 0);
      if (node == null) {
        throw _StepFailure('not_found', 'no semantics node with role $role');
      }
      // A role with no label can only be acted on through its own geometry.
      return _finderForNode(node);
    }
    throw _StepFailure(
      'bad_scenario',
      'the selector names nothing this runner can resolve',
    );
  }

  /// The finder a step names, optionally narrowed to the nth match.
  ///
  /// `index` exists because a scenario legitimately needs "the second clip on this
  /// track" when every clip carries a generated id, and the alternatives are either
  /// a coordinate (forbidden) or a guess. It is still a semantic selector: the
  /// match order is the tree's order, not a pixel position.
  Finder _requireFinder(Map<String, Object?> step) {
    final finder = _anyFinder(step);
    final index = step['index'];
    if (index is int) return finder.at(index);
    return finder;
  }

  SemanticsNode? _semanticsNodeForRole(String role, {int index = 0}) {
    final matched = <SemanticsNode>[];
    void visit(SemanticsNode node) {
      if (_roleOf(node) == role && node.label.isNotEmpty) matched.add(node);
      node.visitChildren((child) {
        visit(child);
        return true;
      });
    }

    final root = _semanticsRoot();
    if (root != null) visit(root);
    return index < matched.length ? matched[index] : null;
  }

  /// Map a semantics node back to the widget that owns it, so an action goes
  /// through the normal hit-test path instead of a bare coordinate.
  Finder _finderForNode(SemanticsNode node) {
    final rect = node.rect;
    return find.byElementPredicate((element) {
      final widget = element.widget;
      if (widget is! RenderObjectWidget) return false;
      final renderObject = element.renderObject;
      if (renderObject is! RenderBox || !renderObject.attached) return false;
      final origin = renderObject.localToGlobal(Offset.zero);
      final candidate = origin & renderObject.size;
      return (candidate.center - rect.center).distance < 1.0;
    });
  }

  static String _roleOf(SemanticsNode node) {
    final data = node.getSemanticsData();
    final flags = data.flagsCollection;
    if (flags.isButton) return 'button';
    if (flags.isTextField) return 'textfield';
    if (flags.isSlider) return 'slider';
    if (flags.isChecked == ui.CheckedState.isTrue) return 'checkbox';
    if (flags.isInMutuallyExclusiveGroup) return 'radio';
    if (flags.isLink) return 'link';
    if (flags.isImage) return 'image';
    if (flags.isHeader) return 'header';
    if (data.hasAction(ui.SemanticsAction.tap)) return 'tappable';
    if (data.hasAction(ui.SemanticsAction.scrollLeft) ||
        data.hasAction(ui.SemanticsAction.scrollUp)) {
      return 'scrollable';
    }
    return 'structure';
  }

  static String _describe(SemanticsData data) {
    final flags = data.flagsCollection;
    return [
      'label="${data.label}"',
      'value="${data.value}"',
      'button=${flags.isButton}',
      'enabled=${flags.isEnabled != ui.Tristate.isFalse}',
      'selected=${flags.isSelected == ui.Tristate.isTrue}',
      'checked=${flags.isChecked == ui.CheckedState.isTrue}',
      'focused=${data.flagsCollection.isFocused == ui.Tristate.isTrue}',
    ].join(' ');
  }

  /// Scroll a control into view and let the scroll settle before acting.
  ///
  /// `ensureVisible` on its own is not enough: a tap delivered in the same frame
  /// as the scroll is hit-tested against the pre-scroll position. Measured
  /// 2026-10-18 on the compact editor at 320x440, the timeline's Add Audio button
  /// reported a successful tap and no track was added, because the button had
  /// just been scrolled into view inside a horizontal viewport.
  Future<void> _scrollIntoView(Finder finder, {bool settle = true}) async {
    await tester.ensureVisible(finder);
    if (settle) await _settle();
  }

  // -- observation -------------------------------------------------------
  /// Expands `$OR_UI_ROOT` to the repository root the harness reports.
  ///
  /// A scenario that types a file path has to name a file that exists on every host, and
  /// the harness sets the workspace root in the environment it launches the app with. The
  /// expansion is deliberately narrow: one token, no shell, no other variable.
  String _expandRoot(String value) {
    if (!value.contains(r'$OR_UI_ROOT')) return value;
    final root = Platform.environment['GITHUB_WORKSPACE'] ?? '';
    return value.replaceAll(r'$OR_UI_ROOT', root);
  }

  Future<void> _settle() async {
    await tester.pump();
    for (var i = 0; i < 20; i++) {
      await tester.pump(const Duration(milliseconds: 16));
      await tester.runAsync(
        () => Future<void>.delayed(const Duration(milliseconds: 8)),
      );
    }
  }

  Future<void> _pumpFor(int milliseconds) async {
    final deadline = DateTime.now().add(Duration(milliseconds: milliseconds));
    while (DateTime.now().isBefore(deadline)) {
      await tester.pump(const Duration(milliseconds: 16));
      await tester.runAsync(
        () => Future<void>.delayed(const Duration(milliseconds: 8)),
      );
    }
  }

  Future<void> _waitForText(
    String text,
    int budgetMs, {
    bool absent = false,
  }) async {
    final deadline = DateTime.now().add(Duration(milliseconds: budgetMs));
    while (DateTime.now().isBefore(deadline)) {
      final present = find.text(text).evaluate().isNotEmpty;
      if (present != absent) return;
      await tester.pump(const Duration(milliseconds: 16));
      await tester.runAsync(
        () => Future<void>.delayed(const Duration(milliseconds: 8)),
      );
    }
    throw _StepFailure(
      'timeout',
      absent
          ? 'text "$text" was still visible after ${budgetMs}ms'
          : 'text "$text" never appeared within ${budgetMs}ms',
    );
  }

  static bool _matchesState(SemanticsData data, String expected) =>
      switch (expected) {
        'enabled' => data.flagsCollection.isEnabled != ui.Tristate.isFalse,
        'disabled' => data.flagsCollection.isEnabled == ui.Tristate.isFalse,
        'selected' => data.flagsCollection.isSelected == ui.Tristate.isTrue,
        'checked' => data.flagsCollection.isChecked == ui.CheckedState.isTrue,
        'focused' => data.flagsCollection.isFocused == ui.Tristate.isTrue,
        'toggled' => data.flagsCollection.isToggled == ui.Tristate.isTrue,
        _ => throw _StepFailure(
          'bad_scenario',
          'unknown semantic state "$expected"',
        ),
      };

  Future<void> _waitForState(Map<String, Object?> step) async {
    final budget = _milliseconds(step['budget_ms'] ?? 8000);
    final finder = _requireFinder(step);
    final expected = '${step['state']}';
    final deadline = DateTime.now().add(Duration(milliseconds: budget));
    while (DateTime.now().isBefore(deadline)) {
      if (finder.evaluate().isNotEmpty &&
          _matchesState(
            tester.getSemantics(finder.first).getSemanticsData(),
            expected,
          )) {
        return;
      }
      await tester.pump(const Duration(milliseconds: 16));
      await tester.runAsync(
        () => Future<void>.delayed(const Duration(milliseconds: 8)),
      );
    }
    throw _StepFailure(
      'timeout',
      'the control never entered state "$expected" within ${budget}ms',
    );
  }

  /// Wait for a semantics label, which is the portable selector: the same
  /// control presents its name as its own Text widget on desktop and as one
  /// multi-line semantics label in the compact sheet.
  Future<void> _waitForLabel(
    String pattern,
    int budgetMs, {
    bool absent = false,
  }) async {
    final regex = RegExp(pattern);
    final deadline = DateTime.now().add(Duration(milliseconds: budgetMs));
    while (DateTime.now().isBefore(deadline)) {
      final present = find.bySemanticsLabel(regex).evaluate().isNotEmpty;
      if (present != absent) return;
      await tester.pump(const Duration(milliseconds: 16));
      await tester.runAsync(
        () => Future<void>.delayed(const Duration(milliseconds: 8)),
      );
    }
    throw _StepFailure(
      'timeout',
      absent
          ? 'a control labelled /$pattern/ was still present after ${budgetMs}ms'
          : 'no control labelled /$pattern/ appeared within ${budgetMs}ms',
    );
  }

  Future<String> _shot(String name) async {
    await _settle();
    final file = 'shot-$name.png';
    try {
      final render =
          boundary.currentContext!.findRenderObject()! as RenderRepaintBoundary;
      final image = await render.toImage(pixelRatio: 1);
      final bytes = await image.toByteData(format: ui.ImageByteFormat.png);
      image.dispose();
      artifacts.writeBytes(file, bytes!.buffer.asUint8List());
    } catch (error) {
      throw _StepFailure('shot_failed', 'could not capture $name: $error');
    }
    return file;
  }

  Future<String> _tree(String name) async {
    final root = _semanticsRoot();
    if (root == null) throw _StepFailure('no_semantics', 'no semantics root');
    final nodes = <Map<String, Object?>>[];
    // `node.rect` is local to the node; a layout review needs the global rect the
    // user actually sees, so accumulate the transform chain from the root.
    void visit(SemanticsNode node, Matrix4 parentTransform) {
      final data = node.getSemanticsData();
      final transform = parentTransform.multiplied(
        node.transform ?? Matrix4.identity(),
      );
      final rect = MatrixUtils.transformRect(transform, node.rect);
      nodes.add({
        'id': node.id,
        'role': _roleOf(node),
        'label': data.label,
        'value': data.value,
        'hint': data.hint,
        'actions': [
          for (final action in ui.SemanticsAction.values)
            if (data.hasAction(action)) action.name,
        ]..sort(),
        'rect': {
          'left': rect.left,
          'top': rect.top,
          'width': rect.width,
          'height': rect.height,
        },
        'children': node.childrenCount,
      });
      node.visitChildren((child) {
        visit(child, transform);
        return true;
      });
    }

    visit(root, Matrix4.identity());
    artifacts.writeText('tree-$name.txt', root.toStringDeep());
    artifacts.writeJson('tree-$name.json', {'nodes': nodes});
    return 'tree-$name.json';
  }

  /// The root of the live semantics tree.
  ///
  /// A [RenderView] owns its own pipeline owner, so the tree hangs off the
  /// render view rather than off the root pipeline owner of the binding.
  SemanticsNode? _semanticsRoot() {
    for (final renderView in tester.binding.renderViews) {
      final node = renderView.owner?.semanticsOwner?.rootSemanticsNode;
      if (node != null) return node;
    }
    return tester.binding.rootPipelineOwner.semanticsOwner?.rootSemanticsNode;
  }

  /// The screen inventory, derived from the running app rather than a list in
  /// this file: every navigation destination the shell exposes is visited and
  /// described by the semantics it produces.
  Future<void> _captureScreenInventory() async {
    final destinations = <String>[];
    for (final element in find.byWidgetPredicate((widget) {
      final key = widget.key;
      return key is ValueKey<String> && key.value.startsWith('nav-');
    }).evaluate()) {
      final key = (element.widget.key! as ValueKey<String>).value;
      final name = key.substring('nav-'.length);
      if (!destinations.contains(name)) destinations.add(name);
    }
    final screens = <Map<String, Object?>>[];
    for (final name in destinations) {
      final finder = find.byKey(ValueKey('nav-$name'));
      if (finder.evaluate().isEmpty) continue;
      await tester.tap(finder.first);
      await _settle();
      final treeName = 'screen-$name';
      await _tree(treeName);
      await _shot(treeName);
      final root = _semanticsRoot();
      final labels = <String>[];
      void visit(SemanticsNode node) {
        if (node.label.isNotEmpty) labels.add(node.label);
        node.visitChildren((child) {
          visit(child);
          return true;
        });
      }

      if (root != null) visit(root);
      screens.add({
        'destination': name,
        'key': 'nav-$name',
        'labels': labels.take(40).toList(),
        'semantics': 'tree-$treeName.json',
        'screenshot': 'shot-$treeName.png',
      });
    }
    artifacts.writeJson('screen-inventory.json', {
      'platform': platform,
      'count': screens.length,
      'screens': screens,
    });
    screenInventory = destinations;
  }

  int _milliseconds(Object? value) => switch (value) {
    null => 100,
    num number => number.toInt(),
    _ => int.parse('$value'),
  };
}

class _StepFailure implements Exception {
  _StepFailure(this.status, this.message);
  final String status;
  final String message;

  @override
  String toString() => message;
}

/// The only host integration the runner replaces: the OS file picker. It serves
/// synthetic fixtures from the artifact directory, so an import journey is a
/// real import of a real file through the real Rust command path.
class _FixturePicker extends FileSelectorProjectPicker {
  _FixturePicker(this.directory, List<String> fixtures) {
    // `copy:<path>` fixtures come from the host so a journey can import real
    // video without the runner generating a container it cannot encode. The
    // file is copied into the scenario directory and treated as the picked
    // media, which keeps the import path (picker -> Rust probe) real.
    String? copied;
    String? projectFixture;
    for (final fixture in fixtures) {
      if (!fixture.startsWith('copy:')) continue;
      final source = File(fixture.substring('copy:'.length));
      if (!source.existsSync()) continue;
      // A copied `.orproj` fixture is the project the scenario opens; it lives
      // in the scenario directory so a save writes there, not into the
      // repository fixture.
      if (source.path.toLowerCase().endsWith('.orproj')) {
        projectFixture ??= source.path;
        continue;
      }
      if (copied != null) continue;
      final target = '${directory.path}/${source.uri.pathSegments.last}';
      source.copySync(target);
      copied = target;
    }
    // A `noisy` fixture carries signal: the silent one cannot exercise anything
    // that measures or filters audio, because digital silence has no noise floor.
    media =
        copied ??
        (fixtures.contains('noisy')
            ? _noisyWav('fixture.wav')
            : _wav('fixture.wav'));
    if (fixtures.contains('invalid')) {
      badMedia = '${directory.path}/invalid.wav';
      File(badMedia).writeAsStringSync('synthetic invalid media');
    } else {
      badMedia = media;
    }
    project = '${directory.path}/scenario.orproj';
    if (projectFixture != null) {
      File(projectFixture).copySync(project);
    }
  }

  final Directory directory;

  /// The caption import journey needs a real SubRip file the review dialog
  /// can read. The fixture picker writes one into the scenario directory so
  /// the import runs through the real preview and import commands.
  @override
  Future<String?> openCaptionFile() async {
    final path = '${directory.path}/scenario-captions.srt';
    File(path).writeAsStringSync(
      '1\n00:00:00,000 --> 00:00:02,000\nHarness caption one\n\n'
      '2\n00:00:02,000 --> 00:00:04,000\nHarness caption two\n\n',
    );
    return path;
  }

  late final String media;
  late final String badMedia;
  late final String project;
  String next = 'media';

  String _wav(String name) {
    final path = '${directory.path}/$name';
    final wav = ByteData(16044);
    void ascii(int offset, String value) {
      for (var i = 0; i < value.length; i++) {
        wav.setUint8(offset + i, value.codeUnitAt(i));
      }
    }

    ascii(0, 'RIFF');
    wav.setUint32(4, 16036, Endian.little);
    ascii(8, 'WAVEfmt ');
    wav.setUint32(16, 16, Endian.little);
    wav.setUint16(20, 1, Endian.little);
    wav.setUint16(22, 1, Endian.little);
    wav.setUint32(24, 8000, Endian.little);
    wav.setUint32(28, 16000, Endian.little);
    wav.setUint16(32, 2, Endian.little);
    wav.setUint16(34, 16, Endian.little);
    ascii(36, 'data');
    wav.setUint32(40, 16000, Endian.little);
    File(path).writeAsBytesSync(wav.buffer.asUint8List());
    return path;
  }

  /// One second of white noise with a tone over the second half, written as
  /// 8 kHz mono PCM. The generator is a small linear congruential sequence so the
  /// fixture is identical on every run and on every host.
  String _noisyWav(String name) {
    const rate = 8000;
    const samples = rate;
    final bytes = ByteData(44 + samples * 2);
    void ascii(int offset, String value) {
      for (var i = 0; i < value.length; i++) {
        bytes.setUint8(offset + i, value.codeUnitAt(i));
      }
    }

    ascii(0, 'RIFF');
    bytes.setUint32(4, 36 + samples * 2, Endian.little);
    ascii(8, 'WAVEfmt ');
    bytes.setUint32(16, 16, Endian.little);
    bytes.setUint16(20, 1, Endian.little);
    bytes.setUint16(22, 1, Endian.little);
    bytes.setUint32(24, rate, Endian.little);
    bytes.setUint32(28, rate * 2, Endian.little);
    bytes.setUint16(32, 2, Endian.little);
    bytes.setUint16(34, 16, Endian.little);
    ascii(36, 'data');
    bytes.setUint32(40, samples * 2, Endian.little);

    var seed = 20261020;
    for (var index = 0; index < samples; index += 1) {
      seed = (seed * 1103515245 + 12345) & 0x7fffffff;
      final noise = (seed / 0x3fffffff) - 1.0;
      final tone = index >= samples ~/ 2
          ? 0.5 * math.sin(2 * math.pi * 440 * index / rate)
          : 0.0;
      final value = ((noise * 0.12 + tone) * 32767)
          .clamp(-32768, 32767)
          .round();
      bytes.setInt16(44 + index * 2, value, Endian.little);
    }
    final path = '${directory.path}/$name';
    File(path).writeAsBytesSync(bytes.buffer.asUint8List());
    return path;
  }

  String get _resolved => switch (next) {
    'invalid' => badMedia,
    _ => media,
  };

  @override
  Future<String?> saveProjectPath({required String suggestedName}) async =>
      project;

  @override
  Future<String?> openProjectPath() async => project;

  // A deterministic export destination keeps the packaged export lane real
  // (Rust render + FFmpeg writer) without a native save panel in the runner.
  @override
  bool get supportsExport => true;

  @override
  Future<String?> saveExportPath({
    required String suggestedName,
    required ExportProfile profile,
  }) async => '${directory.path}/export.${profile.extension}';

  @override
  Future<List<String>> openMediaSources() async => [_resolved];
}
