use super::env_defaults::{
    deserialize_bool_loose, get_default_log_file, get_inventory_plugin_config,
    get_log_level_default, get_log_to_console_default, get_runner_plugin_default, parse_bool_loose,
    raise_on_error, ENV_INVENTORY_PLUGIN, ENV_LOG_FILE, ENV_LOG_LEVEL, ENV_LOG_TO_CONSOLE,
    ENV_RAISE_ON_ERROR, ENV_RUNNER_PLUGIN,
};
use super::{OptionsConfig, RunnerConfig, SSHConfig};
use regex::Regex;
use serde_json::json;
use std::env;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

/// Returns a static reference to a mutex used for synchronizing environment variable access in tests.
///
/// This function ensures that tests modifying environment variables do not run concurrently,
/// preventing race conditions and test interference. The mutex is initialized once and reused
/// across all test invocations.
///
/// # Returns
///
/// A static reference to a `Mutex<()>` that can be locked to serialize environment variable operations.
fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn set_env_var(key: &str, val: &str) {
    // Tests using this helper hold `env_lock`, so process-wide environment
    // mutation is serialized within this test module.
    unsafe { env::set_var(key, val) };
}

fn remove_env_var(key: &str) {
    // Tests using this helper hold `env_lock`, so process-wide environment
    // mutation is serialized within this test module.
    unsafe { env::remove_var(key) };
}

/// Temporarily sets or removes an environment variable for the duration of a test function.
///
/// This function provides a safe way to test code that depends on environment variables by:
/// 1. Acquiring an exclusive lock to prevent concurrent environment modifications
/// 2. Saving the current value of the environment variable
/// 3. Setting or removing the environment variable as specified
/// 4. Executing the provided test function
/// 5. Restoring the original environment variable state
///
/// # Parameters
///
/// * `key` - The name of the environment variable to modify
/// * `val` - The value to set for the environment variable. If `Some(value)`, the variable
///   is set to that value. If `None`, the variable is removed from the environment.
/// * `f` - A closure containing the test code to execute with the modified environment variable
fn with_env_var(key: &str, val: Option<&str>, f: impl FnOnce()) {
    let _guard = env_lock().lock().unwrap();
    let prev = env::var(key).ok();
    match val {
        Some(v) => set_env_var(key, v),
        None => remove_env_var(key),
    }
    f();
    match prev {
        Some(v) => set_env_var(key, &v),
        None => remove_env_var(key),
    }
}

struct Context {
    _tempdir: tempfile::TempDir,
    filename: PathBuf,
}

fn write_temp_ssh_config(contents: &str) -> Context {
    let tempdir = tempfile::tempdir().unwrap();
    let unique = format!(
        "sshconfig_test_{}_{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let filename = tempdir.path().join(unique);
    let mut file = std::fs::File::create(&filename).unwrap();
    file.write_all(contents.as_bytes()).unwrap();
    Context {
        _tempdir: tempdir,
        filename,
    }
}

#[test]
fn validate_ok_with_valid_config() {
    let context = write_temp_ssh_config("Host example\n  HostName example.com\n");
    let ssh_config = SSHConfig {
        config_file: Some(context.filename.to_string_lossy().to_string()),
    };

    let result = ssh_config.validate();
    assert!(result.is_ok());
}

#[test]
fn validate_ok_with_no_config_file() {
    let ssh_config = SSHConfig { config_file: None };
    assert!(ssh_config.validate().is_ok());
}

#[test]
fn validate_err_with_invalid_config() {
    let context = write_temp_ssh_config("Contents that are not valid ssh config contents\n");
    let ssh_config = SSHConfig {
        config_file: Some(context.filename.to_string_lossy().to_string()),
    };
    let result = ssh_config.validate();
    assert!(matches!(
        result,
        Err(crate::SshConfigError::ParseFailed { .. })
    ));
    let pattern =
        Regex::new(r"Failed to parse SSH config file \S+: unknown field: Contents").unwrap();
    assert!(pattern.is_match(&result.unwrap_err().to_string()));
}

#[test]
fn parse_returns_config_when_present() {
    let context = write_temp_ssh_config("Host example\n  HostName example.com\n");
    let ssh_config = SSHConfig {
        config_file: Some(context.filename.to_string_lossy().to_string()),
    };

    let result = ssh_config.parse();
    assert!(matches!(result, Ok(Some(_))));
}

#[test]
fn parse_returns_none_when_missing() {
    let ssh_config = SSHConfig { config_file: None };
    assert!(matches!(ssh_config.parse(), Ok(None)));
}

#[test]
fn ensure_exists_returns_ok_when_present() {
    let context = write_temp_ssh_config("Host example\n  HostName example.com\n");
    let ssh_config = SSHConfig {
        config_file: Some(context.filename.to_string_lossy().to_string()),
    };

    let result = ssh_config.ensure_exists(&context.filename);
    assert!(result.is_ok());
}

#[test]
fn ensure_exists_returns_err_when_missing() {
    let ssh_config = SSHConfig { config_file: None };
    let result = ssh_config.ensure_exists(Path::new("nonexistent_file.txt"));
    assert!(matches!(
        result,
        Err(crate::SshConfigError::NotFound { .. })
    ));
    assert_eq!(
        result.unwrap_err().to_string(),
        "SSH config file not found: nonexistent_file.txt"
    );
}

#[test]
fn options_config_default_is_all_none() {
    let options = OptionsConfig::default();
    assert!(options.hosts_file.is_none());
    assert!(options.groups_file.is_none());
    assert!(options.defaults_file.is_none());
}

#[test]
fn options_config_deserializes_empty_object_to_none() {
    let options: OptionsConfig = serde_json::from_str("{}").unwrap();
    assert!(options.hosts_file.is_none());
    assert!(options.groups_file.is_none());
    assert!(options.defaults_file.is_none());
}

#[test]
fn options_config_deserializes_with_values() {
    let json = r#"{
        "hosts_file": "/tmp/hosts.yaml",
        "groups_file": "/tmp/groups.yaml",
        "defaults_file": "/tmp/defaults.yaml"
    }"#;
    let options: OptionsConfig = serde_json::from_str(json).unwrap();
    assert_eq!(options.hosts_file.as_deref(), Some("/tmp/hosts.yaml"));
    assert_eq!(options.groups_file.as_deref(), Some("/tmp/groups.yaml"));
    assert_eq!(options.defaults_file.as_deref(), Some("/tmp/defaults.yaml"));
}

#[test]
fn runner_config_default_values() {
    let runner = RunnerConfig::default();
    assert_eq!(runner.plugin(), "threaded");
    assert_eq!(runner.options(), &json!({}));
    assert_eq!(runner.worker_count(), None);
    assert_eq!(runner.max_task_depth(), 10);
    assert_eq!(runner.max_connection_attempts(), 3);
}

#[test]
fn runner_config_deserializes_empty_object_to_defaults() {
    let runner: RunnerConfig = serde_json::from_str("{}").unwrap();
    assert_eq!(runner.plugin(), "threaded");
    assert_eq!(runner.options(), &json!({}));
    assert_eq!(runner.worker_count(), None);
    assert_eq!(runner.max_task_depth(), 10);
    assert_eq!(runner.max_connection_attempts(), 3);
}

#[test]
fn runner_config_deserializes_with_values() {
    let json = r#"{
        "plugin": "custom",
        "options": {"queue": "fast", "strategy": "burst"},
        "worker_count": 6,
        "max_task_depth": 5,
        "max_connection_attempts": 7
    }"#;
    let runner: RunnerConfig = serde_json::from_str(json).unwrap();
    assert_eq!(runner.plugin(), "custom");
    assert_eq!(
        runner.options(),
        &json!({"queue": "fast", "strategy": "burst"})
    );
    assert_eq!(runner.worker_count(), Some(6));
    assert_eq!(runner.max_task_depth(), 5);
    assert_eq!(runner.max_connection_attempts(), 7);
}

#[test]
fn runner_config_builder_sets_max_connection_attempts() {
    let runner = RunnerConfig::builder().max_connection_attempts(9).build();

    assert_eq!(runner.max_connection_attempts(), 9);
}

#[test]
fn runner_config_builder_sets_worker_count() {
    let runner = RunnerConfig::builder().worker_count(4).build();

    assert_eq!(runner.worker_count(), Some(4));
}

#[test]
fn parse_bool_loose_accepts_common_values() {
    assert_eq!(parse_bool_loose("true"), Some(true));
    assert_eq!(parse_bool_loose("TrUe"), Some(true));
    assert_eq!(parse_bool_loose("1"), Some(true));
    assert_eq!(parse_bool_loose("yes"), Some(true));
    assert_eq!(parse_bool_loose("on"), Some(true));
    assert_eq!(parse_bool_loose("false"), Some(false));
    assert_eq!(parse_bool_loose("0"), Some(false));
    assert_eq!(parse_bool_loose("no"), Some(false));
    assert_eq!(parse_bool_loose("off"), Some(false));
    assert_eq!(parse_bool_loose("maybe"), None);
}

#[test]
fn deserialize_bool_loose_from_string_and_bool() {
    #[derive(serde::Deserialize)]
    struct T {
        #[serde(deserialize_with = "deserialize_bool_loose")]
        v: bool,
    }

    let t: T = serde_json::from_str(r#"{ "v": "yes" }"#).unwrap();
    assert!(t.v);
    let t: T = serde_json::from_str(r#"{ "v": false }"#).unwrap();
    assert!(!t.v);
}

#[test]
fn deserialize_bool_loose_rejects_invalid_string() {
    #[derive(serde::Deserialize, Debug)]
    struct T {
        #[serde(deserialize_with = "deserialize_bool_loose")]
        _v: bool,
    }

    let err = serde_json::from_str::<T>(r#"{ "_v": "maybe" }"#).unwrap_err();
    assert!(err.to_string().contains("invalid boolean value"));
}

#[test]
fn raise_on_error_uses_env_and_fallbacks() {
    with_env_var(ENV_RAISE_ON_ERROR, Some("true"), || {
        assert!(raise_on_error());
    });
    with_env_var(ENV_RAISE_ON_ERROR, Some("not_a_bool"), || {
        assert!(!raise_on_error());
    });
    with_env_var(ENV_RAISE_ON_ERROR, None, || {
        assert!(!raise_on_error());
    });
}

#[test]
fn get_log_to_console_default_parses_env() {
    with_env_var(ENV_LOG_TO_CONSOLE, Some("yes"), || {
        assert!(get_log_to_console_default());
    });
    with_env_var(ENV_LOG_TO_CONSOLE, Some("no"), || {
        assert!(!get_log_to_console_default());
    });
}

#[test]
fn env_string_defaults_respect_env_and_fallbacks() {
    with_env_var(ENV_INVENTORY_PLUGIN, Some("CustomInv"), || {
        assert_eq!(get_inventory_plugin_config(), "CustomInv");
    });
    with_env_var(ENV_INVENTORY_PLUGIN, None, || {
        assert_eq!(get_inventory_plugin_config(), "FileInventoryPlugin");
    });

    with_env_var(ENV_RUNNER_PLUGIN, Some("CustomRunner"), || {
        assert_eq!(get_runner_plugin_default(), "CustomRunner");
    });
    with_env_var(ENV_RUNNER_PLUGIN, None, || {
        assert_eq!(get_runner_plugin_default(), "threaded");
    });

    with_env_var(ENV_LOG_LEVEL, Some("debug"), || {
        assert_eq!(get_log_level_default(), "debug");
    });
    with_env_var(ENV_LOG_LEVEL, None, || {
        assert_eq!(get_log_level_default(), "info");
    });
}

#[test]
fn get_default_log_file_prefers_env() {
    with_env_var(ENV_LOG_FILE, Some("/tmp/genja-test.log"), || {
        assert_eq!(get_default_log_file(), "/tmp/genja-test.log");
    });
}

#[test]
fn get_default_log_file_uses_cwd_when_env_missing() {
    let _guard = env_lock().lock().unwrap();
    let prev = env::var(ENV_LOG_FILE).ok();
    remove_env_var(ENV_LOG_FILE);

    let tempdir = tempfile::tempdir().unwrap();
    let prev_dir = env::current_dir().unwrap();
    env::set_current_dir(tempdir.path()).unwrap();

    let expected = tempdir.path().join("genja.log");
    assert_eq!(
        get_default_log_file(),
        expected.to_string_lossy().to_string()
    );

    env::set_current_dir(prev_dir).unwrap();
    match prev {
        Some(v) => set_env_var(ENV_LOG_FILE, &v),
        None => remove_env_var(ENV_LOG_FILE),
    }
}

#[test]
fn settings_from_file_errors_when_missing() {
    let tempdir = tempfile::tempdir().unwrap();
    let missing = tempdir.path().join("missing.yaml");
    let err = super::Settings::from_file(missing.to_string_lossy().as_ref()).unwrap_err();
    assert!(err.to_string().to_lowercase().contains("not found"));
}

#[test]
fn settings_from_file_errors_on_unsupported_extension() {
    let tempdir = tempfile::tempdir().unwrap();
    let file_path = tempdir.path().join("config.txt");
    std::fs::write(&file_path, "{}").unwrap();
    let err = super::Settings::from_file(file_path.to_string_lossy().as_ref()).unwrap_err();
    assert!(matches!(
        err,
        crate::ConfigLoadError::UnsupportedFormat { .. }
    ));
}

#[test]
fn settings_from_file_errors_on_invalid_json() {
    let tempdir = tempfile::tempdir().unwrap();
    let file_path = tempdir.path().join("config.json");
    std::fs::write(&file_path, "{ not valid json").unwrap();
    let err = super::Settings::from_file(file_path.to_string_lossy().as_ref()).unwrap_err();
    assert!(!err.to_string().is_empty());
}

#[test]
fn settings_from_file_uses_defaults_for_empty_file() {
    let _guard = env_lock().lock().unwrap();
    let keys = [
        ENV_RAISE_ON_ERROR,
        ENV_INVENTORY_PLUGIN,
        ENV_RUNNER_PLUGIN,
        ENV_LOG_LEVEL,
        ENV_LOG_FILE,
        ENV_LOG_TO_CONSOLE,
    ];

    let prev: Vec<(String, Option<String>)> = keys
        .iter()
        .map(|key| (key.to_string(), env::var(key).ok()))
        .collect();

    for key in keys {
        remove_env_var(key);
    }

    let tempdir = tempfile::tempdir().unwrap();
    let file_path = tempdir.path().join("config.yaml");
    std::fs::write(&file_path, "{}").unwrap();
    let settings = super::Settings::from_file(file_path.to_string_lossy().as_ref()).unwrap();

    assert!(!settings.core().raise_on_error());
    assert_eq!(settings.inventory().plugin(), "FileInventoryPlugin");
    assert_eq!(settings.runner().plugin(), "threaded");
    assert_eq!(settings.logging().level(), "info");
    assert!(settings.logging().enabled());

    for (key, val) in prev {
        match val {
            Some(v) => set_env_var(&key, &v),
            None => remove_env_var(&key),
        }
    }
}

#[test]
fn settings_from_file_loads_populated_yaml() {
    let ssh_context = write_temp_ssh_config("Host example\n  HostName example.com\n");
    let tempdir = tempfile::tempdir().unwrap();
    let file_path = tempdir.path().join("config.yaml");
    let yaml = format!(
        r#"
core:
  raise_on_error: "yes"
inventory:
  plugin: "CustomInventoryPlugin"
  options:
    hosts_file: "./inventory/hosts.yaml"
    groups_file: "./inventory/groups.yaml"
    defaults_file: "./inventory/defaults.yaml"
  transform_function: "normalize_inventory"
  transform_function_options:
    mode: "strict"
    retries: 2
ssh:
  config_file: "{}"
runner:
  plugin: "serial"
  options:
    queue: "fast"
  worker_count: 6
  max_task_depth: 5
  max_connection_attempts: 7
logging:
  enabled: "no"
  level: "debug"
  log_file: "./custom.log"
  to_console: "yes"
  file_size: 2048
  max_file_count: 4
"#,
        ssh_context.filename.display()
    );
    std::fs::write(&file_path, yaml).unwrap();

    let settings = super::Settings::from_file(file_path.to_string_lossy().as_ref()).unwrap();

    assert!(settings.core().raise_on_error());
    assert_eq!(settings.inventory().plugin(), "CustomInventoryPlugin");
    assert_eq!(
        settings.inventory().options().hosts_file(),
        Some("./inventory/hosts.yaml")
    );
    assert_eq!(
        settings.inventory().options().groups_file(),
        Some("./inventory/groups.yaml")
    );
    assert_eq!(
        settings.inventory().options().defaults_file(),
        Some("./inventory/defaults.yaml")
    );
    assert_eq!(
        settings.inventory().transform_function(),
        Some("normalize_inventory")
    );
    assert_eq!(
        settings
            .inventory()
            .transform_function_options()
            .and_then(|options| options.get("mode"))
            .and_then(|value| value.as_str()),
        Some("strict")
    );
    assert_eq!(
        settings
            .inventory()
            .transform_function_options()
            .and_then(|options| options.get("retries"))
            .and_then(|value| value.as_u64()),
        Some(2)
    );
    assert_eq!(
        settings.ssh().config_file(),
        Some(ssh_context.filename.to_string_lossy().as_ref())
    );
    assert_eq!(settings.runner().plugin(), "serial");
    assert_eq!(settings.runner().options(), &json!({ "queue": "fast" }));
    assert_eq!(settings.runner().worker_count(), Some(6));
    assert_eq!(settings.runner().max_task_depth(), 5);
    assert_eq!(settings.runner().max_connection_attempts(), 7);
    assert!(!settings.logging().enabled());
    assert_eq!(settings.logging().level(), "debug");
    assert_eq!(settings.logging().log_file(), "./custom.log");
    assert!(settings.logging().to_console());
    assert_eq!(settings.logging().file_size(), 2048);
    assert_eq!(settings.logging().max_file_count(), 4);
}

#[test]
fn settings_from_file_errors_on_invalid_ssh_config() {
    let ssh_context = write_temp_ssh_config("Contents that are not valid ssh config contents\n");
    let tempdir = tempfile::tempdir().unwrap();
    let file_path = tempdir.path().join("config.yaml");
    let yaml = format!(
        r#"
ssh:
  config_file: "{}"
"#,
        ssh_context.filename.display()
    );
    std::fs::write(&file_path, yaml).unwrap();

    let err = super::Settings::from_file(file_path.to_string_lossy().as_ref()).unwrap_err();
    assert!(matches!(
        err,
        crate::ConfigLoadError::SshConfig(crate::SshConfigError::ParseFailed { .. })
    ));
}

#[test]
fn inventory_loads_empty_files() {
    let tempdir = tempfile::tempdir().unwrap();
    let hosts_path = tempdir.path().join("hosts.json");
    let groups_path = tempdir.path().join("groups.json");
    let defaults_path = tempdir.path().join("defaults.json");
    std::fs::write(&hosts_path, "{}").unwrap();
    std::fs::write(&groups_path, "{}").unwrap();
    std::fs::write(&defaults_path, "{}").unwrap();

    let options = super::OptionsConfig::builder()
        .hosts_file(hosts_path.to_string_lossy().as_ref())
        .groups_file(groups_path.to_string_lossy().as_ref())
        .defaults_file(defaults_path.to_string_lossy().as_ref())
        .build();
    let config = super::InventoryConfig::builder().options(options).build();

    let (hosts, groups, defaults) = config.load_inventory_files().unwrap();
    assert!(hosts.is_empty());
    assert!(groups.unwrap().is_empty());
    assert!(defaults.unwrap().is_empty());
}

#[test]
fn inventory_load_errors_on_missing_file() {
    let tempdir = tempfile::tempdir().unwrap();
    let missing = tempdir.path().join("missing.json");
    let options = super::OptionsConfig::builder()
        .hosts_file(missing.to_string_lossy().as_ref())
        .build();
    let config = super::InventoryConfig::builder().options(options).build();
    let err = config.load_inventory_files().unwrap_err();
    assert!(matches!(
        err,
        crate::InventoryLoadError::Read {
            kind: crate::InventoryFileKind::Hosts,
            ..
        }
    ));
}

#[test]
fn inventory_load_errors_on_unsupported_extension() {
    let tempdir = tempfile::tempdir().unwrap();
    let file_path = tempdir.path().join("hosts.txt");
    std::fs::write(&file_path, "{}").unwrap();
    let options = super::OptionsConfig::builder()
        .hosts_file(file_path.to_string_lossy().as_ref())
        .build();
    let config = super::InventoryConfig::builder().options(options).build();
    let err = config.load_inventory_files().unwrap_err();
    assert!(matches!(
        err,
        crate::InventoryLoadError::UnsupportedFormat {
            kind: crate::InventoryFileKind::Hosts,
            ..
        }
    ));
}
