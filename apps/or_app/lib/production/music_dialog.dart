import 'package:flutter/material.dart';

import '../design/or_colors.dart';
import '../design/or_typography.dart';
import '../project/project_gateway.dart';
import 'ducking_service.dart';
import 'music_prompt.dart';
import 'music_service.dart';

class MusicApplied {
  const MusicApplied({
    required this.trackId,
    required this.clipId,
    required this.ducked,
    required this.undoSteps,
    required this.bed,
  });

  final String trackId;
  final String clipId;
  final bool ducked;
  final int undoSteps;
  final MusicBed bed;
}

/// Generates a background bed and places it on its own audio track.
///
/// When the user selects a voice track the dialog also ducks the music under
/// every overlapping voice clip, using ordinary split and gain commands.
class MusicDialog extends StatefulWidget {
  const MusicDialog({
    super.key,
    required this.gateway,
    required this.session,
    required this.project,
  });

  final ProjectGateway gateway;
  final ProjectSessionHandle session;
  final ProjectReadModel project;

  @override
  State<MusicDialog> createState() => _MusicDialogState();
}

class _MusicDialogState extends State<MusicDialog> {
  final MusicService _music = MusicService();
  final TextEditingController _promptField = TextEditingController();

  String _moodId = musicMoods.first.id;
  double _tempo = 72;
  double _seconds = 60;
  String? _voiceTrackId;
  bool _duck = true;
  List<ProjectTimelineTrack> _audioTracks = const [];
  bool _busy = false;
  String _stage = '';
  double? _progress;
  String? _error;
  MusicPlan? _plan;
  int _variation = 0;

  /// Reads the prompt and makes the visible controls agree with it, so the user
  /// always sees the exact bed that "Generate and add" will write.
  void _interpretPrompt() {
    final text = _promptField.text.trim();
    if (text.isEmpty) {
      if (_plan != null && mounted) setState(() => _plan = null);
      return;
    }
    final plan = MusicPromptInterpreter.interpret(
      text,
      fallbackTempo: _tempo.round(),
      fallbackSeconds: _seconds.round(),
      variation: _variation,
      fallbackMoodId: _moodId,
    );
    if (!mounted) return;
    setState(() {
      _plan = plan;
      _moodId = plan.moodId;
      _tempo = plan.tempo.toDouble();
      _seconds = plan.seconds.toDouble();
    });
  }

  /// The controls stay authoritative; the prompt contributes the mix, octave
  /// and seed. Moving a slider after a prompt must not snap back.
  MusicPlan? _effectivePlan() {
    final base = _plan;
    if (base == null) return null;
    return MusicPlan(
      moodId: _moodId,
      tempo: _tempo.round(),
      seconds: _seconds.round(),
      drumsScale: base.drumsScale,
      padScale: base.padScale,
      arpScale: base.arpScale,
      brightnessScale: base.brightnessScale,
      octaveShift: base.octaveShift,
      seed: base.seed,
      summary: base.summary,
      notes: base.notes,
    );
  }

  String get _planReadback {
    final mood = musicMoods.firstWhere(
      (candidate) => candidate.id == _moodId,
      orElse: () => musicMoods.first,
    );
    final plan = _plan;
    final drums = (plan?.drumsScale ?? 1) <= 0
        ? 'no drums'
        : (plan?.drumsScale ?? 1) > 1
        ? 'strong drums'
        : 'light drums';
    return '${mood.label.toLowerCase()} · ${_tempo.round()} bpm · '
        '${_seconds.round()} s · $drums';
  }

  void _useVariation() {
    _variation = nextMusicVariation(_variation);
    _interpretPrompt();
  }

  @override
  void initState() {
    super.initState();
    _loadTracks();
    _promptField.addListener(_interpretPrompt);
  }

  @override
  void dispose() {
    _promptField.dispose();
    super.dispose();
  }

  Future<void> _loadTracks() async {
    try {
      final tracks = await widget.gateway.listTimelineTracks(widget.session);
      final audio = tracks.items
          .where(
            (track) =>
                track.kind == ProjectTimelineTrackKind.audio &&
                track.clipCount > 0 &&
                !track.state.locked,
          )
          .toList(growable: false);
      if (!mounted) return;
      setState(() {
        _audioTracks = audio;
        _voiceTrackId = audio.length == 1 ? audio.single.trackId : null;
      });
    } catch (_) {
      // No tracks is a valid state; the music still applies without ducking.
    }
  }

  Future<void> _generate() async {
    setState(() {
      _busy = true;
      _error = null;
      _stage = 'Generating the music bed…';
      _progress = null;
    });
    try {
      final directory = await _music.outputDirectory();
      final plan = _effectivePlan();
      final bed = await _music.generate(
        moodId: _moodId,
        tempo: _tempo.round(),
        seconds: _seconds.round(),
        directory: directory,
        seed: plan?.seed ?? 1,
        plan: plan,
      );
      if (!mounted) return;
      setState(() => _stage = 'Importing and placing the bed…');
      final applied = await _apply(bed);
      if (!mounted) return;
      Navigator.of(context).pop(applied);
    } on MusicException catch (error) {
      if (mounted) setState(() => _error = error.message);
    } on DuckingException catch (error) {
      if (mounted) setState(() => _error = error.message);
    } on ProjectGatewayException catch (error) {
      if (mounted) setState(() => _error = error.message);
    } catch (error) {
      if (mounted) {
        setState(() => _error = 'The music bed could not be added: $error');
      }
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<MusicApplied> _apply(MusicBed bed) async {
    var current = widget.project;
    final imported = await widget.gateway.importMedia(
      widget.session,
      current,
      bed.path,
    );
    current = await _advance(current, imported, 'import the music bed');
    final media = await _findImportedMedia(bed.path);
    if (media == null) {
      throw const MusicException('The music bed was imported but not found.');
    }
    final trackResult = await widget.gateway.addTimelineTrack(
      widget.session,
      current,
      ProjectTimelineTrackKind.audio,
    );
    current = await _advance(current, trackResult, 'add a music track');
    final tracks = await widget.gateway.listTimelineTracks(widget.session);
    final audioTracks = tracks.items
        .where((track) => track.kind == ProjectTimelineTrackKind.audio)
        .toList(growable: false);
    if (audioTracks.isEmpty) {
      throw const MusicException('The new music track was not found.');
    }
    final trackId = audioTracks.last.trackId;
    final durationMs = _mediaDurationMs(media, bed.durationMs);
    final inserted = await widget.gateway.insertTimelineClip(
      widget.session,
      current,
      trackId: trackId,
      mediaId: media.mediaId,
      timelineStart: ProjectRationalTime(BigInt.zero, 1),
      sourceStart: ProjectRationalTime(BigInt.zero, 1),
      duration: ProjectRationalTime(BigInt.from(durationMs), 1000),
    );
    current = await _advance(current, inserted, 'place the music bed');
    final clips = await widget.gateway.listTimelineClips(
      widget.session,
      trackId: trackId,
      offset: 0,
      limit: 100,
    );
    ProjectTimelineClip? musicClip;
    for (final clip in clips.items) {
      if (clip.mediaId == media.mediaId) {
        musicClip = clip;
        break;
      }
    }
    if (musicClip == null) {
      throw const MusicException('The placed music clip was not found.');
    }
    var ducked = false;
    var undoSteps = 0;
    final voiceTrack = _voiceTrackId;
    if (_duck && voiceTrack != null) {
      if (!mounted) {
        return MusicApplied(
          trackId: trackId,
          clipId: musicClip.clipId,
          ducked: false,
          undoSteps: 0,
          bed: bed,
        );
      }
      setState(() => _stage = 'Ducking the music under the voice track…');
      final result = await DuckingService(widget.gateway).duck(
        session: widget.session,
        project: current,
        musicTrackId: trackId,
        musicClipId: musicClip.clipId,
        voiceTrackId: voiceTrack,
        duckDecibels: -12,
      );
      ducked = result.changed;
      undoSteps = result.undoSteps;
    }
    return MusicApplied(
      trackId: trackId,
      clipId: musicClip.clipId,
      ducked: ducked,
      undoSteps: undoSteps,
      bed: bed,
    );
  }

  Future<ProjectReadModel> _advance(
    ProjectReadModel current,
    ProjectActionResult result,
    String action,
  ) async {
    if (!result.succeeded) {
      throw MusicException(
        result.message.isEmpty ? 'Could not $action.' : result.message,
      );
    }
    return result.view ?? await widget.gateway.summary(widget.session);
  }

  Future<ProjectMediaItem?> _findImportedMedia(String path) async {
    var page = await widget.gateway.listMediaPage(
      widget.session,
      offset: 0,
      limit: 100,
    );
    if (page.totalCount > 100) {
      page = await widget.gateway.listMediaPage(
        widget.session,
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

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: const Text('Generate background music'),
      content: SizedBox(
        width: 580,
        child: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              const Text(
                'Describe the bed you want, then generate it. It is synthesised '
                'locally, stays royalty-free, and can be ducked under a '
                'narration track in the same step.',
              ),
              const SizedBox(height: 12),
              TextField(
                key: const ValueKey('music-prompt'),
                controller: _promptField,
                enabled: !_busy,
                minLines: 1,
                maxLines: 3,
                decoration: const InputDecoration(
                  border: OutlineInputBorder(),
                  labelText: 'Describe the music',
                  hintText:
                      'e.g. “nhạc nền nhẹ nhàng 90 giây, không trống” or '
                      '“tense trailer 120 bpm 45s”',
                  helperText: 'Vietnamese or English: mood, tempo (bpm), length, drums.',
                  helperMaxLines: 2,
                ),
              ),
              if (_plan != null) ...[
                const SizedBox(height: 6),
                Row(
                  children: [
                    const Icon(
                      Icons.auto_awesome_outlined,
                      size: 14,
                      color: OrColors.textMuted,
                    ),
                    const SizedBox(width: 6),
                    Expanded(
                      child: Text(
                        _planReadback,
                        key: const ValueKey('music-plan-readback'),
                        style: const TextStyle(
                          color: OrColors.textSecondary,
                          fontSize: OrType.metadataCompactSize,
                        ),
                      ),
                    ),
                    TextButton(
                      key: const ValueKey('music-variation'),
                      onPressed: _busy ? null : _useVariation,
                      child: const Text('Variation'),
                    ),
                  ],
                ),
                for (final note in _plan!.notes)
                  Text(
                    note,
                    style: const TextStyle(
                      color: OrColors.textMuted,
                      fontSize: OrType.tinySize,
                    ),
                  ),
              ],
              const SizedBox(height: 6),
              Wrap(
                spacing: 6,
                runSpacing: 4,
                children: [
                  for (
                    var index = 0;
                    index < _musicPromptExamples.length;
                    index++
                  )
                    ActionChip(
                      key: ValueKey('music-prompt-example-$index'),
                      label: Text(
                        _musicPromptExamples[index].label,
                        style: const TextStyle(fontSize: OrType.tinySize),
                      ),
                      onPressed: _busy
                          ? null
                          : () {
                              _promptField.text =
                                  _musicPromptExamples[index].prompt;
                              _promptField.selection = TextSelection.collapsed(
                                offset: _promptField.text.length,
                              );
                              _interpretPrompt();
                            },
                    ),
                ],
              ),
              const SizedBox(height: 8),
              const Divider(height: 1),
              const SizedBox(height: 4),
              const Text(
                'Fine-tune',
                style: TextStyle(
                  color: OrColors.textMuted,
                  fontSize: OrType.tinySize,
                  fontWeight: FontWeight.w600,
                  letterSpacing: 0.6,
                ),
              ),
              const SizedBox(height: 4),
              DropdownButtonFormField<String>(
                key: const ValueKey('music-mood'),
                initialValue: _moodId,
                decoration: const InputDecoration(
                  border: OutlineInputBorder(),
                  labelText: 'Mood',
                ),
                items: [
                  for (final mood in musicMoods)
                    DropdownMenuItem(value: mood.id, child: Text(mood.label)),
                ],
                onChanged: _busy
                    ? null
                    : (value) => setState(() => _moodId = value ?? _moodId),
              ),
              const SizedBox(height: 8),
              _sliderRow(
                label: 'Tempo',
                value: _tempo,
                min: 50,
                max: 160,
                divisions: 22,
                display: '${_tempo.round()} bpm',
                onChanged: (value) => setState(() => _tempo = value),
              ),
              _sliderRow(
                label: 'Length',
                value: _seconds,
                min: 10,
                max: 180,
                divisions: 34,
                display: '${_seconds.round()} s',
                onChanged: (value) => setState(() => _seconds = value),
              ),
              const SizedBox(height: 4),
              if (_audioTracks.isEmpty)
                Text(
                  'No existing audio track to duck under. The bed will be placed at full level.',
                  style: TextStyle(color: OrColors.textSecondary),
                )
              else ...[
                DropdownButtonFormField<String?>(
                  key: ValueKey('music-voice-track-${_voiceTrackId ?? 'none'}'),
                  initialValue: _voiceTrackId,
                  decoration: const InputDecoration(
                    border: OutlineInputBorder(),
                    labelText: 'Lower music under',
                  ),
                  items: [
                    const DropdownMenuItem<String?>(
                      value: null,
                      child: Text('Nothing — keep full level'),
                    ),
                    for (final track in _audioTracks)
                      DropdownMenuItem<String?>(
                        value: track.trackId,
                        child: Text(
                          'Audio track ${track.trackId.substring(0, 8)} '
                          '(${track.clipCount} clips)',
                        ),
                      ),
                  ],
                  onChanged: _busy
                      ? null
                      : (value) => setState(() => _voiceTrackId = value),
                ),
                if (_voiceTrackId != null)
                  CheckboxListTile(
                    key: const ValueKey('music-duck'),
                    contentPadding: EdgeInsets.zero,
                    value: _duck,
                    onChanged: _busy
                        ? null
                        : (value) => setState(() => _duck = value ?? true),
                    title: const Text('Duck by -12 dB under this track'),
                  ),
              ],
              if (_busy) ...[
                const SizedBox(height: 8),
                LinearProgressIndicator(value: _progress),
                const SizedBox(height: 6),
                Text(
                  _stage,
                  key: const ValueKey('music-stage'),
                  style: TextStyle(color: OrColors.textSecondary),
                ),
              ],
              if (_error != null) ...[
                const SizedBox(height: 8),
                Text(
                  _error!,
                  key: const ValueKey('music-error'),
                  style: TextStyle(color: Theme.of(context).colorScheme.error),
                ),
              ],
            ],
          ),
        ),
      ),
      actions: [
        TextButton(
          onPressed: _busy ? null : () => Navigator.of(context).pop(),
          child: const Text('Cancel'),
        ),
        FilledButton(
          key: const ValueKey('music-generate'),
          onPressed: _busy ? null : _generate,
          child: const Text('Generate and add'),
        ),
      ],
    );
  }

  Widget _sliderRow({
    required String label,
    required double value,
    required double min,
    required double max,
    required int divisions,
    required String display,
    required ValueChanged<double> onChanged,
  }) {
    return Row(
      children: [
        SizedBox(width: 72, child: Text(label)),
        Expanded(
          child: Slider(
            value: value,
            min: min,
            max: max,
            divisions: divisions,
            label: display,
            onChanged: _busy ? null : onChanged,
          ),
        ),
        SizedBox(
          width: 72,
          child: Text(
            display,
            textAlign: TextAlign.end,
            style: TextStyle(color: OrColors.textSecondary),
          ),
        ),
      ],
    );
  }
}

/// One-tap starting points. They are examples, not the whole grammar: the field
/// accepts any sentence whose mood, tempo, length or drum intent can be read.
const List<({String label, String prompt})> _musicPromptExamples = [
  (
    label: 'Nhẹ nhàng 90 giây',
    prompt: 'nhạc nền nhẹ nhàng thư giãn 90 giây, không trống',
  ),
  (label: 'Vlog sôi động', prompt: 'upbeat vlog 120 bpm 45 giây, trống nhẹ'),
  (
    label: 'Trailer kịch tính',
    prompt: 'căng thẳng kịch tính cho trailer, 100 bpm, 30 giây',
  ),
];
