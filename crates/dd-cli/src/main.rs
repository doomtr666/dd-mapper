//! `dd-mapper`: builds, checks and previews Dungeondraft maps from the shell.

use std::process::ExitCode;

use dd_cli::args::parse;
use dd_cli::run::{Outcome, run};

/// Exit status of a command that could not run.
const FAILURE: u8 = 2;

fn main() -> ExitCode {
    let outcome = parse(std::env::args().skip(1))
        .and_then(|invocation| run(&invocation, &mut std::io::stdout().lock()));
    match outcome {
        Ok(Outcome::Done) => ExitCode::SUCCESS,
        Ok(Outcome::Warnings) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("dd-mapper: {error:#}");
            ExitCode::from(FAILURE)
        }
    }
}
