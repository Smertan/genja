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
    assert!(stdout.contains("describe"));
    assert!(stdout.contains("list"));
}

#[test]
fn task_describe_help_prints_identity_and_output_formats() {
    let output = genja_command()
        .arg("task")
        .arg("describe")
        .arg("--help")
        .output()
        .expect("genja task describe --help should run");

    assert!(
        output.status.success(),
        "genja task describe --help should succeed: {output:?}"
    );

    let stdout = String::from_utf8(output.stdout).expect("task describe help should be UTF-8");

    assert!(stdout.contains("Usage: genja task describe"));
    assert!(stdout.contains("<IDENTITY>"));
    assert!(stdout.contains("--output"));
    assert!(stdout.contains("table"));
    assert!(stdout.contains("json"));
    assert!(stdout.contains("yaml"));
    assert!(stdout.contains("markdown"));
}

#[test]
fn task_describe_requires_identity() {
    let output = genja_command()
        .arg("task")
        .arg("describe")
        .output()
        .expect("genja task describe should run");

    assert!(
        !output.status.success(),
        "missing identity should fail: {output:?}"
    );

    let stderr = String::from_utf8(output.stderr).expect("error output should be UTF-8");

    assert!(stderr.contains("required"));
    assert!(stderr.contains("<IDENTITY>"));
}

#[test]
fn task_describe_rejects_unsupported_output_format() {
    let output = genja_command()
        .arg("task")
        .arg("describe")
        .arg("acme.examples.backup_config@1.0.0")
        .arg("--output")
        .arg("toml")
        .output()
        .expect("genja task describe should run");

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
fn task_list_defaults_to_empty_table_output() {
    let output = genja_command()
        .arg("task")
        .arg("list")
        .output()
        .expect("genja task list should run");

    assert!(
        output.status.success(),
        "genja task list should succeed: {output:?}"
    );

    let stdout = String::from_utf8(output.stdout).expect("task list output should be UTF-8");

    assert_eq!(stdout, "No registered tasks found.\n");
}

#[test]
fn task_list_outputs_empty_json() {
    let output = genja_command()
        .arg("task")
        .arg("list")
        .arg("--output")
        .arg("json")
        .output()
        .expect("genja task list --output json should run");

    assert!(
        output.status.success(),
        "genja task list --output json should succeed: {output:?}"
    );

    let stdout = String::from_utf8(output.stdout).expect("JSON output should be UTF-8");
    let value: serde_json::Value = serde_json::from_str(&stdout).expect("JSON should parse");

    assert_eq!(value, serde_json::json!([]));
}

#[test]
fn task_list_outputs_empty_yaml() {
    let output = genja_command()
        .arg("task")
        .arg("list")
        .arg("--output")
        .arg("yaml")
        .output()
        .expect("genja task list --output yaml should run");

    assert!(
        output.status.success(),
        "genja task list --output yaml should succeed: {output:?}"
    );

    let stdout = String::from_utf8(output.stdout).expect("YAML output should be UTF-8");
    let value: yaml_serde::Value = yaml_serde::from_str(&stdout).expect("YAML should parse");

    assert_eq!(value, yaml_serde::Value::Sequence(Vec::new()));
}

#[test]
fn task_list_outputs_empty_markdown_table() {
    let output = genja_command()
        .arg("task")
        .arg("list")
        .arg("--output")
        .arg("markdown")
        .output()
        .expect("genja task list --output markdown should run");

    assert!(
        output.status.success(),
        "genja task list --output markdown should succeed: {output:?}"
    );

    let stdout = String::from_utf8(output.stdout).expect("Markdown output should be UTF-8");

    assert!(stdout.contains("| ID | Version | Name | Mode | Constructible | Description |"));
    assert!(stdout.contains("|----|"));
}

#[test]
fn task_list_explicit_table_output_matches_default() {
    let output = genja_command()
        .arg("task")
        .arg("list")
        .arg("--output")
        .arg("table")
        .output()
        .expect("genja task list --output table should run");

    assert!(
        output.status.success(),
        "genja task list --output table should succeed: {output:?}"
    );

    let stdout = String::from_utf8(output.stdout).expect("table output should be UTF-8");

    assert_eq!(stdout, "No registered tasks found.\n");
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
