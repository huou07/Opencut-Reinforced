use serde_json::Value;

use super::timeline::*;
use super::*;

impl ProjectSession {
    pub(super) fn apply_media_add(
        &mut self,
        arguments: Value,
    ) -> Result<ChangeSet, OperationError> {
        let arguments: MediaAddArguments = serde_json::from_value(arguments)
            .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
        if arguments.item.metadata().validate().is_err() {
            return Err(OperationError::new(OperationErrorCode::InvalidArguments));
        }
        ensure_media_id_available(&self.project, arguments.item.id())?;
        ensure_media_source_available(&self.project, arguments.item.source())?;

        let before_revision = self.project_revision();
        let after_revision = before_revision
            .checked_next()
            .map_err(|_| OperationError::new(OperationErrorCode::RevisionOverflow))?;
        self.project
            .try_reserve_media_items(1)
            .map_err(|_| OperationError::new(OperationErrorCode::HistoryStorageFailure))?;
        self.history
            .undo
            .try_reserve(1)
            .map_err(|_| OperationError::new(OperationErrorCode::HistoryStorageFailure))?;

        let index = self.project.media_items().len();
        let change_set = ChangeSet::media_added(arguments.item.clone(), index);
        // All validation, revision checks, and required storage allocations are complete.
        self.project
            .insert_media_for_command(arguments.item, index, after_revision);
        self.history.undo.push(change_set.clone());
        self.history.redo.clear();
        Ok(change_set)
    }

    pub(super) fn apply_media_remove(
        &mut self,
        arguments: Value,
    ) -> Result<ChangeSet, OperationError> {
        let arguments: MediaRemoveArguments = serde_json::from_value(arguments)
            .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
        let Some(index) = self
            .project
            .media_items()
            .iter()
            .position(|item| item.id() == arguments.id)
        else {
            return Err(OperationError::new(OperationErrorCode::MediaNotFound));
        };
        if self.project.timeline().references_media(arguments.id) {
            return Err(OperationError::new(OperationErrorCode::MediaInUse));
        }

        let before_revision = self.project_revision();
        let after_revision = before_revision
            .checked_next()
            .map_err(|_| OperationError::new(OperationErrorCode::RevisionOverflow))?;
        self.history
            .undo
            .try_reserve(1)
            .map_err(|_| OperationError::new(OperationErrorCode::HistoryStorageFailure))?;

        let item = self.project.media_items()[index].clone();
        let change_set = ChangeSet::media_removed(item, index);
        // All validation, revision checks, and history allocation are complete.
        self.project.remove_media_for_command(index, after_revision);
        self.history.undo.push(change_set.clone());
        self.history.redo.clear();
        Ok(change_set)
    }

    pub(super) fn apply_media_relink(
        &mut self,
        arguments: Value,
    ) -> Result<ChangeSet, OperationError> {
        let arguments: MediaRelinkArguments = serde_json::from_value(arguments)
            .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
        if arguments.item.metadata().validate().is_err() {
            return Err(OperationError::new(OperationErrorCode::InvalidArguments));
        }
        let Some(index) = self
            .project
            .media_items()
            .iter()
            .position(|item| item.id() == arguments.item.id())
        else {
            return Err(OperationError::new(OperationErrorCode::MediaNotFound));
        };
        let before = &self.project.media_items()[index];
        if before == &arguments.item {
            return Ok(ChangeSet::default());
        }
        if self
            .project
            .media_items()
            .iter()
            .enumerate()
            .any(|(candidate_index, item)| {
                candidate_index != index && item.source() == arguments.item.source()
            })
        {
            return Err(OperationError::new(
                OperationErrorCode::MediaSourceAlreadyExists,
            ));
        }
        self.project
            .timeline()
            .validate_replacing_media(self.project.media_items(), &arguments.item)
            .map_err(|_| OperationError::new(OperationErrorCode::TimelineMediaIncompatible))?;

        let after_revision = self
            .project_revision()
            .checked_next()
            .map_err(|_| OperationError::new(OperationErrorCode::RevisionOverflow))?;
        self.history
            .undo
            .try_reserve(1)
            .map_err(|_| OperationError::new(OperationErrorCode::HistoryStorageFailure))?;

        let change_set = ChangeSet::media_relinked(before.clone(), arguments.item.clone(), index)?;
        self.project
            .replace_media_for_command(index, arguments.item, after_revision);
        self.history.undo.push(change_set.clone());
        self.history.redo.clear();
        Ok(change_set)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MediaAddArguments {
    item: MediaItem,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MediaRemoveArguments {
    id: MediaId,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MediaRelinkArguments {
    item: MediaItem,
}

pub(super) fn deserialize_canonical_media_id<'de, D>(deserializer: D) -> Result<MediaId, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    let id = value.parse::<MediaId>().map_err(serde::de::Error::custom)?;
    if id.to_string() != value {
        return Err(serde::de::Error::custom(
            "media ID must use canonical lowercase UUID text",
        ));
    }
    Ok(id)
}
