import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/library/creative_directions.dart';
import 'package:or_app/library/edit_styles.dart';

EditStyle _styleOf(String id) =>
    bundledEditStyles.firstWhere((style) => style.id == id);

EditStyleSource _source(String id, double seconds, {bool video = true}) =>
    EditStyleSource(
      mediaId: id,
      frames: (seconds * 30).round(),
      hasVideo: video,
      label: '$id.mp4',
    );

void main() {
  group('edit style plan', () {
    test('a documentary opens on its longest material', () {
      final plan = planEditStyleCut(
        style: _styleOf('cinematic-documentary'),
        sources: [_source('a', 4), _source('b', 20), _source('c', 9)],
        frameRate: 30,
      );

      expect(plan.shots.first.mediaId, 'b');
      expect(plan.shots.first.why, contains('Opens on b.mp4'));
      expect(plan.shots.first.frameCount, 8 * 30);
      expect(plan.shots[1].mediaId, 'c');
      expect(plan.shots[1].frameCount, 6 * 30);
      expect(plan.summary, '3 shots · 18.0 s');
    });

    test('a fast style opens on its shortest usable material', () {
      final plan = planEditStyleCut(
        style: _styleOf('fast-paced-educational'),
        sources: [_source('a', 9), _source('b', 2), _source('c', 5)],
        frameRate: 30,
      );

      expect(plan.shots.first.mediaId, 'b');
      expect(plan.shots.first.frameCount, (1.5 * 30).round());
      expect(plan.shots[1].mediaId, 'c');
      expect(plan.shots[1].frameCount, (2.5 * 30).round());
    });

    test('a vlog keeps the order the person imported', () {
      final plan = planEditStyleCut(
        style: _styleOf('personal-vlog'),
        sources: [_source('a', 9), _source('b', 2), _source('c', 5)],
        frameRate: 30,
      );

      expect(plan.shots.map((shot) => shot.mediaId), ['a', 'b', 'c']);
    });

    test('shots follow each other without a gap or an overlap', () {
      final plan = planEditStyleCut(
        style: _styleOf('product-showcase'),
        sources: [
          for (var index = 0; index < 6; index++) _source('clip$index', 10),
        ],
        frameRate: 30,
      );

      var expected = 0;
      for (final shot in plan.shots) {
        expect(shot.startFrame, expected);
        expected += shot.frameCount;
      }
      expect(plan.totalFrames, expected);
    });

    test('a style cuts no more shots than it wants and says so', () {
      final plan = planEditStyleCut(
        style: _styleOf('product-showcase'),
        sources: [
          for (var index = 0; index < 12; index++) _source('clip$index', 10),
        ],
        frameRate: 30,
      );

      expect(plan.shots.length, _styleOf('product-showcase').maxShots);
      expect(plan.notes.join(' '), contains('Used 8 of 12 clips'));
    });

    test('a clip too short to see is skipped instead of cut', () {
      final plan = planEditStyleCut(
        style: _styleOf('fast-paced-educational'),
        sources: [_source('blink', 0.2), _source('audio-only', 10, video: false), _source('real', 6)],
        frameRate: 30,
      );

      expect(plan.shots.map((shot) => shot.mediaId), ['real']);
    });

    test('importing nothing usable explains what to do', () {
      final plan = planEditStyleCut(
        style: _styleOf('cinematic-documentary'),
        sources: const [],
        frameRate: 30,
      );

      expect(plan.isEmpty, isTrue);
      expect(plan.notes.single, contains('import a clip first'));
    });

    test('a frame rate of zero falls back to thirty instead of dividing by it', () {
      final plan = planEditStyleCut(
        style: _styleOf('personal-vlog'),
        sources: [_source('a', 6)],
        frameRate: 0,
      );

      expect(plan.frameRate, 30);
      expect(plan.shots.single.frameCount, 3 * 30);
    });

    test('the title clip carries the project name when there is one', () {
      final style = _styleOf('interview-storytelling');

      expect(editStyleTitle(style, projectName: '  Anne  '), 'Anne');
      expect(editStyleTitle(style, projectName: '   '), style.titleText);
    });
  });

  group('creative directions', () {
    test('every direction answers all seven brief questions', () {
      for (final direction in bundledCreativeDirections) {
        expect(direction.summary, isNotEmpty, reason: direction.id);
        expect(direction.bestFor.length, greaterThanOrEqualTo(3));
        expect(direction.voice, isNotEmpty, reason: direction.id);
        expect(direction.script, isNotEmpty, reason: direction.id);
        expect(direction.storytelling, isNotEmpty, reason: direction.id);
        expect(direction.graphics, isNotEmpty, reason: direction.id);
        expect(direction.captions, isNotEmpty, reason: direction.id);
        expect(direction.editing, isNotEmpty, reason: direction.id);
        expect(direction.audio, isNotEmpty, reason: direction.id);
      }
      expect(
        bundledCreativeDirections.map((direction) => direction.id).toSet().length,
        bundledCreativeDirections.length,
      );
    });

    test('a brief reads in the order a video is decided', () {
      final brief = bundledCreativeDirections.first.toBrief();
      final voice = brief.indexOf('Voice and narration');
      final script = brief.indexOf('Script shape');
      final audio = brief.indexOf('Audio\n');

      expect(voice, greaterThan(0));
      expect(script, greaterThan(voice));
      expect(audio, greaterThan(script));
    });

    test('a saved copy round-trips through JSON and is marked mine', () {
      final copy = bundledCreativeDirections.last.copyWith(
        id: 'mine',
        name: 'My direction',
        isMine: true,
      );

      final restored = CreativeDirection.fromJson(copy.toJson());

      expect(restored.name, 'My direction');
      expect(restored.isMine, isTrue);
      expect(restored.editing, copy.editing);
      expect(restored.bestFor, copy.bestFor);
    });
  });
}
