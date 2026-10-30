// Timeline clip insert, overwrite, split, move, delete and rate commands.

use super::super::*;
use super::support::*;
use super::*;
use serde_json::json;
use std::str::FromStr;

#[test]
fn overwrite_insert_removes_a_clip_it_fully_covers() {
    let mut session = session_with_state("overwrite-test", 0);
    seed_media_and_track(&mut session, MEDIA_A, TRACK_A);
    insert_clip(
        &mut session,
        CLIP_A,
        TRACK_A,
        MEDIA_A,
        (0, 1),
        (0, 1),
        (4, 1),
    )
    .unwrap();
    let revision_before = session.project_revision();

    insert_clip_mode(
        &mut session,
        CLIP_B,
        TRACK_A,
        MEDIA_A,
        (0, 1),
        (4, 1),
        "overwrite",
        None,
    )
    .unwrap();

    let spans = clip_spans(&session, TRACK_A);
    assert_eq!(spans.len(), 1, "the covered clip must be gone");
    assert_eq!(spans[0].0, CLIP_B);
    assert_eq!(spans[0].1, RationalTime::new(0, 1).unwrap());
    assert_eq!(spans[0].2, RationalTime::new(4, 1).unwrap());
    assert_eq!(
        session.project_revision().value(),
        revision_before.value() + 1,
        "an overwrite is one revision"
    );

    execute(&mut session, "history.undo", json!({})).unwrap();
    let restored = clip_spans(&session, TRACK_A);
    assert_eq!(restored.len(), 1, "one undo restores the replaced range");
    assert_eq!(restored[0].0, CLIP_A);
    assert_eq!(restored[0].1, RationalTime::new(0, 1).unwrap());
    assert_eq!(restored[0].2, RationalTime::new(4, 1).unwrap());
}

#[test]
fn overwrite_insert_trims_a_clip_it_crosses_at_its_head() {
    let mut session = session_with_state("overwrite-test", 0);
    seed_media_and_track(&mut session, MEDIA_A, TRACK_A);
    insert_clip(
        &mut session,
        CLIP_A,
        TRACK_A,
        MEDIA_A,
        (2, 1),
        (0, 1),
        (4, 1),
    )
    .unwrap();

    insert_clip_mode(
        &mut session,
        CLIP_B,
        TRACK_A,
        MEDIA_A,
        (0, 1),
        (4, 1),
        "overwrite",
        None,
    )
    .unwrap();

    let spans = clip_spans(&session, TRACK_A);
    assert_eq!(spans.len(), 2);
    assert_eq!(spans[0].0, CLIP_B);
    assert_eq!(spans[1].0, CLIP_A);
    assert_eq!(
        spans[1].1,
        RationalTime::new(4, 1).unwrap(),
        "the crossed clip keeps only its tail"
    );
    assert_eq!(spans[1].2, RationalTime::new(2, 1).unwrap());
}

#[test]
fn overwrite_insert_trims_a_clip_it_crosses_at_its_tail() {
    let mut session = session_with_state("overwrite-test", 0);
    seed_media_and_track(&mut session, MEDIA_A, TRACK_A);
    insert_clip(
        &mut session,
        CLIP_A,
        TRACK_A,
        MEDIA_A,
        (0, 1),
        (0, 1),
        (4, 1),
    )
    .unwrap();

    insert_clip_mode(
        &mut session,
        CLIP_B,
        TRACK_A,
        MEDIA_A,
        (2, 1),
        (4, 1),
        "overwrite",
        None,
    )
    .unwrap();

    let spans = clip_spans(&session, TRACK_A);
    assert_eq!(spans.len(), 2);
    assert_eq!(spans[0].0, CLIP_A);
    assert_eq!(spans[0].1, RationalTime::new(0, 1).unwrap());
    assert_eq!(
        spans[0].2,
        RationalTime::new(2, 1).unwrap(),
        "the crossed clip keeps only its head"
    );
    assert_eq!(spans[1].0, CLIP_B);
}

#[test]
fn overwrite_inside_a_clip_needs_a_fragment_id_and_names_it() {
    let mut session = session_with_state("overwrite-test", 0);
    seed_media_and_track(&mut session, MEDIA_A, TRACK_A);
    insert_clip(
        &mut session,
        CLIP_A,
        TRACK_A,
        MEDIA_A,
        (0, 1),
        (0, 1),
        (8, 1),
    )
    .unwrap();

    let error = insert_clip_mode(
        &mut session,
        CLIP_B,
        TRACK_A,
        MEDIA_A,
        (2, 1),
        (2, 1),
        "overwrite",
        None,
    )
    .unwrap_err();

    assert_eq!(
        error.code,
        super::super::OperationErrorCode::TimelineOverwriteSplitRequiresClipId,
        "silently dropping the tail would be worse than refusing"
    );
    let spans = clip_spans(&session, TRACK_A);
    assert_eq!(spans.len(), 1, "a refused overwrite changes nothing");
    assert_eq!(spans[0].0, CLIP_A);
}

#[test]
fn overwrite_inside_a_clip_splits_it_with_the_callers_id() {
    let mut session = session_with_state("overwrite-test", 0);
    seed_media_and_track(&mut session, MEDIA_A, TRACK_A);
    insert_clip(
        &mut session,
        CLIP_A,
        TRACK_A,
        MEDIA_A,
        (0, 1),
        (0, 1),
        (8, 1),
    )
    .unwrap();

    insert_clip_mode(
        &mut session,
        CLIP_B,
        TRACK_A,
        MEDIA_A,
        (2, 1),
        (2, 1),
        "overwrite",
        Some(CLIP_C),
    )
    .unwrap();

    let spans = clip_spans(&session, TRACK_A);
    assert_eq!(spans.len(), 3);
    assert_eq!(spans[0].0, CLIP_A);
    assert_eq!(spans[0].2, RationalTime::new(2, 1).unwrap());
    assert_eq!(spans[1].0, CLIP_B);
    assert_eq!(spans[1].1, RationalTime::new(2, 1).unwrap());
    assert_eq!(spans[1].2, RationalTime::new(2, 1).unwrap());
    assert_eq!(spans[2].0, CLIP_C);
    assert_eq!(spans[2].1, RationalTime::new(4, 1).unwrap());
    assert_eq!(spans[2].2, RationalTime::new(4, 1).unwrap());

    execute(&mut session, "history.undo", json!({})).unwrap();
    let restored = clip_spans(&session, TRACK_A);
    assert_eq!(restored.len(), 1, "undo restores the unsplit clip");
    assert_eq!(restored[0].0, CLIP_A);
    assert_eq!(restored[0].2, RationalTime::new(8, 1).unwrap());
}

#[test]
fn insert_mode_still_rejects_an_overlap() {
    let mut session = session_with_state("overwrite-test", 0);
    seed_media_and_track(&mut session, MEDIA_A, TRACK_A);
    insert_clip(
        &mut session,
        CLIP_A,
        TRACK_A,
        MEDIA_A,
        (0, 1),
        (0, 1),
        (4, 1),
    )
    .unwrap();

    let error = insert_clip(
        &mut session,
        CLIP_B,
        TRACK_A,
        MEDIA_A,
        (2, 1),
        (0, 1),
        (2, 1),
    )
    .unwrap_err();

    assert_eq!(
        error.code,
        super::super::OperationErrorCode::TimelineOverlap,
        "the default mode keeps its old meaning"
    );
}

#[test]
fn a_media_clip_without_a_rate_cannot_map_different_source_and_timeline_durations() {
    let mut session = session_with_state("rate-invariant", 0);
    seed_media_and_track(&mut session, MEDIA_A, TRACK_A);
    insert_clip_at(&mut session, CLIP_A, TRACK_A, MEDIA_A, (0, 1), (1, 1));
    let before = clip_spans(&session, TRACK_A);
    let revision_before = session.project_revision();

    // One second of source over half a second of timeline needs an explicit 2x
    // rate; omitting it means identity speed and remains invalid.
    let error = execute(
        &mut session,
        "timeline.clip.update",
        json!({
            "clip_id": CLIP_A,
            "timeline_duration": rational_json(1, 2),
            "content": {
                "type": "media",
                "media_id": MEDIA_A,
                "source_range": {
                    "start": rational_json(0, 1),
                    "duration": rational_json(1, 1),
                },
            },
            "settings": {
                "type": "visual",
                "transform": {
                    "x_milli_canvas": 0,
                    "y_milli_canvas": 0,
                    "scale_x_milli": 1000,
                    "scale_y_milli": 1000,
                    "rotation_milli_degrees": 0,
                    "anchor_x_basis_points": 5000,
                    "anchor_y_basis_points": 5000,
                },
                "crop": {
                    "left_basis_points": 0,
                    "top_basis_points": 0,
                    "right_basis_points": 0,
                    "bottom_basis_points": 0,
                },
                "opacity": {"basis_points": 10000},
                "effects": [],
                "transition_in": null,
                "transition_out": null,
            },
        }),
    )
    .unwrap_err();

    assert_eq!(
        error.code,
        super::super::OperationErrorCode::InvalidArguments
    );
    assert_eq!(session.project_revision().value(), revision_before.value());
    assert_eq!(clip_spans(&session, TRACK_A), before);
}

#[test]
fn a_media_clip_can_play_at_a_different_rate_and_undo_restores_identity() {
    let mut session = session_with_state("rate-update", 0);
    seed_media_and_track(&mut session, MEDIA_A, TRACK_A);
    insert_clip_at(&mut session, CLIP_A, TRACK_A, MEDIA_A, (0, 1), (1, 1));
    let revision_before = session.project_revision().value();

    let media_id = MediaId::from_str(MEDIA_A).unwrap();
    let clip_id = ClipId::from_str(CLIP_A).unwrap();
    session
        .execute_command(CommandEnvelope::update_timeline_clip(
            session.project_id(),
            session.project_instance_id(),
            session.project_revision(),
            clip_id,
            RationalTime::new(1, 2).unwrap(),
            ClipContent::Media {
                media_id,
                source_range: TimeRange::new(RationalTime::ZERO, RationalTime::new(1, 1).unwrap())
                    .unwrap(),
                playback_rate: RationalRate::new(2, 1).unwrap(),
                reversed: false,
                frozen: false,
            },
            ClipSettings::default_for(TrackKind::Video),
        ))
        .unwrap();

    let clip = &session.project().timeline().tracks()[0].clips()[0];
    assert_eq!(clip.timeline_duration(), RationalTime::new(1, 2).unwrap());
    assert_eq!(
        clip.content().playback_rate(),
        RationalRate::new(2, 1).unwrap()
    );
    assert_eq!(session.project_revision().value(), revision_before + 1);

    execute(&mut session, "history.undo", json!({})).unwrap();
    let clip = &session.project().timeline().tracks()[0].clips()[0];
    assert_eq!(clip.timeline_duration(), RationalTime::new(1, 1).unwrap());
    assert_eq!(clip.content().playback_rate(), RationalRate::ONE);

    execute(&mut session, "history.redo", json!({})).unwrap();
    let clip = &session.project().timeline().tracks()[0].clips()[0];
    assert_eq!(clip.timeline_duration(), RationalTime::new(1, 2).unwrap());
    assert_eq!(
        clip.content().playback_rate(),
        RationalRate::new(2, 1).unwrap()
    );
}

#[test]
fn timeline_clip_insert_sorts_exact_clips_and_undo_redo_restore_them() {
    let mut session = fixed_session();
    session
        .execute_command(media_add(
            media_item(MEDIA_A, "file:///missing/offline.mov"),
            0,
        ))
        .unwrap();
    add_track(&mut session, TRACK_A, "video").unwrap();
    insert_clip(
        &mut session,
        CLIP_B,
        TRACK_A,
        MEDIA_A,
        (4, 1),
        (3, 2),
        (2, 1),
    )
    .unwrap();
    let result = insert_clip(
        &mut session,
        CLIP_A,
        TRACK_A,
        MEDIA_A,
        (0, 1),
        (0, 1),
        (2, 1),
    )
    .unwrap();
    insert_clip(
        &mut session,
        CLIP_C,
        TRACK_A,
        MEDIA_A,
        (8, 1),
        (5, 2),
        (2, 1),
    )
    .unwrap();

    let clips = session.project().timeline().tracks()[0].clips();
    assert_eq!(
        clips
            .iter()
            .map(|clip| clip.id().to_string())
            .collect::<Vec<_>>(),
        [CLIP_A, CLIP_B, CLIP_C]
    );
    assert_eq!(clips[1].media_id().unwrap().to_string(), MEDIA_A);
    assert_eq!(clips[1].timeline_start(), RationalTime::new(4, 1).unwrap());
    assert_eq!(
        clips[1].source_range(),
        Some(
            TimeRange::new(
                RationalTime::new(3, 2).unwrap(),
                RationalTime::new(2, 1).unwrap()
            )
            .unwrap()
        )
    );
    assert_eq!(result.change_set.changes().len(), 1);
    assert_eq!(session.history.undo.last().unwrap().changes().len(), 1);
    let before_undo = session.project().timeline().clone();
    session
        .execute_command(undo(session.project_revision().value()))
        .unwrap();
    assert!(
        !session.project().timeline().tracks()[0]
            .clips()
            .iter()
            .any(|clip| clip.id().to_string() == CLIP_C)
    );
    session
        .execute_command(redo(session.project_revision().value()))
        .unwrap();
    assert_eq!(session.project().timeline(), &before_undo);
}

#[test]
fn timeline_clip_overlap_is_rejected_and_adjacency_is_allowed() {
    let mut session = fixed_session();
    session
        .execute_command(media_add(
            media_item(MEDIA_A, "file:///missing/offline.mov"),
            0,
        ))
        .unwrap();
    add_track(&mut session, TRACK_A, "video").unwrap();
    insert_clip(
        &mut session,
        CLIP_A,
        TRACK_A,
        MEDIA_A,
        (0, 1),
        (0, 1),
        (4, 1),
    )
    .unwrap();
    let before = session.project().clone();
    let before_revision = session.project_revision();
    let before_history = session.history.undo.clone();
    let error = insert_clip(
        &mut session,
        CLIP_B,
        TRACK_A,
        MEDIA_A,
        (3, 1),
        (0, 1),
        (2, 1),
    )
    .unwrap_err();
    assert_eq!(error.code, OperationErrorCode::TimelineOverlap);
    assert_eq!(session.project(), &before);
    assert_eq!(session.project_revision(), before_revision);
    assert_eq!(session.history.undo, before_history);

    insert_clip(
        &mut session,
        CLIP_B,
        TRACK_A,
        MEDIA_A,
        (4, 1),
        (0, 1),
        (2, 1),
    )
    .unwrap();
    assert_eq!(
        session.project().timeline().tracks()[0].clips()[1].timeline_start(),
        RationalTime::new(4, 1).unwrap()
    );
}

#[test]
fn timeline_clip_validation_uses_compatible_stream_and_known_duration_bounds() {
    let mut session = fixed_session();
    session
        .execute_command(media_add(
            video_media_item_with_duration(
                MEDIA_A,
                "file:///missing/video.mov",
                Some(RationalTime::new(10, 1).unwrap()),
                Some(RationalTime::new(5, 1).unwrap()),
            ),
            0,
        ))
        .unwrap();
    session
        .execute_command(media_add(
            audio_media_item(MEDIA_B, "file:///missing/audio.wav"),
            1,
        ))
        .unwrap();
    session
        .execute_command(media_add(
            video_media_item_with_duration(
                MEDIA_C,
                "file:///missing/container-duration.mov",
                Some(RationalTime::new(5, 1).unwrap()),
                None,
            ),
            2,
        ))
        .unwrap();
    session
        .execute_command(media_add(
            video_media_item_with_duration(MEDIA_D, "file:///missing/unknown.mov", None, None),
            3,
        ))
        .unwrap();
    add_track(&mut session, TRACK_A, "video").unwrap();
    add_track(&mut session, TRACK_B, "audio").unwrap();

    let missing = insert_clip(
        &mut session,
        CLIP_A,
        TRACK_A,
        "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
        (0, 1),
        (0, 1),
        (1, 1),
    )
    .unwrap_err();
    assert_eq!(missing.code, OperationErrorCode::MediaNotFound);

    for (track_id, media_id) in [(TRACK_A, MEDIA_B), (TRACK_B, MEDIA_A)] {
        let error = insert_clip(
            &mut session,
            CLIP_A,
            track_id,
            media_id,
            (0, 1),
            (0, 1),
            (1, 1),
        )
        .unwrap_err();
        assert_eq!(error.code, OperationErrorCode::TimelineMediaIncompatible);
    }

    let stream_overrun = insert_clip(
        &mut session,
        CLIP_A,
        TRACK_A,
        MEDIA_A,
        (0, 1),
        (0, 1),
        (6, 1),
    )
    .unwrap_err();
    assert_eq!(stream_overrun.code, OperationErrorCode::InvalidArguments);

    let container_overrun = insert_clip(
        &mut session,
        CLIP_A,
        TRACK_A,
        MEDIA_C,
        (0, 1),
        (4, 1),
        (2, 1),
    )
    .unwrap_err();
    assert_eq!(container_overrun.code, OperationErrorCode::InvalidArguments);

    insert_clip(
        &mut session,
        CLIP_A,
        TRACK_A,
        MEDIA_D,
        (0, 1),
        (100, 1),
        (2, 1),
    )
    .unwrap();
    insert_clip(
        &mut session,
        CLIP_B,
        TRACK_B,
        MEDIA_B,
        (0, 1),
        (0, 1),
        (2, 1),
    )
    .unwrap();
    assert_eq!(
        session.project().timeline().tracks()[0].clips()[0]
            .media_id()
            .unwrap()
            .to_string(),
        MEDIA_D
    );
}

#[test]
fn timeline_clip_move_rejects_both_cross_kind_directions_for_audio_video_media() {
    let mut session = fixed_session();
    session
        .execute_command(media_add(
            audio_video_media_item(MEDIA_A, "file:///missing/audio-video.mkv"),
            0,
        ))
        .unwrap();
    add_track(&mut session, TRACK_A, "video").unwrap();
    add_track(&mut session, TRACK_B, "audio").unwrap();
    insert_clip(
        &mut session,
        CLIP_A,
        TRACK_A,
        MEDIA_A,
        (0, 1),
        (0, 1),
        (1, 1),
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

    for (clip_id, destination) in [(CLIP_A, TRACK_B), (CLIP_B, TRACK_A)] {
        let before = session.project().clone();
        let before_undo = session.history.undo.clone();
        let before_revision = session.project_revision();
        let error = execute(
            &mut session,
            "timeline.clip.move",
            json!({
                "clip_id": clip_id,
                "track_id": destination,
                "timeline_start": rational_json(2, 1),
            }),
        )
        .unwrap_err();
        assert_eq!(error.code, OperationErrorCode::InvalidArguments);
        assert_eq!(session.project(), &before);
        assert_eq!(session.history.undo, before_undo);
        assert_eq!(session.project_revision(), before_revision);
    }
}

#[test]
fn timeline_clip_trim_uses_absolute_exact_edges_and_round_trips_history() {
    let mut session = fixed_session();
    session
        .execute_command(media_add(
            media_item(MEDIA_A, "file:///missing/offline.mov"),
            0,
        ))
        .unwrap();
    add_track(&mut session, TRACK_A, "video").unwrap();
    insert_clip(
        &mut session,
        CLIP_A,
        TRACK_A,
        MEDIA_A,
        (2, 1),
        (1, 1),
        (6, 1),
    )
    .unwrap();
    let original = session.project().timeline().clone();

    let start = execute(
        &mut session,
        "timeline.clip.trim",
        json!({
            "clip_id": CLIP_A,
            "edge": "start",
            "timeline_time": rational_json(3, 1),
        }),
    )
    .unwrap();
    assert!(matches!(
        start.change_set.changes(),
        [ProjectChange::TimelineClipTrimmed { before, after, .. }]
            if before.timeline_start == RationalTime::new(2, 1).unwrap()
                && after.timeline_start == RationalTime::new(3, 1).unwrap()
                && after.source_range().unwrap().start() == RationalTime::new(2, 1).unwrap()
                && after.source_range().unwrap().duration() == RationalTime::new(5, 1).unwrap()
    ));
    assert_eq!(
        start.after_revision,
        start.before_revision.checked_next().unwrap()
    );
    let trimmed = &session.project().timeline().tracks()[0].clips()[0];
    assert_eq!(trimmed.timeline_start(), RationalTime::new(3, 1).unwrap());
    assert_eq!(
        trimmed.source_range().unwrap().start(),
        RationalTime::new(2, 1).unwrap()
    );
    assert_eq!(
        trimmed.source_range().unwrap().duration(),
        RationalTime::new(5, 1).unwrap()
    );

    let end = execute(
        &mut session,
        "timeline.clip.trim",
        json!({
            "clip_id": CLIP_A,
            "edge": "end",
            "timeline_time": rational_json(7, 1),
        }),
    )
    .unwrap();
    assert!(end.changed);
    assert_eq!(
        session.project().timeline().tracks()[0].clips()[0].source_range(),
        Some(
            TimeRange::new(
                RationalTime::new(2, 1).unwrap(),
                RationalTime::new(4, 1).unwrap()
            )
            .unwrap()
        )
    );

    session
        .execute_command(undo(session.project_revision().value()))
        .unwrap();
    let no_op_revision = session.project_revision();
    let no_op_undo = session.history.undo.clone();
    let no_op_redo = session.history.redo.clone();
    let no_op = execute(
        &mut session,
        "timeline.clip.trim",
        json!({
            "clip_id": CLIP_A,
            "edge": "start",
            "timeline_time": rational_json(3, 1),
        }),
    )
    .unwrap();
    assert!(!no_op.changed);
    assert_eq!(session.project_revision(), no_op_revision);
    assert_eq!(session.history.undo, no_op_undo);
    assert_eq!(session.history.redo, no_op_redo);

    session
        .execute_command(undo(session.project_revision().value()))
        .unwrap();
    assert_eq!(session.project().timeline(), &original);
    session
        .execute_command(redo(session.project_revision().value()))
        .unwrap();
    assert_eq!(
        session.project().timeline().tracks()[0].clips()[0].timeline_start(),
        RationalTime::new(3, 1).unwrap()
    );
}

#[test]
fn timeline_clip_trim_rejects_unknown_fields_overlap_negative_source_and_bounds_atomically() {
    let mut session = fixed_session();
    session
        .execute_command(media_add(
            media_item(MEDIA_A, "file:///missing/offline.mov"),
            0,
        ))
        .unwrap();
    add_track(&mut session, TRACK_A, "video").unwrap();
    insert_clip(
        &mut session,
        CLIP_A,
        TRACK_A,
        MEDIA_A,
        (0, 1),
        (0, 1),
        (2, 1),
    )
    .unwrap();
    insert_clip(
        &mut session,
        CLIP_B,
        TRACK_A,
        MEDIA_A,
        (3, 1),
        (2, 1),
        (4, 1),
    )
    .unwrap();
    let before = session.project().clone();
    let before_undo = session.history.undo.clone();
    let overlap = execute(
        &mut session,
        "timeline.clip.trim",
        json!({
            "clip_id": CLIP_B,
            "edge": "start",
            "timeline_time": rational_json(1, 1),
        }),
    )
    .unwrap_err();
    assert_eq!(overlap.code, OperationErrorCode::TimelineOverlap);
    assert_eq!(session.project(), &before);
    assert_eq!(session.history.undo, before_undo);

    let negative_source = execute(
        &mut session,
        "timeline.clip.trim",
        json!({
            "clip_id": CLIP_B,
            "edge": "start",
            "timeline_time": rational_json(0, 1),
        }),
    )
    .unwrap_err();
    assert_eq!(negative_source.code, OperationErrorCode::InvalidArguments);
    let strict = execute(
        &mut session,
        "timeline.clip.trim",
        json!({
            "clip_id": CLIP_B,
            "edge": "end",
            "timeline_time": rational_json(8, 1),
            "extra": true,
        }),
    )
    .unwrap_err();
    assert_eq!(strict.code, OperationErrorCode::InvalidArguments);
}

#[test]
fn timeline_clip_trim_allows_exact_earlier_extension_and_enforces_source_bounds() {
    let mut session = fixed_session();
    session
        .execute_command(media_add(
            video_media_item_with_duration(
                MEDIA_A,
                "file:///missing/bounded.mov",
                Some(RationalTime::new(10, 1).unwrap()),
                Some(RationalTime::new(10, 1).unwrap()),
            ),
            0,
        ))
        .unwrap();
    add_track(&mut session, TRACK_A, "video").unwrap();
    insert_clip(
        &mut session,
        CLIP_A,
        TRACK_A,
        MEDIA_A,
        (2, 1),
        (4, 1),
        (4, 1),
    )
    .unwrap();

    let extended = execute(
        &mut session,
        "timeline.clip.trim",
        json!({
            "clip_id": CLIP_A,
            "edge": "start",
            "timeline_time": rational_json(3, 2),
        }),
    )
    .unwrap();
    assert!(extended.changed);
    let clip = &session.project().timeline().tracks()[0].clips()[0];
    assert_eq!(clip.timeline_start(), RationalTime::new(3, 2).unwrap());
    assert_eq!(
        clip.source_range().unwrap().start(),
        RationalTime::new(7, 2).unwrap()
    );
    assert_eq!(
        clip.source_range().unwrap().duration(),
        RationalTime::new(9, 2).unwrap()
    );

    let before = session.project().clone();
    let before_history = session.history.clone();
    let overrun = execute(
        &mut session,
        "timeline.clip.trim",
        json!({
            "clip_id": CLIP_A,
            "edge": "end",
            "timeline_time": rational_json(9, 1),
        }),
    )
    .unwrap_err();
    assert_eq!(overrun.code, OperationErrorCode::InvalidArguments);
    assert_eq!(session.project(), &before);
    assert_eq!(session.history, before_history);
}

#[test]
fn timeline_clip_split_partitions_exactly_and_restores_history() {
    let mut session = fixed_session();
    session
        .execute_command(media_add(
            media_item(MEDIA_A, "file:///missing/offline.mov"),
            0,
        ))
        .unwrap();
    add_track(&mut session, TRACK_A, "video").unwrap();
    insert_clip(
        &mut session,
        CLIP_A,
        TRACK_A,
        MEDIA_A,
        (2, 1),
        (1, 1),
        (6, 1),
    )
    .unwrap();
    let original = session.project().timeline().clone();
    let split = execute(
        &mut session,
        "timeline.clip.split",
        json!({
            "clip_id": CLIP_A,
            "new_clip_id": CLIP_B,
            "timeline_time": rational_json(5, 1),
        }),
    )
    .unwrap();
    assert!(matches!(
        split.change_set.changes(),
        [ProjectChange::TimelineClipSplit { before, left_after, right_after, .. }]
            if before.clip_id == left_after.clip_id
                && left_after.timeline_start == RationalTime::new(2, 1).unwrap()
                && left_after.source_range() == Some(TimeRange::new(RationalTime::new(1, 1).unwrap(), RationalTime::new(3, 1).unwrap()).unwrap())
                && right_after.clip_id == ClipId::from_str(CLIP_B).unwrap()
                && right_after.timeline_start == RationalTime::new(5, 1).unwrap()
                && right_after.source_range() == Some(TimeRange::new(RationalTime::new(4, 1).unwrap(), RationalTime::new(3, 1).unwrap()).unwrap())
    ));
    let clips = session.project().timeline().tracks()[0].clips();
    assert_eq!(clips.len(), 2);
    assert_eq!(clips[0].id().to_string(), CLIP_A);
    assert_eq!(clips[1].id().to_string(), CLIP_B);
    assert_eq!(
        clips[0].source_range().unwrap().duration(),
        RationalTime::new(3, 1).unwrap()
    );
    assert_eq!(
        clips[1].source_range().unwrap().start(),
        RationalTime::new(4, 1).unwrap()
    );
    assert_eq!(
        clips[1].source_range().unwrap().duration(),
        RationalTime::new(3, 1).unwrap()
    );

    session
        .execute_command(undo(session.project_revision().value()))
        .unwrap();
    assert_eq!(session.project().timeline(), &original);
    session
        .execute_command(redo(session.project_revision().value()))
        .unwrap();
    assert_eq!(session.project().timeline().tracks()[0].clips().len(), 2);

    let before = session.project().clone();
    let before_history = session.history.clone();
    for (time, expected) in [
        ((2, 1), OperationErrorCode::InvalidArguments),
        ((8, 1), OperationErrorCode::InvalidArguments),
    ] {
        let error = execute(
            &mut session,
            "timeline.clip.split",
            json!({
                "clip_id": CLIP_A,
                "new_clip_id": CLIP_C,
                "timeline_time": rational_json(time.0, time.1),
            }),
        )
        .unwrap_err();
        assert_eq!(error.code, expected);
        assert_eq!(session.project(), &before);
        assert_eq!(session.history, before_history);
    }
    let duplicate = execute(
        &mut session,
        "timeline.clip.split",
        json!({
            "clip_id": CLIP_A,
            "new_clip_id": CLIP_B,
            "timeline_time": rational_json(3, 1),
        }),
    )
    .unwrap_err();
    assert_eq!(
        duplicate.code,
        OperationErrorCode::TimelineClipIdAlreadyExists
    );
    let before_strict = session.project().clone();
    let before_strict_history = session.history.clone();
    let strict = execute(
        &mut session,
        "timeline.clip.split",
        json!({
            "clip_id": CLIP_A,
            "new_clip_id": CLIP_C,
            "timeline_time": rational_json(3, 1),
            "extra": true,
        }),
    )
    .unwrap_err();
    assert_eq!(strict.code, OperationErrorCode::InvalidArguments);
    assert_eq!(session.project(), &before_strict);
    assert_eq!(session.history, before_strict_history);
}

#[test]
fn timeline_clip_split_rejects_capacity_before_mutation() {
    let mut session = fixed_session();
    session
        .execute_command(media_add(
            media_item(MEDIA_A, "file:///missing/offline.mov"),
            0,
        ))
        .unwrap();
    let clips = (0..MAX_TIMELINE_CLIPS)
        .map(|index| {
            TimelineClip::from_parts_for_codec(
                if index == 0 {
                    ClipId::from_str(CLIP_A).unwrap()
                } else {
                    ClipId::generate()
                },
                MediaId::from_str(MEDIA_A).unwrap(),
                RationalTime::new(index as i64, 1).unwrap(),
                TimeRange::new(
                    RationalTime::new(0, 1).unwrap(),
                    RationalTime::new(1, 1).unwrap(),
                )
                .unwrap(),
            )
        })
        .collect();
    session
        .project
        .set_timeline_for_test(ProjectTimeline::from_tracks_for_codec(vec![fixture_track(
            TRACK_A,
            TrackKind::Video,
            clips,
        )]));
    let before = session.project().clone();
    let before_history = session.history.clone();
    let error = execute(
        &mut session,
        "timeline.clip.split",
        json!({
            "clip_id": CLIP_A,
            "new_clip_id": CLIP_B,
            "timeline_time": rational_json(1, 2),
        }),
    )
    .unwrap_err();
    assert_eq!(error.code, OperationErrorCode::TimelineLimitExceeded);
    assert_eq!(session.project(), &before);
    assert_eq!(session.history, before_history);
}

#[test]
fn timeline_clip_ripple_delete_shifts_only_a_track_and_keeps_a_compact_recipe() {
    let mut session = fixed_session();
    session
        .execute_command(media_add(
            media_item(MEDIA_A, "file:///missing/offline.mov"),
            0,
        ))
        .unwrap();
    add_track(&mut session, TRACK_A, "video").unwrap();
    add_track(&mut session, TRACK_B, "video").unwrap();
    insert_clip(
        &mut session,
        CLIP_A,
        TRACK_A,
        MEDIA_A,
        (0, 1),
        (0, 1),
        (2, 1),
    )
    .unwrap();
    insert_clip(
        &mut session,
        CLIP_B,
        TRACK_A,
        MEDIA_A,
        (4, 1),
        (2, 1),
        (2, 1),
    )
    .unwrap();
    insert_clip(
        &mut session,
        CLIP_C,
        TRACK_A,
        MEDIA_A,
        (9, 1),
        (4, 1),
        (1, 1),
    )
    .unwrap();
    insert_clip(
        &mut session,
        CLIP_D,
        TRACK_B,
        MEDIA_A,
        (1, 1),
        (5, 1),
        (1, 1),
    )
    .unwrap();
    let before = session.project().timeline().clone();

    let strict_before = session.project().clone();
    let strict_history = session.history.clone();
    let strict = execute(
        &mut session,
        "timeline.clip.ripple_delete",
        json!({"clip_id": CLIP_B, "extra": true}),
    )
    .unwrap_err();
    assert_eq!(strict.code, OperationErrorCode::InvalidArguments);
    assert_eq!(session.project(), &strict_before);
    assert_eq!(session.history, strict_history);

    let result = execute(
        &mut session,
        "timeline.clip.ripple_delete",
        json!({"clip_id": CLIP_B}),
    )
    .unwrap();
    assert!(matches!(
        result.change_set.changes(),
        [ProjectChange::TimelineClipRippleDeleted { index: 1, shifted_count: 1, shift_duration, .. }]
            if *shift_duration == RationalTime::new(2, 1).unwrap()
    ));
    let encoded_change = serde_json::to_string(&result.change_set).unwrap();
    assert!(encoded_change.contains("shifted_count"));
    assert!(!encoded_change.contains("clips"));
    let track = &session.project().timeline().tracks()[0];
    assert_eq!(track.clips().len(), 2);
    assert_eq!(track.clips()[0].id().to_string(), CLIP_A);
    assert_eq!(
        track.clips()[0].timeline_start(),
        RationalTime::new(0, 1).unwrap()
    );
    assert_eq!(track.clips()[1].id().to_string(), CLIP_C);
    assert_eq!(
        track.clips()[1].timeline_start(),
        RationalTime::new(7, 1).unwrap()
    );
    assert_eq!(
        session.project().timeline().tracks()[1].clips()[0]
            .id()
            .to_string(),
        CLIP_D
    );
    assert_eq!(
        session.project().timeline().tracks()[1].clips()[0].timeline_start(),
        RationalTime::new(1, 1).unwrap()
    );

    session
        .execute_command(undo(session.project_revision().value()))
        .unwrap();
    assert_eq!(session.project().timeline(), &before);
    session
        .execute_command(redo(session.project_revision().value()))
        .unwrap();
    assert_eq!(
        session.project().timeline().tracks()[0].clips()[1]
            .id()
            .to_string(),
        CLIP_C
    );
    assert_eq!(
        session.project().timeline().tracks()[0].clips()[1].timeline_start(),
        RationalTime::new(7, 1).unwrap()
    );
}

#[test]
fn timeline_clip_ripple_history_conflict_is_atomic_even_when_the_suffix_stays_valid() {
    let mut session = fixed_session();
    session
        .execute_command(media_add(
            media_item(MEDIA_A, "file:///missing/offline.mov"),
            0,
        ))
        .unwrap();
    add_track(&mut session, TRACK_A, "video").unwrap();
    insert_clip(
        &mut session,
        CLIP_A,
        TRACK_A,
        MEDIA_A,
        (0, 1),
        (0, 1),
        (2, 1),
    )
    .unwrap();
    insert_clip(
        &mut session,
        CLIP_B,
        TRACK_A,
        MEDIA_A,
        (4, 1),
        (2, 1),
        (2, 1),
    )
    .unwrap();
    insert_clip(
        &mut session,
        CLIP_C,
        TRACK_A,
        MEDIA_A,
        (9, 1),
        (4, 1),
        (1, 1),
    )
    .unwrap();
    execute(
        &mut session,
        "timeline.clip.ripple_delete",
        json!({"clip_id": CLIP_B}),
    )
    .unwrap();
    session
        .project
        .set_timeline_for_test(ProjectTimeline::from_tracks_for_codec(vec![fixture_track(
            TRACK_A,
            TrackKind::Video,
            vec![
                fixture_clip(CLIP_A, MEDIA_A, (0, 1), (0, 1), (2, 1)),
                fixture_clip(CLIP_C, MEDIA_A, (8, 1), (4, 1), (1, 1)),
            ],
        )]));
    assert_history_conflict(&mut session, "history.undo");
}

#[test]
fn timeline_clip_ids_are_project_wide_and_clip_limits_are_checked_before_mutation() {
    let mut session = fixed_session();
    session
        .execute_command(media_add(
            media_item(MEDIA_A, "file:///missing/offline.mov"),
            0,
        ))
        .unwrap();
    add_track(&mut session, TRACK_A, "video").unwrap();
    add_track(&mut session, TRACK_B, "video").unwrap();
    insert_clip(
        &mut session,
        CLIP_A,
        TRACK_A,
        MEDIA_A,
        (0, 1),
        (0, 1),
        (2, 1),
    )
    .unwrap();

    let before = session.project().clone();
    let before_undo = session.history.undo.clone();
    let duplicate = insert_clip(
        &mut session,
        CLIP_A,
        TRACK_B,
        MEDIA_A,
        (0, 1),
        (2, 1),
        (2, 1),
    )
    .unwrap_err();
    assert_eq!(
        duplicate.code,
        OperationErrorCode::TimelineClipIdAlreadyExists
    );
    assert_eq!(session.project(), &before);
    assert_eq!(session.history.undo, before_undo);

    let track_a =
        find_timeline_track_index(&session.project, TrackId::from_str(TRACK_A).unwrap()).unwrap();
    for (max_total, max_per_track) in [(0, 1), (1, 0)] {
        assert_eq!(
            ensure_timeline_clip_capacity(
                &session.project,
                track_a,
                1,
                1,
                max_total,
                max_per_track,
            )
            .unwrap_err()
            .code,
            OperationErrorCode::TimelineLimitExceeded
        );
    }
    assert_eq!(session.project(), &before);
    assert_eq!(session.history.undo, before_undo);
}

#[test]
fn timeline_clips_move_moves_a_selection_in_one_step_and_one_undo() {
    let mut session = fixed_session();
    session
        .execute_command(media_add(
            media_item(MEDIA_A, "file:///missing/offline.mov"),
            0,
        ))
        .unwrap();
    add_track(&mut session, TRACK_A, "video").unwrap();
    insert_clip(
        &mut session,
        CLIP_A,
        TRACK_A,
        MEDIA_A,
        (0, 1),
        (0, 1),
        (1, 1),
    )
    .unwrap();
    insert_clip(
        &mut session,
        CLIP_B,
        TRACK_A,
        MEDIA_A,
        (2, 1),
        (1, 1),
        (1, 1),
    )
    .unwrap();
    let before = clip_spans(&session, TRACK_A);

    let moved = execute(
        &mut session,
        "timeline.clips.move",
        json!({
            "track_id": TRACK_A,
            "clip_ids": [CLIP_A, CLIP_B],
            "delta": rational_json(1, 1),
        }),
    )
    .unwrap();
    assert!(moved.changed);
    assert_eq!(moved.change_set.changes().len(), 1);
    let after = clip_spans(&session, TRACK_A);
    assert_eq!(after[0].1, RationalTime::new(1, 1).unwrap());
    assert_eq!(after[1].1, RationalTime::new(3, 1).unwrap());

    execute(&mut session, "history.undo", json!({})).unwrap();
    assert_eq!(
        clip_spans(&session, TRACK_A),
        before,
        "one undo restores the whole selection"
    );
}

#[test]
fn timeline_clips_move_rejects_unknown_clip_and_a_start_below_zero() {
    let mut session = fixed_session();
    session
        .execute_command(media_add(
            media_item(MEDIA_A, "file:///missing/offline.mov"),
            0,
        ))
        .unwrap();
    add_track(&mut session, TRACK_A, "video").unwrap();
    insert_clip(
        &mut session,
        CLIP_A,
        TRACK_A,
        MEDIA_A,
        (0, 1),
        (0, 1),
        (1, 1),
    )
    .unwrap();

    // A malformed id never reaches the handler: the envelope rejects it.
    let malformed = execute(
        &mut session,
        "timeline.clips.move",
        json!({
            "track_id": TRACK_A,
            "clip_ids": ["missing-clip"],
            "delta": rational_json(1, 1),
        }),
    );
    assert_eq!(
        malformed.unwrap_err().code,
        OperationErrorCode::InvalidArguments
    );

    // A well-formed id that is not on the track is the handler's own error.
    let unknown = execute(
        &mut session,
        "timeline.clips.move",
        json!({
            "track_id": TRACK_A,
            "clip_ids": ["00000000-0000-4000-8000-000000000000"],
            "delta": rational_json(1, 1),
        }),
    );
    assert_eq!(
        unknown.unwrap_err().code,
        OperationErrorCode::TimelineClipNotFound
    );

    let behind_zero = execute(
        &mut session,
        "timeline.clips.move",
        json!({
            "track_id": TRACK_A,
            "clip_ids": [CLIP_A],
            "delta": rational_json(-1, 1),
        }),
    );
    assert!(behind_zero.is_err());
}

#[test]
fn timeline_clip_move_reorders_and_preserves_semantics_across_same_kind_tracks() {
    let mut session = fixed_session();
    session
        .execute_command(media_add(
            media_item(MEDIA_A, "file:///missing/offline.mov"),
            0,
        ))
        .unwrap();
    add_track(&mut session, TRACK_A, "video").unwrap();
    add_track(&mut session, TRACK_B, "video").unwrap();
    add_track(&mut session, TRACK_C, "audio").unwrap();
    insert_clip(
        &mut session,
        CLIP_A,
        TRACK_A,
        MEDIA_A,
        (0, 1),
        (1, 2),
        (2, 1),
    )
    .unwrap();
    insert_clip(
        &mut session,
        CLIP_B,
        TRACK_A,
        MEDIA_A,
        (4, 1),
        (3, 2),
        (2, 1),
    )
    .unwrap();
    insert_clip(
        &mut session,
        CLIP_C,
        TRACK_A,
        MEDIA_A,
        (8, 1),
        (5, 2),
        (2, 1),
    )
    .unwrap();

    let moved = execute(
        &mut session,
        "timeline.clip.move",
        json!({"clip_id": CLIP_A, "track_id": TRACK_A, "timeline_start": rational_json(10, 1)}),
    )
    .unwrap();
    assert!(moved.changed);
    assert_eq!(
        moved.after_revision.value(),
        moved.before_revision.value() + 1
    );
    assert_eq!(
        session.project().timeline().tracks()[0]
            .clips()
            .iter()
            .map(|clip| clip.id().to_string())
            .collect::<Vec<_>>(),
        [CLIP_B, CLIP_C, CLIP_A]
    );
    let moved_clip = TimelineClipState::from(&session.project().timeline().tracks()[0].clips()[2]);
    assert_eq!(moved_clip.media_id().unwrap().to_string(), MEDIA_A);
    assert_eq!(
        moved_clip.source_range().unwrap().start(),
        RationalTime::new(1, 2).unwrap()
    );
    assert_eq!(
        moved_clip.source_range().unwrap().duration(),
        RationalTime::new(2, 1).unwrap()
    );
    assert_eq!(moved_clip.timeline_start, RationalTime::new(10, 1).unwrap());

    session
        .execute_command(undo(session.project_revision().value()))
        .unwrap();
    assert_eq!(
        session.project().timeline().tracks()[0]
            .clips()
            .iter()
            .map(|clip| clip.id().to_string())
            .collect::<Vec<_>>(),
        [CLIP_A, CLIP_B, CLIP_C]
    );
    session
        .execute_command(redo(session.project_revision().value()))
        .unwrap();
    assert_eq!(
        session.project().timeline().tracks()[0].clips()[2].timeline_start(),
        RationalTime::new(10, 1).unwrap()
    );

    execute(
        &mut session,
        "timeline.clip.move",
        json!({"clip_id": CLIP_B, "track_id": TRACK_B, "timeline_start": rational_json(10, 1)}),
    )
    .unwrap();
    let destination = &session.project().timeline().tracks()[1].clips()[0];
    assert_eq!(destination.id().to_string(), CLIP_B);
    assert_eq!(destination.media_id().unwrap().to_string(), MEDIA_A);
    assert_eq!(
        destination.source_range().unwrap().start(),
        RationalTime::new(3, 2).unwrap()
    );
    assert_eq!(
        destination.timeline_start(),
        RationalTime::new(10, 1).unwrap()
    );
    assert!(
        session.project().timeline().tracks()[0]
            .clips()
            .iter()
            .all(|clip| clip.id().to_string() != CLIP_B)
    );
    session
        .execute_command(undo(session.project_revision().value()))
        .unwrap();
    assert_eq!(
        session.project().timeline().tracks()[0].clips()[0]
            .id()
            .to_string(),
        CLIP_B
    );
    assert!(session.project().timeline().tracks()[1].clips().is_empty());
    session
        .execute_command(redo(session.project_revision().value()))
        .unwrap();
    assert_eq!(
        session.project().timeline().tracks()[1].clips()[0]
            .id()
            .to_string(),
        CLIP_B
    );
    let before_media_remove = session.project().clone();
    assert_eq!(
        code(&session.execute_command(media_remove(MEDIA_A, session.project_revision().value(),))),
        OperationErrorCode::MediaInUse
    );
    assert_eq!(session.project(), &before_media_remove);

    let before = session.project().clone();
    let before_undo = session.history.undo.clone();
    let cross_kind = execute(
        &mut session,
        "timeline.clip.move",
        json!({"clip_id": CLIP_B, "track_id": TRACK_C, "timeline_start": rational_json(0, 1)}),
    )
    .unwrap_err();
    assert_eq!(cross_kind.code, OperationErrorCode::InvalidArguments);
    assert_eq!(session.project(), &before);
    assert_eq!(session.history.undo, before_undo);
}

#[test]
fn timeline_clip_move_overlap_failure_and_no_op_preserve_redo() {
    let mut session = fixed_session();
    session
        .execute_command(media_add(
            media_item(MEDIA_A, "file:///missing/offline.mov"),
            0,
        ))
        .unwrap();
    add_track(&mut session, TRACK_A, "video").unwrap();
    insert_clip(
        &mut session,
        CLIP_A,
        TRACK_A,
        MEDIA_A,
        (0, 1),
        (0, 1),
        (2, 1),
    )
    .unwrap();
    insert_clip(
        &mut session,
        CLIP_B,
        TRACK_A,
        MEDIA_A,
        (4, 1),
        (2, 1),
        (2, 1),
    )
    .unwrap();
    execute(
        &mut session,
        "timeline.clip.move",
        json!({"clip_id": CLIP_B, "track_id": TRACK_A, "timeline_start": rational_json(6, 1)}),
    )
    .unwrap();
    session
        .execute_command(undo(session.project_revision().value()))
        .unwrap();
    assert_eq!(session.history.redo.len(), 1);
    let before = session.project().clone();
    let before_undo = session.history.undo.clone();
    let before_redo = session.history.redo.clone();
    let before_revision = session.project_revision();

    let overlap = execute(
        &mut session,
        "timeline.clip.move",
        json!({"clip_id": CLIP_B, "track_id": TRACK_A, "timeline_start": rational_json(1, 1)}),
    )
    .unwrap_err();
    assert_eq!(overlap.code, OperationErrorCode::TimelineOverlap);
    assert_eq!(session.project(), &before);
    assert_eq!(session.project_revision(), before_revision);
    assert_eq!(session.history.undo, before_undo);
    assert_eq!(session.history.redo, before_redo);

    let no_op = execute(
        &mut session,
        "timeline.clip.move",
        json!({"clip_id": CLIP_B, "track_id": TRACK_A, "timeline_start": rational_json(4, 1)}),
    )
    .unwrap();
    assert!(!no_op.changed);
    assert!(no_op.change_set.is_empty());
    assert_eq!(no_op.before_revision, before_revision);
    assert_eq!(no_op.after_revision, before_revision);
    assert_eq!(session.history.undo, before_undo);
    assert_eq!(session.history.redo, before_redo);

    let mut stale_no_op = history_command(
        "timeline.clip.move",
        before_revision.value().saturating_sub(1),
        json!({"clip_id": CLIP_B, "track_id": TRACK_A, "timeline_start": rational_json(4, 1)}),
    );
    stale_no_op.project_id = session.project_id();
    assert_eq!(
        code(&session.execute_command(stale_no_op)),
        OperationErrorCode::RevisionConflict
    );
    assert_eq!(session.history.redo, before_redo);

    session
        .execute_command(redo(session.project_revision().value()))
        .unwrap();
    assert_eq!(
        session.project().timeline().tracks()[0].clips()[1].timeline_start(),
        RationalTime::new(6, 1).unwrap()
    );
}

#[test]
fn timeline_clip_delete_restores_exact_clip_and_media_remove_guard_is_preserved() {
    let mut session = fixed_session();
    seed_video_track_and_clip(&mut session, MEDIA_A, TRACK_A, CLIP_A, (0, 1), (2, 1));
    insert_clip(
        &mut session,
        CLIP_B,
        TRACK_A,
        MEDIA_A,
        (4, 1),
        (2, 1),
        (2, 1),
    )
    .unwrap();
    insert_clip(
        &mut session,
        CLIP_C,
        TRACK_A,
        MEDIA_A,
        (8, 1),
        (4, 1),
        (2, 1),
    )
    .unwrap();
    let expected = TimelineClipState::from(&session.project().timeline().tracks()[0].clips()[1]);
    let before_remove = session.project().clone();
    assert_eq!(
        code(&session.execute_command(media_remove(MEDIA_A, session.project_revision().value()))),
        OperationErrorCode::MediaInUse
    );
    assert_eq!(session.project(), &before_remove);

    let deleted = execute(
        &mut session,
        "timeline.clip.delete",
        json!({"clip_id": CLIP_B}),
    )
    .unwrap();
    assert!(deleted.changed);
    assert_eq!(deleted.change_set.changes().len(), 1);
    assert_eq!(
        session.project().timeline().tracks()[0]
            .clips()
            .iter()
            .map(|clip| clip.id().to_string())
            .collect::<Vec<_>>(),
        [CLIP_A, CLIP_C]
    );
    assert_eq!(session.project().media_items().len(), 1);
    session
        .execute_command(undo(session.project_revision().value()))
        .unwrap();
    assert_eq!(
        TimelineClipState::from(&session.project().timeline().tracks()[0].clips()[1]),
        expected
    );
    assert_eq!(
        session.project().timeline().tracks()[0]
            .clips()
            .iter()
            .map(|clip| clip.id().to_string())
            .collect::<Vec<_>>(),
        [CLIP_A, CLIP_B, CLIP_C]
    );
    session
        .execute_command(redo(session.project_revision().value()))
        .unwrap();
    assert_eq!(
        session.project().timeline().tracks()[0]
            .clips()
            .iter()
            .map(|clip| clip.id().to_string())
            .collect::<Vec<_>>(),
        [CLIP_A, CLIP_C]
    );
    let still_in_use = session
        .execute_command(media_remove(MEDIA_A, session.project_revision().value()))
        .unwrap_err();
    assert_eq!(still_in_use.code, OperationErrorCode::MediaInUse);
    execute(
        &mut session,
        "timeline.clip.delete",
        json!({"clip_id": CLIP_A}),
    )
    .unwrap();
    execute(
        &mut session,
        "timeline.clip.delete",
        json!({"clip_id": CLIP_C}),
    )
    .unwrap();
    assert!(session.project().timeline().tracks()[0].clips().is_empty());
    let removed_media = session
        .execute_command(media_remove(MEDIA_A, session.project_revision().value()))
        .unwrap();
    assert!(removed_media.changed);
    assert!(session.project().media_items().is_empty());
}

#[test]
fn video_visual_settings_update_validation_history_and_serialization_are_exact() {
    let mut session = fixed_session();
    let media_id = MediaId::from_str(MEDIA_A).unwrap();
    let track_id = TrackId::from_str(TRACK_A).unwrap();
    let clip_id = ClipId::from_str(CLIP_A).unwrap();
    session
        .execute_command(media_add(
            media_item(MEDIA_A, "file:///missing/transform.mov"),
            0,
        ))
        .unwrap();
    add_track(&mut session, TRACK_A, "video").unwrap();

    let content = ClipContent::Media {
        media_id,
        source_range: TimeRange::new(
            RationalTime::new(1, 4).unwrap(),
            RationalTime::new(2, 1).unwrap(),
        )
        .unwrap(),
        playback_rate: RationalRate::ONE,
        reversed: false,
        frozen: false,
    };
    let duration = RationalTime::new(2, 1).unwrap();
    session
        .execute_command(CommandEnvelope::insert_timeline_clip_content(
            session.project_id(),
            session.project_instance_id(),
            session.project_revision(),
            clip_id,
            track_id,
            RationalTime::new(3, 2).unwrap(),
            duration,
            content.clone(),
            ClipSettings::Visual(VisualSettings::default()),
        ))
        .unwrap();

    let visual = VisualSettings {
        transform: Transform {
            x_milli_canvas: 250,
            y_milli_canvas: -125,
            scale_x_milli: 1_500,
            scale_y_milli: 750,
            rotation_milli_degrees: 15_000,
            anchor_x_basis_points: 2_500,
            anchor_y_basis_points: 7_500,
        },
        crop: Crop {
            left_basis_points: 1_000,
            top_basis_points: 2_000,
            right_basis_points: 500,
            bottom_basis_points: 1_500,
        },
        opacity: Opacity {
            basis_points: 6_250,
        },
        ..VisualSettings::default()
    };
    session
        .execute_command(CommandEnvelope::update_timeline_clip(
            session.project_id(),
            session.project_instance_id(),
            session.project_revision(),
            clip_id,
            duration,
            content.clone(),
            ClipSettings::Visual(visual.clone()),
        ))
        .unwrap();

    let read_settings = |session: &ProjectSession| {
        session
            .execute_query(QueryEnvelope::timeline_clips_v2(
                session.project_id(),
                session.project_instance_id(),
                track_id,
                0,
                10,
            ))
            .unwrap()
            .timeline_clip_page_v2
            .unwrap()
            .items
            .remove(0)
    };
    let updated = read_settings(&session);
    assert_eq!(updated.timeline_start, RationalTime::new(3, 2).unwrap());
    assert_eq!(updated.timeline_duration, duration);
    assert_eq!(updated.content, content);
    assert_eq!(updated.settings, ClipSettings::Visual(visual.clone()));

    let invalid_before = session.project().clone();
    let invalid_history = session.history.clone();
    let mut invalid_visual = visual.clone();
    invalid_visual.transform.scale_x_milli = 0;
    let error = session
        .execute_command(CommandEnvelope::update_timeline_clip(
            session.project_id(),
            session.project_instance_id(),
            session.project_revision(),
            clip_id,
            duration,
            content.clone(),
            ClipSettings::Visual(invalid_visual),
        ))
        .unwrap_err();
    assert_eq!(error.code, OperationErrorCode::InvalidArguments);
    assert_eq!(session.project(), &invalid_before);
    assert_eq!(session.history, invalid_history);

    let decoded = decode_project(&encode_project(session.project()).unwrap()).unwrap();
    assert_eq!(
        decoded.timeline().tracks()[0].clips()[0].settings(),
        &ClipSettings::Visual(visual.clone())
    );

    session
        .execute_command(CommandEnvelope::undo(
            session.project_id(),
            session.project_instance_id(),
            session.project_revision(),
        ))
        .unwrap();
    assert_eq!(
        read_settings(&session).settings,
        ClipSettings::Visual(VisualSettings::default())
    );
    session
        .execute_command(CommandEnvelope::redo(
            session.project_id(),
            session.project_instance_id(),
            session.project_revision(),
        ))
        .unwrap();
    assert_eq!(
        read_settings(&session).settings,
        ClipSettings::Visual(visual)
    );
}

#[test]
fn timeline_arguments_are_strict_and_invalid_times_never_mutate() {
    let mut session = fixed_session();
    session
        .execute_command(media_add(
            media_item(MEDIA_A, "file:///missing/offline.mov"),
            0,
        ))
        .unwrap();
    add_track(&mut session, TRACK_A, "video").unwrap();
    let base = json!({
        "clip_id": CLIP_A,
        "track_id": TRACK_A,
        "media_id": MEDIA_A,
        "timeline_start": {"numerator": 0, "denominator": 1},
        "source_range": {
            "start": {"numerator": 0, "denominator": 1},
            "duration": {"numerator": 2, "denominator": 1},
        }
    });
    let mut invalids = vec![json!({})];
    let mut invalid = base.clone();
    invalid.as_object_mut().unwrap().remove("clip_id");
    invalids.push(invalid);
    let mut invalid = base.clone();
    invalid["unknown"] = json!(true);
    invalids.push(invalid);
    let mut invalid = base.clone();
    invalid["timeline_start"] = json!("0/1");
    invalids.push(invalid);
    let mut invalid = base.clone();
    invalid["timeline_start"]["extra"] = json!(1);
    invalids.push(invalid);
    let mut invalid = base.clone();
    invalid["source_range"]["start"]["denominator"] = json!(0);
    invalids.push(invalid);
    let mut invalid = base.clone();
    invalid["clip_id"] = json!("bad");
    invalids.push(invalid);
    let mut invalid = base.clone();
    invalid["track_id"] = json!("bad");
    invalids.push(invalid);
    let mut invalid = base.clone();
    invalid["media_id"] = json!("bad");
    invalids.push(invalid);
    for field in ["timeline_start", "source_range.start"] {
        let mut invalid = base.clone();
        let time = if field == "timeline_start" {
            &mut invalid["timeline_start"]
        } else {
            &mut invalid["source_range"]["start"]
        };
        time["numerator"] = json!(-1);
        invalids.push(invalid);
    }
    for duration in [0, -1] {
        let mut invalid = base.clone();
        invalid["source_range"]["duration"]["numerator"] = json!(duration);
        invalids.push(invalid);
    }
    for field in ["timeline_start", "source_range.start"] {
        let mut invalid = base.clone();
        let time = if field == "timeline_start" {
            &mut invalid["timeline_start"]
        } else {
            &mut invalid["source_range"]["start"]
        };
        time["numerator"] = json!(i64::MAX);
        invalids.push(invalid);
    }

    let before = session.project().clone();
    let undo = session.history.undo.clone();
    for arguments in invalids {
        assert_eq!(
            code(&execute(&mut session, "timeline.clip.insert", arguments)),
            OperationErrorCode::InvalidArguments
        );
        assert_eq!(session.project(), &before);
        assert_eq!(session.history.undo, undo);
    }
}

#[test]
fn every_timeline_mutation_is_rejected_from_grouped_transactions() {
    for command_id in [
        "timeline.track.add",
        "timeline.track.remove",
        "timeline.track.move",
        "timeline.clip.insert",
        "timeline.clip.move",
        "timeline.clip.delete",
        "timeline.clip.trim",
        "timeline.clip.slip",
        "timeline.clip.roll",
        "timeline.clip.slide",
        "timeline.clip.split",
        "timeline.clip.duck",
        "timeline.clip.ripple_delete",
    ] {
        let mut session = fixed_session();
        let before = session.project().clone();
        let error = session
            .execute_transaction(transaction(vec![call(command_id, 1, json!({}))], 0))
            .unwrap_err();
        assert_eq!(
            error.code,
            OperationErrorCode::CommandNotAllowedInTransaction
        );
        assert_eq!(session.project(), &before);
        assert!(session.history.undo.is_empty());
        assert!(session.history.redo.is_empty());
    }
}

#[test]
fn representative_timeline_mutation_checks_project_instance_and_revision_preconditions() {
    let mut session = fixed_session();
    let mut command = history_command(
        "timeline.track.add",
        0,
        json!({"track_id": TRACK_A, "kind": "video"}),
    );
    command.project_id = ProjectId::from_str(OTHER_PROJECT_ID).unwrap();
    assert_eq!(
        code(&session.execute_command(command.clone())),
        OperationErrorCode::ProjectIdMismatch
    );
    command.project_id = session.project_id();
    command.project_instance_id = ProjectInstanceId::from_str(OTHER_INSTANCE_ID).unwrap();
    assert_eq!(
        code(&session.execute_command(command.clone())),
        OperationErrorCode::ProjectInstanceMismatch
    );
    command.project_instance_id = session.project_instance_id();
    command.expected_project_revision = ProjectRevision::new(1);
    assert_eq!(
        code(&session.execute_command(command)),
        OperationErrorCode::RevisionConflict
    );
    assert!(session.project().timeline().tracks().is_empty());
    assert_eq!(session.project_revision(), ProjectRevision::INITIAL);
    assert!(session.history.undo.is_empty());
}

#[test]
fn every_timeline_edit_rejects_revision_overflow_before_mutation() {
    let mut add = session_with_state("A", u64::MAX);
    let add_before = add.project().clone();
    assert_eq!(
        code(&add.execute_command(history_command(
            "timeline.track.add",
            u64::MAX,
            json!({"track_id": TRACK_A, "kind": "video"}),
        ))),
        OperationErrorCode::RevisionOverflow
    );
    assert_eq!(add.project(), &add_before);

    let mut remove = session_with_state("A", u64::MAX);
    remove
        .project
        .set_timeline_for_test(ProjectTimeline::from_tracks_for_codec(vec![fixture_track(
            TRACK_A,
            TrackKind::Video,
            Vec::new(),
        )]));
    let remove_before = remove.project().clone();
    assert_eq!(
        code(&execute(
            &mut remove,
            "timeline.track.remove",
            json!({"track_id": TRACK_A})
        )),
        OperationErrorCode::RevisionOverflow
    );
    assert_eq!(remove.project(), &remove_before);

    let mut insert = max_revision_session_with_clip(false);
    let insert_before = insert.project().clone();
    assert_eq!(
        code(&insert_clip(
            &mut insert,
            CLIP_B,
            TRACK_A,
            MEDIA_A,
            (4, 1),
            (2, 1),
            (2, 1)
        )),
        OperationErrorCode::RevisionOverflow
    );
    assert_eq!(insert.project(), &insert_before);

    let mut move_clip = max_revision_session_with_clip(true);
    let move_before = move_clip.project().clone();
    assert_eq!(
        code(&execute(
            &mut move_clip,
            "timeline.clip.move",
            json!({"clip_id": CLIP_A, "track_id": TRACK_A, "timeline_start": rational_json(4, 1)}),
        )),
        OperationErrorCode::RevisionOverflow
    );
    assert_eq!(move_clip.project(), &move_before);

    let mut delete_clip = max_revision_session_with_clip(true);
    let delete_before = delete_clip.project().clone();
    assert_eq!(
        code(&execute(
            &mut delete_clip,
            "timeline.clip.delete",
            json!({"clip_id": CLIP_A})
        )),
        OperationErrorCode::RevisionOverflow
    );
    assert_eq!(delete_clip.project(), &delete_before);
    let mut trim_clip = max_revision_session_with_clip(true);
    let trim_before = trim_clip.project().clone();
    assert_eq!(
        code(&execute(
            &mut trim_clip,
            "timeline.clip.trim",
            json!({
                "clip_id": CLIP_A,
                "edge": "end",
                "timeline_time": rational_json(1, 1),
            }),
        )),
        OperationErrorCode::RevisionOverflow
    );
    assert_eq!(trim_clip.project(), &trim_before);

    let mut split_clip = max_revision_session_with_clip(true);
    let split_before = split_clip.project().clone();
    assert_eq!(
        code(&execute(
            &mut split_clip,
            "timeline.clip.split",
            json!({
                "clip_id": CLIP_A,
                "new_clip_id": CLIP_B,
                "timeline_time": rational_json(1, 1),
            }),
        )),
        OperationErrorCode::RevisionOverflow
    );
    assert_eq!(split_clip.project(), &split_before);

    let mut ripple_delete_clip = max_revision_session_with_clip(true);
    let ripple_delete_before = ripple_delete_clip.project().clone();
    assert_eq!(
        code(&execute(
            &mut ripple_delete_clip,
            "timeline.clip.ripple_delete",
            json!({"clip_id": CLIP_A}),
        )),
        OperationErrorCode::RevisionOverflow
    );
    assert_eq!(ripple_delete_clip.project(), &ripple_delete_before);

    for session in [
        &add,
        &remove,
        &insert,
        &move_clip,
        &delete_clip,
        &trim_clip,
        &split_clip,
        &ripple_delete_clip,
    ] {
        assert_eq!(session.project_revision(), ProjectRevision::new(u64::MAX));
        assert!(session.history.undo.is_empty());
        assert!(session.history.redo.is_empty());
    }
}
