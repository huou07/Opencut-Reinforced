//! The declarative, non-executable MotionScene source format (ADR 0007).
//!
//! A `.ormotion.json` scene is **data, not a program**. It is versioned, seekable
//! and inspectable; it has no expression language, no script, no environment or
//! filesystem reference, and no provider call. Everything here is a pure parse,
//! validation or summary: nothing in this crate renders, evaluates pixels, spawns a
//! process, or touches a project.
//!
//! ADR 0007 deferred the exact serialized field names and bounds to the phase that
//! implements them. This crate is that implementation, and the decisions it makes
//! are recorded in the ADR: one flat node shape per kind with `deny_unknown_fields`
//! everywhere, exact rational times from `or_core`'s time model, and diagnostics
//! with stable codes and JSON paths.

mod code;

pub use code::{
    CodeLanguage, CodeSceneError, CodeSceneOptions, CodeTheme, CodeToken, CodeTokenKind,
    MAX_CODE_COLUMNS, MAX_CODE_LINE_BYTES, MAX_CODE_LINES, MAX_CODE_SOURCE_BYTES,
    scene_from_source, tokenize_line,
};

use or_core::RationalTime;
use serde::{Deserialize, Serialize};
use std::fmt;

/// The `format` value every scene carries.
pub const MOTION_SCENE_FORMAT: &str = "opencut-reinforced-motion-scene";
/// The schema version this crate reads and writes.
pub const MOTION_SCENE_SCHEMA_VERSION: u32 = 1;
/// The largest scene this accepts. A scene is authored text, not media, so this is
/// generous for a hand-written file and still bounded.
pub const MAX_MOTION_SCENE_BYTES: usize = 4 * 1024 * 1024;
/// The most nodes a scene may hold, counted through nested groups.
pub const MAX_MOTION_NODES: usize = 4096;
/// How deep groups may nest. A bounded depth keeps validation and later evaluation
/// from recursing without limit.
pub const MAX_MOTION_NODE_DEPTH: usize = 16;
/// The most animations a scene may hold.
pub const MAX_MOTION_ANIMATIONS: usize = 4096;
/// The most keyframes one animation may hold.
pub const MAX_MOTION_KEYFRAMES: usize = 4096;
/// The most bytes one text node's string may hold.
pub const MAX_MOTION_TEXT_BYTES: usize = 64 * 1024;

/// The whole scene document.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MotionScene {
    pub format: String,
    pub schema_version: u32,
    pub scene_id: String,
    pub name: String,
    pub duration: RationalTime,
    pub canvas: MotionCanvas,
    #[serde(default)]
    pub assets: Vec<MotionAsset>,
    #[serde(default)]
    pub nodes: Vec<MotionNode>,
    #[serde(default)]
    pub animations: Vec<MotionAnimation>,
}

/// The fixed drawing surface. Pixels, so a scene renders identically everywhere.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MotionCanvas {
    pub width: u32,
    pub height: u32,
}

/// A scene-local asset reference. It is an identity, never a URL: rendering must
/// not require the network, and a missing asset is a diagnostic rather than a
/// fetch.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MotionAsset {
    pub asset_id: String,
    /// The content digest the scene was authored against, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    /// A scene-local or project-relative path. Absolute URLs are refused by
    /// validation, because they would make rendering depend on a network.
    pub path: String,
}

/// A point in canvas pixels.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MotionPoint {
    pub x: f64,
    pub y: f64,
}

/// A size in canvas pixels.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MotionSize {
    pub width: f64,
    pub height: f64,
}

/// A typed colour. Not a CSS string: there is no expression language here, and a
/// typed value is what a renderer can rely on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MotionColor {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    #[serde(default = "opaque_alpha")]
    pub alpha: u8,
}

fn opaque_alpha() -> u8 {
    255
}

/// The node kinds V1 supports. `Polyline` from ADR 0007 is renderer-gated and is
/// deliberately absent rather than accepted and ignored.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MotionNodeKind {
    Group,
    Text,
    Rectangle,
    Ellipse,
    Arrow,
    Image,
}

/// One highlighted run of a text node's string, by byte offset. Spans are typed
/// values, never markup: there is no inline syntax to parse and nothing to execute.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MotionTextSpan {
    /// Inclusive byte offset into the node's text.
    pub start: usize,
    /// Exclusive byte offset into the node's text.
    pub end: usize,
    pub color: MotionColor,
}

/// One node. The shape is flat and strict: `deny_unknown_fields` refuses anything
/// this version does not know, and validation states which fields each kind
/// requires, so a scene cannot smuggle behaviour in through an unused field.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MotionNode {
    pub node_id: String,
    pub kind: MotionNodeKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<MotionNode>>,
    /// Text content. It is literal: it is never parsed as a template, an
    /// expression or a format string.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<MotionPoint>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<MotionSize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end: Option<MotionPoint>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill: Option<MotionColor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke: Option<MotionColor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke_width: Option<f64>,
    /// A text node's type size in canvas pixels.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_size: Option<f64>,
    /// An image node's asset identity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asset_id: Option<String>,
    /// 0..=1. Absent means fully opaque.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opacity: Option<f64>,
    /// Degrees, counter-clockwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rotation_degrees: Option<f64>,
    /// Highlighted runs of `text`, ordered and non-overlapping. Absent means the
    /// node's `fill` colours the whole string.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spans: Option<Vec<MotionTextSpan>>,
}

/// The animatable properties V1 supports. A locked set: there is no expression
/// language and no arbitrary property path.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MotionProperty {
    TranslationX,
    TranslationY,
    Scale,
    Rotation,
    Opacity,
    Width,
    Height,
}

/// The easing set. Small and locked, per ADR 0007.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MotionEasing {
    #[default]
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
}

/// One keyframe: an exact time and a value.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MotionKeyframe {
    pub at: RationalTime,
    pub value: f64,
}

/// One animated property of one node.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MotionAnimation {
    pub node_id: String,
    pub property: MotionProperty,
    pub keyframes: Vec<MotionKeyframe>,
    #[serde(default)]
    pub easing: MotionEasing,
}

/// Why a scene could not be read at all.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MotionSceneError {
    /// The file is larger than [`MAX_MOTION_SCENE_BYTES`].
    TooLarge,
    /// The bytes are not a JSON object this version understands.
    InvalidJson,
    /// The `format` value is not [`MOTION_SCENE_FORMAT`].
    UnsupportedFormat,
    /// The `schema_version` is not [`MOTION_SCENE_SCHEMA_VERSION`]. A future
    /// version is refused rather than guessed at, exactly as the project format
    /// refuses one.
    UnsupportedSchemaVersion,
}

impl MotionSceneError {
    /// The stable wire spelling.
    pub fn code(self) -> &'static str {
        match self {
            Self::TooLarge => "MOTION_SCENE_TOO_LARGE",
            Self::InvalidJson => "MOTION_INVALID_JSON",
            Self::UnsupportedFormat => "MOTION_UNSUPPORTED_FORMAT",
            Self::UnsupportedSchemaVersion => "MOTION_UNSUPPORTED_SCHEMA_VERSION",
        }
    }
}

impl fmt::Display for MotionSceneError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::TooLarge => "the scene is larger than this version accepts",
            Self::InvalidJson => "the scene is not a JSON object this version understands",
            Self::UnsupportedFormat => "the scene's format field is not a MotionScene format",
            Self::UnsupportedSchemaVersion => {
                "the scene's schema_version is not one this version reads"
            }
        };
        formatter.write_str(message)
    }
}

/// One machine-actionable problem, with a stable code and the JSON path it is at.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MotionDiagnostic {
    pub code: String,
    pub path: String,
    pub message: String,
}

impl MotionDiagnostic {
    fn new(code: &str, path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.to_owned(),
            path: path.into(),
            message: message.into(),
        }
    }
}

/// The validator's answer, in the shape ADR 0007 documents.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MotionValidation {
    pub ok: bool,
    pub errors: Vec<MotionDiagnostic>,
}

/// What a scene contains, for a client that wants a summary without rendering.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MotionSceneInspection {
    pub scene_id: String,
    pub name: String,
    pub schema_version: u32,
    pub duration_milliseconds: i64,
    pub canvas_width: u32,
    pub canvas_height: u32,
    pub node_count: usize,
    pub asset_count: usize,
    pub animation_count: usize,
    pub keyframe_count: usize,
    /// The node kinds present, in first-appearance order.
    pub node_kinds: Vec<String>,
    pub validation: MotionValidation,
}

/// Parses a scene, refusing an unreadable size, malformed JSON, an unknown format
/// and a future schema version before anything looks at the contents.
pub fn parse_motion_scene(bytes: &[u8]) -> Result<MotionScene, MotionSceneError> {
    if bytes.len() > MAX_MOTION_SCENE_BYTES {
        return Err(MotionSceneError::TooLarge);
    }
    let scene: MotionScene =
        serde_json::from_slice(bytes).map_err(|_| MotionSceneError::InvalidJson)?;
    if scene.format != MOTION_SCENE_FORMAT {
        return Err(MotionSceneError::UnsupportedFormat);
    }
    if scene.schema_version != MOTION_SCENE_SCHEMA_VERSION {
        return Err(MotionSceneError::UnsupportedSchemaVersion);
    }
    Ok(scene)
}

/// Validates a parsed scene, returning every problem it finds rather than the
/// first: a client fixing a scene wants the whole list.
pub fn validate_scene(scene: &MotionScene) -> MotionValidation {
    let mut errors = Vec::new();

    if scene.format != MOTION_SCENE_FORMAT {
        errors.push(MotionDiagnostic::new(
            "MOTION_UNSUPPORTED_FORMAT",
            "/format",
            format!("format must be {MOTION_SCENE_FORMAT}"),
        ));
    }
    if scene.schema_version != MOTION_SCENE_SCHEMA_VERSION {
        errors.push(MotionDiagnostic::new(
            "MOTION_UNSUPPORTED_SCHEMA_VERSION",
            "/schema_version",
            format!("schema_version must be {MOTION_SCENE_SCHEMA_VERSION}"),
        ));
    }
    if !is_uuid_v4(&scene.scene_id) {
        errors.push(MotionDiagnostic::new(
            "MOTION_INVALID_SCENE_ID",
            "/scene_id",
            "scene_id must be a UUIDv4",
        ));
    }
    if scene.name.trim().is_empty() {
        errors.push(MotionDiagnostic::new(
            "MOTION_INVALID_NAME",
            "/name",
            "name must not be empty",
        ));
    }
    if scene.duration.is_negative() || scene.duration.is_zero() {
        errors.push(MotionDiagnostic::new(
            "MOTION_INVALID_DURATION",
            "/duration",
            "duration must be greater than zero",
        ));
    }
    if scene.canvas.width == 0 || scene.canvas.height == 0 {
        errors.push(MotionDiagnostic::new(
            "MOTION_INVALID_CANVAS",
            "/canvas",
            "canvas width and height must be greater than zero",
        ));
    }

    let mut asset_ids = Vec::with_capacity(scene.assets.len());
    for (index, asset) in scene.assets.iter().enumerate() {
        let path = format!("/assets/{index}");
        if asset.asset_id.trim().is_empty() {
            errors.push(MotionDiagnostic::new(
                "MOTION_INVALID_ASSET_ID",
                format!("{path}/asset_id"),
                "asset_id must not be empty",
            ));
        } else if asset_ids.contains(&asset.asset_id) {
            errors.push(MotionDiagnostic::new(
                "MOTION_DUPLICATE_ASSET_ID",
                format!("{path}/asset_id"),
                "asset_id must be unique",
            ));
        } else {
            asset_ids.push(asset.asset_id.clone());
        }
        if asset.path.trim().is_empty() {
            errors.push(MotionDiagnostic::new(
                "MOTION_INVALID_ASSET_PATH",
                format!("{path}/path"),
                "path must not be empty",
            ));
        } else if is_remote_reference(&asset.path) {
            // Rendering must never require the network; a remote reference is
            // refused rather than fetched.
            errors.push(MotionDiagnostic::new(
                "MOTION_REMOTE_ASSET_REFUSED",
                format!("{path}/path"),
                "assets must be scene-local or project-relative, never a remote URL",
            ));
        }
    }

    let mut node_ids: Vec<String> = Vec::new();
    let mut node_count = 0;
    walk_nodes(
        &scene.nodes,
        "/nodes",
        0,
        &asset_ids,
        &mut node_ids,
        &mut node_count,
        &mut errors,
    );
    if node_count > MAX_MOTION_NODES {
        errors.push(MotionDiagnostic::new(
            "MOTION_TOO_MANY_NODES",
            "/nodes",
            format!("a scene may hold at most {MAX_MOTION_NODES} nodes"),
        ));
    }
    if scene.animations.len() > MAX_MOTION_ANIMATIONS {
        errors.push(MotionDiagnostic::new(
            "MOTION_TOO_MANY_ANIMATIONS",
            "/animations",
            format!("a scene may hold at most {MAX_MOTION_ANIMATIONS} animations"),
        ));
    }

    let mut animated: Vec<(String, MotionProperty)> = Vec::new();
    for (index, animation) in scene.animations.iter().enumerate() {
        let path = format!("/animations/{index}");
        if !node_ids.contains(&animation.node_id) {
            errors.push(MotionDiagnostic::new(
                "MOTION_ANIMATION_UNKNOWN_NODE",
                format!("{path}/node_id"),
                "node_id must name a node in this scene",
            ));
        }
        if animated.iter().any(|(node_id, property)| {
            node_id == &animation.node_id && *property == animation.property
        }) {
            errors.push(MotionDiagnostic::new(
                "MOTION_ANIMATION_DUPLICATE_PROPERTY",
                format!("{path}/property"),
                "one node may animate a property once",
            ));
        } else {
            animated.push((animation.node_id.clone(), animation.property));
        }
        if animation.keyframes.is_empty() {
            errors.push(MotionDiagnostic::new(
                "MOTION_ANIMATION_EMPTY",
                format!("{path}/keyframes"),
                "an animation needs at least one keyframe",
            ));
        }
        if animation.keyframes.len() > MAX_MOTION_KEYFRAMES {
            errors.push(MotionDiagnostic::new(
                "MOTION_TOO_MANY_KEYFRAMES",
                format!("{path}/keyframes"),
                format!("an animation may hold at most {MAX_MOTION_KEYFRAMES} keyframes"),
            ));
        }
        let mut previous: Option<RationalTime> = None;
        for (keyframe_index, keyframe) in animation.keyframes.iter().enumerate() {
            let keyframe_path = format!("{path}/keyframes/{keyframe_index}");
            if keyframe.at.is_negative() {
                errors.push(MotionDiagnostic::new(
                    "MOTION_KEYFRAME_OUT_OF_RANGE",
                    format!("{keyframe_path}/at"),
                    "a keyframe time must not be negative",
                ));
            } else if keyframe.at > scene.duration {
                errors.push(MotionDiagnostic::new(
                    "MOTION_KEYFRAME_OUT_OF_RANGE",
                    format!("{keyframe_path}/at"),
                    "a keyframe time must not be past the scene's duration",
                ));
            }
            if let Some(previous) = previous
                && keyframe.at < previous
            {
                errors.push(MotionDiagnostic::new(
                    "MOTION_KEYFRAMES_UNSORTED",
                    format!("{keyframe_path}/at"),
                    "keyframes must be in ascending time order",
                ));
            }
            previous = Some(keyframe.at);
            if animation.property == MotionProperty::Opacity
                && !(0.0..=1.0).contains(&keyframe.value)
            {
                errors.push(MotionDiagnostic::new(
                    "MOTION_INVALID_OPACITY",
                    format!("{keyframe_path}/value"),
                    "an opacity keyframe must be between 0 and 1",
                ));
            }
        }
    }

    MotionValidation {
        ok: errors.is_empty(),
        errors,
    }
}

/// Summarises a scene and validates it in the same pass, so an inspection can
/// never report a scene as usable when validation refuses it.
pub fn inspect_motion_scene(scene: &MotionScene) -> MotionSceneInspection {
    let mut node_count = 0;
    let mut node_kinds: Vec<String> = Vec::new();
    // Depth-first, document order: a group's children are seen where the document
    // puts them, before the next sibling of the group. An explicit stack rather than
    // recursion, because a parsed scene can nest deeper than validation allows and
    // inspection must not overflow the stack while reporting that.
    let mut pending: Vec<&MotionNode> = scene.nodes.iter().rev().collect();
    while let Some(node) = pending.pop() {
        node_count += 1;
        let name = node_kind_name(node.kind).to_owned();
        if !node_kinds.contains(&name) {
            node_kinds.push(name);
        }
        if let Some(children) = &node.children {
            pending.extend(children.iter().rev());
        }
    }
    let keyframe_count = scene
        .animations
        .iter()
        .map(|animation| animation.keyframes.len())
        .sum();
    MotionSceneInspection {
        scene_id: scene.scene_id.clone(),
        name: scene.name.clone(),
        schema_version: scene.schema_version,
        duration_milliseconds: duration_milliseconds(scene.duration),
        canvas_width: scene.canvas.width,
        canvas_height: scene.canvas.height,
        node_count,
        asset_count: scene.assets.len(),
        animation_count: scene.animations.len(),
        keyframe_count,
        node_kinds,
        validation: validate_scene(scene),
    }
}

/// A scene's duration in whole milliseconds, truncated toward zero.
pub fn duration_milliseconds(duration: RationalTime) -> i64 {
    let numerator = duration.numerator() as i128 * 1000;
    let denominator = duration.denominator() as i128;
    (numerator / denominator) as i64
}

fn node_kind_name(kind: MotionNodeKind) -> &'static str {
    match kind {
        MotionNodeKind::Group => "group",
        MotionNodeKind::Text => "text",
        MotionNodeKind::Rectangle => "rectangle",
        MotionNodeKind::Ellipse => "ellipse",
        MotionNodeKind::Arrow => "arrow",
        MotionNodeKind::Image => "image",
    }
}

#[allow(clippy::too_many_arguments)]
fn walk_nodes(
    nodes: &[MotionNode],
    path: &str,
    depth: usize,
    asset_ids: &[String],
    node_ids: &mut Vec<String>,
    node_count: &mut usize,
    errors: &mut Vec<MotionDiagnostic>,
) {
    if depth >= MAX_MOTION_NODE_DEPTH {
        errors.push(MotionDiagnostic::new(
            "MOTION_DEPTH_EXCEEDED",
            path,
            format!("groups may nest at most {MAX_MOTION_NODE_DEPTH} deep"),
        ));
        return;
    }
    for (index, node) in nodes.iter().enumerate() {
        let node_path = format!("{path}/{index}");
        *node_count += 1;
        if node.node_id.trim().is_empty() {
            errors.push(MotionDiagnostic::new(
                "MOTION_INVALID_NODE_ID",
                format!("{node_path}/node_id"),
                "node_id must not be empty",
            ));
        } else if node_ids.contains(&node.node_id) {
            errors.push(MotionDiagnostic::new(
                "MOTION_DUPLICATE_NODE_ID",
                format!("{node_path}/node_id"),
                "node_id must be unique across the whole scene, including nested groups",
            ));
        } else {
            node_ids.push(node.node_id.clone());
        }

        match node.kind {
            MotionNodeKind::Group => {
                if node.children.is_none() {
                    errors.push(MotionDiagnostic::new(
                        "MOTION_MISSING_CHILDREN",
                        format!("{node_path}/children"),
                        "a group node needs children",
                    ));
                }
            }
            MotionNodeKind::Text => {
                match &node.text {
                    None => errors.push(MotionDiagnostic::new(
                        "MOTION_MISSING_TEXT",
                        format!("{node_path}/text"),
                        "a text node needs text",
                    )),
                    Some(text) if text.len() > MAX_MOTION_TEXT_BYTES => {
                        errors.push(MotionDiagnostic::new(
                            "MOTION_TEXT_TOO_LARGE",
                            format!("{node_path}/text"),
                            format!("text may hold at most {MAX_MOTION_TEXT_BYTES} bytes"),
                        ));
                    }
                    Some(_) => {}
                }
                if let Some(spans) = &node.spans {
                    if node.text.is_none() {
                        errors.push(MotionDiagnostic::new(
                            "MOTION_SPANS_ON_NON_TEXT",
                            format!("{node_path}/spans"),
                            "only a text node can carry highlighted spans",
                        ));
                    }
                    let text_len = node.text.as_ref().map(String::len).unwrap_or(0);
                    let mut previous_end = 0;
                    for (span_index, span) in spans.iter().enumerate() {
                        let span_path = format!("{node_path}/spans/{span_index}");
                        if span.start >= span.end || span.end > text_len {
                            errors.push(MotionDiagnostic::new(
                                "MOTION_INVALID_SPAN_RANGE",
                                format!("{span_path}/end"),
                                "a span must cover at least one byte and stay inside the text",
                            ));
                        }
                        if span.start < previous_end {
                            errors.push(MotionDiagnostic::new(
                                "MOTION_SPANS_UNSORTED",
                                format!("{span_path}/start"),
                                "spans must be ordered and must not overlap",
                            ));
                        }
                        previous_end = previous_end.max(span.end);
                    }
                } else if node.text.is_none() && node.kind == MotionNodeKind::Text {
                    // No spans is fine; a text node without text is already refused
                    // above.
                }
                if node.font_size.is_some_and(|size| size <= 0.0) {
                    errors.push(MotionDiagnostic::new(
                        "MOTION_INVALID_FONT_SIZE",
                        format!("{node_path}/font_size"),
                        "font_size must be greater than zero",
                    ));
                }
            }
            MotionNodeKind::Rectangle | MotionNodeKind::Ellipse => {
                if let Some(size) = node.size
                    && (size.width <= 0.0 || size.height <= 0.0)
                {
                    errors.push(MotionDiagnostic::new(
                        "MOTION_INVALID_GEOMETRY",
                        format!("{node_path}/size"),
                        "width and height must be greater than zero",
                    ));
                }
                if node.size.is_none() {
                    errors.push(MotionDiagnostic::new(
                        "MOTION_MISSING_SIZE",
                        format!("{node_path}/size"),
                        "a rectangle or ellipse node needs a size",
                    ));
                }
            }
            MotionNodeKind::Arrow => {
                if node.position.is_none() || node.end.is_none() {
                    errors.push(MotionDiagnostic::new(
                        "MOTION_MISSING_ENDPOINTS",
                        format!("{node_path}/end"),
                        "an arrow node needs a position and an end",
                    ));
                }
            }
            MotionNodeKind::Image => match &node.asset_id {
                None => errors.push(MotionDiagnostic::new(
                    "MOTION_MISSING_ASSET",
                    format!("{node_path}/asset_id"),
                    "an image node needs an asset_id",
                )),
                Some(asset_id) if !asset_ids.contains(asset_id) => {
                    errors.push(MotionDiagnostic::new(
                        "MOTION_UNKNOWN_ASSET",
                        format!("{node_path}/asset_id"),
                        "asset_id must name an asset in this scene",
                    ));
                }
                Some(_) => {}
            },
        }

        if node
            .opacity
            .is_some_and(|opacity| !(0.0..=1.0).contains(&opacity))
        {
            errors.push(MotionDiagnostic::new(
                "MOTION_INVALID_OPACITY",
                format!("{node_path}/opacity"),
                "opacity must be between 0 and 1",
            ));
        }
        if node.stroke_width.is_some_and(|width| width < 0.0) {
            errors.push(MotionDiagnostic::new(
                "MOTION_INVALID_STROKE",
                format!("{node_path}/stroke_width"),
                "stroke_width must not be negative",
            ));
        }

        if let Some(children) = &node.children {
            walk_nodes(
                children,
                &format!("{node_path}/children"),
                depth + 1,
                asset_ids,
                node_ids,
                node_count,
                errors,
            );
        }
    }
}

fn is_uuid_v4(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 36 {
        return false;
    }
    for (index, byte) in bytes.iter().enumerate() {
        match index {
            8 | 13 | 18 | 23 => {
                if *byte != b'-' {
                    return false;
                }
            }
            _ => {
                if !byte.is_ascii_hexdigit() {
                    return false;
                }
            }
        }
    }
    // Version 4 and the RFC 4122 variant.
    bytes[14] == b'4' && matches!(bytes[19], b'8' | b'9' | b'a' | b'b' | b'A' | b'B')
}

fn is_remote_reference(path: &str) -> bool {
    let lowered = path.trim().to_ascii_lowercase();
    lowered.starts_with("http://")
        || lowered.starts_with("https://")
        || lowered.starts_with("//")
        || lowered.starts_with("ftp://")
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCENE_ID: &str = "6f1c2f5e-3a4b-4c5d-8e9f-0a1b2c3d4e5f";

    fn scene() -> MotionScene {
        MotionScene {
            format: MOTION_SCENE_FORMAT.to_owned(),
            schema_version: MOTION_SCENE_SCHEMA_VERSION,
            scene_id: SCENE_ID.to_owned(),
            name: "Explainer".to_owned(),
            duration: RationalTime::new(4, 1).unwrap(),
            canvas: MotionCanvas {
                width: 1920,
                height: 1080,
            },
            assets: vec![MotionAsset {
                asset_id: "diagram".to_owned(),
                sha256: Some("a".repeat(64)),
                path: "assets/diagram.png".to_owned(),
            }],
            nodes: vec![
                MotionNode {
                    node_id: "code".to_owned(),
                    kind: MotionNodeKind::Text,
                    children: None,
                    text: Some("fn main() {}".to_owned()),
                    position: Some(MotionPoint { x: 40.0, y: 60.0 }),
                    size: None,
                    end: None,
                    fill: Some(MotionColor {
                        red: 255,
                        green: 255,
                        blue: 255,
                        alpha: 255,
                    }),
                    stroke: None,
                    stroke_width: None,
                    font_size: Some(28.0),
                    asset_id: None,
                    opacity: None,
                    rotation_degrees: None,
                    spans: None,
                },
                MotionNode {
                    node_id: "callout".to_owned(),
                    kind: MotionNodeKind::Group,
                    children: Some(vec![MotionNode {
                        node_id: "arrow".to_owned(),
                        kind: MotionNodeKind::Arrow,
                        children: None,
                        text: None,
                        position: Some(MotionPoint { x: 200.0, y: 80.0 }),
                        size: None,
                        end: Some(MotionPoint { x: 320.0, y: 140.0 }),
                        fill: None,
                        stroke: Some(MotionColor {
                            red: 255,
                            green: 200,
                            blue: 0,
                            alpha: 255,
                        }),
                        stroke_width: Some(3.0),
                        font_size: None,
                        asset_id: None,
                        opacity: Some(1.0),
                        rotation_degrees: None,
                        spans: None,
                    }]),
                    text: None,
                    position: None,
                    size: None,
                    end: None,
                    fill: None,
                    stroke: None,
                    stroke_width: None,
                    font_size: None,
                    asset_id: None,
                    opacity: None,
                    rotation_degrees: None,
                    spans: None,
                },
                MotionNode {
                    node_id: "diagram".to_owned(),
                    kind: MotionNodeKind::Image,
                    children: None,
                    text: None,
                    position: Some(MotionPoint { x: 900.0, y: 400.0 }),
                    size: Some(MotionSize {
                        width: 640.0,
                        height: 360.0,
                    }),
                    end: None,
                    fill: None,
                    stroke: None,
                    stroke_width: None,
                    font_size: None,
                    asset_id: Some("diagram".to_owned()),
                    opacity: Some(0.9),
                    rotation_degrees: None,
                    spans: None,
                },
            ],
            animations: vec![MotionAnimation {
                node_id: "code".to_owned(),
                property: MotionProperty::Opacity,
                keyframes: vec![
                    MotionKeyframe {
                        at: RationalTime::new(0, 1).unwrap(),
                        value: 0.0,
                    },
                    MotionKeyframe {
                        at: RationalTime::new(1, 1).unwrap(),
                        value: 1.0,
                    },
                ],
                easing: MotionEasing::EaseOut,
            }],
        }
    }

    #[test]
    fn a_valid_scene_round_trips_through_json_unchanged() {
        let original = scene();
        let bytes = serde_json::to_vec(&original).unwrap();
        let parsed = parse_motion_scene(&bytes).unwrap();
        assert_eq!(parsed, original);
        assert!(validate_scene(&parsed).ok);
    }

    #[test]
    fn an_unknown_field_is_refused_rather_than_ignored() {
        // Every struct denies unknown fields, so a scene cannot carry behaviour
        // this version does not implement.
        let mut value = serde_json::to_value(scene()).unwrap();
        value["nodes"][0]["script"] = serde_json::json!("while(true){}");
        let error = parse_motion_scene(&serde_json::to_vec(&value).unwrap()).unwrap_err();
        assert_eq!(error, MotionSceneError::InvalidJson);
        assert_eq!(error.code(), "MOTION_INVALID_JSON");
    }

    #[test]
    fn text_is_literal_and_never_a_template_or_expression() {
        // The format has no expression language: a string that looks like one is
        // text, preserved byte for byte, and nothing evaluates it.
        let mut value = serde_json::to_value(scene()).unwrap();
        let payload = "$(rm -rf /) {{ 1 + 1 }} ${HOME}";
        value["nodes"][0]["text"] = serde_json::json!(payload);
        let parsed = parse_motion_scene(&serde_json::to_vec(&value).unwrap()).unwrap();
        assert_eq!(parsed.nodes[0].text.as_deref(), Some(payload));
        assert!(validate_scene(&parsed).ok);
    }

    #[test]
    fn a_future_schema_version_is_refused_rather_than_guessed_at() {
        let mut value = serde_json::to_value(scene()).unwrap();
        value["schema_version"] = serde_json::json!(2);
        assert_eq!(
            parse_motion_scene(&serde_json::to_vec(&value).unwrap()).unwrap_err(),
            MotionSceneError::UnsupportedSchemaVersion
        );
    }

    #[test]
    fn an_oversized_or_foreign_document_is_refused_before_its_contents_matter() {
        let oversized = vec![b' '; MAX_MOTION_SCENE_BYTES + 1];
        assert_eq!(
            parse_motion_scene(&oversized).unwrap_err(),
            MotionSceneError::TooLarge
        );
        let mut value = serde_json::to_value(scene()).unwrap();
        value["format"] = serde_json::json!("some-other-format");
        assert_eq!(
            parse_motion_scene(&serde_json::to_vec(&value).unwrap()).unwrap_err(),
            MotionSceneError::UnsupportedFormat
        );
    }

    #[test]
    fn a_remote_asset_is_refused_because_rendering_never_fetches() {
        let mut value = serde_json::to_value(scene()).unwrap();
        value["assets"][0]["path"] = serde_json::json!("https://example.invalid/diagram.png");
        let parsed = parse_motion_scene(&serde_json::to_vec(&value).unwrap()).unwrap();
        let validation = validate_scene(&parsed);
        assert!(!validation.ok);
        assert_eq!(validation.errors[0].code, "MOTION_REMOTE_ASSET_REFUSED");
        assert_eq!(validation.errors[0].path, "/assets/0/path");
    }

    #[test]
    fn diagnostics_name_the_exact_path_of_every_problem() {
        let mut value = serde_json::to_value(scene()).unwrap();
        value["duration"] = serde_json::json!({"numerator": 0, "denominator": 1});
        value["nodes"][0]["opacity"] = serde_json::json!(1.5);
        value["nodes"][1]["children"][0]["asset_id"] = serde_json::json!("missing");
        value["animations"][0]["keyframes"] = serde_json::json!([
            {"at": {"numerator": 3, "denominator": 1}, "value": 1.0},
            {"at": {"numerator": 1, "denominator": 1}, "value": 0.0},
        ]);
        let parsed = parse_motion_scene(&serde_json::to_vec(&value).unwrap()).unwrap();
        let validation = validate_scene(&parsed);
        let codes = validation
            .errors
            .iter()
            .map(|error| (error.code.as_str(), error.path.as_str()))
            .collect::<Vec<_>>();
        assert!(
            codes.contains(&("MOTION_INVALID_DURATION", "/duration")),
            "{codes:?}"
        );
        assert!(
            codes.contains(&("MOTION_INVALID_OPACITY", "/nodes/0/opacity")),
            "{codes:?}"
        );
        assert!(
            codes.contains(&("MOTION_KEYFRAMES_UNSORTED", "/animations/0/keyframes/1/at")),
            "{codes:?}"
        );
        // The arrow's asset_id is meaningless for its kind and is not an error, but
        // an image naming an asset that does not exist is.
        assert!(!validation.ok);
    }

    #[test]
    fn a_duplicate_node_id_anywhere_in_the_tree_is_refused() {
        let mut value = serde_json::to_value(scene()).unwrap();
        value["nodes"][2]["node_id"] = serde_json::json!("code");
        let parsed = parse_motion_scene(&serde_json::to_vec(&value).unwrap()).unwrap();
        let validation = validate_scene(&parsed);
        assert!(
            validation
                .errors
                .iter()
                .any(|error| error.code == "MOTION_DUPLICATE_NODE_ID"
                    && error.path == "/nodes/2/node_id"),
            "{:?}",
            validation.errors
        );
    }

    #[test]
    fn nesting_past_the_depth_limit_is_reported_rather_than_recursed_through() {
        let mut value = serde_json::to_value(scene()).unwrap();
        let mut node = serde_json::json!({
            "node_id": "leaf",
            "kind": "rectangle",
            "size": {"width": 1.0, "height": 1.0},
        });
        for index in 0..(MAX_MOTION_NODE_DEPTH + 2) {
            node = serde_json::json!({
                "node_id": format!("group-{index}"),
                "kind": "group",
                "children": [node],
            });
        }
        value["nodes"] = serde_json::json!([node]);
        let parsed = parse_motion_scene(&serde_json::to_vec(&value).unwrap()).unwrap();
        let validation = validate_scene(&parsed);
        assert!(
            validation
                .errors
                .iter()
                .any(|error| error.code == "MOTION_DEPTH_EXCEEDED"),
            "{:?}",
            validation.errors
        );
    }

    #[test]
    fn an_animation_naming_a_missing_node_or_repeating_a_property_is_refused() {
        let mut value = serde_json::to_value(scene()).unwrap();
        value["animations"] = serde_json::json!([
            {
                "node_id": "nobody",
                "property": "opacity",
                "keyframes": [{"at": {"numerator": 0, "denominator": 1}, "value": 1.0}],
            },
            {
                "node_id": "code",
                "property": "opacity",
                "keyframes": [{"at": {"numerator": 0, "denominator": 1}, "value": 1.0}],
            },
            {
                "node_id": "code",
                "property": "opacity",
                "keyframes": [{"at": {"numerator": 0, "denominator": 1}, "value": 1.0}],
            },
        ]);
        let parsed = parse_motion_scene(&serde_json::to_vec(&value).unwrap()).unwrap();
        let codes = validate_scene(&parsed)
            .errors
            .iter()
            .map(|error| error.code.clone())
            .collect::<Vec<_>>();
        assert!(
            codes.contains(&"MOTION_ANIMATION_UNKNOWN_NODE".to_owned()),
            "{codes:?}"
        );
        assert!(
            codes.contains(&"MOTION_ANIMATION_DUPLICATE_PROPERTY".to_owned()),
            "{codes:?}"
        );
    }

    #[test]
    fn inspection_summarises_the_scene_and_carries_the_validation() {
        let inspection = inspect_motion_scene(&scene());
        assert_eq!(inspection.node_count, 4);
        assert_eq!(inspection.asset_count, 1);
        assert_eq!(inspection.animation_count, 1);
        assert_eq!(inspection.keyframe_count, 2);
        assert_eq!(inspection.duration_milliseconds, 4_000);
        assert_eq!(inspection.canvas_width, 1920);
        assert_eq!(
            inspection.node_kinds,
            vec!["text", "group", "arrow", "image"]
        );
        assert!(inspection.validation.ok);

        let mut broken = scene();
        broken.canvas.width = 0;
        let inspection = inspect_motion_scene(&broken);
        assert!(!inspection.validation.ok);
        assert_eq!(
            inspection.validation.errors[0].code,
            "MOTION_INVALID_CANVAS"
        );
    }

    #[test]
    fn evaluation_is_deterministic_and_in_document_order() {
        let scene = scene();
        let time = RationalTime::new(1, 2).unwrap();
        let first = evaluate_scene(&scene, time).unwrap();
        let second = evaluate_scene(&scene, time).unwrap();
        assert_eq!(first, second);
        assert_eq!(
            first
                .instructions
                .iter()
                .map(|instruction| instruction.node_id.as_str())
                .collect::<Vec<_>>(),
            vec!["code", "callout", "arrow", "diagram"]
        );
        assert_eq!(
            first
                .instructions
                .iter()
                .map(|instruction| instruction.depth)
                .collect::<Vec<_>>(),
            vec![0, 0, 1, 0]
        );
    }

    #[test]
    fn a_time_outside_the_half_open_interval_is_refused_rather_than_clamped() {
        let scene = scene();
        let duration = scene.duration;
        assert_eq!(
            evaluate_scene(&scene, duration).unwrap_err(),
            MotionEvaluationError::TimeOutOfRange
        );
        assert_eq!(
            evaluate_scene(&scene, RationalTime::new(-1, 1).unwrap()).unwrap_err(),
            MotionEvaluationError::TimeOutOfRange
        );
        assert_eq!(
            MotionEvaluationError::TimeOutOfRange.code(),
            "MOTION_TIME_OUT_OF_RANGE"
        );
        // The last instant inside the interval evaluates.
        assert!(evaluate_scene(&scene, RationalTime::new(3999, 1000).unwrap()).is_ok());
    }

    #[test]
    fn an_invalid_scene_is_refused_instead_of_partially_drawn() {
        let mut broken = scene();
        broken.duration = RationalTime::new(0, 1).unwrap();
        let error = evaluate_scene(&broken, RationalTime::new(0, 1).unwrap()).unwrap_err();
        match error {
            MotionEvaluationError::InvalidScene(validation) => {
                assert!(!validation.ok);
                assert_eq!(validation.errors[0].code, "MOTION_INVALID_DURATION");
            }
            other => panic!("expected an invalid-scene refusal, got {other:?}"),
        }
    }

    #[test]
    fn animation_resolves_by_property_and_holds_outside_its_keyframes() {
        let scene = scene();
        // The opacity track runs 0 -> 1 over one second with ease_out.
        let start = evaluate_scene(&scene, RationalTime::new(0, 1).unwrap()).unwrap();
        let code = |frame: &MotionFrame| {
            frame
                .instructions
                .iter()
                .find(|instruction| instruction.node_id == "code")
                .unwrap()
                .opacity
        };
        assert_eq!(code(&start), 0.0);
        let middle = evaluate_scene(&scene, RationalTime::new(1, 2).unwrap()).unwrap();
        // ease_out(0.5) = 0.75, and the value interpolates 0 -> 1.
        assert!((code(&middle) - 0.75).abs() < 1e-9, "{}", code(&middle));
        let after = evaluate_scene(&scene, RationalTime::new(3, 1).unwrap()).unwrap();
        assert_eq!(code(&after), 1.0);
    }

    #[test]
    fn translation_scale_and_geometry_animate_onto_the_instruction() {
        let mut scene = scene();
        scene.animations = vec![
            MotionAnimation {
                node_id: "diagram".to_owned(),
                property: MotionProperty::TranslationX,
                keyframes: vec![
                    MotionKeyframe {
                        at: RationalTime::new(0, 1).unwrap(),
                        value: 0.0,
                    },
                    MotionKeyframe {
                        at: RationalTime::new(2, 1).unwrap(),
                        value: 100.0,
                    },
                ],
                easing: MotionEasing::Linear,
            },
            MotionAnimation {
                node_id: "code".to_owned(),
                property: MotionProperty::Scale,
                keyframes: vec![
                    MotionKeyframe {
                        at: RationalTime::new(0, 1).unwrap(),
                        value: 1.0,
                    },
                    MotionKeyframe {
                        at: RationalTime::new(2, 1).unwrap(),
                        value: 2.0,
                    },
                ],
                easing: MotionEasing::Linear,
            },
            MotionAnimation {
                node_id: "diagram".to_owned(),
                property: MotionProperty::Scale,
                keyframes: vec![
                    MotionKeyframe {
                        at: RationalTime::new(0, 1).unwrap(),
                        value: 1.0,
                    },
                    MotionKeyframe {
                        at: RationalTime::new(2, 1).unwrap(),
                        value: 2.0,
                    },
                ],
                easing: MotionEasing::Linear,
            },
        ];
        let frame = evaluate_scene(&scene, RationalTime::new(1, 1).unwrap()).unwrap();
        let diagram = frame
            .instructions
            .iter()
            .find(|instruction| instruction.node_id == "diagram")
            .unwrap();
        assert_eq!(diagram.position.x, 950.0);
        assert_eq!(diagram.scale, 1.5);
        let size = diagram.size.unwrap();
        assert_eq!(size.width, 960.0);
        assert_eq!(size.height, 540.0);
        // Scale reaches a text node's type size too, so it is not a property that
        // does nothing on text.
        let text = frame
            .instructions
            .iter()
            .find(|instruction| instruction.node_id == "code")
            .unwrap();
        assert_eq!(text.font_size, Some(42.0));
        // The image's own opacity is untouched by the geometry tracks.
        assert!((diagram.opacity - 0.9).abs() < 1e-9);
    }

    #[test]
    fn evaluation_reads_no_asset_and_no_font() {
        // An asset path that exists nowhere still evaluates: evaluation resolves
        // identities and never opens a file, which is what lets a render path run
        // with no filesystem, network or provider access at all.
        let mut value = serde_json::to_value(scene()).unwrap();
        value["assets"][0]["path"] = serde_json::json!("assets/does-not-exist.png");
        value["nodes"][0]["font_size"] = serde_json::json!(28.0);
        let parsed = parse_motion_scene(&serde_json::to_vec(&value).unwrap()).unwrap();
        let frame = evaluate_scene(&parsed, RationalTime::new(1, 1).unwrap()).unwrap();
        assert_eq!(frame.instructions.len(), 4);
        assert_eq!(frame.instructions[3].asset_id.as_deref(), Some("diagram"));
    }

    #[test]
    fn the_easing_set_is_locked_and_bounded() {
        assert_eq!(ease(MotionEasing::Linear, 0.25), 0.25);
        assert_eq!(ease(MotionEasing::EaseIn, 0.5), 0.25);
        assert_eq!(ease(MotionEasing::EaseOut, 0.5), 0.75);
        assert_eq!(ease(MotionEasing::EaseInOut, 0.5), 0.5);
        // Out-of-range progress is clamped rather than extrapolated.
        assert_eq!(ease(MotionEasing::Linear, -1.0), 0.0);
        assert_eq!(ease(MotionEasing::Linear, 2.0), 1.0);
    }

    #[test]
    fn a_keyframe_past_the_duration_is_out_of_range() {
        let mut value = serde_json::to_value(scene()).unwrap();
        value["animations"][0]["keyframes"][1]["at"] =
            serde_json::json!({"numerator": 9, "denominator": 1});
        let parsed = parse_motion_scene(&serde_json::to_vec(&value).unwrap()).unwrap();
        let validation = validate_scene(&parsed);
        assert!(
            validation
                .errors
                .iter()
                .any(|error| error.code == "MOTION_KEYFRAME_OUT_OF_RANGE"
                    && error.path == "/animations/0/keyframes/1/at"),
            "{:?}",
            validation.errors
        );
    }
}

/// One resolved draw instruction: what a renderer needs for one node at one exact
/// time, with every animated property already applied.
///
/// The shape is deliberately flat and typed. A renderer never sees a keyframe, an
/// easing curve or a property name — evaluation has already resolved them — so the
/// render path cannot accidentally depend on scene structure.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MotionInstruction {
    pub node_id: String,
    pub kind: MotionNodeKind,
    /// Nesting depth, 0 for a top-level node. A renderer may use it for ordering.
    pub depth: usize,
    pub position: MotionPoint,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<MotionSize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end: Option<MotionPoint>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill: Option<MotionColor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke: Option<MotionColor>,
    pub stroke_width: f64,
    pub opacity: f64,
    pub rotation_degrees: f64,
    pub scale: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_size: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asset_id: Option<String>,
    /// Highlighted runs of `text`, resolved from the node. Empty means the node's
    /// `fill` colours the whole string.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub spans: Vec<MotionTextSpan>,
}

/// Everything a renderer draws for one exact time.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MotionFrame {
    pub time: RationalTime,
    pub canvas: MotionCanvas,
    /// Instructions in document order, depth first, so the same scene and time
    /// always produce the same list in the same order.
    pub instructions: Vec<MotionInstruction>,
}

/// Why a scene could not be evaluated.
#[derive(Clone, Debug, PartialEq)]
pub enum MotionEvaluationError {
    /// The scene does not validate; evaluation never runs on a scene a validator
    /// refused.
    InvalidScene(MotionValidation),
    /// The time is outside the half-open interval `[0, duration)`.
    TimeOutOfRange,
}

impl MotionEvaluationError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidScene(_) => "MOTION_INVALID_SCENE",
            Self::TimeOutOfRange => "MOTION_TIME_OUT_OF_RANGE",
        }
    }
}

/// Evaluates a scene at one exact time.
///
/// Pure and total over valid scenes: the same scene and time always produce the same
/// frame, in document order. Evaluation reads nothing — no asset, no font, no file,
/// no network, no provider — because the ADR requires that rendering never depends
/// on any of them. An invalid scene is refused rather than partially drawn, and a
/// time outside `[0, duration)` is refused rather than clamped, because a silent
/// clamp would hide a caller's arithmetic error behind a plausible picture.
pub fn evaluate_scene(
    scene: &MotionScene,
    time: RationalTime,
) -> Result<MotionFrame, MotionEvaluationError> {
    let validation = validate_scene(scene);
    if !validation.ok {
        return Err(MotionEvaluationError::InvalidScene(validation));
    }
    if time.is_negative() || time >= scene.duration {
        return Err(MotionEvaluationError::TimeOutOfRange);
    }

    let mut instructions = Vec::new();
    let mut pending: Vec<(&MotionNode, usize)> =
        scene.nodes.iter().rev().map(|node| (node, 0)).collect();
    while let Some((node, depth)) = pending.pop() {
        instructions.push(resolve_node(scene, node, depth, time));
        if let Some(children) = &node.children {
            pending.extend(children.iter().rev().map(|child| (child, depth + 1)));
        }
    }

    Ok(MotionFrame {
        time,
        canvas: scene.canvas,
        instructions,
    })
}

fn resolve_node(
    scene: &MotionScene,
    node: &MotionNode,
    depth: usize,
    time: RationalTime,
) -> MotionInstruction {
    let mut position = node.position.unwrap_or(MotionPoint { x: 0.0, y: 0.0 });
    let mut size = node.size;
    let mut opacity = node.opacity.unwrap_or(1.0);
    let mut rotation_degrees = node.rotation_degrees.unwrap_or(0.0);
    let mut scale = 1.0;

    for animation in scene
        .animations
        .iter()
        .filter(|animation| animation.node_id == node.node_id)
    {
        let Some(value) = sample_animation(animation, time) else {
            continue;
        };
        match animation.property {
            MotionProperty::TranslationX => position.x += value,
            MotionProperty::TranslationY => position.y += value,
            MotionProperty::Scale => scale *= value,
            MotionProperty::Rotation => rotation_degrees += value,
            MotionProperty::Opacity => opacity = value,
            MotionProperty::Width => {
                let mut resolved = size.unwrap_or(MotionSize {
                    width: value,
                    height: 0.0,
                });
                resolved.width = value;
                size = Some(resolved);
            }
            MotionProperty::Height => {
                let mut resolved = size.unwrap_or(MotionSize {
                    width: 0.0,
                    height: value,
                });
                resolved.height = value;
                size = Some(resolved);
            }
        }
    }

    if let Some(resolved) = size {
        size = Some(MotionSize {
            width: resolved.width * scale,
            height: resolved.height * scale,
        });
    }
    // Scale applies to whatever the node draws: a text node's type size as well as a
    // shape's geometry, so the property does not silently do nothing on text.
    let font_size = node.font_size.map(|size| size * scale);

    MotionInstruction {
        node_id: node.node_id.clone(),
        kind: node.kind,
        depth,
        position,
        size,
        end: node.end,
        fill: node.fill,
        stroke: node.stroke,
        stroke_width: node.stroke_width.unwrap_or(0.0),
        opacity: opacity.clamp(0.0, 1.0),
        rotation_degrees,
        scale,
        text: node.text.clone(),
        font_size,
        asset_id: node.asset_id.clone(),
        spans: node.spans.clone().unwrap_or_default(),
    }
}

/// The animated value of one property at `time`.
///
/// Before the first keyframe the first value holds, and after the last the last
/// value holds, so a track is total over the scene. Interpolation is exact rational
/// arithmetic for the time ratio and the easing curve is a locked set, which is what
/// makes the result reproducible rather than merely close.
pub fn sample_animation(animation: &MotionAnimation, time: RationalTime) -> Option<f64> {
    let first = animation.keyframes.first()?;
    if time <= first.at {
        return Some(first.value);
    }
    let last = animation.keyframes.last()?;
    if time >= last.at {
        return Some(last.value);
    }
    for pair in animation.keyframes.windows(2) {
        let [start, end] = pair else {
            continue;
        };
        if time >= start.at && time <= end.at {
            let span = end.at.checked_sub(start.at).ok()?;
            let offset = time.checked_sub(start.at).ok()?;
            let progress = ratio(offset, span);
            let eased = ease(animation.easing, progress);
            return Some(start.value + (end.value - start.value) * eased);
        }
    }
    Some(last.value)
}

/// `offset / span` as an `f64`, computed through the exact rationals rather than
/// through floating-point seconds, so two hosts agree on the same frame.
fn ratio(offset: RationalTime, span: RationalTime) -> f64 {
    if span.is_zero() {
        return 0.0;
    }
    let numerator = offset.numerator() as f64 * span.denominator() as f64;
    let denominator = offset.denominator() as f64 * span.numerator() as f64;
    if denominator == 0.0 {
        return 0.0;
    }
    (numerator / denominator).clamp(0.0, 1.0)
}

/// The locked easing set, applied to a `0..=1` progress.
pub fn ease(easing: MotionEasing, progress: f64) -> f64 {
    let progress = progress.clamp(0.0, 1.0);
    match easing {
        MotionEasing::Linear => progress,
        MotionEasing::EaseIn => progress * progress,
        MotionEasing::EaseOut => 1.0 - (1.0 - progress) * (1.0 - progress),
        MotionEasing::EaseInOut => {
            if progress < 0.5 {
                2.0 * progress * progress
            } else {
                1.0 - 2.0 * (1.0 - progress) * (1.0 - progress)
            }
        }
    }
}
