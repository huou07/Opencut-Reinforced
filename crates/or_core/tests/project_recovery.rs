use or_core::{
    CommandEnvelope, MAX_RECOVERY_FILE_BYTES, ProjectDocument, ProjectRecoveryError,
    ProjectRevision, ProjectSession, RecoveryApplyOutcome, RecoveryConflictReason,
    RecoveryInspection, apply_project_recovery, decode_project, discard_project_recovery,
    encode_project, inspect_project_recovery, load_project_file, save_project_file_atomic,
    write_recovery_checkpoint,
};
use serde_json::{Value, json};
use std::{
    ffi::OsString,
    fs::{self, File},
    path::{Path, PathBuf},
};
use uuid::Uuid;

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("or-project-recovery-{}", Uuid::new_v4()));
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

fn rename(project: &ProjectDocument, names: &[&str]) -> (ProjectDocument, String) {
    let mut session = ProjectSession::open(project.clone());
    let instance_id = session.project_instance_id().to_string();
    for name in names {
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
    (session.project().clone(), instance_id)
}

fn project_at(project: &ProjectDocument, revision: ProjectRevision, name: &str) -> ProjectDocument {
    let mut value: Value = serde_json::from_str(&encode_project(project).unwrap()).unwrap();
    value["project"]["revision"] = json!(revision.value());
    value["project"]["name"] = json!(name);
    decode_project(&serde_json::to_string(&value).unwrap()).unwrap()
}

fn recovery_path(project_path: &Path) -> PathBuf {
    let mut name = OsString::from(".");
    name.push(project_path.file_name().unwrap());
    name.push(".or-recovery");
    let parent = project_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    parent.join(name)
}

fn recovery_wire(base: &ProjectDocument, recovery: &ProjectDocument) -> Value {
    json!({
        "format": "opencut-reinforced-recovery",
        "schema_version": 1,
        "base_project": serde_json::from_str::<Value>(&encode_project(base).unwrap()).unwrap(),
        "recovery_project": serde_json::from_str::<Value>(&encode_project(recovery).unwrap()).unwrap(),
    })
}

fn v1_project_value(project: &ProjectDocument) -> Value {
    let mut value: Value = serde_json::from_str(&encode_project(project).unwrap()).unwrap();
    value["schema_version"] = json!(1);
    value["project"].as_object_mut().unwrap().remove("media");
    value
}

fn write_raw_recovery(project_path: &Path, value: &Value) {
    fs::write(
        recovery_path(project_path),
        serde_json::to_vec(value).unwrap(),
    )
    .unwrap();
}

fn assert_no_temp_orphans(directory: &Path) {
    for entry in fs::read_dir(directory).unwrap() {
        let name = entry.unwrap().file_name();
        assert!(
            !name.to_string_lossy().contains(".or-tmp-"),
            "temporary file was left behind: {name:?}"
        );
    }
}

#[test]
fn no_sidecar_inspects_none_and_apply_is_controlled() {
    let directory = TestDirectory::new();
    let path = directory.project_path();
    let base = ProjectDocument::new("Saved");
    save_project_file_atomic(&path, &base).unwrap();

    assert_eq!(
        inspect_project_recovery(&path).unwrap(),
        RecoveryInspection::None
    );
    assert!(matches!(
        apply_project_recovery(&path),
        Err(ProjectRecoveryError::NoCheckpoint)
    ));
    assert_eq!(load_project_file(path).unwrap(), base);
}

#[test]
fn valid_checkpoint_inspects_candidate_with_metadata_and_recovered_document() {
    let directory = TestDirectory::new();
    let path = directory.project_path();
    let base = ProjectDocument::new("A");
    let (recovery, _) = rename(&base, &["B"]);
    save_project_file_atomic(&path, &base).unwrap();

    write_recovery_checkpoint(&path, &base, &recovery).unwrap();
    let before = fs::read(recovery_path(&path)).unwrap();
    let wire: Value = serde_json::from_slice(&before).unwrap();
    let inspection = inspect_project_recovery(&path).unwrap();

    let RecoveryInspection::Candidate(candidate) = inspection else {
        panic!("expected a recovery candidate");
    };
    assert_eq!(candidate.metadata().project_id, base.id());
    assert_eq!(candidate.metadata().base_revision, ProjectRevision::INITIAL);
    assert_eq!(
        candidate.metadata().recovery_revision,
        ProjectRevision::new(1)
    );
    assert_eq!(candidate.recovery_project(), &recovery);
    assert_eq!(load_project_file(&path).unwrap(), base);
    assert_eq!(fs::read(recovery_path(&path)).unwrap(), before);
    assert_eq!(wire.as_object().unwrap().len(), 4);
    assert_eq!(wire["format"], "opencut-reinforced-recovery");
    assert_eq!(wire["schema_version"], 1);
    assert_eq!(wire["base_project"]["format"], "opencut-reinforced-project");
    assert_eq!(
        wire["recovery_project"]["format"],
        "opencut-reinforced-project"
    );
}

#[test]
fn recovery_v1_sidecar_still_reads_nested_v1_projects_and_applies_as_v2() {
    let directory = TestDirectory::new();
    let path = directory.project_path();
    let base = ProjectDocument::new("Legacy base");
    let mut recovery_value = v1_project_value(&base);
    recovery_value["project"]["revision"] = json!(base.revision().value() + 1);
    recovery_value["project"]["name"] = json!("Recovered legacy");
    let recovery = decode_project(&serde_json::to_string(&recovery_value).unwrap()).unwrap();
    fs::write(&path, serde_json::to_vec(&v1_project_value(&base)).unwrap()).unwrap();
    write_raw_recovery(
        &path,
        &json!({
            "format": "opencut-reinforced-recovery",
            "schema_version": 1,
            "base_project": v1_project_value(&base),
            "recovery_project": recovery_value,
        }),
    );

    let RecoveryInspection::Candidate(candidate) = inspect_project_recovery(&path).unwrap() else {
        panic!("expected the legacy recovery candidate to remain readable");
    };
    assert_eq!(candidate.recovery_project(), &recovery);
    assert!(matches!(
        apply_project_recovery(&path).unwrap(),
        RecoveryApplyOutcome::AppliedAndCleaned
    ));
    let applied = load_project_file(&path).unwrap();
    assert_eq!(applied.revision(), ProjectRevision::new(1));
    assert_eq!(applied.name(), "Recovered legacy");
    let encoded: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    assert_eq!(encoded["schema_version"], 2);
}

#[test]
fn checkpoint_write_preserves_canonical_file_and_input_documents() {
    let directory = TestDirectory::new();
    let path = directory.project_path();
    let base = ProjectDocument::new("A");
    let (recovery, _) = rename(&base, &["B"]);
    let original_base = base.clone();
    let original_recovery = recovery.clone();
    save_project_file_atomic(&path, &base).unwrap();

    write_recovery_checkpoint(&path, &base, &recovery).unwrap();

    assert_eq!(load_project_file(&path).unwrap(), base);
    assert_eq!(base, original_base);
    assert_eq!(recovery, original_recovery);
}

#[test]
fn repeated_checkpoint_replaces_old_snapshot_using_the_same_disk_base() {
    let directory = TestDirectory::new();
    let path = directory.project_path();
    let base = ProjectDocument::new("A");
    let (first, _) = rename(&base, &["B"]);
    let (latest, _) = rename(&first, &["C"]);
    save_project_file_atomic(&path, &base).unwrap();

    write_recovery_checkpoint(&path, &base, &first).unwrap();
    write_recovery_checkpoint(&path, &base, &latest).unwrap();

    let RecoveryInspection::Candidate(candidate) = inspect_project_recovery(&path).unwrap() else {
        panic!("expected latest checkpoint to remain a candidate");
    };
    assert_eq!(candidate.metadata().base_revision, ProjectRevision::INITIAL);
    assert_eq!(
        candidate.metadata().recovery_revision,
        ProjectRevision::new(2)
    );
    assert_eq!(candidate.recovery_project(), &latest);
    assert_eq!(load_project_file(&path).unwrap(), base);
    assert_no_temp_orphans(&directory.0);
}

#[test]
fn wrong_write_base_does_not_create_or_replace_a_checkpoint() {
    let directory = TestDirectory::new();
    let path = directory.project_path();
    let base = ProjectDocument::new("A");
    let (recovery, _) = rename(&base, &["B"]);
    save_project_file_atomic(&path, &base).unwrap();
    write_recovery_checkpoint(&path, &base, &recovery).unwrap();
    let original_checkpoint = fs::read(recovery_path(&path)).unwrap();
    let other_base = ProjectDocument::new("Other");
    let (other_recovery, _) = rename(&other_base, &["Other recovery"]);

    assert!(matches!(
        write_recovery_checkpoint(&path, &other_base, &other_recovery),
        Err(ProjectRecoveryError::DiskBaseMismatch)
    ));
    assert_eq!(load_project_file(&path).unwrap(), base);
    assert_eq!(fs::read(recovery_path(&path)).unwrap(), original_checkpoint);
}

#[test]
fn checkpoint_rejects_different_project_ids_before_filesystem_mutation() {
    let directory = TestDirectory::new();
    let path = directory.project_path();
    let base = ProjectDocument::new("A");
    let foreign = ProjectDocument::new("Foreign");
    let (recovery, _) = rename(&foreign, &["Foreign changed"]);
    save_project_file_atomic(&path, &base).unwrap();

    assert!(matches!(
        write_recovery_checkpoint(&path, &base, &recovery),
        Err(ProjectRecoveryError::ProjectIdMismatch)
    ));
    assert!(!recovery_path(&path).exists());
    assert_eq!(load_project_file(&path).unwrap(), base);
}

#[test]
fn checkpoint_rejects_equal_revisions() {
    let directory = TestDirectory::new();
    let path = directory.project_path();
    let base = ProjectDocument::new("A");
    let recovery = project_at(&base, base.revision(), "Different");
    save_project_file_atomic(&path, &base).unwrap();

    assert!(matches!(
        write_recovery_checkpoint(&path, &base, &recovery),
        Err(ProjectRecoveryError::InvalidRevisionOrdering)
    ));
    assert!(!recovery_path(&path).exists());
}

#[test]
fn checkpoint_rejects_lower_revisions() {
    let directory = TestDirectory::new();
    let path = directory.project_path();
    let (base, _) = rename(&ProjectDocument::new("A"), &["B"]);
    let recovery = project_at(&base, ProjectRevision::INITIAL, "Older");
    save_project_file_atomic(&path, &base).unwrap();

    assert!(matches!(
        write_recovery_checkpoint(&path, &base, &recovery),
        Err(ProjectRecoveryError::InvalidRevisionOrdering)
    ));
    assert!(!recovery_path(&path).exists());
}

#[test]
fn apply_candidate_atomically_saves_snapshot_without_an_increment() {
    let directory = TestDirectory::new();
    let path = directory.project_path();
    let base = ProjectDocument::new("A");
    let (recovery, _) = rename(&base, &["B"]);
    save_project_file_atomic(&path, &base).unwrap();
    write_recovery_checkpoint(&path, &base, &recovery).unwrap();

    assert!(matches!(
        apply_project_recovery(&path).unwrap(),
        RecoveryApplyOutcome::AppliedAndCleaned
    ));
    assert_eq!(load_project_file(&path).unwrap(), recovery);
    assert!(!recovery_path(&path).exists());
}

#[test]
fn apply_preserves_project_id() {
    let directory = TestDirectory::new();
    let path = directory.project_path();
    let base = ProjectDocument::new("A");
    let (recovery, _) = rename(&base, &["B"]);
    save_project_file_atomic(&path, &base).unwrap();
    write_recovery_checkpoint(&path, &base, &recovery).unwrap();

    apply_project_recovery(&path).unwrap();

    let saved = load_project_file(&path).unwrap();
    assert_eq!(saved.id(), base.id());
    assert_eq!(saved.id(), recovery.id());
}

#[test]
fn recovery_does_not_persist_runtime_session_state() {
    let directory = TestDirectory::new();
    let path = directory.project_path();
    let base = ProjectDocument::new("A");
    let (recovery, old_instance_id) = rename(&base, &["B"]);
    save_project_file_atomic(&path, &base).unwrap();
    write_recovery_checkpoint(&path, &base, &recovery).unwrap();

    let sidecar = fs::read_to_string(recovery_path(&path)).unwrap();
    assert!(!sidecar.contains(&old_instance_id));
    for runtime_field in [
        "ProjectInstanceId",
        "project_instance_id",
        "history",
        "undo",
        "redo",
    ] {
        assert!(!sidecar.contains(runtime_field));
    }
    apply_project_recovery(&path).unwrap();
    let mut reopened = ProjectSession::open(load_project_file(&path).unwrap());
    assert_ne!(reopened.project_instance_id().to_string(), old_instance_id);
    let undo = reopened.execute_command(CommandEnvelope {
        command_id: "history.undo".to_owned(),
        schema_version: 1,
        project_id: reopened.project_id(),
        project_instance_id: reopened.project_instance_id(),
        expected_project_revision: reopened.project_revision(),
        arguments: json!({}),
    });
    assert_eq!(
        undo.unwrap_err().code,
        or_core::OperationErrorCode::NothingToUndo
    );
}

#[test]
fn normal_save_makes_recovery_stale_and_explicit_discard_keeps_saved_state() {
    let directory = TestDirectory::new();
    let path = directory.project_path();
    let base = ProjectDocument::new("A");
    let (recovery, _) = rename(&base, &["B"]);
    save_project_file_atomic(&path, &base).unwrap();
    write_recovery_checkpoint(&path, &base, &recovery).unwrap();
    save_project_file_atomic(&path, &recovery).unwrap();

    assert!(matches!(
        inspect_project_recovery(&path).unwrap(),
        RecoveryInspection::Stale(_)
    ));
    assert!(matches!(
        apply_project_recovery(&path),
        Err(ProjectRecoveryError::StaleCheckpoint)
    ));
    assert!(discard_project_recovery(&path).unwrap());
    assert_eq!(load_project_file(&path).unwrap(), recovery);
}

#[test]
fn canonical_revision_newer_than_recovery_is_stale() {
    let directory = TestDirectory::new();
    let path = directory.project_path();
    let base = ProjectDocument::new("A");
    let (recovery, _) = rename(&base, &["B"]);
    let (newer, _) = rename(&recovery, &["C"]);
    save_project_file_atomic(&path, &base).unwrap();
    write_recovery_checkpoint(&path, &base, &recovery).unwrap();
    save_project_file_atomic(&path, &newer).unwrap();

    assert!(matches!(
        inspect_project_recovery(&path).unwrap(),
        RecoveryInspection::Stale(_)
    ));
    assert_eq!(load_project_file(&path).unwrap(), newer);
}

#[test]
fn same_base_revision_with_different_canonical_content_is_conflict() {
    let directory = TestDirectory::new();
    let path = directory.project_path();
    let base = ProjectDocument::new("A");
    let (recovery, _) = rename(&base, &["B"]);
    let changed = project_at(&base, base.revision(), "External edit");
    save_project_file_atomic(&path, &base).unwrap();
    write_recovery_checkpoint(&path, &base, &recovery).unwrap();
    save_project_file_atomic(&path, &changed).unwrap();

    assert!(matches!(
        inspect_project_recovery(&path).unwrap(),
        RecoveryInspection::Conflict {
            reason: RecoveryConflictReason::ChangedLineage,
            ..
        }
    ));
    assert_eq!(load_project_file(&path).unwrap(), changed);
}

#[test]
fn canonical_revision_between_base_and_recovery_is_conflict() {
    let directory = TestDirectory::new();
    let path = directory.project_path();
    let (base, _) = rename(&ProjectDocument::new("A"), &["Base revision 1"]);
    let (recovery, _) = rename(&base, &["Intermediate", "Recovery revision 3"]);
    let (external, _) = rename(&base, &["External revision 2"]);
    save_project_file_atomic(&path, &base).unwrap();
    write_recovery_checkpoint(&path, &base, &recovery).unwrap();
    save_project_file_atomic(&path, &external).unwrap();

    assert!(matches!(
        inspect_project_recovery(&path).unwrap(),
        RecoveryInspection::Conflict { .. }
    ));
    assert_eq!(load_project_file(&path).unwrap(), external);
}

#[test]
fn foreign_canonical_project_is_conflict() {
    let directory = TestDirectory::new();
    let path = directory.project_path();
    let base = ProjectDocument::new("A");
    let (recovery, _) = rename(&base, &["B"]);
    let foreign = ProjectDocument::new("Foreign");
    save_project_file_atomic(&path, &base).unwrap();
    write_recovery_checkpoint(&path, &base, &recovery).unwrap();
    save_project_file_atomic(&path, &foreign).unwrap();

    assert!(matches!(
        inspect_project_recovery(&path).unwrap(),
        RecoveryInspection::Conflict {
            reason: RecoveryConflictReason::ForeignProject,
            ..
        }
    ));
    assert!(matches!(
        apply_project_recovery(&path),
        Err(ProjectRecoveryError::RecoveryConflict)
    ));
    assert_eq!(load_project_file(&path).unwrap(), foreign);
}

#[test]
fn missing_canonical_is_orphaned_conflict_and_is_not_recreated() {
    let directory = TestDirectory::new();
    let path = directory.project_path();
    let base = ProjectDocument::new("A");
    let (recovery, _) = rename(&base, &["B"]);
    save_project_file_atomic(&path, &base).unwrap();
    write_recovery_checkpoint(&path, &base, &recovery).unwrap();
    fs::remove_file(&path).unwrap();

    assert!(matches!(
        inspect_project_recovery(&path).unwrap(),
        RecoveryInspection::Conflict {
            reason: RecoveryConflictReason::Orphaned,
            ..
        }
    ));
    assert!(matches!(
        apply_project_recovery(&path),
        Err(ProjectRecoveryError::RecoveryConflict)
    ));
    assert!(!path.exists());
    assert!(recovery_path(&path).exists());
}

#[test]
fn malformed_recovery_returns_controlled_error_and_canonical_remains_loadable() {
    let directory = TestDirectory::new();
    let path = directory.project_path();
    let base = ProjectDocument::new("A");
    save_project_file_atomic(&path, &base).unwrap();
    fs::write(recovery_path(&path), b"{ broken").unwrap();

    assert!(matches!(
        inspect_project_recovery(&path),
        Err(ProjectRecoveryError::InvalidEnvelope)
    ));
    assert_eq!(load_project_file(&path).unwrap(), base);
}

#[test]
fn wrong_recovery_format_marker_is_rejected() {
    let directory = TestDirectory::new();
    let path = directory.project_path();
    let base = ProjectDocument::new("A");
    let (recovery, _) = rename(&base, &["B"]);
    save_project_file_atomic(&path, &base).unwrap();
    let mut wire = recovery_wire(&base, &recovery);
    wire["format"] = json!("wrong-format");
    write_raw_recovery(&path, &wire);

    assert!(matches!(
        inspect_project_recovery(&path),
        Err(ProjectRecoveryError::WrongFormatMarker)
    ));
}

#[test]
fn unsupported_recovery_schema_is_rejected_without_v1_decode() {
    let directory = TestDirectory::new();
    let path = directory.project_path();
    let base = ProjectDocument::new("A");
    let (recovery, _) = rename(&base, &["B"]);
    save_project_file_atomic(&path, &base).unwrap();
    let mut wire = recovery_wire(&base, &recovery);
    wire["schema_version"] = json!(2);
    write_raw_recovery(&path, &wire);

    assert!(matches!(
        inspect_project_recovery(&path),
        Err(ProjectRecoveryError::UnsupportedSchemaVersion(2))
    ));
}

#[test]
fn unknown_recovery_root_fields_are_rejected() {
    let directory = TestDirectory::new();
    let path = directory.project_path();
    let base = ProjectDocument::new("A");
    let (recovery, _) = rename(&base, &["B"]);
    save_project_file_atomic(&path, &base).unwrap();
    let mut wire = recovery_wire(&base, &recovery);
    wire["unexpected"] = json!(true);
    write_raw_recovery(&path, &wire);

    assert!(matches!(
        inspect_project_recovery(&path),
        Err(ProjectRecoveryError::InvalidEnvelope)
    ));
}

#[test]
fn invalid_nested_base_uses_project_codec_validation() {
    let directory = TestDirectory::new();
    let path = directory.project_path();
    let base = ProjectDocument::new("A");
    let (recovery, _) = rename(&base, &["B"]);
    save_project_file_atomic(&path, &base).unwrap();
    let mut wire = recovery_wire(&base, &recovery);
    wire["base_project"] = json!({ "invalid": true });
    write_raw_recovery(&path, &wire);

    assert!(matches!(
        inspect_project_recovery(&path),
        Err(ProjectRecoveryError::InvalidBaseProject(_))
    ));
}

#[test]
fn invalid_nested_recovery_uses_project_codec_validation() {
    let directory = TestDirectory::new();
    let path = directory.project_path();
    let base = ProjectDocument::new("A");
    let (recovery, _) = rename(&base, &["B"]);
    save_project_file_atomic(&path, &base).unwrap();
    let mut wire = recovery_wire(&base, &recovery);
    wire["recovery_project"] = json!({ "invalid": true });
    write_raw_recovery(&path, &wire);

    assert!(matches!(
        inspect_project_recovery(&path),
        Err(ProjectRecoveryError::InvalidRecoveryProject(_))
    ));
}

#[test]
fn oversized_recovery_is_rejected_before_reading_the_payload() {
    let directory = TestDirectory::new();
    let path = directory.project_path();
    save_project_file_atomic(&path, &ProjectDocument::new("A")).unwrap();
    File::create(recovery_path(&path))
        .unwrap()
        .set_len(MAX_RECOVERY_FILE_BYTES + 1)
        .unwrap();

    assert!(matches!(
        inspect_project_recovery(&path),
        Err(ProjectRecoveryError::FileTooLarge { max_bytes })
            if max_bytes == MAX_RECOVERY_FILE_BYTES
    ));
}

#[test]
fn invalid_recovery_utf8_is_rejected_without_lossy_conversion() {
    let directory = TestDirectory::new();
    let path = directory.project_path();
    save_project_file_atomic(&path, &ProjectDocument::new("A")).unwrap();
    fs::write(recovery_path(&path), [0xff, 0xfe]).unwrap();

    assert!(matches!(
        inspect_project_recovery(&path),
        Err(ProjectRecoveryError::InvalidUtf8)
    ));
}

#[test]
fn discard_valid_checkpoint_is_idempotent_and_leaves_canonical_unchanged() {
    let directory = TestDirectory::new();
    let path = directory.project_path();
    let base = ProjectDocument::new("A");
    let (recovery, _) = rename(&base, &["B"]);
    save_project_file_atomic(&path, &base).unwrap();
    write_recovery_checkpoint(&path, &base, &recovery).unwrap();

    assert!(discard_project_recovery(&path).unwrap());
    assert!(!discard_project_recovery(&path).unwrap());
    assert!(!recovery_path(&path).exists());
    assert_eq!(load_project_file(&path).unwrap(), base);
}

#[test]
fn discard_malformed_checkpoint_does_not_require_decode_or_change_project() {
    let directory = TestDirectory::new();
    let path = directory.project_path();
    let base = ProjectDocument::new("A");
    save_project_file_atomic(&path, &base).unwrap();
    fs::write(recovery_path(&path), b"not valid recovery JSON").unwrap();

    assert!(discard_project_recovery(&path).unwrap());
    assert!(!recovery_path(&path).exists());
    assert_eq!(load_project_file(&path).unwrap(), base);
}

#[test]
fn apply_revalidates_after_prior_inspection_and_preserves_conflicting_checkpoint() {
    let directory = TestDirectory::new();
    let path = directory.project_path();
    let base = ProjectDocument::new("A");
    let (recovery, _) = rename(&base, &["B"]);
    let changed = project_at(&base, base.revision(), "External");
    save_project_file_atomic(&path, &base).unwrap();
    write_recovery_checkpoint(&path, &base, &recovery).unwrap();
    assert!(matches!(
        inspect_project_recovery(&path).unwrap(),
        RecoveryInspection::Candidate(_)
    ));
    save_project_file_atomic(&path, &changed).unwrap();

    assert!(matches!(
        apply_project_recovery(&path),
        Err(ProjectRecoveryError::RecoveryConflict)
    ));
    assert_eq!(load_project_file(&path).unwrap(), changed);
    assert!(recovery_path(&path).exists());
}

#[test]
fn same_recovery_revision_with_different_content_is_conflict() {
    let directory = TestDirectory::new();
    let path = directory.project_path();
    let base = ProjectDocument::new("A");
    let (recovery, _) = rename(&base, &["B"]);
    let changed = project_at(&recovery, recovery.revision(), "Different at same revision");
    save_project_file_atomic(&path, &base).unwrap();
    write_recovery_checkpoint(&path, &base, &recovery).unwrap();
    save_project_file_atomic(&path, &changed).unwrap();

    assert!(matches!(
        inspect_project_recovery(&path).unwrap(),
        RecoveryInspection::Conflict {
            reason: RecoveryConflictReason::ChangedLineage,
            ..
        }
    ));
}

#[test]
fn canonical_rollback_below_base_revision_is_conflict() {
    let directory = TestDirectory::new();
    let path = directory.project_path();
    let (base, _) = rename(&ProjectDocument::new("A"), &["B", "C"]);
    let (recovery, _) = rename(&base, &["D"]);
    let rollback = project_at(&base, ProjectRevision::new(1), "Rollback");
    save_project_file_atomic(&path, &base).unwrap();
    write_recovery_checkpoint(&path, &base, &recovery).unwrap();
    save_project_file_atomic(&path, &rollback).unwrap();

    assert!(matches!(
        inspect_project_recovery(&path).unwrap(),
        RecoveryInspection::Conflict { .. }
    ));
}

#[test]
fn successful_checkpoint_replacement_and_apply_leave_no_temp_orphans() {
    let directory = TestDirectory::new();
    let path = directory.project_path();
    let base = ProjectDocument::new("A");
    let (first, _) = rename(&base, &["B"]);
    let (latest, _) = rename(&first, &["C"]);
    save_project_file_atomic(&path, &base).unwrap();
    write_recovery_checkpoint(&path, &base, &first).unwrap();
    assert_no_temp_orphans(&directory.0);
    write_recovery_checkpoint(&path, &base, &latest).unwrap();
    assert_no_temp_orphans(&directory.0);
    apply_project_recovery(&path).unwrap();
    assert_no_temp_orphans(&directory.0);
}

#[test]
fn checkpoint_inspection_apply_and_discard_work_with_unicode_paths() {
    let directory = TestDirectory::new();
    let unicode_directory = directory.0.join("Tiếng Việt 🎬");
    fs::create_dir(&unicode_directory).unwrap();
    let path = unicode_directory.join("dự án.orproj");
    let base = ProjectDocument::new("A");
    let (recovery, _) = rename(&base, &["B"]);
    save_project_file_atomic(&path, &base).unwrap();
    write_recovery_checkpoint(&path, &base, &recovery).unwrap();
    assert!(matches!(
        inspect_project_recovery(&path).unwrap(),
        RecoveryInspection::Candidate(_)
    ));
    assert!(discard_project_recovery(&path).unwrap());
    assert_eq!(load_project_file(&path).unwrap(), base);

    write_recovery_checkpoint(&path, &base, &recovery).unwrap();
    apply_project_recovery(&path).unwrap();
    assert_eq!(load_project_file(&path).unwrap(), recovery);
    assert!(!recovery_path(&path).exists());
}
