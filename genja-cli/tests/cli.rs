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
    assert!(stdout.contains("task"));
    assert!(stdout.contains("version"));
}

#[test]
fn task_help_prints_task_subcommands() {
    let output = genja_command()
        .arg("task")
        .arg("--help")
        .output()
        .expect("genja task --help should run");

    assert!(
        output.status.success(),
        "genja task --help should succeed: {output:?}"
    );

    let stdout = String::from_utf8(output.stdout).expect("task help output should be UTF-8");

    assert!(stdout.contains("Usage: genja task"));
    assert!(stdout.contains("Commands:"));
    assert!(stdout.contains("list"));
}

#[test]
fn task_list_help_prints_output_formats() {
    let output = genja_command()
        .arg("task")
        .arg("list")
        .arg("--help")
        .output()
        .expect("genja task list --help should run");

    assert!(
        output.status.success(),
        "genja task list --help should succeed: {output:?}"
    );

    let stdout = String::from_utf8(output.stdout).expect("task list help output should be UTF-8");

    assert!(stdout.contains("Usage: genja task list"));
    assert!(stdout.contains("--output"));
    assert!(stdout.contains("table"));
    assert!(stdout.contains("json"));
    assert!(stdout.contains("yaml"));
    assert!(stdout.contains("markdown"));
}

#[test]
fn task_list_rejects_unsupported_output_format() {
    let output = genja_command()
        .arg("task")
        .arg("list")
        .arg("--output")
        .arg("toml")
        .output()
        .expect("genja task list should run");

    assert!(
        !output.status.success(),
        "unsupported output format should fail: {output:?}"
    );

    let stderr = String::from_utf8(output.stderr).expect("error output should be UTF-8");

    assert!(stderr.contains("invalid value"));
    assert!(stderr.contains("toml"));
    assert!(stderr.contains("table"));
    assert!(stderr.contains("json"));
    assert!(stderr.contains("yaml"));
    assert!(stderr.contains("markdown"));
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
