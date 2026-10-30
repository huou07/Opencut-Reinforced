// Read queries: timeline pages, typed clip output and snapping.

use super::super::*;
use super::support::*;
use super::*;
use serde_json::{Value, json};
use std::str::FromStr;

#[test]
fn timeline_queries_are_read_only_canonical_and_bounded() {
    let mut session = fixed_session();
    session
        .execute_command(media_add(
            media_item(MEDIA_A, "file:///missing/offline.mov"),
            0,
        ))
        .unwrap();
    add_track(&mut session, TRACK_A, "video").unwrap();
    add_track(&mut session, TRACK_B, "audio").unwrap();
    insert_clip(
        &mut session,
        CLIP_B,
        TRACK_A,
        MEDIA_A,
        (4, 1),
        (2, 1),
        (1, 1),
    )
    .unwrap();
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
        CLIP_C,
        TRACK_A,
        MEDIA_A,
        (8, 1),
        (4, 1),
        (1, 1),
    )
    .unwrap();

    let before = session.project().clone();
    let undo = session.history.undo.clone();
    let tracks = session
        .execute_query(QueryEnvelope::timeline_tracks(
            session.project_id(),
            session.project_instance_id(),
        ))
        .unwrap();
    assert_eq!(tracks.summary.project_id, session.project_id());
    assert_eq!(
        tracks.summary.project_instance_id,
        session.project_instance_id()
    );
    assert_eq!(tracks.summary.project_revision, session.project_revision());
    assert_eq!(
        tracks.timeline_tracks.unwrap(),
        [
            TimelineTrackSummary {
                track_id: TrackId::from_str(TRACK_A).unwrap(),
                kind: TrackKind::Video,
                clip_count: 3,
            },
            TimelineTrackSummary {
                track_id: TrackId::from_str(TRACK_B).unwrap(),
                kind: TrackKind::Audio,
                clip_count: 0,
            },
        ]
    );

    let page = session
        .execute_query(QueryEnvelope::timeline_clips(
            session.project_id(),
            session.project_instance_id(),
            TrackId::from_str(TRACK_A).unwrap(),
            1,
            1,
        ))
        .unwrap()
        .timeline_clip_page
        .unwrap();
    assert_eq!(page.track_id.to_string(), TRACK_A);
    assert_eq!(page.items[0].clip_id.to_string(), CLIP_B);
    assert_eq!(
        page.items[0].timeline_start,
        RationalTime::new(4, 1).unwrap()
    );
    assert_eq!(page.total_count, 3);
    assert_eq!(page.offset, 1);
    assert_eq!(page.limit, 1);
    assert_eq!(page.next_offset, Some(2));

    let final_page = session
        .execute_query(QueryEnvelope::timeline_clips(
            session.project_id(),
            session.project_instance_id(),
            TrackId::from_str(TRACK_A).unwrap(),
            3,
            MAX_TIMELINE_CLIP_PAGE_SIZE,
        ))
        .unwrap()
        .timeline_clip_page
        .unwrap();
    assert!(final_page.items.is_empty());
    assert_eq!(final_page.next_offset, None);
    let beyond = session
        .execute_query(QueryEnvelope::timeline_clips(
            session.project_id(),
            session.project_instance_id(),
            TrackId::from_str(TRACK_A).unwrap(),
            4,
            1,
        ))
        .unwrap()
        .timeline_clip_page
        .unwrap();
    assert!(beyond.items.is_empty());

    let mut invalid = QueryEnvelope::timeline_clips(
        session.project_id(),
        session.project_instance_id(),
        TrackId::from_str(TRACK_A).unwrap(),
        0,
        MAX_TIMELINE_CLIP_PAGE_SIZE + 1,
    );
    assert_eq!(
        code(&session.execute_query(invalid.clone())),
        OperationErrorCode::InvalidArguments
    );
    invalid.arguments = json!({"track_id": TRACK_C, "offset": 0, "limit": 1});
    assert_eq!(
        code(&session.execute_query(invalid)),
        OperationErrorCode::TimelineTrackNotFound
    );
    assert_eq!(session.project(), &before);
    assert_eq!(session.history.undo, undo);
}

#[test]
fn typed_clip_commands_queries_locking_and_history_preserve_exact_time_and_identity() {
    let mut session = fixed_session();
    let text_track = TrackId::from_str(TRACK_A).unwrap();
    let caption_track = TrackId::from_str(TRACK_B).unwrap();
    for (track_id, kind) in [
        (text_track, TrackKind::Text),
        (caption_track, TrackKind::Caption),
    ] {
        session
            .execute_command(CommandEnvelope::add_timeline_track(
                session.project_id(),
                session.project_instance_id(),
                session.project_revision(),
                track_id,
                kind,
            ))
            .unwrap();
    }

    let text_clip = ClipId::from_str(CLIP_A).unwrap();
    let initial_content = ClipContent::Text {
        text: "Title".to_owned(),
        formatting: TextFormatting::default(),
    };
    let visual_settings = ClipSettings::Visual(VisualSettings::default());
    let inserted = session
        .execute_command(CommandEnvelope::insert_timeline_clip_content(
            session.project_id(),
            session.project_instance_id(),
            session.project_revision(),
            text_clip,
            text_track,
            RationalTime::new(1, 3).unwrap(),
            RationalTime::new(7, 3).unwrap(),
            initial_content.clone(),
            visual_settings.clone(),
        ))
        .unwrap();
    assert_eq!(inserted.after_revision, ProjectRevision::new(3));
    assert!(inserted.changed);

    let caption_clip = ClipId::from_str(CLIP_B).unwrap();
    session
        .execute_command(CommandEnvelope::insert_timeline_clip_content(
            session.project_id(),
            session.project_instance_id(),
            session.project_revision(),
            caption_clip,
            caption_track,
            RationalTime::new(3, 2).unwrap(),
            RationalTime::new(1, 2).unwrap(),
            ClipContent::Caption {
                text: "One caption cue".to_owned(),
                formatting: TextFormatting::default(),
            },
            visual_settings.clone(),
        ))
        .unwrap();

    let page = session
        .execute_query(QueryEnvelope::timeline_clips_v2(
            session.project_id(),
            session.project_instance_id(),
            text_track,
            0,
            10,
        ))
        .unwrap()
        .timeline_clip_page_v2
        .unwrap();
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].clip_id, text_clip);
    assert_eq!(
        page.items[0].timeline_start,
        RationalTime::new(1, 3).unwrap()
    );
    assert_eq!(
        page.items[0].timeline_duration,
        RationalTime::new(7, 3).unwrap()
    );
    assert_eq!(page.items[0].content, initial_content);
    assert_eq!(page.items[0].media_id(), None);
    assert_eq!(page.items[0].source_range(), None);

    let track_state = TrackState::new(true, true, false, false);
    session
        .execute_command(CommandEnvelope::set_timeline_track_state(
            session.project_id(),
            session.project_instance_id(),
            session.project_revision(),
            text_track,
            track_state,
        ))
        .unwrap();
    let locked_update = CommandEnvelope::update_timeline_clip(
        session.project_id(),
        session.project_instance_id(),
        session.project_revision(),
        text_clip,
        RationalTime::new(3, 1).unwrap(),
        ClipContent::Text {
            text: "Edited".to_owned(),
            formatting: TextFormatting::default(),
        },
        visual_settings.clone(),
    );
    assert_eq!(
        code(&session.execute_command(locked_update)),
        OperationErrorCode::TimelineTrackLocked
    );
    assert_eq!(session.project_revision(), ProjectRevision::new(5));

    session
        .execute_command(CommandEnvelope::undo(
            session.project_id(),
            session.project_instance_id(),
            session.project_revision(),
        ))
        .unwrap();
    let updated_content = ClipContent::Text {
        text: "Edited".to_owned(),
        formatting: TextFormatting::default(),
    };
    session
        .execute_command(CommandEnvelope::update_timeline_clip(
            session.project_id(),
            session.project_instance_id(),
            session.project_revision(),
            text_clip,
            RationalTime::new(3, 1).unwrap(),
            updated_content.clone(),
            visual_settings,
        ))
        .unwrap();
    let updated = &session.project().timeline().tracks()[0].clips()[0];
    assert_eq!(updated.id(), text_clip);
    assert_eq!(updated.timeline_start(), RationalTime::new(1, 3).unwrap());
    assert_eq!(
        updated.timeline_duration(),
        RationalTime::new(3, 1).unwrap()
    );
    assert_eq!(updated.content(), &updated_content);

    session
        .execute_command(CommandEnvelope::undo(
            session.project_id(),
            session.project_instance_id(),
            session.project_revision(),
        ))
        .unwrap();
    let restored = &session.project().timeline().tracks()[0].clips()[0];
    assert_eq!(restored.id(), text_clip);
    assert_eq!(
        restored.timeline_duration(),
        RationalTime::new(7, 3).unwrap()
    );
    assert_eq!(restored.content(), &initial_content);
    session
        .execute_command(CommandEnvelope::redo(
            session.project_id(),
            session.project_instance_id(),
            session.project_revision(),
        ))
        .unwrap();
    assert_eq!(
        session.project().timeline().tracks()[0].clips()[0].content(),
        &updated_content
    );
}

#[test]
fn timeline_snap_resolves_zero_clip_boundaries_and_move_end_anchor_exactly() {
    let session = snap_fixture();
    let before_revision = session.project_revision();

    let zero = session
        .execute_query(snap_query(
            &session,
            super::super::TimelineSnapOperation::Move,
            CLIP_A,
            Some(TRACK_B),
            (1, 20),
        ))
        .unwrap();
    let zero = zero.timeline_snap.unwrap();
    assert_eq!(zero.resolved_target_time, RationalTime::ZERO);
    assert!(zero.snapped);
    assert_eq!(zero.moving_anchor, TimelineSnapMovingAnchor::Start);
    assert_eq!(zero.target_kind, TimelineSnapTargetKind::TimelineZero);
    assert_eq!(zero.target_time, RationalTime::ZERO);
    assert_eq!(zero.target_track_id, None);
    assert_eq!(zero.target_clip_id, None);

    let clip_start = session
        .execute_query(snap_query(
            &session,
            super::super::TimelineSnapOperation::Move,
            CLIP_A,
            Some(TRACK_B),
            (161, 20),
        ))
        .unwrap()
        .timeline_snap
        .unwrap();
    assert_eq!(
        clip_start.resolved_target_time,
        RationalTime::new(8, 1).unwrap()
    );
    assert_eq!(clip_start.target_kind, TimelineSnapTargetKind::ClipStart);
    assert_eq!(clip_start.target_track_id.unwrap().to_string(), TRACK_B);
    assert_eq!(
        clip_start.target_clip_id.unwrap().to_string(),
        "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaac"
    );

    let outside = session
        .execute_query(snap_query(
            &session,
            super::super::TimelineSnapOperation::Move,
            CLIP_A,
            Some(TRACK_B),
            (51, 5),
        ))
        .unwrap()
        .timeline_snap
        .unwrap();
    assert!(!outside.snapped);
    assert_eq!(
        outside.resolved_target_time,
        RationalTime::new(51, 5).unwrap()
    );
    assert_eq!(outside.moving_anchor, TimelineSnapMovingAnchor::None);
    assert_eq!(outside.target_kind, TimelineSnapTargetKind::None);

    let end_anchor = session
        .execute_query(snap_query(
            &session,
            super::super::TimelineSnapOperation::Move,
            CLIP_A,
            Some(TRACK_B),
            (13, 2),
        ))
        .unwrap()
        .timeline_snap
        .unwrap();
    assert!(end_anchor.snapped);
    assert_eq!(end_anchor.moving_anchor, TimelineSnapMovingAnchor::End);
    assert_eq!(end_anchor.target_time, RationalTime::new(43, 5).unwrap());
    assert_eq!(
        end_anchor.resolved_target_time,
        RationalTime::new(33, 5).unwrap()
    );

    let trim_start = session
        .execute_query(snap_query(
            &session,
            super::super::TimelineSnapOperation::TrimStart,
            CLIP_A,
            None,
            (1, 20),
        ))
        .unwrap()
        .timeline_snap
        .unwrap();
    assert_eq!(trim_start.moving_anchor, TimelineSnapMovingAnchor::Start);
    assert_eq!(trim_start.target_kind, TimelineSnapTargetKind::TimelineZero);
    assert_eq!(trim_start.resolved_target_time, RationalTime::ZERO);

    let trim_end = session
        .execute_query(snap_query(
            &session,
            super::super::TimelineSnapOperation::TrimEnd,
            CLIP_A,
            None,
            (4, 1),
        ))
        .unwrap()
        .timeline_snap
        .unwrap();
    assert_eq!(trim_end.moving_anchor, TimelineSnapMovingAnchor::End);
    assert_eq!(trim_end.target_kind, TimelineSnapTargetKind::ClipStart);
    assert_eq!(trim_end.target_time, RationalTime::new(4, 1).unwrap());
    assert_eq!(session.project_revision(), before_revision);
}

#[test]
fn timeline_snap_tie_breaks_are_deterministic_and_exclude_the_active_clip() {
    let session = snap_fixture();
    let candidate_tie = session
        .execute_query(snap_query(
            &session,
            super::super::TimelineSnapOperation::Move,
            CLIP_A,
            Some(TRACK_B),
            (9, 1),
        ))
        .unwrap()
        .timeline_snap
        .unwrap();
    assert_eq!(candidate_tie.target_kind, TimelineSnapTargetKind::ClipEnd);
    assert_eq!(
        candidate_tie.target_clip_id.unwrap().to_string(),
        "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaac"
    );
    assert_eq!(candidate_tie.target_track_id.unwrap().to_string(), TRACK_B);

    let self_excluded = session
        .execute_query(snap_query(
            &session,
            super::super::TimelineSnapOperation::TrimStart,
            CLIP_A,
            None,
            (2, 1),
        ))
        .unwrap()
        .timeline_snap
        .unwrap();
    assert!(!self_excluded.snapped);

    let mut short = fixed_session();
    short
        .execute_command(media_add(media_item(MEDIA_A, "file:///missing/tie.mov"), 0))
        .unwrap();
    add_track(&mut short, TRACK_A, "video").unwrap();
    add_track(&mut short, TRACK_B, "video").unwrap();
    insert_clip(&mut short, CLIP_A, TRACK_A, MEDIA_A, (5, 1), (0, 1), (1, 5)).unwrap();
    insert_clip(
        &mut short,
        CLIP_B,
        TRACK_B,
        MEDIA_A,
        (51, 10),
        (1, 1),
        (1, 1),
    )
    .unwrap();
    let anchor_tie = short
        .execute_query(snap_query(
            &short,
            super::super::TimelineSnapOperation::Move,
            CLIP_A,
            Some(TRACK_B),
            (5, 1),
        ))
        .unwrap()
        .timeline_snap
        .unwrap();
    assert_eq!(anchor_tie.moving_anchor, TimelineSnapMovingAnchor::Start);
}

#[test]
fn timeline_snap_scans_unpaged_canonical_clips_and_keeps_query_read_only() {
    let mut session = fixed_session();
    session
        .execute_command(media_add(
            media_item(MEDIA_A, "file:///missing/unloaded.mov"),
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
        (1, 1),
    )
    .unwrap();
    for index in 0..101 {
        let clip_id = ClipId::generate().to_string();
        insert_clip(
            &mut session,
            &clip_id,
            TRACK_B,
            MEDIA_A,
            ((20 + index * 2) as i64, 1),
            (0, 1),
            (1, 1),
        )
        .unwrap();
    }
    let before_revision = session.project_revision();
    let before_history = session.history.undo.clone();
    let result = session
        .execute_query(snap_query(
            &session,
            super::super::TimelineSnapOperation::Move,
            CLIP_A,
            Some(TRACK_B),
            (220, 1),
        ))
        .unwrap()
        .timeline_snap
        .unwrap();
    assert!(result.snapped);
    assert_eq!(result.target_kind, TimelineSnapTargetKind::ClipStart);
    assert_eq!(result.target_time, RationalTime::new(220, 1).unwrap());
    assert_eq!(
        result.resolved_target_time,
        RationalTime::new(220, 1).unwrap()
    );
    assert_eq!(session.project_revision(), before_revision);
    assert_eq!(session.history.undo, before_history);
}

#[test]
fn timeline_snap_preconditions_and_arguments_are_strict() {
    let session = snap_fixture();
    let before_revision = session.project_revision();
    let base = json!({
        "operation": "move",
        "clip_id": CLIP_A,
        "target_track_id": TRACK_B,
        "target_time": rational_json(0, 1),
    });
    let mut unknown = QueryEnvelope::timeline_snap(
        session.project_id(),
        session.project_instance_id(),
        super::super::TimelineSnapOperation::Move,
        ClipId::from_str(CLIP_A).unwrap(),
        Some(TrackId::from_str(TRACK_B).unwrap()),
        RationalTime::ZERO,
    );
    unknown.arguments = json!({
        "operation": "move",
        "clip_id": CLIP_A,
        "target_track_id": TRACK_B,
        "target_time": rational_json(0, 1),
        "threshold": rational_json(1, 8),
    });
    assert_eq!(
        code(&session.execute_query(unknown)),
        OperationErrorCode::InvalidArguments
    );

    let mut missing_track = QueryEnvelope::timeline_snap(
        session.project_id(),
        session.project_instance_id(),
        super::super::TimelineSnapOperation::Move,
        ClipId::from_str(CLIP_A).unwrap(),
        None,
        RationalTime::ZERO,
    );
    missing_track.arguments = base.clone();
    missing_track.arguments["target_track_id"] = Value::Null;
    assert_eq!(
        code(&session.execute_query(missing_track)),
        OperationErrorCode::InvalidArguments
    );

    let trim_track = QueryEnvelope::timeline_snap(
        session.project_id(),
        session.project_instance_id(),
        super::super::TimelineSnapOperation::TrimStart,
        ClipId::from_str(CLIP_A).unwrap(),
        Some(TrackId::from_str(TRACK_B).unwrap()),
        RationalTime::ZERO,
    );
    assert_eq!(
        code(&session.execute_query(trim_track)),
        OperationErrorCode::InvalidArguments
    );

    let wrong_kind = session
        .execute_query(snap_query(
            &session,
            super::super::TimelineSnapOperation::Move,
            CLIP_A,
            Some(TRACK_C),
            (0, 1),
        ))
        .unwrap_err();
    assert_eq!(wrong_kind.code, OperationErrorCode::InvalidArguments);

    let missing_clip = session
        .execute_query(snap_query(
            &session,
            super::super::TimelineSnapOperation::Move,
            "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbd",
            Some(TRACK_B),
            (0, 1),
        ))
        .unwrap_err();
    assert_eq!(missing_clip.code, OperationErrorCode::TimelineClipNotFound);
    assert_eq!(session.project_revision(), before_revision);
}
