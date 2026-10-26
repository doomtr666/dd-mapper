//! Command-line configuration, and the catalog and workspace it describes.

use std::path::{Path, PathBuf};

use dd_app::Workspace;
use dd_catalog::{AssetSource, Catalog, SourceError};
use dd_catalog_io::{MapHarvester, PackIndexSource, TomlCatalogSource};
use dd_format::model::MapFile;

use crate::adapters::{DungeondraftFiles, SchematicRenderer};

/// Help for the options choosing where assets and templates come from.
pub const SOURCE_OPTIONS: &str = "  --packs <dir|file>     Dungeondraft packs (.dungeondraft_pack, Dungeondraft.pck) to index; repeatable
  --maps <dir|file>      saved .dungeondraft_map files to harvest asset facts from; repeatable
  --catalog <file>       hand-written asset notes (catalog.toml)
  --template <file>      a map saved by Dungeondraft, the skeleton of new maps";

const PACK_EXTENSIONS: [&str; 2] = ["dungeondraft_pack", "pck"];
const MAP_EXTENSION: &str = "dungeondraft_map";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Config {
    pub packs: Vec<PathBuf>,
    pub maps: Vec<PathBuf>,
    pub catalog: Option<PathBuf>,
    pub template: Option<PathBuf>,
}

impl Config {
    /// A configuration made of source options only.
    ///
    /// # Errors
    /// Fails on an unknown option or an option without its value.
    pub fn from_args(args: impl IntoIterator<Item = String>) -> anyhow::Result<Self> {
        let mut config = Self::default();
        let mut args = args.into_iter();
        while let Some(option) = args.next() {
            if !config.take(&option, &mut args)? {
                anyhow::bail!(
                    "unknown option {option}; the options are:
{SOURCE_OPTIONS}"
                );
            }
        }
        Ok(config)
    }

    /// Reads `option` and its value from `rest` when it is a source option;
    /// tells whether it was one.
    ///
    /// # Errors
    /// Fails when the option has no value.
    pub fn take(
        &mut self,
        option: &str,
        rest: &mut impl Iterator<Item = String>,
    ) -> anyhow::Result<bool> {
        let mut value = || {
            rest.next()
                .map(PathBuf::from)
                .ok_or_else(|| anyhow::anyhow!("{option} needs a value"))
        };
        match option {
            "--packs" => self.packs.push(value()?),
            "--maps" => self.maps.push(value()?),
            "--catalog" => self.catalog = Some(value()?),
            "--template" => self.template = Some(value()?),
            _ => return Ok(false),
        }
        Ok(true)
    }

    /// The catalog of every configured source: packs, then maps, then notes.
    ///
    /// # Errors
    /// Fails when a source cannot be read.
    pub fn build_catalog(&self) -> Result<Catalog, SourceError> {
        let packs: Vec<PackIndexSource> = files_with_extensions(&self.packs, &PACK_EXTENSIONS)?
            .into_iter()
            .map(PackIndexSource::new)
            .collect();
        let harvester = MapHarvester::new(files_with_extensions(&self.maps, &[MAP_EXTENSION])?);
        let notes = self.catalog.as_ref().map(TomlCatalogSource::new);
        let mut sources: Vec<&dyn AssetSource> =
            packs.iter().map(|pack| pack as &dyn AssetSource).collect();
        sources.push(&harvester);
        if let Some(notes) = &notes {
            sources.push(notes);
        }
        Catalog::from_sources(&sources)
    }

    /// # Errors
    /// Fails when the catalog or the template cannot be read.
    pub fn build_workspace(&self) -> anyhow::Result<Workspace> {
        let files = match &self.template {
            Some(path) => DungeondraftFiles::new(MapFile::parse(&std::fs::read_to_string(path)?)?),
            None => DungeondraftFiles::with_default_template()?,
        };
        Ok(Workspace::new(
            self.build_catalog()?,
            Box::new(files),
            Box::new(SchematicRenderer),
        ))
    }
}

/// The given files, plus the files of the given directories with one of `extensions`.
fn files_with_extensions(
    paths: &[PathBuf],
    extensions: &[&str],
) -> Result<Vec<PathBuf>, SourceError> {
    let mut files = Vec::new();
    for path in paths {
        if path.is_dir() {
            files.extend(directory_files(path, extensions)?);
        } else {
            files.push(path.clone());
        }
    }
    Ok(files)
}

fn directory_files(directory: &Path, extensions: &[&str]) -> Result<Vec<PathBuf>, SourceError> {
    let failure = |error: std::io::Error| SourceError {
        source_name: directory.display().to_string(),
        reason: error.to_string(),
    };
    let mut files = Vec::new();
    for entry in std::fs::read_dir(directory).map_err(failure)? {
        let path = entry.map_err(failure)?.path();
        let wanted = path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extensions.contains(&extension));
        if wanted {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::Config;

    fn args(text: &str) -> impl Iterator<Item = String> {
        text.split_whitespace().map(str::to_owned)
    }

    #[test]
    fn reads_repeatable_and_single_options() {
        let config = Config::from_args(args(
            "--packs a --packs b.pck --maps m --catalog c.toml --template t.dungeondraft_map",
        ))
        .unwrap();
        assert_eq!(
            config,
            Config {
                packs: vec![PathBuf::from("a"), PathBuf::from("b.pck")],
                maps: vec![PathBuf::from("m")],
                catalog: Some(PathBuf::from("c.toml")),
                template: Some(PathBuf::from("t.dungeondraft_map")),
            }
        );
        assert_eq!(Config::from_args(args("")).unwrap(), Config::default());
    }

    #[test]
    fn rejects_unknown_options_and_missing_values() {
        let unknown = Config::from_args(args("--pack a")).unwrap_err().to_string();
        assert!(
            unknown.contains("unknown option --pack;") && unknown.contains("--packs <dir|file>"),
            "{unknown}"
        );
        let missing = Config::from_args(args("--maps")).unwrap_err().to_string();
        assert!(missing.contains("--maps needs a value"), "{missing}");
    }

    #[test]
    fn leaves_other_options_to_the_caller() {
        let mut config = Config::default();
        let mut rest = args("out.png --scale 8");
        assert!(config.take("--maps", &mut rest).unwrap());
        assert!(!config.take("--scale", &mut rest).unwrap());
        assert_eq!(config.maps, [PathBuf::from("out.png")]);
        assert_eq!(rest.collect::<Vec<_>>(), ["--scale", "8"]);
    }

    #[test]
    fn harvests_every_map_of_a_directory() {
        let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures");
        let config = Config {
            maps: vec![fixtures],
            ..Config::default()
        };
        let catalog = config.build_catalog().unwrap();
        let door = catalog
            .get("res://packs/LOREGD01/textures/portals/Door_Underhive_Industrial.webp")
            .unwrap();
        assert_eq!(door.metrics.portal_radius, Some(154.0));
    }

    #[test]
    fn builds_a_workspace_with_the_default_template() {
        let workspace = Config::default().build_workspace().unwrap();
        assert!(workspace.catalog().is_empty());
    }
}
