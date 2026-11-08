//! Provenance for media a companion tool produced.
//!
//! A rendered video is not an editable composition. What makes it revisitable
//! is the record beside it: which tool, which command, which source project and
//! which manifest produced this file. The record is a plain JSON sidecar, so a
//! tool that OR does not own can still be reopened and the result re-rendered
//! without OR knowing how to render it.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::external_tools::{ToolCategory, tool};

/// The schema tag written into every record.
pub const PROVENANCE_SCHEMA: &str = "or.tool-provenance/1";

/// What produced one media file.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolProvenance {
    pub schema: String,
    pub tool_id: String,
    pub tool_name: String,
    pub tool_category: ToolCategory,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_version: Option<String>,
    /// The exact argument vector, without a shell.
    pub command: Vec<String>,
    /// The tool's own project file, when it has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_project: Option<String>,
    /// A manifest or scene file that describes the result.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_manifest: Option<String>,
    /// The file this record describes, as written into the project.
    pub output: String,
}

/// Why a record was rejected.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProvenanceError {
    UnknownTool,
    EmptyOutput,
    EmptyCommand,
    WrongSchema,
}

impl ProvenanceError {
    pub const fn message(self) -> &'static str {
        match self {
            Self::UnknownTool => "the tool id is not in the companion catalogue",
            Self::EmptyOutput => "the record does not name the file it describes",
            Self::EmptyCommand => "the record does not say how the file was produced",
            Self::WrongSchema => "the record is not an or.tool-provenance/1 document",
        }
    }
}

impl std::fmt::Display for ProvenanceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.message())
    }
}

impl std::error::Error for ProvenanceError {}

impl ToolProvenance {
    /// Builds a record from a catalog id and the command that ran.
    pub fn new(tool_id: &str, command: Vec<String>, output: &str) -> Result<Self, ProvenanceError> {
        let descriptor = tool(tool_id).ok_or(ProvenanceError::UnknownTool)?;
        let record = Self {
            schema: PROVENANCE_SCHEMA.to_owned(),
            tool_id: descriptor.id.to_owned(),
            tool_name: descriptor.name.to_owned(),
            tool_category: descriptor.category,
            tool_version: None,
            command,
            source_project: None,
            source_manifest: None,
            output: output.to_owned(),
        };
        record.validate()?;
        Ok(record)
    }

    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.tool_version = Some(version.into());
        self
    }

    pub fn with_source_project(mut self, path: impl Into<String>) -> Self {
        self.source_project = Some(path.into());
        self
    }

    pub fn with_source_manifest(mut self, path: impl Into<String>) -> Self {
        self.source_manifest = Some(path.into());
        self
    }

    pub fn validate(&self) -> Result<(), ProvenanceError> {
        if self.schema != PROVENANCE_SCHEMA {
            return Err(ProvenanceError::WrongSchema);
        }
        if tool(&self.tool_id).is_none() {
            return Err(ProvenanceError::UnknownTool);
        }
        if self.output.trim().is_empty() {
            return Err(ProvenanceError::EmptyOutput);
        }
        if self.command.is_empty() {
            return Err(ProvenanceError::EmptyCommand);
        }
        Ok(())
    }

    /// The sidecar path a media file uses: `clip.mp4.orprovenance.json`.
    pub fn sidecar_path(media: &Path) -> std::path::PathBuf {
        let mut name = media.as_os_str().to_owned();
        name.push(".orprovenance.json");
        std::path::PathBuf::from(name)
    }

    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self).map_err(|error| error.to_string())
    }

    pub fn from_json(text: &str) -> Result<Self, String> {
        let record: Self = serde_json::from_str(text).map_err(|error| error.to_string())?;
        record.validate().map_err(|error| error.to_string())?;
        Ok(record)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_record_round_trips_and_names_the_catalog_entry() {
        let record = ToolProvenance::new(
            "blender",
            vec!["blender".into(), "-b".into(), "scene.blend".into()],
            "shot.mp4",
        )
        .expect("blender is in the catalogue")
        .with_version("4.3.2")
        .with_source_project("scene.blend")
        .with_source_manifest("shot.manifest.json");
        let text = record.to_json().expect("a record encodes");
        let read = ToolProvenance::from_json(&text).expect("a record decodes");
        assert_eq!(read, record);
        assert_eq!(read.tool_name, "Blender");
        assert_eq!(read.tool_category, ToolCategory::ThreeD);
    }

    #[test]
    fn an_unknown_tool_is_rejected_before_a_record_exists() {
        assert_eq!(
            ToolProvenance::new("not-a-tool", vec!["x".into()], "out.mp4").unwrap_err(),
            ProvenanceError::UnknownTool
        );
    }

    #[test]
    fn a_record_without_a_command_or_output_is_rejected() {
        let mut record =
            ToolProvenance::new("ffmpeg", vec!["ffmpeg".into()], "out.mp4").expect("valid");
        record.command.clear();
        assert_eq!(record.validate(), Err(ProvenanceError::EmptyCommand));
        record.command.push("ffmpeg".into());
        record.output.clear();
        assert_eq!(record.validate(), Err(ProvenanceError::EmptyOutput));
    }

    #[test]
    fn a_foreign_document_is_not_a_record() {
        let text = r#"{"schema":"or.edit-plan/1","tool_id":"ffmpeg"}"#;
        assert!(ToolProvenance::from_json(text).is_err());
    }

    #[test]
    fn the_sidecar_sits_beside_the_media() {
        assert_eq!(
            ToolProvenance::sidecar_path(Path::new("/tmp/a.mp4")),
            std::path::PathBuf::from("/tmp/a.mp4.orprovenance.json")
        );
    }
}
