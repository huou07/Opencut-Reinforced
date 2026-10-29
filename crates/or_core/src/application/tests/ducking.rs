// Audio ducking commands.

use super::super::*;
use super::support::*;
use super::*;
use serde_json::json;
use std::str::FromStr;

#[test]
fn ducking_splits_and_lowers_overlapping_segments_in_one_undo_entry() {
    let mut session = fixed_session();
    session
        .execute_command(media_add(
            audio_media_item(MEDIA_A, "file:///missing/music.wav"),
            0,
        ))
        .unwrap();
    session
        .execute_command(media_add(
            audio_media_item(MEDIA_B, "file:///missing/voice.wav"),
            1,
        ))
        .unwrap();
    add_track(&mut session, TRACK_A, "audio").unwrap();
    add_track(&mut session, TRACK_B, "audio").unwrap();
    insert_clip(
        &mut session,
        CLIP_A,
        TRACK_A,
        MEDIA_A,
        (0, 1),
        (0, 1),
        (60, 1),
    )
    .unwrap();
    insert_clip(
        &mut session,
        CLIP_B,
        TRACK_B,
        MEDIA_B,
        (10, 1),
        (0, 1),
        (10, 1),
    )
    .unwrap();
    insert_clip(
        &mut session,
        CLIP_C,
        TRACK_B,
        MEDIA_B,
        (30, 1),
        (0, 1),
        (5, 1),
    )
    .unwrap();

    let undo_before = session.history.undo.len();
    let result = execute(
        &mut session,
        "timeline.clip.duck",
        json!({
            "track_id": TRACK_A,
            "clip_id": CLIP_A,
            "voice_track_id": TRACK_B,
            "duck_millidecibels": -12_000,
        }),
    )
    .unwrap();
    assert!(result.changed);
    assert_eq!(session.history.undo.len(), undo_before + 1);
    assert!(matches!(
        result.change_set.changes(),
        [ProjectChange::TimelineClipsOverwritten { track_id, before, after }]
            if *track_id == TrackId::from_str(TRACK_A).unwrap()
                && before.len() == 1
                && after.len() == 5
    ));
    let music = session
        .project()
        .timeline()
        .tracks()
        .iter()
        .find(|track| track.id() == TrackId::from_str(TRACK_A).unwrap())
        .unwrap();
    let clips = music.clips();
    assert_eq!(clips.len(), 5);
    let spans: Vec<(i64, i64)> = clips
        .iter()
        .map(|clip| {
            (
                clip.timeline_start().numerator(),
                clip.timeline_duration().numerator(),
            )
        })
        .collect();
    assert_eq!(spans, [(0, 10), (10, 10), (20, 10), (30, 5), (35, 25)]);
    let gains: Vec<i32> = clips
        .iter()
        .map(|clip| match clip.settings() {
            ClipSettings::Audio(audio) => audio.gain_millidecibels,
            ClipSettings::Visual(_) => panic!("the music clip stays audio"),
        })
        .collect();
    assert_eq!(gains, [0, -12_000, 0, -12_000, 0]);

    session
        .execute_command(undo(session.project_revision().value()))
        .unwrap();
    let restored = session
        .project()
        .timeline()
        .tracks()
        .iter()
        .find(|track| track.id() == TrackId::from_str(TRACK_A).unwrap())
        .unwrap();
    assert_eq!(restored.clips().len(), 1);
    assert_eq!(restored.clips()[0].timeline_duration().numerator(), 60);
    assert!(matches!(
        restored.clips()[0].settings(),
        ClipSettings::Audio(audio) if audio.gain_millidecibels == 0
    ));
}

#[test]
fn ducking_refuses_a_voice_track_that_does_not_overlap() {
    let mut session = fixed_session();
    session
        .execute_command(media_add(
            audio_media_item(MEDIA_A, "file:///missing/music.wav"),
            0,
        ))
        .unwrap();
    session
        .execute_command(media_add(
            audio_media_item(MEDIA_B, "file:///missing/voice.wav"),
            1,
        ))
        .unwrap();
    add_track(&mut session, TRACK_A, "audio").unwrap();
    add_track(&mut session, TRACK_B, "audio").unwrap();
    insert_clip(
        &mut session,
        CLIP_A,
        TRACK_A,
        MEDIA_A,
        (0, 1),
        (0, 1),
        (10, 1),
    )
    .unwrap();
    insert_clip(
        &mut session,
        CLIP_B,
        TRACK_B,
        MEDIA_B,
        (20, 1),
        (0, 1),
        (5, 1),
    )
    .unwrap();
    let error = execute(
        &mut session,
        "timeline.clip.duck",
        json!({
            "track_id": TRACK_A,
            "clip_id": CLIP_A,
            "voice_track_id": TRACK_B,
            "duck_millidecibels": -12_000,
        }),
    )
    .unwrap_err();
    assert_eq!(error.code, OperationErrorCode::TimelineNothingToDuck);
}
