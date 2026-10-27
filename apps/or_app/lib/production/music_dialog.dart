import 'package:flutter/material.dart';

import '../design/or_colors.dart';
import '../project/project_gateway.dart';
import 'ducking_service.dart';
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

  @override
  void initState() {
    super.initState();
    _loadTracks();
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
      final bed = await _music.generate(
        moodId: _moodId,
        tempo: _tempo.round(),
        seconds: _seconds.round(),
        directory: directory,
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
        (duration.numerator * BigInt.from(1000)) ~/ BigInt.from(duration.denominator);
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
                'A quiet, royalty-free bed is synthesised locally and placed on '
                'its own audio track. It can be ducked under a narration track '
                'in the same step.',
              ),
              const SizedBox(height: 12),
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
