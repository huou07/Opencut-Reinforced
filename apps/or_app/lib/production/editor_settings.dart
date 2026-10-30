import 'dart:convert';
import 'dart:io';

import 'package:path_provider/path_provider.dart';

/// Presentation preferences for the editor shell.
///
/// These are not project state: hiding a control never changes an edit, and the
/// same project opens identically on a machine with a different preference.
class EditorSettings {
  const EditorSettings({this.showInspector = false});

  /// The inspector is an advanced surface. The toolbar and clip dialogs carry
  /// the frequent controls, so the rail tab starts hidden.
  final bool showInspector;

  EditorSettings copyWith({bool? showInspector}) =>
      EditorSettings(showInspector: showInspector ?? this.showInspector);

  Map<String, Object?> toJson() => {
    'version': 1,
    'show_inspector': showInspector,
  };

  static EditorSettings fromJson(Object? value) {
    if (value is! Map<String, Object?>) return const EditorSettings();
    return EditorSettings(showInspector: value['show_inspector'] == true);
  }
}

class EditorSettingsStore {
  EditorSettingsStore({Future<Directory> Function()? supportDirectory})
    : _supportDirectory = supportDirectory ?? getApplicationSupportDirectory;

  final Future<Directory> Function() _supportDirectory;
  Future<void> _pending = Future<void>.value();

  Future<File> settingsFile() async {
    final support = await _supportDirectory();
    return File('${support.path}${Platform.pathSeparator}editor-settings.json');
  }

  Future<EditorSettings> load() async {
    try {
      final file = await settingsFile();
      if (!await file.exists()) return const EditorSettings();
      return EditorSettings.fromJson(jsonDecode(await file.readAsString()));
    } catch (_) {
      return const EditorSettings();
    }
  }

  Future<void> save(EditorSettings settings) {
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
