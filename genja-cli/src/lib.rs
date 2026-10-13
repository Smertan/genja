//! First-party command-line interface for Genja automation workflows.
//!
//! The optional `tui` feature exposes an embeddable browser and a full-screen
//! runner in `genja_cli::tui`, including a project-local `tui::run_main()` helper.
//! It also enables the `genja tui` command, which launches the same basic screen.
//! Task commands and the TUI select configured Python discovery when the
//! project file contains `[tool.genja.tasks]` and otherwise use compiled Rust
//! discovery. Wheel builds use the Python interpreter beside the installed
//! executable by default; other builds resolve `python` from `PATH`.

pub mod commands;
pub mod discovery;
pub mod output;
#[cfg(feature = "tui")]
pub mod tui;

use crate::commands::task::{TaskDescribeError, TaskDocsError, TaskListError};
use crate::discovery::DiscoveryError;
use crate::discovery::TaskDescriptorSource;
use crate::discovery::selection::select_task_descriptor_source_with_python;
use crate::output::{OutputFormat, TaskDocsOutputFormat};
use clap::{Args, Parser, Subcommand};
use std::env;
use std::error::Error;
use std::ffi::OsString;
use std::fmt;
#[cfg(feature = "python-wheel")]
use std::path::Path;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Debug, Parser)]
#[command(
    name = "genja",
    version,
    about = "First-party command-line interface for Genja automation workflows"
)]
struct Cli {
    /// Project configuration file used for Python task discovery.
    #[arg(
        long,
        global = true,
        default_value = "pyproject.toml",
        value_name = "PATH"
    )]
    pyproject: PathBuf,
    /// Python interpreter used for configured task discovery.
    #[arg(long, global = true, value_name = "PATH")]
    python: Option<PathBuf>,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Discover and inspect registered tasks.
    Task(TaskCommand),
    /// Open the terminal task browser.
    #[cfg(feature = "tui")]
    Tui,
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
    /// JSON and YAML output wrap each descriptor with its implementation language.
    List(TaskListArgs),
}

#[derive(Debug, Args)]
struct TaskDescribeArgs {
    /// Task identity as `<id>@<version>` or `rust:`/`python:` plus that identity.
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
    Discovery(DiscoveryError),
    Describe(TaskDescribeError),
    Docs(TaskDocsError),
    List(TaskListError),
    #[cfg(feature = "tui")]
    Tui(tui::TuiError),
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Discovery(error) => write!(f, "{error}"),
            Self::Describe(error) => write!(f, "{error}"),
            Self::Docs(error) => write!(f, "{error}"),
            Self::List(error) => write!(f, "{error}"),
            #[cfg(feature = "tui")]
            Self::Tui(error) => write!(f, "{error}"),
        }
    }
}

impl Error for CliError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Discovery(error) => Some(error),
            Self::Describe(error) => Some(error),
            Self::Docs(error) => Some(error),
            Self::List(error) => Some(error),
            #[cfg(feature = "tui")]
            Self::Tui(error) => Some(error),
        }
    }
}

fn execute(cli: Cli) -> Result<(), CliError> {
    let Cli {
        pyproject,
        python,
        command,
    } = cli;
    match command {
        Some(Command::Task(task)) => {
            let source = select_cli_source(python, pyproject).map_err(CliError::Discovery)?;
            match task.command {
                TaskSubcommand::Describe(args) => {
                    let output =
                        commands::task::describe_task(source.as_ref(), &args.identity, args.output)
                            .map_err(CliError::Describe)?;
                    println!("{output}");
                }
                TaskSubcommand::Docs(args) => {
                    let output = commands::task::docs_tasks(source.as_ref(), args.output)
                        .map_err(CliError::Docs)?;
                    println!("{output}");
                }
                TaskSubcommand::List(args) => {
                    let output = commands::task::list_tasks(source.as_ref(), args.output)
                        .map_err(CliError::List)?;
                    println!("{output}");
                }
            }
        }
        Some(Command::Version) => commands::version::print_version(),
        #[cfg(feature = "tui")]
        Some(Command::Tui) => {
            let source = select_cli_source(python, pyproject).map_err(CliError::Discovery)?;
            tui::run_tui(source.as_ref(), tui::TuiOptions::default()).map_err(CliError::Tui)?;
        }
        None => {}
    }

    Ok(())
}

fn select_cli_source(
    python: Option<PathBuf>,
    pyproject: PathBuf,
) -> Result<Box<dyn TaskDescriptorSource>, DiscoveryError> {
    select_task_descriptor_source_with_python(pyproject, || match python {
        Some(path) => Ok(path),
        None => default_python_interpreter(),
    })
}

#[cfg(not(feature = "python-wheel"))]
fn default_python_interpreter() -> Result<PathBuf, DiscoveryError> {
    Ok(PathBuf::from("python"))
}

#[cfg(feature = "python-wheel")]
fn default_python_interpreter() -> Result<PathBuf, DiscoveryError> {
    let executable = env::current_exe().map_err(|error| {
        DiscoveryError::source_failed(format!(
            "cannot locate the installed Genja CLI to select its Python interpreter: {error}; use --python PATH"
        ))
    })?;
    let python = companion_python_interpreter(&executable);
    if python.is_file() {
        Ok(python)
    } else {
        Err(DiscoveryError::source_failed(format!(
            "cannot find the Python interpreter for the installed Genja CLI at `{}`; use --python PATH",
            python.display()
        )))
    }
}

#[cfg(feature = "python-wheel")]
fn companion_python_interpreter(executable: &Path) -> PathBuf {
    let directory = executable
        .parent()
        .expect("a process executable path has a parent directory");
    directory.join(if cfg!(windows) {
        "python.exe"
    } else {
        "python"
    })
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
/// process. Configured Python tasks are imported through the selected
/// interpreter when a task command or the TUI requests discovery. With the
/// `python-wheel` feature, an omitted `--python` selects the interpreter beside
/// the installed executable; other builds use `python` from `PATH`.
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
    fn python_override_is_distinct_from_default() {
        let default = Cli::try_parse_from(["genja", "task", "list"])
            .expect("default CLI arguments should parse");
        assert!(default.python.is_none());

        let explicit = Cli::try_parse_from(["genja", "task", "list", "--python", "custom-python"])
            .expect("Python override should parse");
        assert_eq!(explicit.python, Some(PathBuf::from("custom-python")));
    }

    #[cfg(feature = "python-wheel")]
    #[test]
    fn wheel_companion_path_is_next_to_executable() {
        #[cfg(windows)]
        {
            let executable = Path::new(r"C:\venv\Scripts\genja.exe");
            assert_eq!(
                companion_python_interpreter(executable),
                PathBuf::from(r"C:\venv\Scripts\python.exe")
            );
        }
        #[cfg(not(windows))]
        {
            let executable = Path::new("/venv/bin/genja");
            assert_eq!(
                companion_python_interpreter(executable),
                PathBuf::from("/venv/bin/python")
            );
        }
    }

    #[test]
    fn run_main_has_binary_entrypoint_shape() {
        let _run_main: fn() -> ExitCode = run_main;
    }

    #[cfg(feature = "tui")]
    #[test]
    fn tui_parses_as_a_top_level_command() {
        let cli = Cli::try_parse_from(["genja", "tui"]).expect("TUI command should parse");
        assert!(matches!(cli.command, Some(Command::Tui)));
    }

    #[cfg(not(feature = "tui"))]
    #[test]
    fn tui_is_rejected_without_the_feature() {
        let error = Cli::try_parse_from(["genja", "tui"]).expect_err("TUI should be disabled");
        assert_eq!(error.kind(), clap::error::ErrorKind::InvalidSubcommand);
    }
}
