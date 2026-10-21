//! Version command handling.

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Print the Genja CLI version.
pub fn print_version() {
    println!("genja {VERSION}");
}
