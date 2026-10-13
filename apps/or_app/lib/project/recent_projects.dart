import 'dart:convert';
import 'dart:io';

import 'package:path_provider/path_provider.dart';

const maxRecentProjects = 12;
const _maxRecentProjectsFileBytes = 128 * 1024;
const _recentProjectsFileName = 'recent-projects.v1.json';

class RecentProject {
  const RecentProject({
    required this.path,
    required this.name,
    required this.lastOpened,
    this.documentUri,
  });

  final String path;
  final String name;
  final DateTime lastOpened;

  /// Present on Android, where [path] is an app-private working copy.
  final String? documentUri;

  String get identity => documentUri ?? path;

  Map<String, Object?> toJson() => {
    'path': path,
    'name': name,
    'lastOpened': lastOpened.toUtc().toIso8601String(),
    if (documentUri != null) 'documentUri': documentUri,
  };

  static RecentProject? tryParse(Object? value) {
    if (value is! Map<String, Object?>) return null;
    final path = value['path'];
    final name = value['name'];
    final lastOpened = value['lastOpened'];
    final documentUri = value['documentUri'];
    if (path is! String ||
        path.length > 4096 ||
        !_isAbsoluteProjectPath(path)) {
      return null;
    }
    if (name is! String || name.trim().isEmpty || name.length > 256) {
      return null;
    }
    if (lastOpened is! String) return null;
    final openedAt = DateTime.tryParse(lastOpened);
    if (openedAt == null) return null;
    if (documentUri != null) {
      if (documentUri is! String || documentUri.length > 4096) return null;
      final uri = Uri.tryParse(documentUri);
      if (uri == null || uri.scheme != 'content' || uri.authority.isEmpty) {
        return null;
      }
    }
    return RecentProject(
      path: path,
      name: name.trim(),
      lastOpened: openedAt.toUtc(),
      documentUri: documentUri as String?,
    );
  }
}

bool _isAbsoluteProjectPath(String path) =>
    path.startsWith('/') ||
    RegExp(r'^[A-Za-z]:[\\/]').hasMatch(path) ||
    path.startsWith(r'\\');

abstract interface class RecentProjectStore {
  Future<List<RecentProject>> load();

  Future<List<RecentProject>> record({
    required String path,
    required String name,
    String? documentUri,
  });
}

/// Stores only project locations and display metadata, never project contents.
/// The canonical project and recovery files remain owned by the Rust core.
class PersistentRecentProjectStore implements RecentProjectStore {
  PersistentRecentProjectStore({
    Future<Directory> Function()? supportDirectory,
    DateTime Function()? now,
  }) : _supportDirectory = supportDirectory ?? getApplicationSupportDirectory,
       _now = now ?? DateTime.now;

  final Future<Directory> Function() _supportDirectory;
  final DateTime Function() _now;
  int _writeSequence = 0;

  Future<File> _file() async => File(
    '${(await _supportDirectory()).path}${Platform.pathSeparator}'
    '$_recentProjectsFileName',
  );

  @override
  Future<List<RecentProject>> load() async {
    final file = await _file();
    if (!await file.exists()) return const [];
    if (await file.length() > _maxRecentProjectsFileBytes) return const [];
    try {
      final decoded = jsonDecode(await file.readAsString());
      if (decoded is! Map<String, Object?> || decoded['version'] != 1) {
        return const [];
      }
      final projects = decoded['projects'];
      if (projects is! List<Object?>) return const [];
      final result = <RecentProject>[];
      final identities = <String>{};
      for (final value in projects) {
        final project = RecentProject.tryParse(value);
        if (project == null || !identities.add(project.identity)) continue;
        result.add(project);
        if (result.length == maxRecentProjects) break;
      }
      result.sort((left, right) => right.lastOpened.compareTo(left.lastOpened));
      return List.unmodifiable(result);
    } on FormatException {
      // This file is only a convenience index. Invalid data never opens a
      // project; a later successful open can rebuild the index.
      return const [];
    } on FileSystemException {
      return const [];
    }
  }

  @override
  Future<List<RecentProject>> record({
    required String path,
    required String name,
    String? documentUri,
  }) async {
    final normalizedName = name.trim();
    if (path.isEmpty ||
        path.length > 4096 ||
        normalizedName.isEmpty ||
        normalizedName.length > 256) {
      return load();
    }
    final entry = RecentProject(
      path: path,
      name: normalizedName,
      documentUri: documentUri,
      lastOpened: _now().toUtc(),
    );
    if (RecentProject.tryParse(entry.toJson()) == null) {
      throw ArgumentError('The recent project location is invalid.');
    }
    final projects = await load();
    final updated = <RecentProject>[
      entry,
      ...projects.where((project) => project.identity != entry.identity),
    ].take(maxRecentProjects).toList(growable: false);

    final file = await _file();
    await file.parent.create(recursive: true);
    final temporary = File(
      '${file.path}.tmp-${_now().microsecondsSinceEpoch}-${++_writeSequence}',
    );
    try {
      await temporary.writeAsString(
        jsonEncode({
          'version': 1,
          'projects': updated.map((project) => project.toJson()).toList(),
        }),
        flush: true,
      );
      await temporary.rename(file.path);
    } finally {
      if (await temporary.exists()) await temporary.delete();
    }
    return List.unmodifiable(updated);
  }
}

/// In-memory store for widget tests and callers that do not need persistence.
class InMemoryRecentProjectStore implements RecentProjectStore {
  InMemoryRecentProjectStore([Iterable<RecentProject> initial = const []])
    : _projects = List.of(initial);

  final List<RecentProject> _projects;

  @override
  Future<List<RecentProject>> load() async => List.unmodifiable(_projects);

  @override
  Future<List<RecentProject>> record({
    required String path,
    required String name,
    String? documentUri,
  }) async {
    final entry = RecentProject(
      path: path,
      name: name,
      documentUri: documentUri,
      lastOpened: DateTime.now().toUtc(),
    );
    _projects.removeWhere((project) => project.identity == entry.identity);
    _projects.insert(0, entry);
    if (_projects.length > maxRecentProjects) {
      _projects.removeRange(maxRecentProjects, _projects.length);
    }
    return List.unmodifiable(_projects);
  }
}
