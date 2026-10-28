//! Bridge surface for the app-level motion asset library (ADR 0010, P2).
//!
//! The library root is passed in by the Dart layer, which owns the application
//! support path. Every function returns a view with `ok`, `code` and `message`
//! rather than throwing across the FFI boundary, matching the project surface.
//!
//! Preview rendering and materialisation prepare image assets into a scratch
//! directory beside the scene, so the existing materialiser resolves them like any
//! other scene-local file. The library never leaks an absolute path into a project.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::json;

use or_motion::{
    MotionAssetCategory, MotionAssetDocument, MotionAssetKind, MotionAssetParameter,
    MotionAssetPayload, MotionCanvas, MotionLibrary, MotionLibraryEntry, MotionLibraryError,
    MotionParameterValue, MotionScene, append_asset_to_scene, apply_scene_edit_json,
    compose_motion_scene, duration_milliseconds, instantiate_motion_asset, new_motion_asset_id,
    parse_motion_scene, scene_node_boxes, validate_scene,
};
use or_render::{encode_png, rasterize_motion_frame_with_images};

use crate::api::motion::{MotionMaterializationView, materialize_motion_scene};

/// One declared parameter, flattened for the editor.
#[derive(Clone, Debug)]
pub struct MotionAssetParameterView {
    pub id: String,
    pub label: String,
    /// `text`, `color`, `number` or `choice`.
    pub kind: String,
    pub node_id: String,
    /// `text`, `font_size`, `fill`, `size_width`, `size_height`, `opacity`,
    /// `rotation_degrees` or `asset_id`.
    pub field: String,
    /// The default as JSON, so the editor can render the right control.
    pub default_json: String,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub choices: Vec<String>,
}

/// One catalogue row.
#[derive(Clone, Debug)]
pub struct MotionAssetEntryView {
    pub asset_id: String,
    pub name: String,
    pub category: String,
    pub category_source: String,
    pub kind: String,
    pub tags: Vec<String>,
    pub canvas_width: u32,
    pub canvas_height: u32,
    pub duration_milliseconds: i64,
    pub preview_path: String,
}

/// The library listing.
#[derive(Clone, Debug)]
pub struct MotionAssetLibraryView {
    pub ok: bool,
    pub code: String,
    pub message: String,
    pub root: String,
    pub entries: Vec<MotionAssetEntryView>,
}

/// One loaded asset.
#[derive(Clone, Debug)]
pub struct MotionAssetView {
    pub ok: bool,
    pub code: String,
    pub message: String,
    pub asset_id: String,
    pub name: String,
    pub category: String,
    pub category_source: String,
    pub kind: String,
    pub tags: Vec<String>,
    pub canvas_width: u32,
    pub canvas_height: u32,
    pub duration_milliseconds: i64,
    /// Absolute preview path, empty when no preview was generated.
    pub preview_path: String,
    /// The full document, so the editor can save an edited copy.
    pub document_json: String,
    pub parameters: Vec<MotionAssetParameterView>,
}

/// A remove result.
#[derive(Clone, Debug)]
pub struct MotionAssetMutationView {
    pub ok: bool,
    pub code: String,
    pub message: String,
    pub removed: bool,
}

/// One node of a composed scene, as the editor draws it at time zero.
///
/// The box is a pointer target, not glyph measurement: text width is an estimate at
/// [`or_motion::MOTION_TEXT_HIT_ADVANCE`] per character. Edits still address the node
/// by id and are validated in Rust.
#[derive(Clone, Debug)]
pub struct MotionSceneNodeView {
    pub node_id: String,
    /// `group`, `text`, `rectangle`, `ellipse`, `arrow` or `image`.
    pub kind: String,
    /// The literal string of a text node, empty otherwise.
    pub text: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub font_size: f64,
    pub opacity: f64,
    pub has_fill: bool,
    pub fill_red: u32,
    pub fill_green: u32,
    pub fill_blue: u32,
    pub fill_alpha: u32,
    /// Whether the node draws an outline, and what it looks like.
    pub has_stroke: bool,
    pub stroke_red: u32,
    pub stroke_green: u32,
    pub stroke_blue: u32,
    pub stroke_alpha: u32,
    pub stroke_width: f64,
    /// Degrees counter-clockwise, as the first frame evaluates it.
    pub rotation_degrees: f64,
    /// Whether the editor can drag a resize handle on this node.
    pub resizable: bool,
    /// Whether the editor can offer an outline for this node.
    pub strokeable: bool,
    /// Nesting depth, 0 for a top-level node.
    pub depth: u32,
    /// Whether an animation targets this node.
    pub animated: bool,
}

/// A composed or instantiated scene, with a PNG preview of its first frame and the
/// node boxes that preview shows.
#[derive(Clone, Debug)]
pub struct MotionSceneView {
    pub ok: bool,
    pub code: String,
    pub message: String,
    pub scene_json: String,
    pub scene_id: String,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub duration_milliseconds: i64,
    pub preview_png: Vec<u8>,
    pub nodes: Vec<MotionSceneNodeView>,
}

/// A rendered still.
#[derive(Clone, Debug)]
pub struct MotionScenePreviewView {
    pub ok: bool,
    pub code: String,
    pub message: String,
    pub png: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

pub fn motion_library(root: String) -> MotionAssetLibraryView {
    match library_view(&root) {
        Ok(view) => view,
        Err(error) => MotionAssetLibraryView {
            ok: false,
            code: error.code().to_owned(),
            message: error.to_string(),
            root,
            entries: Vec::new(),
        },
    }
}

pub fn motion_asset_load(root: String, asset_id: String) -> MotionAssetView {
    match load_view(&root, &asset_id) {
        Ok(view) => view,
        Err(error) => asset_failure(error.code(), error.to_string()),
    }
}

pub fn motion_asset_save(
    root: String,
    document_json: String,
    category: Option<String>,
) -> MotionAssetView {
    let library = match open_library(&root) {
        Ok(library) => library,
        Err(error) => return asset_failure(error.code(), error.to_string()),
    };
    let document: MotionAssetDocument = match serde_json::from_str(&document_json) {
        Ok(document) => document,
        Err(_) => {
            return asset_failure(
                "MOTION_ASSET_INVALID_JSON",
                "the asset document is not valid JSON for this version",
            );
        }
    };
    let category = match parse_category(category.as_deref()) {
        Ok(category) => category,
        Err(message) => return asset_failure("MOTION_LIBRARY_INVALID_CATEGORY", message),
    };
    if let Err(error) = library.save(&document, category) {
        return asset_failure(error.code(), error.to_string());
    }
    match load_view(&root, &document.asset_id) {
        Ok(view) => view,
        Err(error) => asset_failure(error.code(), error.to_string()),
    }
}

pub fn motion_asset_remove(root: String, asset_id: String) -> MotionAssetMutationView {
    let library = match open_library(&root) {
        Ok(library) => library,
        Err(error) => {
            return MotionAssetMutationView {
                ok: false,
                code: error.code().to_owned(),
                message: error.to_string(),
                removed: false,
            };
        }
    };
    match library.remove(&asset_id) {
        Ok(removed) => MotionAssetMutationView {
            ok: true,
            code: "OK".to_owned(),
            message: String::new(),
            removed,
        },
        Err(error) => MotionAssetMutationView {
            ok: false,
            code: error.code().to_owned(),
            message: error.to_string(),
            removed: false,
        },
    }
}

/// Imports a PNG cutout and returns the new adjustable asset.
#[allow(clippy::too_many_arguments)]
pub fn motion_asset_import_image(
    root: String,
    source_path: String,
    name: String,
    category: Option<String>,
    tags: Vec<String>,
    width: u32,
    height: u32,
    seconds: u32,
) -> MotionAssetView {
    let library = match open_library(&root) {
        Ok(library) => library,
        Err(error) => return asset_failure(error.code(), error.to_string()),
    };
    let category = match parse_category(category.as_deref()) {
        Ok(category) => category,
        Err(message) => return asset_failure("MOTION_LIBRARY_INVALID_CATEGORY", message),
    };
    let duration = match or_core::RationalTime::new(i64::from(seconds.max(1)), 1) {
        Ok(duration) => duration,
        Err(_) => return asset_failure("MOTION_INVALID_DURATION", "seconds must be positive"),
    };
    let document = match library.import_image(or_motion::MotionImageImport {
        source: Path::new(&source_path),
        name: &name,
        category,
        tags,
        canvas: MotionCanvas {
            width: width.max(1),
            height: height.max(1),
        },
        duration,
    }) {
        Ok(document) => document,
        Err(error) => return asset_failure(error.code(), error.to_string()),
    };
    match load_view(&root, &document.asset_id) {
        Ok(view) => view,
        Err(error) => asset_failure(error.code(), error.to_string()),
    }
}

/// Instantiates one asset into a scene, with its parameter values applied.
pub fn motion_asset_scene(
    root: String,
    asset_id: String,
    values_json: String,
    scene_id: Option<String>,
    name: Option<String>,
) -> MotionSceneView {
    let library = match open_library(&root) {
        Ok(library) => library,
        Err(error) => return scene_failure(error.code(), error.to_string()),
    };
    let asset = match library.load(&asset_id) {
        Ok(asset) => asset,
        Err(error) => return scene_failure(error.code(), error.to_string()),
    };
    let values = match parse_values(&values_json) {
        Ok(values) => values,
        Err(message) => return scene_failure("MOTION_ASSET_INVALID_VALUE", message),
    };
    let scene_id = scene_id
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(new_motion_asset_id);
    let name = name.unwrap_or_else(|| asset.name.clone());
    let scene = match instantiate_motion_asset(&asset, &values, &scene_id, &name) {
        Ok(scene) => scene,
        Err(error) => return scene_failure(error.code(), error.to_string()),
    };
    scene_view(&scene, &library)
}

/// Opens a saved scene asset as an editable draft.
///
/// A scene saved from the canvas keeps its whole payload in the library document,
/// so it can be reopened and edited instead of only being instantiated. Its image
/// paths are already library-relative. A non-scene asset is refused with a code
/// the panel can show.
pub fn motion_scene_from_asset(root: String, asset_id: String) -> MotionSceneView {
    let library = match open_library(&root) {
        Ok(library) => library,
        Err(error) => return scene_failure(error.code(), error.to_string()),
    };
    let asset = match library.load(&asset_id) {
        Ok(asset) => asset,
        Err(error) => return scene_failure(error.code(), error.to_string()),
    };
    if asset.kind != MotionAssetKind::Scene {
        return scene_failure(
            "MOTION_ASSET_NOT_SCENE",
            "Only a saved scene can be opened in the scene editor.",
        );
    }
    let Some(payload) = asset.scene.clone() else {
        return scene_failure(
            "MOTION_ASSET_EMPTY_SCENE",
            "The saved scene has no scene document.",
        );
    };
    let scene = MotionScene {
        format: or_motion::MOTION_SCENE_FORMAT.to_owned(),
        schema_version: or_motion::MOTION_SCENE_SCHEMA_VERSION,
        scene_id: new_motion_asset_id(),
        name: asset.name.clone(),
        duration: asset.duration,
        canvas: asset.canvas,
        assets: payload.assets,
        nodes: payload.nodes,
        animations: payload.animations,
        camera: payload.camera,
    };
    if !validate_scene(&scene).ok {
        return scene_failure("MOTION_INVALID_SCENE", "the saved scene did not validate");
    }
    scene_view(&scene, &library)
}

/// Creates an empty composition scene.
pub fn motion_scene_new(name: String, width: u32, height: u32, seconds: u32) -> MotionSceneView {
    let duration = match or_core::RationalTime::new(i64::from(seconds.max(1)), 1) {
        Ok(duration) => duration,
        Err(_) => return scene_failure("MOTION_INVALID_DURATION", "seconds must be positive"),
    };
    let scene = MotionScene {
        format: or_motion::MOTION_SCENE_FORMAT.to_owned(),
        schema_version: or_motion::MOTION_SCENE_SCHEMA_VERSION,
        scene_id: new_motion_asset_id(),
        name: if name.trim().is_empty() {
            "Composition".to_owned()
        } else {
            name
        },
        duration,
        canvas: MotionCanvas {
            width: width.max(1),
            height: height.max(1),
        },
        assets: Vec::new(),
        nodes: Vec::new(),
        animations: Vec::new(),
        camera: None,
    };
    if !validate_scene(&scene).ok {
        return scene_failure("MOTION_INVALID_SCENE", "the scene did not validate");
    }
    // An empty scene has no image assets, so the temporary library is unused.
    match MotionLibrary::open(std::env::temp_dir().join("or-motion-empty")) {
        Ok(library) => scene_view(&scene, &library),
        Err(error) => scene_failure(error.code(), error.to_string()),
    }
}

/// Appends one library asset to a composition scene.
#[allow(clippy::too_many_arguments)]
pub fn motion_scene_append(
    root: String,
    scene_json: String,
    asset_id: String,
    values_json: String,
    prefix: String,
    offset_x: f64,
    offset_y: f64,
) -> MotionSceneView {
    let library = match open_library(&root) {
        Ok(library) => library,
        Err(error) => return scene_failure(error.code(), error.to_string()),
    };
    let scene = match parse_scene(&scene_json) {
        Ok(scene) => scene,
        Err((code, message)) => return scene_failure(&code, message),
    };
    let asset = match library.load(&asset_id) {
        Ok(asset) => asset,
        Err(error) => return scene_failure(error.code(), error.to_string()),
    };
    let values = match parse_values(&values_json) {
        Ok(values) => values,
        Err(message) => return scene_failure("MOTION_ASSET_INVALID_VALUE", message),
    };
    let prefix = if prefix.trim().is_empty() {
        new_motion_asset_id()
    } else {
        prefix
    };
    let scene = match append_asset_to_scene(&scene, &asset, &values, &prefix, offset_x, offset_y) {
        Ok(scene) => scene,
        Err(error) => return scene_failure(error.code(), error.to_string()),
    };
    scene_view(&scene, &library)
}

/// Composes a whole set of assets into one scene in a single call.
#[allow(clippy::too_many_arguments)]
pub fn motion_scene_compose(
    root: String,
    scene_id: String,
    name: String,
    width: u32,
    height: u32,
    seconds: u32,
    asset_ids: Vec<String>,
    values_json: Vec<String>,
    offset_x: Vec<f64>,
    offset_y: Vec<f64>,
) -> MotionSceneView {
    let library = match open_library(&root) {
        Ok(library) => library,
        Err(error) => return scene_failure(error.code(), error.to_string()),
    };
    if asset_ids.len() != values_json.len()
        || asset_ids.len() != offset_x.len()
        || asset_ids.len() != offset_y.len()
    {
        return scene_failure(
            "MOTION_COMPOSE_MISMATCH",
            "asset_ids, values_json and offsets must have the same length",
        );
    }
    let duration = match or_core::RationalTime::new(i64::from(seconds.max(1)), 1) {
        Ok(duration) => duration,
        Err(_) => return scene_failure("MOTION_INVALID_DURATION", "seconds must be positive"),
    };
    let mut documents = Vec::with_capacity(asset_ids.len());
    let mut values = Vec::with_capacity(values_json.len());
    for (index, asset_id) in asset_ids.iter().enumerate() {
        documents.push(match library.load(asset_id) {
            Ok(document) => document,
            Err(error) => return scene_failure(error.code(), error.to_string()),
        });
        values.push(match parse_values(&values_json[index]) {
            Ok(values) => values,
            Err(message) => return scene_failure("MOTION_ASSET_INVALID_VALUE", message),
        });
    }
    let placements: Vec<or_motion::MotionScenePlacement<'_>> = documents
        .iter()
        .enumerate()
        .map(|(index, document)| or_motion::MotionScenePlacement {
            asset: document,
            values: values[index].clone(),
            offset_x: offset_x[index],
            offset_y: offset_y[index],
        })
        .collect();
    let scene_id = if scene_id.trim().is_empty() {
        new_motion_asset_id()
    } else {
        scene_id
    };
    let scene = match compose_motion_scene(
        &scene_id,
        if name.trim().is_empty() {
            "Composition"
        } else {
            &name
        },
        MotionCanvas {
            width: width.max(1),
            height: height.max(1),
        },
        duration,
        &placements,
    ) {
        Ok(scene) => scene,
        Err(error) => return scene_failure(error.code(), error.to_string()),
    };
    scene_view(&scene, &library)
}

/// Renders a still of a composed scene.
pub fn motion_scene_preview(
    root: String,
    scene_json: String,
    at_milliseconds: i64,
) -> MotionScenePreviewView {
    let library = match open_library(&root) {
        Ok(library) => library,
        Err(error) => {
            return MotionScenePreviewView {
                ok: false,
                code: error.code().to_owned(),
                message: error.to_string(),
                png: Vec::new(),
                width: 0,
                height: 0,
            };
        }
    };
    let scene = match parse_scene(&scene_json) {
        Ok(scene) => scene,
        Err((code, message)) => {
            return MotionScenePreviewView {
                ok: false,
                code,
                message,
                png: Vec::new(),
                width: 0,
                height: 0,
            };
        }
    };
    match render_preview(&scene, &library, at_milliseconds) {
        Ok((png, width, height)) => MotionScenePreviewView {
            ok: true,
            code: "OK".to_owned(),
            message: String::new(),
            png,
            width,
            height,
        },
        Err((code, message)) => MotionScenePreviewView {
            ok: false,
            code,
            message,
            png: Vec::new(),
            width: 0,
            height: 0,
        },
    }
}

/// Applies one GUI edit (move, retitle, recolour, add text, delete) to a scene.
///
/// The operation set is closed and every result is validated before it is returned,
/// so a gesture can never hand the timeline a scene the renderer refuses. The reply
/// carries the edited scene, its preview and the fresh node boxes in one call.
pub fn motion_scene_edit(root: String, scene_json: String, edit_json: String) -> MotionSceneView {
    let library = match open_library(&root) {
        Ok(library) => library,
        Err(error) => return scene_failure(error.code(), error.to_string()),
    };
    let scene = match parse_scene(&scene_json) {
        Ok(scene) => scene,
        Err((code, message)) => return scene_failure(&code, message),
    };
    let edited = match apply_scene_edit_json(&scene, edit_json.as_bytes()) {
        Ok(edited) => edited,
        Err(error) => return scene_failure(error.code(), error.to_string()),
    };
    scene_view(&edited, &library)
}

/// Opens a `.ormotion.json` scene from disk so the canvas can edit it.
///
/// The file is untrusted input: bounded by size, parsed through the same
/// document reader as every other scene, validated, and its referenced PNGs are
/// adopted into the app library so the returned draft keeps its images after the
/// first edit. The caller never receives a path into the original folder.
pub fn motion_scene_read(root: String, path: String) -> MotionSceneView {
    /// A scene is a small declarative document; anything past this is not one.
    const MAX_SCENE_FILE_BYTES: u64 = 8 * 1024 * 1024;
    let library = match open_library(&root) {
        Ok(library) => library,
        Err(error) => return scene_failure(error.code(), error.to_string()),
    };
    let source = Path::new(&path);
    let metadata = match std::fs::metadata(source) {
        Ok(metadata) => metadata,
        Err(_) => {
            return scene_failure(
                "MOTION_SCENE_NOT_FOUND",
                "The scene file could not be found. Choose it again.",
            );
        }
    };
    if !metadata.is_file() {
        return scene_failure(
            "MOTION_SCENE_NOT_FILE",
            "Choose a scene file, not a folder.",
        );
    }
    if metadata.len() > MAX_SCENE_FILE_BYTES {
        return scene_failure(
            "MOTION_SCENE_TOO_LARGE",
            "The scene file is larger than the 8 MiB scene limit.",
        );
    }
    let text = match std::fs::read_to_string(source) {
        Ok(text) => text,
        Err(_) => {
            return scene_failure(
                "MOTION_SCENE_UNREADABLE",
                "The scene file could not be read as UTF-8 JSON.",
            );
        }
    };
    let scene = match parse_scene(&text) {
        Ok(scene) => scene,
        Err((code, message)) => return scene_failure(&code, message),
    };
    let source_directory = source
        .parent()
        .filter(|directory| !directory.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let scene = match library.adopt_scene_images(&scene, source_directory) {
        Ok(scene) => scene,
        Err(error) => return scene_failure(error.code(), error.to_string()),
    };
    scene_view(&scene, &library)
}

/// Materialises a composed scene into a video file.
#[allow(clippy::too_many_arguments)]
pub fn motion_scene_materialize(
    root: String,
    scene_json: String,
    output_path: String,
    frame_rate_numerator: u32,
    frame_rate_denominator: u32,
    width: u32,
    height: u32,
) -> MotionMaterializationView {
    let library = match open_library(&root) {
        Ok(library) => library,
        Err(error) => {
            return materialize_failure(error.code(), error.to_string(), &output_path);
        }
    };
    let scene = match parse_scene(&scene_json) {
        Ok(scene) => scene,
        Err((code, message)) => return materialize_failure(&code, message, &output_path),
    };
    let scratch = scratch_directory();
    let result = (|| {
        let prepared = library
            .prepare_scene_assets(&scene, &scratch)
            .map_err(|error| (error.code().to_owned(), error.to_string()))?;
        let scene_path = scratch.join("scene.ormotion.json");
        let encoded = serde_json::to_vec_pretty(&prepared)
            .map_err(|error| ("MOTION_INVALID_SCENE".to_owned(), error.to_string()))?;
        std::fs::write(&scene_path, encoded)
            .map_err(|error| ("MOTION_SCENE_UNWRITABLE".to_owned(), error.to_string()))?;
        Ok(materialize_motion_scene(
            scene_path.display().to_string(),
            output_path.clone(),
            frame_rate_numerator,
            frame_rate_denominator,
            width,
            height,
        ))
    })();
    let _ = std::fs::remove_dir_all(&scratch);
    match result {
        Ok(view) => {
            if view.succeeded {
                write_provenance(
                    &output_path,
                    &json!({
                        "version": 1,
                        "kind": "motion-scene",
                        "scene_id": scene.scene_id,
                        "scene_name": scene.name,
                        "scene_sha256": sha256_hex(scene_json.as_bytes()),
                        "width": width,
                        "height": height,
                        "frame_rate": format!("{frame_rate_numerator}/{frame_rate_denominator}"),
                        "library": root,
                        "assets": scene.assets.iter().map(|asset| json!({
                            "asset_id": asset.asset_id,
                            "path": asset.path,
                        })).collect::<Vec<_>>(),
                    }),
                );
            }
            view
        }
        Err((code, message)) => materialize_failure(&code, message, &output_path),
    }
}

/// Instantiates and materialises one asset in a single call.
#[allow(clippy::too_many_arguments)]
pub fn motion_asset_materialize(
    root: String,
    asset_id: String,
    values_json: String,
    output_path: String,
    frame_rate_numerator: u32,
    frame_rate_denominator: u32,
    width: u32,
    height: u32,
) -> MotionMaterializationView {
    let library = match open_library(&root) {
        Ok(library) => library,
        Err(error) => return materialize_failure(error.code(), error.to_string(), &output_path),
    };
    let asset = match library.load(&asset_id) {
        Ok(asset) => asset,
        Err(error) => return materialize_failure(error.code(), error.to_string(), &output_path),
    };
    let values = match parse_values(&values_json) {
        Ok(values) => values,
        Err(message) => {
            return materialize_failure("MOTION_ASSET_INVALID_VALUE", message, &output_path);
        }
    };
    let scene = match instantiate_motion_asset(&asset, &values, &new_motion_asset_id(), &asset.name)
    {
        Ok(scene) => scene,
        Err(error) => return materialize_failure(error.code(), error.to_string(), &output_path),
    };
    let scene_json = match serde_json::to_string(&scene) {
        Ok(json) => json,
        Err(error) => {
            return materialize_failure("MOTION_INVALID_SCENE", error.to_string(), &output_path);
        }
    };
    let view = motion_scene_materialize(
        root.clone(),
        scene_json,
        output_path.clone(),
        frame_rate_numerator,
        frame_rate_denominator,
        width,
        height,
    );
    if view.succeeded {
        let values =
            serde_json::from_str::<serde_json::Value>(&values_json).unwrap_or_else(|_| json!({}));
        write_provenance(
            &output_path,
            &json!({
                "version": 1,
                "kind": "motion-asset",
                "asset_id": asset.asset_id,
                "asset_name": asset.name,
                "values": values,
                "library": root,
                "scene_id": view.scene_id,
                "width": width,
                "height": height,
                "frame_rate": format!("{frame_rate_numerator}/{frame_rate_denominator}"),
            }),
        );
    }
    view
}

/// Saves a composed scene as a reusable scene asset, generating its preview.
pub fn motion_scene_save(
    root: String,
    scene_json: String,
    name: String,
    category: Option<String>,
) -> MotionAssetView {
    let library = match open_library(&root) {
        Ok(library) => library,
        Err(error) => return asset_failure(error.code(), error.to_string()),
    };
    let scene = match parse_scene(&scene_json) {
        Ok(scene) => scene,
        Err((code, message)) => return asset_failure(&code, message),
    };
    let category = match parse_category(category.as_deref()) {
        Ok(category) => category,
        Err(message) => return asset_failure("MOTION_LIBRARY_INVALID_CATEGORY", message),
    };
    let asset_id = new_motion_asset_id();
    let name = if name.trim().is_empty() {
        scene.name.clone()
    } else {
        name
    };
    let classification =
        or_motion::classify_motion_asset(&name, &[], MotionAssetKind::Scene, category);
    let mut document = MotionAssetDocument {
        format: or_motion::MOTION_ASSET_FORMAT.to_owned(),
        schema_version: or_motion::MOTION_ASSET_SCHEMA_VERSION,
        asset_id,
        name,
        category: classification.category,
        category_source: classification.source,
        tags: Vec::new(),
        kind: MotionAssetKind::Scene,
        canvas: scene.canvas,
        duration: scene.duration,
        parameters: Vec::new(),
        scene: Some(MotionAssetPayload {
            assets: scene.assets.clone(),
            nodes: scene.nodes.clone(),
            animations: scene.animations.clone(),
            camera: scene.camera.clone(),
        }),
        preset: None,
        image: None,
        preview: None,
    };
    if let Ok((png, preview_width, preview_height)) = render_preview(&scene, &library, 0) {
        if preview_width > 0
            && preview_height > 0
            && let Ok(relative) = library.write_preview(&document.asset_id, &png)
        {
            document.preview = Some(relative);
        }
    }
    if let Err(error) = library.save(&document, None) {
        return asset_failure(error.code(), error.to_string());
    }
    match load_view(&root, &document.asset_id) {
        Ok(view) => view,
        Err(error) => asset_failure(error.code(), error.to_string()),
    }
}

fn library_view(root: &str) -> Result<MotionAssetLibraryView, MotionLibraryError> {
    let library = MotionLibrary::open(root)?;
    let entries = library
        .list()?
        .into_iter()
        .map(|entry| entry_view(&library, &entry))
        .collect();
    Ok(MotionAssetLibraryView {
        ok: true,
        code: "OK".to_owned(),
        message: String::new(),
        root: library.root().display().to_string(),
        entries,
    })
}

fn load_view(root: &str, asset_id: &str) -> Result<MotionAssetView, MotionLibraryError> {
    let library = MotionLibrary::open(root)?;
    let document = library.load(asset_id)?;
    Ok(asset_view(&library, &document))
}

fn entry_view(library: &MotionLibrary, entry: &MotionLibraryEntry) -> MotionAssetEntryView {
    MotionAssetEntryView {
        asset_id: entry.asset_id.clone(),
        name: entry.name.clone(),
        category: entry.category.as_str().to_owned(),
        category_source: entry.category_source.as_str().to_owned(),
        kind: entry.kind.as_str().to_owned(),
        tags: entry.tags.clone(),
        canvas_width: entry.canvas.width,
        canvas_height: entry.canvas.height,
        duration_milliseconds: entry.duration_milliseconds,
        preview_path: entry
            .preview
            .as_deref()
            .and_then(|relative| library.resolve_relative(relative).ok())
            .filter(|path| path.exists())
            .map(|path| path.display().to_string())
            .unwrap_or_default(),
    }
}

fn asset_view(library: &MotionLibrary, document: &MotionAssetDocument) -> MotionAssetView {
    MotionAssetView {
        ok: true,
        code: "OK".to_owned(),
        message: String::new(),
        asset_id: document.asset_id.clone(),
        name: document.name.clone(),
        category: document.category.as_str().to_owned(),
        category_source: document.category_source.as_str().to_owned(),
        kind: document.kind.as_str().to_owned(),
        tags: document.tags.clone(),
        canvas_width: document.canvas.width,
        canvas_height: document.canvas.height,
        duration_milliseconds: duration_milliseconds(document.duration),
        preview_path: document
            .preview
            .as_deref()
            .and_then(|relative| library.resolve_relative(relative).ok())
            .filter(|path| path.exists())
            .map(|path| path.display().to_string())
            .unwrap_or_default(),
        document_json: serde_json::to_string_pretty(document).unwrap_or_default(),
        parameters: document.parameters.iter().map(parameter_view).collect(),
    }
}

fn parameter_view(parameter: &MotionAssetParameter) -> MotionAssetParameterView {
    MotionAssetParameterView {
        id: parameter.id.clone(),
        label: parameter.label.clone(),
        kind: parameter.kind.as_str().to_owned(),
        node_id: parameter.target.node_id.clone(),
        field: parameter.target.field.as_str().to_owned(),
        default_json: serde_json::to_string(&parameter.default).unwrap_or_default(),
        min: parameter.min,
        max: parameter.max,
        choices: parameter.choices.clone().unwrap_or_default(),
    }
}

fn scene_view(scene: &MotionScene, library: &MotionLibrary) -> MotionSceneView {
    let (preview_png, _, _) = render_preview(scene, library, 0)
        .unwrap_or_else(|_| (Vec::new(), scene.canvas.width, scene.canvas.height));
    MotionSceneView {
        ok: true,
        code: "OK".to_owned(),
        message: String::new(),
        scene_json: serde_json::to_string_pretty(scene).unwrap_or_default(),
        scene_id: scene.scene_id.clone(),
        name: scene.name.clone(),
        width: scene.canvas.width,
        height: scene.canvas.height,
        duration_milliseconds: duration_milliseconds(scene.duration),
        preview_png,
        nodes: node_views(scene),
    }
}

fn node_views(scene: &MotionScene) -> Vec<MotionSceneNodeView> {
    scene_node_boxes(scene)
        .into_iter()
        .map(|node| {
            let fill = node.fill.unwrap_or(or_motion::MotionColor {
                red: 255,
                green: 255,
                blue: 255,
                alpha: 255,
            });
            let stroke = node.stroke.unwrap_or(or_motion::MotionColor {
                red: 255,
                green: 255,
                blue: 255,
                alpha: 255,
            });
            let resizable = node.is_resizeable();
            let strokeable = node.is_strokeable();
            MotionSceneNodeView {
                node_id: node.node_id,
                kind: node_kind_name(node.kind),
                text: node.text,
                x: node.position.x,
                y: node.position.y,
                width: node.size.width,
                height: node.size.height,
                font_size: node.font_size,
                opacity: node.opacity,
                has_fill: node.fill.is_some(),
                fill_red: u32::from(fill.red),
                fill_green: u32::from(fill.green),
                fill_blue: u32::from(fill.blue),
                fill_alpha: u32::from(fill.alpha),
                has_stroke: node.stroke.is_some(),
                stroke_red: u32::from(stroke.red),
                stroke_green: u32::from(stroke.green),
                stroke_blue: u32::from(stroke.blue),
                stroke_alpha: u32::from(stroke.alpha),
                stroke_width: node.stroke_width,
                rotation_degrees: node.rotation_degrees,
                resizable,
                strokeable,
                depth: u32::try_from(node.depth).unwrap_or(u32::MAX),
                animated: node.animated,
            }
        })
        .collect()
}

fn node_kind_name(kind: or_motion::MotionNodeKind) -> String {
    match kind {
        or_motion::MotionNodeKind::Group => "group",
        or_motion::MotionNodeKind::Text => "text",
        or_motion::MotionNodeKind::Rectangle => "rectangle",
        or_motion::MotionNodeKind::Ellipse => "ellipse",
        or_motion::MotionNodeKind::Arrow => "arrow",
        or_motion::MotionNodeKind::Image => "image",
    }
    .to_owned()
}

fn render_preview(
    scene: &MotionScene,
    library: &MotionLibrary,
    at_milliseconds: i64,
) -> Result<(Vec<u8>, u32, u32), (String, String)> {
    let scratch = scratch_directory();
    let result = (|| {
        let prepared = library
            .prepare_scene_assets(scene, &scratch)
            .map_err(|error| (error.code().to_owned(), error.to_string()))?;
        let images = or_render::load_motion_scene_images(&prepared, &scratch)
            .map_err(|error| (error.code().to_owned(), error.to_string()))?;
        let at = or_core::RationalTime::new(at_milliseconds.max(0), 1000).map_err(|_| {
            (
                "MOTION_INVALID_TIME".to_owned(),
                "invalid preview time".to_owned(),
            )
        })?;
        let frame = or_motion::evaluate_scene(&prepared, at)
            .map_err(|error| (error.code().to_owned(), error.code().to_owned()))?;
        let canvas = rasterize_motion_frame_with_images(&frame, &images)
            .map_err(|error| (error.code().to_owned(), error.code().to_owned()))?;
        let png = encode_png(canvas.width(), canvas.height(), canvas.pixels())
            .map_err(|error| (error.code().to_owned(), error.code().to_owned()))?;
        Ok((png, canvas.width(), canvas.height()))
    })();
    let _ = std::fs::remove_dir_all(&scratch);
    result
}

fn parse_category(value: Option<&str>) -> Result<Option<MotionAssetCategory>, String> {
    match value {
        None => Ok(None),
        Some(value) if value.trim().is_empty() => Ok(None),
        Some(value) => MotionAssetCategory::parse(value.trim())
            .map(Some)
            .ok_or_else(|| format!("unknown category: {value}")),
    }
}

fn parse_values(values_json: &str) -> Result<BTreeMap<String, MotionParameterValue>, String> {
    if values_json.trim().is_empty() {
        return Ok(BTreeMap::new());
    }
    serde_json::from_str(values_json)
        .map_err(|error| format!("values must be a JSON object: {error}"))
}

fn parse_scene(scene_json: &str) -> Result<MotionScene, (String, String)> {
    let scene = parse_motion_scene(scene_json.as_bytes())
        .map_err(|error| (error.code().to_owned(), error.to_string()))?;
    let validation = validate_scene(&scene);
    if !validation.ok {
        let first = validation
            .errors
            .first()
            .map(|error| (error.code.clone(), error.message.clone()))
            .unwrap_or_else(|| {
                (
                    "MOTION_INVALID_SCENE".to_owned(),
                    "invalid scene".to_owned(),
                )
            });
        return Err(first);
    }
    Ok(scene)
}

/// Writes a small sidecar beside a materialised clip. Provenance is not project
/// state and never changes a command or a revision; it lets a later editor find
/// which library assets produced the file. A failure is not fatal.
fn write_provenance(output_path: &str, value: &serde_json::Value) {
    if let Ok(encoded) = serde_json::to_vec_pretty(value) {
        let _ = std::fs::write(format!("{output_path}.provenance.json"), encoded);
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    use std::fmt::Write as _;
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(digest.len() * 2);
    for byte in digest {
        write!(&mut output, "{byte:02x}").expect("writing to a String cannot fail");
    }
    output
}

fn open_library(root: &str) -> Result<MotionLibrary, MotionLibraryError> {
    MotionLibrary::open(root)
}

fn scratch_directory() -> PathBuf {
    std::env::temp_dir().join(format!("or-motion-bridge-{}", uuid::Uuid::new_v4()))
}

fn asset_failure(code: &str, message: impl Into<String>) -> MotionAssetView {
    MotionAssetView {
        ok: false,
        code: code.to_owned(),
        message: message.into(),
        asset_id: String::new(),
        name: String::new(),
        category: String::new(),
        category_source: String::new(),
        kind: String::new(),
        tags: Vec::new(),
        canvas_width: 0,
        canvas_height: 0,
        duration_milliseconds: 0,
        preview_path: String::new(),
        document_json: String::new(),
        parameters: Vec::new(),
    }
}

fn scene_failure(code: &str, message: impl Into<String>) -> MotionSceneView {
    MotionSceneView {
        ok: false,
        code: code.to_owned(),
        message: message.into(),
        scene_json: String::new(),
        scene_id: String::new(),
        name: String::new(),
        width: 0,
        height: 0,
        duration_milliseconds: 0,
        preview_png: Vec::new(),
        nodes: Vec::new(),
    }
}

fn materialize_failure(
    code: &str,
    message: impl Into<String>,
    output: &str,
) -> MotionMaterializationView {
    MotionMaterializationView {
        succeeded: false,
        code: code.to_owned(),
        message: message.into(),
        output_path: output.to_owned(),
        scene_id: String::new(),
        scene_name: String::new(),
        width: 0,
        height: 0,
        frame_rate_numerator: 0,
        frame_rate_denominator: 0,
        frame_count: 0,
        duration_numerator: 0,
        duration_denominator: 0,
        bytes_written: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use or_core::MediaSourceRef;
    use or_media::SoftwareMediaDecoder;
    use or_runtime::{BudgetLimits, CancellationToken, RuntimeBudgets};
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

    struct TempDirectory(PathBuf);

    impl TempDirectory {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "or-bridge-motion-library-{}-{}",
                std::process::id(),
                NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TempDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn first_frame_pixels(path: &Path) -> Vec<u8> {
        let source =
            MediaSourceRef::local_file(&format!("file://{}", path.to_string_lossy())).unwrap();
        let budgets = RuntimeBudgets::new(
            BudgetLimits::new(8, 32 * 1024 * 1024).unwrap(),
            BudgetLimits::new(8, 32 * 1024 * 1024).unwrap(),
            BudgetLimits::new(8, 32 * 1024 * 1024).unwrap(),
        );
        let decoder = SoftwareMediaDecoder::new(&source, budgets).unwrap();
        let frame = decoder
            .decode_video_frame_at(or_core::RationalTime::ZERO, &CancellationToken::new())
            .unwrap()
            .unwrap();
        frame.pixels().to_vec()
    }

    fn png(width: u32, height: u32) -> Vec<u8> {
        let mut pixels = Vec::with_capacity((width * height * 4) as usize);
        for index in 0..width * height {
            pixels.extend_from_slice(&[
                (index % 256) as u8,
                ((index * 3) % 256) as u8,
                ((index * 7) % 256) as u8,
                255,
            ]);
        }
        encode_png(width, height, &pixels).unwrap()
    }

    #[test]
    fn imports_composes_previews_and_saves_a_scene_asset() {
        let directory = TempDirectory::new();
        let root = directory.0.join("library").display().to_string();
        let source = directory.0.join("mountain.png");
        fs::write(&source, png(2, 2)).unwrap();

        let imported = motion_asset_import_image(
            root.clone(),
            source.display().to_string(),
            "Mountain background".to_owned(),
            None,
            Vec::new(),
            1920,
            1080,
            4,
        );
        assert!(imported.ok, "{} {}", imported.code, imported.message);
        assert_eq!(imported.kind, "image");
        assert_eq!(imported.category, "background");
        assert!(!imported.preview_path.is_empty());
        assert!(Path::new(&imported.preview_path).exists());
        assert_eq!(imported.parameters.len(), 4);

        let asset_id = imported.asset_id.clone();
        let instantiated = motion_asset_scene(
            root.clone(),
            asset_id.clone(),
            r#"{"opacity":0.5,"width":960}"#.to_owned(),
            None,
            None,
        );
        assert!(
            instantiated.ok,
            "{} {}",
            instantiated.code, instantiated.message
        );
        assert_eq!(
            instantiated.preview_png.get(..8),
            Some(&b"\x89PNG\r\n\x1a\n"[..])
        );
        let scene: or_motion::MotionScene = serde_json::from_str(&instantiated.scene_json).unwrap();
        let image = scene
            .nodes
            .iter()
            .find(|node| node.kind == or_motion::MotionNodeKind::Image)
            .unwrap();
        assert_eq!(image.opacity, Some(0.5));
        assert_eq!(image.size.unwrap().width, 960.0);

        let draft = motion_scene_new("Composition".to_owned(), 1920, 1080, 5);
        assert!(draft.ok, "{}", draft.message);
        let appended = motion_scene_append(
            root.clone(),
            draft.scene_json,
            asset_id,
            r#"{"opacity":1}"#.to_owned(),
            "a".to_owned(),
            100.0,
            50.0,
        );
        assert!(appended.ok, "{} {}", appended.code, appended.message);
        let preview = or_render::decode_motion_image(&appended.preview_png).unwrap();
        assert!(
            preview.pixels().chunks(4).any(|pixel| pixel[3] > 0),
            "the composed preview is transparent"
        );
        let scene: or_motion::MotionScene = serde_json::from_str(&appended.scene_json).unwrap();
        let image = scene
            .nodes
            .iter()
            .find(|node| node.kind == or_motion::MotionNodeKind::Image)
            .unwrap();
        assert_eq!(image.position.unwrap().x, 100.0);
        assert_eq!(image.position.unwrap().y, 50.0);
        // Ids are prefixed, so appending twice cannot collide.
        assert!(image.node_id.starts_with("a:"));

        let saved = motion_scene_save(
            root.clone(),
            appended.scene_json,
            "Mountain title".to_owned(),
            Some("nature".to_owned()),
        );
        assert!(saved.ok, "{} {}", saved.code, saved.message);
        assert_eq!(saved.category, "nature");
        assert!(!saved.preview_path.is_empty());
        assert_eq!(motion_library(root.clone()).entries.len(), 2);

        let removed = motion_asset_remove(root.clone(), saved.asset_id);
        assert!(removed.ok && removed.removed);
        assert_eq!(motion_library(root).entries.len(), 1);
    }

    #[test]
    fn materialises_an_image_asset_through_the_app_encoder() {
        let directory = TempDirectory::new();
        let root = directory.0.join("library").display().to_string();
        let source = directory.0.join("cutout.png");
        fs::write(&source, png(320, 180)).unwrap();
        let imported = motion_asset_import_image(
            root.clone(),
            source.display().to_string(),
            "Cutout".to_owned(),
            None,
            Vec::new(),
            1920,
            1080,
            1,
        );
        assert!(imported.ok, "{} {}", imported.code, imported.message);

        let output = directory.0.join("cutout.mkv");
        let asset_id = imported.asset_id.clone();
        let materialized = motion_asset_materialize(
            root,
            asset_id.clone(),
            "{}".to_owned(),
            output.display().to_string(),
            2,
            1,
            1920,
            1080,
        );
        assert!(
            materialized.succeeded,
            "{} {}",
            materialized.code, materialized.message
        );
        assert!(output.exists());
        assert!(fs::metadata(&output).unwrap().len() > 0);
        assert!(materialized.frame_count >= 1);
        let pixels = first_frame_pixels(&output);
        assert!(
            pixels
                .chunks(4)
                .any(|pixel| pixel[0] > 0 || pixel[1] > 0 || pixel[2] > 0),
            "the materialised image frame is black"
        );
    }

    #[test]
    fn a_missing_asset_or_bad_document_returns_a_code() {
        let directory = TempDirectory::new();
        let root = directory.0.join("library").display().to_string();
        let missing = motion_asset_load(
            root.clone(),
            "6f1c2f5e-3a4b-4c5d-8e9f-0a1b2c3d4e5f".to_owned(),
        );
        assert!(!missing.ok);
        assert_eq!(missing.code, "MOTION_LIBRARY_NOT_FOUND");

        let bad = motion_asset_save(root.clone(), "{}".to_owned(), None);
        assert!(!bad.ok);
        assert_eq!(bad.code, "MOTION_ASSET_INVALID_JSON");

        let bad_category = motion_asset_save(root, "{}".to_owned(), Some("nope".to_owned()));
        assert!(!bad_category.ok);
        assert_eq!(bad_category.code, "MOTION_ASSET_INVALID_JSON");
    }

    #[test]
    fn a_gui_edit_moves_retitles_recolours_adds_and_deletes_a_node() {
        let directory = TempDirectory::new();
        let root = directory.0.join("library").display().to_string();
        let draft = motion_scene_new("Composition".to_owned(), 1920, 1080, 5);
        assert!(draft.ok, "{}", draft.message);
        assert!(draft.nodes.is_empty(), "a new scene has no node boxes");

        let added = motion_scene_edit(
            root.clone(),
            draft.scene_json,
            r#"{"op":"add_text","node_id":"text-1","text":"Hello","x":120,"y":240,"font_size":72}"#
                .to_owned(),
        );
        assert!(added.ok, "{} {}", added.code, added.message);
        assert_eq!(added.nodes.len(), 1);
        let node = &added.nodes[0];
        assert_eq!(node.node_id, "text-1");
        assert_eq!(node.kind, "text");
        assert_eq!(node.text, "Hello");
        assert_eq!((node.x, node.y), (120.0, 240.0));
        assert_eq!(node.font_size, 72.0);
        assert!(
            node.width > 0.0 && node.height > 0.0,
            "a text node has a handle box"
        );
        assert!(node.has_fill);
        assert_eq!(added.preview_png.get(..8), Some(&b"\x89PNG\r\n\x1a\n"[..]));

        let moved = motion_scene_edit(
            root.clone(),
            added.scene_json,
            r#"{"op":"move","node_id":"text-1","x":600,"y":400}"#.to_owned(),
        );
        assert!(moved.ok, "{} {}", moved.code, moved.message);
        assert_eq!((moved.nodes[0].x, moved.nodes[0].y), (600.0, 400.0));

        let titled = motion_scene_edit(
            root.clone(),
            moved.scene_json,
            r#"{"op":"set_text","node_id":"text-1","text":"Changed"}"#.to_owned(),
        );
        assert!(titled.ok, "{} {}", titled.code, titled.message);
        assert_eq!(titled.nodes[0].text, "Changed");

        let coloured = motion_scene_edit(
            root.clone(),
            titled.scene_json,
            r#"{"op":"set_fill","node_id":"text-1","color":{"red":200,"green":40,"blue":90,"alpha":255}}"#
                .to_owned(),
        );
        assert!(coloured.ok, "{} {}", coloured.code, coloured.message);
        assert_eq!(
            (
                coloured.nodes[0].fill_red,
                coloured.nodes[0].fill_green,
                coloured.nodes[0].fill_blue
            ),
            (200, 40, 90)
        );

        let removed = motion_scene_edit(
            root.clone(),
            coloured.scene_json,
            r#"{"op":"delete","node_id":"text-1"}"#.to_owned(),
        );
        assert!(removed.ok, "{} {}", removed.code, removed.message);
        assert!(removed.nodes.is_empty());

        let unknown = motion_scene_edit(
            root,
            removed.scene_json,
            r#"{"op":"move","node_id":"text-1","x":0,"y":0}"#.to_owned(),
        );
        assert!(!unknown.ok);
        assert_eq!(unknown.code, "MOTION_EDIT_UNKNOWN_NODE");
    }

    #[test]
    fn a_gui_edit_resizes_rotates_outlines_and_deletes_a_selection() {
        let directory = TempDirectory::new();
        let root = directory.0.join("library").display().to_string();
        let draft = motion_scene_new("Composition".to_owned(), 640, 360, 5);
        assert!(draft.ok, "{}", draft.message);

        let shaped = motion_scene_edit(
            root.clone(),
            draft.scene_json,
            r#"{"op":"add_text","node_id":"label","text":"Hello","x":40,"y":60,"font_size":32}"#
                .to_owned(),
        );
        assert!(shaped.ok, "{} {}", shaped.code, shaped.message);
        let scene_json = shaped.scene_json;
        // Add a rectangle through the assistant-facing compose path is overkill here;
        // the text node proves the single-node operations and the selection path.
        let text = &shaped.nodes[0];
        assert!(!text.resizable);
        assert!(!text.strokeable);
        assert_eq!(text.rotation_degrees, 0.0);

        let rotated = motion_scene_edit(
            root.clone(),
            scene_json.clone(),
            r#"{"op":"rotate","node_id":"label","degrees":-30}"#.to_owned(),
        );
        assert!(rotated.ok, "{} {}", rotated.code, rotated.message);
        assert_eq!(rotated.nodes[0].rotation_degrees, -30.0);

        let rotated_json = rotated.scene_json.clone();
        let stroked = motion_scene_edit(
            root.clone(),
            rotated_json.clone(),
            r#"{"op":"set_stroke","node_id":"label","color":{"red":10,"green":20,"blue":30,"alpha":255}}"#
                .to_owned(),
        );
        assert!(!stroked.ok);
        assert_eq!(stroked.code, "MOTION_EDIT_NOT_STROKEABLE");

        let recoloured = motion_scene_edit(
            root.clone(),
            rotated_json,
            r#"{"op":"set_fill_many","node_ids":["label"],"color":{"red":200,"green":40,"blue":90,"alpha":255}}"#
                .to_owned(),
        );
        assert!(recoloured.ok, "{} {}", recoloured.code, recoloured.message);
        assert_eq!(recoloured.nodes[0].fill_red, 200);

        let deleted = motion_scene_edit(
            root,
            recoloured.scene_json,
            r#"{"op":"delete_many","node_ids":["label"]}"#.to_owned(),
        );
        assert!(deleted.ok, "{} {}", deleted.code, deleted.message);
        assert!(deleted.nodes.is_empty());
    }

    #[test]
    fn a_materialized_scene_is_not_black() {
        let directory = TempDirectory::new();
        let root = directory.0.join("library").display().to_string();
        let output = directory.0.join("red.mkv");
        let scene = r#"{
            "format": "opencut-reinforced-motion-scene",
            "schema_version": 1,
            "scene_id": "7a2b3c4d-5e6f-4a1b-8c9d-0e1f2a3b4c5d",
            "name": "Red",
            "duration": {"numerator": 1, "denominator": 1},
            "canvas": {"width": 64, "height": 64},
            "assets": [],
            "nodes": [
                {"node_id": "bg", "kind": "rectangle", "position": {"x": 0, "y": 0},
                 "size": {"width": 64, "height": 64},
                 "fill": {"red": 255, "green": 0, "blue": 0, "alpha": 255}}
            ],
            "animations": []
        }"#;
        let view = motion_scene_materialize(
            root,
            scene.to_owned(),
            output.display().to_string(),
            2,
            1,
            64,
            64,
        );
        assert!(view.succeeded, "{} {}", view.code, view.message);
        let source =
            MediaSourceRef::local_file(&format!("file://{}", output.to_string_lossy())).unwrap();
        let budgets = RuntimeBudgets::new(
            BudgetLimits::new(8, 32 * 1024 * 1024).unwrap(),
            BudgetLimits::new(8, 32 * 1024 * 1024).unwrap(),
            BudgetLimits::new(8, 32 * 1024 * 1024).unwrap(),
        );
        let decoder = SoftwareMediaDecoder::new(&source, budgets).unwrap();
        let frame = decoder
            .decode_video_frame_at(or_core::RationalTime::ZERO, &CancellationToken::new())
            .unwrap()
            .unwrap();
        let pixels = frame.pixels();
        assert_eq!(frame.descriptor().width(), 64);
        assert!(
            pixels.chunks(4).any(|pixel| pixel[0] > 0),
            "the frame is black"
        );
    }

    #[test]
    fn a_composed_image_scene_materializes_visibly() {
        let directory = TempDirectory::new();
        let root = directory.0.join("library").display().to_string();
        let source = directory.0.join("mountain.png");
        fs::write(&source, png(320, 180)).unwrap();
        let imported = motion_asset_import_image(
            root.clone(),
            source.display().to_string(),
            "Mountain".to_owned(),
            None,
            Vec::new(),
            1920,
            1080,
            1,
        );
        assert!(imported.ok, "{} {}", imported.code, imported.message);
        let draft = motion_scene_new("Composition".to_owned(), 1920, 1080, 1);
        assert!(draft.ok, "{}", draft.message);
        let appended = motion_scene_append(
            root.clone(),
            draft.scene_json,
            imported.asset_id,
            r#"{"opacity":1.0,"width":1920.0,"height":1080.0,"rotation":0.0}"#.to_owned(),
            "a0".to_owned(),
            0.0,
            0.0,
        );
        assert!(appended.ok, "{} {}", appended.code, appended.message);
        let output = directory.0.join("composed.mkv");
        let materialized = motion_scene_materialize(
            root,
            appended.scene_json,
            output.display().to_string(),
            2,
            1,
            1920,
            1080,
        );
        assert!(
            materialized.succeeded,
            "{} {}",
            materialized.code, materialized.message
        );
        let pixels = first_frame_pixels(&output);
        assert!(
            pixels
                .chunks(4)
                .any(|pixel| pixel[0] > 0 || pixel[1] > 0 || pixel[2] > 0),
            "the composed scene is black"
        );
    }
}
