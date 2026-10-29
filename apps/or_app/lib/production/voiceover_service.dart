import 'dart:io';

import 'package:path_provider/path_provider.dart';

import '../project/project_gateway.dart';

/// One installed macOS voice as `say -v ?` reports it.
class VoiceOption {
  const VoiceOption({required this.name, required this.locale});

  final String name;
  final String locale;

  /// The display name without the locale explanation `say` appends, so
  /// "Linh (Vietnamese (Vietnam))" becomes "Linh".
  String get displayName {
    final match = RegExp(r'^(.*?)\s*\(.*\)$').firstMatch(name);
    final short = match?.group(1)?.trim();
    return short == null || short.isEmpty ? name : short;
  }

  String get label => '$displayName ($locale)';
}

/// One synthesised narration line and its exact timeline span.
class VoiceoverLine {
  const VoiceoverLine({
    required this.index,
    required this.text,
    required this.caption,
    required this.path,
    required this.startMs,
    required this.endMs,
  });

  final int index;
  final String text;
  final String caption;
  final String path;
  final int startMs;
  final int endMs;

  int get durationMs => endMs - startMs;
}

/// The project changes a voiceover run produced.
class VoiceoverApplied {
  const VoiceoverApplied({
    required this.trackId,
    required this.captionCount,
    required this.lines,
  });

  final String trackId;
  final int captionCount;
  final List<VoiceoverLine> lines;
}

class VoiceoverException implements Exception {
  const VoiceoverException(this.message);

  final String message;

  @override
  String toString() => message;
}

/// Offline narration through the macOS `say` voice.
///
/// The app is unsandboxed (ADR 0009), so spawning the system voice is the same
/// local provider boundary the assistant uses. Nothing leaves the machine.
class VoiceoverService {
  VoiceoverService({this.sayExecutable = 'say'});

  final String sayExecutable;

  /// Lists the installed voices. A missing `say` is reported as a setup error,
  /// never as an empty picker.
  Future<List<VoiceOption>> listVoices() async {
    final ProcessResult result;
    try {
      result = await Process.run(sayExecutable, ['-v', '?']);
    } on ProcessException catch (error) {
      throw VoiceoverException(
        'The macOS say command is unavailable (${error.message}).',
      );
    }
    if (result.exitCode != 0) {
      throw const VoiceoverException(
        'The macOS say command could not list its voices.',
      );
    }
    final voices = <VoiceOption>[];
    final seen = <String>{};
    final pattern = RegExp(r'^(.+?)\s+([a-z]{2,3}_[A-Z]{2})(?:\s+#.*)?$');
    for (final raw in result.stdout.toString().split('\n')) {
      final line = raw.trimRight();
      if (line.isEmpty) continue;
      final match = pattern.firstMatch(line);
      if (match == null) continue;
      final name = match.group(1)!.trim();
      final locale = match.group(2)!;
      // macOS can list the same installed voice twice (system and download).
      if (!seen.add('$name\u0000$locale')) continue;
      voices.add(VoiceOption(name: name, locale: locale));
    }
    return voices;
  }

  /// The app's durable folder for generated narration. Project media references
  /// these files, so this must not be a cache the system may delete.
  Future<Directory> outputDirectory() async {
    final support = await getApplicationSupportDirectory();
    final stamp = DateTime.now().millisecondsSinceEpoch;
    return Directory('${support.path}/voiceover/$stamp');
  }

  /// Speaks one line per script line and returns their exact spans.
  ///
  /// Lines are separated by the gap, so the manifest and the caption cues share
  /// one time base. The caller decides whether to apply them to the project.
  Future<List<VoiceoverLine>> synthesize({
    required List<String> lines,
    required String voice,
    required int rate,
    required int gapMs,
    required Directory directory,
    void Function(int completed, int total)? onProgress,
  }) async {
    if (lines.isEmpty) {
      throw const VoiceoverException('Write at least one narration line.');
    }
    if (lines.length > 300) {
      throw const VoiceoverException('A script may contain at most 300 lines.');
    }
    if (rate < 90 || rate > 400) {
      throw const VoiceoverException('The speaking rate must be 90-400 wpm.');
    }
    if (gapMs < 0 || gapMs > 10000) {
      throw const VoiceoverException('The gap must be 0-10000 ms.');
    }
    await directory.create(recursive: true);
    final results = <VoiceoverLine>[];
    var cursor = 0;
    for (var index = 0; index < lines.length; index += 1) {
      final text = lines[index].trim();
      if (text.isEmpty) continue;
      final file = File('${directory.path}/line-${index + 1}.wav');
      final arguments = <String>[
        if (voice != 'default') ...['-v', voice],
        '-r',
        '$rate',
        '-o',
        file.path,
        '--data-format=LEI16@44100',
        text.startsWith('-') ? ' $text' : text,
      ];
      final ProcessResult result;
      try {
        result = await Process.run(sayExecutable, arguments);
      } on ProcessException catch (error) {
        throw VoiceoverException(
          'The macOS say command is unavailable (${error.message}).',
        );
      }
      if (result.exitCode != 0) {
        final detail = result.stderr.toString().trim();
        throw VoiceoverException(
          'say failed for line ${index + 1}: ${detail.isEmpty ? 'exit ${result.exitCode}' : detail}',
        );
      }
      final durationMs = await wavDurationMs(file);
      final startMs = cursor;
      final endMs = startMs + durationMs;
      results.add(
        VoiceoverLine(
          index: index + 1,
          text: text,
          caption: text,
          path: file.path,
          startMs: startMs,
          endMs: endMs,
        ),
      );
      cursor = endMs + gapMs;
      onProgress?.call(index + 1, lines.length);
    }
    if (results.isEmpty) {
      throw const VoiceoverException('Write at least one narration line.');
    }
    return results;
  }

  /// Speaks one short sample with [voice] at [rate] and plays it through the
  /// system output. The returned file lets a test inspect the WAV without audio
  /// hardware; playback is best-effort and never blocks the picker.
  Future<File> preview({
    required String voice,
    required String text,
    required Directory directory,
    int rate = 180,
  }) async {
    if (text.trim().isEmpty) {
      throw const VoiceoverException('Nothing to preview.');
    }
    await directory.create(recursive: true);
    final file = File('${directory.path}/voice-preview.wav');
    final arguments = voiceoverPreviewArguments(
      voice: voice,
      rate: rate,
      text: text,
      outputPath: file.path,
    );
    final ProcessResult result;
    try {
      result = await Process.run(sayExecutable, arguments);
    } on ProcessException catch (error) {
      throw VoiceoverException(
        'The macOS say command is unavailable (${error.message}).',
      );
    }
    if (result.exitCode != 0) {
      final detail = result.stderr.toString().trim();
      throw VoiceoverException(
        'say could not speak the preview: ${detail.isEmpty ? 'exit ${result.exitCode}' : detail}',
      );
    }
    // `afplay` is the system player; a missing player is not a synthesis
    // failure, so the preview file still returns.
    try {
      await Process.run('afplay', [file.path]);
    } on ProcessException {
      // The sample was written; the caller can still inspect or play it.
    }
    return file;
  }

  /// Reads the exact duration of the PCM WAV that `say` wrote.
  Future<int> wavDurationMs(File file) async {
    final bytes = await file.readAsBytes();
    return wavDurationMsFromBytes(bytes);
  }
}

/// Parses a canonical 16-bit PCM WAV's data length. Kept pure so a widget test
/// can check the timing without invoking the system voice.
int wavDurationMsFromBytes(List<int> bytes) {
  if (bytes.length < 44 ||
      String.fromCharCodes(bytes.sublist(0, 4)) != 'RIFF' ||
      String.fromCharCodes(bytes.sublist(8, 12)) != 'WAVE') {
    throw const VoiceoverException('say did not write a RIFF/WAVE file.');
  }
  var offset = 12;
  int? channels;
  int? sampleRate;
  int? bits;
  int? dataBytes;
  while (offset + 8 <= bytes.length) {
    final id = String.fromCharCodes(bytes.sublist(offset, offset + 4));
    final size =
        bytes[offset + 4] |
        (bytes[offset + 5] << 8) |
        (bytes[offset + 6] << 16) |
        (bytes[offset + 7] << 24);
    final body = offset + 8;
    if (id == 'fmt ' && size >= 16) {
      channels = bytes[body + 2] | (bytes[body + 3] << 8);
      sampleRate =
          bytes[body + 4] |
          (bytes[body + 5] << 8) |
          (bytes[body + 6] << 16) |
          (bytes[body + 7] << 24);
      bits = bytes[body + 14] | (bytes[body + 15] << 8);
    } else if (id == 'data') {
      dataBytes = size < bytes.length - body ? size : bytes.length - body;
      break;
    }
    offset = body + size + (size & 1);
  }
  if (channels == null || sampleRate == null || bits == null || dataBytes == null) {
    throw const VoiceoverException('The generated WAV has no PCM format.');
  }
  if (bits != 16 || channels < 1 || sampleRate < 1) {
    throw const VoiceoverException('The generated WAV is not 16-bit PCM.');
  }
  final frameBytes = channels * 2;
  final frames = dataBytes ~/ frameBytes;
  return (frames * 1000) ~/ sampleRate;
}

/// Applies synthesised narration to the open project.
///
/// Every step is an ordinary validated command: import the WAV, append one
/// audio track, insert one clip per line, then import the reviewed caption
/// cues. A failure leaves the project changed up to that step and undoable.
class VoiceoverApplier {
  const VoiceoverApplier(this.gateway);

  final ProjectGateway gateway;

  Future<VoiceoverApplied> apply({
    required ProjectSessionHandle session,
    required ProjectReadModel project,
    required List<VoiceoverLine> lines,
    bool includeCaptions = true,
    void Function(String stage, int completed, int total)? onProgress,
  }) async {
    var current = project;
    final trackResult = await gateway.addTimelineTrack(
      session,
      current,
      ProjectTimelineTrackKind.audio,
    );
    current = await _advance(session, current, trackResult, 'add an audio track');
    final tracks = await gateway.listTimelineTracks(session);
    final audioTracks = tracks.items
        .where((track) => track.kind == ProjectTimelineTrackKind.audio)
        .toList(growable: false);
    if (audioTracks.isEmpty) {
      throw const VoiceoverException('The new audio track was not found.');
    }
    final trackId = audioTracks.last.trackId;

    for (var index = 0; index < lines.length; index += 1) {
      final line = lines[index];
      final imported = await gateway.importMedia(session, current, line.path);
      current = await _advance(
        session,
        current,
        imported,
        'import narration line ${line.index}',
      );
      final media = await _findImportedMedia(session, line.path);
      if (media == null) {
        throw VoiceoverException(
          'Narration line ${line.index} was imported but not found in the library.',
        );
      }
      final duration = _mediaDurationMs(media, line.durationMs);
      final inserted = await gateway.insertTimelineClip(
        session,
        current,
        trackId: trackId,
        mediaId: media.mediaId,
        timelineStart: ProjectRationalTime(BigInt.from(line.startMs), 1000),
        sourceStart: ProjectRationalTime(BigInt.zero, 1),
        duration: ProjectRationalTime(BigInt.from(duration), 1000),
      );
      current = await _advance(
        session,
        current,
        inserted,
        'place narration line ${line.index}',
      );
      onProgress?.call('Placing narration', index + 1, lines.length);
    }

    if (!includeCaptions) {
      return VoiceoverApplied(trackId: trackId, captionCount: 0, lines: lines);
    }
    final cues = lines
        .map(
          (line) => ProjectCaptionCue(
            startMs: BigInt.from(line.startMs),
            endMs: BigInt.from(line.endMs),
            text: line.caption,
          ),
        )
        .toList(growable: false);
    final captions = await gateway.importReviewedTimelineCaptions(
      session,
      current,
      cues,
    );
    if (!captions.succeeded) {
      throw VoiceoverException(captions.message);
    }
    return VoiceoverApplied(
      trackId: trackId,
      captionCount: cues.length,
      lines: lines,
    );
  }

  Future<ProjectReadModel> _advance(
    ProjectSessionHandle session,
    ProjectReadModel current,
    ProjectActionResult result,
    String action,
  ) async {
    if (!result.succeeded) {
      throw VoiceoverException(result.message.isEmpty ? 'Could not $action.' : result.message);
    }
    return result.view ?? await gateway.summary(session);
  }

  Future<ProjectMediaItem?> _findImportedMedia(
    ProjectSessionHandle session,
    String path,
  ) async {
    var page = await gateway.listMediaPage(session, offset: 0, limit: 100);
    if (page.totalCount > 100) {
      page = await gateway.listMediaPage(
        session,
        offset: page.totalCount - 100,
        limit: 100,
      );
    }
    final expected = Uri.file(path).toString();
    final name = path.split('/').last;
    for (final item in page.items.reversed) {
      if (item.sourceUri == expected ||
          item.sourceUri.endsWith(name) ||
          (Uri.tryParse(item.sourceUri)?.toFilePath() == path)) {
        return item;
      }
    }
    return null;
  }

  int _mediaDurationMs(ProjectMediaItem media, int measuredMs) {
    final duration = media.firstAudioDuration ?? media.containerDuration;
    if (duration == null || duration.denominator == 0) return measuredMs;
    final mediaMs =
        (duration.numerator * BigInt.from(1000)) ~/ BigInt.from(duration.denominator);
    if (mediaMs <= BigInt.zero) return measuredMs;
    return mediaMs.toInt() < measuredMs ? mediaMs.toInt() : measuredMs;
  }
}

/// The `say` arguments for a one-shot preview at [rate], so the spoken sample
/// matches the configured rate and stays short.
List<String> voiceoverPreviewArguments({
  required String voice,
  required int rate,
  required String text,
  required String outputPath,
}) {
  final sample = text.trim();
  return [
    if (voice != 'default') ...['-v', voice],
    '-r',
    '${rate.clamp(90, 400)}',
    '-o',
    outputPath,
    '--data-format=LEI16@44100',
    sample.length > 240 ? sample.substring(0, 240) : sample,
  ];
}
