import 'dart:io';

import 'package:file_selector/file_selector.dart';

abstract interface class ProjectFilePicker {
  bool get isSupported;
  Future<String?> openProjectPath();
  Future<String?> saveProjectPath({required String suggestedName});
}

class FileSelectorProjectPicker implements ProjectFilePicker {
  const FileSelectorProjectPicker();

  static const _projectType = XTypeGroup(
    label: 'Opencut Reinforced project',
    extensions: ['orproj'],
  );

  @override
  bool get isSupported =>
      Platform.isMacOS || Platform.isWindows || Platform.isLinux;

  @override
  Future<String?> openProjectPath() async {
    if (!isSupported) return null;
    final file = await openFile(acceptedTypeGroups: [_projectType]);
    return file?.path;
  }

  @override
  Future<String?> saveProjectPath({required String suggestedName}) async {
    if (!isSupported) return null;
    final location = await getSaveLocation(
      acceptedTypeGroups: [_projectType],
      suggestedName: suggestedName,
    );
    return location?.path;
  }
}
