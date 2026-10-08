//! Python task descriptor discovery through the `genja-py` helper process.
//!
//! This source launches the helper only when a caller requests descriptors.
//! It leaves project configuration detection to the caller.

use std::env;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use super::{
    DiscoveryError, DiscoveryResult, TaskDescriptor, TaskDescriptorSource, TaskImplementation,
    TaskLanguage,
};

/// Task descriptor source backed by configured Python modules in a project.
///
/// When queried, this source runs the selected interpreter with
/// `-m genja._discovery_cli --pyproject <path>` from the directory containing
/// the project file. It parses descriptor JSON from stdout and uses stderr for
/// failure diagnostics. The selected Python environment must have `genja-py`
/// installed; this source does not check whether Python tasks are configured.
#[derive(Debug, Clone)]
pub struct PythonTaskDescriptorSource {
    python: PathBuf,
    pyproject: PathBuf,
}

impl PythonTaskDescriptorSource {
    /// Create a source for the selected interpreter and project file.
    ///
    /// Relative paths are resolved from the process working directory when
    /// [`TaskDescriptorSource::list_tasks`] or
    /// [`TaskDescriptorSource::list_implementations`] runs, rather than when
    /// this constructor is called. A bare interpreter name such as `python`
    /// is resolved through `PATH`.
    pub fn new(python: impl Into<PathBuf>, pyproject: impl Into<PathBuf>) -> Self {
        Self {
            python: python.into(),
            pyproject: pyproject.into(),
        }
    }

    fn discover(&self) -> DiscoveryResult<Vec<TaskDescriptor>> {
        let caller_directory = env::current_dir().map_err(|error| {
            DiscoveryError::source_failed(format!(
                "cannot determine the working directory for Python task discovery: {error}"
            ))
        })?;
        let pyproject = absolute_path(&self.pyproject, &caller_directory);
        let project_directory = pyproject
            .parent()
            .expect("an absolute project file path has a parent");
        let python = interpreter_path(&self.python, &caller_directory);

        let output = Command::new(&python)
            .arg("-m")
            .arg("genja._discovery_cli")
            .arg("--pyproject")
            .arg(&pyproject)
            .current_dir(project_directory)
            .output()
            .map_err(|error| {
                DiscoveryError::source_failed(format!(
                    "cannot start Python task discovery with `{}` for `{}`: {error}",
                    python.display(),
                    pyproject.display()
                ))
            })?;

        if !output.status.success() {
            let diagnostics = String::from_utf8_lossy(&output.stderr);
            let diagnostics = diagnostics.trim();
            let suffix = if diagnostics.is_empty() {
                String::new()
            } else {
                format!("; stderr: {diagnostics}")
            };
            return Err(DiscoveryError::source_failed(format!(
                "Python task discovery with `{}` for `{}` exited with {}{suffix}",
                python.display(),
                pyproject.display(),
                output.status
            )));
        }

        serde_json::from_slice(&output.stdout).map_err(|error| {
            DiscoveryError::source_failed(format!(
                "invalid descriptor JSON from Python task discovery with `{}` for `{}`: {error}",
                python.display(),
                pyproject.display()
            ))
        })
    }
}

impl TaskDescriptorSource for PythonTaskDescriptorSource {
    fn list_tasks(&self) -> DiscoveryResult<Vec<TaskDescriptor>> {
        self.discover()
    }

    fn list_implementations(&self) -> DiscoveryResult<Vec<TaskImplementation>> {
        Ok(self
            .discover()?
            .into_iter()
            .map(|descriptor| TaskImplementation::new(descriptor, Some(TaskLanguage::Python)))
            .collect())
    }
}

/// Resolve a project path before changing the helper process's working directory.
///
/// The absolute `--pyproject` argument still points to the requested file when
/// the child starts in the project directory.
fn absolute_path(path: &Path, caller_directory: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        caller_directory.join(path)
    }
}

/// Resolve the selected Python interpreter before launching the helper process.
///
/// A bare executable name such as `python` should be left alone so the OS can
/// resolve it from `PATH`. Relative paths like `./venv/bin/python` are made
/// absolute against the caller's directory instead of the later project working
/// directory.
fn interpreter_path(python: &Path, caller_directory: &Path) -> PathBuf {
    if matches!(python.components().next(), Some(Component::Normal(_)))
        && python.components().count() == 1
    {
        python.to_path_buf()
    } else {
        absolute_path(python, caller_directory)
    }
}

#[cfg(all(test, any(unix, windows)))]
mod tests {
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    use genja_core::task::{TaskDescriptorMetadata, TaskExecutionMode};
    use tempfile::TempDir;

    use super::*;

    fn fixture_source() -> (TempDir, PathBuf, PythonTaskDescriptorSource) {
        let directory = tempfile::tempdir().expect("temporary directory should exist");
        let pyproject = directory.path().join("pyproject.toml");
        fs::write(&pyproject, "[tool.genja.tasks]\nmodules = []\n")
            .expect("project file should be written");

        let (interpreter_name, fixture_name) = if cfg!(windows) {
            ("fake-python.cmd", "fake_python_discovery.cmd")
        } else {
            ("fake-python", "fake_python_discovery.sh")
        };
        let interpreter = directory.path().join(interpreter_name);
        fs::copy(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures")
                .join(fixture_name),
            &interpreter,
        )
        .expect("fixture interpreter should be copied");
        #[cfg(unix)]
        {
            let mut permissions = fs::metadata(&interpreter)
                .expect("fixture interpreter should exist")
                .permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&interpreter, permissions)
                .expect("fixture interpreter should be executable");
        }

        let source = PythonTaskDescriptorSource::new(&interpreter, &pyproject);
        (directory, pyproject, source)
    }

    fn descriptor() -> TaskDescriptor {
        TaskDescriptor::explicit(
            "acme.python.backup",
            "1.0.0",
            TaskDescriptorMetadata {
                name: "backup".to_string(),
                description: Some("Back up files".to_string()),
                execution_mode: TaskExecutionMode::Async,
                connection_plugin_name: None,
                processor_names: Vec::new(),
                retry: None,
            },
            None,
            false,
        )
    }

    /// Verify that the helper's JSON payload is parsed successfully and the
    /// discovered descriptors are tagged as Python tasks.
    #[test]
    fn parses_helper_json_and_assigns_python_language() {
        let (directory, pyproject, source) = fixture_source();
        let expected = descriptor();
        fs::write(
            directory.path().join("helper_stdout.txt"),
            serde_json::to_vec(&vec![expected.clone()]).expect("descriptor should serialize"),
        )
        .expect("helper output should be written");
        fs::write(
            directory.path().join("helper_stderr.txt"),
            "import message\n",
        )
        .expect("helper diagnostics should be written");

        let implementations = source
            .list_implementations()
            .expect("Python descriptors should be discovered");

        assert_eq!(implementations.len(), 1);
        assert_eq!(implementations[0].descriptor(), &expected);
        assert_eq!(implementations[0].language(), Some(TaskLanguage::Python));
        assert_eq!(
            fs::read_to_string(directory.path().join("observed_cwd.txt"))
                .expect("working directory should be recorded"),
            directory.path().display().to_string()
        );
        assert_eq!(
            fs::read_to_string(directory.path().join("observed_pyproject.txt"))
                .expect("project path should be recorded"),
            pyproject.display().to_string()
        );
    }

    /// An empty discovery result should be accepted without error.
    #[test]
    fn accepts_empty_helper_list() {
        let (directory, _, source) = fixture_source();
        fs::write(directory.path().join("helper_stdout.txt"), "[]\n")
            .expect("helper output should be written");

        assert_eq!(source.list_tasks(), Ok(Vec::new()));
        assert_eq!(source.list_implementations(), Ok(Vec::new()));
    }

    /// A failed helper execution should surface the interpreter and project
    /// context instead of returning partial task data.
    #[test]
    fn reports_process_failure_without_returning_stdout_descriptors() {
        let (directory, _, source) = fixture_source();
        fs::write(directory.path().join("helper_stdout.txt"), "[]\n")
            .expect("helper output should be written");
        fs::write(
            directory.path().join("helper_stderr.txt"),
            "failed to import backups\n",
        )
        .expect("helper diagnostics should be written");
        fs::write(directory.path().join("helper_exit_code.txt"), "7\n")
            .expect("helper exit status should be written");

        let error = source
            .list_implementations()
            .expect_err("failed helper should not return descriptors");
        let message = error.to_string();
        assert!(message.contains("7"), "{message}");
        assert!(message.contains("failed to import backups"), "{message}");
        assert!(message.contains("pyproject.toml"), "{message}");
    }

    /// Malformed JSON from the helper should be wrapped in a structured
    /// discovery error for the caller.
    #[test]
    fn reports_invalid_helper_json() {
        let (directory, _, source) = fixture_source();
        fs::write(
            directory.path().join("helper_stdout.txt"),
            "import message\n[]\n",
        )
        .expect("helper output should be written");

        let error = source
            .list_tasks()
            .expect_err("malformed helper output should fail");
        assert!(
            error.to_string().contains("invalid descriptor JSON"),
            "{error}"
        );
    }

    /// Missing interpreters should include both the interpreter path and the
    /// project file in the error message.
    #[test]
    fn reports_missing_interpreter_with_context() {
        let (directory, pyproject, _) = fixture_source();
        let missing = directory.path().join("missing-python");
        let source = PythonTaskDescriptorSource::new(&missing, &pyproject);

        let error = source
            .list_tasks()
            .expect_err("missing interpreter should fail");
        let message = error.to_string();
        assert!(
            message.contains(&missing.display().to_string()),
            "{message}"
        );
        assert!(
            message.contains(&pyproject.display().to_string()),
            "{message}"
        );
    }

    /// Relative interpreter paths should be anchored to the caller's directory
    /// before the helper process changes its working directory.
    #[test]
    fn relative_interpreter_paths_are_resolved_before_changing_directory() {
        let caller_directory = Path::new(if cfg!(windows) {
            r"C:\caller"
        } else {
            "/caller"
        });

        assert_eq!(
            interpreter_path(Path::new("python"), caller_directory),
            PathBuf::from("python")
        );
        assert_eq!(
            interpreter_path(Path::new("./venv/bin/python"), caller_directory),
            caller_directory.join("venv/bin/python")
        );
    }
}
