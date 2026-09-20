//! First-party command-line interface for Genja automation workflows.

pub mod commands;
pub mod discovery;
pub mod output;

use crate::commands::task::{TaskDescribeError, TaskDocsError, TaskListError};
use crate::discovery::rust::CompiledTaskDescriptorSource;
use crate::output::{OutputFormat, TaskDocsOutputFormat};
use clap::{Args, Parser, Subcommand};
use std::env;
use std::error::Error;
use std::ffi::OsString;
use std::fmt;
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
    /// Describe one registered task descriptor.
    Describe(TaskDescribeArgs),
    /// Generate Markdown documentation for registered task descriptors.
    Docs(TaskDocsArgs),
    /// List registered task descriptor summaries.
    ///
    /// Table and Markdown output are compact summary views. Use
    /// `genja task describe <identity>` for full descriptor metadata.
    /// JSON and YAML output include full descriptor items for the listed tasks.
    List(TaskListArgs),
}

#[derive(Debug, Args)]
struct TaskDescribeArgs {
    /// Task identity in `<task-id>@<task-version>` form.
    identity: String,
    /// Output format to render.
    #[arg(long, value_enum, default_value = "table")]
    output: OutputFormat,
}

#[derive(Debug, Args)]
struct TaskListArgs {
    /// Output format to render.
    #[arg(long, value_enum, default_value = "table")]
    output: OutputFormat,
}

#[derive(Debug, Args)]
struct TaskDocsArgs {
    /// Output format to render.
    #[arg(long, value_enum, default_value = "markdown")]
    output: TaskDocsOutputFormat,
}

#[derive(Debug)]
enum CliError {
    Describe(TaskDescribeError),
    Docs(TaskDocsError),
    List(TaskListError),
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Describe(error) => write!(f, "{error}"),
            Self::Docs(error) => write!(f, "{error}"),
            Self::List(error) => write!(f, "{error}"),
        }
    }
}

impl Error for CliError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Describe(error) => Some(error),
            Self::Docs(error) => Some(error),
            Self::List(error) => Some(error),
        }
    }
}

fn execute(cli: Cli) -> Result<(), CliError> {
    match cli.command {
        Some(Command::Task(task)) => match task.command {
            TaskSubcommand::Describe(args) => {
                let source = CompiledTaskDescriptorSource::new();
                let output = commands::task::describe_task(&source, &args.identity, args.output)
                    .map_err(CliError::Describe)?;
                println!("{output}");
            }
            TaskSubcommand::Docs(args) => {
                let source = CompiledTaskDescriptorSource::new();
                let output =
                    commands::task::docs_tasks(&source, args.output).map_err(CliError::Docs)?;
                println!("{output}");
            }
            TaskSubcommand::List(args) => {
                let source = CompiledTaskDescriptorSource::new();
                let output =
                    commands::task::list_tasks(&source, args.output).map_err(CliError::List)?;
                println!("{output}");
            }
        },
        Some(Command::Version) => commands::version::print_version(),
        None => {}
    }

    Ok(())
}

/// Runs the Genja CLI using the current process arguments.
///
/// This is the standard helper for binaries that delegate their `main`
/// function to Genja CLI behavior:
///
/// ```no_run
/// fn main() -> std::process::ExitCode {
///     genja_cli::run_main()
/// }
/// ```
///
/// Project-local binaries can use this helper to expose Genja CLI commands
/// while linking their own compiled Rust task registrations into the running
/// process.
pub fn run_main() -> ExitCode {
    run(env::args_os())
}

/// Runs the Genja CLI with the provided process arguments.
///
/// This lower-level entrypoint is useful for tests and callers that need to
/// provide their own argument iterator. Binary `main` functions should usually
/// call [`run_main`] instead.
pub fn run<I, T>(_args: I) -> ExitCode
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    match Cli::try_parse_from(_args) {
        Ok(cli) => execute(cli).map_or_else(
            |error| {
                eprintln!("error: {error}");
                ExitCode::FAILURE
            },
            |()| ExitCode::SUCCESS,
        ),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_main_has_binary_entrypoint_shape() {
        let _run_main: fn() -> ExitCode = run_main;
    }
}
