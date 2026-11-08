import 'dart:convert';
import 'dart:io';

import 'creative_directions.dart';

/// The person's own creative directions, kept beside the app's settings.
///
/// A direction is small text, so one JSON file is enough. A copy of a bundled
/// direction is written only when the person asks for it; the bundled list is
/// never written to.
class CreativeDirectionStore {
  CreativeDirectionStore(this._file);

  /// The store used by the app, under the application support directory.
  CreativeDirectionStore.inDirectory(Directory directory)
    : _file = File('${directory.path}/creative-directions.v1.json');

  final File _file;

  Future<List<CreativeDirection>> load() async {
    if (!await _file.exists()) return const [];
    try {
      final decoded = jsonDecode(await _file.readAsString());
      if (decoded is! Map<String, Object?>) return const [];
      final items = decoded['directions'];
      if (items is! List) return const [];
      return [
        for (final item in items)
          if (item is Map<String, Object?>) CreativeDirection.fromJson(item),
      ];
    } on FormatException {
      // A hand-edited or truncated file must not take the library down.
      return const [];
    } on FileSystemException {
      return const [];
    }
  }

  Future<void> save(List<CreativeDirection> directions) async {
    await _file.parent.create(recursive: true);
    await _file.writeAsString(
      const JsonEncoder.withIndent('  ').convert({
        'version': 1,
        'directions': [for (final direction in directions) direction.toJson()],
      }),
    );
  }

  /// Adds or replaces a direction by id.
  Future<List<CreativeDirection>> upsert(CreativeDirection direction) async {
    final items = [...await load()];
    final index = items.indexWhere((item) => item.id == direction.id);
    if (index == -1) {
      items.add(direction);
    } else {
      items[index] = direction;
    }
    await save(items);
    return items;
  }

  Future<List<CreativeDirection>> remove(String id) async {
    final items = [
      for (final item in await load())
        if (item.id != id) item,
    ];
    await save(items);
    return items;
  }
}

/// A stable id for a copy, so duplicating twice never overwrites the first.
String copiedDirectionId(String baseId, Iterable<String> existing) {
  var index = 1;
  var candidate = '$baseId-mine';
  while (existing.contains(candidate)) {
    index++;
    candidate = '$baseId-mine-$index';
  }
  return candidate;
}
