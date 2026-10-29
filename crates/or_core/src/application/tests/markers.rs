// Timeline marker commands and the snap queries that use them.

use super::super::*;
use super::support::*;
use super::*;
use serde_json::json;
use std::str::FromStr;

#[test]
fn timeline_marker_commands_keep_canonical_order_and_semantic_history() {
    let mut session = fixed_session();
    marker_add(&mut session, MARKER_B, (2, 1), "same").unwrap();
    marker_add(&mut session, MARKER_A, (2, 1), "same").unwrap();
    marker_add(&mut session, MARKER_C, (1, 1), "same").unwrap();
    assert_eq!(session.project_revision(), ProjectRevision::new(3));
    assert_eq!(
        session
            .project()
            .timeline()
            .markers()
            .iter()
            .map(|marker| marker.id().to_string())
            .collect::<Vec<_>>(),
        [MARKER_C, MARKER_A, MARKER_B]
    );

    session.execute_command(undo(3)).unwrap();
    assert_eq!(session.project_revision(), ProjectRevision::new(4));
    assert_eq!(session.project().timeline().markers().len(), 2);
    let redo_depth = session.history.redo.len();
    let no_op_move = execute(
        &mut session,
        "timeline.marker.move",
        json!({
            "marker_id": MARKER_A,
            "timeline_time": rational_json(2, 1),
        }),
    )
    .unwrap();
    assert!(!no_op_move.changed);
    assert_eq!(session.project_revision(), ProjectRevision::new(4));
    assert_eq!(session.history.redo.len(), redo_depth);
    let no_op_rename = execute(
        &mut session,
        "timeline.marker.rename",
        json!({"marker_id": MARKER_A, "label": "same"}),
    )
    .unwrap();
    assert!(!no_op_rename.changed);
    assert_eq!(session.history.redo.len(), redo_depth);

    session.execute_command(redo(4)).unwrap();
    let moved = execute(
        &mut session,
        "timeline.marker.move",
        json!({
            "marker_id": MARKER_B,
            "timeline_time": rational_json(0, 1),
        }),
    )
    .unwrap();
    assert!(moved.changed);
    assert_eq!(session.project_revision(), ProjectRevision::new(6));
    assert_eq!(
        session
            .project()
            .timeline()
            .markers()
            .iter()
            .map(|marker| marker.id().to_string())
            .collect::<Vec<_>>(),
        [MARKER_B, MARKER_C, MARKER_A]
    );
    session.execute_command(undo(6)).unwrap();
    assert_eq!(
        session
            .project()
            .timeline()
            .markers()
            .iter()
            .map(|marker| marker.id().to_string())
            .collect::<Vec<_>>(),
        [MARKER_C, MARKER_A, MARKER_B]
    );
    session.execute_command(redo(7)).unwrap();
    assert_eq!(
        session.project().timeline().markers()[0].id(),
        marker_id(MARKER_B)
    );
}

#[test]
fn timeline_marker_rename_delete_and_history_restore_exact_semantic_state() {
    let mut session = fixed_session();
    let added = marker_add(&mut session, MARKER_A, (1, 1), "before").unwrap();
    assert!(matches!(
        added.change_set.changes(),
        [ProjectChange::TimelineMarkerAdded { marker, index: 0 }]
            if marker.marker_id == marker_id(MARKER_A)
                && marker.timeline_time == RationalTime::new(1, 1).unwrap()
                && marker.label == "before"
    ));

    let renamed = execute(
        &mut session,
        "timeline.marker.rename",
        json!({"marker_id": MARKER_A, "label": "after"}),
    )
    .unwrap();
    assert!(matches!(
        renamed.change_set.changes(),
        [ProjectChange::TimelineMarkerRenamed {
            marker_id: id,
            time,
            before_label,
            after_label,
        }]
            if *id == marker_id(MARKER_A)
                && *time == RationalTime::new(1, 1).unwrap()
                && before_label == "before"
                && after_label == "after"
    ));
    session.execute_command(undo(2)).unwrap();
    assert_eq!(session.project().timeline().markers()[0].label(), "before");
    session.execute_command(redo(3)).unwrap();
    assert_eq!(session.project().timeline().markers()[0].label(), "after");

    execute(
        &mut session,
        "timeline.marker.move",
        json!({
            "marker_id": MARKER_A,
            "timeline_time": rational_json(2, 1),
        }),
    )
    .unwrap();
    assert_eq!(
        session.project().timeline().markers()[0].timeline_time(),
        RationalTime::new(2, 1).unwrap()
    );
    session.execute_command(undo(5)).unwrap();
    assert_eq!(
        session.project().timeline().markers()[0].timeline_time(),
        RationalTime::new(1, 1).unwrap()
    );
    session.execute_command(redo(6)).unwrap();
    assert_eq!(
        session.project().timeline().markers()[0].timeline_time(),
        RationalTime::new(2, 1).unwrap()
    );

    marker_add(&mut session, MARKER_B, (3, 1), "delete me").unwrap();
    execute(
        &mut session,
        "timeline.marker.delete",
        json!({"marker_id": MARKER_B}),
    )
    .unwrap();
    assert_eq!(session.project().timeline().markers().len(), 1);
    session.execute_command(undo(9)).unwrap();
    assert_eq!(session.project().timeline().markers().len(), 2);
    assert_eq!(
        session.project().timeline().markers()[1].id(),
        marker_id(MARKER_B)
    );
    session.execute_command(redo(10)).unwrap();
    assert_eq!(session.project().timeline().markers().len(), 1);
    assert_eq!(
        session.project().timeline().markers()[0].id(),
        marker_id(MARKER_A)
    );
}

#[test]
fn timeline_marker_commands_validate_bounds_duplicates_and_transactions() {
    let mut session = fixed_session();
    marker_add(&mut session, MARKER_A, (1, 1), "label").unwrap();
    let before = session.project().clone();
    for arguments in [
        json!({
            "marker_id": MARKER_A,
            "timeline_time": rational_json(2, 1),
            "label": "duplicate",
        }),
        json!({
            "marker_id": MARKER_B,
            "timeline_time": rational_json(-1, 1),
            "label": "negative",
        }),
        json!({
            "marker_id": MARKER_B,
            "timeline_time": rational_json(2, 1),
            "label": "   ",
        }),
        json!({
            "marker_id": MARKER_B,
            "timeline_time": rational_json(2, 1),
            "label": "ok",
            "extra": true,
        }),
    ] {
        let result = execute(&mut session, "timeline.marker.add", arguments);
        assert!(matches!(
            result,
            Err(error) if error.code == OperationErrorCode::InvalidArguments
                || error.code == OperationErrorCode::TimelineMarkerIdAlreadyExists
        ));
        assert_eq!(session.project(), &before);
    }
    let oversized = "x".repeat(MAX_TIMELINE_MARKER_LABEL_BYTES + 1);
    assert_eq!(
        code(&execute(
            &mut session,
            "timeline.marker.add",
            json!({
                "marker_id": MARKER_B,
                "timeline_time": rational_json(2, 1),
                "label": oversized,
            }),
        )),
        OperationErrorCode::InvalidArguments
    );
    let transaction = transaction(
        vec![call(
            "timeline.marker.add",
            1,
            json!({
                "marker_id": MARKER_B,
                "timeline_time": rational_json(2, 1),
                "label": "blocked",
            }),
        )],
        1,
    );
    assert_eq!(
        code(&session.execute_transaction(transaction)),
        OperationErrorCode::CommandNotAllowedInTransaction
    );
    assert_eq!(session.project_revision(), ProjectRevision::new(1));
}

#[test]
fn timeline_marker_missing_and_revision_overflow_fail_atomically() {
    let mut missing = fixed_session();
    let before = missing.project().clone();
    for (command_id, arguments) in [
        (
            "timeline.marker.move",
            json!({
                "marker_id": MARKER_A,
                "timeline_time": rational_json(1, 1),
            }),
        ),
        (
            "timeline.marker.rename",
            json!({"marker_id": MARKER_A, "label": "missing"}),
        ),
        ("timeline.marker.delete", json!({"marker_id": MARKER_A})),
    ] {
        assert_eq!(
            code(&execute(&mut missing, command_id, arguments)),
            OperationErrorCode::TimelineMarkerNotFound
        );
        assert_eq!(missing.project(), &before);
        assert!(missing.history.undo.is_empty());
        assert!(missing.history.redo.is_empty());
    }

    for (command_id, arguments, has_marker) in [
        (
            "timeline.marker.add",
            json!({
                "marker_id": MARKER_A,
                "timeline_time": rational_json(1, 1),
                "label": "overflow",
            }),
            false,
        ),
        (
            "timeline.marker.move",
            json!({
                "marker_id": MARKER_A,
                "timeline_time": rational_json(2, 1),
            }),
            true,
        ),
        (
            "timeline.marker.rename",
            json!({"marker_id": MARKER_A, "label": "overflow"}),
            true,
        ),
        (
            "timeline.marker.delete",
            json!({"marker_id": MARKER_A}),
            true,
        ),
    ] {
        let mut session = session_with_state("A", u64::MAX);
        if has_marker {
            session
                .project
                .set_timeline_for_test(ProjectTimeline::from_parts_for_codec(
                    vec![],
                    vec![TimelineMarker::from_parts_for_codec(
                        marker_id(MARKER_A),
                        RationalTime::new(1, 1).unwrap(),
                        "existing".to_owned(),
                    )],
                ));
        }
        let before = session.project().clone();
        assert_eq!(
            code(&execute(&mut session, command_id, arguments)),
            OperationErrorCode::RevisionOverflow
        );
        assert_eq!(session.project(), &before);
        assert!(session.history.undo.is_empty());
        assert!(session.history.redo.is_empty());
    }
}

#[test]
fn timeline_marker_limit_and_history_conflict_are_bounded_and_atomic() {
    let mut markers = Vec::with_capacity(MAX_TIMELINE_MARKERS);
    for index in 0..MAX_TIMELINE_MARKERS {
        markers.push(TimelineMarker::from_parts_for_codec(
            MarkerId::generate(),
            RationalTime::new(index as i64, 1).unwrap(),
            "bounded".to_owned(),
        ));
    }
    let mut project = ProjectDocument::new("A");
    project.set_timeline_for_test(ProjectTimeline::from_parts_for_codec(vec![], markers));
    let mut session = ProjectSession {
        project,
        project_instance_id: ProjectInstanceId::from_str(INSTANCE_ID).unwrap(),
        history: super::super::SessionHistory::default(),
    };
    let before = session.project().clone();
    assert_eq!(
        code(
            &session.execute_command(CommandEnvelope::add_timeline_marker(
                session.project_id(),
                session.project_instance_id(),
                session.project_revision(),
                marker_id(MARKER_A),
                RationalTime::new(10001, 1).unwrap(),
                "overflow",
            ))
        ),
        OperationErrorCode::TimelineLimitExceeded
    );
    assert_eq!(session.project(), &before);

    let mut session = fixed_session();
    marker_add(&mut session, MARKER_A, (1, 1), "conflict").unwrap();
    session
        .project
        .set_timeline_for_test(ProjectTimeline::default());
    let before = session.project().clone();
    let undo_stack = session.history.undo.clone();
    assert_eq!(
        code(&session.execute_command(undo(1))),
        OperationErrorCode::HistoryConflict
    );
    assert_eq!(session.project(), &before);
    assert_eq!(session.history.undo, undo_stack);
    assert!(session.history.redo.is_empty());
}

#[test]
fn timeline_marker_query_is_bounded_canonical_read_only_and_wire_compatible() {
    let mut session = fixed_session();
    marker_add(&mut session, MARKER_B, (2, 1), "same").unwrap();
    marker_add(&mut session, MARKER_A, (2, 1), "same").unwrap();
    marker_add(&mut session, MARKER_C, (1, 1), "same").unwrap();
    let before = session.project().clone();
    let undo = session.history.undo.clone();

    let page = session
        .execute_query(QueryEnvelope::timeline_markers(
            session.project_id(),
            session.project_instance_id(),
            1,
            2,
        ))
        .unwrap();
    let page_value = page.timeline_marker_page.unwrap();
    assert_eq!(page_value.total_count, 3);
    assert_eq!(page_value.offset, 1);
    assert_eq!(page_value.limit, 2);
    assert_eq!(page_value.next_offset, None);
    assert_eq!(page_value.items[0].marker_id, marker_id(MARKER_A));
    assert_eq!(page_value.items[1].marker_id, marker_id(MARKER_B));

    let empty = session
        .execute_query(QueryEnvelope::timeline_markers(
            session.project_id(),
            session.project_instance_id(),
            99,
            MAX_TIMELINE_MARKER_PAGE_SIZE,
        ))
        .unwrap();
    assert!(empty.timeline_marker_page.unwrap().items.is_empty());
    assert_eq!(
        code(&session.execute_query(QueryEnvelope::timeline_markers(
            session.project_id(),
            session.project_instance_id(),
            0,
            MAX_TIMELINE_MARKER_PAGE_SIZE + 1,
        ))),
        OperationErrorCode::InvalidArguments
    );
    for arguments in [
        json!({"offset": 0, "limit": 1, "unexpected": true}),
        json!({"offset": -1, "limit": 1}),
        json!({"offset": 0.5, "limit": 1}),
        json!({"offset": 0, "limit": "1"}),
    ] {
        let invalid = QueryEnvelope {
            query_id: "timeline.markers".to_owned(),
            schema_version: 1,
            project_id: session.project_id(),
            project_instance_id: session.project_instance_id(),
            arguments,
        };
        assert_eq!(
            code(&session.execute_query(invalid)),
            OperationErrorCode::InvalidArguments
        );
    }
    assert_eq!(session.project(), &before);
    assert_eq!(session.history.undo, undo);

    let summary = session
        .execute_query(summary_query())
        .expect("summary query should still use its original wire shape");
    assert!(!serde_json::to_value(summary)
        .unwrap()
        .as_object()
        .unwrap()
        .contains_key("timeline_marker_page"));

    let mut bounded = fixed_session();
    let label = "x".repeat(MAX_TIMELINE_MARKER_LABEL_BYTES);
    for index in 0..MAX_TIMELINE_MARKER_PAGE_SIZE {
        let marker_id = format!("00000000-0000-4000-8000-{index:012x}");
        marker_add(&mut bounded, &marker_id, (index as i64, 1), &label).unwrap();
    }
    let encoded = serde_json::to_vec(
        &bounded
            .execute_query(QueryEnvelope::timeline_markers(
                bounded.project_id(),
                bounded.project_instance_id(),
                0,
                MAX_TIMELINE_MARKER_PAGE_SIZE,
            ))
            .unwrap(),
    )
    .unwrap();
    assert!(encoded.len() < 1_000_000);
}

#[test]
fn timeline_markers_do_not_change_when_clip_edits_run() {
    let mut session = fixed_session();
    seed_video_track_and_clip(&mut session, MEDIA_A, TRACK_A, CLIP_A, (0, 1), (4, 1));
    marker_add(&mut session, MARKER_A, (10, 1), "persistent").unwrap();
    let expected = session.project().timeline().markers().to_vec();
    execute(
        &mut session,
        "timeline.clip.move",
        json!({"clip_id": CLIP_A, "track_id": TRACK_A, "timeline_start": rational_json(4, 1)}),
    )
    .unwrap();
    execute(
        &mut session,
        "timeline.clip.trim",
        json!({"clip_id": CLIP_A, "edge": "end", "timeline_time": rational_json(6, 1)}),
    )
    .unwrap();
    execute(
        &mut session,
        "timeline.clip.split",
        json!({
            "clip_id": CLIP_A,
            "new_clip_id": CLIP_B,
            "timeline_time": rational_json(5, 1),
        }),
    )
    .unwrap();
    execute(
        &mut session,
        "timeline.clip.ripple_delete",
        json!({"clip_id": CLIP_B}),
    )
    .unwrap();
    assert_eq!(session.project().timeline().markers(), expected.as_slice());
}

#[test]
fn timeline_snap_v1_ignores_markers_and_v2_reports_marker_winners_and_ties() {
    let mut session = fixed_session();
    seed_video_track_and_clip(&mut session, MEDIA_A, TRACK_A, CLIP_A, (0, 1), (1, 1));
    marker_add(&mut session, MARKER_A, (3, 1), "snap").unwrap();
    let before_revision = session.project_revision();
    let before_history = session.history.undo.clone();
    let v1 = session
        .execute_query(QueryEnvelope::timeline_snap(
            session.project_id(),
            session.project_instance_id(),
            super::super::TimelineSnapOperation::TrimEnd,
            CLIP_A.parse().unwrap(),
            None,
            RationalTime::new(31, 10).unwrap(),
        ))
        .unwrap();
    assert_eq!(v1.schema_version, 1);
    let v1_json = serde_json::to_value(&v1).unwrap();
    assert!(!v1_json["timeline_snap"]
        .as_object()
        .unwrap()
        .contains_key("target_marker_id"));
    assert_eq!(
        v1.timeline_snap.unwrap().target_kind,
        TimelineSnapTargetKind::None
    );

    let v2 = session
        .execute_query(QueryEnvelope::timeline_snap_v2(
            session.project_id(),
            session.project_instance_id(),
            super::super::TimelineSnapOperation::TrimEnd,
            CLIP_A.parse().unwrap(),
            None,
            RationalTime::new(31, 10).unwrap(),
        ))
        .unwrap();
    assert_eq!(v2.schema_version, 2);
    let snap = v2.timeline_snap.unwrap();
    assert_eq!(snap.target_kind, TimelineSnapTargetKind::Marker);
    assert_eq!(snap.target_marker_id, Some(marker_id(MARKER_A)));
    assert_eq!(snap.target_time, RationalTime::new(3, 1).unwrap());
    assert_eq!(session.project_revision(), before_revision);
    assert_eq!(session.history.undo, before_history);

    let mut unsupported = QueryEnvelope::timeline_snap_v2(
        session.project_id(),
        session.project_instance_id(),
        super::super::TimelineSnapOperation::TrimEnd,
        CLIP_A.parse().unwrap(),
        None,
        RationalTime::new(31, 10).unwrap(),
    );
    unsupported.schema_version = 3;
    assert_eq!(
        code(&session.execute_query(unsupported)),
        OperationErrorCode::UnsupportedQuerySchema
    );
    let mut other_query = summary_query();
    other_query.schema_version = 2;
    assert_eq!(
        code(&session.execute_query(other_query)),
        OperationErrorCode::UnsupportedQuerySchema
    );

    let mut tie_session = fixed_session();
    seed_video_track_and_clip(&mut tie_session, MEDIA_A, TRACK_A, CLIP_A, (0, 1), (1, 1));
    add_track(&mut tie_session, TRACK_B, "video").unwrap();
    insert_clip(
        &mut tie_session,
        CLIP_B,
        TRACK_B,
        MEDIA_A,
        (1, 1),
        (1, 1),
        (1, 1),
    )
    .unwrap();
    marker_add(&mut tie_session, MARKER_A, (1, 1), "tie").unwrap();
    let tie = tie_session
        .execute_query(QueryEnvelope::timeline_snap_v2(
            tie_session.project_id(),
            tie_session.project_instance_id(),
            super::super::TimelineSnapOperation::TrimEnd,
            CLIP_A.parse().unwrap(),
            None,
            RationalTime::new(1, 1).unwrap(),
        ))
        .unwrap();
    assert_eq!(
        tie.timeline_snap.unwrap().target_kind,
        TimelineSnapTargetKind::ClipStart
    );

    let mut zero_tie = fixed_session();
    seed_video_track_and_clip(&mut zero_tie, MEDIA_A, TRACK_A, CLIP_A, (0, 1), (1, 1));
    marker_add(&mut zero_tie, MARKER_A, (0, 1), "zero").unwrap();
    let zero_tie = zero_tie
        .execute_query(QueryEnvelope::timeline_snap_v2(
            zero_tie.project_id(),
            zero_tie.project_instance_id(),
            super::super::TimelineSnapOperation::TrimStart,
            CLIP_A.parse().unwrap(),
            None,
            RationalTime::new(1, 20).unwrap(),
        ))
        .unwrap()
        .timeline_snap
        .unwrap();
    assert_eq!(zero_tie.target_kind, TimelineSnapTargetKind::TimelineZero);
    assert_eq!(zero_tie.target_marker_id, None);

    let mut marker_tie = fixed_session();
    seed_video_track_and_clip(&mut marker_tie, MEDIA_A, TRACK_A, CLIP_A, (0, 1), (1, 1));
    marker_add(&mut marker_tie, MARKER_B, (3, 1), "later").unwrap();
    marker_add(&mut marker_tie, MARKER_A, (3, 1), "earlier").unwrap();
    let marker_tie = marker_tie
        .execute_query(QueryEnvelope::timeline_snap_v2(
            marker_tie.project_id(),
            marker_tie.project_instance_id(),
            super::super::TimelineSnapOperation::TrimEnd,
            CLIP_A.parse().unwrap(),
            None,
            RationalTime::new(31, 10).unwrap(),
        ))
        .unwrap()
        .timeline_snap
        .unwrap();
    assert_eq!(marker_tie.target_kind, TimelineSnapTargetKind::Marker);
    assert_eq!(marker_tie.target_marker_id, Some(marker_id(MARKER_A)));

    let mut move_end = fixed_session();
    seed_video_track_and_clip(&mut move_end, MEDIA_A, TRACK_A, CLIP_A, (0, 1), (1, 1));
    marker_add(&mut move_end, MARKER_A, (3, 1), "move end").unwrap();
    let move_end = move_end
        .execute_query(QueryEnvelope::timeline_snap_v2(
            move_end.project_id(),
            move_end.project_instance_id(),
            super::super::TimelineSnapOperation::Move,
            CLIP_A.parse().unwrap(),
            Some(TRACK_A.parse().unwrap()),
            RationalTime::new(21, 10).unwrap(),
        ))
        .unwrap()
        .timeline_snap
        .unwrap();
    assert_eq!(move_end.target_kind, TimelineSnapTargetKind::Marker);
    assert_eq!(move_end.target_marker_id, Some(marker_id(MARKER_A)));
    assert_eq!(move_end.moving_anchor, TimelineSnapMovingAnchor::End);
    assert_eq!(
        move_end.resolved_target_time,
        RationalTime::new(2, 1).unwrap()
    );
}
