import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/library/creative_direction_store.dart';
import 'package:or_app/library/creative_directions.dart';

void main() {
  late Directory directory;
  late CreativeDirectionStore store;

  setUp(() async {
    directory = await Directory.systemTemp.createTemp('or-directions-');
    store = CreativeDirectionStore.inDirectory(directory);
  });

  tearDown(() async {
    if (await directory.exists()) await directory.delete(recursive: true);
  });

  test('an empty store reads as no directions instead of failing', () async {
    expect(await store.load(), isEmpty);
  });

  test('a kept copy survives a reload and remembers it is mine', () async {
    final copy = bundledCreativeDirections.first.copyWith(
      id: 'mine-1',
      name: 'My documentary',
      isMine: true,
    );

    await store.upsert(copy);
    final reloaded = await CreativeDirectionStore.inDirectory(
      directory,
    ).load();

    expect(reloaded.single.name, 'My documentary');
    expect(reloaded.single.isMine, isTrue);
    expect(reloaded.single.audio, copy.audio);
  });

  test('upserting the same id replaces instead of duplicating', () async {
    final first = bundledCreativeDirections.first.copyWith(id: 'same');
    await store.upsert(first);
    await store.upsert(first.copyWith(name: 'Renamed'));

    final items = await store.load();

    expect(items.length, 1);
    expect(items.single.name, 'Renamed');
  });

  test('removing a copy leaves the rest alone', () async {
    await store.upsert(bundledCreativeDirections.first.copyWith(id: 'a'));
    await store.upsert(bundledCreativeDirections.last.copyWith(id: 'b'));

    final items = await store.remove('a');

    expect(items.map((item) => item.id), ['b']);
    expect((await store.load()).map((item) => item.id), ['b']);
  });

  test('a hand-edited file is ignored rather than crashing the library', () async {
    await File('${directory.path}/creative-directions.v1.json')
        .writeAsString('{not json');

    expect(await store.load(), isEmpty);
  });

  test('copy ids do not collide when a direction is kept twice', () {
    expect(copiedDirectionId('brief', ['brief']), 'brief-mine');
    expect(copiedDirectionId('brief', ['brief', 'brief-mine']), 'brief-mine-2');
    expect(
      copiedDirectionId('brief', ['brief', 'brief-mine', 'brief-mine-2']),
      'brief-mine-3',
    );
  });
}
