//! End-to-end tests: the `dd-mapper` binary on example plans and fixtures.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const PNG_SIGNATURE: &[u8] = b"\x89PNG";

fn data(file: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/data")
        .join(file)
}

fn fixture(file: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(file)
}

/// A fresh path in the test scratch directory.
fn scratch(file: &str) -> PathBuf {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(file);
    let _ = std::fs::remove_file(&path);
    path
}

/// Runs `dd-mapper` with the test catalog.
fn dd_mapper(args: &[&Path]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_dd-mapper"))
        .args(args)
        .arg("--catalog")
        .arg(data("catalog.toml"))
        .output()
        .unwrap()
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "{:?}\n{}{}",
        output.status,
        stdout(output),
        stderr(output)
    );
}

#[test]
fn prints_its_help() {
    let output = dd_mapper(&[Path::new("help")]);
    assert_success(&output);
    assert!(stdout(&output).contains("generate <plan.toml>"));
    assert!(stdout(&output).contains("--packs"));
}

#[test]
fn generates_a_map_and_its_preview_from_a_plan() {
    let map = scratch("hab.dungeondraft_map");
    let png = scratch("hab.png");
    let output = dd_mapper(&[
        Path::new("generate"),
        &data("hab_block.toml"),
        Path::new("--out"),
        &map,
        Path::new("--preview"),
        &png,
    ]);
    assert_success(&output);
    assert!(
        stdout(&output).contains("No warning."),
        "{}",
        stdout(&output)
    );
    assert!(std::fs::read(&png).unwrap().starts_with(PNG_SIGNATURE));

    let validated = dd_mapper(&[Path::new("validate"), &map]);
    assert_success(&validated);
    assert_eq!(stdout(&validated), "No warning.\n");
}

#[test]
fn validation_fails_on_design_warnings() {
    let map = scratch("vault.dungeondraft_map");
    let generated = dd_mapper(&[
        Path::new("generate"),
        &data("sealed_vault.toml"),
        Path::new("--out"),
        &map,
    ]);
    assert_success(&generated);
    assert!(stdout(&generated).contains("room R0 has no door"));

    let validated = dd_mapper(&[Path::new("validate"), &map]);
    assert_eq!(validated.status.code(), Some(1));
    assert_eq!(stdout(&validated), "room R0 has no door\n");
}

#[test]
fn previews_a_saved_map() {
    let png = scratch("fixture.png");
    let output = dd_mapper(&[
        Path::new("preview"),
        &fixture("test.dungeondraft_map"),
        Path::new("--out"),
        &png,
        Path::new("--scale"),
        Path::new("8"),
    ]);
    assert_success(&output);
    assert!(std::fs::read(&png).unwrap().starts_with(PNG_SIGNATURE));
}

#[test]
fn searches_and_summarizes_the_catalog() {
    let summary = dd_mapper(&[Path::new("catalog")]);
    assert_success(&summary);
    assert!(
        stdout(&summary).starts_with("6 assets.\n"),
        "{}",
        stdout(&summary)
    );

    let search = dd_mapper(&[
        Path::new("catalog"),
        Path::new("crate"),
        Path::new("--kind"),
        Path::new("object"),
    ]);
    assert_success(&search);
    assert!(
        stdout(&search)
            .contains("Container_Large_Metal_Crate_Green.webp | objects | 1.8 x 1.5 cells"),
        "{}",
        stdout(&search)
    );
}

#[test]
fn reports_errors_on_stderr() {
    let output = dd_mapper(&[Path::new("draw")]);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("unknown command draw"));

    let broken = dd_mapper(&[
        Path::new("generate"),
        &data("missing.toml"),
        Path::new("--out"),
        &scratch("missing.dungeondraft_map"),
    ]);
    assert_eq!(broken.status.code(), Some(2));
    assert!(
        stderr(&broken).contains("missing.toml"),
        "{}",
        stderr(&broken)
    );
}
