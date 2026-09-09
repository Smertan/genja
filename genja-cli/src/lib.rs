//! First-party command-line interface for Genja automation workflows.

use clap::{Parser, Subcommand};
use std::ffi::OsString;
use std::process::ExitCode;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Parser)]
#[command(
    name = "genja",
    version,
    about = "First-party command-line interface for Genja automation workflows"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Print the Genja CLI version.
    Version,
}

/// Runs the Genja CLI with the provided process arguments.
pub fn run<I, T>(_args: I) -> ExitCode
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    match Cli::try_parse_from(_args) {
        Ok(cli) => {
            match cli.command {
                Some(Command::Version) => println!("genja {VERSION}"),
                None => {}
            }

            ExitCode::SUCCESS
        }
        Err(error) => {
            let exit_code = error.exit_code();

            if let Err(print_error) = error.print() {
                eprintln!("failed to print command-line error: {print_error}");
                return ExitCode::FAILURE;
            }

            u8::try_from(exit_code)
                .map(ExitCode::from)
                .unwrap_or(ExitCode::FAILURE)
        }
    }
}
