use std::process::ExitCode;

fn main() -> ExitCode {
    genja_cli::run(std::env::args_os())
}
