//! First-party command-line interface for Genja automation workflows.

pub mod commands;
pub mod discovery;
pub mod output;

use crate::commands::task::{TaskDescribeError, TaskListError};
use crate::discovery::rust::CompiledTaskDescriptorSource;
use crate::output::OutputFormat;
use clap::{Args, Parser, Subcommand};
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
    /// List registered task descriptors.
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

#[derive(Debug)]
enum CliError {
    TaskDescribe(TaskDescribeError),
    TaskList(TaskListError),
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TaskDescribe(error) => write!(f, "{error}"),
            Self::TaskList(error) => write!(f, "{error}"),
        }
    }
}

impl Error for CliError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::TaskDescribe(error) => Some(error),
            Self::TaskList(error) => Some(error),
        }
    }
}

fn execute(cli: Cli) -> Result<(), CliError> {
    match cli.command {
        Some(Command::Task(task)) => match task.command {
            TaskSubcommand::Describe(args) => {
                let source = CompiledTaskDescriptorSource::new();
                let output = commands::task::describe_task(&source, &args.identity, args.output)
                    .map_err(CliError::TaskDescribe)?;
                println!("{output}");
            }
            TaskSubcommand::List(args) => {
                let source = CompiledTaskDescriptorSource::new();
                let output =
                    commands::task::list_tasks(&source, args.output).map_err(CliError::TaskList)?;
                println!("{output}");
            }
        },
        Some(Command::Version) => commands::version::print_version(),
        None => {}
    }

    Ok(())
}

/// Runs the Genja CLI with the provided process arguments.
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
