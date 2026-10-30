// Request dispatch, catalogs, project summary, rename and sequence rate.

use super::super::*;
use super::support::*;
use super::*;
use serde_json::{Value, json};
use std::str::FromStr;

#[test]
fn common_dispatch_reuses_command_query_and_transaction_paths() {
    let mut session = fixed_session();
    let command = session.handle_application_request(ApplicationRequest::Command(rename("B", 0)));
    assert!(
        matches!(command, ApplicationResponse::Command(result) if result.after_revision == ProjectRevision::new(1))
    );

    let query = session.handle_application_request(ApplicationRequest::Query(summary_query()));
    assert!(matches!(query, ApplicationResponse::Query(result) if result.summary.name == "B"));

    let transaction = session.handle_application_request(ApplicationRequest::Transaction(
        transaction(vec![rename_call("C"), rename_call("D")], 1),
    ));
    assert!(
        matches!(transaction, ApplicationResponse::Transaction(result) if result.after_revision == ProjectRevision::new(2) && result.command_count == 2)
    );

    let stale = session.handle_application_request(ApplicationRequest::Command(rename("E", 1)));
    assert!(
        matches!(stale, ApplicationResponse::Error(error) if error.code == OperationErrorCode::RevisionConflict)
    );
}

#[test]
fn catalogs_are_exact_unique_and_deterministic() {
    assert_eq!(
        command_catalog(),
        &[
            CommandDescriptor {
                id: "project.rename",
                schema_version: 1,
                mutates_project: true,
                allowed_in_transaction: true,
            },
            CommandDescriptor {
                id: "history.undo",
                schema_version: 1,
                mutates_project: true,
                allowed_in_transaction: false,
            },
            CommandDescriptor {
                id: "history.redo",
                schema_version: 1,
                mutates_project: true,
                allowed_in_transaction: false,
            },
            CommandDescriptor {
                id: "media.add",
                schema_version: 1,
                mutates_project: true,
                allowed_in_transaction: false,
            },
            CommandDescriptor {
                id: "media.remove",
                schema_version: 1,
                mutates_project: true,
                allowed_in_transaction: false,
            },
            CommandDescriptor {
                id: "media.relink",
                schema_version: 1,
                mutates_project: true,
                allowed_in_transaction: false,
            },
            CommandDescriptor {
                id: "timeline.track.add",
                schema_version: 2,
                mutates_project: true,
                allowed_in_transaction: false,
            },
            CommandDescriptor {
                id: "timeline.track.remove",
                schema_version: 1,
                mutates_project: true,
                allowed_in_transaction: false,
            },
            CommandDescriptor {
                id: "timeline.track.move",
                schema_version: 1,
                mutates_project: true,
                allowed_in_transaction: false,
            },
            CommandDescriptor {
                id: "timeline.clip.insert",
                schema_version: 1,
                mutates_project: true,
                allowed_in_transaction: false,
            },
            CommandDescriptor {
                id: "timeline.track.set_state",
                schema_version: 1,
                mutates_project: true,
                allowed_in_transaction: false,
            },
            CommandDescriptor {
                id: "timeline.clip.insert_content",
                schema_version: 1,
                mutates_project: true,
                allowed_in_transaction: false,
            },
            CommandDescriptor {
                id: "timeline.captions.import",
                schema_version: 1,
                mutates_project: true,
                allowed_in_transaction: false,
            },
            CommandDescriptor {
                id: "timeline.clip.update",
                schema_version: 1,
                mutates_project: true,
                allowed_in_transaction: false,
            },
            CommandDescriptor {
                id: "timeline.clip.move",
                schema_version: 1,
                mutates_project: true,
                allowed_in_transaction: false,
            },
            CommandDescriptor {
                id: "timeline.clips.move",
                schema_version: 1,
                mutates_project: true,
                allowed_in_transaction: false,
            },
            CommandDescriptor {
                id: "timeline.clip.delete",
                schema_version: 1,
                mutates_project: true,
                allowed_in_transaction: false,
            },
            CommandDescriptor {
                id: "timeline.clip.trim",
                schema_version: 1,
                mutates_project: true,
                allowed_in_transaction: false,
            },
            CommandDescriptor {
                id: "timeline.clip.slip",
                schema_version: 1,
                mutates_project: true,
                allowed_in_transaction: false,
            },
            CommandDescriptor {
                id: "timeline.clip.roll",
                schema_version: 1,
                mutates_project: true,
                allowed_in_transaction: false,
            },
            CommandDescriptor {
                id: "timeline.clip.slide",
                schema_version: 1,
                mutates_project: true,
                allowed_in_transaction: false,
            },
            CommandDescriptor {
                id: "timeline.clip.split",
                schema_version: 1,
                mutates_project: true,
                allowed_in_transaction: false,
            },
            CommandDescriptor {
                id: "timeline.clip.duck",
                schema_version: 1,
                mutates_project: true,
                allowed_in_transaction: false,
            },
            CommandDescriptor {
                id: "timeline.clip.ripple_delete",
                schema_version: 1,
                mutates_project: true,
                allowed_in_transaction: false,
            },
            CommandDescriptor {
                id: "timeline.marker.add",
                schema_version: 1,
                mutates_project: true,
                allowed_in_transaction: false,
            },
            CommandDescriptor {
                id: "timeline.marker.move",
                schema_version: 1,
                mutates_project: true,
                allowed_in_transaction: false,
            },
            CommandDescriptor {
                id: "timeline.marker.rename",
                schema_version: 1,
                mutates_project: true,
                allowed_in_transaction: false,
            },
            CommandDescriptor {
                id: "timeline.marker.delete",
                schema_version: 1,
                mutates_project: true,
                allowed_in_transaction: false,
            },
            CommandDescriptor {
                id: "timeline.sequence.set_frame_rate",
                schema_version: 1,
                mutates_project: true,
                allowed_in_transaction: false,
            },
        ]
    );
    assert_eq!(
        query_catalog(),
        &[
            QueryDescriptor {
                id: "project.summary",
                schema_version: 1,
            },
            QueryDescriptor {
                id: "media.list",
                schema_version: 1,
            },
            QueryDescriptor {
                id: "media.get",
                schema_version: 1,
            },
            QueryDescriptor {
                id: "timeline.tracks",
                schema_version: 2,
            },
            QueryDescriptor {
                id: "timeline.clips",
                schema_version: 2,
            },
            QueryDescriptor {
                id: "timeline.snap",
                schema_version: 2,
            },
            QueryDescriptor {
                id: "timeline.markers",
                schema_version: 1,
            },
            QueryDescriptor {
                id: "timeline.sequence.settings",
                schema_version: 1,
            },
        ]
    );
    assert_eq!(command_catalog(), command_catalog());
    assert_eq!(COMMANDS.len(), 29);
    assert_eq!(QUERIES.len(), 8);
}

#[test]
fn phase_4d_error_codes_serialize_to_stable_machine_names() {
    for (code, expected) in [
        (
            OperationErrorCode::UnsupportedTransactionSchema,
            "UNSUPPORTED_TRANSACTION_SCHEMA",
        ),
        (OperationErrorCode::EmptyTransaction, "EMPTY_TRANSACTION"),
        (
            OperationErrorCode::CommandNotAllowedInTransaction,
            "COMMAND_NOT_ALLOWED_IN_TRANSACTION",
        ),
        (OperationErrorCode::NothingToUndo, "NOTHING_TO_UNDO"),
        (OperationErrorCode::NothingToRedo, "NOTHING_TO_REDO"),
        (OperationErrorCode::HistoryConflict, "HISTORY_CONFLICT"),
        (
            OperationErrorCode::HistoryStorageFailure,
            "HISTORY_STORAGE_FAILURE",
        ),
    ] {
        assert_eq!(
            serde_json::to_string(&code).unwrap(),
            format!("\"{expected}\"")
        );
    }
}

#[test]
fn timeline_operation_error_codes_serialize_to_stable_machine_names() {
    for (code, expected) in [
        (
            OperationErrorCode::TimelineTrackIdAlreadyExists,
            "TIMELINE_TRACK_ID_ALREADY_EXISTS",
        ),
        (
            OperationErrorCode::TimelineTrackNotFound,
            "TIMELINE_TRACK_NOT_FOUND",
        ),
        (
            OperationErrorCode::TimelineTrackNotEmpty,
            "TIMELINE_TRACK_NOT_EMPTY",
        ),
        (
            OperationErrorCode::TimelineClipIdAlreadyExists,
            "TIMELINE_CLIP_ID_ALREADY_EXISTS",
        ),
        (
            OperationErrorCode::TimelineClipNotFound,
            "TIMELINE_CLIP_NOT_FOUND",
        ),
        (
            OperationErrorCode::TimelineMediaIncompatible,
            "TIMELINE_MEDIA_INCOMPATIBLE",
        ),
        (OperationErrorCode::TimelineOverlap, "TIMELINE_OVERLAP"),
        (
            OperationErrorCode::TimelineLimitExceeded,
            "TIMELINE_LIMIT_EXCEEDED",
        ),
    ] {
        assert_eq!(
            serde_json::to_string(&code).unwrap(),
            format!("\"{expected}\"")
        );
    }
}

#[test]
fn opening_session_preserves_document_and_has_valid_runtime_identity() {
    let document = project();
    let original = document.clone();
    let session = ProjectSession::open(document);

    assert_eq!(session.project(), &original);
    assert_eq!(session.project_id(), original.id());
    assert_eq!(session.project_revision(), original.revision());
    assert!(ProjectInstanceId::from_str(&session.project_instance_id().to_string()).is_ok());
    assert_eq!(session.into_project(), original);
}

#[test]
fn rename_changes_name_and_increments_revision_once() {
    let mut session = fixed_session();
    let result = session.execute_command(rename("B", 0)).unwrap();

    assert_eq!(session.project().name(), "B");
    assert_eq!(session.project_revision(), ProjectRevision::new(1));
    assert_eq!(result.before_revision, ProjectRevision::new(0));
    assert_eq!(result.after_revision, ProjectRevision::new(1));
    assert!(result.changed);
}

#[test]
fn rename_preserves_unicode_exactly() {
    let mut session = fixed_session();
    let name = "  Tiếng Việt – cà phê 🎬  ";
    let result = session.execute_command(rename(name, 0)).unwrap();

    assert_eq!(session.project().name(), name);
    assert!(result.changed);
}

#[test]
fn each_real_rename_increments_revision_exactly_once() {
    let mut session = fixed_session();
    session.execute_command(rename("B", 0)).unwrap();
    let result = session.execute_command(rename("C", 1)).unwrap();

    assert_eq!(session.project().name(), "C");
    assert_eq!(result.before_revision, ProjectRevision::new(1));
    assert_eq!(result.after_revision, ProjectRevision::new(2));
}

#[test]
fn rename_to_same_name_is_a_successful_no_op() {
    let mut session = fixed_session();
    let result = session.execute_command(rename("A", 0)).unwrap();

    assert_eq!(session.project().name(), "A");
    assert_eq!(result.before_revision, ProjectRevision::INITIAL);
    assert_eq!(result.after_revision, ProjectRevision::INITIAL);
    assert!(!result.changed);
}

#[test]
fn revision_conflict_returns_current_revision_without_mutation() {
    let mut session = fixed_session();
    session.execute_command(rename("B", 0)).unwrap();
    let before = session.project().clone();
    let result = session.execute_command(rename("C", 0));

    assert_eq!(code(&result), OperationErrorCode::RevisionConflict);
    let error = result.unwrap_err();
    assert_eq!(error.current_revision, Some(ProjectRevision::new(1)));
    assert_eq!(
        serde_json::to_value(error).unwrap(),
        json!({ "code": "REVISION_CONFLICT", "current_revision": 1 })
    );
    assert_eq!(session.project(), &before);
}

#[test]
fn wrong_instance_is_rejected_even_with_matching_project_and_revision() {
    let mut session = fixed_session();
    let mut command = rename("B", 0);
    command.project_instance_id = ProjectInstanceId::from_str(OTHER_INSTANCE_ID).unwrap();
    let before = session.project().clone();

    assert_eq!(
        code(&session.execute_command(command)),
        OperationErrorCode::ProjectInstanceMismatch
    );
    assert_eq!(session.project(), &before);
}

#[test]
fn wrong_project_is_rejected_without_mutation() {
    let mut session = fixed_session();
    let mut command = rename("B", 0);
    command.project_id = ProjectId::from_str(OTHER_PROJECT_ID).unwrap();
    let before = session.project().clone();

    assert_eq!(
        code(&session.execute_command(command)),
        OperationErrorCode::ProjectIdMismatch
    );
    assert_eq!(session.project(), &before);
}

#[test]
fn unknown_and_unsupported_commands_are_rejected_before_mutation() {
    let mut session = fixed_session();
    let before = session.project().clone();
    let mut unknown = rename("B", 0);
    unknown.command_id = "project.delete".to_owned();
    assert_eq!(
        code(&session.execute_command(unknown)),
        OperationErrorCode::UnknownCommand
    );

    let mut unsupported = rename("B", 0);
    unsupported.schema_version = 2;
    assert_eq!(
        code(&session.execute_command(unsupported)),
        OperationErrorCode::UnsupportedCommandSchema
    );
    assert_eq!(session.project(), &before);
}

#[test]
fn command_validation_follows_id_schema_project_instance_revision_order() {
    let mut session = fixed_session();
    let mut command = rename("B", 99);
    command.command_id = "unknown.command".to_owned();
    command.schema_version = 2;
    command.project_id = ProjectId::from_str(OTHER_PROJECT_ID).unwrap();
    command.project_instance_id = ProjectInstanceId::from_str(OTHER_INSTANCE_ID).unwrap();
    assert_eq!(
        code(&session.execute_command(command.clone())),
        OperationErrorCode::UnknownCommand
    );

    command.command_id = "project.rename".to_owned();
    assert_eq!(
        code(&session.execute_command(command.clone())),
        OperationErrorCode::UnsupportedCommandSchema
    );

    command.schema_version = 1;
    assert_eq!(
        code(&session.execute_command(command.clone())),
        OperationErrorCode::ProjectIdMismatch
    );

    command.project_id = ProjectId::from_str(PROJECT_ID).unwrap();
    assert_eq!(
        code(&session.execute_command(command.clone())),
        OperationErrorCode::ProjectInstanceMismatch
    );

    command.project_instance_id = ProjectInstanceId::from_str(INSTANCE_ID).unwrap();
    assert_eq!(
        code(&session.execute_command(command)),
        OperationErrorCode::RevisionConflict
    );
    assert_eq!(session.project().name(), "A");
    assert_eq!(session.project_revision(), ProjectRevision::INITIAL);
}

#[test]
fn invalid_rename_arguments_are_rejected_without_mutation() {
    let mut session = fixed_session();
    let before = session.project().clone();
    for arguments in [
        json!({}),
        json!({ "name": 5 }),
        json!({ "name": "B", "extra": true }),
    ] {
        let mut command = rename("B", 0);
        command.arguments = arguments;
        assert_eq!(
            code(&session.execute_command(command)),
            OperationErrorCode::InvalidArguments
        );
        assert_eq!(session.project(), &before);
    }
}

#[test]
fn revision_overflow_rejects_rename_without_partial_mutation() {
    let mut session = ProjectSession {
        project: decode_project(
            r#"{"format":"opencut-reinforced-project","schema_version":1,"project":{"id":"01234567-89ab-4def-8123-456789abcdef","revision":18446744073709551615,"name":"A"}}"#,
        )
        .unwrap(),
        project_instance_id: ProjectInstanceId::from_str(INSTANCE_ID).unwrap(),
        history: super::super::SessionHistory::default(),
    };
    let before = session.project().clone();
    let result = session.execute_command(rename("B", u64::MAX));

    assert_eq!(code(&result), OperationErrorCode::RevisionOverflow);
    assert_eq!(session.project(), &before);
}

#[test]
fn project_summary_is_read_only_and_returns_current_state() {
    let session = fixed_session();
    let before = session.project().clone();
    let result = session.execute_query(summary_query()).unwrap();

    assert_eq!(result.query_id, "project.summary");
    assert_eq!(result.schema_version, 1);
    assert_eq!(result.summary.project_id, session.project_id());
    assert_eq!(
        result.summary.project_instance_id,
        session.project_instance_id()
    );
    assert_eq!(result.summary.project_revision, ProjectRevision::INITIAL);
    assert_eq!(result.summary.name, "A");
    assert_eq!(
        serde_json::to_value(&result).unwrap(),
        json!({
            "query_id": "project.summary",
            "schema_version": 1,
            "project_id": PROJECT_ID,
            "project_instance_id": INSTANCE_ID,
            "project_revision": 0,
            "name": "A"
        })
    );
    assert_eq!(session.project(), &before);
    assert_eq!(session.project_revision(), ProjectRevision::INITIAL);
}

#[test]
fn summary_after_rename_observes_new_canonical_state() {
    let mut session = fixed_session();
    session.execute_command(rename("B", 0)).unwrap();
    let summary = session.execute_query(summary_query()).unwrap().summary;

    assert_eq!(summary.name, "B");
    assert_eq!(summary.project_revision, ProjectRevision::new(1));
}

#[test]
fn wrong_project_and_instance_queries_are_rejected_without_state_change() {
    let session = fixed_session();
    let before = session.project().clone();
    let mut wrong_project = summary_query();
    wrong_project.project_id = ProjectId::from_str(OTHER_PROJECT_ID).unwrap();
    assert_eq!(
        code(&session.execute_query(wrong_project)),
        OperationErrorCode::ProjectIdMismatch
    );

    let mut wrong_instance = summary_query();
    wrong_instance.project_instance_id = ProjectInstanceId::from_str(OTHER_INSTANCE_ID).unwrap();
    assert_eq!(
        code(&session.execute_query(wrong_instance)),
        OperationErrorCode::ProjectInstanceMismatch
    );
    assert_eq!(session.project(), &before);
}

#[test]
fn unknown_and_unsupported_queries_are_rejected() {
    let session = fixed_session();
    let mut unknown = summary_query();
    unknown.query_id = "project.timeline".to_owned();
    assert_eq!(
        code(&session.execute_query(unknown)),
        OperationErrorCode::UnknownQuery
    );

    let mut unsupported = summary_query();
    unsupported.schema_version = 2;
    assert_eq!(
        code(&session.execute_query(unsupported)),
        OperationErrorCode::UnsupportedQuerySchema
    );
}

#[test]
fn query_validation_follows_id_schema_project_instance_arguments_order() {
    let session = fixed_session();
    let mut query = summary_query();
    query.query_id = "unknown.query".to_owned();
    query.schema_version = 2;
    query.project_id = ProjectId::from_str(OTHER_PROJECT_ID).unwrap();
    query.project_instance_id = ProjectInstanceId::from_str(OTHER_INSTANCE_ID).unwrap();
    query.arguments = json!({ "unexpected": true });
    assert_eq!(
        code(&session.execute_query(query.clone())),
        OperationErrorCode::UnknownQuery
    );

    query.query_id = "project.summary".to_owned();
    assert_eq!(
        code(&session.execute_query(query.clone())),
        OperationErrorCode::UnsupportedQuerySchema
    );

    query.schema_version = 1;
    assert_eq!(
        code(&session.execute_query(query.clone())),
        OperationErrorCode::ProjectIdMismatch
    );

    query.project_id = ProjectId::from_str(PROJECT_ID).unwrap();
    assert_eq!(
        code(&session.execute_query(query.clone())),
        OperationErrorCode::ProjectInstanceMismatch
    );

    query.project_instance_id = ProjectInstanceId::from_str(INSTANCE_ID).unwrap();
    assert_eq!(
        code(&session.execute_query(query)),
        OperationErrorCode::InvalidArguments
    );
    assert_eq!(session.project().name(), "A");
    assert_eq!(session.project_revision(), ProjectRevision::INITIAL);
}

#[test]
fn summary_requires_an_empty_object_of_arguments() {
    let session = fixed_session();
    for arguments in [json!({ "unexpected": true }), json!([]), Value::Null] {
        let mut query = summary_query();
        query.arguments = arguments;
        assert_eq!(
            code(&session.execute_query(query)),
            OperationErrorCode::InvalidArguments
        );
    }
}

#[test]
fn envelopes_round_trip_and_reject_unknown_fields_and_invalid_typed_ids() {
    let command = rename("Tiếng Việt", 7);
    let encoded = serde_json::to_string(&command).unwrap();
    assert_eq!(
        serde_json::from_str::<CommandEnvelope>(&encoded).unwrap(),
        command
    );
    let with_extra_command_field =
        encoded.replace("\"command_id\"", "\"unexpected\":true,\"command_id\"");
    assert!(serde_json::from_str::<CommandEnvelope>(&with_extra_command_field).is_err());

    let query = summary_query();
    let encoded = serde_json::to_string(&query).unwrap();
    assert_eq!(
        serde_json::from_str::<QueryEnvelope>(&encoded).unwrap(),
        query
    );
    let with_extra_query_field =
        encoded.replace("\"query_id\"", "\"unexpected\":true,\"query_id\"");
    assert!(serde_json::from_str::<QueryEnvelope>(&with_extra_query_field).is_err());

    let bad_id = encoded.replace(PROJECT_ID, "00000000-0000-1000-8000-000000000000");
    assert!(serde_json::from_str::<QueryEnvelope>(&bad_id).is_err());
    let bad_instance_id = encoded.replace(INSTANCE_ID, "00000000-0000-1000-8000-000000000000");
    assert!(serde_json::from_str::<QueryEnvelope>(&bad_instance_id).is_err());
}

#[test]
fn rename_round_trips_through_project_codec_without_persisting_session_identity() {
    let mut session = fixed_session();
    session.execute_command(rename("Renamed", 0)).unwrap();
    let encoded = encode_project(session.project()).unwrap();
    let decoded = decode_project(&encoded).unwrap();

    assert_eq!(decoded.name(), "Renamed");
    assert_eq!(decoded.revision(), ProjectRevision::new(1));
    assert!(!encoded.contains(&session.project_instance_id().to_string()));
    assert!(!encoded.contains("instance_id"));
}

#[test]
fn rename_results_report_changes_and_no_op_changesets_are_empty() {
    let mut session = fixed_session();
    let result = session.execute_command(rename("B", 0)).unwrap();
    assert_name_change(&result.change_set, "A", "B");

    let no_op = session.execute_command(rename("B", 1)).unwrap();
    assert!(!no_op.changed);
    assert_eq!(no_op.before_revision, no_op.after_revision);
    assert!(no_op.change_set.is_empty());
    assert_eq!(session.history.undo.len(), 1);
    assert!(session.history.redo.is_empty());
}

#[test]
fn grouped_renames_commit_once_with_a_normalized_changeset_and_one_history_entry() {
    let mut session = fixed_session();
    let result = session
        .execute_transaction(transaction(vec![rename_call("B"), rename_call("C")], 0))
        .unwrap();

    assert_eq!(session.project().name(), "C");
    assert_eq!(result.schema_version, 1);
    assert_eq!(result.before_revision, ProjectRevision::new(0));
    assert_eq!(result.after_revision, ProjectRevision::new(1));
    assert!(result.changed);
    assert_eq!(result.command_count, 2);
    assert_name_change(&result.change_set, "A", "C");
    assert_eq!(session.history.undo.len(), 1);
    assert!(session.history.redo.is_empty());
}

#[test]
fn single_rename_creates_one_history_entry_and_undo_returns_inverse_change() {
    let mut session = fixed_session();
    let renamed = session.execute_command(rename("B", 0)).unwrap();
    assert_eq!(renamed.after_revision, ProjectRevision::new(1));
    assert_eq!(session.history.undo.len(), 1);
    assert!(session.history.redo.is_empty());

    let result = session.execute_command(undo(1)).unwrap();
    assert_eq!(session.project().name(), "A");
    assert_eq!(result.before_revision, ProjectRevision::new(1));
    assert_eq!(result.after_revision, ProjectRevision::new(2));
    assert!(result.changed);
    assert_name_change(&result.change_set, "B", "A");
    assert!(session.history.undo.is_empty());
    assert_eq!(session.history.redo.len(), 1);
}

#[test]
fn sequence_rate_command_query_noop_and_history_use_the_generic_application_path() {
    let mut session = fixed_session();
    let initial = session.handle_application_request(ApplicationRequest::Query(
        QueryEnvelope::timeline_sequence_settings(
            session.project_id(),
            session.project_instance_id(),
        ),
    ));
    assert!(matches!(
        initial,
        ApplicationResponse::Query(result)
            if result.timeline_sequence_settings.as_ref().is_some_and(|settings| settings.sequence_frame_rate.is_none())
                && result.summary.project_revision == ProjectRevision::new(0)
    ));

    let rate = RationalRate::new(24_000, 1_001).unwrap();
    let changed = session
        .handle_application_request(ApplicationRequest::Command(sequence_rate(Some(rate), 0)));
    assert!(matches!(
        changed,
        ApplicationResponse::Command(result)
            if result.changed
                && result.before_revision == ProjectRevision::new(0)
                && result.after_revision == ProjectRevision::new(1)
                && matches!(result.change_set.changes(), [ProjectChange::TimelineSequenceFrameRateChanged { before: None, after: Some(value) }] if *value == rate)
    ));
    assert_eq!(
        session.project().timeline().sequence_frame_rate(),
        Some(rate)
    );

    let query = session.handle_application_request(ApplicationRequest::Query(
        QueryEnvelope::timeline_sequence_settings(
            session.project_id(),
            session.project_instance_id(),
        ),
    ));
    assert!(matches!(
        query,
        ApplicationResponse::Query(result)
            if result.timeline_sequence_settings.as_ref().is_some_and(|settings| settings.sequence_frame_rate == Some(rate))
    ));

    let no_op = session
        .execute_command(sequence_rate(Some(rate), 1))
        .unwrap();
    assert!(!no_op.changed);
    assert_eq!(session.project_revision(), ProjectRevision::new(1));
    assert_eq!(session.history.undo.len(), 1);

    let cleared = session.execute_command(sequence_rate(None, 1)).unwrap();
    assert!(cleared.changed);
    assert_eq!(session.project_revision(), ProjectRevision::new(2));
    assert_eq!(session.project().timeline().sequence_frame_rate(), None);
    let undo_depth = session.history.undo.len();
    let redo_depth = session.history.redo.len();
    let clear_no_op = session.execute_command(sequence_rate(None, 2)).unwrap();
    assert!(!clear_no_op.changed);
    assert_eq!(session.history.undo.len(), undo_depth);
    assert_eq!(session.history.redo.len(), redo_depth);

    session.execute_command(undo(2)).unwrap();
    assert_eq!(session.project_revision(), ProjectRevision::new(3));
    assert_eq!(
        session.project().timeline().sequence_frame_rate(),
        Some(rate)
    );
    session.execute_command(redo(3)).unwrap();
    assert_eq!(session.project_revision(), ProjectRevision::new(4));
    assert_eq!(session.project().timeline().sequence_frame_rate(), None);

    for invalid in [
        json!({}),
        json!({"sequence_frame_rate": {"numerator": 0, "denominator": 1}}),
        json!({"sequence_frame_rate": {"numerator": 24, "denominator": 1, "extra": true}}),
    ] {
        assert_eq!(
            code(&session.execute_command(history_command(
                "timeline.sequence.set_frame_rate",
                session.project_revision().value(),
                invalid,
            ))),
            OperationErrorCode::InvalidArguments
        );
    }

    let query = QueryEnvelope {
        query_id: "timeline.sequence.settings".to_owned(),
        schema_version: 1,
        project_id: session.project_id(),
        project_instance_id: session.project_instance_id(),
        arguments: json!({"unexpected": true}),
    };
    assert_eq!(
        code(&session.execute_query(query)),
        OperationErrorCode::InvalidArguments
    );
    let transaction = transaction(
        vec![call(
            "timeline.sequence.set_frame_rate",
            1,
            json!({"sequence_frame_rate": {"numerator": 24, "denominator": 1}}),
        )],
        4,
    );
    assert_eq!(
        code(&session.execute_transaction(transaction)),
        OperationErrorCode::CommandNotAllowedInTransaction
    );
}
