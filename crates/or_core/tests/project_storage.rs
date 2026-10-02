use or_core::{
    ApplicationRequest, ApplicationResponse, ClipId, CommandEnvelope, MAX_PROJECT_FILE_BYTES,
    MarkerId, MediaId, OperationErrorCode, ProjectDocument, ProjectFileSession,
    ProjectFileSessionErrorCode, ProjectRevision, ProjectSession, ProjectStorageError,
    RationalTime, TimeRange, TrackId, TrackKind, decode_project, encode_project, load_project_file,
    save_project_file_atomic,
};
use serde_json::json;
use std::{
    fs::{self, File},
    io,
    path::{Path, PathBuf},
    str::FromStr,
};
use uuid::Uuid;

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("or-project-storage-{}", Uuid::new_v4()));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn project_path(&self) -> PathBuf {
        self.0.join("example.orproj")
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn rename(session: &mut ProjectSession, name: &str) {
    session
        .execute_command(CommandEnvelope {
            command_id: "project.rename".to_owned(),
            schema_version: 1,
            project_id: session.project_id(),
            project_instance_id: session.project_instance_id(),
            expected_project_revision: session.project_revision(),
            arguments: json!({ "name": name }),
        })
        .unwrap();
}

fn project_with_timeline_media() -> ProjectDocument {
    let project = ProjectDocument::new("Timeline storage");
    let mut wire: serde_json::Value =
        serde_json::from_str(&encode_project(&project).unwrap()).unwrap();
    wire["project"]["media"] = serde_json::json!([{
        "id": "44444444-4444-4444-8444-444444444444",
        "source": {"kind": "local_file", "uri": "file:///C:/missing/timeline-fixture.mov"},
        "metadata": {
            "format_names": ["matroska"],
            "duration": null,
            "file_size_bytes": 42,
            "streams": [{
                "kind": "video",
                "metadata": {
                    "index": 0,
                    "codec_name": null,
                    "width": 1920,
                    "height": 1080,
                    "pixel_format": null,
                    "average_frame_rate": {"numerator": 24, "denominator": 1},
                    "duration": null
                }
            }]
        }
    }]);
    decode_project(&wire.to_string()).unwrap()
}

fn timeline_command(
    session: &mut ProjectFileSession,
    command_id: &str,
    arguments: serde_json::Value,
) {
    let project = session.session();
    let request = ApplicationRequest::Command(CommandEnvelope {
        command_id: command_id.to_owned(),
        schema_version: 1,
        project_id: project.project_id(),
        project_instance_id: project.project_instance_id(),
        expected_project_revision: project.project_revision(),
        arguments,
    });
    match session.handle_application_request(request) {
        ApplicationResponse::Command(result) => assert!(result.changed),
        ApplicationResponse::Error(error) => panic!("{command_id} failed: {error}"),
        response => panic!("{command_id} returned unexpected response: {response:?}"),
    }
}

fn rational(numerator: i64, denominator: u32) -> serde_json::Value {
    serde_json::json!({"numerator": numerator, "denominator": denominator})
}

fn names(directory: &Path) -> Vec<String> {
    fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect()
}

#[test]
fn saves_and_loads_canonical_state_without_runtime_session_history() {
    let directory = TestDirectory::new();
    let path = directory.project_path();
    let mut session = ProjectSession::open(ProjectDocument::new("Initial"));
    rename(&mut session, "Renamed");
    let original = session.project().clone();
    let instance_id = session.project_instance_id().to_string();

    save_project_file_atomic(&path, session.project()).unwrap();

    assert_eq!(session.project(), &original);
    let bytes = fs::read_to_string(&path).unwrap();
    assert!(!bytes.contains(&instance_id));
    for runtime_state in ["instance_id", "history", "undo", "redo", "change_set"] {
        assert!(!bytes.contains(runtime_state));
    }

    let loaded = load_project_file(&path).unwrap();
    assert_eq!(loaded.id(), original.id());
    assert_eq!(loaded.revision(), ProjectRevision::new(1));
    assert_eq!(loaded.name(), "Renamed");

    let mut reopened = ProjectSession::open(loaded);
    let undo = reopened.execute_command(CommandEnvelope {
        command_id: "history.undo".to_owned(),
        schema_version: 1,
        project_id: reopened.project_id(),
        project_instance_id: reopened.project_instance_id(),
        expected_project_revision: reopened.project_revision(),
        arguments: json!({}),
    });
    assert_eq!(undo.unwrap_err().code, OperationErrorCode::NothingToUndo);
}

#[test]
fn application_timeline_commands_save_reopen_exactly_and_do_not_persist_history() {
    let directory = TestDirectory::new();
    let path = directory.project_path();
    let seeded = project_with_timeline_media();
    let project_id = seeded.id();
    save_project_file_atomic(&path, &seeded).unwrap();
    let mut session = ProjectFileSession::open(&path).unwrap();
    let track_a = TrackId::from_str("22222222-2222-4222-8222-222222222222").unwrap();
    let track_b = TrackId::from_str("88888888-8888-4888-8888-888888888888").unwrap();
    let track_c = TrackId::from_str("99999999-9999-4999-8999-999999999999").unwrap();
    let clip_a = ClipId::from_str("33333333-3333-4333-8333-333333333333").unwrap();
    let clip_b = ClipId::from_str("55555555-5555-4555-8555-555555555555").unwrap();
    let media_id = MediaId::from_str("44444444-4444-4444-8444-444444444444").unwrap();
    for (track_id, kind) in [
        (track_a, TrackKind::Video),
        (track_b, TrackKind::Audio),
        (track_c, TrackKind::Video),
    ] {
        timeline_command(
            &mut session,
            "timeline.track.add",
            serde_json::json!({"track_id": track_id, "kind": kind}),
        );
    }
    for (clip_id, at, source_start) in [(clip_b, 4, 3), (clip_a, 0, 0)] {
        timeline_command(
            &mut session,
            "timeline.clip.insert",
            serde_json::json!({
                "clip_id": clip_id,
                "track_id": track_a,
                "media_id": media_id,
                "timeline_start": rational(at, 1),
                "source_range": {
                    "start": rational(source_start, 2),
                    "duration": rational(2, 1),
                },
            }),
        );
    }
    timeline_command(
        &mut session,
        "timeline.clip.move",
        serde_json::json!({
            "clip_id": clip_a,
            "track_id": track_c,
            "timeline_start": rational(8, 1),
        }),
    );
    assert_eq!(
        session.session().project_revision(),
        ProjectRevision::new(6)
    );
    assert!(session.is_dirty());
    session.save().unwrap();

    let saved = load_project_file(&path).unwrap();
    assert_eq!(saved.id(), project_id);
    assert_eq!(saved.revision(), ProjectRevision::new(6));
    let tracks = saved.timeline().tracks();
    assert_eq!(
        tracks.iter().map(|track| track.id()).collect::<Vec<_>>(),
        [track_a, track_b, track_c]
    );
    assert_eq!(tracks[0].kind(), TrackKind::Video);
    assert_eq!(tracks[1].kind(), TrackKind::Audio);
    assert_eq!(tracks[2].kind(), TrackKind::Video);
    assert_eq!(tracks[0].clips().len(), 1);
    assert_eq!(tracks[0].clips()[0].id(), clip_b);
    assert_eq!(tracks[0].clips()[0].media_id(), Some(media_id));
    assert_eq!(
        tracks[0].clips()[0].timeline_start(),
        RationalTime::new(4, 1).unwrap()
    );
    assert_eq!(
        tracks[0].clips()[0].source_range(),
        Some(
            TimeRange::new(
                RationalTime::new(3, 2).unwrap(),
                RationalTime::new(2, 1).unwrap()
            )
            .unwrap()
        )
    );
    assert_eq!(tracks[2].clips().len(), 1);
    assert_eq!(tracks[2].clips()[0].id(), clip_a);
    assert_eq!(tracks[2].clips()[0].media_id(), Some(media_id));
    assert_eq!(
        tracks[2].clips()[0].timeline_start(),
        RationalTime::new(8, 1).unwrap()
    );
    assert_eq!(
        tracks[2].clips()[0].source_range(),
        Some(TimeRange::new(RationalTime::ZERO, RationalTime::new(2, 1).unwrap()).unwrap())
    );

    let reopened = ProjectFileSession::open(&path).unwrap();
    assert_eq!(reopened.session().project_id(), project_id);
    assert_eq!(
        reopened.session().project_revision(),
        ProjectRevision::new(6)
    );
    assert_eq!(reopened.session().project().timeline(), saved.timeline());
    let mut reopened = reopened;
    let no_undo =
        reopened.handle_application_request(ApplicationRequest::Command(CommandEnvelope {
            command_id: "history.undo".to_owned(),
            schema_version: 1,
            project_id,
            project_instance_id: reopened.session().project_instance_id(),
            expected_project_revision: ProjectRevision::new(6),
            arguments: serde_json::json!({}),
        }));
    assert!(
        matches!(no_undo, ApplicationResponse::Error(error) if error.code == OperationErrorCode::NothingToUndo)
    );

    let mut undo_session = ProjectFileSession::open(&path).unwrap();
    timeline_command(
        &mut undo_session,
        "timeline.clip.move",
        serde_json::json!({
            "clip_id": clip_a,
            "track_id": track_a,
            "timeline_start": rational(8, 1),
        }),
    );
    let undone =
        undo_session.handle_application_request(ApplicationRequest::Command(CommandEnvelope {
            command_id: "history.undo".to_owned(),
            schema_version: 1,
            project_id,
            project_instance_id: undo_session.session().project_instance_id(),
            expected_project_revision: ProjectRevision::new(7),
            arguments: serde_json::json!({}),
        }));
    assert!(
        matches!(undone, ApplicationResponse::Command(result) if result.after_revision == ProjectRevision::new(8))
    );
    undo_session.save().unwrap();
    let after_undo_save = load_project_file(&path).unwrap();
    assert_eq!(after_undo_save.revision(), ProjectRevision::new(8));
    assert_eq!(
        after_undo_save.timeline().tracks()[2].clips()[0].id(),
        clip_a
    );
    assert_eq!(
        after_undo_save.timeline().tracks()[2].clips()[0].timeline_start(),
        RationalTime::new(8, 1).unwrap()
    );
    let mut after_undo_reopen = ProjectFileSession::open(&path).unwrap();
    let no_redo = after_undo_reopen.handle_application_request(ApplicationRequest::Command(
        CommandEnvelope {
            command_id: "history.redo".to_owned(),
            schema_version: 1,
            project_id,
            project_instance_id: after_undo_reopen.session().project_instance_id(),
            expected_project_revision: ProjectRevision::new(8),
            arguments: serde_json::json!({}),
        },
    ));
    assert!(
        matches!(no_redo, ApplicationResponse::Error(error) if error.code == OperationErrorCode::NothingToRedo)
    );
}

#[test]
fn advanced_timeline_commands_save_reopen_exactly_as_project_schema_v7() {
    let directory = TestDirectory::new();
    let path = directory.project_path();
    let seeded = project_with_timeline_media();
    let project_id = seeded.id();
    save_project_file_atomic(&path, &seeded).unwrap();
    let mut session = ProjectFileSession::open(&path).unwrap();
    let track_id = TrackId::from_str("22222222-2222-4222-8222-222222222222").unwrap();
    let clip_a = ClipId::from_str("33333333-3333-4333-8333-333333333333").unwrap();
    let clip_b = ClipId::from_str("55555555-5555-4555-8555-555555555555").unwrap();
    let media_id = MediaId::from_str("44444444-4444-4444-8444-444444444444").unwrap();

    timeline_command(
        &mut session,
        "timeline.track.add",
        serde_json::json!({"track_id": track_id, "kind": TrackKind::Video}),
    );
    timeline_command(
        &mut session,
        "timeline.clip.insert",
        serde_json::json!({
            "clip_id": clip_a,
            "track_id": track_id,
            "media_id": media_id,
            "timeline_start": rational(2, 1),
            "source_range": {"start": rational(1, 1), "duration": rational(4, 1)},
        }),
    );
    timeline_command(
        &mut session,
        "timeline.clip.insert",
        serde_json::json!({
            "clip_id": clip_b,
            "track_id": track_id,
            "media_id": media_id,
            "timeline_start": rational(8, 1),
            "source_range": {"start": rational(4, 1), "duration": rational(1, 1)},
        }),
    );
    timeline_command(
        &mut session,
        "timeline.clip.trim",
        serde_json::json!({
            "clip_id": clip_a,
            "edge": "start",
            "timeline_time": rational(3, 2),
        }),
    );
    timeline_command(
        &mut session,
        "timeline.clip.split",
        serde_json::json!({
            "clip_id": clip_a,
            "new_clip_id": ClipId::from_str("66666666-6666-4666-8666-666666666666").unwrap(),
            "timeline_time": rational(4, 1),
        }),
    );
    let split_right = ClipId::from_str("66666666-6666-4666-8666-666666666666").unwrap();
    timeline_command(
        &mut session,
        "timeline.clip.ripple_delete",
        serde_json::json!({"clip_id": split_right}),
    );
    let marker_id = MarkerId::from_str("11111111-1111-4111-8111-111111111111").unwrap();
    timeline_command(
        &mut session,
        "timeline.marker.add",
        serde_json::json!({
            "marker_id": marker_id,
            "timeline_time": rational(12, 1),
            "label": "persisted marker"
        }),
    );
    timeline_command(
        &mut session,
        "timeline.sequence.set_frame_rate",
        serde_json::json!({
            "sequence_frame_rate": {"numerator": 30_000, "denominator": 1_001}
        }),
    );
    assert_eq!(
        session.session().project_revision(),
        ProjectRevision::new(8)
    );

    session.save().unwrap();
    let encoded = serde_json::from_slice::<serde_json::Value>(&fs::read(&path).unwrap()).unwrap();
    assert_eq!(encoded["schema_version"], 7);
    for runtime_state in ["instance_id", "history", "undo", "redo", "change_set"] {
        assert!(!encoded.to_string().contains(runtime_state));
    }
    let saved = load_project_file(&path).unwrap();
    assert_eq!(saved.id(), project_id);
    assert_eq!(saved.revision(), ProjectRevision::new(8));
    assert_eq!(
        saved.timeline().sequence_frame_rate(),
        Some(or_core::RationalRate::new(30_000, 1_001).unwrap())
    );
    assert_eq!(saved.timeline().markers()[0].id(), marker_id);
    assert_eq!(saved.timeline().markers()[0].label(), "persisted marker");
    let clips = saved.timeline().tracks()[0].clips();
    assert_eq!(clips.len(), 2);
    assert_eq!(clips[0].id(), clip_a);
    assert_eq!(clips[0].timeline_start(), RationalTime::new(3, 2).unwrap());
    assert_eq!(
        clips[0].source_range(),
        Some(
            TimeRange::new(
                RationalTime::new(1, 2).unwrap(),
                RationalTime::new(5, 2).unwrap()
            )
            .unwrap()
        )
    );
    assert_eq!(clips[1].id(), clip_b);
    assert_eq!(clips[1].timeline_start(), RationalTime::new(6, 1).unwrap());

    let reopened = ProjectFileSession::open(&path).unwrap();
    assert_eq!(reopened.session().project_id(), project_id);
    assert_eq!(
        reopened.session().project_revision(),
        ProjectRevision::new(8)
    );
    assert_eq!(reopened.session().project().timeline(), saved.timeline());
    let mut reopened = reopened;
    let no_undo =
        reopened.handle_application_request(ApplicationRequest::Command(CommandEnvelope {
            command_id: "history.undo".to_owned(),
            schema_version: 1,
            project_id,
            project_instance_id: reopened.session().project_instance_id(),
            expected_project_revision: ProjectRevision::new(8),
            arguments: serde_json::json!({}),
        }));
    assert!(
        matches!(no_undo, ApplicationResponse::Error(error) if error.code == OperationErrorCode::NothingToUndo)
    );
}

#[test]
fn atomically_replaces_an_existing_project_file() {
    let directory = TestDirectory::new();
    let path = directory.project_path();
    fs::write(&path, b"previous file contents").unwrap();
    let document = ProjectDocument::new("Replacement");

    save_project_file_atomic(&path, &document).unwrap();

    assert_eq!(load_project_file(path).unwrap(), document);
    assert_eq!(names(&directory.0), vec!["example.orproj".to_owned()]);
}

#[test]
fn saves_to_and_loads_from_unicode_paths() {
    let directory = TestDirectory::new();
    let unicode_directory = directory.0.join("Tiếng Việt 🎬");
    fs::create_dir(&unicode_directory).unwrap();
    let path = unicode_directory.join("dự án.orproj");
    let document = ProjectDocument::new("Ví dụ");

    save_project_file_atomic(&path, &document).unwrap();

    assert_eq!(load_project_file(path).unwrap(), document);
}

#[test]
fn rejects_invalid_utf8_and_invalid_project_codec_data() {
    let directory = TestDirectory::new();
    let invalid_utf8 = directory.0.join("invalid-utf8.orproj");
    fs::write(&invalid_utf8, [0xff, 0xfe]).unwrap();
    assert!(matches!(
        load_project_file(invalid_utf8),
        Err(ProjectStorageError::InvalidUtf8)
    ));

    let invalid_project = directory.0.join("invalid-project.orproj");
    fs::write(&invalid_project, b"not a project").unwrap();
    assert!(matches!(
        load_project_file(invalid_project),
        Err(ProjectStorageError::Codec(_))
    ));
}

#[test]
fn rejects_files_over_the_limit_before_decoding_them() {
    let directory = TestDirectory::new();
    let path = directory.project_path();
    File::create(&path)
        .unwrap()
        .set_len(MAX_PROJECT_FILE_BYTES + 1)
        .unwrap();

    assert!(matches!(
        load_project_file(path),
        Err(ProjectStorageError::TooLarge { max_bytes }) if max_bytes == MAX_PROJECT_FILE_BYTES
    ));
}

#[test]
fn rejects_missing_files_and_does_not_create_missing_parent_directories() {
    let directory = TestDirectory::new();
    let missing = directory.project_path();
    assert!(matches!(
        load_project_file(&missing),
        Err(ProjectStorageError::Io(error)) if error.kind() == io::ErrorKind::NotFound
    ));

    let parent = directory.0.join("missing-parent");
    let error = save_project_file_atomic(parent.join("example.orproj"), &ProjectDocument::new("A"))
        .unwrap_err();
    assert!(matches!(
        error,
        ProjectStorageError::TemporaryFile {
            operation: or_core::TempFileOperation::Create,
            ..
        }
    ));
    assert!(!parent.exists());
}

#[test]
fn failed_replacement_preserves_destination_and_removes_the_temporary_sibling() {
    let directory = TestDirectory::new();
    let destination = directory.0.join("existing-directory");
    fs::create_dir(&destination).unwrap();
    fs::write(destination.join("marker"), b"keep").unwrap();

    assert!(matches!(
        save_project_file_atomic(&destination, &ProjectDocument::new("A")),
        Err(ProjectStorageError::Replace(_))
    ));

    assert_eq!(fs::read(destination.join("marker")).unwrap(), b"keep");
    assert_eq!(names(&directory.0), vec!["existing-directory".to_owned()]);
}

#[test]
fn rejects_oversized_encoded_state_before_creating_a_temporary_file() {
    let directory = TestDirectory::new();
    let path = directory.project_path();
    fs::write(&path, b"preserve the old project").unwrap();
    let too_long_name = "x".repeat(MAX_PROJECT_FILE_BYTES as usize);
    let document = ProjectDocument::new(too_long_name);

    assert!(matches!(
        save_project_file_atomic(&path, &document),
        Err(ProjectStorageError::TooLarge { max_bytes }) if max_bytes == MAX_PROJECT_FILE_BYTES
    ));

    assert_eq!(fs::read(&path).unwrap(), b"preserve the old project");
    assert_eq!(names(&directory.0), vec!["example.orproj".to_owned()]);
}

#[test]
fn create_new_project_round_trips_without_clobbering_existing_files() {
    let directory = TestDirectory::new();
    let path = directory.project_path();
    let mut session = ProjectFileSession::create_new(&path, "Exact project name ").unwrap();
    let project_id = session.session().project_id();
    let instance_id = session.session().project_instance_id();
    let initial = load_project_file(&path).unwrap();

    assert_eq!(initial.name(), "Exact project name ");
    assert_eq!(initial.revision(), ProjectRevision::INITIAL);
    assert_eq!(
        session.session().project_revision(),
        ProjectRevision::INITIAL
    );
    assert!(!session.is_dirty());
    assert!(
        std::str::from_utf8(&fs::read(&path).unwrap())
            .unwrap()
            .contains("\"schema_version\": 7")
    );

    let undo = session.handle_application_request(ApplicationRequest::Command(CommandEnvelope {
        command_id: "history.undo".to_owned(),
        schema_version: 1,
        project_id,
        project_instance_id: instance_id,
        expected_project_revision: ProjectRevision::INITIAL,
        arguments: json!({}),
    }));
    assert!(matches!(undo, ApplicationResponse::Error(_)));

    let original = ProjectDocument::new("Do not replace");
    save_project_file_atomic(&path, &original).unwrap();
    let original_bytes = fs::read(&path).unwrap();
    let error = ProjectFileSession::create_new(&path, "Replacement").unwrap_err();

    assert_eq!(error.code(), ProjectFileSessionErrorCode::DestinationExists);
    assert_eq!(fs::read(&path).unwrap(), original_bytes);
    assert_eq!(load_project_file(path).unwrap(), original);
}
