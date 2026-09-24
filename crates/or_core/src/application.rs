use crate::{ProjectDocument, ProjectId, ProjectInstanceId, ProjectRevision};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{error::Error, fmt};

const PROJECT_RENAME_ID: &str = "project.rename";
const PROJECT_SUMMARY_ID: &str = "project.summary";
const HISTORY_UNDO_ID: &str = "history.undo";
const HISTORY_REDO_ID: &str = "history.redo";
const OPERATION_SCHEMA_VERSION: u64 = 1;
pub const CURRENT_TRANSACTION_SCHEMA_VERSION: u64 = 1;

/// Static discovery information for a command implemented by the core.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct CommandDescriptor {
    pub id: &'static str,
    pub schema_version: u64,
    pub mutates_project: bool,
    pub allowed_in_transaction: bool,
}

/// Static discovery information for a query implemented by the core.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct QueryDescriptor {
    pub id: &'static str,
    pub schema_version: u64,
}

const COMMANDS: [CommandDescriptor; 3] = [
    CommandDescriptor {
        id: PROJECT_RENAME_ID,
        schema_version: OPERATION_SCHEMA_VERSION,
        mutates_project: true,
        allowed_in_transaction: true,
    },
    CommandDescriptor {
        id: HISTORY_UNDO_ID,
        schema_version: OPERATION_SCHEMA_VERSION,
        mutates_project: true,
        allowed_in_transaction: false,
    },
    CommandDescriptor {
        id: HISTORY_REDO_ID,
        schema_version: OPERATION_SCHEMA_VERSION,
        mutates_project: true,
        allowed_in_transaction: false,
    },
];

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

/// A child operation in a grouped transaction. State preconditions live on the transaction.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandCall {
    pub command_id: String,
    pub schema_version: u64,
    pub arguments: Value,
}

/// A versioned atomic group of command calls.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransactionEnvelope {
    pub schema_version: u64,
    pub project_id: ProjectId,
    pub project_instance_id: ProjectInstanceId,
    pub expected_project_revision: ProjectRevision,
    pub commands: Vec<CommandCall>,
}

/// A read-only query request for the active project session.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QueryEnvelope {
    pub query_id: String,
    pub schema_version: u64,
    pub project_id: ProjectId,
    pub project_instance_id: ProjectInstanceId,
    pub arguments: Value,
}

/// One canonical project-state delta produced by a transaction.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ProjectChange {
    ProjectName { before: String, after: String },
}

/// Net canonical content changes produced by one transaction.
///
/// A ChangeSet is a result description, not a mutation request.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct ChangeSet {
    changes: Vec<ProjectChange>,
}

impl ChangeSet {
    /// Returns the transaction's normalized canonical changes.
    pub fn changes(&self) -> &[ProjectChange] {
        &self.changes
    }

    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }

    fn project_name(before: &str, after: &str) -> Self {
        if before == after {
            Self::default()
        } else {
            Self {
                changes: vec![ProjectChange::ProjectName {
                    before: before.to_owned(),
                    after: after.to_owned(),
                }],
            }
        }
    }

    fn apply_to_name(
        &self,
        current_name: &str,
        reverse: bool,
    ) -> Result<(String, Self), OperationError> {
        let [ProjectChange::ProjectName { before, after }] = self.changes.as_slice() else {
            return Err(OperationError::new(OperationErrorCode::HistoryConflict));
        };
        let (expected, target) = if reverse {
            (after, before)
        } else {
            (before, after)
        };
        if current_name != expected {
            return Err(OperationError::new(OperationErrorCode::HistoryConflict));
        }

        Ok((target.clone(), Self::project_name(current_name, target)))
    }
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
    pub change_set: ChangeSet,
}

/// Result of applying one transaction.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct TransactionResult {
    pub schema_version: u64,
    pub project_id: ProjectId,
    pub project_instance_id: ProjectInstanceId,
    pub before_revision: ProjectRevision,
    pub after_revision: ProjectRevision,
    pub changed: bool,
    pub command_count: usize,
    pub change_set: ChangeSet,
}

/// Canonical and runtime identity values returned by project.summary.
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
    UnsupportedTransactionSchema,
    EmptyTransaction,
    CommandNotAllowedInTransaction,
    ProjectIdMismatch,
    ProjectInstanceMismatch,
    RevisionConflict,
    InvalidArguments,
    RevisionOverflow,
    NothingToUndo,
    NothingToRedo,
    HistoryConflict,
    HistoryStorageFailure,
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
            OperationErrorCode::UnsupportedTransactionSchema => "unsupported transaction schema",
            OperationErrorCode::EmptyTransaction => "transaction contains no commands",
            OperationErrorCode::CommandNotAllowedInTransaction => {
                "command is not allowed in a transaction"
            }
            OperationErrorCode::ProjectIdMismatch => "command or query targets another project",
            OperationErrorCode::ProjectInstanceMismatch => {
                "command or query targets another project instance"
            }
            OperationErrorCode::RevisionConflict => "expected project revision is stale",
            OperationErrorCode::InvalidArguments => "operation arguments are invalid",
            OperationErrorCode::RevisionOverflow => "project revision cannot be incremented",
            OperationErrorCode::NothingToUndo => "there is no project change to undo",
            OperationErrorCode::NothingToRedo => "there is no project change to redo",
            OperationErrorCode::HistoryConflict => {
                "project state does not match the history change"
            }
            OperationErrorCode::HistoryStorageFailure => {
                "session history could not reserve storage"
            }
        };

        formatter.write_str(message)?;
        if let Some(revision) = self.current_revision {
            write!(formatter, " (current revision: {revision})")?;
        }
        Ok(())
    }
}

impl Error for OperationError {}

#[derive(Debug, Default, Eq, PartialEq)]
struct SessionHistory {
    undo: Vec<ChangeSet>,
    redo: Vec<ChangeSet>,
}

#[derive(Clone, Copy)]
enum HistoryDirection {
    Undo,
    Redo,
}

/// One loaded runtime instance of a canonical project document.
///
/// The instance ID and transaction history are session-only and never part of the
/// persisted document.
#[derive(Debug, Eq, PartialEq)]
pub struct ProjectSession {
    project: ProjectDocument,
    project_instance_id: ProjectInstanceId,
    history: SessionHistory,
}

impl ProjectSession {
    /// Opens a document into a new runtime instance without changing its state.
    pub fn open(project: ProjectDocument) -> Self {
        Self {
            project,
            project_instance_id: ProjectInstanceId::generate(),
            history: SessionHistory::default(),
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

    /// Dispatches a versioned command after validating its project/session state.
    pub fn execute_command(
        &mut self,
        envelope: CommandEnvelope,
    ) -> Result<CommandResult, OperationError> {
        let descriptor = COMMANDS
            .iter()
            .find(|descriptor| descriptor.id == envelope.command_id.as_str())
            .ok_or_else(|| OperationError::new(OperationErrorCode::UnknownCommand))?;
        if envelope.schema_version != descriptor.schema_version {
            return Err(OperationError::new(
                OperationErrorCode::UnsupportedCommandSchema,
            ));
        }
        self.check_project_preconditions(
            envelope.project_id,
            envelope.project_instance_id,
            envelope.expected_project_revision,
        )?;

        let command_id = envelope.command_id;
        let schema_version = envelope.schema_version;
        let before_revision = self.project_revision();
        let change_set = match command_id.as_str() {
            PROJECT_RENAME_ID => {
                let call = CommandCall {
                    command_id: command_id.clone(),
                    schema_version,
                    arguments: envelope.arguments,
                };
                self.apply_forward_commands(std::slice::from_ref(&call))?.1
            }
            HISTORY_UNDO_ID => {
                parse_empty_arguments(envelope.arguments)?;
                self.apply_history(HistoryDirection::Undo)?.2
            }
            HISTORY_REDO_ID => {
                parse_empty_arguments(envelope.arguments)?;
                self.apply_history(HistoryDirection::Redo)?.2
            }
            _ => return Err(OperationError::new(OperationErrorCode::UnknownCommand)),
        };

        let after_revision = self.project_revision();
        Ok(CommandResult {
            command_id,
            schema_version,
            project_id: self.project_id(),
            project_instance_id: self.project_instance_id,
            before_revision,
            after_revision,
            changed: !change_set.is_empty(),
            change_set,
        })
    }

    /// Evaluates a command group in staged state and commits its net change atomically.
    pub fn execute_transaction(
        &mut self,
        envelope: TransactionEnvelope,
    ) -> Result<TransactionResult, OperationError> {
        if envelope.schema_version != CURRENT_TRANSACTION_SCHEMA_VERSION {
            return Err(OperationError::new(
                OperationErrorCode::UnsupportedTransactionSchema,
            ));
        }
        self.check_project_preconditions(
            envelope.project_id,
            envelope.project_instance_id,
            envelope.expected_project_revision,
        )?;
        if envelope.commands.is_empty() {
            return Err(OperationError::new(OperationErrorCode::EmptyTransaction));
        }

        let before_revision = self.project_revision();
        let command_count = envelope.commands.len();
        let (after_revision, change_set) = self.apply_forward_commands(&envelope.commands)?;
        Ok(TransactionResult {
            schema_version: CURRENT_TRANSACTION_SCHEMA_VERSION,
            project_id: self.project_id(),
            project_instance_id: self.project_instance_id,
            before_revision,
            after_revision,
            changed: !change_set.is_empty(),
            command_count,
            change_set,
        })
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

    fn check_project_preconditions(
        &self,
        project_id: ProjectId,
        project_instance_id: ProjectInstanceId,
        expected_revision: ProjectRevision,
    ) -> Result<(), OperationError> {
        if project_id != self.project_id() {
            return Err(OperationError::new(OperationErrorCode::ProjectIdMismatch));
        }
        if project_instance_id != self.project_instance_id {
            return Err(OperationError::new(
                OperationErrorCode::ProjectInstanceMismatch,
            ));
        }
        if expected_revision != self.project_revision() {
            return Err(OperationError::revision_conflict(self.project_revision()));
        }
        Ok(())
    }

    fn apply_forward_commands(
        &mut self,
        commands: &[CommandCall],
    ) -> Result<(ProjectRevision, ChangeSet), OperationError> {
        let before_revision = self.project_revision();
        let original_name = self.project.name().to_owned();
        let mut staged_name = original_name.clone();
        for command in commands {
            stage_groupable_command(command, &mut staged_name)?;
        }

        let change_set = ChangeSet::project_name(&original_name, &staged_name);
        if change_set.is_empty() {
            return Ok((before_revision, change_set));
        }

        let after_revision = before_revision
            .checked_next()
            .map_err(|_| OperationError::new(OperationErrorCode::RevisionOverflow))?;
        let history_entry = change_set.clone();
        self.history
            .undo
            .try_reserve(1)
            .map_err(|_| OperationError::new(OperationErrorCode::HistoryStorageFailure))?;

        // All validation, staging, revision checks, and history allocation are complete.
        self.project.rename_for_command(staged_name, after_revision);
        self.history.undo.push(history_entry);
        self.history.redo.clear();
        Ok((after_revision, change_set))
    }

    fn apply_history(
        &mut self,
        direction: HistoryDirection,
    ) -> Result<(ProjectRevision, ProjectRevision, ChangeSet), OperationError> {
        let entry = match direction {
            HistoryDirection::Undo => self
                .history
                .undo
                .last()
                .cloned()
                .ok_or_else(|| OperationError::new(OperationErrorCode::NothingToUndo))?,
            HistoryDirection::Redo => self
                .history
                .redo
                .last()
                .cloned()
                .ok_or_else(|| OperationError::new(OperationErrorCode::NothingToRedo))?,
        };
        let (target_name, applied_change) = entry.apply_to_name(
            self.project.name(),
            matches!(direction, HistoryDirection::Undo),
        )?;
        let before_revision = self.project_revision();
        let after_revision = before_revision
            .checked_next()
            .map_err(|_| OperationError::new(OperationErrorCode::RevisionOverflow))?;

        match direction {
            HistoryDirection::Undo => self.history.redo.try_reserve(1),
            HistoryDirection::Redo => self.history.undo.try_reserve(1),
        }
        .map_err(|_| OperationError::new(OperationErrorCode::HistoryStorageFailure))?;

        let moved_entry = match direction {
            HistoryDirection::Undo => self.history.undo.pop(),
            HistoryDirection::Redo => self.history.redo.pop(),
        }
        .ok_or_else(|| OperationError::new(OperationErrorCode::HistoryConflict))?;

        // The destination stack was reserved and the paired document update cannot fail.
        self.project.rename_for_command(target_name, after_revision);
        match direction {
            HistoryDirection::Undo => self.history.redo.push(moved_entry),
            HistoryDirection::Redo => self.history.undo.push(moved_entry),
        }
        Ok((before_revision, after_revision, applied_change))
    }
}

fn stage_groupable_command(
    command: &CommandCall,
    staged_name: &mut String,
) -> Result<(), OperationError> {
    let descriptor = COMMANDS
        .iter()
        .find(|descriptor| descriptor.id == command.command_id.as_str())
        .ok_or_else(|| OperationError::new(OperationErrorCode::UnknownCommand))?;
    if command.schema_version != descriptor.schema_version {
        return Err(OperationError::new(
            OperationErrorCode::UnsupportedCommandSchema,
        ));
    }
    if !descriptor.allowed_in_transaction {
        return Err(OperationError::new(
            OperationErrorCode::CommandNotAllowedInTransaction,
        ));
    }

    let arguments: RenameArguments = serde_json::from_value(command.arguments.clone())
        .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
    *staged_name = arguments.name;
    Ok(())
}

fn parse_empty_arguments(arguments: Value) -> Result<(), OperationError> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct EmptyArguments {}

    serde_json::from_value::<EmptyArguments>(arguments)
        .map(|_| ())
        .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RenameArguments {
    name: String,
}
#[cfg(test)]
mod tests {
    use super::{
        COMMANDS, CURRENT_TRANSACTION_SCHEMA_VERSION, ChangeSet, CommandCall, CommandDescriptor,
        CommandEnvelope, OperationErrorCode, ProjectChange, ProjectSession, QUERIES,
        QueryDescriptor, QueryEnvelope, TransactionEnvelope, command_catalog, query_catalog,
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
            history: super::SessionHistory::default(),
        }
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
            ]
        );
        assert_eq!(
            query_catalog(),
            &[QueryDescriptor {
                id: "project.summary",
                schema_version: 1,
            }]
        );
        assert_eq!(command_catalog(), command_catalog());
        assert_eq!(COMMANDS.len(), 3);
        assert_eq!(QUERIES.len(), 1);
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
    fn grouped_transaction_persists_only_canonical_state_through_orproj_v1() {
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
}
