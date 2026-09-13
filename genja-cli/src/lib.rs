//! First-party command-line interface for Genja automation workflows.

pub mod commands;
pub mod discovery;
pub mod output;

use crate::output::OutputFormat;
use clap::{Args, Parser, Subcommand};
use std::ffi::OsString;
use std::process::ExitCode;

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
    /// Discover and inspect registered tasks.
    Task(TaskCommand),
    /// Print the Genja CLI version.
    Version,
}

#[derive(Debug, Args)]
struct TaskCommand {
    #[command(subcommand)]
    command: TaskSubcommand,
}

#[derive(Debug, Subcommand)]
enum TaskSubcommand {
    /// List registered task descriptors.
    List(TaskListArgs),
}

#[derive(Debug, Args)]
struct TaskListArgs {
    /// Output format to render.
    #[arg(long, value_enum, default_value = "table")]
    output: OutputFormat,
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
                Some(Command::Task(task)) => match task.command {
                    TaskSubcommand::List(args) => commands::task::list_tasks(args.output),
                },
                Some(Command::Version) => commands::version::print_version(),
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
