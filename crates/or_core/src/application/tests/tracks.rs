// Track add/remove/move/limit and caption import.

use super::super::*;
use super::support::*;
use super::*;
use serde_json::json;
use std::str::FromStr;

#[test]
fn moving_a_track_reorders_it_without_touching_its_clips() {
    let mut session = session_with_state("track-move", 0);
    seed_media_and_track(&mut session, MEDIA_A, TRACK_A);
    add_track(&mut session, TRACK_B, "audio").unwrap();
    let third = "77777777-7777-4777-8777-777777777777";
    add_track(&mut session, third, "video").unwrap();
    insert_clip_at(&mut session, CLIP_A, TRACK_A, MEDIA_A, (0, 1), (1, 1));
    let clips_before = clip_spans(&session, TRACK_A);
    let revision_before = session.project_revision();

    // Move the first track to the end.
    move_track(&mut session, TRACK_A, 2).unwrap();

    assert_eq!(
        track_order(&session),
        vec![TRACK_B.to_owned(), third.to_owned(), TRACK_A.to_owned()],
        "the other tracks shift up by one"
    );
    assert_eq!(
        clip_spans(&session, TRACK_A),
        clips_before,
        "the moved track keeps its clips"
    );
    assert_eq!(
        session.project_revision().value(),
        revision_before.value() + 1
    );

    execute(&mut session, "history.undo", json!({})).unwrap();
    assert_eq!(
        track_order(&session),
        vec![TRACK_A.to_owned(), TRACK_B.to_owned(), third.to_owned()],
        "one undo puts the track back"
    );
    assert_eq!(clip_spans(&session, TRACK_A), clips_before);

    // Redo must work too: a move is its own inverse, and the inverse change is
    // what the history entry records.
    execute(&mut session, "history.redo", json!({})).unwrap();
    assert_eq!(
        track_order(&session),
        vec![TRACK_B.to_owned(), third.to_owned(), TRACK_A.to_owned()]
    );
}

#[test]
fn moving_a_track_to_its_own_index_is_a_no_op() {
    let mut session = session_with_state("track-move-noop", 0);
    seed_media_and_track(&mut session, MEDIA_A, TRACK_A);
    add_track(&mut session, TRACK_B, "audio").unwrap();
    let revision_before = session.project_revision();

    let result = move_track(&mut session, TRACK_A, 0).unwrap();
    assert!(!result.changed);
    assert_eq!(session.project_revision().value(), revision_before.value());
}

#[test]
fn moving_a_track_past_the_end_or_an_unknown_track_is_refused() {
    let mut session = session_with_state("track-move-refused", 0);
    seed_media_and_track(&mut session, MEDIA_A, TRACK_A);
    add_track(&mut session, TRACK_B, "audio").unwrap();
    let order_before = track_order(&session);
    let revision_before = session.project_revision();

    let error = move_track(&mut session, TRACK_A, 2).unwrap_err();
    assert_eq!(
        error.code,
        super::super::OperationErrorCode::InvalidArguments
    );
    let unknown = "99999999-9999-4999-8999-999999999999";
    let error = move_track(&mut session, unknown, 0).unwrap_err();
    assert_eq!(
        error.code,
        super::super::OperationErrorCode::TimelineTrackNotFound
    );
    assert_eq!(track_order(&session), order_before);
    assert_eq!(session.project_revision().value(), revision_before.value());
}

/// A media clip without an explicit rate still plays at identity speed, so its
/// source window and timeline duration must be exactly equal.

#[test]
fn timeline_track_add_preserves_requested_id_kind_revision_and_history() {
    let mut session = fixed_session();
    let result = add_track(&mut session, TRACK_A, "video").unwrap();

    assert!(result.changed);
    assert_eq!(result.before_revision, ProjectRevision::new(0));
    assert_eq!(result.after_revision, ProjectRevision::new(1));
    assert!(matches!(
        result.change_set.changes(),
        [ProjectChange::TimelineTrackAdded {
            track_id,
            track_kind: TrackKind::Video,
            index: 0,
            ..
        }] if *track_id == TrackId::from_str(TRACK_A).unwrap()
    ));
    assert_eq!(session.project().timeline().tracks().len(), 1);
    assert_eq!(
        session.project().timeline().tracks()[0].id().to_string(),
        TRACK_A
    );
    assert_eq!(
        session.project().timeline().tracks()[0].kind(),
        TrackKind::Video
    );
    assert!(session.project().timeline().tracks()[0].clips().is_empty());
    assert_eq!(session.history.undo.len(), 1);
    assert!(session.history.undo[0].changes().len() == 1);
    assert!(session.history.redo.is_empty());

    session.execute_command(undo(1)).unwrap();
    assert!(session.project().timeline().tracks().is_empty());
    assert_eq!(session.project_revision(), ProjectRevision::new(2));
    session.execute_command(redo(2)).unwrap();
    assert_eq!(
        session.project().timeline().tracks()[0].id().to_string(),
        TRACK_A
    );
    assert_eq!(session.project_revision(), ProjectRevision::new(3));
}

#[test]
fn caption_track_import_is_one_atomic_revision_and_one_undo_entry() {
    let mut session = fixed_session();
    let track_id = TrackId::from_str(TRACK_A).unwrap();
    let clips = vec![
        caption_clip_state(CLIP_A, (125, 1000), (9, 4), "First caption"),
        caption_clip_state(CLIP_B, (5, 2), (1, 2), "Second caption"),
    ];
    let result = session
        .execute_command(CommandEnvelope::import_timeline_captions(
            session.project_id(),
            session.project_instance_id(),
            session.project_revision(),
            track_id,
            clips,
        ))
        .unwrap();

    assert_eq!(result.before_revision, ProjectRevision::new(0));
    assert_eq!(result.after_revision, ProjectRevision::new(1));
    assert!(matches!(
        result.change_set.changes(),
        [ProjectChange::TimelineCaptionTrackAdded { track_id: added_id, clips, .. }]
            if *added_id == track_id && clips.len() == 2
    ));
    assert_eq!(session.project().timeline().tracks().len(), 1);
    let track = &session.project().timeline().tracks()[0];
    assert_eq!(track.id(), track_id);
    assert_eq!(track.kind(), TrackKind::Caption);
    assert_eq!(track.clips().len(), 2);
    assert_eq!(session.history.undo.len(), 1);

    let undone = session.execute_command(undo(1)).unwrap();
    assert!(session.project().timeline().tracks().is_empty());
    assert!(matches!(
        undone.change_set.changes(),
        [ProjectChange::TimelineCaptionTrackRemoved { .. }]
    ));
    assert!(matches!(
        session.history.redo[0].changes(),
        [ProjectChange::TimelineCaptionTrackAdded { .. }]
    ));
    session.execute_command(redo(2)).unwrap();
    assert_eq!(session.project().timeline().tracks()[0].clips().len(), 2);
    assert_eq!(session.project_revision(), ProjectRevision::new(3));
}

#[test]
fn caption_track_import_rejects_overlap_and_duplicate_ids_without_partial_change() {
    let mut session = fixed_session();
    let import = |session: &ProjectSession, track: &str, clips| {
        CommandEnvelope::import_timeline_captions(
            session.project_id(),
            session.project_instance_id(),
            session.project_revision(),
            TrackId::from_str(track).unwrap(),
            clips,
        )
    };
    let before = session.project().clone();
    let error = session
        .execute_command(import(
            &session,
            TRACK_A,
            vec![
                caption_clip_state(CLIP_A, (0, 1), (2, 1), "one"),
                caption_clip_state(CLIP_B, (1, 1), (2, 1), "two"),
            ],
        ))
        .unwrap_err();
    assert_eq!(error.code, OperationErrorCode::TimelineOverlap);
    assert_eq!(session.project(), &before);
    assert!(session.history.undo.is_empty());

    let error = session
        .execute_command(import(
            &session,
            TRACK_A,
            vec![
                caption_clip_state(CLIP_A, (0, 1), (1, 2), "one"),
                caption_clip_state(CLIP_A, (1, 1), (1, 2), "two"),
            ],
        ))
        .unwrap_err();
    assert_eq!(error.code, OperationErrorCode::TimelineClipIdAlreadyExists);
    assert_eq!(session.project(), &before);
    assert!(session.history.undo.is_empty());
}

#[test]
fn timeline_track_add_appends_in_canonical_order_and_duplicate_is_atomic() {
    let mut session = fixed_session();
    add_track(&mut session, TRACK_A, "video").unwrap();
    add_track(&mut session, TRACK_B, "audio").unwrap();
    add_track(&mut session, TRACK_C, "video").unwrap();
    let ids = session
        .project()
        .timeline()
        .tracks()
        .iter()
        .map(|track| track.id().to_string())
        .collect::<Vec<_>>();
    assert_eq!(ids, [TRACK_A, TRACK_B, TRACK_C]);

    let before_project = session.project().clone();
    let before_undo = session.history.undo.clone();
    let before_redo = session.history.redo.clone();
    let error = add_track(&mut session, TRACK_A, "audio").unwrap_err();
    assert_eq!(error.code, OperationErrorCode::TimelineTrackIdAlreadyExists);
    assert_eq!(session.project(), &before_project);
    assert_eq!(session.history.undo, before_undo);
    assert_eq!(session.history.redo, before_redo);
}

#[test]
fn timeline_track_limit_rejects_the_next_track_without_mutation() {
    let mut session = fixed_session();
    for _ in 0..MAX_TIMELINE_TRACKS {
        add_track(&mut session, &TrackId::generate().to_string(), "video").unwrap();
    }
    let before_revision = session.project_revision();
    let before_history = session.history.undo.len();
    let error = add_track(&mut session, TRACK_A, "audio").unwrap_err();

    assert_eq!(error.code, OperationErrorCode::TimelineLimitExceeded);
    assert_eq!(
        session.project().timeline().tracks().len(),
        MAX_TIMELINE_TRACKS
    );
    assert_eq!(session.project_revision(), before_revision);
    assert_eq!(session.history.undo.len(), before_history);
}

#[test]
fn timeline_track_remove_is_empty_only_and_undo_restores_exact_order() {
    let mut session = fixed_session();
    add_track(&mut session, TRACK_A, "video").unwrap();
    add_track(&mut session, TRACK_B, "audio").unwrap();
    add_track(&mut session, TRACK_C, "video").unwrap();
    let result = execute(
        &mut session,
        "timeline.track.remove",
        json!({"track_id": TRACK_B}),
    )
    .unwrap();
    assert_eq!(result.after_revision, ProjectRevision::new(4));
    let ids = session
        .project()
        .timeline()
        .tracks()
        .iter()
        .map(|track| track.id().to_string())
        .collect::<Vec<_>>();
    assert_eq!(ids, [TRACK_A, TRACK_C]);

    session.execute_command(undo(4)).unwrap();
    let ids = session
        .project()
        .timeline()
        .tracks()
        .iter()
        .map(|track| track.id().to_string())
        .collect::<Vec<_>>();
    assert_eq!(ids, [TRACK_A, TRACK_B, TRACK_C]);
    assert_eq!(session.project_revision(), ProjectRevision::new(5));
    session.execute_command(redo(5)).unwrap();
    assert_eq!(
        session.project().timeline().tracks()[1].id().to_string(),
        TRACK_C
    );
    assert_eq!(session.project_revision(), ProjectRevision::new(6));

    let error = execute(
        &mut session,
        "timeline.track.remove",
        json!({"track_id": TRACK_B}),
    )
    .unwrap_err();
    assert_eq!(error.code, OperationErrorCode::TimelineTrackNotFound);
}

#[test]
fn timeline_track_remove_rejects_nonempty_track_without_partial_change() {
    let mut session = fixed_session();
    seed_video_track_and_clip(&mut session, MEDIA_A, TRACK_A, CLIP_A, (0, 1), (2, 1));
    let before = session.project().clone();
    let before_undo = session.history.undo.clone();
    let error = execute(
        &mut session,
        "timeline.track.remove",
        json!({"track_id": TRACK_A}),
    )
    .unwrap_err();
    assert_eq!(error.code, OperationErrorCode::TimelineTrackNotEmpty);
    assert_eq!(session.project(), &before);
    assert_eq!(session.history.undo, before_undo);
}
