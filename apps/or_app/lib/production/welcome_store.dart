import 'dart:convert';
import 'dart:io';

import 'package:path_provider/path_provider.dart';

/// Whether the first-run panel has been dismissed.
///
/// This is not project state: dismissing the panel never changes an edit, and the
/// same project opens identically on a machine that has seen the panel and one
/// that has not. It is stored beside the editor preferences for the same reason.
class WelcomeState {
  const WelcomeState({this.dismissed = false});

  final bool dismissed;

  WelcomeState copyWith({bool? dismissed}) =>
      WelcomeState(dismissed: dismissed ?? this.dismissed);

  Map<String, Object?> toJson() => {'version': 1, 'dismissed': dismissed};

  static WelcomeState fromJson(Object? value) {
    if (value is! Map<String, Object?>) return const WelcomeState();
    return WelcomeState(dismissed: value['dismissed'] == true);
  }
}

class WelcomeStore {
  WelcomeStore({Future<Directory> Function()? supportDirectory})
    : _supportDirectory = supportDirectory ?? getApplicationSupportDirectory;

  final Future<Directory> Function() _supportDirectory;
  Future<void> _pending = Future<void>.value();

  Future<File> stateFile() async {
    final support = await _supportDirectory();
    return File('${support.path}${Platform.pathSeparator}welcome.json');
  }

  /// A missing or unreadable file means "not dismissed": the panel is the only
  /// thing that explains the app, so a broken file must not hide it forever.
  Future<WelcomeState> load() async {
    try {
      final file = await stateFile();
      if (!await file.exists()) return const WelcomeState();
      return WelcomeState.fromJson(jsonDecode(await file.readAsString()));
    } catch (_) {
      return const WelcomeState();
    }
  }

  Future<void> save(WelcomeState state) {
    final write = _pending.then((_) async {
      final file = await stateFile();
      await file.parent.create(recursive: true);
      final temporary = File('${file.path}.tmp');
      await temporary.writeAsString(jsonEncode(state.toJson()));
      await temporary.rename(file.path);
    });
    _pending = write.catchError((_) {});
    return write;
  }
}
