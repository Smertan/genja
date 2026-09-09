use std::process::Command;

fn genja_command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_genja"))
}

#[test]
fn help_prints_top_level_usage() {
    let output = genja_command()
        .arg("--help")
        .output()
        .expect("genja --help should run");

    assert!(
        output.status.success(),
        "genja --help should succeed: {output:?}"
    );

    let stdout = String::from_utf8(output.stdout).expect("help output should be UTF-8");

    assert!(stdout.contains("Usage: genja"));
    assert!(stdout.contains("Commands:"));
    assert!(stdout.contains("version"));
}

#[test]
fn version_command_prints_cli_version() {
    let output = genja_command()
        .arg("version")
        .output()
        .expect("genja version should run");

    assert!(
        output.status.success(),
        "genja version should succeed: {output:?}"
    );

    let stdout = String::from_utf8(output.stdout).expect("version output should be UTF-8");

    assert_eq!(stdout, format!("genja {}\n", env!("CARGO_PKG_VERSION")));
}
