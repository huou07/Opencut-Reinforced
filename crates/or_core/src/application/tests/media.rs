// Media add, remove, relink and get/list queries.

use super::super::*;
use super::support::*;
use super::*;
use serde_json::{Value, json};
use std::str::FromStr;

#[test]
fn media_operation_error_codes_serialize_to_stable_machine_names() {
    for (code, expected) in [
        (
            OperationErrorCode::MediaIdAlreadyExists,
            "MEDIA_ID_ALREADY_EXISTS",
        ),
        (
            OperationErrorCode::MediaSourceAlreadyExists,
            "MEDIA_SOURCE_ALREADY_EXISTS",
        ),
        (OperationErrorCode::MediaNotFound, "MEDIA_NOT_FOUND"),
        (OperationErrorCode::MediaInUse, "MEDIA_IN_USE"),
    ] {
        assert_eq!(
            serde_json::to_string(&code).unwrap(),
            format!("\"{expected}\"")
        );
    }
}

#[test]
fn media_add_mutates_once_and_reports_the_persisted_item() {
    let mut session = fixed_session();
    let item = media_item(
        "00000000-0000-4000-8000-000000000001",
        "file:///media/one.mkv",
    );

    let result = session.execute_command(media_add(item.clone(), 0)).unwrap();

    assert_eq!(result.before_revision, ProjectRevision::new(0));
    assert_eq!(result.after_revision, ProjectRevision::new(1));
    assert!(result.changed);
    assert_eq!(session.project().media_items(), std::slice::from_ref(&item));
    assert_eq!(session.history.undo.len(), 1);
    assert!(session.history.redo.is_empty());
    assert!(matches!(
        result.change_set.changes(),
        [ProjectChange::MediaAdded { item: added, index: 0 }] if added == &item
    ));
    let persisted = decode_project(&encode_project(session.project()).unwrap()).unwrap();
    assert_eq!(persisted.media_items(), &[item]);
    assert_eq!(persisted.revision(), ProjectRevision::new(1));
}

#[test]
fn duplicate_media_id_and_source_are_rejected_without_state_change() {
    let mut session = fixed_session();
    let original = media_item(
        "00000000-0000-4000-8000-000000000001",
        "file:///media/one.mkv",
    );
    session
        .execute_command(media_add(original.clone(), 0))
        .unwrap();
    let before = session.project().clone();
    let history_len = session.history.undo.len();

    let duplicate_id = media_item(
        "00000000-0000-4000-8000-000000000001",
        "file:///media/two.mkv",
    );
    assert_eq!(
        code(&session.execute_command(media_add(duplicate_id, 1))),
        OperationErrorCode::MediaIdAlreadyExists
    );

    let duplicate_source = media_item(
        "00000000-0000-4000-8000-000000000002",
        "file:///media/one.mkv",
    );
    assert_eq!(
        code(&session.execute_command(media_add(duplicate_source, 1))),
        OperationErrorCode::MediaSourceAlreadyExists
    );
    assert_eq!(session.project(), &before);
    assert_eq!(session.history.undo.len(), history_len);
    assert!(session.history.redo.is_empty());
}

#[test]
fn stale_prepared_media_add_returns_revision_conflict_without_retry() {
    let mut session = fixed_session();
    let prepared = media_item(
        "00000000-0000-4000-8000-000000000001",
        "file:///media/one.mkv",
    );
    let captured_revision_command = media_add(prepared, 0);
    session
        .execute_command(rename("Changed during probe", 0))
        .unwrap();
    let before = session.project().clone();
    let undo_len = session.history.undo.len();

    assert_eq!(
        code(&session.execute_command(captured_revision_command)),
        OperationErrorCode::RevisionConflict
    );
    assert_eq!(session.project(), &before);
    assert_eq!(session.project_revision(), ProjectRevision::new(1));
    assert_eq!(session.history.undo.len(), undo_len);
    assert!(session.history.redo.is_empty());
}

#[test]
fn media_add_rejects_invalid_arguments_and_is_not_transactional() {
    let mut session = fixed_session();
    for arguments in [json!({}), json!({ "item": {}, "extra": true })] {
        let mut command = media_add(
            media_item(
                "00000000-0000-4000-8000-000000000001",
                "file:///media/one.mkv",
            ),
            0,
        );
        command.arguments = arguments;
        assert_eq!(
            code(&session.execute_command(command)),
            OperationErrorCode::InvalidArguments
        );
    }
    let result = session.execute_transaction(transaction(
        vec![call(
            "media.add",
            1,
            json!({ "item": media_item(
                "00000000-0000-4000-8000-000000000001",
                "file:///media/one.mkv"
            ) }),
        )],
        0,
    ));
    assert_eq!(
        code(&result),
        OperationErrorCode::CommandNotAllowedInTransaction
    );
    assert_eq!(session.project_revision(), ProjectRevision::INITIAL);
    assert!(session.project().media_items().is_empty());
    assert!(session.history.undo.is_empty());
}

#[test]
fn media_remove_rejects_unknown_id_and_success_increments_once() {
    let mut session = fixed_session();
    assert_eq!(
        code(&session.execute_command(media_remove("00000000-0000-4000-8000-000000000001", 0))),
        OperationErrorCode::MediaNotFound
    );
    let item = media_item(
        "00000000-0000-4000-8000-000000000001",
        "file:///media/one.mkv",
    );
    session.execute_command(media_add(item.clone(), 0)).unwrap();

    let result = session
        .execute_command(media_remove("00000000-0000-4000-8000-000000000001", 1))
        .unwrap();
    assert_eq!(result.before_revision, ProjectRevision::new(1));
    assert_eq!(result.after_revision, ProjectRevision::new(2));
    assert!(session.project().media_items().is_empty());
    assert!(matches!(
        result.change_set.changes(),
        [ProjectChange::MediaRemoved { item: removed, index: 0 }] if removed == &item
    ));
    assert_eq!(
        code(&session.execute_command(media_remove("00000000-0000-4000-8000-000000000001", 2))),
        OperationErrorCode::MediaNotFound
    );
    assert_eq!(session.project_revision(), ProjectRevision::new(2));
}

#[test]
fn media_relink_preserves_identity_and_timeline_references_through_history() {
    let mut session = fixed_session();
    let original = media_item(
        "00000000-0000-4000-8000-000000000001",
        "file:///media/original.mkv",
    );
    let replacement = media_item(
        "00000000-0000-4000-8000-000000000001",
        "file:///media/relinked.mkv",
    );
    let media_id = original.id();
    session
        .execute_command(media_add(original.clone(), 0))
        .unwrap();
    session
        .project
        .set_timeline_for_test(ProjectTimeline::from_tracks_for_codec(vec![
            TimelineTrack::from_parts_for_codec(
                TrackId::from_str("22222222-2222-4222-8222-222222222222").unwrap(),
                TrackKind::Video,
                vec![TimelineClip::from_parts_for_codec(
                    ClipId::from_str("33333333-3333-4333-8333-333333333333").unwrap(),
                    media_id,
                    RationalTime::ZERO,
                    TimeRange::new(RationalTime::ZERO, RationalTime::new(1, 1).unwrap()).unwrap(),
                )],
            ),
        ]));

    let result = session
        .execute_command(media_relink(replacement.clone(), 1))
        .unwrap();
    assert_eq!(result.after_revision, ProjectRevision::new(2));
    assert_eq!(
        session.project().media_items(),
        std::slice::from_ref(&replacement)
    );
    assert!(session.project().timeline().references_media(media_id));
    assert!(matches!(
        result.change_set.changes(),
        [ProjectChange::MediaRelinked { before, after, index: 0 }]
            if before == &original && after == &replacement
    ));

    let undone = session.execute_command(undo(2)).unwrap();
    assert_eq!(
        session.project().media_items(),
        std::slice::from_ref(&original)
    );
    assert!(session.project().timeline().references_media(media_id));
    assert!(matches!(
        undone.change_set.changes(),
        [ProjectChange::MediaRelinked { before, after, index: 0 }]
            if before == &replacement && after == &original
    ));

    session.execute_command(redo(3)).unwrap();
    assert_eq!(session.project().media_items(), &[replacement]);
    assert_eq!(session.project_revision(), ProjectRevision::new(4));
}

#[test]
fn media_relink_rejects_incompatible_or_duplicate_sources_atomically() {
    let mut session = fixed_session();
    let original = media_item(
        "00000000-0000-4000-8000-000000000001",
        "file:///media/original.mkv",
    );
    let other = media_item(
        "00000000-0000-4000-8000-000000000002",
        "file:///media/other.mkv",
    );
    session
        .execute_command(media_add(original.clone(), 0))
        .unwrap();
    session
        .execute_command(media_add(other.clone(), 1))
        .unwrap();
    session
        .project
        .set_timeline_for_test(ProjectTimeline::from_tracks_for_codec(vec![
            TimelineTrack::from_parts_for_codec(
                TrackId::from_str("22222222-2222-4222-8222-222222222222").unwrap(),
                TrackKind::Video,
                vec![TimelineClip::from_parts_for_codec(
                    ClipId::from_str("33333333-3333-4333-8333-333333333333").unwrap(),
                    original.id(),
                    RationalTime::ZERO,
                    TimeRange::new(RationalTime::ZERO, RationalTime::new(1, 1).unwrap()).unwrap(),
                )],
            ),
        ]));
    let before_project = session.project().clone();
    let before_undo = session.history.undo.clone();

    let incompatible = audio_media_item(&original.id().to_string(), "file:///media/audio-only.mkv");
    assert_eq!(
        code(&session.execute_command(media_relink(incompatible, 2))),
        OperationErrorCode::TimelineMediaIncompatible
    );
    let duplicate = media_item(&original.id().to_string(), other.source().uri());
    assert_eq!(
        code(&session.execute_command(media_relink(duplicate, 2))),
        OperationErrorCode::MediaSourceAlreadyExists
    );
    assert_eq!(session.project(), &before_project);
    assert_eq!(session.history.undo, before_undo);
}

#[test]
fn media_remove_rejects_referenced_media_without_changing_state_or_history() {
    let mut session = fixed_session();
    let item = media_item(
        "00000000-0000-4000-8000-000000000001",
        "file:///media/one.mkv",
    );
    let media_id = item.id();
    session.execute_command(media_add(item, 0)).unwrap();
    session.execute_command(rename("B", 1)).unwrap();
    session.execute_command(undo(2)).unwrap();
    assert_eq!(session.history.redo.len(), 1);

    session
        .project
        .set_timeline_for_test(ProjectTimeline::from_tracks_for_codec(vec![
            TimelineTrack::from_parts_for_codec(
                TrackId::from_str("22222222-2222-4222-8222-222222222222").unwrap(),
                TrackKind::Video,
                vec![TimelineClip::from_parts_for_codec(
                    crate::ClipId::from_str("33333333-3333-4333-8333-333333333333").unwrap(),
                    media_id,
                    RationalTime::ZERO,
                    TimeRange::new(RationalTime::ZERO, RationalTime::new(1, 1).unwrap()).unwrap(),
                )],
            ),
        ]));
    let before_project = session.project().clone();
    let before_undo = session.history.undo.clone();
    let before_redo = session.history.redo.clone();

    let result = session.execute_command(media_remove(&media_id.to_string(), 3));

    assert!(matches!(
        result,
        Err(error) if error.code == OperationErrorCode::MediaInUse
    ));
    assert_eq!(session.project(), &before_project);
    assert_eq!(session.history.undo, before_undo);
    assert_eq!(session.history.redo, before_redo);
    assert_eq!(session.project_revision(), ProjectRevision::new(3));
    assert_eq!(session.project().media_items().len(), 1);
    assert_eq!(session.project().timeline(), before_project.timeline());
}

#[test]
fn undo_and_redo_media_add_restore_the_same_identity_and_metadata() {
    let mut session = fixed_session();
    let item = media_item(
        "00000000-0000-4000-8000-000000000001",
        "file:///media/one.mkv",
    );
    session.execute_command(media_add(item.clone(), 0)).unwrap();

    let undo_result = session.execute_command(undo(1)).unwrap();
    assert!(session.project().media_items().is_empty());
    assert_eq!(undo_result.after_revision, ProjectRevision::new(2));
    assert!(matches!(
        undo_result.change_set.changes(),
        [ProjectChange::MediaRemoved { item: removed, index: 0 }] if removed == &item
    ));

    let redo_result = session.execute_command(redo(2)).unwrap();
    assert_eq!(session.project().media_items(), std::slice::from_ref(&item));
    assert_eq!(redo_result.after_revision, ProjectRevision::new(3));
    assert!(matches!(
        redo_result.change_set.changes(),
        [ProjectChange::MediaAdded { item: added, index: 0 }] if added == &item
    ));
}

#[test]
fn undo_remove_restores_original_media_order_and_redo_removes_it_again() {
    let mut session = fixed_session();
    let a = media_item(
        "00000000-0000-4000-8000-000000000001",
        "file:///media/a.mkv",
    );
    let b = media_item(
        "00000000-0000-4000-8000-000000000002",
        "file:///media/b.mkv",
    );
    let c = media_item(
        "00000000-0000-4000-8000-000000000003",
        "file:///media/c.mkv",
    );
    session.execute_command(media_add(a.clone(), 0)).unwrap();
    session.execute_command(media_add(b.clone(), 1)).unwrap();
    session.execute_command(media_add(c.clone(), 2)).unwrap();
    session
        .execute_command(media_remove("00000000-0000-4000-8000-000000000002", 3))
        .unwrap();
    assert_eq!(session.project().media_items(), &[a.clone(), c.clone()]);

    let undo_result = session.execute_command(undo(4)).unwrap();
    assert_eq!(
        session.project().media_items(),
        &[a.clone(), b.clone(), c.clone()]
    );
    assert!(matches!(
        undo_result.change_set.changes(),
        [ProjectChange::MediaAdded { item, index: 1 }] if item == &b
    ));

    session.execute_command(redo(5)).unwrap();
    assert_eq!(session.project().media_items(), &[a, c]);
    assert_eq!(session.project_revision(), ProjectRevision::new(6));
}

#[test]
fn media_history_conflicts_do_not_partially_change_state_or_stacks() {
    let mut session = fixed_session();
    let item = media_item(
        "00000000-0000-4000-8000-000000000001",
        "file:///media/one.mkv",
    );
    session.execute_command(media_add(item, 0)).unwrap();
    session
        .project
        .remove_media_for_command(0, ProjectRevision::new(1));
    let before = session.project().clone();
    let undo_len = session.history.undo.len();
    let redo_len = session.history.redo.len();

    assert_eq!(
        code(&session.execute_command(undo(1))),
        OperationErrorCode::HistoryConflict
    );
    assert_eq!(session.project(), &before);
    assert_eq!(session.history.undo.len(), undo_len);
    assert_eq!(session.history.redo.len(), redo_len);
}

#[test]
fn media_list_pages_are_bounded_ordered_and_read_only() {
    let mut session = fixed_session();
    let expected_ids = (0..250)
        .map(|index| {
            let item = MediaItem::new(
                MediaId::generate(),
                MediaSourceRef::local_file(format!("file:///generated/{index}.mkv")).unwrap(),
                MediaMetadata::from_probe(vec!["matroska".to_owned()], None, 42, Vec::new()),
            )
            .unwrap();
            let id = item.id();
            let revision = session.project_revision().value();
            session.execute_command(media_add(item, revision)).unwrap();
            id
        })
        .collect::<Vec<_>>();
    let before = session.project().clone();
    let mut collected = Vec::new();
    let mut offset = 0;

    loop {
        let result = session
            .execute_query(QueryEnvelope::media_list(
                session.project_id(),
                session.project_instance_id(),
                offset,
                MAX_MEDIA_PAGE_SIZE,
            ))
            .unwrap();
        let page = result.media_page.as_ref().unwrap();
        assert_eq!(page.total_count, expected_ids.len());
        assert_eq!(page.offset, offset);
        assert!(page.items.len() <= MAX_MEDIA_PAGE_SIZE);
        let encoded = serde_json::to_vec(&result).unwrap();
        assert!(encoded.len() < 1024 * 1024);
        let wire = serde_json::from_slice::<Value>(&encoded).unwrap();
        let wire = wire.as_object().unwrap();
        assert!(!wire.contains_key("media_item"));
        assert!(!wire.contains_key("timeline"));
        assert!(!wire.contains_key("timeline_tracks"));
        assert!(!wire.contains_key("timeline_clip_page"));
        assert_eq!(
            serde_json::from_slice::<super::super::QueryResult>(&encoded).unwrap(),
            result
        );
        collected.extend(page.items.iter().map(MediaItem::id));
        match page.next_offset {
            Some(next) => offset = next,
            None => break,
        }
    }

    assert_eq!(collected, expected_ids);
    let beyond_end = session
        .execute_query(QueryEnvelope::media_list(
            session.project_id(),
            session.project_instance_id(),
            1_000,
            MAX_MEDIA_PAGE_SIZE,
        ))
        .unwrap()
        .media_page
        .unwrap();
    assert!(beyond_end.items.is_empty());
    assert_eq!(beyond_end.total_count, expected_ids.len());
    assert_eq!(beyond_end.next_offset, None);
    assert_eq!(session.project(), &before);
}

#[test]
fn media_list_rejects_invalid_page_bounds_and_argument_shapes() {
    let session = fixed_session();
    for arguments in [
        json!({ "offset": 0, "limit": 0 }),
        json!({ "offset": 0, "limit": MAX_MEDIA_PAGE_SIZE + 1 }),
        json!({ "offset": "0", "limit": 1 }),
        json!({ "offset": 0, "limit": 1, "extra": true }),
    ] {
        let mut query =
            QueryEnvelope::media_list(session.project_id(), session.project_instance_id(), 0, 1);
        query.arguments = arguments;
        assert_eq!(
            code(&session.execute_query(query)),
            OperationErrorCode::InvalidArguments
        );
    }
    assert_eq!(session.project_revision(), ProjectRevision::INITIAL);
}

#[test]
fn media_get_returns_one_persisted_item_without_mutating_project_or_revision() {
    let mut session = fixed_session();
    let item = media_item(
        "00000000-0000-4000-8000-000000000001",
        "file:///media/one.mkv",
    );
    session.execute_command(media_add(item.clone(), 0)).unwrap();
    let before = session.project().clone();
    let result = session
        .execute_query(QueryEnvelope::media_get(
            session.project_id(),
            session.project_instance_id(),
            item.id(),
        ))
        .unwrap();

    assert_eq!(result.query_id, "media.get");
    assert_eq!(result.media_page, None);
    assert_eq!(result.media_item.as_deref(), Some(&item));
    let wire = serde_json::to_value(&result).unwrap();
    assert!(!wire.as_object().unwrap().contains_key("timeline"));
    assert!(!wire.as_object().unwrap().contains_key("timeline_tracks"));
    assert!(!wire.as_object().unwrap().contains_key("timeline_clip_page"));
    assert_eq!(
        serde_json::from_value::<super::super::QueryResult>(wire).unwrap(),
        result
    );
    assert_eq!(session.project(), &before);
    assert_eq!(session.project_revision(), ProjectRevision::new(1));
}

#[test]
fn media_get_rejects_unknown_ids_and_invalid_arguments() {
    let session = fixed_session();
    let missing = MediaId::generate();
    assert_eq!(
        code(&session.execute_query(QueryEnvelope::media_get(
            session.project_id(),
            session.project_instance_id(),
            missing,
        ))),
        OperationErrorCode::MediaNotFound
    );

    let mut invalid =
        QueryEnvelope::media_get(session.project_id(), session.project_instance_id(), missing);
    invalid.arguments = json!({ "media_id": missing, "extra": true });
    assert_eq!(
        code(&session.execute_query(invalid)),
        OperationErrorCode::InvalidArguments
    );
    assert_eq!(session.project_revision(), ProjectRevision::INITIAL);
}
