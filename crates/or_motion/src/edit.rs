//! Direct scene editing for the GUI (ADR 0010, P4).
//!
//! The Motion surface can move, resize, rotate, retitle, recolour, outline, add and
//! delete the nodes of a composed scene, singly or as a selection. Every one of those
//! gestures is a **typed, closed operation** applied here and re-validated through
//! the ordinary [`validate_scene`] path, so a drag can never write a scene the
//! renderer would refuse, and the app never grows a second, unchecked mutation path.
//!
//! An edit names **evaluated** geometry — where the picture puts a node — not the raw
//! field. A node under a translation track has an anchor *plus* the track's value at
//! time zero; the operations here subtract that offset, so dropping an object where
//! the pointer is does not make it jump by the track's first keyframe.
//!
//! Nothing here renders, touches the filesystem, or evaluates an expression: an edit
//! is a pure `MotionScene -> MotionScene` step plus a small, bounded hit-box summary
//! the UI uses to draw its handles.

use crate::{
    MAX_MOTION_NODES, MAX_MOTION_TEXT_BYTES, MotionCanvas, MotionColor, MotionNode, MotionNodeKind,
    MotionPoint, MotionProperty, MotionScene, MotionSize, validate_scene,
};
use or_core::RationalTime;
use serde::{Deserialize, Serialize};
use std::fmt;

/// The largest edit document accepted. An edit names a selection and a few numbers,
/// so this is generous and still bounded.
pub const MAX_MOTION_EDIT_BYTES: usize = 256 * 1024;
/// The most nodes one selection operation may name.
pub const MAX_MOTION_EDIT_NODES: usize = 1024;
/// How wide one glyph is assumed to be, as a fraction of the type size, when a text
/// node's handle box is measured. This is a **hit box for a pointer**, not glyph
/// shaping: the renderer still measures real text with `cosmic-text`.
pub const MOTION_TEXT_HIT_ADVANCE: f64 = 0.56;
/// The type size a new text node gets when the caller does not name one.
pub const MOTION_DEFAULT_TEXT_SIZE: f64 = 64.0;
/// The outline width a node gets when it is first outlined.
pub const MOTION_DEFAULT_STROKE_WIDTH: f64 = 4.0;
/// The widest outline an edit may write.
pub const MAX_MOTION_STROKE_WIDTH: f64 = 200.0;

/// One node's target position, for a multi-node move.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MotionNodeTarget {
    pub node_id: String,
    /// The evaluated canvas position the node should occupy at time zero.
    pub x: f64,
    pub y: f64,
}

/// One gesture on a composed scene.
///
/// The tag is `op` and the set is closed: `deny_unknown_fields` means a client that
/// invents an operation gets a refusal, not a silently ignored edit.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum MotionSceneEdit {
    /// Puts one node where the picture shows it, at a canvas position.
    Move { node_id: String, x: f64, y: f64 },
    /// Puts a whole selection at its target positions in one step.
    MoveMany { nodes: Vec<MotionNodeTarget> },
    /// Sets a shape's evaluated box. `x`/`y` are the evaluated top-left corner; when
    /// absent the anchor is left alone and only the size changes.
    Resize {
        node_id: String,
        width: f64,
        height: f64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        x: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        y: Option<f64>,
    },
    /// Sets a node's evaluated rotation in degrees.
    Rotate { node_id: String, degrees: f64 },
    /// Replaces a text node's literal content.
    SetText { node_id: String, text: String },
    /// Sets a node's fill colour.
    SetFill { node_id: String, color: MotionColor },
    /// Sets the fill colour of a whole selection.
    SetFillMany {
        node_ids: Vec<String>,
        color: MotionColor,
    },
    /// Sets or clears a shape's outline. `color: null` removes it.
    SetStroke {
        node_id: String,
        color: Option<MotionColor>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        width: Option<f64>,
    },
    /// Sets a text node's type size in canvas pixels.
    SetFontSize { node_id: String, font_size: f64 },
    /// Removes a node, its children and the animations that targeted it.
    Delete { node_id: String },
    /// Removes a whole selection.
    DeleteMany { node_ids: Vec<String> },
    /// Appends a new text node.
    AddText {
        /// A caller-chosen id; absent means one is generated.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        node_id: Option<String>,
        text: String,
        x: f64,
        y: f64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        font_size: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        fill: Option<MotionColor>,
    },
}

/// Why an edit was refused. Codes are stable and prefixed `MOTION_EDIT_`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MotionSceneEditError {
    /// The edit document is larger than [`MAX_MOTION_EDIT_BYTES`].
    TooLarge,
    /// Not a JSON edit this version understands.
    InvalidJson,
    /// No node in the scene carries that id.
    UnknownNode(String),
    /// A new node would reuse an id that already exists.
    DuplicateNode(String),
    /// The operation needs a text node and this one is not.
    NotText(String),
    /// The operation needs a node that draws something.
    NotDrawable(String),
    /// The operation needs a node with a resizeable box.
    NotResizable(String),
    /// The operation needs a node that can carry an outline.
    NotStrokeable(String),
    /// The node's size is animated, so a static size cannot change it.
    AnimatedSize(String),
    /// The operation names no nodes.
    EmptySelection,
    /// The operation names more nodes than [`MAX_MOTION_EDIT_NODES`].
    SelectionTooLarge,
    /// A coordinate, size or colour was not usable.
    InvalidValue(String),
    /// Applying the edit produced a scene the validator refuses.
    InvalidScene,
}

impl MotionSceneEditError {
    /// The stable diagnostic code this error reports.
    pub fn code(&self) -> &'static str {
        match self {
            Self::TooLarge => "MOTION_EDIT_TOO_LARGE",
            Self::InvalidJson => "MOTION_EDIT_INVALID_JSON",
            Self::UnknownNode(_) => "MOTION_EDIT_UNKNOWN_NODE",
            Self::DuplicateNode(_) => "MOTION_EDIT_DUPLICATE_NODE",
            Self::NotText(_) => "MOTION_EDIT_NOT_TEXT",
            Self::NotDrawable(_) => "MOTION_EDIT_NOT_DRAWABLE",
            Self::NotResizable(_) => "MOTION_EDIT_NOT_RESIZABLE",
            Self::NotStrokeable(_) => "MOTION_EDIT_NOT_STROKEABLE",
            Self::AnimatedSize(_) => "MOTION_EDIT_ANIMATED_SIZE",
            Self::EmptySelection => "MOTION_EDIT_EMPTY_SELECTION",
            Self::SelectionTooLarge => "MOTION_EDIT_SELECTION_TOO_LARGE",
            Self::InvalidValue(_) => "MOTION_EDIT_INVALID_VALUE",
            Self::InvalidScene => "MOTION_EDIT_INVALID_SCENE",
        }
    }
}

impl fmt::Display for MotionSceneEditError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLarge => write!(
                formatter,
                "an edit may hold at most {MAX_MOTION_EDIT_BYTES} bytes"
            ),
            Self::InvalidJson => write!(formatter, "the edit is not a supported operation"),
            Self::UnknownNode(node_id) => write!(formatter, "no node with id {node_id}"),
            Self::DuplicateNode(node_id) => write!(formatter, "a node with id {node_id} exists"),
            Self::NotText(node_id) => write!(formatter, "{node_id} is not a text node"),
            Self::NotDrawable(node_id) => write!(formatter, "{node_id} does not draw a fill"),
            Self::NotResizable(node_id) => {
                write!(formatter, "{node_id} has no resizeable box")
            }
            Self::NotStrokeable(node_id) => write!(formatter, "{node_id} cannot carry an outline"),
            Self::AnimatedSize(node_id) => write!(
                formatter,
                "{node_id} has an animated width or height, so its size is driven by the track"
            ),
            Self::EmptySelection => write!(formatter, "the edit names no nodes"),
            Self::SelectionTooLarge => write!(
                formatter,
                "one edit may name at most {MAX_MOTION_EDIT_NODES} nodes"
            ),
            Self::InvalidValue(problem) => write!(formatter, "{problem}"),
            Self::InvalidScene => write!(formatter, "the edited scene did not validate"),
        }
    }
}

/// One node as the editor draws it: an anchor, a handle box and what to show.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct MotionNodeBox {
    pub node_id: String,
    pub kind: MotionNodeKind,
    /// The literal string of a text node, empty otherwise.
    pub text: String,
    pub position: MotionPoint,
    /// The handle box in canvas pixels, already scaled and rotated as the picture
    /// shows it. Text width is an estimate (see [`MOTION_TEXT_HIT_ADVANCE`]); shapes
    /// use their evaluated size.
    pub size: MotionSize,
    pub font_size: f64,
    pub opacity: f64,
    pub fill: Option<MotionColor>,
    pub stroke: Option<MotionColor>,
    pub stroke_width: f64,
    /// Degrees, counter-clockwise, as evaluated at time zero.
    pub rotation_degrees: f64,
    /// Nesting depth, 0 for a top-level node.
    pub depth: usize,
    /// Whether an animation targets this node. The editor edits the static anchor;
    /// a translation track moves on top of it.
    pub animated: bool,
}

impl MotionNodeBox {
    /// Where a pointer drag of `delta` should put this box, in evaluated canvas
    /// coordinates.
    pub fn moved_by(&self, delta: MotionPoint) -> MotionPoint {
        MotionPoint {
            x: self.position.x + delta.x,
            y: self.position.y + delta.y,
        }
    }

    /// Whether this box carries a shape whose size the editor can drag.
    pub fn is_resizeable(&self) -> bool {
        matches!(
            self.kind,
            MotionNodeKind::Rectangle | MotionNodeKind::Ellipse | MotionNodeKind::Image
        )
    }

    /// Whether the renderer draws an outline for this kind.
    pub fn is_strokeable(&self) -> bool {
        matches!(
            self.kind,
            MotionNodeKind::Rectangle | MotionNodeKind::Ellipse | MotionNodeKind::Arrow
        )
    }
}

/// Parses an edit, refusing an oversized or malformed document.
pub fn parse_scene_edit(bytes: &[u8]) -> Result<MotionSceneEdit, MotionSceneEditError> {
    if bytes.len() > MAX_MOTION_EDIT_BYTES {
        return Err(MotionSceneEditError::TooLarge);
    }
    serde_json::from_slice(bytes).map_err(|_| MotionSceneEditError::InvalidJson)
}

/// The track values a node's fields are adjusted by at time zero.
#[derive(Clone, Copy, Debug, Default)]
struct TrackOffset {
    x: f64,
    y: f64,
    rotation: f64,
    scale: f64,
    width: Option<f64>,
    height: Option<f64>,
}

impl TrackOffset {
    fn scale_or_one(self) -> f64 {
        if self.scale.is_finite() && self.scale != 0.0 {
            self.scale
        } else {
            1.0
        }
    }
}

/// Reads one node's track offset at time zero with the evaluator's own sampling, so
/// an edit lands exactly where the preview shows the node.
fn track_offset(scene: &MotionScene, node_id: &str) -> TrackOffset {
    let mut offset = TrackOffset {
        scale: 1.0,
        ..TrackOffset::default()
    };
    for animation in scene
        .animations
        .iter()
        .filter(|animation| animation.node_id == node_id)
    {
        let Some(value) = crate::sample_animation(animation, RationalTime::ZERO) else {
            continue;
        };
        match animation.property {
            MotionProperty::TranslationX => offset.x += value,
            MotionProperty::TranslationY => offset.y += value,
            MotionProperty::Rotation => offset.rotation += value,
            MotionProperty::Scale => offset.scale *= value,
            MotionProperty::Width => offset.width = Some(value),
            MotionProperty::Height => offset.height = Some(value),
            MotionProperty::Opacity => {}
        }
    }
    offset
}

/// Applies one edit and validates the result.
///
/// The input scene is **not** re-validated here: the bridge parses and validates it
/// before calling, and an edit is meaningless on a scene the renderer refuses. The
/// result is always validated, so an edit can never produce a scene that renders
/// differently than it previews.
pub fn apply_scene_edit(
    scene: &MotionScene,
    edit: &MotionSceneEdit,
) -> Result<MotionScene, MotionSceneEditError> {
    let mut next = scene.clone();
    match edit {
        MotionSceneEdit::Move { node_id, x, y } => {
            move_to(&mut next, scene, node_id, *x, *y)?;
        }
        MotionSceneEdit::MoveMany { nodes } => {
            check_selection_size(nodes.len())?;
            if nodes.is_empty() {
                return Err(MotionSceneEditError::EmptySelection);
            }
            for target in nodes {
                move_to(&mut next, scene, &target.node_id, target.x, target.y)?;
            }
        }
        MotionSceneEdit::Resize {
            node_id,
            width,
            height,
            x,
            y,
        } => {
            if !width.is_finite() || !height.is_finite() || *width <= 0.0 || *height <= 0.0 {
                return Err(MotionSceneEditError::InvalidValue(
                    "a size must be a finite, positive number".to_owned(),
                ));
            }
            if x.is_some_and(|value| !value.is_finite())
                || y.is_some_and(|value| !value.is_finite())
            {
                return Err(MotionSceneEditError::InvalidValue(
                    "a position must be a finite number".to_owned(),
                ));
            }
            let offset = track_offset(scene, node_id);
            if offset.width.is_some() || offset.height.is_some() {
                return Err(MotionSceneEditError::AnimatedSize(node_id.clone()));
            }
            let scale = offset.scale_or_one();
            let node = find_node_mut(&mut next.nodes, node_id)
                .ok_or_else(|| MotionSceneEditError::UnknownNode(node_id.clone()))?;
            if !matches!(
                node.kind,
                MotionNodeKind::Rectangle | MotionNodeKind::Ellipse | MotionNodeKind::Image
            ) {
                return Err(MotionSceneEditError::NotResizable(node_id.clone()));
            }
            node.size = Some(MotionSize {
                width: (width / scale).clamp(0.5, f64::from(scene.canvas.width).max(1.0) * 8.0),
                height: (height / scale).clamp(0.5, f64::from(scene.canvas.height).max(1.0) * 8.0),
            });
            if let (Some(x), Some(y)) = (x, y) {
                node.position = Some(MotionPoint {
                    x: clamp_axis(x - offset.x, scene.canvas.width),
                    y: clamp_axis(y - offset.y, scene.canvas.height),
                });
            }
        }
        MotionSceneEdit::Rotate { node_id, degrees } => {
            if !degrees.is_finite() {
                return Err(MotionSceneEditError::InvalidValue(
                    "a rotation must be a finite number".to_owned(),
                ));
            }
            let offset = track_offset(scene, node_id);
            let node = find_node_mut(&mut next.nodes, node_id)
                .ok_or_else(|| MotionSceneEditError::UnknownNode(node_id.clone()))?;
            node.rotation_degrees = Some(normalize_degrees(degrees - offset.rotation));
        }
        MotionSceneEdit::SetText { node_id, text } => {
            if text.len() > MAX_MOTION_TEXT_BYTES {
                return Err(MotionSceneEditError::InvalidValue(format!(
                    "text may hold at most {MAX_MOTION_TEXT_BYTES} bytes"
                )));
            }
            let node = find_node_mut(&mut next.nodes, node_id)
                .ok_or_else(|| MotionSceneEditError::UnknownNode(node_id.clone()))?;
            if node.kind != MotionNodeKind::Text {
                return Err(MotionSceneEditError::NotText(node_id.clone()));
            }
            node.text = Some(text.clone());
            // Highlight runs are byte ranges into the old string; keeping them would
            // point at the wrong characters or fall outside the new text.
            node.spans = None;
        }
        MotionSceneEdit::SetFill { node_id, color } => {
            let node = find_node_mut(&mut next.nodes, node_id)
                .ok_or_else(|| MotionSceneEditError::UnknownNode(node_id.clone()))?;
            if node.kind == MotionNodeKind::Group {
                return Err(MotionSceneEditError::NotDrawable(node_id.clone()));
            }
            node.fill = Some(*color);
        }
        MotionSceneEdit::SetFillMany { node_ids, color } => {
            check_selection_size(node_ids.len())?;
            if node_ids.is_empty() {
                return Err(MotionSceneEditError::EmptySelection);
            }
            for node_id in node_ids {
                let node = find_node_mut(&mut next.nodes, node_id)
                    .ok_or_else(|| MotionSceneEditError::UnknownNode(node_id.clone()))?;
                if node.kind == MotionNodeKind::Group {
                    return Err(MotionSceneEditError::NotDrawable(node_id.clone()));
                }
                node.fill = Some(*color);
            }
        }
        MotionSceneEdit::SetStroke {
            node_id,
            color,
            width,
        } => {
            let node = find_node_mut(&mut next.nodes, node_id)
                .ok_or_else(|| MotionSceneEditError::UnknownNode(node_id.clone()))?;
            if !matches!(
                node.kind,
                MotionNodeKind::Rectangle | MotionNodeKind::Ellipse | MotionNodeKind::Arrow
            ) {
                return Err(MotionSceneEditError::NotStrokeable(node_id.clone()));
            }
            match color {
                None => {
                    node.stroke = None;
                    node.stroke_width = None;
                }
                Some(color) => {
                    let chosen = width.unwrap_or_else(|| {
                        node.stroke_width
                            .filter(|width| *width > 0.0)
                            .unwrap_or(MOTION_DEFAULT_STROKE_WIDTH)
                    });
                    if !chosen.is_finite() || chosen <= 0.0 {
                        return Err(MotionSceneEditError::InvalidValue(
                            "an outline width must be a positive number".to_owned(),
                        ));
                    }
                    node.stroke = Some(*color);
                    node.stroke_width = Some(chosen.min(MAX_MOTION_STROKE_WIDTH));
                }
            }
        }
        MotionSceneEdit::SetFontSize { node_id, font_size } => {
            if !font_size.is_finite() || *font_size <= 0.0 {
                return Err(MotionSceneEditError::InvalidValue(
                    "font_size must be greater than zero".to_owned(),
                ));
            }
            let node = find_node_mut(&mut next.nodes, node_id)
                .ok_or_else(|| MotionSceneEditError::UnknownNode(node_id.clone()))?;
            if node.kind != MotionNodeKind::Text {
                return Err(MotionSceneEditError::NotText(node_id.clone()));
            }
            node.font_size = Some(*font_size);
        }
        MotionSceneEdit::Delete { node_id } => {
            if !remove_node(&mut next.nodes, node_id) {
                return Err(MotionSceneEditError::UnknownNode(node_id.clone()));
            }
            next.animations
                .retain(|animation| &animation.node_id != node_id);
        }
        MotionSceneEdit::DeleteMany { node_ids } => {
            check_selection_size(node_ids.len())?;
            if node_ids.is_empty() {
                return Err(MotionSceneEditError::EmptySelection);
            }
            let mut unique: Vec<&str> = Vec::new();
            for node_id in node_ids {
                if !unique.contains(&node_id.as_str()) {
                    unique.push(node_id);
                }
            }
            for node_id in &unique {
                if !remove_node(&mut next.nodes, node_id) {
                    return Err(MotionSceneEditError::UnknownNode((*node_id).to_owned()));
                }
            }
            next.animations
                .retain(|animation| !unique.contains(&animation.node_id.as_str()));
        }
        MotionSceneEdit::AddText {
            node_id,
            text,
            x,
            y,
            font_size,
            fill,
        } => {
            if text.len() > MAX_MOTION_TEXT_BYTES {
                return Err(MotionSceneEditError::InvalidValue(format!(
                    "text may hold at most {MAX_MOTION_TEXT_BYTES} bytes"
                )));
            }
            let font_size = font_size.unwrap_or(MOTION_DEFAULT_TEXT_SIZE);
            if !font_size.is_finite() || font_size <= 0.0 {
                return Err(MotionSceneEditError::InvalidValue(
                    "font_size must be greater than zero".to_owned(),
                ));
            }
            if !x.is_finite() || !y.is_finite() {
                return Err(MotionSceneEditError::InvalidValue(
                    "a position must be a finite number".to_owned(),
                ));
            }
            let node_id = match node_id {
                Some(node_id) if !node_id.trim().is_empty() => {
                    if node_exists(&next.nodes, node_id) {
                        return Err(MotionSceneEditError::DuplicateNode(node_id.clone()));
                    }
                    node_id.clone()
                }
                _ => next_text_node_id(&next.nodes),
            };
            if node_count(&next.nodes) >= MAX_MOTION_NODES {
                return Err(MotionSceneEditError::InvalidScene);
            }
            next.nodes.push(MotionNode {
                node_id,
                kind: MotionNodeKind::Text,
                children: None,
                text: Some(text.clone()),
                position: Some(MotionPoint {
                    x: clamp_axis(*x, scene.canvas.width),
                    y: clamp_axis(*y, scene.canvas.height),
                }),
                size: None,
                end: None,
                fill: Some(fill.unwrap_or(MotionColor {
                    red: 245,
                    green: 245,
                    blue: 245,
                    alpha: 255,
                })),
                stroke: None,
                stroke_width: None,
                font_size: Some(font_size),
                asset_id: None,
                opacity: None,
                rotation_degrees: None,
                spans: None,
            });
        }
    }
    if !validate_scene(&next).ok {
        return Err(MotionSceneEditError::InvalidScene);
    }
    Ok(next)
}

/// Parses and applies an edit in one step.
pub fn apply_scene_edit_json(
    scene: &MotionScene,
    bytes: &[u8],
) -> Result<MotionScene, MotionSceneEditError> {
    apply_scene_edit(scene, &parse_scene_edit(bytes)?)
}

/// Puts one node where the picture shows it, subtracting the track that is drawn on
/// top of its anchor so a dropped object stays under the pointer.
fn move_to(
    next: &mut MotionScene,
    scene: &MotionScene,
    node_id: &str,
    x: f64,
    y: f64,
) -> Result<(), MotionSceneEditError> {
    if !x.is_finite() || !y.is_finite() {
        return Err(MotionSceneEditError::InvalidValue(
            "a position must be a finite number".to_owned(),
        ));
    }
    let offset = track_offset(scene, node_id);
    let node = find_node_mut(&mut next.nodes, node_id)
        .ok_or_else(|| MotionSceneEditError::UnknownNode(node_id.to_owned()))?;
    node.position = Some(MotionPoint {
        x: clamp_axis(x - offset.x, scene.canvas.width),
        y: clamp_axis(y - offset.y, scene.canvas.height),
    });
    Ok(())
}

fn check_selection_size(count: usize) -> Result<(), MotionSceneEditError> {
    if count > MAX_MOTION_EDIT_NODES {
        return Err(MotionSceneEditError::SelectionTooLarge);
    }
    Ok(())
}

/// The nodes of a scene with the handle box each one occupies at time zero.
///
/// Time zero is the frame the composition preview shows, so a box always sits where
/// the picture puts the node. A translation track moves on top of the static anchor
/// and is reflected here; the editor still edits the anchor.
pub fn scene_node_boxes(scene: &MotionScene) -> Vec<MotionNodeBox> {
    scene_node_boxes_at(scene, RationalTime::ZERO)
}

/// The same boxes at an exact time, so an agent or a reviewer can read a
/// mid-animation composition instead of guessing from a screenshot.
pub fn scene_node_boxes_at(scene: &MotionScene, time: RationalTime) -> Vec<MotionNodeBox> {
    let animated = |node_id: &str| {
        scene
            .animations
            .iter()
            .any(|animation| animation.node_id == node_id)
    };
    match crate::evaluate_scene(scene, time) {
        Ok(frame) => frame
            .instructions
            .into_iter()
            .map(|instruction| {
                let font_size = instruction.font_size.unwrap_or(0.0);
                let text = instruction.text.unwrap_or_default();
                let size = match instruction.kind {
                    MotionNodeKind::Text => text_box_size(&text, font_size),
                    MotionNodeKind::Group => MotionSize {
                        width: 0.0,
                        height: 0.0,
                    },
                    MotionNodeKind::Arrow => {
                        let end = instruction.end.unwrap_or(instruction.position);
                        MotionSize {
                            width: (end.x - instruction.position.x).abs(),
                            height: (end.y - instruction.position.y).abs(),
                        }
                    }
                    MotionNodeKind::Rectangle | MotionNodeKind::Ellipse | MotionNodeKind::Image => {
                        instruction.size.unwrap_or(MotionSize {
                            width: 0.0,
                            height: 0.0,
                        })
                    }
                };
                MotionNodeBox {
                    animated: animated(&instruction.node_id),
                    node_id: instruction.node_id,
                    kind: instruction.kind,
                    text,
                    position: instruction.position,
                    size,
                    font_size,
                    opacity: instruction.opacity,
                    fill: instruction.fill,
                    stroke: instruction.stroke,
                    stroke_width: instruction.stroke_width,
                    rotation_degrees: instruction.rotation_degrees,
                    depth: instruction.depth,
                }
            })
            .collect(),
        // A scene whose frame cannot be evaluated (an empty duration, say) is
        // still worth listing so the editor or an agent can fix it.
        Err(_) => raw_node_boxes(&scene.nodes, 0, &animated),
    }
}

/// The canvas a scene declares.
pub fn scene_canvas(scene: &MotionScene) -> MotionCanvas {
    scene.canvas
}

fn raw_node_boxes(
    nodes: &[MotionNode],
    depth: usize,
    animated: &impl Fn(&str) -> bool,
) -> Vec<MotionNodeBox> {
    let mut boxes = Vec::new();
    for node in nodes {
        let font_size = node.font_size.unwrap_or(0.0);
        let position = node.position.unwrap_or(MotionPoint { x: 0.0, y: 0.0 });
        let size = match node.kind {
            MotionNodeKind::Text => text_box_size(node.text.as_deref().unwrap_or(""), font_size),
            MotionNodeKind::Group => MotionSize {
                width: 0.0,
                height: 0.0,
            },
            MotionNodeKind::Arrow => {
                let end = node.end.unwrap_or(MotionPoint { x: 0.0, y: 0.0 });
                MotionSize {
                    width: (end.x - position.x).abs(),
                    height: (end.y - position.y).abs(),
                }
            }
            MotionNodeKind::Rectangle | MotionNodeKind::Ellipse | MotionNodeKind::Image => {
                node.size.unwrap_or(MotionSize {
                    width: 0.0,
                    height: 0.0,
                })
            }
        };
        boxes.push(MotionNodeBox {
            node_id: node.node_id.clone(),
            kind: node.kind,
            text: node.text.clone().unwrap_or_default(),
            position,
            size,
            font_size,
            opacity: node.opacity.unwrap_or(1.0),
            fill: node.fill,
            stroke: node.stroke,
            stroke_width: node.stroke_width.unwrap_or(0.0),
            rotation_degrees: node.rotation_degrees.unwrap_or(0.0),
            depth,
            animated: animated(&node.node_id),
        });
        if let Some(children) = &node.children {
            boxes.extend(raw_node_boxes(children, depth + 1, animated));
        }
    }
    boxes
}

/// The box a text node covers at time zero: one line, estimated advance per glyph.
fn text_box_size(text: &str, font_size: f64) -> MotionSize {
    let font_size = font_size.max(1.0);
    let glyphs = text.chars().count().max(1) as f64;
    MotionSize {
        width: (glyphs * font_size * MOTION_TEXT_HIT_ADVANCE).max(font_size * 0.5),
        height: font_size * 1.25,
    }
}

/// Wraps an angle into `(-180, 180]` so a rotation field stays readable.
fn normalize_degrees(degrees: f64) -> f64 {
    let wrapped = degrees.rem_euclid(360.0);
    if wrapped > 180.0 {
        wrapped - 360.0
    } else {
        wrapped
    }
}

fn clamp_axis(value: f64, extent: u32) -> f64 {
    value.clamp(0.0, extent.max(1) as f64)
}

fn find_node_mut<'a>(nodes: &'a mut [MotionNode], node_id: &str) -> Option<&'a mut MotionNode> {
    for node in nodes.iter_mut() {
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

fn node_exists(nodes: &[MotionNode], node_id: &str) -> bool {
    nodes.iter().any(|node| {
        node.node_id == node_id
            || node
                .children
                .as_deref()
                .is_some_and(|children| node_exists(children, node_id))
    })
}

fn remove_node(nodes: &mut Vec<MotionNode>, node_id: &str) -> bool {
    if let Some(index) = nodes.iter().position(|node| node.node_id == node_id) {
        nodes.remove(index);
        return true;
    }
    for node in nodes.iter_mut() {
        if let Some(children) = node.children.as_mut()
            && remove_node(children, node_id)
        {
            return true;
        }
    }
    false
}

fn node_count(nodes: &[MotionNode]) -> usize {
    nodes
        .iter()
        .map(|node| 1 + node.children.as_deref().map_or(0, node_count))
        .sum()
}

fn next_text_node_id(nodes: &[MotionNode]) -> String {
    let mut index = 1;
    loop {
        let candidate = format!("text-{index}");
        if !node_exists(nodes, &candidate) {
            return candidate;
        }
        index += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        MOTION_SCENE_FORMAT, MOTION_SCENE_SCHEMA_VERSION, MotionAnimation, MotionEasing,
        MotionKeyframe,
    };

    fn white() -> MotionColor {
        MotionColor {
            red: 255,
            green: 255,
            blue: 255,
            alpha: 255,
        }
    }

    fn red() -> MotionColor {
        MotionColor {
            red: 220,
            green: 40,
            blue: 90,
            alpha: 255,
        }
    }

    fn scene() -> MotionScene {
        MotionScene {
            format: MOTION_SCENE_FORMAT.to_owned(),
            schema_version: MOTION_SCENE_SCHEMA_VERSION,
            scene_id: "6f1c2f5e-3a4b-4c5d-8e9f-0a1b2c3d4e5f".to_owned(),
            name: "Composition".to_owned(),
            duration: RationalTime::new(5, 1).unwrap(),
            canvas: MotionCanvas {
                width: 1920,
                height: 1080,
            },
            assets: Vec::new(),
            nodes: vec![
                MotionNode {
                    node_id: "title".to_owned(),
                    kind: MotionNodeKind::Text,
                    children: None,
                    text: Some("Hello".to_owned()),
                    position: Some(MotionPoint { x: 100.0, y: 200.0 }),
                    size: None,
                    end: None,
                    fill: Some(white()),
                    stroke: None,
                    stroke_width: None,
                    font_size: Some(64.0),
                    asset_id: None,
                    opacity: None,
                    rotation_degrees: None,
                    spans: None,
                },
                MotionNode {
                    node_id: "card".to_owned(),
                    kind: MotionNodeKind::Group,
                    children: Some(vec![MotionNode {
                        node_id: "card-body".to_owned(),
                        kind: MotionNodeKind::Rectangle,
                        children: None,
                        text: None,
                        position: Some(MotionPoint { x: 40.0, y: 40.0 }),
                        size: Some(MotionSize {
                            width: 320.0,
                            height: 180.0,
                        }),
                        end: None,
                        fill: Some(white()),
                        stroke: None,
                        stroke_width: None,
                        font_size: None,
                        asset_id: None,
                        opacity: None,
                        rotation_degrees: None,
                        spans: None,
                    }]),
                    text: None,
                    position: Some(MotionPoint { x: 0.0, y: 0.0 }),
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
            ],
            animations: Vec::new(),
        }
    }

    fn translation(node_id: &str, property: MotionProperty, value: f64) -> MotionAnimation {
        MotionAnimation {
            node_id: node_id.to_owned(),
            property,
            keyframes: vec![
                MotionKeyframe {
                    at: RationalTime::ZERO,
                    value,
                },
                MotionKeyframe {
                    at: RationalTime::new(1, 1).unwrap(),
                    value,
                },
            ],
            easing: MotionEasing::Linear,
        }
    }

    #[test]
    fn moving_a_node_sets_its_anchor_and_clamps_to_the_canvas() {
        let edited = apply_scene_edit(
            &scene(),
            &MotionSceneEdit::Move {
                node_id: "title".to_owned(),
                x: 640.0,
                y: -40.0,
            },
        )
        .unwrap();
        let title = &edited.nodes[0];
        assert_eq!(title.position, Some(MotionPoint { x: 640.0, y: 0.0 }));

        let far = apply_scene_edit(
            &scene(),
            &MotionSceneEdit::Move {
                node_id: "title".to_owned(),
                x: 9000.0,
                y: 9000.0,
            },
        )
        .unwrap();
        assert_eq!(
            far.nodes[0].position,
            Some(MotionPoint {
                x: 1920.0,
                y: 1080.0
            })
        );
    }

    #[test]
    fn moving_an_animated_node_subtracts_the_track_so_it_lands_under_the_pointer() {
        let mut animated = scene();
        animated.animations = vec![
            translation("title", MotionProperty::TranslationX, 200.0),
            translation("title", MotionProperty::TranslationY, 60.0),
        ];
        // The picture shows the title at (300, 260); the anchor is (100, 200).
        let boxes = scene_node_boxes(&animated);
        assert_eq!(boxes[0].position, MotionPoint { x: 300.0, y: 260.0 });

        let edited = apply_scene_edit(
            &animated,
            &MotionSceneEdit::Move {
                node_id: "title".to_owned(),
                x: 300.0,
                y: 260.0,
            },
        )
        .unwrap();
        let anchor = edited.nodes[0].position.unwrap();
        assert_eq!(anchor, MotionPoint { x: 100.0, y: 200.0 });
        assert_eq!(
            scene_node_boxes(&edited)[0].position,
            MotionPoint { x: 300.0, y: 260.0 }
        );
    }

    #[test]
    fn a_multi_node_move_places_every_node_at_its_target() {
        let edited = apply_scene_edit(
            &scene(),
            &MotionSceneEdit::MoveMany {
                nodes: vec![
                    MotionNodeTarget {
                        node_id: "title".to_owned(),
                        x: 500.0,
                        y: 300.0,
                    },
                    MotionNodeTarget {
                        node_id: "card-body".to_owned(),
                        x: 100.0,
                        y: 120.0,
                    },
                ],
            },
        )
        .unwrap();
        assert_eq!(
            edited.nodes[0].position,
            Some(MotionPoint { x: 500.0, y: 300.0 })
        );
        let body = &edited.nodes[1].children.as_ref().unwrap()[0];
        assert_eq!(body.position, Some(MotionPoint { x: 100.0, y: 120.0 }));

        let empty =
            apply_scene_edit(&scene(), &MotionSceneEdit::MoveMany { nodes: vec![] }).unwrap_err();
        assert_eq!(empty.code(), "MOTION_EDIT_EMPTY_SELECTION");
    }

    #[test]
    fn resizing_a_shape_sets_its_evaluated_size_and_refuses_text() {
        let resized = apply_scene_edit(
            &scene(),
            &MotionSceneEdit::Resize {
                node_id: "card-body".to_owned(),
                width: 480.0,
                height: 240.0,
                x: None,
                y: None,
            },
        )
        .unwrap();
        assert_eq!(
            resized.nodes[1].children.as_ref().unwrap()[0].size,
            Some(MotionSize {
                width: 480.0,
                height: 240.0
            })
        );

        let text = apply_scene_edit(
            &scene(),
            &MotionSceneEdit::Resize {
                node_id: "title".to_owned(),
                width: 10.0,
                height: 10.0,
                x: None,
                y: None,
            },
        )
        .unwrap_err();
        assert_eq!(text.code(), "MOTION_EDIT_NOT_RESIZABLE");

        let zero = apply_scene_edit(
            &scene(),
            &MotionSceneEdit::Resize {
                node_id: "card-body".to_owned(),
                width: 0.0,
                height: 10.0,
                x: None,
                y: None,
            },
        )
        .unwrap_err();
        assert_eq!(zero.code(), "MOTION_EDIT_INVALID_VALUE");
    }

    #[test]
    fn resizing_divides_out_a_scale_track_and_refuses_an_animated_size() {
        let mut scaled = scene();
        scaled.animations = vec![translation("card-body", MotionProperty::Scale, 2.0)];
        let resized = apply_scene_edit(
            &scaled,
            &MotionSceneEdit::Resize {
                node_id: "card-body".to_owned(),
                width: 640.0,
                height: 360.0,
                x: None,
                y: None,
            },
        )
        .unwrap();
        // The base size is halved so the evaluated size lands on the target.
        assert_eq!(
            resized.nodes[1].children.as_ref().unwrap()[0].size,
            Some(MotionSize {
                width: 320.0,
                height: 180.0
            })
        );
        assert_eq!(
            scene_node_boxes(&resized)[2].size,
            MotionSize {
                width: 640.0,
                height: 360.0
            }
        );

        let mut animated_width = scene();
        animated_width.animations = vec![translation("card-body", MotionProperty::Width, 100.0)];
        let refused = apply_scene_edit(
            &animated_width,
            &MotionSceneEdit::Resize {
                node_id: "card-body".to_owned(),
                width: 200.0,
                height: 100.0,
                x: None,
                y: None,
            },
        )
        .unwrap_err();
        assert_eq!(refused.code(), "MOTION_EDIT_ANIMATED_SIZE");
    }

    #[test]
    fn a_positioned_resize_moves_the_anchor_with_the_corner() {
        let resized = apply_scene_edit(
            &scene(),
            &MotionSceneEdit::Resize {
                node_id: "card-body".to_owned(),
                width: 200.0,
                height: 100.0,
                x: Some(60.0),
                y: Some(70.0),
            },
        )
        .unwrap();
        let body = &resized.nodes[1].children.as_ref().unwrap()[0];
        assert_eq!(body.position, Some(MotionPoint { x: 60.0, y: 70.0 }));
        assert_eq!(
            body.size,
            Some(MotionSize {
                width: 200.0,
                height: 100.0
            })
        );
    }

    #[test]
    fn rotating_sets_the_angle_and_wraps_into_a_readable_range() {
        let rotated = apply_scene_edit(
            &scene(),
            &MotionSceneEdit::Rotate {
                node_id: "card-body".to_owned(),
                degrees: 370.0,
            },
        )
        .unwrap();
        assert_eq!(
            rotated.nodes[1].children.as_ref().unwrap()[0].rotation_degrees,
            Some(10.0)
        );

        let mut spinning = scene();
        spinning.animations = vec![translation("title", MotionProperty::Rotation, 45.0)];
        let edited = apply_scene_edit(
            &spinning,
            &MotionSceneEdit::Rotate {
                node_id: "title".to_owned(),
                degrees: 90.0,
            },
        )
        .unwrap();
        assert_eq!(edited.nodes[0].rotation_degrees, Some(45.0));
        assert_eq!(scene_node_boxes(&edited)[0].rotation_degrees, 90.0);
    }

    #[test]
    fn an_outline_can_be_set_recoloured_and_cleared() {
        let outlined = apply_scene_edit(
            &scene(),
            &MotionSceneEdit::SetStroke {
                node_id: "card-body".to_owned(),
                color: Some(red()),
                width: None,
            },
        )
        .unwrap();
        let body = &outlined.nodes[1].children.as_ref().unwrap()[0];
        assert_eq!(body.stroke, Some(red()));
        assert_eq!(body.stroke_width, Some(MOTION_DEFAULT_STROKE_WIDTH));

        let wide = apply_scene_edit(
            &outlined,
            &MotionSceneEdit::SetStroke {
                node_id: "card-body".to_owned(),
                color: Some(white()),
                width: Some(12.0),
            },
        )
        .unwrap();
        let body = &wide.nodes[1].children.as_ref().unwrap()[0];
        assert_eq!(body.stroke, Some(white()));
        assert_eq!(body.stroke_width, Some(12.0));

        let cleared = apply_scene_edit(
            &wide,
            &MotionSceneEdit::SetStroke {
                node_id: "card-body".to_owned(),
                color: None,
                width: None,
            },
        )
        .unwrap();
        let body = &cleared.nodes[1].children.as_ref().unwrap()[0];
        assert_eq!(body.stroke, None);
        assert_eq!(body.stroke_width, None);

        let text = apply_scene_edit(
            &scene(),
            &MotionSceneEdit::SetStroke {
                node_id: "title".to_owned(),
                color: Some(red()),
                width: None,
            },
        )
        .unwrap_err();
        assert_eq!(text.code(), "MOTION_EDIT_NOT_STROKEABLE");
    }

    #[test]
    fn a_selection_can_be_recoloured_and_deleted_without_animations_left_behind() {
        let mut animated = scene();
        animated.animations = vec![translation("card-body", MotionProperty::Opacity, 0.5)];
        let coloured = apply_scene_edit(
            &animated,
            &MotionSceneEdit::SetFillMany {
                node_ids: vec!["title".to_owned(), "card-body".to_owned()],
                color: red(),
            },
        )
        .unwrap();
        assert_eq!(coloured.nodes[0].fill, Some(red()));
        assert_eq!(
            coloured.nodes[1].children.as_ref().unwrap()[0].fill,
            Some(red())
        );

        let group = apply_scene_edit(
            &scene(),
            &MotionSceneEdit::SetFillMany {
                node_ids: vec!["card".to_owned()],
                color: red(),
            },
        )
        .unwrap_err();
        assert_eq!(group.code(), "MOTION_EDIT_NOT_DRAWABLE");

        let deleted = apply_scene_edit(
            &animated,
            &MotionSceneEdit::DeleteMany {
                node_ids: vec![
                    "card-body".to_owned(),
                    "title".to_owned(),
                    "card-body".to_owned(),
                ],
            },
        )
        .unwrap();
        // The group itself was not named, so it stays with its children removed.
        assert_eq!(deleted.nodes.len(), 1);
        assert_eq!(deleted.nodes[0].node_id, "card");
        assert!(deleted.nodes[0].children.as_ref().unwrap().is_empty());
        assert!(deleted.animations.is_empty());

        let unknown = apply_scene_edit(
            &scene(),
            &MotionSceneEdit::DeleteMany {
                node_ids: vec!["title".to_owned(), "missing".to_owned()],
            },
        )
        .unwrap_err();
        assert_eq!(unknown.code(), "MOTION_EDIT_UNKNOWN_NODE");
    }

    #[test]
    fn editing_text_clears_highlight_runs_that_would_point_at_old_bytes() {
        let mut start = scene();
        start.nodes[0].spans = Some(vec![crate::MotionTextSpan {
            start: 0,
            end: 5,
            color: red(),
        }]);
        let edited = apply_scene_edit(
            &start,
            &MotionSceneEdit::SetText {
                node_id: "title".to_owned(),
                text: "A much longer replacement".to_owned(),
            },
        )
        .unwrap();
        assert_eq!(
            edited.nodes[0].text.as_deref(),
            Some("A much longer replacement")
        );
        assert_eq!(edited.nodes[0].spans, None);
    }

    #[test]
    fn the_text_operation_refuses_a_shape_and_font_size_refuses_zero() {
        let wrong_kind = apply_scene_edit(
            &scene(),
            &MotionSceneEdit::SetText {
                node_id: "card-body".to_owned(),
                text: "nope".to_owned(),
            },
        )
        .unwrap_err();
        assert_eq!(wrong_kind.code(), "MOTION_EDIT_NOT_TEXT");

        let zero = apply_scene_edit(
            &scene(),
            &MotionSceneEdit::SetFontSize {
                node_id: "title".to_owned(),
                font_size: 0.0,
            },
        )
        .unwrap_err();
        assert_eq!(zero.code(), "MOTION_EDIT_INVALID_VALUE");
    }

    #[test]
    fn deleting_a_nested_node_also_removes_its_animations() {
        let mut animated = scene();
        animated.animations = vec![translation("card-body", MotionProperty::Opacity, 0.5)];
        let edited = apply_scene_edit(
            &animated,
            &MotionSceneEdit::Delete {
                node_id: "card-body".to_owned(),
            },
        )
        .unwrap();
        assert!(edited.nodes[1].children.as_ref().unwrap().is_empty());
        assert!(edited.animations.is_empty());
    }

    #[test]
    fn adding_text_appends_a_node_that_validates_and_gets_a_unique_id() {
        let added = apply_scene_edit(
            &scene(),
            &MotionSceneEdit::AddText {
                node_id: None,
                text: "Second line".to_owned(),
                x: 120.0,
                y: 480.0,
                font_size: Some(48.0),
                fill: None,
            },
        )
        .unwrap();
        assert_eq!(added.nodes.len(), 3);
        let node = added.nodes.last().unwrap();
        assert_eq!(node.node_id, "text-1");
        assert_eq!(node.kind, MotionNodeKind::Text);
        assert_eq!(node.font_size, Some(48.0));

        let duplicate = apply_scene_edit(
            &scene(),
            &MotionSceneEdit::AddText {
                node_id: Some("title".to_owned()),
                text: "Clash".to_owned(),
                x: 0.0,
                y: 0.0,
                font_size: None,
                fill: None,
            },
        )
        .unwrap_err();
        assert_eq!(duplicate.code(), "MOTION_EDIT_DUPLICATE_NODE");
    }

    #[test]
    fn an_unknown_node_and_malformed_json_return_codes() {
        let unknown = apply_scene_edit(
            &scene(),
            &MotionSceneEdit::Delete {
                node_id: "missing".to_owned(),
            },
        )
        .unwrap_err();
        assert_eq!(unknown.code(), "MOTION_EDIT_UNKNOWN_NODE");

        assert_eq!(
            parse_scene_edit(b"not json").unwrap_err().code(),
            "MOTION_EDIT_INVALID_JSON"
        );
        assert_eq!(
            parse_scene_edit(br#"{"op":"explode"}"#).unwrap_err().code(),
            "MOTION_EDIT_INVALID_JSON"
        );
        let oversized = vec![b' '; MAX_MOTION_EDIT_BYTES + 1];
        assert_eq!(
            parse_scene_edit(&oversized).unwrap_err().code(),
            "MOTION_EDIT_TOO_LARGE"
        );
    }

    #[test]
    fn too_many_nodes_in_one_selection_is_refused_before_anything_changes() {
        let too_many: Vec<String> = (0..=MAX_MOTION_EDIT_NODES)
            .map(|i| format!("n{i}"))
            .collect();
        let refused = apply_scene_edit(
            &scene(),
            &MotionSceneEdit::DeleteMany { node_ids: too_many },
        )
        .unwrap_err();
        assert_eq!(refused.code(), "MOTION_EDIT_SELECTION_TOO_LARGE");
    }

    #[test]
    fn boxes_report_where_the_first_frame_puts_each_node() {
        let boxes = scene_node_boxes(&scene());
        assert_eq!(boxes.len(), 3);
        let title = &boxes[0];
        assert_eq!(title.node_id, "title");
        assert_eq!(title.kind, MotionNodeKind::Text);
        assert_eq!(title.position, MotionPoint { x: 100.0, y: 200.0 });
        assert!(title.size.width > 0.0 && title.size.height > 0.0);
        assert!(!title.animated);
        assert!(!title.is_resizeable());
        // A group draws nothing, so it carries no handle box of its own.
        assert_eq!(boxes[1].node_id, "card");
        assert_eq!(boxes[1].size.width, 0.0);
        assert_eq!(boxes[2].node_id, "card-body");
        assert_eq!(
            boxes[2].size,
            MotionSize {
                width: 320.0,
                height: 180.0
            }
        );
        assert!(boxes[2].is_resizeable());
        assert!(boxes[2].is_strokeable());
    }

    #[test]
    fn a_translation_track_moves_the_handle_but_the_anchor_stays_editable() {
        let mut animated = scene();
        animated.animations = vec![translation("title", MotionProperty::TranslationX, 20.0)];
        let boxes = scene_node_boxes(&animated);
        assert!(boxes[0].animated);
        assert_eq!(boxes[0].position.x, 120.0);
        assert_eq!(animated.nodes[0].position.unwrap().x, 100.0);
    }

    #[test]
    fn a_box_reports_the_outline_and_rotation_the_renderer_will_draw() {
        let mut outlined = scene();
        outlined.nodes[1].children.as_mut().unwrap()[0].stroke = Some(red());
        outlined.nodes[1].children.as_mut().unwrap()[0].stroke_width = Some(6.0);
        outlined.nodes[1].children.as_mut().unwrap()[0].rotation_degrees = Some(-30.0);
        let boxes = scene_node_boxes(&outlined);
        assert_eq!(boxes[2].stroke, Some(red()));
        assert_eq!(boxes[2].stroke_width, 6.0);
        assert_eq!(boxes[2].rotation_degrees, -30.0);
    }
}
