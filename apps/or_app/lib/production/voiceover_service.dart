import 'dart:async';
import 'dart:io';

import 'package:path_provider/path_provider.dart';

import '../project/project_gateway.dart';
import '../support/host_platform.dart';

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
  VoiceoverService({
    // An app launched from the Finder or a test runner has a minimal PATH, so
    // the system binaries are addressed absolutely instead of through `say`.
    this.sayExecutable = '/usr/bin/say',
    this.processTimeout = const Duration(minutes: 2),
    HostPlatform? platform,
  }) : _host = platform ?? HostPlatform.current();

  final String sayExecutable;

  /// The host whose system voice this instance speaks through. macOS has
  /// `say`, Windows has SpeechSynthesis in .NET, and the freedesktop hosts
  /// have `espeak-ng`; a caller should never have to know that.
  final HostPlatform _host;

  /// A wedged audio device or a hung `say` must surface as an error instead of
  /// a spinner that never stops.
  final Duration processTimeout;

  /// Lists the installed voices. A missing `say` is reported as a setup error,
  /// never as an empty picker.
  Future<List<VoiceOption>> listVoices() async {
    if (_host.isWindows) return _listWindowsVoices();
    if (!_host.isMacOS) return _listEspeakVoices();
    final ProcessResult result;
    try {
      result = await Process.run(sayExecutable, [
        '-v',
        '?',
      ]).timeout(processTimeout);
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
    if (voices.isEmpty) {
      // An empty catalog is a setup problem, not a valid answer: keep what say
      // actually printed so the environment can be inspected afterwards.
      final rows = result.stdout.toString().split('\n').length;
      throw VoiceoverException(
        'No installed voice could be read (say printed $rows rows; see '
        'or-say-voices.log).',
      );
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
      await _speakToWav(
        text: text,
        voice: voice,
        rate: rate,
        file: file,
        context: 'line ${index + 1}',
      );
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
    await _speakToWav(
      text: text,
      voice: voice,
      rate: rate,
      file: file,
      context: 'the preview',
    );
    // Playback is best-effort: a host with no player still returns the sample,
    // which the caller can inspect or play itself.
    await _playAudio(file);
    return file;
  }

  /// Writes [text] to [file] as a 16-bit PCM WAV with the host's own voice.
  ///
  /// One call, three programs: `say` on macOS, SpeechSynthesis inside
  /// PowerShell on Windows, `espeak-ng` on the freedesktop hosts. The file
  /// format is the same so the timeline, the captions and the WAV reader do not
  /// branch per platform.
  Future<void> _speakToWav({
    required String text,
    required String voice,
    required int rate,
    required File file,
    required String context,
  }) async {
    final boundedRate = rate.clamp(90, 400);
    if (_host.isMacOS) {
      await _runVoice(
        sayExecutable,
        [
          if (voice != 'default') ...['-v', voice],
          '-r',
          '$boundedRate',
          '-o',
          file.path,
          '--data-format=LEI16@44100',
          text.startsWith('-') ? ' $text' : text,
        ],
        context,
        'say',
      );
      return;
    }
    if (_host.isWindows) {
      final textFile = File(
        '${file.parent.path}/voice-text-${DateTime.now().microsecondsSinceEpoch}.txt',
      );
      await textFile.writeAsString(text);
      final script =
          "Add-Type -AssemblyName System.Speech\n"
          "\$synth = New-Object System.Speech.Synthesis.SpeechSynthesizer\n"
          "\$synth.Rate = ${_sapiRate(boundedRate)}\n"
          "${voice == 'default' ? '' : "\$synth.SelectVoice('${_powershellQuote(voice)}')\n"}"
          "\$synth.SetOutputToWaveFile('${_powershellQuote(file.path)}')\n"
          "\$synth.Speak([IO.File]::ReadAllText('${_powershellQuote(textFile.path)}'))\n"
          "\$synth.Dispose()\n";
      try {
        await _runVoice(
          'powershell',
          ['-NoProfile', '-NonInteractive', '-Command', script],
          context,
          'Windows speech',
        );
      } finally {
        if (await textFile.exists()) await textFile.delete();
      }
      return;
    }
    final program = await _espeakProgram();
    await _runVoice(
      program,
      [
        '-v',
        voice == 'default' ? 'en' : voice,
        '-s',
        '$boundedRate',
        '-w',
        file.path,
        text,
      ],
      context,
      'espeak-ng',
    );
  }

  /// Plays [file] through the host's audio output when one is available.
  ///
  /// A missing player is not a synthesis failure: the caller keeps the sample.
  Future<void> _playAudio(File file) async {
    if (_host.isMacOS) {
      try {
        await Process.run('/usr/bin/afplay', [file.path]);
      } on ProcessException {
        // The sample was written; the caller can still inspect or play it.
      }
      return;
    }
    if (_host.isWindows) {
      final script =
          "(New-Object Media.SoundPlayer '${_powershellQuote(file.path)}').PlaySync()";
      try {
        await Process.run('powershell', [
          '-NoProfile',
          '-NonInteractive',
          '-Command',
          script,
        ]);
      } on ProcessException {
        // No player, no problem: the WAV is on disk.
      }
      return;
    }
    for (final candidate in const ['paplay', 'aplay', 'ffplay']) {
      final extra = candidate == 'ffplay'
          ? const ['-nodisp', '-autoexit']
          : const [];
      try {
        final result = await Process.run(candidate, [...extra, file.path]);
        if (result.exitCode == 0) return;
      } on ProcessException {
        continue;
      }
    }
  }

  Future<ProcessResult> _runVoice(
    String program,
    List<String> arguments,
    String context,
    String label,
  ) async {
    final ProcessResult result;
    try {
      result = await Process.run(program, arguments).timeout(processTimeout);
    } on TimeoutException {
      throw VoiceoverException('$label did not finish $context in time.');
    } on ProcessException catch (error) {
      throw VoiceoverException(_unavailableMessage(error, label));
    }
    if (result.exitCode != 0) {
      final detail = result.stderr.toString().trim();
      throw VoiceoverException(
        '$label failed for $context: '
        '${detail.isEmpty ? 'exit ${result.exitCode}' : detail}',
      );
    }
    return result;
  }

  String _unavailableMessage(ProcessException error, String label) {
    if (_host.isLinux) {
      return 'The system voice is unavailable (${error.message}). Install '
          'espeak-ng with your package manager, for example '
          '`sudo apt install espeak-ng`, or use a local voice server instead.';
    }
    return '$label is unavailable (${error.message}).';
  }

  /// The `espeak-ng` binary, or the older `espeak` when only that is installed.
  Future<String> _espeakProgram() async {
    for (final name in const ['espeak-ng', 'espeak']) {
      final found = _host.findExecutable(
        name,
        extraDirectories: const ['/usr/bin', '/usr/local/bin', '/bin'],
      );
      if (found != null) return found;
    }
    throw const VoiceoverException(
      'The system voice needs espeak-ng on Linux. Install it with your package '
      'manager, for example `sudo apt install espeak-ng`, or use a local voice '
      'server instead.',
    );
  }

  Future<List<VoiceOption>> _listWindowsVoices() async {
    const script =
        "Add-Type -AssemblyName System.Speech\n"
        "\$synth = New-Object System.Speech.Synthesis.SpeechSynthesizer\n"
        "\$synth.GetInstalledVoices() | ForEach-Object { \$info = \$_.VoiceInfo; "
        "\"\$(\$info.Name)|\$(\$info.Culture.Name)\" }\n";
    final result = await _runVoice(
      'powershell',
      ['-NoProfile', '-NonInteractive', '-Command', script],
      'the voice list',
      'Windows speech',
    );
    final voices = <VoiceOption>[];
    for (final raw in result.stdout.toString().split('\n')) {
      final line = raw.trim();
      if (line.isEmpty) continue;
      final parts = line.split('|');
      if (parts.length < 2) continue;
      voices.add(VoiceOption(name: parts[0], locale: parts[1]));
    }
    if (voices.isEmpty) {
      throw const VoiceoverException(
        'Windows reported no installed speech voices.',
      );
    }
    return voices;
  }

  Future<List<VoiceOption>> _listEspeakVoices() async {
    final program = await _espeakProgram();
    final result = await _runVoice(
      program,
      ['--voices'],
      'the voice list',
      'espeak-ng',
    );
    final voices = <VoiceOption>[];
    final seen = <String>{};
    for (final raw in result.stdout.toString().split('\n')) {
      final line = raw.trimRight();
      if (line.isEmpty) continue;
      final columns = line.trim().split(RegExp(r'\s+'));
      if (columns.length < 4 || columns.first == 'Pty') continue;
      final language = columns[1];
      final name = columns[3];
      if (!seen.add('$name\u0000$language')) continue;
      voices.add(VoiceOption(name: name, locale: language));
    }
    if (voices.isEmpty) {
      throw const VoiceoverException(
        'espeak-ng listed no voices; check its installation.',
      );
    }
    return voices;
  }

  /// SAPI's rate is -10..10, and the narration picker's is words per minute.
  int _sapiRate(int wordsPerMinute) =>
      ((wordsPerMinute - 175) / 22.5).round().clamp(-10, 10);

  String _powershellQuote(String value) => value.replaceAll("'", "''");

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
  if (channels == null ||
      sampleRate == null ||
      bits == null ||
      dataBytes == null) {
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
    current = await _advance(
      session,
      current,
      trackResult,
      'add an audio track',
    );
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
      throw VoiceoverException(
        result.message.isEmpty ? 'Could not $action.' : result.message,
      );
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
        (duration.numerator * BigInt.from(1000)) ~/
        BigInt.from(duration.denominator);
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
