//! First-party command-line interface for Genja automation workflows.

use std::ffi::OsString;
use std::process::ExitCode;

/// Runs the Genja CLI with the provided process arguments.
pub fn run<I, T>(_args: I) -> ExitCode
where
    I: IntoIterator<Item = T>,
    T: Into<OsString>,
{
    ExitCode::SUCCESS
}
