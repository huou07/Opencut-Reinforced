import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/library/project_templates.dart';
import 'package:or_app/library/sound_library.dart';
import 'package:or_app/library/text_presets.dart';
import 'package:or_app/library/usage_guide.dart';

/// The catalogue is data, so it is checked as data: every entry must be usable
/// by the command path without a human noticing a typo at click time.
void main() {
  group('project templates', () {
    test('every template has a unique id and a usable canvas', () {
      final ids = <String>{};
      for (final template in projectTemplates) {
        expect(ids.add(template.id), isTrue, reason: 'duplicate ${template.id}');
        expect(template.name.trim(), isNotEmpty);
        expect(template.summary.trim(), isNotEmpty);
        expect(template.suggestedName.trim(), isNotEmpty);
        expect(template.canvas.width, greaterThan(0));
        expect(template.canvas.height, greaterThan(0));
        expect(template.canvas.width.isEven, isTrue);
        expect(template.canvas.height.isEven, isTrue);
        expect(template.frameRate.numerator, greaterThan(0));
        expect(template.frameRate.denominator, greaterThan(0));
        expect(template.nextSteps, isNotEmpty);
      }
    });

    test('every starter overlay resolves to a text preset', () {
      for (final template in projectTemplates) {
        for (final overlay in template.overlays) {
          expect(
            textPresetById(overlay.presetId),
            isNotNull,
            reason: '${template.id} names ${overlay.presetId}',
          );
          expect(overlay.text.trim(), isNotEmpty);
          expect(overlay.durationMilliseconds, greaterThan(0));
          expect(overlay.startMilliseconds, greaterThanOrEqualTo(0));
          expect(overlay.start.denominator, 1000);
        }
      }
    });

    test('lookup by id round-trips and an unknown id is null', () {
      for (final template in projectTemplates) {
        expect(projectTemplateById(template.id)?.id, template.id);
        expect(template.canvasLabel, contains('×'));
        expect(template.frameRateLabel, contains('fps'));
      }
      expect(projectTemplateById('no-such-template'), isNull);
    });
  });

  group('text presets', () {
    test('has unique ids and legal formatting', () {
      final ids = <String>{};
      for (final preset in textPresets) {
        expect(ids.add(preset.id), isTrue, reason: 'duplicate ${preset.id}');
        expect(preset.sample.trim(), isNotEmpty);
        expect(preset.useCase.trim(), isNotEmpty);
        final formatting = preset.formatting;
        expect(formatting.sizeMilliPoints, greaterThanOrEqualTo(8000));
        expect(formatting.sizeMilliPoints, lessThanOrEqualTo(400000));
        expect(formatting.opacityBasisPoints, inInclusiveRange(0, 10000));
      }
    });

    test('lookup by id round-trips and an unknown id is null', () {
      for (final preset in textPresets) {
        expect(textPresetById(preset.id)?.id, preset.id);
      }
      expect(textPresetById('no-such-preset'), isNull);
    });
  });

  group('sound effects', () {
    test('has unique ids, names and file names', () {
      final ids = <String>{};
      final files = <String>{};
      for (final effect in soundEffects) {
        expect(ids.add(effect.id), isTrue, reason: 'duplicate ${effect.id}');
        expect(files.add(effect.fileName), isTrue);
        expect(effect.fileName, endsWith('.wav'));
        expect(effect.durationMilliseconds, inInclusiveRange(20, 10000));
        expect(effect.description.trim(), isNotEmpty);
      }
    });

    test('renders a real 16-bit mono PCM WAV at the requested rate', () {
      for (final effect in soundEffects) {
        final bytes = renderSoundWav(effect);
        final data = ByteData.sublistView(bytes);
        expect(String.fromCharCodes(bytes.sublist(0, 4)), 'RIFF');
        expect(String.fromCharCodes(bytes.sublist(8, 12)), 'WAVE');
        expect(String.fromCharCodes(bytes.sublist(12, 16)), 'fmt ');
        expect(data.getUint16(20, Endian.little), 1, reason: 'PCM');
        expect(data.getUint16(22, Endian.little), 1, reason: 'mono');
        expect(data.getUint32(24, Endian.little), 48000);
        expect(data.getUint16(34, Endian.little), 16, reason: 'bit depth');
        expect(String.fromCharCodes(bytes.sublist(36, 40)), 'data');
        final expectedSamples = (48000 * effect.durationMilliseconds / 1000)
            .round();
        expect(data.getUint32(40, Endian.little), expectedSamples * 2);
        expect(bytes.length, 44 + expectedSamples * 2);
        expect(data.getUint32(4, Endian.little), bytes.length - 8);
      }
    });

    test('is deterministic and audible but never clips', () {
      for (final effect in soundEffects) {
        final first = renderSoundWav(effect);
        final second = renderSoundWav(effect);
        expect(first, second, reason: '${effect.name} must be repeatable');
        final data = ByteData.sublistView(first);
        var peak = 0;
        for (var offset = 44; offset + 1 < first.length; offset += 2) {
          final sample = data.getInt16(offset, Endian.little).abs();
          if (sample > peak) peak = sample;
        }
        expect(peak, greaterThan(1000), reason: '${effect.name} is silent');
        expect(peak, lessThan(32767), reason: '${effect.name} clips');
      }
    });

    test('lookup by id round-trips and an unknown id is null', () {
      for (final effect in soundEffects) {
        expect(soundEffectById(effect.id)?.id, effect.id);
      }
      expect(soundEffectById('no-such-sound'), isNull);
    });
  });

  group('usage guide', () {
    test('has unique sections and every section explains something', () {
      final ids = <String>{};
      for (final section in usageGuide) {
        expect(ids.add(section.id), isTrue, reason: 'duplicate ${section.id}');
        expect(section.title.trim(), isNotEmpty);
        expect(section.summary.trim(), isNotEmpty);
        expect(section.steps, isNotEmpty);
        for (final step in section.steps) {
          expect(step.title.trim(), isNotEmpty);
          expect(step.detail.trim(), isNotEmpty);
        }
      }
      expect(usageGuide.length, greaterThanOrEqualTo(8));
    });

    test('shortcut rows do not hard-code one desktop modifier', () {
      for (final section in usageGuide) {
        for (final shortcut in section.shortcuts) {
          expect(shortcut.keys.trim(), isNotEmpty);
          expect(shortcut.action.trim(), isNotEmpty);
          expect(shortcut.keys, isNot(startsWith('Cmd+')));
          expect(shortcut.keys, isNot(startsWith('Ctrl+')));
        }
      }
    });

    test('lookup by id round-trips and an unknown id is null', () {
      for (final section in usageGuide) {
        expect(guideSectionById(section.id)?.id, section.id);
      }
      expect(guideSectionById('no-such-section'), isNull);
    });
  });
}
