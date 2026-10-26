//! Runs a command against a workspace, writing results to `out`.

use std::io::Write;
use std::path::Path;

use anyhow::Context as _;
use dd_app::{Workspace, asset_line};
use dd_catalog::AssetQuery;

use crate::args::{Command, DEFAULT_SCALE, Generate, Invocation, usage};
use crate::plan::Plan;

/// How a command ended, when it did not fail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Done,
    /// The map is valid but probably wrong.
    Warnings,
}

/// # Errors
/// Fails when a file cannot be read or written, or a plan cannot be built.
pub fn run(invocation: &Invocation, out: &mut impl Write) -> anyhow::Result<Outcome> {
    let workspace = || invocation.config.build_workspace();
    match &invocation.command {
        Command::Help => {
            writeln!(out, "{}", usage())?;
            Ok(Outcome::Done)
        }
        Command::Generate(request) => generate(&mut workspace()?, request, out),
        Command::Validate { map } => validate(&mut workspace()?, map, out),
        Command::Preview {
            map,
            out: png,
            scale,
        } => {
            let mut workspace = workspace()?;
            let id = open(&mut workspace, map, out)?;
            write_preview(&workspace, &id, png, *scale)?;
            Ok(Outcome::Done)
        }
        Command::Catalog(query) => {
            catalog(&workspace()?, query.as_ref(), out)?;
            Ok(Outcome::Done)
        }
    }
}

/// Builds and saves the map; design warnings are shown but do not fail.
fn generate(
    workspace: &mut Workspace,
    request: &Generate,
    out: &mut impl Write,
) -> anyhow::Result<Outcome> {
    let path = &request.plan;
    let text =
        std::fs::read_to_string(path).with_context(|| format!("cannot read {}", path.display()))?;
    let plan = Plan::parse(&text).with_context(|| format!("{}", path.display()))?;
    let id = plan.build(workspace)?;
    workspace.save(&id, &request.out)?;
    writeln!(out, "Wrote {}.", request.out.display())?;
    if let Some(png) = &request.preview {
        write_preview(workspace, &id, png, DEFAULT_SCALE)?;
        writeln!(out, "Wrote {}.", png.display())?;
    }
    report_warnings(workspace, &id, out)?;
    Ok(Outcome::Done)
}

fn validate(
    workspace: &mut Workspace,
    map: &Path,
    out: &mut impl Write,
) -> anyhow::Result<Outcome> {
    let id = open(workspace, map, out)?;
    report_warnings(workspace, &id, out)
}

/// Opens `map`, noting on stderr what the import could not keep.
fn open(workspace: &mut Workspace, map: &Path, out: &mut impl Write) -> anyhow::Result<String> {
    let opened = workspace.open_map(map)?;
    out.flush()?;
    for warning in &opened.warnings {
        eprintln!("not imported: {warning}");
    }
    Ok(opened.id)
}

fn write_preview(workspace: &Workspace, id: &str, png: &Path, scale: u32) -> anyhow::Result<()> {
    let image = workspace.preview_png(id, scale)?;
    std::fs::write(png, image).with_context(|| format!("cannot write {}", png.display()))
}

fn report_warnings(
    workspace: &Workspace,
    id: &str,
    out: &mut impl Write,
) -> anyhow::Result<Outcome> {
    let warnings = workspace.validate(id)?;
    if warnings.is_empty() {
        writeln!(out, "No warning.")?;
        return Ok(Outcome::Done);
    }
    for warning in &warnings {
        writeln!(out, "{warning}")?;
    }
    Ok(Outcome::Warnings)
}

fn catalog(
    workspace: &Workspace,
    query: Option<&AssetQuery>,
    out: &mut impl Write,
) -> anyhow::Result<()> {
    let Some(query) = query else {
        write!(out, "{}", workspace.catalog_summary())?;
        return Ok(());
    };
    let assets = workspace.search_catalog(query);
    if assets.is_empty() {
        writeln!(
            out,
            "No asset matches; try fewer or other words, or another kind."
        )?;
    }
    for asset in assets {
        writeln!(out, "{}", asset_line(asset))?;
    }
    Ok(())
}
