import 'dart:io';

import 'package:file_selector/file_selector.dart';
import 'package:flutter/services.dart';

abstract interface class ProjectFilePicker {
  bool get isSupported;
  bool get supportsMediaImport;
  bool get supportsExport;
  Future<String?> openProjectPath();
  Future<List<String>> openMediaSources();
  Future<String?> saveProjectPath({required String suggestedName});
  Future<String?> saveExportPath({required String suggestedName});
  Future<String?> openCaptionFile();
  Future<void> cleanupCaptionFile(String path);
  Future<String?> saveCaptionPath({required String suggestedName});
  Future<void> publishCaptionPath(String path);
  Future<void> discardCaptionPath(String path);
  Future<void> publishExportPath(String path);
  Future<void> discardExportPath(String path);
  Future<void> cancelExportPublish(String path);
  Future<ProjectFileSyncResult?> synchronizeProjectPath(String path);
}

class ProjectFileSyncResult {
  const ProjectFileSyncResult({required this.verified});

  final bool verified;
}

class ProjectSafStorageException implements Exception {
  const ProjectSafStorageException(this.message);

  final String message;
}

class FileSelectorProjectPicker implements ProjectFilePicker {
  const FileSelectorProjectPicker();

  static const _projectType = XTypeGroup(
    label: 'Opencut Reinforced project',
    extensions: ['orproj'],
  );
  static const _exportType = XTypeGroup(
    label: 'Matroska video',
    extensions: ['mkv'],
  );
  static const _captionType = XTypeGroup(
    label: 'SubRip or WebVTT captions',
    extensions: ['srt', 'vtt'],
  );

  @override
  bool get isSupported =>
      Platform.isMacOS || Platform.isWindows || Platform.isLinux;

  @override
  bool get supportsMediaImport => isSupported;

  @override
  bool get supportsExport => isSupported;

  @override
  Future<String?> openProjectPath() async {
    if (!isSupported) return null;
    final file = await openFile(acceptedTypeGroups: [_projectType]);
    return file?.path;
  }

  @override
  Future<List<String>> openMediaSources() async {
    if (!isSupported) return const [];
    final files = await openFiles();
    return files.map((file) => file.path).toList(growable: false);
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

  @override
  Future<String?> saveExportPath({required String suggestedName}) async {
    if (!isSupported) return null;
    final location = await getSaveLocation(
      acceptedTypeGroups: [_exportType],
      suggestedName: suggestedName,
    );
    return location?.path;
  }

  @override
  Future<String?> openCaptionFile() async {
    if (!isSupported) return null;
    final file = await openFile(acceptedTypeGroups: [_captionType]);
    return file?.path;
  }

  @override
  Future<void> cleanupCaptionFile(String path) async {}

  @override
  Future<String?> saveCaptionPath({required String suggestedName}) async {
    if (!isSupported) return null;
    final location = await getSaveLocation(
      acceptedTypeGroups: [_captionType],
      suggestedName: suggestedName,
    );
    return location?.path;
  }

  @override
  Future<void> publishCaptionPath(String path) async {}

  @override
  Future<void> discardCaptionPath(String path) async {}

  @override
  Future<void> publishExportPath(String path) async {}

  @override
  Future<void> discardExportPath(String path) async {}

  @override
  Future<void> cancelExportPublish(String path) async {}

  @override
  Future<ProjectFileSyncResult?> synchronizeProjectPath(String path) async =>
      null;
}

class AndroidSafProjectPicker implements ProjectFilePicker {
  AndroidSafProjectPicker({MethodChannel? channel})
    : _channel = channel ?? _channelInstance;

  static const _channelInstance = MethodChannel(
    'io.github.huou07.or_app/saf_storage',
  );

  final MethodChannel _channel;
  final Map<String, String> _documentUrisByWorkingPath = {};
  final Map<String, String> _exportUrisByWorkingPath = {};
  final Set<String> _captionPaths = {};
  final Map<String, String> _captionExportUrisByWorkingPath = {};

  @override
  bool get isSupported => Platform.isAndroid;

  @override
  bool get supportsMediaImport => true;

  @override
  bool get supportsExport => true;

  @override
  Future<String?> openProjectPath() => _selectProject('openProject');

  @override
  Future<String?> saveProjectPath({required String suggestedName}) =>
      _selectProject('createProject', {'suggestedName': suggestedName});

  Future<String?> _selectProject(
    String method, [
    Map<String, Object?>? arguments,
  ]) async {
    try {
      final response = await _channel.invokeMapMethod<String, Object?>(
        method,
        arguments,
      );
      if (response == null) return null;
      final path = response['workingPath'];
      final documentUri = response['documentUri'];
      if (path is! String || documentUri is! String) {
        throw const ProjectSafStorageException(
          'The selected project location is invalid.',
        );
      }
      final uri = Uri.tryParse(documentUri);
      if (uri == null || uri.scheme != 'content' || uri.authority.isEmpty) {
        throw const ProjectSafStorageException(
          'The selected project location is invalid.',
        );
      }
      _documentUrisByWorkingPath[path] = documentUri;
      return path;
    } on PlatformException catch (error) {
      throw _storageError(error.code);
    } on MissingPluginException {
      throw const ProjectSafStorageException(
        'Android project storage is unavailable.',
      );
    }
  }

  @override
  Future<List<String>> openMediaSources() async {
    try {
      final response = await _channel.invokeMapMethod<String, Object?>(
        'openMedia',
      );
      if (response == null) return const [];
      final rawSources = response['sourceUris'];
      if (rawSources is! List<Object?> || rawSources.isEmpty) {
        throw const ProjectSafStorageException(
          'The selected media sources are invalid.',
        );
      }
      final sources = <String>[];
      for (final rawSource in rawSources) {
        if (rawSource is! String) {
          throw const ProjectSafStorageException(
            'The selected media sources are invalid.',
          );
        }
        final uri = Uri.tryParse(rawSource);
        if (uri == null || uri.scheme != 'content' || uri.authority.isEmpty) {
          throw const ProjectSafStorageException(
            'The selected media sources are invalid.',
          );
        }
        if (!sources.contains(rawSource)) sources.add(rawSource);
      }
      return sources;
    } on PlatformException catch (error) {
      throw _storageError(error.code);
    } on MissingPluginException {
      throw const ProjectSafStorageException(
        'Android media storage is unavailable.',
      );
    }
  }

  @override
  Future<String?> saveExportPath({required String suggestedName}) async {
    try {
      final response = await _channel.invokeMapMethod<String, Object?>(
        'createExport',
        {'suggestedName': suggestedName},
      );
      if (response == null) return null;
      final path = response['workingPath'];
      final documentUri = response['documentUri'];
      if (path is! String || documentUri is! String) {
        throw const ProjectSafStorageException(
          'The selected export location is invalid.',
        );
      }
      final uri = Uri.tryParse(documentUri);
      if (uri == null || uri.scheme != 'content' || uri.authority.isEmpty) {
        throw const ProjectSafStorageException(
          'The selected export location is invalid.',
        );
      }
      _exportUrisByWorkingPath[path] = documentUri;
      return path;
    } on PlatformException catch (error) {
      throw _storageError(error.code);
    } on MissingPluginException {
      throw const ProjectSafStorageException(
        'Android export storage is unavailable.',
      );
    }
  }

  @override
  Future<String?> openCaptionFile() async {
    try {
      final response = await _channel.invokeMapMethod<String, Object?>(
        'openCaptionFile',
      );
      if (response == null) return null;
      final path = response['workingPath'];
      if (path is! String || path.isEmpty) {
        throw const ProjectSafStorageException(
          'The selected caption file is invalid.',
        );
      }
      _captionPaths.add(path);
      return path;
    } on PlatformException catch (error) {
      throw _storageError(error.code);
    } on MissingPluginException {
      throw const ProjectSafStorageException(
        'Android caption storage is unavailable.',
      );
    }
  }

  @override
  Future<void> cleanupCaptionFile(String path) async {
    if (!_captionPaths.remove(path)) return;
    try {
      await _channel.invokeMethod<void>('deleteCaptionFile', {
        'workingPath': path,
      });
    } on PlatformException catch (error) {
      throw _storageError(error.code);
    }
  }

  @override
  Future<String?> saveCaptionPath({required String suggestedName}) async {
    final extension = suggestedName.toLowerCase().endsWith('.vtt')
        ? 'vtt'
        : 'srt';
    final mimeType = extension == 'vtt' ? 'text/vtt' : 'application/x-subrip';
    try {
      final response = await _channel.invokeMapMethod<String, Object?>(
        'createCaptionExport',
        {
          'suggestedName': suggestedName,
          'extension': extension,
          'mimeType': mimeType,
        },
      );
      if (response == null) return null;
      final path = response['workingPath'];
      final documentUri = response['documentUri'];
      final uri = documentUri is String ? Uri.tryParse(documentUri) : null;
      if (path is! String ||
          uri == null ||
          uri.scheme != 'content' ||
          uri.authority.isEmpty) {
        throw const ProjectSafStorageException(
          'The selected caption export location is invalid.',
        );
      }
      _captionExportUrisByWorkingPath[path] = documentUri as String;
      return path;
    } on PlatformException catch (error) {
      throw _storageError(error.code);
    } on MissingPluginException {
      throw const ProjectSafStorageException(
        'Android caption storage is unavailable.',
      );
    }
  }

  @override
  Future<void> publishCaptionPath(String path) async {
    final uri = _captionExportUrisByWorkingPath[path];
    if (uri == null) {
      throw const ProjectSafStorageException(
        'The selected caption export location is no longer available.',
      );
    }
    try {
      await _channel.invokeMethod<void>('publishCaptionExport', {
        'workingPath': path,
      });
      _captionExportUrisByWorkingPath.remove(path);
    } on PlatformException catch (error) {
      throw _storageError(error.code);
    }
  }

  @override
  Future<void> discardCaptionPath(String path) async {
    if (_captionExportUrisByWorkingPath.remove(path) == null) return;
    try {
      await _channel.invokeMethod<void>('discardCaptionExport', {
        'workingPath': path,
      });
    } on PlatformException catch (error) {
      throw _storageError(error.code);
    }
  }

  @override
  Future<void> publishExportPath(String path) async {
    final documentUri = _exportUrisByWorkingPath[path];
    if (documentUri == null) {
      throw const ProjectSafStorageException(
        'The selected export location is no longer available.',
      );
    }
    try {
      await _channel.invokeMethod<void>('publishExport', {
        'workingPath': path,
        'documentUri': documentUri,
      });
      _exportUrisByWorkingPath.remove(path);
    } on PlatformException catch (error) {
      throw _storageError(error.code);
    }
  }

  @override
  Future<void> discardExportPath(String path) async {
    final documentUri = _exportUrisByWorkingPath.remove(path);
    if (documentUri == null) return;
    try {
      await _channel.invokeMethod<void>('discardExport', {
        'workingPath': path,
        'documentUri': documentUri,
      });
    } on PlatformException catch (error) {
      throw _storageError(error.code);
    }
  }

  @override
  Future<void> cancelExportPublish(String path) async {
    try {
      await _channel.invokeMethod<void>('cancelExportPublish', {
        'workingPath': path,
      });
    } on PlatformException catch (error) {
      throw _storageError(error.code);
    }
  }

  @override
  Future<ProjectFileSyncResult?> synchronizeProjectPath(String path) async {
    final documentUri = _documentUrisByWorkingPath[path];
    if (documentUri == null) return null;
    try {
      final response = await _channel.invokeMapMethod<String, Object?>(
        'synchronizeProject',
        {'workingPath': path, 'documentUri': documentUri},
      );
      if (response?['verified'] case final bool verified) {
        return ProjectFileSyncResult(verified: verified);
      }
      throw const ProjectSafStorageException(
        'The external project could not be synchronized.',
      );
    } on PlatformException catch (error) {
      throw _storageError(error.code);
    } on MissingPluginException {
      throw const ProjectSafStorageException(
        'Android project storage is unavailable.',
      );
    }
  }

  static ProjectSafStorageException _storageError(
    String code,
  ) => ProjectSafStorageException(switch (code) {
    'CAPTION_FILE_TOO_LARGE' => 'Caption files must be 8 MiB or smaller.',
    'CAPTION_PICK_FAILED' => 'The selected caption file could not be opened.',
    'CAPTION_EXPORT_PERMISSION_REQUIRED' =>
      'The selected caption location cannot be written.',
    'CAPTION_EXPORT_PATH_INVALID' =>
      'Choose a valid SRT or WebVTT caption location.',
    'CAPTION_EXPORT_FAILED' => 'The caption file could not be saved.',
    'EXPORT_CANCELLED' => 'Export cancelled',
    'EXPORT_PERMISSION_REQUIRED' =>
      'Access to the selected export location is unavailable.',
    'EXPORT_SAVE_FAILED' =>
      'The exported video could not be saved to the selected location.',
    'EXPORT_BUSY' => 'The export is already being saved.',
    'PROJECT_NOT_EMPTY' => 'The selected document is not empty. Choose an empty document to create a project.',
    'PROJECT_ALREADY_MANAGED' => 'This document already has a project copy on this device. Open the project instead.',
    'EXTERNAL_PROJECT_CHANGED' =>
      'The external project changed outside OR. Its changes were preserved.',
    'PROJECT_SYNC_FAILED' => 'The project is saved on this device, but could not be synchronized to the selected document.',
    _ => 'The selected project or export location is unavailable.',
  });
}

ProjectFilePicker createDefaultProjectFilePicker() => Platform.isAndroid
    ? AndroidSafProjectPicker()
    : const FileSelectorProjectPicker();
