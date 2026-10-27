//! The declarative MotionAsset document (ADR 0010) and its instantiation.
//!
//! A motion asset is reusable authored content — a whole scene template, a
//! fragment of nodes, one animation preset, or a still image cutout. It is
//! **data, not a program**: same `deny_unknown_fields` discipline as a
//! MotionScene, no expression language, no network reference, no provider call.
//!
//! Adjustability is a bounded list of typed parameters. A parameter names one
//! node field, carries a default that must equal that field, and is substituted
//! into a copy of the payload before the result is validated as an ordinary
//! MotionScene. Nothing in this module touches the filesystem or a project.

use crate::{
    MAX_MOTION_ANIMATIONS, MAX_MOTION_KEYFRAMES, MAX_MOTION_NODE_DEPTH, MAX_MOTION_NODES,
    MAX_MOTION_TEXT_BYTES, MOTION_SCENE_FORMAT, MOTION_SCENE_SCHEMA_VERSION, MotionAnimation,
    MotionAsset, MotionCanvas, MotionColor, MotionDiagnostic, MotionEasing, MotionKeyframe,
    MotionNode, MotionNodeKind, MotionProperty, MotionScene, MotionValidation, is_uuid_v4,
    validate_scene,
};
use or_core::RationalTime;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fmt};

/// The `format` value every motion asset carries.
pub const MOTION_ASSET_FORMAT: &str = "opencut-reinforced-motion-asset";
/// The schema version this crate reads and writes.
pub const MOTION_ASSET_SCHEMA_VERSION: u32 = 1;
/// The largest asset document this accepts.
pub const MAX_MOTION_ASSET_BYTES: usize = 4 * 1024 * 1024;
/// The most parameters one asset may declare.
pub const MAX_MOTION_ASSET_PARAMETERS: usize = 64;
/// The most tags one asset may carry.
pub const MAX_MOTION_ASSET_TAGS: usize = 16;
/// The most bytes one tag may hold.
pub const MAX_MOTION_ASSET_TAG_BYTES: usize = 64;
/// The most bytes one asset name may hold.
pub const MAX_MOTION_ASSET_NAME_BYTES: usize = 128;
/// The most bytes one parameter id or label may hold.
pub const MAX_MOTION_ASSET_PARAMETER_LABEL_BYTES: usize = 128;
/// The most choices one `choice` parameter may offer.
pub const MAX_MOTION_ASSET_CHOICES: usize = 32;

/// A fresh UUIDv4 for a new asset or instantiated scene.
pub fn new_motion_asset_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// What the payload means when it is placed or reused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MotionAssetKind {
    /// A whole scene template.
    Scene,
    /// Nodes and animations merged into an existing scene.
    Fragment,
    /// One animation preset applied to a selected node.
    Motion,
    /// A still image cutout.
    Image,
}

impl MotionAssetKind {
    /// The stable wire spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Scene => "scene",
            Self::Fragment => "fragment",
            Self::Motion => "motion",
            Self::Image => "image",
        }
    }

    /// Parses the wire spelling.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "scene" => Some(Self::Scene),
            "fragment" => Some(Self::Fragment),
            "motion" => Some(Self::Motion),
            "image" => Some(Self::Image),
            _ => None,
        }
    }
}

/// The closed browse taxonomy. English wire values; synonyms are classifier
/// input, not extra categories.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MotionAssetCategory {
    Background,
    People,
    Nature,
    Animal,
    Vehicle,
    Object,
    Text,
    Shape,
    Icon,
    Chart,
    Effect,
    Motion,
}

impl MotionAssetCategory {
    /// The stable wire spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Background => "background",
            Self::People => "people",
            Self::Nature => "nature",
            Self::Animal => "animal",
            Self::Vehicle => "vehicle",
            Self::Object => "object",
            Self::Text => "text",
            Self::Shape => "shape",
            Self::Icon => "icon",
            Self::Chart => "chart",
            Self::Effect => "effect",
            Self::Motion => "motion",
        }
    }

    /// Parses the wire spelling.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "background" => Some(Self::Background),
            "people" => Some(Self::People),
            "nature" => Some(Self::Nature),
            "animal" => Some(Self::Animal),
            "vehicle" => Some(Self::Vehicle),
            "object" => Some(Self::Object),
            "text" => Some(Self::Text),
            "shape" => Some(Self::Shape),
            "icon" => Some(Self::Icon),
            "chart" => Some(Self::Chart),
            "effect" => Some(Self::Effect),
            "motion" => Some(Self::Motion),
            _ => None,
        }
    }

    fn keywords(self) -> &'static [&'static str] {
        match self {
            Self::Background => &["background", "backdrop", "bg", "wallpaper", "nền", "phông"],
            Self::People => &[
                "people",
                "person",
                "human",
                "man",
                "woman",
                "girl",
                "boy",
                "child",
                "crowd",
                "người",
                "đàn ông",
                "phụ nữ",
                "trẻ em",
            ],
            Self::Nature => &[
                "nature",
                "natural",
                "landscape",
                "mountain",
                "tree",
                "forest",
                "sky",
                "ocean",
                "sea",
                "beach",
                "flower",
                "thiên nhiên",
                "núi",
                "cây",
                "biển",
                "trời",
            ],
            Self::Animal => &[
                "animal",
                "dog",
                "cat",
                "bird",
                "fish",
                "horse",
                "tiger",
                "lion",
                "elephant",
                "động vật",
                "chó",
                "mèo",
                "chim",
                "cá",
                "hổ",
                "voi",
            ],
            Self::Vehicle => &[
                "vehicle",
                "car",
                "truck",
                "bus",
                "bike",
                "bicycle",
                "motorcycle",
                "train",
                "plane",
                "ship",
                "boat",
                "xe",
                "ô tô",
                "tàu",
                "máy bay",
            ],
            Self::Object => &["object", "prop", "vật thể", "đồ vật"],
            Self::Text => &["text", "title", "caption", "headline", "chữ", "tiêu đề"],
            Self::Shape => &[
                "shape",
                "circle",
                "rectangle",
                "square",
                "triangle",
                "arrow",
                "hình",
            ],
            Self::Icon => &["icon", "logo", "pictogram", "biểu tượng"],
            Self::Chart => &["chart", "graph", "diagram", "plot", "biểu đồ", "sơ đồ"],
            Self::Effect => &["effect", "glow", "blur", "particle", "hiệu ứng"],
            Self::Motion => &[
                "motion",
                "animation",
                "animate",
                "transition",
                "chuyển động",
                "hoạt hình",
            ],
        }
    }
}

/// Which rule produced the stored category.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MotionAssetCategorySource {
    /// The author or the user chose it.
    #[default]
    Explicit,
    /// A tag matched a category keyword.
    Tags,
    /// The name matched a category keyword.
    Filename,
    /// No rule matched; the kind default applied.
    Default,
}

impl MotionAssetCategorySource {
    /// The stable wire spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Explicit => "explicit",
            Self::Tags => "tags",
            Self::Filename => "filename",
            Self::Default => "default",
        }
    }
}

impl fmt::Display for MotionAssetKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl fmt::Display for MotionAssetCategory {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl fmt::Display for MotionAssetCategorySource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// What a classification decided, and why.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MotionClassification {
    pub category: MotionAssetCategory,
    pub source: MotionAssetCategorySource,
}

/// Classifies an asset deterministically: explicit, then tags, then name tokens,
/// then a kind default. No model call, no network, no locale dependence beyond
/// lowercasing.
pub fn classify_motion_asset(
    name: &str,
    tags: &[String],
    kind: MotionAssetKind,
    explicit: Option<MotionAssetCategory>,
) -> MotionClassification {
    if let Some(category) = explicit {
        return MotionClassification {
            category,
            source: MotionAssetCategorySource::Explicit,
        };
    }
    for tag in tags {
        if let Some(category) = category_for_text(tag) {
            return MotionClassification {
                category,
                source: MotionAssetCategorySource::Tags,
            };
        }
    }
    if let Some(category) = category_for_text(name) {
        return MotionClassification {
            category,
            source: MotionAssetCategorySource::Filename,
        };
    }
    MotionClassification {
        category: match kind {
            MotionAssetKind::Motion => MotionAssetCategory::Motion,
            _ => MotionAssetCategory::Object,
        },
        source: MotionAssetCategorySource::Default,
    }
}

fn category_for_text(value: &str) -> Option<MotionAssetCategory> {
    let tokens = tokens(value);
    if tokens.is_empty() {
        return None;
    }
    let lowered = value.to_lowercase();
    for category in [
        MotionAssetCategory::Background,
        MotionAssetCategory::People,
        MotionAssetCategory::Nature,
        MotionAssetCategory::Animal,
        MotionAssetCategory::Vehicle,
        MotionAssetCategory::Object,
        MotionAssetCategory::Text,
        MotionAssetCategory::Shape,
        MotionAssetCategory::Icon,
        MotionAssetCategory::Chart,
        MotionAssetCategory::Effect,
        MotionAssetCategory::Motion,
    ] {
        let keywords = category.keywords();
        let matched = keywords.iter().any(|keyword| {
            if keyword.contains(' ') {
                lowered.contains(keyword)
            } else {
                tokens.iter().any(|token| token.as_str() == *keyword)
            }
        });
        if matched {
            return Some(category);
        }
    }
    None
}

fn tokens(value: &str) -> Vec<String> {
    value
        .to_lowercase()
        .split(|character: char| !character.is_alphanumeric())
        .filter(|token| !token.is_empty())
        .map(str::to_owned)
        .collect()
}

/// One node field a parameter can address.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MotionParameterField {
    Text,
    FontSize,
    Fill,
    SizeWidth,
    SizeHeight,
    Opacity,
    RotationDegrees,
    AssetId,
}

impl MotionParameterField {
    /// The stable wire spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::FontSize => "font_size",
            Self::Fill => "fill",
            Self::SizeWidth => "size_width",
            Self::SizeHeight => "size_height",
            Self::Opacity => "opacity",
            Self::RotationDegrees => "rotation_degrees",
            Self::AssetId => "asset_id",
        }
    }
}

impl fmt::Display for MotionParameterField {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// A typed parameter value. A choice is a `Text` that must appear in its list.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MotionParameterValue {
    Text(String),
    Number(f64),
    Color(MotionColor),
}

/// The value kind a parameter declares.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MotionParameterKind {
    Text,
    Color,
    Number,
    Choice,
}

impl MotionParameterKind {
    /// The stable wire spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Color => "color",
            Self::Number => "number",
            Self::Choice => "choice",
        }
    }
}

/// The node field a parameter writes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MotionParameterTarget {
    pub node_id: String,
    pub field: MotionParameterField,
}

/// One declared parameter with its current (default) value.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MotionAssetParameter {
    pub id: String,
    pub label: String,
    #[serde(rename = "type")]
    pub kind: MotionParameterKind,
    pub target: MotionParameterTarget,
    pub default: MotionParameterValue,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub choices: Option<Vec<String>>,
}

/// The reusable body of a scene or fragment asset.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MotionAssetPayload {
    #[serde(default)]
    pub assets: Vec<MotionAsset>,
    #[serde(default)]
    pub nodes: Vec<MotionNode>,
    #[serde(default)]
    pub animations: Vec<MotionAnimation>,
}

/// One animation preset for the `motion` kind.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MotionPreset {
    pub property: MotionProperty,
    pub keyframes: Vec<MotionKeyframe>,
    #[serde(default)]
    pub easing: MotionEasing,
}

/// The still-image reference for the `image` kind.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MotionImage {
    /// Relative to the library entry; absolute paths and URLs are refused.
    pub file: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
}

/// The whole asset document.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MotionAssetDocument {
    pub format: String,
    pub schema_version: u32,
    pub asset_id: String,
    pub name: String,
    pub category: MotionAssetCategory,
    #[serde(default)]
    pub category_source: MotionAssetCategorySource,
    #[serde(default)]
    pub tags: Vec<String>,
    pub kind: MotionAssetKind,
    pub canvas: MotionCanvas,
    pub duration: RationalTime,
    #[serde(default)]
    pub parameters: Vec<MotionAssetParameter>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scene: Option<MotionAssetPayload>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preset: Option<MotionPreset>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<MotionImage>,
    /// Library-relative preview image, when one has been produced.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preview: Option<String>,
}

/// Why an asset document could not be read at all.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MotionAssetError {
    /// Larger than [`MAX_MOTION_ASSET_BYTES`].
    TooLarge,
    /// Not a JSON object this version understands.
    InvalidJson,
    /// `format` is not [`MOTION_ASSET_FORMAT`].
    UnsupportedFormat,
    /// `schema_version` is not [`MOTION_ASSET_SCHEMA_VERSION`].
    UnsupportedSchemaVersion,
    /// The document parsed but does not validate; call [`validate_motion_asset`].
    Invalid,
    /// The kind cannot become a scene (a preset or an image).
    NotInstantiable(&'static str),
    /// An override names a parameter the asset does not declare.
    UnknownParameter(String),
    /// An override value does not fit its parameter.
    InvalidValue(String),
}

impl MotionAssetError {
    /// The stable wire spelling.
    pub fn code(&self) -> &'static str {
        match self {
            Self::TooLarge => "MOTION_ASSET_TOO_LARGE",
            Self::InvalidJson => "MOTION_ASSET_INVALID_JSON",
            Self::UnsupportedFormat => "MOTION_ASSET_UNSUPPORTED_FORMAT",
            Self::UnsupportedSchemaVersion => "MOTION_ASSET_UNSUPPORTED_SCHEMA_VERSION",
            Self::Invalid => "MOTION_ASSET_INVALID",
            Self::NotInstantiable(_) => "MOTION_ASSET_NOT_INSTANTIABLE",
            Self::UnknownParameter(_) => "MOTION_ASSET_UNKNOWN_PARAMETER",
            Self::InvalidValue(_) => "MOTION_ASSET_INVALID_VALUE",
        }
    }
}

impl fmt::Display for MotionAssetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLarge => formatter.write_str("the asset is larger than this version accepts"),
            Self::InvalidJson => {
                formatter.write_str("the asset is not a JSON object this version understands")
            }
            Self::UnsupportedFormat => {
                formatter.write_str("the asset's format field is not a MotionAsset format")
            }
            Self::UnsupportedSchemaVersion => {
                formatter.write_str("the asset's schema_version is not one this version reads")
            }
            Self::Invalid => formatter.write_str("the asset does not validate"),
            Self::NotInstantiable(reason) => {
                write!(formatter, "the asset cannot become a scene: {reason}")
            }
            Self::UnknownParameter(id) => {
                write!(formatter, "the value names an unknown parameter: {id}")
            }
            Self::InvalidValue(message) => {
                write!(formatter, "the value does not fit its parameter: {message}")
            }
        }
    }
}

impl std::error::Error for MotionAssetError {}

/// Reads an asset document. Bounds and the format identity are checked before the
/// caller validates the body, exactly like a scene.
pub fn parse_motion_asset(bytes: &[u8]) -> Result<MotionAssetDocument, MotionAssetError> {
    if bytes.len() > MAX_MOTION_ASSET_BYTES {
        return Err(MotionAssetError::TooLarge);
    }
    let asset: MotionAssetDocument =
        serde_json::from_slice(bytes).map_err(|_| MotionAssetError::InvalidJson)?;
    if asset.format != MOTION_ASSET_FORMAT {
        return Err(MotionAssetError::UnsupportedFormat);
    }
    if asset.schema_version != MOTION_ASSET_SCHEMA_VERSION {
        return Err(MotionAssetError::UnsupportedSchemaVersion);
    }
    Ok(asset)
}

/// Validates the whole document. Returns every problem rather than the first.
pub fn validate_motion_asset(asset: &MotionAssetDocument) -> MotionValidation {
    let mut errors = Vec::new();

    if asset.format != MOTION_ASSET_FORMAT {
        errors.push(diagnostic(
            "MOTION_ASSET_UNSUPPORTED_FORMAT",
            "/format",
            format!("format must be {MOTION_ASSET_FORMAT}"),
        ));
    }
    if asset.schema_version != MOTION_ASSET_SCHEMA_VERSION {
        errors.push(diagnostic(
            "MOTION_ASSET_UNSUPPORTED_SCHEMA_VERSION",
            "/schema_version",
            format!("schema_version must be {MOTION_ASSET_SCHEMA_VERSION}"),
        ));
    }
    if !is_uuid_v4(&asset.asset_id) {
        errors.push(diagnostic(
            "MOTION_ASSET_INVALID_ID",
            "/asset_id",
            "asset_id must be a UUIDv4",
        ));
    }
    if asset.name.trim().is_empty() {
        errors.push(diagnostic(
            "MOTION_ASSET_INVALID_NAME",
            "/name",
            "name must not be empty",
        ));
    } else if asset.name.len() > MAX_MOTION_ASSET_NAME_BYTES {
        errors.push(diagnostic(
            "MOTION_ASSET_NAME_TOO_LARGE",
            "/name",
            format!("name may hold at most {MAX_MOTION_ASSET_NAME_BYTES} bytes"),
        ));
    }
    if asset.duration.is_negative() || asset.duration.is_zero() {
        errors.push(diagnostic(
            "MOTION_ASSET_INVALID_DURATION",
            "/duration",
            "duration must be greater than zero",
        ));
    }
    if asset.canvas.width == 0 || asset.canvas.height == 0 {
        errors.push(diagnostic(
            "MOTION_ASSET_INVALID_CANVAS",
            "/canvas",
            "canvas width and height must be greater than zero",
        ));
    }
    if let Some(preview) = &asset.preview
        && !is_safe_relative_path(preview)
    {
        errors.push(diagnostic(
            "MOTION_ASSET_INVALID_PREVIEW_PATH",
            "/preview",
            "preview must be a relative path inside the library",
        ));
    }
    validate_tags(asset, &mut errors);

    let mut node_kinds: Vec<(String, MotionNodeKind)> = Vec::new();
    match asset.kind {
        MotionAssetKind::Scene | MotionAssetKind::Fragment => {
            if asset.preset.is_some() || asset.image.is_some() {
                errors.push(diagnostic(
                    "MOTION_ASSET_UNEXPECTED_PAYLOAD",
                    "/preset",
                    "a scene or fragment carries nodes, not a preset or an image",
                ));
            }
            match &asset.scene {
                None => errors.push(diagnostic(
                    "MOTION_ASSET_MISSING_PAYLOAD",
                    "/scene",
                    "a scene or fragment needs a scene body",
                )),
                Some(payload) => {
                    if payload.nodes.is_empty() {
                        errors.push(diagnostic(
                            "MOTION_ASSET_EMPTY_SCENE",
                            "/scene/nodes",
                            "a scene or fragment needs at least one node",
                        ));
                    }
                    validate_payload(asset, payload, &mut errors);
                    collect_nodes(&payload.nodes, 0, &mut node_kinds);
                }
            }
        }
        MotionAssetKind::Motion => {
            if asset.scene.is_some() || asset.image.is_some() {
                errors.push(diagnostic(
                    "MOTION_ASSET_UNEXPECTED_PAYLOAD",
                    "/scene",
                    "a motion preset carries a preset, not a scene body or an image",
                ));
            }
            match &asset.preset {
                None => errors.push(diagnostic(
                    "MOTION_ASSET_MISSING_PAYLOAD",
                    "/preset",
                    "a motion asset needs a preset",
                )),
                Some(preset) => validate_preset(asset, preset, &mut errors),
            }
        }
        MotionAssetKind::Image => {
            if asset.preset.is_some() {
                errors.push(diagnostic(
                    "MOTION_ASSET_UNEXPECTED_PAYLOAD",
                    "/preset",
                    "an image asset carries a scene body and an image reference, not a preset",
                ));
            }
            match &asset.image {
                None => errors.push(diagnostic(
                    "MOTION_ASSET_MISSING_PAYLOAD",
                    "/image",
                    "an image asset needs an image reference",
                )),
                Some(image) => {
                    if !is_safe_relative_path(&image.file) {
                        errors.push(diagnostic(
                            "MOTION_ASSET_INVALID_IMAGE_PATH",
                            "/image/file",
                            "the image must be a relative path inside the library",
                        ));
                    }
                    if let Some(sha256) = &image.sha256
                        && !is_sha256(sha256)
                    {
                        errors.push(diagnostic(
                            "MOTION_ASSET_INVALID_DIGEST",
                            "/image/sha256",
                            "sha256 must be 64 lowercase hex characters",
                        ));
                    }
                }
            }
            match &asset.scene {
                None => errors.push(diagnostic(
                    "MOTION_ASSET_MISSING_PAYLOAD",
                    "/scene",
                    "an image asset needs a scene body with an image node",
                )),
                Some(payload) => {
                    if payload.nodes.is_empty() {
                        errors.push(diagnostic(
                            "MOTION_ASSET_EMPTY_SCENE",
                            "/scene/nodes",
                            "an image asset needs at least one node",
                        ));
                    }
                    validate_payload(asset, payload, &mut errors);
                    collect_nodes(&payload.nodes, 0, &mut node_kinds);
                    if !has_node_kind(&payload.nodes, MotionNodeKind::Image) {
                        errors.push(diagnostic(
                            "MOTION_ASSET_MISSING_IMAGE_NODE",
                            "/scene/nodes",
                            "an image asset needs at least one image node",
                        ));
                    }
                    if let Some(image) = &asset.image {
                        match payload
                            .assets
                            .iter()
                            .find(|scene_asset| scene_asset.asset_id == asset.asset_id)
                        {
                            None => errors.push(diagnostic(
                                "MOTION_ASSET_IMAGE_REFERENCE_MISSING",
                                "/scene/assets",
                                "the scene body must include the image asset_id",
                            )),
                            Some(scene_asset) if scene_asset.path != image.file => {
                                errors.push(diagnostic(
                                    "MOTION_ASSET_IMAGE_PATH_MISMATCH",
                                    "/image/file",
                                    "the image file must match the scene asset path",
                                ));
                            }
                            Some(_) => {}
                        }
                        if !has_image_node(&payload.nodes, &asset.asset_id) {
                            errors.push(diagnostic(
                                "MOTION_ASSET_IMAGE_NODE_MISMATCH",
                                "/scene/nodes",
                                "an image node must reference the image asset_id",
                            ));
                        }
                    }
                }
            }
        }
    }

    if asset.kind == MotionAssetKind::Motion && !asset.parameters.is_empty() {
        errors.push(diagnostic(
            "MOTION_ASSET_PARAMETERS_UNSUPPORTED",
            "/parameters",
            "a motion preset has no node to bind parameters to",
        ));
    }

    validate_parameters(asset, &node_kinds, &mut errors);

    MotionValidation {
        ok: errors.is_empty(),
        errors,
    }
}

fn validate_tags(asset: &MotionAssetDocument, errors: &mut Vec<MotionDiagnostic>) {
    if asset.tags.len() > MAX_MOTION_ASSET_TAGS {
        errors.push(diagnostic(
            "MOTION_ASSET_TOO_MANY_TAGS",
            "/tags",
            format!("an asset may carry at most {MAX_MOTION_ASSET_TAGS} tags"),
        ));
    }
    let mut seen: Vec<&str> = Vec::new();
    for (index, tag) in asset.tags.iter().enumerate() {
        let path = format!("/tags/{index}");
        if tag.trim().is_empty() {
            errors.push(diagnostic(
                "MOTION_ASSET_INVALID_TAG",
                &path,
                "a tag must not be empty",
            ));
        } else if tag.len() > MAX_MOTION_ASSET_TAG_BYTES {
            errors.push(diagnostic(
                "MOTION_ASSET_TAG_TOO_LARGE",
                &path,
                format!("a tag may hold at most {MAX_MOTION_ASSET_TAG_BYTES} bytes"),
            ));
        } else if seen.contains(&tag.as_str()) {
            errors.push(diagnostic(
                "MOTION_ASSET_DUPLICATE_TAG",
                &path,
                "tags must be unique",
            ));
        } else {
            seen.push(tag);
        }
    }
}

fn validate_payload(
    asset: &MotionAssetDocument,
    payload: &MotionAssetPayload,
    errors: &mut Vec<MotionDiagnostic>,
) {
    // The scene validator owns node, asset and animation rules. Feed it a scene
    // envelope built from fields this validator already checked, then keep only
    // the body diagnostics so the paths point into `/scene`.
    let scene = MotionScene {
        format: MOTION_SCENE_FORMAT.to_owned(),
        schema_version: MOTION_SCENE_SCHEMA_VERSION,
        scene_id: asset.asset_id.clone(),
        name: asset.name.clone(),
        duration: asset.duration,
        canvas: asset.canvas,
        assets: payload.assets.clone(),
        nodes: payload.nodes.clone(),
        animations: payload.animations.clone(),
    };
    for mut error in validate_scene(&scene).errors {
        if error.path.starts_with("/nodes")
            || error.path.starts_with("/animations")
            || error.path.starts_with("/assets")
        {
            error.path = format!("/scene{}", error.path);
            errors.push(error);
        }
    }
    if payload.nodes.len() > MAX_MOTION_NODES {
        errors.push(diagnostic(
            "MOTION_ASSET_TOO_MANY_NODES",
            "/scene/nodes",
            format!("a scene body may hold at most {MAX_MOTION_NODES} nodes"),
        ));
    }
    if payload.animations.len() > MAX_MOTION_ANIMATIONS {
        errors.push(diagnostic(
            "MOTION_ASSET_TOO_MANY_ANIMATIONS",
            "/scene/animations",
            format!("a scene body may hold at most {MAX_MOTION_ANIMATIONS} animations"),
        ));
    }
}

fn validate_preset(
    asset: &MotionAssetDocument,
    preset: &MotionPreset,
    errors: &mut Vec<MotionDiagnostic>,
) {
    if preset.keyframes.is_empty() {
        errors.push(diagnostic(
            "MOTION_ASSET_EMPTY_PRESET",
            "/preset/keyframes",
            "a preset needs at least one keyframe",
        ));
    }
    if preset.keyframes.len() > MAX_MOTION_KEYFRAMES {
        errors.push(diagnostic(
            "MOTION_ASSET_TOO_MANY_KEYFRAMES",
            "/preset/keyframes",
            format!("a preset may hold at most {MAX_MOTION_KEYFRAMES} keyframes"),
        ));
    }
    let mut previous: Option<RationalTime> = None;
    for (index, keyframe) in preset.keyframes.iter().enumerate() {
        let path = format!("/preset/keyframes/{index}");
        if keyframe.at.is_negative() {
            errors.push(diagnostic(
                "MOTION_ASSET_KEYFRAME_OUT_OF_RANGE",
                &path,
                "a keyframe time must not be negative",
            ));
        } else if keyframe.at > asset.duration {
            errors.push(diagnostic(
                "MOTION_ASSET_KEYFRAME_OUT_OF_RANGE",
                &path,
                "a keyframe time must not be past the asset's duration",
            ));
        }
        if let Some(previous) = previous
            && keyframe.at < previous
        {
            errors.push(diagnostic(
                "MOTION_ASSET_KEYFRAMES_UNSORTED",
                &path,
                "keyframes must be in ascending time order",
            ));
        }
        previous = Some(keyframe.at);
        if preset.property == MotionProperty::Opacity && !(0.0..=1.0).contains(&keyframe.value) {
            errors.push(diagnostic(
                "MOTION_ASSET_INVALID_OPACITY",
                &path,
                "an opacity keyframe must be between 0 and 1",
            ));
        }
    }
}

fn validate_parameters(
    asset: &MotionAssetDocument,
    nodes: &[(String, MotionNodeKind)],
    errors: &mut Vec<MotionDiagnostic>,
) {
    if asset.parameters.len() > MAX_MOTION_ASSET_PARAMETERS {
        errors.push(diagnostic(
            "MOTION_ASSET_TOO_MANY_PARAMETERS",
            "/parameters",
            format!("an asset may declare at most {MAX_MOTION_ASSET_PARAMETERS} parameters"),
        ));
    }
    let mut ids: Vec<&str> = Vec::new();
    let mut targets: Vec<(String, MotionParameterField)> = Vec::new();
    for (index, parameter) in asset.parameters.iter().enumerate() {
        let path = format!("/parameters/{index}");
        if parameter.id.trim().is_empty() {
            errors.push(diagnostic(
                "MOTION_ASSET_INVALID_PARAMETER_ID",
                format!("{path}/id"),
                "a parameter id must not be empty",
            ));
        } else if ids.contains(&parameter.id.as_str()) {
            errors.push(diagnostic(
                "MOTION_ASSET_DUPLICATE_PARAMETER_ID",
                format!("{path}/id"),
                "parameter ids must be unique",
            ));
        } else {
            ids.push(parameter.id.as_str());
        }
        if parameter.label.trim().is_empty()
            || parameter.label.len() > MAX_MOTION_ASSET_PARAMETER_LABEL_BYTES
        {
            errors.push(diagnostic(
                "MOTION_ASSET_INVALID_PARAMETER_LABEL",
                format!("{path}/label"),
                format!(
                    "a label must not be empty and may hold at most \
                     {MAX_MOTION_ASSET_PARAMETER_LABEL_BYTES} bytes"
                ),
            ));
        }
        if parameter.target.node_id.trim().is_empty() {
            errors.push(diagnostic(
                "MOTION_ASSET_INVALID_PARAMETER_TARGET",
                format!("{path}/target/node_id"),
                "target.node_id must not be empty",
            ));
        }
        let target_key = (parameter.target.node_id.clone(), parameter.target.field);
        if targets.contains(&target_key) {
            errors.push(diagnostic(
                "MOTION_ASSET_DUPLICATE_PARAMETER_TARGET",
                format!("{path}/target"),
                "two parameters must not write the same node field",
            ));
        } else {
            targets.push(target_key);
        }

        if matches!(
            asset.kind,
            MotionAssetKind::Scene | MotionAssetKind::Fragment | MotionAssetKind::Image
        ) {
            match nodes.iter().find(|(id, _)| id == &parameter.target.node_id) {
                None => errors.push(diagnostic(
                    "MOTION_ASSET_UNKNOWN_PARAMETER_NODE",
                    format!("{path}/target/node_id"),
                    "target.node_id must name a node in the scene body",
                )),
                Some((_, kind)) => {
                    if !kind_supports_field(*kind, parameter.target.field) {
                        errors.push(diagnostic(
                            "MOTION_ASSET_PARAMETER_FIELD_UNSUPPORTED",
                            format!("{path}/target/field"),
                            format!(
                                "a {} node has no {}",
                                kind_name(*kind),
                                parameter.target.field
                            ),
                        ));
                    }
                }
            }
        }

        if let Some(problem) = value_problem(parameter, &parameter.default) {
            errors.push(diagnostic(
                "MOTION_ASSET_PARAMETER_INVALID",
                format!("{path}/default"),
                problem,
            ));
        }
        if parameter.min.is_some_and(|value| !value.is_finite())
            || parameter.max.is_some_and(|value| !value.is_finite())
        {
            errors.push(diagnostic(
                "MOTION_ASSET_PARAMETER_INVALID_RANGE",
                format!("{path}/min"),
                "min and max must be finite numbers",
            ));
        } else if let (Some(min), Some(max)) = (parameter.min, parameter.max)
            && min > max
        {
            errors.push(diagnostic(
                "MOTION_ASSET_PARAMETER_INVALID_RANGE",
                format!("{path}/min"),
                "min must not exceed max",
            ));
        }
        if parameter.kind == MotionParameterKind::Choice {
            match &parameter.choices {
                None => errors.push(diagnostic(
                    "MOTION_ASSET_PARAMETER_INVALID_CHOICES",
                    format!("{path}/choices"),
                    "a choice parameter needs at least one choice",
                )),
                Some(choices) => {
                    if choices.is_empty() || choices.len() > MAX_MOTION_ASSET_CHOICES {
                        errors.push(diagnostic(
                            "MOTION_ASSET_PARAMETER_INVALID_CHOICES",
                            format!("{path}/choices"),
                            format!(
                                "a choice parameter needs 1..={MAX_MOTION_ASSET_CHOICES} choices"
                            ),
                        ));
                    } else if let MotionParameterValue::Text(value) = &parameter.default
                        && !choices.contains(value)
                    {
                        errors.push(diagnostic(
                            "MOTION_ASSET_PARAMETER_CHOICE_UNKNOWN",
                            format!("{path}/default"),
                            "the default must be one of the choices",
                        ));
                    }
                }
            }
        }

        if let Some((_, kind)) = nodes.iter().find(|(id, _)| id == &parameter.target.node_id) {
            let field = parameter.target.field;
            if kind_supports_field(*kind, field) && !default_matches(asset, parameter) {
                errors.push(diagnostic(
                    "MOTION_ASSET_PARAMETER_DEFAULT_MISMATCH",
                    format!("{path}/default"),
                    "the default must equal the targeted node field",
                ));
            }
        }
    }
}

fn kind_supports_field(kind: MotionNodeKind, field: MotionParameterField) -> bool {
    match field {
        MotionParameterField::Text | MotionParameterField::FontSize => kind == MotionNodeKind::Text,
        MotionParameterField::SizeWidth | MotionParameterField::SizeHeight => {
            matches!(
                kind,
                MotionNodeKind::Rectangle | MotionNodeKind::Ellipse | MotionNodeKind::Image
            )
        }
        MotionParameterField::AssetId => kind == MotionNodeKind::Image,
        MotionParameterField::Fill => matches!(
            kind,
            MotionNodeKind::Text
                | MotionNodeKind::Rectangle
                | MotionNodeKind::Ellipse
                | MotionNodeKind::Arrow
        ),
        MotionParameterField::Opacity | MotionParameterField::RotationDegrees => true,
    }
}

fn kind_name(kind: MotionNodeKind) -> &'static str {
    match kind {
        MotionNodeKind::Group => "group",
        MotionNodeKind::Text => "text",
        MotionNodeKind::Rectangle => "rectangle",
        MotionNodeKind::Ellipse => "ellipse",
        MotionNodeKind::Arrow => "arrow",
        MotionNodeKind::Image => "image",
    }
}

/// The reason a value cannot be written through this parameter, if any.
fn value_problem(parameter: &MotionAssetParameter, value: &MotionParameterValue) -> Option<String> {
    match (parameter.kind, parameter.target.field, value) {
        (
            MotionParameterKind::Text,
            MotionParameterField::Text,
            MotionParameterValue::Text(text),
        ) => {
            if text.len() > MAX_MOTION_TEXT_BYTES {
                return Some(format!(
                    "text may hold at most {MAX_MOTION_TEXT_BYTES} bytes"
                ));
            }
        }
        (
            MotionParameterKind::Text,
            MotionParameterField::AssetId,
            MotionParameterValue::Text(text),
        ) => {
            if text.trim().is_empty() {
                return Some("asset_id must not be empty".to_owned());
            }
        }
        (
            MotionParameterKind::Choice,
            MotionParameterField::Text,
            MotionParameterValue::Text(text),
        )
        | (
            MotionParameterKind::Choice,
            MotionParameterField::AssetId,
            MotionParameterValue::Text(text),
        ) => {
            match &parameter.choices {
                Some(choices) if choices.contains(text) => {}
                Some(_) => return Some("the value must be one of the choices".to_owned()),
                None => return Some("the parameter has no choices".to_owned()),
            }
            if text.len() > MAX_MOTION_TEXT_BYTES {
                return Some(format!(
                    "text may hold at most {MAX_MOTION_TEXT_BYTES} bytes"
                ));
            }
        }
        (
            MotionParameterKind::Color,
            MotionParameterField::Fill,
            MotionParameterValue::Color(_),
        ) => {}
        (
            MotionParameterKind::Number,
            MotionParameterField::FontSize,
            MotionParameterValue::Number(number),
        )
        | (
            MotionParameterKind::Number,
            MotionParameterField::SizeWidth,
            MotionParameterValue::Number(number),
        )
        | (
            MotionParameterKind::Number,
            MotionParameterField::SizeHeight,
            MotionParameterValue::Number(number),
        ) => {
            if !number.is_finite() || *number <= 0.0 {
                return Some("the value must be a finite number greater than zero".to_owned());
            }
            if let Some(problem) = range_problem(parameter, *number) {
                return Some(problem);
            }
        }
        (
            MotionParameterKind::Number,
            MotionParameterField::Opacity,
            MotionParameterValue::Number(number),
        ) => {
            if !number.is_finite() || !(0.0..=1.0).contains(number) {
                return Some("opacity must be between 0 and 1".to_owned());
            }
            if let Some(problem) = range_problem(parameter, *number) {
                return Some(problem);
            }
        }
        (
            MotionParameterKind::Number,
            MotionParameterField::RotationDegrees,
            MotionParameterValue::Number(number),
        ) => {
            if !number.is_finite() {
                return Some("rotation must be a finite number".to_owned());
            }
            if let Some(problem) = range_problem(parameter, *number) {
                return Some(problem);
            }
        }
        _ => {
            return Some(format!(
                "a {} parameter cannot write {}",
                parameter.kind.as_str(),
                parameter.target.field
            ));
        }
    }
    None
}

fn range_problem(parameter: &MotionAssetParameter, number: f64) -> Option<String> {
    if let Some(min) = parameter.min
        && number < min
    {
        return Some(format!("the value must be at least {min}"));
    }
    if let Some(max) = parameter.max
        && number > max
    {
        return Some(format!("the value must be at most {max}"));
    }
    None
}

fn default_matches(asset: &MotionAssetDocument, parameter: &MotionAssetParameter) -> bool {
    let Some(payload) = asset.scene.as_ref() else {
        return true;
    };
    let Some(node) = find_node(&payload.nodes, &parameter.target.node_id) else {
        return true;
    };
    match field_value(node, parameter.target.field) {
        Some(value) => value == parameter.default,
        None => false,
    }
}

fn field_value(node: &MotionNode, field: MotionParameterField) -> Option<MotionParameterValue> {
    match field {
        MotionParameterField::Text => node.text.clone().map(MotionParameterValue::Text),
        MotionParameterField::FontSize => node.font_size.map(MotionParameterValue::Number),
        MotionParameterField::Fill => node.fill.map(MotionParameterValue::Color),
        MotionParameterField::SizeWidth => node
            .size
            .as_ref()
            .map(|size| MotionParameterValue::Number(size.width)),
        MotionParameterField::SizeHeight => node
            .size
            .as_ref()
            .map(|size| MotionParameterValue::Number(size.height)),
        MotionParameterField::Opacity => node.opacity.map(MotionParameterValue::Number),
        MotionParameterField::RotationDegrees => {
            node.rotation_degrees.map(MotionParameterValue::Number)
        }
        MotionParameterField::AssetId => node.asset_id.clone().map(MotionParameterValue::Text),
    }
}

fn find_node<'a>(nodes: &'a [MotionNode], node_id: &str) -> Option<&'a MotionNode> {
    for node in nodes {
        if node.node_id == node_id {
            return Some(node);
        }
        if let Some(children) = &node.children
            && let Some(found) = find_node(children, node_id)
        {
            return Some(found);
        }
    }
    None
}

fn find_node_mut<'a>(nodes: &'a mut [MotionNode], node_id: &str) -> Option<&'a mut MotionNode> {
    for node in nodes {
        if node.node_id == node_id {
            return Some(node);
        }
        if let Some(children) = node.children.as_mut()
            && let Some(found) = find_node_mut(children, node_id)
        {
            return Some(found);
        }
    }
    None
}

fn collect_nodes(nodes: &[MotionNode], depth: usize, out: &mut Vec<(String, MotionNodeKind)>) {
    if depth >= MAX_MOTION_NODE_DEPTH {
        return;
    }
    for node in nodes {
        out.push((node.node_id.clone(), node.kind));
        if let Some(children) = &node.children {
            collect_nodes(children, depth + 1, out);
        }
    }
}

/// Instantiates a scene or fragment into a fully validated MotionScene.
///
/// `overrides` addresses declared parameter ids; omitted parameters keep the value
/// already in the payload (which validation proved equal to their default). A
/// fragment's node ids and animation targets are remapped with the asset id so a
/// later merge cannot collide with the host scene.
pub fn instantiate_motion_asset(
    asset: &MotionAssetDocument,
    overrides: &BTreeMap<String, MotionParameterValue>,
    scene_id: &str,
    name: &str,
) -> Result<MotionScene, MotionAssetError> {
    if !validate_motion_asset(asset).ok {
        return Err(MotionAssetError::Invalid);
    }
    let payload = match asset.kind {
        MotionAssetKind::Motion => {
            return Err(MotionAssetError::NotInstantiable(
                "a motion preset is applied to a node, not turned into a scene",
            ));
        }
        MotionAssetKind::Scene | MotionAssetKind::Fragment | MotionAssetKind::Image => {
            asset.scene.clone().ok_or(MotionAssetError::Invalid)?
        }
    };
    let mut nodes = payload.nodes;
    let mut animations = payload.animations;
    for (id, value) in overrides {
        let Some(parameter) = asset
            .parameters
            .iter()
            .find(|parameter| &parameter.id == id)
        else {
            return Err(MotionAssetError::UnknownParameter(id.clone()));
        };
        if let Some(problem) = value_problem(parameter, value) {
            return Err(MotionAssetError::InvalidValue(format!("{id}: {problem}")));
        }
        let node = find_node_mut(&mut nodes, &parameter.target.node_id).ok_or_else(|| {
            MotionAssetError::InvalidValue(format!("{id}: the target node is missing"))
        })?;
        apply_value(node, parameter.target.field, value)
            .map_err(|problem| MotionAssetError::InvalidValue(format!("{id}: {problem}")))?;
    }
    if asset.kind == MotionAssetKind::Fragment {
        remap_ids(&asset.asset_id, &mut nodes, &mut animations);
    }
    let scene = MotionScene {
        format: MOTION_SCENE_FORMAT.to_owned(),
        schema_version: MOTION_SCENE_SCHEMA_VERSION,
        scene_id: scene_id.to_owned(),
        name: if name.trim().is_empty() {
            asset.name.clone()
        } else {
            name.to_owned()
        },
        duration: asset.duration,
        canvas: asset.canvas,
        assets: payload.assets,
        nodes,
        animations,
    };
    if !validate_scene(&scene).ok {
        return Err(MotionAssetError::Invalid);
    }
    Ok(scene)
}

fn apply_value(
    node: &mut MotionNode,
    field: MotionParameterField,
    value: &MotionParameterValue,
) -> Result<(), &'static str> {
    match (field, value) {
        (MotionParameterField::Text, MotionParameterValue::Text(text)) => {
            node.text = Some(text.clone());
        }
        (MotionParameterField::FontSize, MotionParameterValue::Number(number)) => {
            node.font_size = Some(*number);
        }
        (MotionParameterField::Fill, MotionParameterValue::Color(color)) => {
            node.fill = Some(*color);
        }
        (MotionParameterField::SizeWidth, MotionParameterValue::Number(width)) => {
            let size = node.size.as_mut().ok_or("the node has no size")?;
            size.width = *width;
        }
        (MotionParameterField::SizeHeight, MotionParameterValue::Number(height)) => {
            let size = node.size.as_mut().ok_or("the node has no size")?;
            size.height = *height;
        }
        (MotionParameterField::Opacity, MotionParameterValue::Number(number)) => {
            node.opacity = Some(*number);
        }
        (MotionParameterField::RotationDegrees, MotionParameterValue::Number(number)) => {
            node.rotation_degrees = Some(*number);
        }
        (MotionParameterField::AssetId, MotionParameterValue::Text(asset_id)) => {
            node.asset_id = Some(asset_id.clone());
        }
        _ => return Err("the value kind does not fit the field"),
    }
    Ok(())
}

fn remap_ids(prefix: &str, nodes: &mut [MotionNode], animations: &mut [MotionAnimation]) {
    remap_node_ids(prefix, nodes);
    for animation in animations.iter_mut() {
        animation.node_id = format!("{prefix}:{}", animation.node_id);
    }
}

fn remap_node_ids(prefix: &str, nodes: &mut [MotionNode]) {
    for node in nodes.iter_mut() {
        node.node_id = format!("{prefix}:{}", node.node_id);
        if let Some(children) = node.children.as_mut() {
            remap_node_ids(prefix, children);
        }
    }
}

fn has_node_kind(nodes: &[MotionNode], kind: MotionNodeKind) -> bool {
    nodes.iter().any(|node| {
        node.kind == kind
            || node
                .children
                .as_ref()
                .is_some_and(|children| has_node_kind(children, kind))
    })
}

fn has_image_node(nodes: &[MotionNode], asset_id: &str) -> bool {
    nodes.iter().any(|node| {
        (node.kind == MotionNodeKind::Image && node.asset_id.as_deref() == Some(asset_id))
            || node
                .children
                .as_ref()
                .is_some_and(|children| has_image_node(children, asset_id))
    })
}

/// One asset placed while composing a scene.
pub struct MotionScenePlacement<'a> {
    pub asset: &'a MotionAssetDocument,
    pub values: BTreeMap<String, MotionParameterValue>,
    pub offset_x: f64,
    pub offset_y: f64,
}

/// Appends one asset to a scene, remapping its ids with `prefix` and shifting its
/// nodes by the placement offset.
///
/// The result is a new validated scene; the input is untouched. A whole-scene
/// template, a fragment and an image all take this path, so composition cannot
/// disagree with single-asset instantiation.
pub fn append_asset_to_scene(
    scene: &MotionScene,
    asset: &MotionAssetDocument,
    values: &BTreeMap<String, MotionParameterValue>,
    prefix: &str,
    offset_x: f64,
    offset_y: f64,
) -> Result<MotionScene, MotionAssetError> {
    if prefix.trim().is_empty() {
        return Err(MotionAssetError::InvalidValue(
            "a composition prefix must not be empty".to_owned(),
        ));
    }
    let mut addition = instantiate_motion_asset(asset, values, &scene.scene_id, &scene.name)?;
    remap_ids(prefix, &mut addition.nodes, &mut addition.animations);
    remap_scene_asset_ids(prefix, &mut addition.assets, &mut addition.nodes);
    offset_nodes(&mut addition.nodes, offset_x, offset_y);
    let mut merged = scene.clone();
    merged.nodes.extend(addition.nodes);
    merged.animations.extend(addition.animations);
    merged.assets.extend(addition.assets);
    if !validate_scene(&merged).ok {
        return Err(MotionAssetError::Invalid);
    }
    Ok(merged)
}

/// Composes validated assets into one scene.
///
/// Each placement is appended with its index as the remap prefix, so placing the
/// same asset twice cannot collide, and every node position is shifted by the
/// placement offset. The result is validated as an ordinary MotionScene.
pub fn compose_motion_scene(
    scene_id: &str,
    name: &str,
    canvas: MotionCanvas,
    duration: RationalTime,
    placements: &[MotionScenePlacement<'_>],
) -> Result<MotionScene, MotionAssetError> {
    let mut scene = MotionScene {
        format: MOTION_SCENE_FORMAT.to_owned(),
        schema_version: MOTION_SCENE_SCHEMA_VERSION,
        scene_id: scene_id.to_owned(),
        name: name.to_owned(),
        duration,
        canvas,
        assets: Vec::new(),
        nodes: Vec::new(),
        animations: Vec::new(),
    };
    for (index, placement) in placements.iter().enumerate() {
        scene = append_asset_to_scene(
            &scene,
            placement.asset,
            &placement.values,
            &format!("{index}"),
            placement.offset_x,
            placement.offset_y,
        )?;
    }
    Ok(scene)
}

fn remap_scene_asset_ids(prefix: &str, assets: &mut [MotionAsset], nodes: &mut [MotionNode]) {
    for asset in assets.iter_mut() {
        asset.asset_id = format!("{prefix}:{}", asset.asset_id);
    }
    remap_node_asset_ids(prefix, nodes);
}

fn remap_node_asset_ids(prefix: &str, nodes: &mut [MotionNode]) {
    for node in nodes.iter_mut() {
        if let Some(asset_id) = node.asset_id.as_mut() {
            *asset_id = format!("{prefix}:{asset_id}");
        }
        if let Some(children) = node.children.as_mut() {
            remap_node_asset_ids(prefix, children);
        }
    }
}

fn offset_nodes(nodes: &mut [MotionNode], offset_x: f64, offset_y: f64) {
    for node in nodes.iter_mut() {
        if let Some(position) = node.position.as_mut() {
            position.x += offset_x;
            position.y += offset_y;
        }
        if let Some(end) = node.end.as_mut() {
            end.x += offset_x;
            end.y += offset_y;
        }
        if let Some(children) = node.children.as_mut() {
            offset_nodes(children, offset_x, offset_y);
        }
    }
}

fn diagnostic(code: &str, path: impl Into<String>, message: impl Into<String>) -> MotionDiagnostic {
    MotionDiagnostic::new(code, path, message)
}

/// A relative, library-local path: no absolute root, no parent segment, no
/// platform separator and no scheme.
pub fn is_safe_relative_path(path: &str) -> bool {
    if path.is_empty()
        || path.starts_with('/')
        || path.contains('\\')
        || path.contains('\u{0000}')
        || path.contains("://")
    {
        return false;
    }
    if path.as_bytes().get(1) == Some(&b':') {
        return false;
    }
    for segment in path.split('/') {
        if segment.is_empty() || segment == "." || segment == ".." {
            return false;
        }
    }
    true
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn asset_value() -> serde_json::Value {
        json!({
            "format": MOTION_ASSET_FORMAT,
            "schema_version": MOTION_ASSET_SCHEMA_VERSION,
            "asset_id": "6f1c2f5e-3a4b-4c5d-8e9f-0a1b2c3d4e5f",
            "name": "Explainer title",
            "category": "text",
            "category_source": "explicit",
            "tags": ["title"],
            "kind": "scene",
            "canvas": {"width": 1920, "height": 1080},
            "duration": {"numerator": 4, "denominator": 1},
            "parameters": [
                {
                    "id": "title",
                    "label": "Title",
                    "type": "text",
                    "target": {"node_id": "title", "field": "text"},
                    "default": "Hello"
                },
                {
                    "id": "accent",
                    "label": "Accent",
                    "type": "color",
                    "target": {"node_id": "title", "field": "fill"},
                    "default": {"red": 220, "green": 220, "blue": 220, "alpha": 255}
                }
            ],
            "scene": {
                "assets": [],
                "nodes": [
                    {
                        "node_id": "background",
                        "kind": "rectangle",
                        "position": {"x": 0, "y": 0},
                        "size": {"width": 1920, "height": 1080},
                        "fill": {"red": 20, "green": 20, "blue": 24, "alpha": 255}
                    },
                    {
                        "node_id": "title",
                        "kind": "text",
                        "text": "Hello",
                        "position": {"x": 80, "y": 120},
                        "font_size": 64,
                        "fill": {"red": 220, "green": 220, "blue": 220, "alpha": 255}
                    }
                ],
                "animations": []
            }
        })
    }

    fn parse(value: serde_json::Value) -> MotionAssetDocument {
        parse_motion_asset(&serde_json::to_vec(&value).unwrap()).unwrap()
    }

    #[test]
    fn a_scene_asset_round_trips_and_validates() {
        let asset = parse(asset_value());
        assert_eq!(asset.kind, MotionAssetKind::Scene);
        assert_eq!(asset.parameters.len(), 2);
        let validation = validate_motion_asset(&asset);
        assert!(validation.ok, "{:?}", validation.errors);
        let encoded = serde_json::to_vec(&asset).unwrap();
        let decoded = parse_motion_asset(&encoded).unwrap();
        assert_eq!(decoded, asset);
    }

    #[test]
    fn the_format_identity_is_refused_before_the_body() {
        let mut value = asset_value();
        value["format"] = json!("something-else");
        let bytes = serde_json::to_vec(&value).unwrap();
        assert_eq!(
            parse_motion_asset(&bytes).unwrap_err(),
            MotionAssetError::UnsupportedFormat
        );

        let mut value = asset_value();
        value["schema_version"] = json!(2);
        let bytes = serde_json::to_vec(&value).unwrap();
        assert_eq!(
            parse_motion_asset(&bytes).unwrap_err(),
            MotionAssetError::UnsupportedSchemaVersion
        );
    }

    #[test]
    fn an_unknown_field_is_refused_rather_than_ignored() {
        let mut value = asset_value();
        value["script"] = json!("while (true) {}");
        let bytes = serde_json::to_vec(&value).unwrap();
        assert_eq!(
            parse_motion_asset(&bytes).unwrap_err(),
            MotionAssetError::InvalidJson
        );
    }

    #[test]
    fn a_parameter_must_target_a_real_node_and_supported_field() {
        let mut value = asset_value();
        value["parameters"][0]["target"]["node_id"] = json!("missing");
        let validation = validate_motion_asset(&parse(value));
        assert!(!validation.ok);
        assert!(
            validation
                .errors
                .iter()
                .any(|error| error.code == "MOTION_ASSET_UNKNOWN_PARAMETER_NODE")
        );

        let mut value = asset_value();
        value["parameters"][1]["target"]["field"] = json!("font_size");
        let validation = validate_motion_asset(&parse(value));
        assert!(
            validation
                .errors
                .iter()
                .any(|error| error.code == "MOTION_ASSET_PARAMETER_INVALID")
        );
    }

    #[test]
    fn a_parameter_default_must_equal_the_target_field() {
        let mut value = asset_value();
        value["parameters"][0]["default"] = json!("Other");
        let validation = validate_motion_asset(&parse(value));
        assert!(
            validation
                .errors
                .iter()
                .any(|error| error.code == "MOTION_ASSET_PARAMETER_DEFAULT_MISMATCH")
        );
    }

    #[test]
    fn two_parameters_may_not_write_the_same_field() {
        let mut value = asset_value();
        value["parameters"][1]["target"] = json!({"node_id": "title", "field": "text"});
        value["parameters"][1]["type"] = json!("text");
        value["parameters"][1]["default"] = json!("Hello");
        let validation = validate_motion_asset(&parse(value));
        assert!(
            validation
                .errors
                .iter()
                .any(|error| error.code == "MOTION_ASSET_DUPLICATE_PARAMETER_TARGET")
        );
    }

    #[test]
    fn a_choice_parameter_checks_its_list() {
        let mut value = asset_value();
        value["parameters"][0] = json!({
            "id": "title",
            "label": "Title",
            "type": "choice",
            "target": {"node_id": "title", "field": "text"},
            "default": "Hello",
            "choices": ["Hello", "Goodbye"]
        });
        assert!(validate_motion_asset(&parse(value)).ok);

        let mut value = asset_value();
        value["parameters"][0] = json!({
            "id": "title",
            "label": "Title",
            "type": "choice",
            "target": {"node_id": "title", "field": "text"},
            "default": "Hello",
            "choices": ["Other"]
        });
        let validation = validate_motion_asset(&parse(value));
        assert!(
            validation
                .errors
                .iter()
                .any(|error| error.code == "MOTION_ASSET_PARAMETER_CHOICE_UNKNOWN")
        );
    }

    #[test]
    fn kind_payloads_are_exclusive() {
        let mut value = asset_value();
        value["kind"] = json!("motion");
        let validation = validate_motion_asset(&parse(value));
        assert!(
            validation
                .errors
                .iter()
                .any(|error| error.code == "MOTION_ASSET_UNEXPECTED_PAYLOAD")
        );
        assert!(
            validation
                .errors
                .iter()
                .any(|error| error.code == "MOTION_ASSET_MISSING_PAYLOAD")
        );
    }

    #[test]
    fn a_motion_preset_validates_against_the_duration() {
        let mut value = asset_value();
        value["kind"] = json!("motion");
        value["scene"] = serde_json::Value::Null;
        value["parameters"] = json!([]);
        value["preset"] = json!({
            "property": "opacity",
            "keyframes": [
                {"at": {"numerator": 0, "denominator": 1}, "value": 0},
                {"at": {"numerator": 5, "denominator": 1}, "value": 1}
            ]
        });
        let validation = validate_motion_asset(&parse(value));
        assert!(
            validation
                .errors
                .iter()
                .any(|error| error.code == "MOTION_ASSET_KEYFRAME_OUT_OF_RANGE")
        );
    }

    fn image_asset_value(file: &str) -> serde_json::Value {
        let asset_id = "5d0c3b2a-1f4e-4a6b-9c8d-7e6f5a4b3c2d";
        json!({
            "format": MOTION_ASSET_FORMAT,
            "schema_version": MOTION_ASSET_SCHEMA_VERSION,
            "asset_id": asset_id,
            "name": "Mountain background",
            "category": "background",
            "category_source": "explicit",
            "tags": [],
            "kind": "image",
            "canvas": {"width": 1920, "height": 1080},
            "duration": {"numerator": 4, "denominator": 1},
            "parameters": [
                {
                    "id": "opacity",
                    "label": "Opacity",
                    "type": "number",
                    "target": {"node_id": "image", "field": "opacity"},
                    "default": 1,
                    "min": 0,
                    "max": 1
                },
                {
                    "id": "width",
                    "label": "Width",
                    "type": "number",
                    "target": {"node_id": "image", "field": "size_width"},
                    "default": 1920,
                    "min": 1
                }
            ],
            "image": {"file": file},
            "scene": {
                "assets": [{"asset_id": asset_id, "path": file}],
                "nodes": [
                    {
                        "node_id": "image",
                        "kind": "image",
                        "asset_id": asset_id,
                        "position": {"x": 0, "y": 0},
                        "size": {"width": 1920, "height": 1080},
                        "opacity": 1
                    }
                ],
                "animations": []
            }
        })
    }

    #[test]
    fn an_image_asset_carries_its_scene_body_and_reference() {
        assert!(validate_motion_asset(&parse(image_asset_value("images/pic.png"))).ok);

        let mut value = image_asset_value("../outside.png");
        value["scene"]["assets"][0]["path"] = json!("../outside.png");
        let validation = validate_motion_asset(&parse(value));
        assert!(
            validation
                .errors
                .iter()
                .any(|error| error.code == "MOTION_ASSET_INVALID_IMAGE_PATH")
        );

        let mut value = image_asset_value("images/pic.png");
        value["scene"]["assets"] = json!([]);
        let validation = validate_motion_asset(&parse(value));
        assert!(
            validation
                .errors
                .iter()
                .any(|error| error.code == "MOTION_ASSET_IMAGE_REFERENCE_MISSING")
        );
    }

    #[test]
    fn an_image_asset_instantiates_and_keeps_its_adjustable_fields() {
        let asset = parse(image_asset_value("images/pic.png"));
        let mut overrides = BTreeMap::new();
        overrides.insert("opacity".to_owned(), MotionParameterValue::Number(0.5));
        overrides.insert("width".to_owned(), MotionParameterValue::Number(960.0));
        let scene = instantiate_motion_asset(
            &asset,
            &overrides,
            "7a2b3c4d-5e6f-4a1b-8c9d-0e1f2a3b4c5d",
            "Imported",
        )
        .unwrap();
        let node = scene
            .nodes
            .iter()
            .find(|node| node.kind == MotionNodeKind::Image)
            .unwrap();
        assert_eq!(node.opacity, Some(0.5));
        assert_eq!(node.size.unwrap().width, 960.0);
        assert_eq!(scene.assets[0].path, "images/pic.png");
    }

    #[test]
    fn classification_is_explicit_then_tags_then_name_then_default() {
        let tags = vec!["vehicle".to_owned()];
        let classified = classify_motion_asset("Car", &tags, MotionAssetKind::Scene, None);
        assert_eq!(classified.category, MotionAssetCategory::Vehicle);
        assert_eq!(classified.source, MotionAssetCategorySource::Tags);

        let classified =
            classify_motion_asset("Mountain background", &[], MotionAssetKind::Scene, None);
        assert_eq!(classified.category, MotionAssetCategory::Background);
        assert_eq!(classified.source, MotionAssetCategorySource::Filename);

        let classified = classify_motion_asset("Untitled", &[], MotionAssetKind::Scene, None);
        assert_eq!(classified.category, MotionAssetCategory::Object);
        assert_eq!(classified.source, MotionAssetCategorySource::Default);

        let classified = classify_motion_asset(
            "Untitled",
            &[],
            MotionAssetKind::Scene,
            Some(MotionAssetCategory::People),
        );
        assert_eq!(classified.category, MotionAssetCategory::People);
        assert_eq!(classified.source, MotionAssetCategorySource::Explicit);
    }

    #[test]
    fn classification_matches_vietnamese_tokens() {
        for (name, expected) in [
            ("Nền thành phố", MotionAssetCategory::Background),
            ("Con chó", MotionAssetCategory::Animal),
            ("Xe hơi", MotionAssetCategory::Vehicle),
            ("Biểu đồ doanh thu", MotionAssetCategory::Chart),
        ] {
            let classified = classify_motion_asset(name, &[], MotionAssetKind::Scene, None);
            assert_eq!(classified.category, expected, "{name}");
        }
    }

    #[test]
    fn instantiation_applies_text_and_colour_and_keeps_defaults() {
        let asset = parse(asset_value());
        let mut overrides = BTreeMap::new();
        overrides.insert(
            "title".to_owned(),
            MotionParameterValue::Text("Changed".to_owned()),
        );
        overrides.insert(
            "accent".to_owned(),
            MotionParameterValue::Color(MotionColor {
                red: 10,
                green: 20,
                blue: 30,
                alpha: 255,
            }),
        );
        let scene = instantiate_motion_asset(
            &asset,
            &overrides,
            "7a2b3c4d-5e6f-4a1b-8c9d-0e1f2a3b4c5d",
            "Instantiated",
        )
        .unwrap();
        assert_eq!(scene.name, "Instantiated");
        assert_eq!(scene.nodes.len(), 2);
        let title = scene
            .nodes
            .iter()
            .find(|node| node.node_id == "title")
            .unwrap();
        assert_eq!(title.text.as_deref(), Some("Changed"));
        assert_eq!(title.fill.unwrap().red, 10);
    }

    #[test]
    fn instantiation_refuses_unknown_ids_and_bad_values() {
        let asset = parse(asset_value());
        let mut overrides = BTreeMap::new();
        overrides.insert("missing".to_owned(), MotionParameterValue::Number(1.0));
        assert_eq!(
            instantiate_motion_asset(
                &asset,
                &overrides,
                "7a2b3c4d-5e6f-4a1b-8c9d-0e1f2a3b4c5d",
                "x"
            )
            .unwrap_err(),
            MotionAssetError::UnknownParameter("missing".to_owned())
        );

        let mut overrides = BTreeMap::new();
        overrides.insert(
            "accent".to_owned(),
            MotionParameterValue::Text("nope".to_owned()),
        );
        assert!(matches!(
            instantiate_motion_asset(
                &asset,
                &overrides,
                "7a2b3c4d-5e6f-4a1b-8c9d-0e1f2a3b4c5d",
                "x"
            )
            .unwrap_err(),
            MotionAssetError::InvalidValue(_)
        ));
    }

    #[test]
    fn a_fragment_remaps_ids_so_two_instances_cannot_collide() {
        let mut value = asset_value();
        value["kind"] = json!("fragment");
        value["scene"]["animations"] = json!([
            {
                "node_id": "title",
                "property": "opacity",
                "keyframes": [
                    {"at": {"numerator": 0, "denominator": 1}, "value": 0},
                    {"at": {"numerator": 1, "denominator": 2}, "value": 1}
                ]
            }
        ]);
        let asset = parse(value);
        let scene = instantiate_motion_asset(
            &asset,
            &BTreeMap::new(),
            "7a2b3c4d-5e6f-4a1b-8c9d-0e1f2a3b4c5d",
            "Fragment",
        )
        .unwrap();
        assert!(
            scene
                .nodes
                .iter()
                .all(|node| node.node_id.starts_with(&format!("{}:", asset.asset_id)))
        );
        assert_eq!(
            scene.animations[0].node_id,
            format!("{}:title", asset.asset_id)
        );
    }

    #[test]
    fn a_motion_preset_is_not_turned_into_a_scene() {
        let mut value = asset_value();
        value["kind"] = json!("motion");
        value["scene"] = serde_json::Value::Null;
        value["parameters"] = json!([]);
        value["preset"] = json!({
            "property": "opacity",
            "keyframes": [{"at": {"numerator": 0, "denominator": 1}, "value": 1}]
        });
        let asset = parse(value);
        assert!(matches!(
            instantiate_motion_asset(
                &asset,
                &BTreeMap::new(),
                "7a2b3c4d-5e6f-4a1b-8c9d-0e1f2a3b4c5d",
                "x"
            )
            .unwrap_err(),
            MotionAssetError::NotInstantiable(_)
        ));
    }

    #[test]
    fn composition_remaps_ids_and_offsets_each_placement() {
        let text = parse(asset_value());
        let image = parse(image_asset_value("images/pic.png"));
        let scene = compose_motion_scene(
            "7a2b3c4d-5e6f-4a1b-8c9d-0e1f2a3b4c5d",
            "Composed",
            MotionCanvas {
                width: 1920,
                height: 1080,
            },
            RationalTime::new(5, 1).unwrap(),
            &[
                MotionScenePlacement {
                    asset: &text,
                    values: BTreeMap::new(),
                    offset_x: 0.0,
                    offset_y: 0.0,
                },
                MotionScenePlacement {
                    asset: &image,
                    values: BTreeMap::new(),
                    offset_x: 100.0,
                    offset_y: 50.0,
                },
            ],
        )
        .unwrap();
        assert!(validate_scene(&scene).ok);
        let ids: Vec<&str> = scene
            .nodes
            .iter()
            .map(|node| node.node_id.as_str())
            .collect();
        let unique: std::collections::BTreeSet<&str> = ids.iter().copied().collect();
        assert_eq!(unique.len(), ids.len());
        let image_node = scene
            .nodes
            .iter()
            .find(|node| node.kind == MotionNodeKind::Image)
            .unwrap();
        assert_eq!(image_node.position.unwrap().x, 100.0);
        assert_eq!(image_node.position.unwrap().y, 50.0);
        assert!(
            image_node
                .asset_id
                .as_deref()
                .is_some_and(|id| id.starts_with("1:"))
        );
        assert!(
            scene
                .assets
                .iter()
                .all(|asset| asset.asset_id.starts_with('1'))
        );
    }

    #[test]
    fn a_size_parameter_can_resize_an_object() {
        let mut value = asset_value();
        value["parameters"].as_array_mut().unwrap().push(json!({
            "id": "width",
            "label": "Width",
            "type": "number",
            "target": {"node_id": "background", "field": "size_width"},
            "default": 1920
        }));
        let asset = parse(value);
        let mut overrides = BTreeMap::new();
        overrides.insert("width".to_owned(), MotionParameterValue::Number(1280.0));
        let scene = instantiate_motion_asset(
            &asset,
            &overrides,
            "7a2b3c4d-5e6f-4a1b-8c9d-0e1f2a3b4c5d",
            "x",
        )
        .unwrap();
        let background = scene
            .nodes
            .iter()
            .find(|node| node.node_id == "background")
            .unwrap();
        assert_eq!(background.size.unwrap().width, 1280.0);
    }

    #[test]
    fn safe_relative_paths_refuse_escape_attempts() {
        for path in [
            "/etc/passwd",
            "../outside.png",
            "a/../../b.png",
            "a\\b.png",
            "https://example.com/a.png",
            "C:/a.png",
            "",
        ] {
            assert!(!is_safe_relative_path(path), "{path}");
        }
        assert!(is_safe_relative_path("images/pic.png"));
    }
}
