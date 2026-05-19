use genja_core::settings::{InventoryConfig, OptionsConfig};
use genja_core::{
    ConfigLoadError, InventoryFileKind, InventoryLoadError, Settings, SshConfigError,
};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

struct Context {
    _tempdir: tempfile::TempDir,
    filename: PathBuf,
}

fn write_temp_ssh_config(contents: &str) -> Context {
    let tempdir = tempfile::tempdir().expect("tempdir should be created");
    let unique = format!(
        "sshconfig_test_{}_{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos()
    );
    let filename = tempdir.path().join(unique);
    let mut file = fs::File::create(&filename).expect("ssh config file should be created");
    file.write_all(contents.as_bytes())
        .expect("ssh config should be written");
    Context {
        _tempdir: tempdir,
        filename,
    }
}

#[test]
fn settings_from_file_loads_populated_yaml_via_public_api() {
    let ssh_context = write_temp_ssh_config("Host example\n  HostName example.com\n");
    let tempdir = tempfile::tempdir().expect("tempdir should be created");
    let file_path = tempdir.path().join("settings.yaml");
    let yaml = format!(
        r#"
core:
  raise_on_error: "yes"
inventory:
  plugin: "CustomInventoryPlugin"
  options:
    hosts_file: "./inventory/hosts.yaml"
ssh:
  config_file: "{}"
runner:
  plugin: "serial"
  worker_count: 4
  max_task_depth: 6
  max_connection_attempts: 8
logging:
  enabled: "no"
  level: "debug"
  log_file: "./custom.log"
  to_console: "yes"
"#,
        ssh_context.filename.display()
    );
    fs::write(&file_path, yaml).expect("settings file should be written");

    let settings = Settings::from_file(file_path.to_string_lossy().as_ref())
        .expect("settings should load from yaml");

    assert!(settings.core().raise_on_error());
    assert_eq!(settings.inventory().plugin(), "CustomInventoryPlugin");
    assert_eq!(
        settings.inventory().options().hosts_file(),
        Some("./inventory/hosts.yaml")
    );
    assert_eq!(
        settings.ssh().config_file(),
        Some(ssh_context.filename.to_string_lossy().as_ref())
    );
    assert_eq!(settings.runner().plugin(), "serial");
    assert_eq!(settings.runner().worker_count(), Some(4));
    assert_eq!(settings.runner().max_task_depth(), 6);
    assert_eq!(settings.runner().max_connection_attempts(), 8);
    assert!(!settings.logging().enabled());
    assert_eq!(settings.logging().level(), "debug");
    assert_eq!(settings.logging().log_file(), "./custom.log");
    assert!(settings.logging().to_console());
}

#[test]
fn settings_from_file_rejects_invalid_ssh_config_via_public_api() {
    let ssh_context = write_temp_ssh_config("Contents that are not valid ssh config contents\n");
    let tempdir = tempfile::tempdir().expect("tempdir should be created");
    let file_path = tempdir.path().join("settings.yaml");
    let yaml = format!(
        r#"
ssh:
  config_file: "{}"
"#,
        ssh_context.filename.display()
    );
    fs::write(&file_path, yaml).expect("settings file should be written");

    let err = Settings::from_file(file_path.to_string_lossy().as_ref())
        .expect_err("invalid ssh config should fail");

    assert!(matches!(
        err,
        ConfigLoadError::SshConfig(SshConfigError::ParseFailed { .. })
    ));
}

#[test]
fn inventory_config_load_inventory_files_reads_public_files() {
    let tempdir = tempfile::tempdir().expect("tempdir should be created");
    let hosts_path = tempdir.path().join("hosts.yaml");
    let groups_path = tempdir.path().join("groups.yaml");
    let defaults_path = tempdir.path().join("defaults.yaml");

    fs::write(
        &hosts_path,
        r#"
router1:
  hostname: 10.0.0.1
  username: admin
"#,
    )
    .expect("hosts file should be written");
    fs::write(
        &groups_path,
        r#"
core:
  username: netops
"#,
    )
    .expect("groups file should be written");
    fs::write(
        &defaults_path,
        r#"
username: ubuntu
"#,
    )
    .expect("defaults file should be written");

    let config = InventoryConfig::builder()
        .options(
            OptionsConfig::builder()
                .hosts_file(path_str(&hosts_path))
                .groups_file(path_str(&groups_path))
                .defaults_file(path_str(&defaults_path))
                .build(),
        )
        .build();

    let (hosts, groups, defaults) = config
        .load_inventory_files()
        .expect("inventory files should load");

    assert_eq!(hosts.len(), 1);
    assert_eq!(
        hosts.get("router1").and_then(|host| host.hostname()),
        Some("10.0.0.1")
    );

    let groups = groups.expect("groups should be loaded");
    assert_eq!(groups.len(), 1);
    assert_eq!(
        groups.get("core").and_then(|group| group.username()),
        Some("netops")
    );

    let defaults = defaults.expect("defaults should be loaded");
    assert_eq!(defaults.username(), Some("ubuntu"));
}

#[test]
fn inventory_config_load_inventory_files_surfaces_typed_errors_publicly() {
    let tempdir = tempfile::tempdir().expect("tempdir should be created");
    let missing = tempdir.path().join("missing.json");
    let config = InventoryConfig::builder()
        .options(
            OptionsConfig::builder()
                .hosts_file(path_str(&missing))
                .build(),
        )
        .build();

    let err = config
        .load_inventory_files()
        .expect_err("missing hosts file should fail");

    assert!(matches!(
        err,
        InventoryLoadError::Read {
            kind: InventoryFileKind::Hosts,
            ..
        }
    ));
}

fn path_str(path: &Path) -> &str {
    path.to_str().expect("path should be valid utf-8")
}
