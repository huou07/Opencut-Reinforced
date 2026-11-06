import 'dart:convert';
import 'dart:io';

import 'package:path_provider/path_provider.dart';

/// Presentation preferences for the editor shell.
///
/// These are not project state: hiding a control never changes an edit, and the
/// same project opens identically on a machine with a different preference.
class EditorSettings {
  const EditorSettings({
    this.showInspector = false,
    this.simpleMode = true,
    this.showGuides = false,
    this.tipsDismissed = false,
    this.performanceProfile = 'recommended',
    this.preferredH264Encoder,
    this.graphicsBackend = 'auto',
  });

  /// The inspector is an advanced surface. The toolbar and clip dialogs carry
  /// the frequent controls, so the rail tab starts hidden.
  final bool showInspector;

  /// Guided mode, the product's "Simple mode": the editor keeps one extra row
  /// that names the next useful action, and the export dialog leads with what
  /// the file is for instead of codec choices. It is a visibility preference —
  /// every command stays reachable and the project format does not change.
  final bool simpleMode;

  /// Title/action safe areas, thirds and centre drawn over the viewer. They are
  /// a preview overlay only and never reach the exported frame.
  final bool showGuides;

  /// The next-step row can be hidden on its own, without turning off the rest
  /// of guided mode. Settings owns the switch that brings it back.
  final bool tipsDismissed;

  /// How exports should use this machine, by wire name: `recommended`,
  /// `maximum_performance`, `balanced`, or `compatibility`. The engine still
  /// refuses an encoder that does not open here; the profile is a preference,
  /// not a promise.
  final String performanceProfile;

  /// An H.264 encoder the person chose by name, or null to let the profile
  /// decide. A name this build does not carry falls back and says why.
  final String? preferredH264Encoder;

  /// The graphics backend the renderer opens on the next start: `auto` or one
  /// of `metal`, `vulkan`, `dx12`, `gles`. The device is opened once per
  /// process, so a change is stored and applied next launch.
  final String graphicsBackend;

  EditorSettings copyWith({
    bool? showInspector,
    bool? simpleMode,
    bool? showGuides,
    bool? tipsDismissed,
    String? performanceProfile,
    String? graphicsBackend,
  }) => EditorSettings(
    showInspector: showInspector ?? this.showInspector,
    simpleMode: simpleMode ?? this.simpleMode,
    showGuides: showGuides ?? this.showGuides,
    tipsDismissed: tipsDismissed ?? this.tipsDismissed,
    performanceProfile: performanceProfile ?? this.performanceProfile,
    preferredH264Encoder: preferredH264Encoder,
    graphicsBackend: graphicsBackend ?? this.graphicsBackend,
  );

  /// Sets or clears the encoder override without touching the other fields.
  EditorSettings withPreferredH264Encoder(String? encoder) => EditorSettings(
    showInspector: showInspector,
    simpleMode: simpleMode,
    showGuides: showGuides,
    tipsDismissed: tipsDismissed,
    performanceProfile: performanceProfile,
    preferredH264Encoder: encoder,
    graphicsBackend: graphicsBackend,
  );

  Map<String, Object?> toJson() => {
    'version': 4,
    'show_inspector': showInspector,
    'simple_mode': simpleMode,
    'show_guides': showGuides,
    'tips_dismissed': tipsDismissed,
    'performance_profile': performanceProfile,
    'preferred_h264_encoder': preferredH264Encoder,
    'graphics_backend': graphicsBackend,
  };

  /// A version-1 file predates guided mode and guides. Guided mode is on by
  /// default, so an upgrade opens in the beginner surface rather than silently
  /// keeping the dense one.
  static EditorSettings fromJson(Object? value) {
    if (value is! Map<String, Object?>) return const EditorSettings();
    return EditorSettings(
      showInspector: value['show_inspector'] == true,
      simpleMode: value['simple_mode'] != false,
      showGuides: value['show_guides'] == true,
      tipsDismissed: value['tips_dismissed'] == true,
      performanceProfile:
          value['performance_profile'] ==
              'maximum_performance' ||
          value['performance_profile'] == 'balanced' ||
          value['performance_profile'] == 'compatibility'
      ? value['performance_profile'] as String
      : 'recommended',
      preferredH264Encoder: value['preferred_h264_encoder'] is String
          ? value['preferred_h264_encoder'] as String
          : null,
      graphicsBackend: switch (value['graphics_backend']) {
        'metal' || 'vulkan' || 'dx12' || 'gles' =>
          value['graphics_backend'] as String,
        _ => 'auto',
      },
    );
  }
}

class EditorSettingsStore {
  /// [loader] and [saver] replace the file with an in-memory value. Widget tests
  /// need that: a `testWidgets` body runs inside fake async, where a real file
  /// read never completes, so the store must resolve on a microtask instead.
  EditorSettingsStore({
    Future<Directory> Function()? supportDirectory,
    this.loader,
    this.saver,
  }) : _supportDirectory = supportDirectory ?? getApplicationSupportDirectory;

  final Future<Directory> Function() _supportDirectory;
  final Future<EditorSettings> Function()? loader;
  final Future<void> Function(EditorSettings settings)? saver;
  Future<void> _pending = Future<void>.value();

  Future<File> settingsFile() async {
    final support = await _supportDirectory();
    return File('${support.path}${Platform.pathSeparator}editor-settings.json');
  }

  Future<EditorSettings> load() async {
    final read = loader;
    if (read != null) {
      try {
        return await read();
      } catch (_) {
        return const EditorSettings();
      }
    }
    try {
      final file = await settingsFile();
      if (!await file.exists()) return const EditorSettings();
      return EditorSettings.fromJson(jsonDecode(await file.readAsString()));
    } catch (_) {
      return const EditorSettings();
    }
  }

  Future<void> save(EditorSettings settings) {
    final write0 = saver;
    if (write0 != null) {
      return _pending = write0(settings);
    }
    final write = _pending.then((_) async {
      final file = await settingsFile();
      await file.parent.create(recursive: true);
      final temporary = File('${file.path}.tmp');
      await temporary.writeAsString(jsonEncode(settings.toJson()));
      await temporary.rename(file.path);
    });
    _pending = write.catchError((_) {});
    return write;
  }
}
