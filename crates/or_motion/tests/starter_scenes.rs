//! The scene documents the app bundles as first-run starters.
//!
//! They are ordinary `.ormotion.json` files, so the only thing this lane needs to
//! prove is that the shipped files stay valid as the schema moves: a starter that
//! stops parsing would fail at the moment a new user taps it.

use or_motion::{parse_motion_scene, validate_scene};
use std::{fs, path::Path};

fn starter_directory() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../apps/or_app/assets/motion_starters")
}

fn starter_files() -> Vec<std::path::PathBuf> {
    let mut files: Vec<_> = fs::read_dir(starter_directory())
        .expect("the bundled starter directory exists")
        .map(|entry| entry.expect("a readable directory entry").path())
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("json"))
        .collect();
    files.sort();
    files
}

#[test]
fn every_bundled_starter_parses_and_validates() {
    let files = starter_files();
    assert_eq!(
        files.len(),
        4,
        "the app declares four starter scenes in its pubspec; found {files:?}"
    );
    let mut ids = Vec::new();
    for path in &files {
        let bytes = fs::read(path).expect("the starter file is readable");
        let scene = parse_motion_scene(&bytes)
            .unwrap_or_else(|error| panic!("{} does not parse: {error:?}", path.display()));
        let validation = validate_scene(&scene);
        assert!(
            validation.ok,
            "{} does not validate: {:?}",
            path.display(),
            validation.errors
        );
        assert_eq!(
            (scene.canvas.width, scene.canvas.height),
            (1920, 1080),
            "{} is not a 1080p landscape starter",
            path.display()
        );
        assert!(
            scene.assets.is_empty(),
            "{} depends on an image the bundle does not carry",
            path.display()
        );
        assert!(
            !scene.nodes.is_empty() && !scene.animations.is_empty(),
            "{} is an empty composition",
            path.display()
        );
        let seconds = scene.duration.numerator() as f64 / scene.duration.denominator() as f64;
        assert!(
            (1.0..=10.0).contains(&seconds),
            "{} runs for {seconds} s, outside the starter range",
            path.display()
        );
        ids.push(scene.scene_id.clone());
    }
    ids.sort();
    let unique = {
        let mut copy = ids.clone();
        copy.dedup();
        copy.len()
    };
    assert_eq!(unique, ids.len(), "two starters share a scene id");
}
