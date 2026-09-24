use crate::{ProjectDocument, ProjectId, ProjectInstanceId, ProjectRevision};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{error::Error, fmt};

const PROJECT_RENAME_ID: &str = "project.rename";
const PROJECT_SUMMARY_ID: &str = "project.summary";
const OPERATION_SCHEMA_VERSION: u64 = 1;

/// Static discovery information for a command implemented by the core.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct CommandDescriptor {
    pub id: &'static str,
    pub schema_version: u64,
    pub mutates_project: bool,
}

/// Static discovery information for a query implemented by the core.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct QueryDescriptor {
    pub id: &'static str,
    pub schema_version: u64,
}

const COMMANDS: [CommandDescriptor; 1] = [CommandDescriptor {
    id: PROJECT_RENAME_ID,
    schema_version: OPERATION_SCHEMA_VERSION,
    mutates_project: true,
}];

const QUERIES: [QueryDescriptor; 1] = [QueryDescriptor {
    id: PROJECT_SUMMARY_ID,
    schema_version: OPERATION_SCHEMA_VERSION,
}];

/// Returns the deterministic catalog of currently supported commands.
pub fn command_catalog() -> &'static [CommandDescriptor] {
    &COMMANDS
}

/// Returns the deterministic catalog of currently supported queries.
pub fn query_catalog() -> &'static [QueryDescriptor] {
    &QUERIES
}

/// A versioned command request with typed project/session/revision preconditions.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandEnvelope {
    pub command_id: String,
    pub schema_version: u64,
    pub project_id: ProjectId,
    pub project_instance_id: ProjectInstanceId,
    pub expected_project_revision: ProjectRevision,
    pub arguments: Value,
}

/// A versioned read request for the active project session.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QueryEnvelope {
    pub query_id: String,
    pub schema_version: u64,
    pub project_id: ProjectId,
    pub project_instance_id: ProjectInstanceId,
    pub arguments: Value,
}

/// Result of applying one command to a project session.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CommandResult {
    pub command_id: String,
    pub schema_version: u64,
    pub project_id: ProjectId,
    pub project_instance_id: ProjectInstanceId,
    pub before_revision: ProjectRevision,
    pub after_revision: ProjectRevision,
    pub changed: bool,
}

/// Canonical and runtime identity values returned by `project.summary`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ProjectSummary {
    pub project_id: ProjectId,
    pub project_instance_id: ProjectInstanceId,
    pub project_revision: ProjectRevision,
    pub name: String,
}

/// Result of a project query. The summary is a read-only snapshot of current state.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct QueryResult {
    pub query_id: String,
    pub schema_version: u64,
    #[serde(flatten)]
    pub summary: ProjectSummary,
}

/// Stable machine-readable operation failure categories.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OperationErrorCode {
    UnknownCommand,
    UnsupportedCommandSchema,
    UnknownQuery,
    UnsupportedQuerySchema,
    ProjectIdMismatch,
    ProjectInstanceMismatch,
    RevisionConflict,
    InvalidArguments,
    RevisionOverflow,
}

/// A safe structured operation error with a stable code and optional context.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationError {
    pub code: OperationErrorCode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_revision: Option<ProjectRevision>,
}

impl OperationError {
    fn new(code: OperationErrorCode) -> Self {
        Self {
            code,
            current_revision: None,
        }
    }

    fn revision_conflict(current_revision: ProjectRevision) -> Self {
        Self {
            code: OperationErrorCode::RevisionConflict,
            current_revision: Some(current_revision),
        }
    }
}

impl fmt::Display for OperationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self.code {
            OperationErrorCode::UnknownCommand => "unknown command",
            OperationErrorCode::UnsupportedCommandSchema => "unsupported command schema",
            OperationErrorCode::UnknownQuery => "unknown query",
            OperationErrorCode::UnsupportedQuerySchema => "unsupported query schema",
            OperationErrorCode::ProjectIdMismatch => "command or query targets another project",
            OperationErrorCode::ProjectInstanceMismatch => {
                "command or query targets another project instance"
            }
            OperationErrorCode::RevisionConflict => "expected project revision is stale",
            OperationErrorCode::InvalidArguments => "operation arguments are invalid",
            OperationErrorCode::RevisionOverflow => "project revision cannot be incremented",
        };

        formatter.write_str(message)?;
        if let Some(revision) = self.current_revision {
            write!(formatter, " (current revision: {revision})")?;
        }
        Ok(())
    }
}

impl Error for OperationError {}

/// One loaded runtime instance of a canonical project document.
///
/// The instance ID is session-only and is never part of the persisted document.
#[derive(Debug, Eq, PartialEq)]
pub struct ProjectSession {
    project: ProjectDocument,
    project_instance_id: ProjectInstanceId,
}

impl ProjectSession {
    /// Opens a document into a new runtime instance without changing its state.
    pub fn open(project: ProjectDocument) -> Self {
        Self {
            project,
            project_instance_id: ProjectInstanceId::generate(),
        }
    }

    pub const fn project_id(&self) -> ProjectId {
        self.project.id()
    }

    pub const fn project_instance_id(&self) -> ProjectInstanceId {
        self.project_instance_id
    }

    pub const fn project_revision(&self) -> ProjectRevision {
        self.project.revision()
    }

    pub const fn project(&self) -> &ProjectDocument {
        &self.project
    }

    pub fn into_project(self) -> ProjectDocument {
        self.project
    }

    /// Dispatches and validates a structured command before any canonical mutation.
    pub fn execute_command(
        &mut self,
        envelope: CommandEnvelope,
    ) -> Result<CommandResult, OperationError> {
        if envelope.command_id != PROJECT_RENAME_ID {
            return Err(OperationError::new(OperationErrorCode::UnknownCommand));
        }
        if envelope.schema_version != OPERATION_SCHEMA_VERSION {
            return Err(OperationError::new(
                OperationErrorCode::UnsupportedCommandSchema,
            ));
        }
        if envelope.project_id != self.project_id() {
            return Err(OperationError::new(OperationErrorCode::ProjectIdMismatch));
        }
        if envelope.project_instance_id != self.project_instance_id {
            return Err(OperationError::new(
                OperationErrorCode::ProjectInstanceMismatch,
            ));
        }
        if envelope.expected_project_revision != self.project_revision() {
            return Err(OperationError::revision_conflict(self.project_revision()));
        }

        let arguments: RenameArguments = serde_json::from_value(envelope.arguments)
            .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
        let before_revision = self.project_revision();
        if arguments.name == self.project.name() {
            return Ok(self.command_result(before_revision, before_revision, false));
        }

        let after_revision = before_revision
            .checked_next()
            .map_err(|_| OperationError::new(OperationErrorCode::RevisionOverflow))?;

        // Every fallible check is complete; these paired field updates cannot fail.
        self.project
            .rename_for_command(arguments.name, after_revision);
        Ok(self.command_result(before_revision, after_revision, true))
    }

    /// Dispatches and executes a read-only query against current session state.
    pub fn execute_query(&self, envelope: QueryEnvelope) -> Result<QueryResult, OperationError> {
        if envelope.query_id != PROJECT_SUMMARY_ID {
            return Err(OperationError::new(OperationErrorCode::UnknownQuery));
        }
        if envelope.schema_version != OPERATION_SCHEMA_VERSION {
            return Err(OperationError::new(
                OperationErrorCode::UnsupportedQuerySchema,
            ));
        }
        if envelope.project_id != self.project_id() {
            return Err(OperationError::new(OperationErrorCode::ProjectIdMismatch));
        }
        if envelope.project_instance_id != self.project_instance_id {
            return Err(OperationError::new(
                OperationErrorCode::ProjectInstanceMismatch,
            ));
        }
        if !envelope
            .arguments
            .as_object()
            .is_some_and(|arguments| arguments.is_empty())
        {
            return Err(OperationError::new(OperationErrorCode::InvalidArguments));
        }

        Ok(QueryResult {
            query_id: PROJECT_SUMMARY_ID.to_owned(),
            schema_version: OPERATION_SCHEMA_VERSION,
            summary: ProjectSummary {
                project_id: self.project_id(),
                project_instance_id: self.project_instance_id,
                project_revision: self.project_revision(),
                name: self.project.name().to_owned(),
            },
        })
    }

    fn command_result(
        &self,
        before_revision: ProjectRevision,
        after_revision: ProjectRevision,
        changed: bool,
    ) -> CommandResult {
        CommandResult {
            command_id: PROJECT_RENAME_ID.to_owned(),
            schema_version: OPERATION_SCHEMA_VERSION,
            project_id: self.project_id(),
            project_instance_id: self.project_instance_id,
            before_revision,
            after_revision,
            changed,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RenameArguments {
    name: String,
}

#[cfg(test)]
mod tests {
    use super::{
        COMMANDS, CommandDescriptor, CommandEnvelope, OperationErrorCode, ProjectSession, QUERIES,
        QueryDescriptor, QueryEnvelope, command_catalog, query_catalog,
    };
    use crate::{
        ProjectDocument, ProjectId, ProjectInstanceId, ProjectRevision, decode_project,
        encode_project,
    };
    use serde_json::{Value, json};
    use std::str::FromStr;

    const PROJECT_ID: &str = "01234567-89ab-4def-8123-456789abcdef";
    const OTHER_PROJECT_ID: &str = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa";
    const INSTANCE_ID: &str = "fedcba98-7654-4cba-8fed-cba987654321";
    const OTHER_INSTANCE_ID: &str = "11111111-1111-4111-8111-111111111111";

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
    fn catalogs_are_exact_unique_and_deterministic() {
        assert_eq!(
            command_catalog(),
            &[CommandDescriptor {
                id: "project.rename",
                schema_version: 1,
                mutates_project: true,
            }]
        );
        assert_eq!(
            query_catalog(),
            &[QueryDescriptor {
                id: "project.summary",
                schema_version: 1,
            }]
        );
        assert_eq!(COMMANDS.len(), 1);
        assert_eq!(QUERIES.len(), 1);
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
            project: decode_project(&format!(
                r#"{{"format":"opencut-reinforced-project","schema_version":1,"project":{{"id":"{PROJECT_ID}","revision":{u64::MAX},"name":"A"}}}}"#
            ))
            .unwrap(),
            project_instance_id: ProjectInstanceId::from_str(INSTANCE_ID).unwrap(),
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
}
