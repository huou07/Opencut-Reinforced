// Shared fixtures: sessions, envelopes and material used by more than one owner file.

use super::super::*;
use super::*;
use serde_json::{Value, json};
use std::str::FromStr;

pub(super) fn project() -> ProjectDocument {
    decode_project(&format!(
        r#"{{"format":"opencut-reinforced-project","schema_version":1,"project":{{"id":"{PROJECT_ID}","revision":0,"name":"A"}}}}"#
    ))
    .unwrap()
}

pub(super) fn fixed_session() -> ProjectSession {
    ProjectSession {
        project: project(),
        project_instance_id: ProjectInstanceId::from_str(INSTANCE_ID).unwrap(),
        history: super::super::SessionHistory::default(),
    }
}

pub(super) fn media_item(id: &str, uri: &str) -> MediaItem {
    MediaItem::new(
        MediaId::from_str(id).unwrap(),
        MediaSourceRef::local_file(uri).unwrap(),
        MediaMetadata::from_probe(
            vec!["matroska".to_owned()],
            None,
            42,
            vec![MediaStreamMetadata::Video(VideoStreamMetadata::from_probe(
                0,
                None,
                NonZeroU32::new(1920).unwrap(),
                NonZeroU32::new(1080).unwrap(),
                None,
                Some(RationalRate::new(24, 1).unwrap()),
                None,
            ))],
        ),
    )
    .unwrap()
}

pub(super) fn audio_media_item(id: &str, uri: &str) -> MediaItem {
    MediaItem::new(
        MediaId::from_str(id).unwrap(),
        MediaSourceRef::local_file(uri).unwrap(),
        MediaMetadata::from_probe(
            vec!["wav".to_owned()],
            None,
            42,
            vec![MediaStreamMetadata::Audio(AudioStreamMetadata::from_probe(
                0, None, None, None, None, None,
            ))],
        ),
    )
    .unwrap()
}

pub(super) fn video_media_item_with_duration(
    id: &str,
    uri: &str,
    container_duration: Option<RationalTime>,
    stream_duration: Option<RationalTime>,
) -> MediaItem {
    MediaItem::new(
        MediaId::from_str(id).unwrap(),
        MediaSourceRef::local_file(uri).unwrap(),
        MediaMetadata::from_probe(
            vec!["matroska".to_owned()],
            container_duration,
            42,
            vec![MediaStreamMetadata::Video(VideoStreamMetadata::from_probe(
                0,
                None,
                NonZeroU32::new(1920).unwrap(),
                NonZeroU32::new(1080).unwrap(),
                None,
                Some(RationalRate::new(24, 1).unwrap()),
                stream_duration,
            ))],
        ),
    )
    .unwrap()
}

pub(super) fn audio_video_media_item(id: &str, uri: &str) -> MediaItem {
    MediaItem::new(
        MediaId::from_str(id).unwrap(),
        MediaSourceRef::local_file(uri).unwrap(),
        MediaMetadata::from_probe(
            vec!["matroska".to_owned()],
            None,
            42,
            vec![
                MediaStreamMetadata::Video(VideoStreamMetadata::from_probe(
                    0,
                    None,
                    NonZeroU32::new(1920).unwrap(),
                    NonZeroU32::new(1080).unwrap(),
                    None,
                    Some(RationalRate::new(24, 1).unwrap()),
                    None,
                )),
                MediaStreamMetadata::Audio(AudioStreamMetadata::from_probe(
                    1, None, None, None, None, None,
                )),
            ],
        ),
    )
    .unwrap()
}

pub(super) fn media_add(item: MediaItem, revision: u64) -> CommandEnvelope {
    CommandEnvelope::add_media(
        ProjectId::from_str(PROJECT_ID).unwrap(),
        ProjectInstanceId::from_str(INSTANCE_ID).unwrap(),
        ProjectRevision::new(revision),
        item,
    )
}

pub(super) fn media_remove(id: &str, revision: u64) -> CommandEnvelope {
    CommandEnvelope::remove_media(
        ProjectId::from_str(PROJECT_ID).unwrap(),
        ProjectInstanceId::from_str(INSTANCE_ID).unwrap(),
        ProjectRevision::new(revision),
        MediaId::from_str(id).unwrap(),
    )
}

pub(super) fn media_relink(item: MediaItem, revision: u64) -> CommandEnvelope {
    CommandEnvelope::relink_media(
        ProjectId::from_str(PROJECT_ID).unwrap(),
        ProjectInstanceId::from_str(INSTANCE_ID).unwrap(),
        ProjectRevision::new(revision),
        item,
    )
}

pub(super) fn session_with_state(name: &str, revision: u64) -> ProjectSession {
    let project = decode_project(&format!(
        r#"{{"format":"opencut-reinforced-project","schema_version":1,"project":{{"id":"{PROJECT_ID}","revision":{revision},"name":"{name}"}}}}"#
    ))
    .unwrap();
    ProjectSession {
        project,
        project_instance_id: ProjectInstanceId::from_str(INSTANCE_ID).unwrap(),
        history: super::super::SessionHistory::default(),
    }
}

pub(super) fn rename(name: &str, revision: u64) -> CommandEnvelope {
    CommandEnvelope {
        command_id: "project.rename".to_owned(),
        schema_version: 1,
        project_id: ProjectId::from_str(PROJECT_ID).unwrap(),
        project_instance_id: ProjectInstanceId::from_str(INSTANCE_ID).unwrap(),
        expected_project_revision: ProjectRevision::new(revision),
        arguments: json!({ "name": name }),
    }
}

pub(super) fn call(command_id: &str, schema_version: u64, arguments: Value) -> CommandCall {
    CommandCall {
        command_id: command_id.to_owned(),
        schema_version,
        arguments,
    }
}

pub(super) fn rename_call(name: &str) -> CommandCall {
    call("project.rename", 1, json!({ "name": name }))
}

pub(super) fn transaction(commands: Vec<CommandCall>, revision: u64) -> TransactionEnvelope {
    TransactionEnvelope {
        schema_version: CURRENT_TRANSACTION_SCHEMA_VERSION,
        project_id: ProjectId::from_str(PROJECT_ID).unwrap(),
        project_instance_id: ProjectInstanceId::from_str(INSTANCE_ID).unwrap(),
        expected_project_revision: ProjectRevision::new(revision),
        commands,
    }
}

pub(super) fn history_command(
    command_id: &str,
    revision: u64,
    arguments: Value,
) -> CommandEnvelope {
    CommandEnvelope {
        command_id: command_id.to_owned(),
        schema_version: 1,
        project_id: ProjectId::from_str(PROJECT_ID).unwrap(),
        project_instance_id: ProjectInstanceId::from_str(INSTANCE_ID).unwrap(),
        expected_project_revision: ProjectRevision::new(revision),
        arguments,
    }
}

pub(super) fn undo(revision: u64) -> CommandEnvelope {
    history_command("history.undo", revision, json!({}))
}

pub(super) fn redo(revision: u64) -> CommandEnvelope {
    history_command("history.redo", revision, json!({}))
}

pub(super) fn sequence_rate(rate: Option<RationalRate>, revision: u64) -> CommandEnvelope {
    CommandEnvelope::set_timeline_sequence_frame_rate(
        ProjectId::from_str(PROJECT_ID).unwrap(),
        ProjectInstanceId::from_str(INSTANCE_ID).unwrap(),
        ProjectRevision::new(revision),
        rate,
    )
}

pub(super) fn execute(
    session: &mut ProjectSession,
    command_id: &str,
    arguments: Value,
) -> Result<super::super::CommandResult, super::super::OperationError> {
    session.execute_command(history_command(
        command_id,
        session.project_revision().value(),
        arguments,
    ))
}

pub(super) fn add_track(
    session: &mut ProjectSession,
    id: &str,
    kind: &str,
) -> Result<super::super::CommandResult, super::super::OperationError> {
    execute(
        session,
        "timeline.track.add",
        json!({"track_id": id, "kind": kind}),
    )
}

pub(super) fn rational_json(numerator: i64, denominator: u32) -> Value {
    json!({"numerator": numerator, "denominator": denominator})
}

pub(super) fn marker_id(value: &str) -> MarkerId {
    value.parse().unwrap()
}

pub(super) fn marker_add(
    session: &mut ProjectSession,
    marker_id: &str,
    timeline_time: (i64, u32),
    label: &str,
) -> Result<super::super::CommandResult, super::super::OperationError> {
    execute(
        session,
        "timeline.marker.add",
        json!({
            "marker_id": marker_id,
            "timeline_time": rational_json(timeline_time.0, timeline_time.1),
            "label": label,
        }),
    )
}

pub(super) fn snap_query(
    session: &ProjectSession,
    operation: super::super::TimelineSnapOperation,
    clip_id: &str,
    target_track_id: Option<&str>,
    target_time: (i64, u32),
) -> QueryEnvelope {
    QueryEnvelope::timeline_snap(
        session.project_id(),
        session.project_instance_id(),
        operation,
        clip_id.parse().unwrap(),
        target_track_id.map(|id| id.parse().unwrap()),
        RationalTime::new(target_time.0, target_time.1).unwrap(),
    )
}

pub(super) fn insert_clip(
    session: &mut ProjectSession,
    clip_id: &str,
    track_id: &str,
    media_id: &str,
    timeline_start: (i64, u32),
    source_start: (i64, u32),
    duration: (i64, u32),
) -> Result<super::super::CommandResult, super::super::OperationError> {
    execute(
        session,
        "timeline.clip.insert",
        json!({
            "clip_id": clip_id,
            "track_id": track_id,
            "media_id": media_id,
            "timeline_start": rational_json(timeline_start.0, timeline_start.1),
            "source_range": {
                "start": rational_json(source_start.0, source_start.1),
                "duration": rational_json(duration.0, duration.1),
            }
        }),
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn insert_clip_mode(
    session: &mut ProjectSession,
    clip_id: &str,
    track_id: &str,
    media_id: &str,
    timeline_start: (i64, u32),
    duration: (i64, u32),
    mode: &str,
    split_clip_id: Option<&str>,
) -> Result<super::super::CommandResult, super::super::OperationError> {
    let mut arguments = json!({
        "clip_id": clip_id,
        "track_id": track_id,
        "media_id": media_id,
        "timeline_start": rational_json(timeline_start.0, timeline_start.1),
        "source_range": {
            "start": rational_json(0, 1),
            "duration": rational_json(duration.0, duration.1),
        },
        "mode": mode,
    });
    if let Some(split_clip_id) = split_clip_id {
        arguments["overwrite_split_clip_id"] = json!(split_clip_id);
    }
    execute(session, "timeline.clip.insert", arguments)
}

/// The track's clips as `(id, start, duration)` in timeline order.

pub(super) fn clip_spans(
    session: &ProjectSession,
    track_id: &str,
) -> Vec<(String, RationalTime, RationalTime)> {
    let tracks = session.project().timeline().tracks();
    let index = tracks
        .iter()
        .position(|track| track.id().to_string() == track_id)
        .expect("track exists");
    tracks[index]
        .clips()
        .iter()
        .map(|clip| {
            (
                clip.id().to_string(),
                clip.timeline_start(),
                clip.timeline_duration(),
            )
        })
        .collect()
}

pub(super) fn seed_media_and_track(session: &mut ProjectSession, media_id: &str, track_id: &str) {
    session
        .execute_command(media_add(
            media_item(media_id, "file:///missing/offline.mov"),
            session.project_revision().value(),
        ))
        .unwrap();
    add_track(session, track_id, "video").unwrap();
}

pub(super) fn insert_clip_at(
    session: &mut ProjectSession,
    clip_id: &str,
    track_id: &str,
    media_id: &str,
    timeline_start: (i64, u32),
    duration: (i64, u32),
) {
    insert_clip(
        session,
        clip_id,
        track_id,
        media_id,
        timeline_start,
        (0, 1),
        duration,
    )
    .unwrap();
}

pub(super) fn ripple_trim_end(
    session: &mut ProjectSession,
    clip_id: &str,
    target: (i64, u32),
    ripple: bool,
) -> Result<super::super::CommandResult, super::super::OperationError> {
    execute(
        session,
        "timeline.clip.trim",
        json!({
            "clip_id": clip_id,
            "edge": "end",
            "timeline_time": rational_json(target.0, target.1),
            "ripple": ripple,
        }),
    )
}

pub(super) fn slip_clip(
    session: &mut ProjectSession,
    clip_id: &str,
    delta: (i64, u32),
) -> Result<super::super::CommandResult, super::super::OperationError> {
    execute(
        session,
        "timeline.clip.slip",
        json!({
            "clip_id": clip_id,
            "delta": rational_json(delta.0, delta.1),
        }),
    )
}

/// The clip's source window start, which is what a slip moves.

pub(super) fn source_start(
    session: &ProjectSession,
    track_id: &str,
    clip_id: &str,
) -> RationalTime {
    let tracks = session.project().timeline().tracks();
    let index = tracks
        .iter()
        .position(|track| track.id().to_string() == track_id)
        .expect("track exists");
    let clip = tracks[index]
        .clips()
        .iter()
        .find(|clip| clip.id().to_string() == clip_id)
        .expect("clip exists");
    match clip.content() {
        ClipContent::Media { source_range, .. } => source_range.start(),
        other => panic!("expected media content, found {other:?}"),
    }
}

pub(super) fn roll_boundary(
    session: &mut ProjectSession,
    clip_id: &str,
    delta: (i64, u32),
) -> Result<super::super::CommandResult, super::super::OperationError> {
    execute(
        session,
        "timeline.clip.roll",
        json!({
            "clip_id": clip_id,
            "delta": rational_json(delta.0, delta.1),
        }),
    )
}

pub(super) fn slide_clip(
    session: &mut ProjectSession,
    clip_id: &str,
    delta: (i64, u32),
) -> Result<super::super::CommandResult, super::super::OperationError> {
    execute(
        session,
        "timeline.clip.slide",
        json!({
            "clip_id": clip_id,
            "delta": rational_json(delta.0, delta.1),
        }),
    )
}

/// Three adjacent one-second clips, which is the smallest track a slide has a
/// middle clip to move on.

pub(super) fn seed_three_adjacent_clips(session: &mut ProjectSession) {
    seed_media_and_track(session, MEDIA_A, TRACK_A);
    insert_clip_at(session, CLIP_A, TRACK_A, MEDIA_A, (0, 1), (1, 1));
    insert_clip_at(session, CLIP_B, TRACK_A, MEDIA_A, (1, 1), (1, 1));
    insert_clip_at(session, CLIP_C, TRACK_A, MEDIA_A, (2, 1), (1, 1));
}

pub(super) fn move_track(
    session: &mut ProjectSession,
    track_id: &str,
    to_index: i64,
) -> Result<super::super::CommandResult, super::super::OperationError> {
    execute(
        session,
        "timeline.track.move",
        json!({
            "track_id": track_id,
            "to_index": to_index,
        }),
    )
}

/// The track kinds in order, which is the only thing a move changes.

pub(super) fn track_order(session: &ProjectSession) -> Vec<String> {
    session
        .project()
        .timeline()
        .tracks()
        .iter()
        .map(|track| track.id().to_string())
        .collect()
}

pub(super) fn seed_video_track_and_clip(
    session: &mut ProjectSession,
    media_id: &str,
    track_id: &str,
    clip_id: &str,
    timeline_start: (i64, u32),
    duration: (i64, u32),
) {
    session
        .execute_command(media_add(
            media_item(media_id, "file:///missing/offline.mov"),
            session.project_revision().value(),
        ))
        .unwrap();
    add_track(session, track_id, "video").unwrap();
    insert_clip(
        session,
        clip_id,
        track_id,
        media_id,
        timeline_start,
        (0, 1),
        duration,
    )
    .unwrap();
}

pub(super) fn fixture_clip(
    clip_id: &str,
    media_id: &str,
    timeline_start: (i64, u32),
    source_start: (i64, u32),
    duration: (i64, u32),
) -> TimelineClip {
    TimelineClip::from_parts_for_codec(
        clip_id.parse().unwrap(),
        media_id.parse().unwrap(),
        RationalTime::new(timeline_start.0, timeline_start.1).unwrap(),
        TimeRange::new(
            RationalTime::new(source_start.0, source_start.1).unwrap(),
            RationalTime::new(duration.0, duration.1).unwrap(),
        )
        .unwrap(),
    )
}

pub(super) fn fixture_track(
    track_id: &str,
    kind: TrackKind,
    clips: Vec<TimelineClip>,
) -> TimelineTrack {
    TimelineTrack::from_parts_for_codec(track_id.parse().unwrap(), kind, clips)
}

pub(super) fn assert_name_change(change_set: &ChangeSet, before: &str, after: &str) {
    assert!(matches!(
        change_set.changes(),
        [ProjectChange::ProjectName { before: actual_before, after: actual_after }]
            if actual_before.as_str() == before && actual_after.as_str() == after
    ));
}

pub(super) fn summary_query() -> QueryEnvelope {
    QueryEnvelope {
        query_id: "project.summary".to_owned(),
        schema_version: 1,
        project_id: ProjectId::from_str(PROJECT_ID).unwrap(),
        project_instance_id: ProjectInstanceId::from_str(INSTANCE_ID).unwrap(),
        arguments: json!({}),
    }
}

pub(super) fn code<T>(result: &Result<T, super::super::OperationError>) -> OperationErrorCode {
    result.as_ref().err().expect("operation should fail").code
}

pub(super) fn caption_clip_state(
    id: &str,
    start: (i64, u32),
    duration: (i64, u32),
    text: &str,
) -> TimelineClipState {
    TimelineClipState {
        clip_id: ClipId::from_str(id).unwrap(),
        timeline_start: RationalTime::new(start.0, start.1).unwrap(),
        timeline_duration: RationalTime::new(duration.0, duration.1).unwrap(),
        content: ClipContent::Caption {
            text: text.to_owned(),
            formatting: crate::TextFormatting::DEFAULT,
        },
        settings: ClipSettings::Visual(crate::VisualSettings::default()),
    }
}

pub(super) fn snap_fixture() -> ProjectSession {
    let mut session = fixed_session();
    session
        .execute_command(media_add(
            media_item(MEDIA_A, "file:///missing/snap.mov"),
            0,
        ))
        .unwrap();
    add_track(&mut session, TRACK_A, "video").unwrap();
    add_track(&mut session, TRACK_B, "video").unwrap();
    add_track(&mut session, TRACK_C, "audio").unwrap();
    add_track(&mut session, TRACK_D, "video").unwrap();
    insert_clip(
        &mut session,
        CLIP_A,
        TRACK_A,
        MEDIA_A,
        (2, 1),
        (0, 1),
        (2, 1),
    )
    .unwrap();
    insert_clip(
        &mut session,
        CLIP_B,
        TRACK_B,
        MEDIA_A,
        (0, 1),
        (0, 1),
        (1, 1),
    )
    .unwrap();
    insert_clip(
        &mut session,
        CLIP_C,
        TRACK_B,
        MEDIA_A,
        (4, 1),
        (1, 1),
        (1, 2),
    )
    .unwrap();
    insert_clip(
        &mut session,
        CLIP_D,
        TRACK_B,
        MEDIA_A,
        (49, 10),
        (2, 1),
        (1, 1),
    )
    .unwrap();
    insert_clip(
        &mut session,
        CLIP_E,
        TRACK_B,
        MEDIA_A,
        (61, 10),
        (3, 1),
        (1, 1),
    )
    .unwrap();
    insert_clip(
        &mut session,
        "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaac",
        TRACK_B,
        MEDIA_A,
        (8, 1),
        (4, 1),
        (1, 1),
    )
    .unwrap();
    insert_clip(
        &mut session,
        "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaad",
        TRACK_B,
        MEDIA_A,
        (9, 1),
        (5, 1),
        (1, 1),
    )
    .unwrap();
    insert_clip(
        &mut session,
        "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbc",
        TRACK_D,
        MEDIA_A,
        (9, 1),
        (6, 1),
        (1, 1),
    )
    .unwrap();
    insert_clip(
        &mut session,
        "cccccccc-cccc-4ccc-8ccc-cccccccccccd",
        TRACK_D,
        MEDIA_A,
        (43, 5),
        (7, 1),
        (1, 5),
    )
    .unwrap();
    session
}

pub(super) fn assert_history_conflict(session: &mut ProjectSession, command_id: &str) {
    let before_project = session.project().clone();
    let before_undo = session.history.undo.clone();
    let before_redo = session.history.redo.clone();
    let before_revision = session.project_revision();
    let result = session.execute_command(history_command(
        command_id,
        before_revision.value(),
        json!({}),
    ));
    assert_eq!(code(&result), OperationErrorCode::HistoryConflict);
    assert_eq!(session.project(), &before_project);
    assert_eq!(session.history.undo, before_undo);
    assert_eq!(session.history.redo, before_redo);
    assert_eq!(session.project_revision(), before_revision);
}

pub(super) fn max_revision_session_with_clip(include_clip: bool) -> ProjectSession {
    let mut session = session_with_state("A", u64::MAX);
    session.project.insert_media_for_command(
        media_item(MEDIA_A, "file:///missing/offline.mov"),
        0,
        ProjectRevision::new(u64::MAX),
    );
    let clips = if include_clip {
        vec![fixture_clip(CLIP_A, MEDIA_A, (0, 1), (0, 1), (2, 1))]
    } else {
        Vec::new()
    };
    session
        .project
        .set_timeline_for_test(ProjectTimeline::from_tracks_for_codec(vec![fixture_track(
            TRACK_A,
            TrackKind::Video,
            clips,
        )]));
    session
}
