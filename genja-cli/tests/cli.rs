use std::process::Command;

fn genja_command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_genja"))
}

#[test]
fn successful_commands_write_only_to_stdout() {
    let cases: &[&[&str]] = &[
        &["--help"],
        &["version"],
        &["task", "list"],
        &["task", "list", "--output", "json"],
        &["task", "list", "--output", "yaml"],
        &["task", "list", "--output", "markdown"],
        &["task", "docs"],
    ];

    for args in cases {
        let output = genja_command()
            .args(*args)
            .output()
            .expect("CLI should run");
        assert_eq!(output.status.code(), Some(0), "{args:?}: {output:?}");
        assert!(!output.stdout.is_empty(), "{args:?}");
        assert!(output.stderr.is_empty(), "{args:?}: {output:?}");
    }
}

#[test]
fn describe_errors_preserve_clean_stdout_for_every_format() {
    let cases = [
        ("acme.examples.backup_config", "invalid task identity"),
        (
            "acme.examples.backup_config@invalid",
            "invalid task identity",
        ),
        ("acme.examples.missing@1.0.0", "was not found"),
    ];

    for format in ["table", "json", "yaml", "markdown"] {
        for (identity, message) in cases {
            let output = genja_command()
                .args(["task", "describe", identity, "--output", format])
                .output()
                .expect("CLI should run");
            assert_eq!(output.status.code(), Some(1), "{output:?}");
            assert!(output.stdout.is_empty(), "{output:?}");
            let stderr = String::from_utf8(output.stderr).expect("UTF-8 error");
            assert!(stderr.starts_with("error: "), "{stderr}");
            assert!(stderr.contains(message), "{stderr}");
            assert!(stderr.contains(identity), "{stderr}");
        }
    }
}

#[test]
fn argument_errors_exit_with_two_and_write_only_to_stderr() {
    let cases: &[&[&str]] = &[
        &["task", "describe"],
        &["task", "list", "--output", "xml"],
        &[
            "task",
            "describe",
            "acme.examples.backup_config@1.0.0",
            "--output",
            "xml",
        ],
        &["task", "docs", "--output", "json"],
    ];

    for args in cases {
        let output = genja_command()
            .args(*args)
            .output()
            .expect("CLI should run");
        assert_eq!(output.status.code(), Some(2), "{args:?}: {output:?}");
        assert!(output.stdout.is_empty(), "{args:?}: {output:?}");
        assert!(!output.stderr.is_empty(), "{args:?}");
    }
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
    let has_tui_command = stdout
        .lines()
        .any(|line| line.split_whitespace().next() == Some("tui"));
    assert_eq!(has_tui_command, cfg!(feature = "tui"));
}

#[cfg(feature = "tui")]
#[test]
fn tui_help_succeeds_without_starting_a_terminal_session() {
    let output = genja_command()
        .args(["tui", "--help"])
        .output()
        .expect("TUI help should run");

    assert!(output.status.success(), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 help");
    assert!(stdout.contains("Usage: genja tui"), "{stdout}");
    assert!(
        stdout.contains("Open the terminal task browser"),
        "{stdout}"
    );
}

#[cfg(not(feature = "tui"))]
#[test]
fn tui_command_is_unavailable_in_cli_only_builds() {
    let output = genja_command().arg("tui").output().expect("CLI should run");

    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    let stderr = String::from_utf8(output.stderr).expect("UTF-8 error");
    assert!(stderr.contains("unrecognized subcommand 'tui'"), "{stderr}");
}

#[cfg(feature = "tui")]
#[test]
fn tui_startup_errors_report_failure_without_terminal_output() {
    let output = genja_command()
        .arg("tui")
        .stdin(std::process::Stdio::null())
        .output()
        .expect("TUI command should run");

    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    let stderr = String::from_utf8(output.stderr).expect("UTF-8 error");
    assert_eq!(
        stderr.trim_end(),
        "error: terminal error: the TUI requires an interactive terminal"
    );
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
    assert!(stdout.contains("docs"));
    assert!(stdout.contains("list"));
}

#[test]
fn task_docs_help_prints_markdown_output_format() {
    let output = genja_command()
        .arg("task")
        .arg("docs")
        .arg("--help")
        .output()
        .expect("genja task docs --help should run");

    assert!(
        output.status.success(),
        "genja task docs --help should succeed: {output:?}"
    );

    let stdout = String::from_utf8(output.stdout).expect("task docs help should be UTF-8");

    assert!(stdout.contains("Usage: genja task docs"));
    assert!(stdout.contains("--output"));
    assert!(stdout.contains("markdown"));
}

#[test]
fn task_docs_rejects_unsupported_output_format() {
    let output = genja_command()
        .arg("task")
        .arg("docs")
        .arg("--output")
        .arg("json")
        .output()
        .expect("genja task docs should run");

    assert!(
        !output.status.success(),
        "unsupported output format should fail: {output:?}"
    );

    let stderr = String::from_utf8(output.stderr).expect("error output should be UTF-8");

    assert!(stderr.contains("invalid value"));
    assert!(stderr.contains("json"));
    assert!(stderr.contains("markdown"));
}

#[test]
fn task_docs_defaults_to_empty_markdown_catalogue() {
    let output = genja_command()
        .arg("task")
        .arg("docs")
        .output()
        .expect("genja task docs should run");

    assert!(
        output.status.success(),
        "genja task docs should succeed: {output:?}"
    );

    let stdout = String::from_utf8(output.stdout).expect("docs output should be UTF-8");

    assert_eq!(stdout, "# Task Catalogue\n\nNo registered tasks found.\n");
}

#[test]
fn task_docs_accepts_explicit_markdown_output() {
    let output = genja_command()
        .arg("task")
        .arg("docs")
        .arg("--output")
        .arg("markdown")
        .output()
        .expect("genja task docs --output markdown should run");

    assert!(
        output.status.success(),
        "genja task docs --output markdown should succeed: {output:?}"
    );

    let stdout = String::from_utf8(output.stdout).expect("docs output should be UTF-8");

    assert_eq!(stdout, "# Task Catalogue\n\nNo registered tasks found.\n");
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
fn task_describe_rejects_invalid_identity() {
    let output = genja_command()
        .arg("task")
        .arg("describe")
        .arg("acme.examples.backup_config")
        .output()
        .expect("genja task describe should run");

    assert!(
        !output.status.success(),
        "invalid identity should fail: {output:?}"
    );

    let stderr = String::from_utf8(output.stderr).expect("error output should be UTF-8");

    assert!(stderr.contains("error: invalid task identity"));
    assert!(stderr.contains("acme.examples.backup_config"));
    assert!(stderr.contains("exactly one `@` separator"));
}

#[test]
fn task_describe_returns_not_found_for_missing_identity() {
    let output = genja_command()
        .arg("task")
        .arg("describe")
        .arg("acme.examples.missing@1.0.0")
        .output()
        .expect("genja task describe should run");

    assert!(
        !output.status.success(),
        "missing identity should fail: {output:?}"
    );

    let stderr = String::from_utf8(output.stderr).expect("error output should be UTF-8");

    assert!(stderr.contains("error: task descriptor"));
    assert!(stderr.contains("acme.examples.missing@1.0.0"));
    assert!(stderr.contains("was not found"));
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
    assert!(stdout.contains("compact summary views"));
    assert!(stdout.contains("genja task describe <identity>"));
    assert!(stdout.contains("JSON and YAML output"));
    assert!(stdout.contains("full descriptor items"));
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

    assert!(stdout.contains("| ID | Version | Name | Source | Mode | Constructible |"));
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
