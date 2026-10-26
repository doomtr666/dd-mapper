//! The command line: a command, its arguments, and the source options.

use std::collections::BTreeMap;
use std::path::PathBuf;

use dd_catalog::{AssetKind, AssetQuery};
use dd_setup::{Config, SOURCE_OPTIONS};

/// Preview scale when none is given, in pixels per cell.
pub const DEFAULT_SCALE: u32 = 32;
/// Catalog results when no limit is given.
const DEFAULT_LIMIT: usize = 30;

pub const USAGE: &str = "\
usage: dd-mapper <command> [arguments] [source options]

commands:
  generate <plan.toml> --out <map.dungeondraft_map> [--preview <map.png>]
                         build a map from a plan, optionally with its preview
  validate <map.dungeondraft_map>
                         list likely design mistakes; fails when there are some
  preview <map.dungeondraft_map> --out <map.png> [--scale <pixels per cell>]
                         draw a schematic preview
  catalog [words...] [--kind <kind>] [--pack <id>] [--tag <tag>]... [--limit <n>]
                         search assets; without criteria, count them by pack and kind
  help                   show this help

source options:";

/// Build a map from a plan, and maybe its preview.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Generate {
    pub plan: PathBuf,
    pub out: PathBuf,
    pub preview: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Generate(Generate),
    Validate {
        map: PathBuf,
    },
    Preview {
        map: PathBuf,
        out: PathBuf,
        scale: u32,
    },
    /// No query: summarize the catalog.
    Catalog(Option<AssetQuery>),
    Help,
}

/// What to do, and where assets come from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invocation {
    pub command: Command,
    pub config: Config,
}

/// The full help text.
#[must_use]
pub fn usage() -> String {
    format!("{USAGE}\n{SOURCE_OPTIONS}")
}

/// # Errors
/// Fails on an unknown command or option, or missing arguments.
pub fn parse(args: impl IntoIterator<Item = String>) -> anyhow::Result<Invocation> {
    let mut config = Config::default();
    let mut words = Vec::new();
    let mut options: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        if !arg.starts_with("--") || arg == "--help" {
            words.push(arg);
            continue;
        }
        if config.take(&arg, &mut args)? {
            continue;
        }
        let value = args
            .next()
            .ok_or_else(|| anyhow::anyhow!("{arg} needs a value"))?;
        options.entry(arg).or_default().push(value);
    }
    let mut words = words.into_iter();
    let arguments = Arguments {
        command: words.next().unwrap_or_else(|| "help".to_owned()),
        files: words.collect(),
        options,
    };
    Ok(Invocation {
        command: arguments.command()?,
        config,
    })
}

/// The command name, its other words, and its own options.
struct Arguments {
    command: String,
    files: Vec<String>,
    options: BTreeMap<String, Vec<String>>,
}

impl Arguments {
    fn command(&self) -> anyhow::Result<Command> {
        match self.command.as_str() {
            "help" | "--help" => Ok(Command::Help),
            "generate" => self.generate(),
            "validate" => {
                self.allow(&[])?;
                Ok(Command::Validate {
                    map: self.one_file("map file")?,
                })
            }
            "preview" => self.preview(),
            "catalog" => self.catalog(),
            other => anyhow::bail!("unknown command {other}; run dd-mapper help"),
        }
    }

    fn generate(&self) -> anyhow::Result<Command> {
        self.allow(&["--out", "--preview"])?;
        Ok(Command::Generate(Generate {
            plan: self.one_file("plan file")?,
            out: self.required("--out")?,
            preview: self.last("--preview").map(PathBuf::from),
        }))
    }

    fn preview(&self) -> anyhow::Result<Command> {
        self.allow(&["--out", "--scale"])?;
        Ok(Command::Preview {
            map: self.one_file("map file")?,
            out: self.required("--out")?,
            scale: self.number("--scale")?.unwrap_or(DEFAULT_SCALE),
        })
    }

    fn catalog(&self) -> anyhow::Result<Command> {
        self.allow(&["--kind", "--pack", "--tag", "--limit"])?;
        if self.files.is_empty() && self.options.is_empty() {
            return Ok(Command::Catalog(None));
        }
        let kind = self.last("--kind").map(parse_kind).transpose()?;
        Ok(Command::Catalog(Some(AssetQuery {
            text: self.files.join(" "),
            kind,
            tags: self.options.get("--tag").cloned().unwrap_or_default(),
            pack: self.last("--pack").map(str::to_owned),
            limit: self.number("--limit")?.unwrap_or(DEFAULT_LIMIT),
        })))
    }

    fn allow(&self, allowed: &[&str]) -> anyhow::Result<()> {
        match self
            .options
            .keys()
            .find(|option| !allowed.contains(&option.as_str()))
        {
            Some(option) => anyhow::bail!("{} has no option {option}", self.command),
            None => Ok(()),
        }
    }

    fn one_file(&self, what: &str) -> anyhow::Result<PathBuf> {
        match self.files.as_slice() {
            [file] => Ok(PathBuf::from(file)),
            [] => anyhow::bail!("{} needs a {what}", self.command),
            _ => anyhow::bail!("{} takes one {what}", self.command),
        }
    }

    fn required(&self, option: &str) -> anyhow::Result<PathBuf> {
        self.last(option)
            .map(PathBuf::from)
            .ok_or_else(|| anyhow::anyhow!("{} needs {option}", self.command))
    }

    fn last(&self, option: &str) -> Option<&str> {
        self.options.get(option)?.last().map(String::as_str)
    }

    fn number<T: std::str::FromStr>(&self, option: &str) -> anyhow::Result<Option<T>> {
        self.last(option)
            .map(|text| {
                text.parse()
                    .map_err(|_| anyhow::anyhow!("{option} needs a number, not {text}"))
            })
            .transpose()
    }
}

/// A kind by its folder name, singular or plural: `wall` or `walls`.
fn parse_kind(text: &str) -> anyhow::Result<AssetKind> {
    AssetKind::from_folder(text)
        .or_else(|| AssetKind::from_folder(&format!("{text}s")))
        .ok_or_else(|| {
            anyhow::anyhow!(
                "{text} is not an asset kind; try tileset, wall, portal, object, light, \
                 pattern, terrain or path"
            )
        })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use dd_catalog::{AssetKind, AssetQuery};

    use super::{Command, Generate, parse};

    fn command(line: &str) -> anyhow::Result<Command> {
        parse(line.split_whitespace().map(str::to_owned)).map(|invocation| invocation.command)
    }

    fn error(line: &str) -> String {
        command(line).unwrap_err().to_string()
    }

    #[test]
    fn reads_generate_with_its_preview() {
        assert_eq!(
            command("generate hab.toml --out hab.dungeondraft_map --preview hab.png").unwrap(),
            Command::Generate(Generate {
                plan: PathBuf::from("hab.toml"),
                out: PathBuf::from("hab.dungeondraft_map"),
                preview: Some(PathBuf::from("hab.png")),
            })
        );
    }

    #[test]
    fn reads_validate_and_preview() {
        assert_eq!(
            command("validate den.dungeondraft_map").unwrap(),
            Command::Validate {
                map: PathBuf::from("den.dungeondraft_map")
            }
        );
        assert_eq!(
            command("preview den.dungeondraft_map --out den.png").unwrap(),
            Command::Preview {
                map: PathBuf::from("den.dungeondraft_map"),
                out: PathBuf::from("den.png"),
                scale: 32,
            }
        );
        assert!(matches!(
            command("preview den.dungeondraft_map --out den.png --scale 8"),
            Ok(Command::Preview { scale: 8, .. })
        ));
    }

    #[test]
    fn reads_catalog_searches_and_summaries() {
        assert_eq!(command("catalog").unwrap(), Command::Catalog(None));
        assert_eq!(
            command("catalog metal crate --kind object --tag container --limit 5").unwrap(),
            Command::Catalog(Some(AssetQuery {
                text: "metal crate".to_owned(),
                kind: Some(AssetKind::Object),
                tags: vec!["container".to_owned()],
                pack: None,
                limit: 5,
            }))
        );
        assert!(matches!(
            command("catalog --kind walls --pack LOREGD01"),
            Ok(Command::Catalog(Some(AssetQuery {
                kind: Some(AssetKind::Wall),
                ..
            })))
        ));
    }

    #[test]
    fn keeps_source_options_anywhere() {
        let invocation = parse(
            "--packs packs validate den.dungeondraft_map --maps maps"
                .split_whitespace()
                .map(str::to_owned),
        )
        .unwrap();
        assert_eq!(invocation.config.packs, [PathBuf::from("packs")]);
        assert_eq!(invocation.config.maps, [PathBuf::from("maps")]);
    }

    #[test]
    fn asks_for_help() {
        assert_eq!(command("").unwrap(), Command::Help);
        assert_eq!(command("help").unwrap(), Command::Help);
        assert_eq!(command("--help").unwrap(), Command::Help);
    }

    #[test]
    fn explains_what_is_wrong() {
        assert!(error("draw x").contains("unknown command draw"));
        assert!(error("generate plan.toml").contains("generate needs --out"));
        assert!(error("validate").contains("validate needs a map file"));
        assert!(error("validate a b").contains("validate takes one map file"));
        assert!(error("validate a --out b").contains("validate has no option --out"));
        assert!(error("preview a --out b --scale big").contains("--scale needs a number"));
        assert!(error("catalog --kind roof").contains("roof is not an asset kind"));
        assert!(error("catalog --limit").contains("--limit needs a value"));
    }
}
