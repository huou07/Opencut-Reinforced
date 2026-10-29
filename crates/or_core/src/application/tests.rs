    use super::{
        ApplicationRequest, ApplicationResponse, COMMANDS, CURRENT_TRANSACTION_SCHEMA_VERSION,
        ChangeSet, CommandCall, CommandDescriptor, CommandEnvelope, OperationErrorCode,
        ProjectChange, ProjectSession, QUERIES, QueryDescriptor, QueryEnvelope, TimelineClipState,
        TimelineSnapMovingAnchor, TimelineSnapTargetKind, TimelineTrackSummary,
        TransactionEnvelope, command_catalog, query_catalog,
    };
    use crate::{
        AudioStreamMetadata, ClipContent, ClipId, ClipSettings, Crop, MAX_MEDIA_PAGE_SIZE,
        MAX_TIMELINE_CLIP_PAGE_SIZE, MAX_TIMELINE_CLIPS, MAX_TIMELINE_MARKER_LABEL_BYTES,
        MAX_TIMELINE_MARKER_PAGE_SIZE, MAX_TIMELINE_MARKERS, MAX_TIMELINE_TRACKS, MarkerId,
        MediaId, MediaItem, MediaMetadata, MediaSourceRef, MediaStreamMetadata, Opacity,
        ProjectDocument, ProjectId, ProjectInstanceId, ProjectRevision, ProjectTimeline, RationalRate, RationalTime,
        TextFormatting, TimeRange, TimelineClip, TimelineMarker, TimelineTrack, TrackId, TrackKind,
        TrackState, Transform, VideoStreamMetadata, VisualSettings, decode_project, encode_project,
    };
    use serde_json::{Value, json};
    use std::{num::NonZeroU32, str::FromStr};

    use super::timeline::*;

    const PROJECT_ID: &str = "01234567-89ab-4def-8123-456789abcdef";
    const OTHER_PROJECT_ID: &str = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa";
    const INSTANCE_ID: &str = "fedcba98-7654-4cba-8fed-cba987654321";
    const OTHER_INSTANCE_ID: &str = "11111111-1111-4111-8111-111111111111";
    const TRACK_A: &str = "22222222-2222-4222-8222-222222222222";
    const TRACK_B: &str = "88888888-8888-4888-8888-888888888888";
    const TRACK_C: &str = "99999999-9999-4999-8999-999999999999";
    const TRACK_D: &str = "eeeeeeee-eeee-4eee-8eee-eeeeeeeeeeee";
    const CLIP_A: &str = "33333333-3333-4333-8333-333333333333";
    const CLIP_B: &str = "55555555-5555-4555-8555-555555555555";
    const CLIP_C: &str = "66666666-6666-4666-8666-666666666666";
    const CLIP_D: &str = "99999999-9999-4999-8999-999999999998";
    const CLIP_E: &str = "eeeeeeee-eeee-4eee-8eee-eeeeeeeeeeef";
    const MEDIA_A: &str = "44444444-4444-4444-8444-444444444444";
    const MEDIA_B: &str = "77777777-7777-4777-8777-777777777777";
    const MEDIA_C: &str = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaab";
    const MEDIA_D: &str = "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb";
    const MARKER_A: &str = "11111111-1111-4111-8111-111111111111";
    const MARKER_B: &str = "33333333-3333-4333-8333-333333333333";
    const MARKER_C: &str = "55555555-5555-4555-8555-555555555555";

    fn project() -> ProjectDocument {
        decode_project(&format!(
            r#"{{"format":"opencut-reinforced-project","schema_version":1,"project":{{"id":"{PROJECT_ID}","revision":0,"name":"A"}}}}"#
        ))
        .unwrap()
    }

    fn fixed_session() -> ProjectSession {
        ProjectSession {
            project: project(),
            project_instance_id: ProjectInstanceId::from_str(INSTANCE_ID).unwrap(),
            history: super::SessionHistory::default(),
        }
    }

    fn media_item(id: &str, uri: &str) -> MediaItem {
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

    fn audio_media_item(id: &str, uri: &str) -> MediaItem {
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

    fn video_media_item_with_duration(
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

    fn audio_video_media_item(id: &str, uri: &str) -> MediaItem {
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

    fn media_add(item: MediaItem, revision: u64) -> CommandEnvelope {
        CommandEnvelope::add_media(
            ProjectId::from_str(PROJECT_ID).unwrap(),
            ProjectInstanceId::from_str(INSTANCE_ID).unwrap(),
            ProjectRevision::new(revision),
            item,
        )
    }

    fn media_remove(id: &str, revision: u64) -> CommandEnvelope {
        CommandEnvelope::remove_media(
            ProjectId::from_str(PROJECT_ID).unwrap(),
            ProjectInstanceId::from_str(INSTANCE_ID).unwrap(),
            ProjectRevision::new(revision),
            MediaId::from_str(id).unwrap(),
        )
    }

    fn media_relink(item: MediaItem, revision: u64) -> CommandEnvelope {
        CommandEnvelope::relink_media(
            ProjectId::from_str(PROJECT_ID).unwrap(),
            ProjectInstanceId::from_str(INSTANCE_ID).unwrap(),
            ProjectRevision::new(revision),
            item,
        )
    }

    fn session_with_state(name: &str, revision: u64) -> ProjectSession {
        let project = decode_project(&format!(
            r#"{{"format":"opencut-reinforced-project","schema_version":1,"project":{{"id":"{PROJECT_ID}","revision":{revision},"name":"{name}"}}}}"#
        ))
        .unwrap();
        ProjectSession {
            project,
            project_instance_id: ProjectInstanceId::from_str(INSTANCE_ID).unwrap(),
            history: super::SessionHistory::default(),
        }
    }

    fn rename(name: &str, revision: u64) -> CommandEnvelope {
        CommandEnvelope {
            command_id: "project.rename".to_owned(),
            schema_version: 1,
            project_id: ProjectId::from_str(PROJECT_ID).unwrap(),
            project_instance_id: ProjectInstanceId::from_str(INSTANCE_ID).unwrap(),
            expected_project_revision: ProjectRevision::new(revision),
            arguments: json!({ "name": name }),
        }
    }

    fn call(command_id: &str, schema_version: u64, arguments: Value) -> CommandCall {
        CommandCall {
            command_id: command_id.to_owned(),
            schema_version,
            arguments,
        }
    }

    fn rename_call(name: &str) -> CommandCall {
        call("project.rename", 1, json!({ "name": name }))
    }

    fn transaction(commands: Vec<CommandCall>, revision: u64) -> TransactionEnvelope {
        TransactionEnvelope {
            schema_version: CURRENT_TRANSACTION_SCHEMA_VERSION,
            project_id: ProjectId::from_str(PROJECT_ID).unwrap(),
            project_instance_id: ProjectInstanceId::from_str(INSTANCE_ID).unwrap(),
            expected_project_revision: ProjectRevision::new(revision),
            commands,
        }
    }

    fn history_command(command_id: &str, revision: u64, arguments: Value) -> CommandEnvelope {
        CommandEnvelope {
            command_id: command_id.to_owned(),
            schema_version: 1,
            project_id: ProjectId::from_str(PROJECT_ID).unwrap(),
            project_instance_id: ProjectInstanceId::from_str(INSTANCE_ID).unwrap(),
            expected_project_revision: ProjectRevision::new(revision),
            arguments,
        }
    }

    fn undo(revision: u64) -> CommandEnvelope {
        history_command("history.undo", revision, json!({}))
    }

    fn redo(revision: u64) -> CommandEnvelope {
        history_command("history.redo", revision, json!({}))
    }

    fn sequence_rate(rate: Option<RationalRate>, revision: u64) -> CommandEnvelope {
        CommandEnvelope::set_timeline_sequence_frame_rate(
            ProjectId::from_str(PROJECT_ID).unwrap(),
            ProjectInstanceId::from_str(INSTANCE_ID).unwrap(),
            ProjectRevision::new(revision),
            rate,
        )
    }

    fn execute(
        session: &mut ProjectSession,
        command_id: &str,
        arguments: Value,
    ) -> Result<super::CommandResult, super::OperationError> {
        session.execute_command(history_command(
            command_id,
            session.project_revision().value(),
            arguments,
        ))
    }

    fn add_track(
        session: &mut ProjectSession,
        id: &str,
        kind: &str,
    ) -> Result<super::CommandResult, super::OperationError> {
        execute(
            session,
            "timeline.track.add",
            json!({"track_id": id, "kind": kind}),
        )
    }

    fn rational_json(numerator: i64, denominator: u32) -> Value {
        json!({"numerator": numerator, "denominator": denominator})
    }

    fn marker_id(value: &str) -> MarkerId {
        value.parse().unwrap()
    }

    fn marker_add(
        session: &mut ProjectSession,
        marker_id: &str,
        timeline_time: (i64, u32),
        label: &str,
    ) -> Result<super::CommandResult, super::OperationError> {
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

    fn snap_query(
        session: &ProjectSession,
        operation: super::TimelineSnapOperation,
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

    fn insert_clip(
        session: &mut ProjectSession,
        clip_id: &str,
        track_id: &str,
        media_id: &str,
        timeline_start: (i64, u32),
        source_start: (i64, u32),
        duration: (i64, u32),
    ) -> Result<super::CommandResult, super::OperationError> {
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
    fn insert_clip_mode(
        session: &mut ProjectSession,
        clip_id: &str,
        track_id: &str,
        media_id: &str,
        timeline_start: (i64, u32),
        duration: (i64, u32),
        mode: &str,
        split_clip_id: Option<&str>,
    ) -> Result<super::CommandResult, super::OperationError> {
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
    fn clip_spans(
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

    fn seed_media_and_track(session: &mut ProjectSession, media_id: &str, track_id: &str) {
        session
            .execute_command(media_add(
                media_item(media_id, "file:///missing/offline.mov"),
                session.project_revision().value(),
            ))
            .unwrap();
        add_track(session, track_id, "video").unwrap();
    }

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
            super::OperationErrorCode::TimelineOverwriteSplitRequiresClipId,
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
            super::OperationErrorCode::TimelineOverlap,
            "the default mode keeps its old meaning"
        );
    }

    fn insert_clip_at(
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

    fn ripple_trim_end(
        session: &mut ProjectSession,
        clip_id: &str,
        target: (i64, u32),
        ripple: bool,
    ) -> Result<super::CommandResult, super::OperationError> {
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

    #[test]
    fn ripple_trim_pulls_every_later_clip_left() {
        let mut session = session_with_state("ripple-trim", 0);
        seed_media_and_track(&mut session, MEDIA_A, TRACK_A);
        insert_clip_at(&mut session, CLIP_A, TRACK_A, MEDIA_A, (0, 1), (4, 1));
        insert_clip_at(&mut session, CLIP_B, TRACK_A, MEDIA_A, (4, 1), (2, 1));
        insert_clip_at(&mut session, CLIP_C, TRACK_A, MEDIA_A, (6, 1), (2, 1));
        let revision_before = session.project_revision();

        ripple_trim_end(&mut session, CLIP_A, (2, 1), true).unwrap();

        let spans = clip_spans(&session, TRACK_A);
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[0].2, RationalTime::new(2, 1).unwrap());
        assert_eq!(
            spans[1].1,
            RationalTime::new(2, 1).unwrap(),
            "the next clip closes the gap"
        );
        assert_eq!(
            spans[2].1,
            RationalTime::new(4, 1).unwrap(),
            "and the one after it moves by the same delta"
        );
        assert_eq!(spans[1].2, RationalTime::new(2, 1).unwrap());
        assert_eq!(
            session.project_revision().value(),
            revision_before.value() + 1,
            "a ripple trim is one revision"
        );

        execute(&mut session, "history.undo", json!({})).unwrap();
        let restored = clip_spans(&session, TRACK_A);
        assert_eq!(restored[0].2, RationalTime::new(4, 1).unwrap());
        assert_eq!(
            restored[1].1,
            RationalTime::new(4, 1).unwrap(),
            "one undo restores every shifted clip"
        );
        assert_eq!(restored[2].1, RationalTime::new(6, 1).unwrap());
    }

    #[test]
    fn ripple_trim_opens_a_gap_when_the_clip_lengthens() {
        let mut session = session_with_state("ripple-lengthen", 0);
        seed_media_and_track(&mut session, MEDIA_A, TRACK_A);
        insert_clip_at(&mut session, CLIP_A, TRACK_A, MEDIA_A, (0, 1), (2, 1));
        insert_clip_at(&mut session, CLIP_B, TRACK_A, MEDIA_A, (2, 1), (2, 1));

        ripple_trim_end(&mut session, CLIP_A, (3, 1), true).unwrap();

        let spans = clip_spans(&session, TRACK_A);
        assert_eq!(spans[0].2, RationalTime::new(3, 1).unwrap());
        assert_eq!(
            spans[1].1,
            RationalTime::new(3, 1).unwrap(),
            "the later clip moves right with the extended edge"
        );
    }

    #[test]
    fn plain_trim_leaves_later_clips_where_they_are() {
        let mut session = session_with_state("plain-trim", 0);
        seed_media_and_track(&mut session, MEDIA_A, TRACK_A);
        insert_clip_at(&mut session, CLIP_A, TRACK_A, MEDIA_A, (0, 1), (4, 1));
        insert_clip_at(&mut session, CLIP_B, TRACK_A, MEDIA_A, (4, 1), (2, 1));

        // No ripple argument at all: the default must keep the old meaning, which
        // is the regression that would silently move every user's later clips.
        execute(
            &mut session,
            "timeline.clip.trim",
            json!({
                "clip_id": CLIP_A,
                "edge": "end",
                "timeline_time": rational_json(2, 1),
            }),
        )
        .unwrap();

        let spans = clip_spans(&session, TRACK_A);
        assert_eq!(spans[0].2, RationalTime::new(2, 1).unwrap());
        assert_eq!(
            spans[1].1,
            RationalTime::new(4, 1).unwrap(),
            "a plain trim leaves the gap"
        );
    }

    #[test]
    fn a_ripple_trim_to_the_current_edge_is_a_no_op() {
        let mut session = session_with_state("ripple-noop", 0);
        seed_media_and_track(&mut session, MEDIA_A, TRACK_A);
        insert_clip_at(&mut session, CLIP_A, TRACK_A, MEDIA_A, (0, 1), (4, 1));
        insert_clip_at(&mut session, CLIP_B, TRACK_A, MEDIA_A, (4, 1), (2, 1));
        let revision_before = session.project_revision();

        // A ripple is still a trim: an edge that does not move changes nothing and,
        // in particular, must not shift the later clips.
        let result = ripple_trim_end(&mut session, CLIP_A, (4, 1), true).unwrap();
        assert!(!result.changed);
        assert_eq!(session.project_revision().value(), revision_before.value());
        let spans = clip_spans(&session, TRACK_A);
        assert_eq!(spans[0].2, RationalTime::new(4, 1).unwrap());
        assert_eq!(spans[1].1, RationalTime::new(4, 1).unwrap());
    }

    #[test]
    fn a_ripple_trim_outside_the_clip_is_refused_and_changes_nothing() {
        let mut session = session_with_state("ripple-refused", 0);
        seed_media_and_track(&mut session, MEDIA_A, TRACK_A);
        insert_clip_at(&mut session, CLIP_A, TRACK_A, MEDIA_A, (0, 1), (2, 1));
        insert_clip_at(&mut session, CLIP_B, TRACK_A, MEDIA_A, (2, 1), (2, 1));
        let revision_before = session.project_revision();

        // An end trim to or before the clip's own start is invalid for any end
        // trim, ripple or not, and the track must be untouched afterwards. (An end
        // beyond the current end is a lengthening and is legal.)
        let error = ripple_trim_end(&mut session, CLIP_A, (0, 1), true).unwrap_err();
        assert_eq!(error.code, super::OperationErrorCode::InvalidArguments);
        assert_eq!(session.project_revision().value(), revision_before.value());
        let spans = clip_spans(&session, TRACK_A);
        assert_eq!(spans[0].2, RationalTime::new(2, 1).unwrap());
        assert_eq!(spans[1].1, RationalTime::new(2, 1).unwrap());
    }

    #[test]
    fn ripple_on_a_start_edge_is_refused_with_the_plain_trim_meaning() {
        let mut session = session_with_state("ripple-start", 0);
        seed_media_and_track(&mut session, MEDIA_A, TRACK_A);
        insert_clip_at(&mut session, CLIP_A, TRACK_A, MEDIA_A, (0, 1), (4, 1));

        let error = execute(
            &mut session,
            "timeline.clip.trim",
            json!({
                "clip_id": CLIP_A,
                "edge": "start",
                "timeline_time": rational_json(1, 1),
                "ripple": true,
            }),
        )
        .unwrap_err();

        assert_eq!(
            error.code,
            super::OperationErrorCode::InvalidArguments,
            "ripple-in has two meanings; refusing is the honest default"
        );
        let spans = clip_spans(&session, TRACK_A);
        assert_eq!(spans[0].1, RationalTime::new(0, 1).unwrap());
        assert_eq!(spans[0].2, RationalTime::new(4, 1).unwrap());
    }

    fn slip_clip(
        session: &mut ProjectSession,
        clip_id: &str,
        delta: (i64, u32),
    ) -> Result<super::CommandResult, super::OperationError> {
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
    fn source_start(session: &ProjectSession, track_id: &str, clip_id: &str) -> RationalTime {
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

    #[test]
    fn slip_moves_the_source_window_and_leaves_the_cut_where_it_was() {
        let mut session = session_with_state("slip", 0);
        seed_media_and_track(&mut session, MEDIA_A, TRACK_A);
        insert_clip_at(&mut session, CLIP_A, TRACK_A, MEDIA_A, (1, 1), (1, 1));
        insert_clip_at(&mut session, CLIP_B, TRACK_A, MEDIA_A, (2, 1), (1, 1));
        let revision_before = session.project_revision();
        let spans_before = clip_spans(&session, TRACK_A);

        slip_clip(&mut session, CLIP_A, (1, 4)).unwrap();

        let spans_after = clip_spans(&session, TRACK_A);
        assert_eq!(
            spans_before, spans_after,
            "a slip never moves a clip or its neighbours"
        );
        assert_eq!(
            source_start(&session, TRACK_A, CLIP_A),
            RationalTime::new(1, 4).unwrap(),
            "the source window moved by the delta"
        );
        assert_eq!(
            session.project_revision().value(),
            revision_before.value() + 1,
            "a slip is one revision"
        );

        execute(&mut session, "history.undo", json!({})).unwrap();
        assert_eq!(
            source_start(&session, TRACK_A, CLIP_A),
            RationalTime::new(0, 1).unwrap(),
            "undo restores the original source window"
        );
        assert_eq!(spans_before, clip_spans(&session, TRACK_A));
    }

    #[test]
    fn slip_backwards_takes_earlier_source_material() {
        let mut session = session_with_state("slip-back", 0);
        seed_media_and_track(&mut session, MEDIA_A, TRACK_A);
        insert_clip_at(&mut session, CLIP_A, TRACK_A, MEDIA_A, (0, 1), (1, 1));
        // Start an eighth of a second into the source so there is room to go back.
        slip_clip(&mut session, CLIP_A, (1, 8)).unwrap();
        assert_eq!(
            source_start(&session, TRACK_A, CLIP_A),
            RationalTime::new(1, 8).unwrap()
        );

        slip_clip(&mut session, CLIP_A, (-1, 8)).unwrap();
        assert_eq!(
            source_start(&session, TRACK_A, CLIP_A),
            RationalTime::new(0, 1).unwrap(),
            "a negative delta takes earlier material"
        );
    }

    #[test]
    fn a_slip_past_the_start_of_the_source_is_refused() {
        let mut session = session_with_state("slip-refused", 0);
        seed_media_and_track(&mut session, MEDIA_A, TRACK_A);
        insert_clip_at(&mut session, CLIP_A, TRACK_A, MEDIA_A, (0, 1), (1, 1));
        let revision_before = session.project_revision();

        // Nothing exists before the start of the media, so the clip must not be
        // left pointing at it.
        let error = slip_clip(&mut session, CLIP_A, (-1, 1)).unwrap_err();
        assert_eq!(error.code, super::OperationErrorCode::InvalidArguments);
        assert_eq!(session.project_revision().value(), revision_before.value());
        assert_eq!(
            source_start(&session, TRACK_A, CLIP_A),
            RationalTime::new(0, 1).unwrap()
        );
    }

    #[test]
    fn a_slip_of_zero_is_a_no_op() {
        let mut session = session_with_state("slip-zero", 0);
        seed_media_and_track(&mut session, MEDIA_A, TRACK_A);
        insert_clip_at(&mut session, CLIP_A, TRACK_A, MEDIA_A, (0, 1), (1, 1));
        let revision_before = session.project_revision();

        let result = slip_clip(&mut session, CLIP_A, (0, 1)).unwrap();
        assert!(!result.changed);
        assert_eq!(session.project_revision().value(), revision_before.value());
    }

    fn roll_boundary(
        session: &mut ProjectSession,
        clip_id: &str,
        delta: (i64, u32),
    ) -> Result<super::CommandResult, super::OperationError> {
        execute(
            session,
            "timeline.clip.roll",
            json!({
                "clip_id": clip_id,
                "delta": rational_json(delta.0, delta.1),
            }),
        )
    }

    #[test]
    fn roll_moves_the_boundary_and_both_source_windows() {
        let mut session = session_with_state("roll", 0);
        seed_media_and_track(&mut session, MEDIA_A, TRACK_A);
        insert_clip_at(&mut session, CLIP_A, TRACK_A, MEDIA_A, (0, 1), (1, 1));
        insert_clip_at(&mut session, CLIP_B, TRACK_A, MEDIA_A, (1, 1), (1, 1));
        let revision_before = session.project_revision();

        roll_boundary(&mut session, CLIP_A, (1, 4)).unwrap();

        let spans = clip_spans(&session, TRACK_A);
        assert_eq!(
            spans[0].2,
            RationalTime::new(5, 4).unwrap(),
            "the left clip gives up a quarter second"
        );
        assert_eq!(
            spans[1].1,
            RationalTime::new(5, 4).unwrap(),
            "the right clip starts where the boundary moved to"
        );
        assert_eq!(spans[1].2, RationalTime::new(3, 4).unwrap());
        assert_eq!(
            source_start(&session, TRACK_A, CLIP_A),
            RationalTime::new(0, 1).unwrap(),
            "the left clip's source does not move"
        );
        assert_eq!(
            source_start(&session, TRACK_A, CLIP_B),
            RationalTime::new(1, 4).unwrap(),
            "the right clip's source advances with its start"
        );
        assert_eq!(
            session.project_revision().value(),
            revision_before.value() + 1,
            "a roll is one revision"
        );

        execute(&mut session, "history.undo", json!({})).unwrap();
        let restored = clip_spans(&session, TRACK_A);
        assert_eq!(restored[0].2, RationalTime::new(1, 1).unwrap());
        assert_eq!(restored[1].1, RationalTime::new(1, 1).unwrap());
        assert_eq!(
            source_start(&session, TRACK_A, CLIP_B),
            RationalTime::new(0, 1).unwrap(),
            "one undo restores both sides"
        );
    }

    #[test]
    fn roll_needs_two_clips_that_actually_touch() {
        let mut session = session_with_state("roll-gap", 0);
        seed_media_and_track(&mut session, MEDIA_A, TRACK_A);
        insert_clip_at(&mut session, CLIP_A, TRACK_A, MEDIA_A, (0, 1), (1, 1));
        // A one-second gap: there is no shared boundary to roll.
        insert_clip_at(&mut session, CLIP_B, TRACK_A, MEDIA_A, (2, 1), (1, 1));

        let error = roll_boundary(&mut session, CLIP_A, (1, 4)).unwrap_err();
        assert_eq!(error.code, super::OperationErrorCode::InvalidArguments);
        let spans = clip_spans(&session, TRACK_A);
        assert_eq!(spans[0].2, RationalTime::new(1, 1).unwrap());
        assert_eq!(spans[1].1, RationalTime::new(2, 1).unwrap());
    }

    #[test]
    fn roll_past_either_clip_is_refused() {
        let mut session = session_with_state("roll-bounds", 0);
        seed_media_and_track(&mut session, MEDIA_A, TRACK_A);
        insert_clip_at(&mut session, CLIP_A, TRACK_A, MEDIA_A, (0, 1), (1, 1));
        insert_clip_at(&mut session, CLIP_B, TRACK_A, MEDIA_A, (1, 1), (1, 1));

        // Consuming the whole right clip leaves it no duration.
        let error = roll_boundary(&mut session, CLIP_A, (1, 1)).unwrap_err();
        assert_eq!(error.code, super::OperationErrorCode::InvalidArguments);
        // And the same on the other side.
        let error = roll_boundary(&mut session, CLIP_A, (-1, 1)).unwrap_err();
        assert_eq!(error.code, super::OperationErrorCode::InvalidArguments);
        let spans = clip_spans(&session, TRACK_A);
        assert_eq!(spans[0].2, RationalTime::new(1, 1).unwrap());
        assert_eq!(spans[1].2, RationalTime::new(1, 1).unwrap());
    }

    #[test]
    fn a_roll_of_zero_is_a_no_op() {
        let mut session = session_with_state("roll-zero", 0);
        seed_media_and_track(&mut session, MEDIA_A, TRACK_A);
        insert_clip_at(&mut session, CLIP_A, TRACK_A, MEDIA_A, (0, 1), (1, 1));
        insert_clip_at(&mut session, CLIP_B, TRACK_A, MEDIA_A, (1, 1), (1, 1));
        let revision_before = session.project_revision();

        let result = roll_boundary(&mut session, CLIP_A, (0, 1)).unwrap();
        assert!(!result.changed);
        assert_eq!(session.project_revision().value(), revision_before.value());
    }

    fn slide_clip(
        session: &mut ProjectSession,
        clip_id: &str,
        delta: (i64, u32),
    ) -> Result<super::CommandResult, super::OperationError> {
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
    fn seed_three_adjacent_clips(session: &mut ProjectSession) {
        seed_media_and_track(session, MEDIA_A, TRACK_A);
        insert_clip_at(session, CLIP_A, TRACK_A, MEDIA_A, (0, 1), (1, 1));
        insert_clip_at(session, CLIP_B, TRACK_A, MEDIA_A, (1, 1), (1, 1));
        insert_clip_at(session, CLIP_C, TRACK_A, MEDIA_A, (2, 1), (1, 1));
    }

    #[test]
    fn slide_moves_the_clip_and_lets_both_neighbours_absorb_it() {
        let mut session = session_with_state("slide", 0);
        seed_three_adjacent_clips(&mut session);
        let revision_before = session.project_revision();

        slide_clip(&mut session, CLIP_B, (1, 4)).unwrap();

        let spans = clip_spans(&session, TRACK_A);
        assert_eq!(
            spans[0].2,
            RationalTime::new(5, 4).unwrap(),
            "the previous clip grows"
        );
        assert_eq!(
            spans[1].1,
            RationalTime::new(5, 4).unwrap(),
            "the clip moves"
        );
        assert_eq!(
            spans[1].2,
            RationalTime::new(1, 1).unwrap(),
            "and keeps its duration"
        );
        assert_eq!(
            spans[2].1,
            RationalTime::new(9, 4).unwrap(),
            "the next clip starts later"
        );
        assert_eq!(
            spans[2].2,
            RationalTime::new(3, 4).unwrap(),
            "and gives up the same amount"
        );
        assert_eq!(
            source_start(&session, TRACK_A, CLIP_B),
            RationalTime::new(0, 1).unwrap(),
            "the moving clip's source never changes"
        );
        assert_eq!(
            source_start(&session, TRACK_A, CLIP_C),
            RationalTime::new(1, 4).unwrap(),
            "the next clip's source advances with its start"
        );
        assert_eq!(
            session.project_revision().value(),
            revision_before.value() + 1
        );

        execute(&mut session, "history.undo", json!({})).unwrap();
        let restored = clip_spans(&session, TRACK_A);
        assert_eq!(restored[0].2, RationalTime::new(1, 1).unwrap());
        assert_eq!(restored[1].1, RationalTime::new(1, 1).unwrap());
        assert_eq!(restored[2].1, RationalTime::new(2, 1).unwrap());
        assert_eq!(
            source_start(&session, TRACK_A, CLIP_C),
            RationalTime::new(0, 1).unwrap(),
            "one undo restores all three clips"
        );
    }

    #[test]
    fn slide_backwards_is_refused_when_the_next_clip_has_no_earlier_material() {
        let mut session = session_with_state("slide-back-refused", 0);
        seed_three_adjacent_clips(&mut session);

        // The next clip starts at the beginning of its own source: it has nothing
        // earlier to give, so pulling the clip back would reference material that
        // does not exist. That is a refusal, not a trim.
        let error = slide_clip(&mut session, CLIP_B, (-1, 4)).unwrap_err();
        assert_eq!(error.code, super::OperationErrorCode::InvalidArguments);
        let spans = clip_spans(&session, TRACK_A);
        assert_eq!(spans[1].1, RationalTime::new(1, 1).unwrap());
        assert_eq!(spans[2].1, RationalTime::new(2, 1).unwrap());
    }

    #[test]
    fn slide_backwards_pulls_the_clip_earlier_when_the_material_allows_it() {
        let mut session = session_with_state("slide-back", 0);
        seed_media_and_track(&mut session, MEDIA_A, TRACK_A);
        insert_clip_at(&mut session, CLIP_A, TRACK_A, MEDIA_A, (0, 1), (1, 1));
        insert_clip_at(&mut session, CLIP_B, TRACK_A, MEDIA_A, (1, 1), (1, 1));
        // This clip starts a quarter second into its source, so it can give that
        // quarter second back when the slide pulls it earlier.
        insert_clip(
            &mut session,
            CLIP_C,
            TRACK_A,
            MEDIA_A,
            (2, 1),
            (1, 4),
            (1, 1),
        )
        .unwrap();

        slide_clip(&mut session, CLIP_B, (-1, 4)).unwrap();

        let spans = clip_spans(&session, TRACK_A);
        assert_eq!(spans[0].2, RationalTime::new(3, 4).unwrap());
        assert_eq!(spans[1].1, RationalTime::new(3, 4).unwrap());
        assert_eq!(
            spans[1].2,
            RationalTime::new(1, 1).unwrap(),
            "the moving clip keeps its duration"
        );
        assert_eq!(spans[2].1, RationalTime::new(7, 4).unwrap());
        assert_eq!(spans[2].2, RationalTime::new(5, 4).unwrap());
        assert_eq!(
            source_start(&session, TRACK_A, CLIP_C),
            RationalTime::new(0, 1).unwrap(),
            "the next clip's source moves back with its start"
        );
    }

    #[test]
    fn slide_needs_touching_neighbours_on_both_sides() {
        let mut session = session_with_state("slide-edges", 0);
        seed_three_adjacent_clips(&mut session);
        // The first and last clip of a track have no neighbour to borrow from.
        let error = slide_clip(&mut session, CLIP_A, (1, 4)).unwrap_err();
        assert_eq!(error.code, super::OperationErrorCode::InvalidArguments);
        let error = slide_clip(&mut session, CLIP_C, (1, 4)).unwrap_err();
        assert_eq!(error.code, super::OperationErrorCode::InvalidArguments);

        // And a gap means there is nothing to borrow either.
        let mut gapped = session_with_state("slide-gap", 0);
        seed_media_and_track(&mut gapped, MEDIA_A, TRACK_A);
        insert_clip_at(&mut gapped, CLIP_A, TRACK_A, MEDIA_A, (0, 1), (1, 1));
        insert_clip_at(&mut gapped, CLIP_B, TRACK_A, MEDIA_A, (2, 1), (1, 1));
        insert_clip_at(&mut gapped, CLIP_C, TRACK_A, MEDIA_A, (3, 1), (1, 1));
        let error = slide_clip(&mut gapped, CLIP_B, (1, 4)).unwrap_err();
        assert_eq!(error.code, super::OperationErrorCode::InvalidArguments);
        let spans = clip_spans(&gapped, TRACK_A);
        assert_eq!(spans[1].1, RationalTime::new(2, 1).unwrap());
    }

    #[test]
    fn slide_that_would_consume_a_neighbour_is_refused() {
        let mut session = session_with_state("slide-bounds", 0);
        seed_three_adjacent_clips(&mut session);

        let error = slide_clip(&mut session, CLIP_B, (2, 1)).unwrap_err();
        assert_eq!(error.code, super::OperationErrorCode::InvalidArguments);
        let error = slide_clip(&mut session, CLIP_B, (-2, 1)).unwrap_err();
        assert_eq!(error.code, super::OperationErrorCode::InvalidArguments);
        let spans = clip_spans(&session, TRACK_A);
        assert_eq!(spans[0].2, RationalTime::new(1, 1).unwrap());
        assert_eq!(spans[1].1, RationalTime::new(1, 1).unwrap());
    }

    #[test]
    fn a_slide_of_zero_is_a_no_op() {
        let mut session = session_with_state("slide-zero", 0);
        seed_three_adjacent_clips(&mut session);
        let revision_before = session.project_revision();

        let result = slide_clip(&mut session, CLIP_B, (0, 1)).unwrap();
        assert!(!result.changed);
        assert_eq!(session.project_revision().value(), revision_before.value());
    }

    fn move_track(
        session: &mut ProjectSession,
        track_id: &str,
        to_index: i64,
    ) -> Result<super::CommandResult, super::OperationError> {
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
    fn track_order(session: &ProjectSession) -> Vec<String> {
        session
            .project()
            .timeline()
            .tracks()
            .iter()
            .map(|track| track.id().to_string())
            .collect()
    }

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
        assert_eq!(error.code, super::OperationErrorCode::InvalidArguments);
        let unknown = "99999999-9999-4999-8999-999999999999";
        let error = move_track(&mut session, unknown, 0).unwrap_err();
        assert_eq!(error.code, super::OperationErrorCode::TimelineTrackNotFound);
        assert_eq!(track_order(&session), order_before);
        assert_eq!(session.project_revision().value(), revision_before.value());
    }

    /// A media clip without an explicit rate still plays at identity speed, so its
    /// source window and timeline duration must be exactly equal.
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

        assert_eq!(error.code, super::OperationErrorCode::InvalidArguments);
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
                    source_range: TimeRange::new(
                        RationalTime::ZERO,
                        RationalTime::new(1, 1).unwrap(),
                    )
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

    fn seed_video_track_and_clip(
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

    fn fixture_clip(
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

    fn fixture_track(track_id: &str, kind: TrackKind, clips: Vec<TimelineClip>) -> TimelineTrack {
        TimelineTrack::from_parts_for_codec(track_id.parse().unwrap(), kind, clips)
    }

    fn assert_name_change(change_set: &ChangeSet, before: &str, after: &str) {
        assert!(matches!(
            change_set.changes(),
            [ProjectChange::ProjectName { before: actual_before, after: actual_after }]
                if actual_before.as_str() == before && actual_after.as_str() == after
        ));
    }

    fn summary_query() -> QueryEnvelope {
        QueryEnvelope {
            query_id: "project.summary".to_owned(),
            schema_version: 1,
            project_id: ProjectId::from_str(PROJECT_ID).unwrap(),
            project_instance_id: ProjectInstanceId::from_str(INSTANCE_ID).unwrap(),
            arguments: json!({}),
        }
    }

    fn code<T>(result: &Result<T, super::OperationError>) -> OperationErrorCode {
        result.as_ref().err().expect("operation should fail").code
    }

    #[test]
    fn common_dispatch_reuses_command_query_and_transaction_paths() {
        let mut session = fixed_session();
        let command =
            session.handle_application_request(ApplicationRequest::Command(rename("B", 0)));
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

    fn caption_clip_state(
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
            find_timeline_track_index(&session.project, TrackId::from_str(TRACK_A).unwrap())
                .unwrap();
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
        let moved_clip =
            TimelineClipState::from(&session.project().timeline().tracks()[0].clips()[2]);
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
            code(
                &session
                    .execute_command(media_remove(MEDIA_A, session.project_revision().value(),))
            ),
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
        let expected =
            TimelineClipState::from(&session.project().timeline().tracks()[0].clips()[1]);
        let before_remove = session.project().clone();
        assert_eq!(
            code(
                &session.execute_command(media_remove(MEDIA_A, session.project_revision().value()))
            ),
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

    fn snap_fixture() -> ProjectSession {
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

    #[test]
    fn timeline_snap_resolves_zero_clip_boundaries_and_move_end_anchor_exactly() {
        let session = snap_fixture();
        let before_revision = session.project_revision();

        let zero = session
            .execute_query(snap_query(
                &session,
                super::TimelineSnapOperation::Move,
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
                super::TimelineSnapOperation::Move,
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
                super::TimelineSnapOperation::Move,
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
                super::TimelineSnapOperation::Move,
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
                super::TimelineSnapOperation::TrimStart,
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
                super::TimelineSnapOperation::TrimEnd,
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
                super::TimelineSnapOperation::Move,
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
                super::TimelineSnapOperation::TrimStart,
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
                super::TimelineSnapOperation::Move,
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
                super::TimelineSnapOperation::Move,
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
            super::TimelineSnapOperation::Move,
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
            super::TimelineSnapOperation::Move,
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
            super::TimelineSnapOperation::TrimStart,
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
                super::TimelineSnapOperation::Move,
                CLIP_A,
                Some(TRACK_C),
                (0, 1),
            ))
            .unwrap_err();
        assert_eq!(wrong_kind.code, OperationErrorCode::InvalidArguments);

        let missing_clip = session
            .execute_query(snap_query(
                &session,
                super::TimelineSnapOperation::Move,
                "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbd",
                Some(TRACK_B),
                (0, 1),
            ))
            .unwrap_err();
        assert_eq!(missing_clip.code, OperationErrorCode::TimelineClipNotFound);
        assert_eq!(session.project_revision(), before_revision);
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

    fn assert_history_conflict(session: &mut ProjectSession, command_id: &str) {
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

    fn max_revision_session_with_clip(include_clip: bool) -> ProjectSession {
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
                        TimeRange::new(RationalTime::ZERO, RationalTime::new(1, 1).unwrap())
                            .unwrap(),
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
                        TimeRange::new(RationalTime::ZERO, RationalTime::new(1, 1).unwrap())
                            .unwrap(),
                    )],
                ),
            ]));
        let before_project = session.project().clone();
        let before_undo = session.history.undo.clone();

        let incompatible =
            audio_media_item(&original.id().to_string(), "file:///media/audio-only.mkv");
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
                        TimeRange::new(RationalTime::ZERO, RationalTime::new(1, 1).unwrap())
                            .unwrap(),
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
                serde_json::from_slice::<super::QueryResult>(&encoded).unwrap(),
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
            let mut query = QueryEnvelope::media_list(
                session.project_id(),
                session.project_instance_id(),
                0,
                1,
            );
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
            serde_json::from_value::<super::QueryResult>(wire).unwrap(),
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
            history: super::SessionHistory::default(),
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
        wrong_instance.project_instance_id =
            ProjectInstanceId::from_str(OTHER_INSTANCE_ID).unwrap();
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
        wrong_instance.project_instance_id =
            ProjectInstanceId::from_str(OTHER_INSTANCE_ID).unwrap();
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
            history: super::SessionHistory::default(),
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
        assert!(
            !serde_json::to_value(summary)
                .unwrap()
                .as_object()
                .unwrap()
                .contains_key("timeline_marker_page")
        );

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
                super::TimelineSnapOperation::TrimEnd,
                CLIP_A.parse().unwrap(),
                None,
                RationalTime::new(31, 10).unwrap(),
            ))
            .unwrap();
        assert_eq!(v1.schema_version, 1);
        let v1_json = serde_json::to_value(&v1).unwrap();
        assert!(
            !v1_json["timeline_snap"]
                .as_object()
                .unwrap()
                .contains_key("target_marker_id")
        );
        assert_eq!(
            v1.timeline_snap.unwrap().target_kind,
            TimelineSnapTargetKind::None
        );

        let v2 = session
            .execute_query(QueryEnvelope::timeline_snap_v2(
                session.project_id(),
                session.project_instance_id(),
                super::TimelineSnapOperation::TrimEnd,
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
            super::TimelineSnapOperation::TrimEnd,
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
                super::TimelineSnapOperation::TrimEnd,
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
                super::TimelineSnapOperation::TrimStart,
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
                super::TimelineSnapOperation::TrimEnd,
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
                super::TimelineSnapOperation::Move,
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
