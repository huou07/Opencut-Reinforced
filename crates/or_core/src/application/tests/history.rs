// Undo, redo, transactions and persistence of session history.

use super::super::*;
use super::support::*;
use super::*;
use serde_json::{json, Value};
use std::str::FromStr;

#[test]
fn timeline_history_chain_undoes_and_redoes_each_semantic_edit_once() {
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
    execute(
        &mut session,
        "timeline.clip.move",
        json!({"clip_id": CLIP_A, "track_id": TRACK_A, "timeline_start": rational_json(4, 1)}),
    )
    .unwrap();
    execute(
        &mut session,
        "timeline.clip.delete",
        json!({"clip_id": CLIP_A}),
    )
    .unwrap();
    assert_eq!(session.history.undo.len(), 5);
    assert!(session.project().timeline().tracks()[0].clips().is_empty());

    let mut observed_revisions = Vec::new();
    for _ in 0..4 {
        let result = session
            .execute_command(undo(session.project_revision().value()))
            .unwrap();
        observed_revisions.push(result.after_revision.value());
    }
    assert_eq!(observed_revisions, [6, 7, 8, 9]);
    assert!(session.project().timeline().tracks().is_empty());
    assert_eq!(session.history.redo.len(), 4);

    for expected_revision in 10..=13 {
        let result = session
            .execute_command(redo(session.project_revision().value()))
            .unwrap();
        assert_eq!(result.after_revision.value(), expected_revision);
    }
    assert_eq!(session.project_revision(), ProjectRevision::new(13));
    assert!(session.project().timeline().tracks()[0].clips().is_empty());
    assert!(session.history.undo.len() >= 4);
    assert!(session.history.redo.is_empty());
}

#[test]
fn timeline_real_edit_clears_redo_history() {
    let mut session = fixed_session();
    seed_video_track_and_clip(&mut session, MEDIA_A, TRACK_A, CLIP_A, (0, 1), (2, 1));
    execute(
        &mut session,
        "timeline.clip.move",
        json!({"clip_id": CLIP_A, "track_id": TRACK_A, "timeline_start": rational_json(4, 1)}),
    )
    .unwrap();
    session
        .execute_command(undo(session.project_revision().value()))
        .unwrap();
    assert_eq!(session.history.redo.len(), 1);
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
    assert!(session.history.redo.is_empty());
}

#[test]
fn timeline_history_conflicts_leave_project_and_both_stacks_unchanged() {
    let mut added_track = fixed_session();
    add_track(&mut added_track, TRACK_A, "video").unwrap();
    added_track
        .project
        .set_timeline_for_test(ProjectTimeline::from_tracks_for_codec(vec![fixture_track(
            TRACK_B,
            TrackKind::Audio,
            Vec::new(),
        )]));
    assert_history_conflict(&mut added_track, "history.undo");

    let mut removed_track = fixed_session();
    add_track(&mut removed_track, TRACK_A, "video").unwrap();
    add_track(&mut removed_track, TRACK_B, "audio").unwrap();
    execute(
        &mut removed_track,
        "timeline.track.remove",
        json!({"track_id": TRACK_B}),
    )
    .unwrap();
    removed_track
        .execute_command(undo(removed_track.project_revision().value()))
        .unwrap();
    removed_track
        .project
        .set_timeline_for_test(ProjectTimeline::from_tracks_for_codec(vec![
            fixture_track(TRACK_A, TrackKind::Video, Vec::new()),
            fixture_track(TRACK_B, TrackKind::Video, Vec::new()),
        ]));
    assert_history_conflict(&mut removed_track, "history.redo");

    let mut inserted_clip = fixed_session();
    seed_video_track_and_clip(&mut inserted_clip, MEDIA_A, TRACK_A, CLIP_A, (0, 1), (2, 1));
    inserted_clip
        .project
        .set_timeline_for_test(ProjectTimeline::from_tracks_for_codec(vec![fixture_track(
            TRACK_A,
            TrackKind::Video,
            vec![fixture_clip(CLIP_A, MEDIA_A, (1, 1), (0, 1), (2, 1))],
        )]));
    assert_history_conflict(&mut inserted_clip, "history.undo");

    let mut deleted_clip = fixed_session();
    seed_video_track_and_clip(&mut deleted_clip, MEDIA_A, TRACK_A, CLIP_A, (0, 1), (2, 1));
    execute(
        &mut deleted_clip,
        "timeline.clip.delete",
        json!({"clip_id": CLIP_A}),
    )
    .unwrap();
    deleted_clip
        .execute_command(undo(deleted_clip.project_revision().value()))
        .unwrap();
    deleted_clip
        .project
        .set_timeline_for_test(ProjectTimeline::from_tracks_for_codec(vec![fixture_track(
            TRACK_A,
            TrackKind::Video,
            vec![fixture_clip(CLIP_A, MEDIA_A, (1, 1), (0, 1), (2, 1))],
        )]));
    assert_history_conflict(&mut deleted_clip, "history.redo");

    let mut moved_clip = fixed_session();
    seed_video_track_and_clip(&mut moved_clip, MEDIA_A, TRACK_A, CLIP_A, (0, 1), (2, 1));
    add_track(&mut moved_clip, TRACK_B, "video").unwrap();
    execute(
        &mut moved_clip,
        "timeline.clip.move",
        json!({"clip_id": CLIP_A, "track_id": TRACK_B, "timeline_start": rational_json(4, 1)}),
    )
    .unwrap();
    moved_clip
        .execute_command(undo(moved_clip.project_revision().value()))
        .unwrap();
    moved_clip
        .project
        .set_timeline_for_test(ProjectTimeline::from_tracks_for_codec(vec![
            fixture_track(
                TRACK_A,
                TrackKind::Video,
                vec![fixture_clip(CLIP_A, MEDIA_A, (1, 1), (0, 1), (2, 1))],
            ),
            fixture_track(TRACK_B, TrackKind::Video, Vec::new()),
        ]));
    assert_history_conflict(&mut moved_clip, "history.redo");
}

#[test]
fn net_no_op_transaction_keeps_revision_and_redo_history() {
    let mut session = fixed_session();
    session.execute_command(rename("B", 0)).unwrap();
    session.execute_command(undo(1)).unwrap();
    let result = session
        .execute_transaction(transaction(vec![rename_call("B"), rename_call("A")], 2))
        .unwrap();

    assert_eq!(session.project().name(), "A");
    assert_eq!(session.project_revision(), ProjectRevision::new(2));
    assert!(!result.changed);
    assert_eq!(result.before_revision, result.after_revision);
    assert!(result.change_set.is_empty());
    assert!(session.history.undo.is_empty());
    assert_eq!(session.history.redo.len(), 1);

    session.execute_command(redo(2)).unwrap();
    assert_eq!(session.project().name(), "B");
    assert_eq!(session.project_revision(), ProjectRevision::new(3));
}

#[test]
fn invalid_child_rolls_back_the_entire_transaction_and_preserves_history() {
    let mut session = fixed_session();
    let before = session.project().clone();
    let result = session.execute_transaction(transaction(
        vec![
            rename_call("B"),
            call("project.rename", 1, json!({ "extra": true })),
        ],
        0,
    ));

    assert_eq!(code(&result), OperationErrorCode::InvalidArguments);
    assert_eq!(session.project(), &before);
    assert!(session.history.undo.is_empty());
    assert!(session.history.redo.is_empty());
}

#[test]
fn unknown_or_unsupported_child_rolls_back_the_entire_transaction() {
    for child in [
        call("project.missing", 1, json!({})),
        call("project.rename", 2, json!({ "name": "C" })),
    ] {
        let mut session = fixed_session();
        let before = session.project().clone();
        let result = session.execute_transaction(transaction(vec![rename_call("B"), child], 0));

        let expected = if result
            .as_ref()
            .is_err_and(|error| error.code == OperationErrorCode::UnknownCommand)
        {
            OperationErrorCode::UnknownCommand
        } else {
            OperationErrorCode::UnsupportedCommandSchema
        };
        assert_eq!(code(&result), expected);
        assert_eq!(session.project(), &before);
        assert!(session.history.undo.is_empty());
    }
}

#[test]
fn history_commands_are_not_allowed_inside_a_transaction() {
    let mut session = fixed_session();
    let before = session.project().clone();
    let result = session.execute_transaction(transaction(
        vec![rename_call("B"), call("history.undo", 1, json!({}))],
        0,
    ));

    assert_eq!(
        code(&result),
        OperationErrorCode::CommandNotAllowedInTransaction
    );
    assert_eq!(session.project(), &before);
    assert!(session.history.undo.is_empty());
    assert!(session.history.redo.is_empty());
}

#[test]
fn empty_transaction_is_rejected_without_mutation() {
    let mut session = fixed_session();
    let before = session.project().clone();
    assert_eq!(
        code(&session.execute_transaction(transaction(Vec::new(), 0))),
        OperationErrorCode::EmptyTransaction
    );
    assert_eq!(session.project(), &before);
    assert!(session.history.undo.is_empty());
    assert!(session.history.redo.is_empty());
}

#[test]
fn transaction_preconditions_and_schema_are_checked_before_commands() {
    let mut session = fixed_session();
    let mut wrong_project = transaction(Vec::new(), 0);
    wrong_project.project_id = ProjectId::from_str(OTHER_PROJECT_ID).unwrap();
    assert_eq!(
        code(&session.execute_transaction(wrong_project)),
        OperationErrorCode::ProjectIdMismatch
    );

    let mut wrong_instance = transaction(Vec::new(), 0);
    wrong_instance.project_instance_id = ProjectInstanceId::from_str(OTHER_INSTANCE_ID).unwrap();
    assert_eq!(
        code(&session.execute_transaction(wrong_instance)),
        OperationErrorCode::ProjectInstanceMismatch
    );

    assert_eq!(
        code(&session.execute_transaction(transaction(Vec::new(), 1))),
        OperationErrorCode::RevisionConflict
    );

    let mut unsupported = transaction(Vec::new(), 99);
    unsupported.schema_version = 2;
    assert_eq!(
        code(&session.execute_transaction(unsupported)),
        OperationErrorCode::UnsupportedTransactionSchema
    );

    assert_eq!(session.project().name(), "A");
    assert_eq!(session.project_revision(), ProjectRevision::INITIAL);
    assert!(session.history.undo.is_empty());
}

#[test]
fn redo_restores_a_change_with_a_new_revision() {
    let mut session = fixed_session();
    session.execute_command(rename("B", 0)).unwrap();
    session.execute_command(undo(1)).unwrap();

    let result = session.execute_command(redo(2)).unwrap();
    assert_eq!(session.project().name(), "B");
    assert_eq!(result.before_revision, ProjectRevision::new(2));
    assert_eq!(result.after_revision, ProjectRevision::new(3));
    assert!(result.changed);
    assert_name_change(&result.change_set, "A", "B");
    assert_eq!(session.history.undo.len(), 1);
    assert!(session.history.redo.is_empty());
}

#[test]
fn one_undo_reverses_a_whole_grouped_transaction() {
    let mut session = fixed_session();
    session
        .execute_transaction(transaction(vec![rename_call("B"), rename_call("C")], 0))
        .unwrap();

    let result = session.execute_command(undo(1)).unwrap();
    assert_eq!(session.project().name(), "A");
    assert_eq!(result.after_revision, ProjectRevision::new(2));
    assert_name_change(&result.change_set, "C", "A");
    assert!(session.history.undo.is_empty());
    assert_eq!(session.history.redo.len(), 1);
}

#[test]
fn new_real_edit_clears_redo_history() {
    let mut session = fixed_session();
    session.execute_command(rename("B", 0)).unwrap();
    session.execute_command(undo(1)).unwrap();
    assert_eq!(session.history.redo.len(), 1);

    session.execute_command(rename("C", 2)).unwrap();
    assert_eq!(session.project().name(), "C");
    assert_eq!(session.project_revision(), ProjectRevision::new(3));
    assert!(session.history.redo.is_empty());
    assert_eq!(
        code(&session.execute_command(redo(3))),
        OperationErrorCode::NothingToRedo
    );
}

#[test]
fn successful_no_op_command_does_not_clear_redo_history() {
    let mut session = fixed_session();
    session.execute_command(rename("B", 0)).unwrap();
    session.execute_command(undo(1)).unwrap();

    let result = session.execute_command(rename("A", 2)).unwrap();
    assert!(!result.changed);
    assert_eq!(session.project_revision(), ProjectRevision::new(2));
    assert_eq!(session.history.redo.len(), 1);
    session.execute_command(redo(2)).unwrap();
    assert_eq!(session.project().name(), "B");
}

#[test]
fn failed_edit_does_not_clear_redo_history() {
    let mut session = fixed_session();
    session.execute_command(rename("B", 0)).unwrap();
    session.execute_command(undo(1)).unwrap();
    let mut invalid = rename("C", 2);
    invalid.arguments = json!({});

    assert_eq!(
        code(&session.execute_command(invalid)),
        OperationErrorCode::InvalidArguments
    );
    assert_eq!(session.project().name(), "A");
    assert_eq!(session.project_revision(), ProjectRevision::new(2));
    assert_eq!(session.history.redo.len(), 1);
    session.execute_command(redo(2)).unwrap();
    assert_eq!(session.project().name(), "B");
}

#[test]
fn empty_history_and_invalid_history_arguments_are_rejected() {
    let mut session = fixed_session();
    assert_eq!(
        code(&session.execute_command(undo(0))),
        OperationErrorCode::NothingToUndo
    );
    assert_eq!(
        code(&session.execute_command(redo(0))),
        OperationErrorCode::NothingToRedo
    );

    for arguments in [Value::Null, json!({ "unexpected": true })] {
        let invalid = history_command("history.undo", 0, arguments);
        assert_eq!(
            code(&session.execute_command(invalid)),
            OperationErrorCode::InvalidArguments
        );
    }
    assert_eq!(session.project_revision(), ProjectRevision::INITIAL);
}

#[test]
fn forward_revision_overflow_changes_neither_project_nor_history() {
    let mut session = session_with_state("A", u64::MAX);
    session.history.redo.push(ChangeSet::project_name("A", "B"));
    let before = session.project().clone();
    let redo_before = session.history.redo.clone();
    assert_eq!(
        code(&session.execute_command(rename("B", u64::MAX))),
        OperationErrorCode::RevisionOverflow
    );
    assert_eq!(session.project(), &before);
    assert!(session.history.undo.is_empty());
    assert_eq!(session.history.redo, redo_before);
}

#[test]
fn undo_and_redo_revision_overflow_preserve_document_and_stacks() {
    let mut undo_session = session_with_state("B", u64::MAX);
    undo_session
        .history
        .undo
        .push(ChangeSet::project_name("A", "B"));
    let before = undo_session.project().clone();
    let undo_before = undo_session.history.undo.clone();
    assert_eq!(
        code(&undo_session.execute_command(undo(u64::MAX))),
        OperationErrorCode::RevisionOverflow
    );
    assert_eq!(undo_session.project(), &before);
    assert_eq!(undo_session.history.undo, undo_before);
    assert!(undo_session.history.redo.is_empty());

    let mut redo_session = session_with_state("A", u64::MAX);
    redo_session
        .history
        .redo
        .push(ChangeSet::project_name("A", "B"));
    let before = redo_session.project().clone();
    let redo_before = redo_session.history.redo.clone();
    assert_eq!(
        code(&redo_session.execute_command(redo(u64::MAX))),
        OperationErrorCode::RevisionOverflow
    );
    assert_eq!(redo_session.project(), &before);
    assert_eq!(redo_session.history.redo, redo_before);
    assert!(redo_session.history.undo.is_empty());
}

#[test]
fn history_conflicts_fail_without_partial_mutation_or_stack_changes() {
    let mut undo_session = session_with_state("wrong", 0);
    undo_session
        .history
        .undo
        .push(ChangeSet::project_name("A", "B"));
    let before = undo_session.project().clone();
    let undo_before = undo_session.history.undo.clone();
    assert_eq!(
        code(&undo_session.execute_command(undo(0))),
        OperationErrorCode::HistoryConflict
    );
    assert_eq!(undo_session.project(), &before);
    assert_eq!(undo_session.history.undo, undo_before);
    assert!(undo_session.history.redo.is_empty());

    let mut redo_session = session_with_state("wrong", 0);
    redo_session
        .history
        .redo
        .push(ChangeSet::project_name("A", "B"));
    let before = redo_session.project().clone();
    let redo_before = redo_session.history.redo.clone();
    assert_eq!(
        code(&redo_session.execute_command(redo(0))),
        OperationErrorCode::HistoryConflict
    );
    assert_eq!(redo_session.project(), &before);
    assert_eq!(redo_session.history.redo, redo_before);
    assert!(redo_session.history.undo.is_empty());
}

#[test]
fn transaction_and_command_call_serde_are_strict_and_round_trip() {
    let call = rename_call("B");
    let call_json = serde_json::to_string(&call).unwrap();
    assert_eq!(
        serde_json::from_str::<CommandCall>(&call_json).unwrap(),
        call
    );
    assert_eq!(
        serde_json::from_str::<Value>(&call_json)
            .unwrap()
            .as_object()
            .unwrap()
            .len(),
        3
    );
    let extra_call = call_json.replace("\"command_id\"", "\"extra\":true,\"command_id\"");
    assert!(serde_json::from_str::<CommandCall>(&extra_call).is_err());

    let envelope = transaction(vec![call], 7);
    let encoded = serde_json::to_string(&envelope).unwrap();
    assert_eq!(
        serde_json::from_str::<TransactionEnvelope>(&encoded).unwrap(),
        envelope
    );
    let with_extra = encoded.replace("\"schema_version\"", "\"extra\":true,\"schema_version\"");
    assert!(serde_json::from_str::<TransactionEnvelope>(&with_extra).is_err());

    let bad_project = encoded.replace(PROJECT_ID, "00000000-0000-1000-8000-000000000000");
    assert!(serde_json::from_str::<TransactionEnvelope>(&bad_project).is_err());
    let bad_instance = encoded.replace(INSTANCE_ID, "00000000-0000-1000-8000-000000000000");
    assert!(serde_json::from_str::<TransactionEnvelope>(&bad_instance).is_err());
    assert!(serde_json::from_str::<TransactionEnvelope>("{}").is_err());
}

#[test]
fn grouped_transaction_persists_only_canonical_state_through_orproj_v3() {
    let mut session = fixed_session();
    session
        .execute_transaction(transaction(vec![rename_call("B"), rename_call("C")], 0))
        .unwrap();
    let encoded = encode_project(session.project()).unwrap();
    let decoded = decode_project(&encoded).unwrap();

    assert_eq!(decoded.name(), "C");
    assert_eq!(decoded.revision(), ProjectRevision::new(1));
    assert!(!encoded.contains(&session.project_instance_id().to_string()));
    for forbidden in ["instance_id", "undo", "redo", "change_set"] {
        assert!(!encoded.contains(forbidden));
    }
}

#[test]
fn persistence_after_undo_keeps_current_revision_and_omits_history() {
    let mut session = fixed_session();
    session.execute_command(rename("B", 0)).unwrap();
    session.execute_command(undo(1)).unwrap();
    let encoded = encode_project(session.project()).unwrap();
    let decoded = decode_project(&encoded).unwrap();

    assert_eq!(decoded.name(), "A");
    assert_eq!(decoded.revision(), ProjectRevision::new(2));
    assert!(!encoded.contains(&session.project_instance_id().to_string()));
    for forbidden in ["instance_id", "undo", "redo", "change_set"] {
        assert!(!encoded.contains(forbidden));
    }
}

#[test]
fn reopening_a_document_starts_with_empty_session_history() {
    let mut session = fixed_session();
    session.execute_command(rename("B", 0)).unwrap();
    session.execute_command(undo(1)).unwrap();
    let encoded = encode_project(session.project()).unwrap();
    let mut reopened = ProjectSession::open(decode_project(&encoded).unwrap());

    assert_eq!(reopened.project_id(), session.project_id());
    assert_eq!(reopened.project_revision(), ProjectRevision::new(2));
    assert_eq!(reopened.project().name(), "A");
    assert!(ProjectInstanceId::from_str(&reopened.project_instance_id().to_string()).is_ok());
    let mut undo = undo(2);
    undo.project_instance_id = reopened.project_instance_id();
    assert_eq!(
        code(&reopened.execute_command(undo)),
        OperationErrorCode::NothingToUndo
    );
    let mut redo = redo(2);
    redo.project_instance_id = reopened.project_instance_id();
    assert_eq!(
        code(&reopened.execute_command(redo)),
        OperationErrorCode::NothingToRedo
    );
}
