//! Combined discovery of compiled Rust and configured Python tasks.

use super::config::has_python_task_configuration;
use super::python::PythonTaskDescriptorSource;
use super::rust::CompiledTaskDescriptorSource;
use super::{
    DiscoveryError, DiscoveryResult, TaskDescriptor, TaskDescriptorSource, TaskImplementation,
    TaskRegistrationKey, parse_task_descriptor_identity,
};

/// Descriptor source that combines compiled Rust tasks with configured Python tasks.
///
/// The Python helper is only started when the project file contains
/// `[tool.genja.tasks]`. Both languages may provide the same descriptor ID and
/// version; callers should use [`TaskDescriptorSource::list_implementations`]
/// when they need to distinguish those implementations.
#[derive(Debug, Clone)]
pub struct CombinedTaskDescriptorSource {
    compiled: CompiledTaskDescriptorSource,
    python: PythonTaskDescriptorSource,
}

impl CombinedTaskDescriptorSource {
    /// Combine compiled Rust discovery with a configured Python source.
    pub fn new(compiled: CompiledTaskDescriptorSource, python: PythonTaskDescriptorSource) -> Self {
        Self { compiled, python }
    }
}

impl TaskDescriptorSource for CombinedTaskDescriptorSource {
    fn list_tasks(&self) -> DiscoveryResult<Vec<TaskDescriptor>> {
        Ok(self
            .list_implementations()?
            .into_iter()
            .map(|implementation| implementation.descriptor().clone())
            .collect())
    }

    fn list_implementations(&self) -> DiscoveryResult<Vec<TaskImplementation>> {
        let configured = has_python_task_configuration(self.python.pyproject_path())?;
        let rust = self.compiled.list_implementations()?;
        let python = if configured {
            self.python.list_implementations()?
        } else {
            Vec::new()
        };
        merge_implementations(rust, python)
    }

    fn describe_task(&self, identity: &str) -> DiscoveryResult<TaskDescriptor> {
        let identity = parse_task_descriptor_identity(identity)?;
        let implementations = self.list_implementations()?;
        find_unique_descriptor(&implementations, identity.id(), identity.version())
    }

    fn describe_task_by_key(&self, key: &TaskRegistrationKey) -> DiscoveryResult<TaskDescriptor> {
        let implementations = self.list_implementations()?;
        find_unique_descriptor(&implementations, key.id(), key.version())
    }
}

fn merge_implementations(
    mut rust: Vec<TaskImplementation>,
    mut python: Vec<TaskImplementation>,
) -> DiscoveryResult<Vec<TaskImplementation>> {
    rust.append(&mut python);
    rust.sort_by(|left, right| {
        left.descriptor()
            .id
            .cmp(&right.descriptor().id)
            .then_with(|| left.descriptor().version.cmp(&right.descriptor().version))
            .then_with(|| left.language().cmp(&right.language()))
    });

    for pair in rust.windows(2) {
        let left = &pair[0];
        let right = &pair[1];
        if left.descriptor().id == right.descriptor().id
            && left.descriptor().version == right.descriptor().version
            && left.language() == right.language()
        {
            let language = left.language().ok_or_else(|| {
                DiscoveryError::source_failed(
                    "combined discovery received a task implementation without a language",
                )
            })?;
            return Err(DiscoveryError::DuplicateImplementation {
                id: left.descriptor().id.clone(),
                version: left.descriptor().version.clone(),
                language,
            });
        }
    }

    Ok(rust)
}

fn find_unique_descriptor(
    implementations: &[TaskImplementation],
    id: &str,
    version: &str,
) -> DiscoveryResult<TaskDescriptor> {
    let matching = implementations
        .iter()
        .filter(|implementation| {
            implementation.descriptor().id == id && implementation.descriptor().version == version
        })
        .collect::<Vec<_>>();

    match matching.as_slice() {
        [] => Err(DiscoveryError::NotFound {
            id: id.to_string(),
            version: Some(version.to_string()),
        }),
        [implementation] => Ok(implementation.descriptor().clone()),
        _ => Err(DiscoveryError::AmbiguousImplementation {
            id: id.to_string(),
            version: version.to_string(),
            languages: matching
                .iter()
                .filter_map(|implementation| implementation.language())
                .collect(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    #[cfg(any(unix, windows))]
    use std::path::Path;

    use genja_core::task::{TaskDescriptorMetadata, TaskExecutionMode};

    use super::*;
    use crate::discovery::TaskLanguage;

    fn implementation(id: &str, version: &str, language: TaskLanguage) -> TaskImplementation {
        TaskImplementation::new(
            TaskDescriptor::explicit(
                id,
                version,
                TaskDescriptorMetadata {
                    name: id.replace('.', "_"),
                    description: None,
                    execution_mode: TaskExecutionMode::Async,
                    connection_plugin_name: None,
                    processor_names: Vec::new(),
                    retry: None,
                },
                None,
                false,
            ),
            Some(language),
        )
    }

    #[test]
    fn skips_python_when_project_has_no_task_configuration() {
        let directory = tempfile::tempdir().expect("temporary directory should exist");
        let pyproject = directory.path().join("pyproject.toml");
        fs::write(&pyproject, "[project]\nname = 'rust-only'\n")
            .expect("project file should be written");
        let compiled = CompiledTaskDescriptorSource::new();
        let python =
            PythonTaskDescriptorSource::new(directory.path().join("missing-python"), &pyproject);
        let combined = CombinedTaskDescriptorSource::new(compiled, python);

        assert_eq!(
            combined.list_implementations(),
            compiled.list_implementations()
        );
    }

    #[test]
    fn configured_python_failure_does_not_return_rust_only_results() {
        let directory = tempfile::tempdir().expect("temporary directory should exist");
        let pyproject = directory.path().join("pyproject.toml");
        fs::write(&pyproject, "[tool.genja.tasks]\nmodules = []\n")
            .expect("project file should be written");
        let missing = directory.path().join("missing-python");
        let combined = CombinedTaskDescriptorSource::new(
            CompiledTaskDescriptorSource::new(),
            PythonTaskDescriptorSource::new(&missing, &pyproject),
        );

        let error = combined
            .list_implementations()
            .expect_err("configured Python failure should fail all discovery");
        assert!(error.to_string().contains("missing-python"), "{error}");
    }

    #[test]
    fn malformed_project_file_fails_before_discovery() {
        let directory = tempfile::tempdir().expect("temporary directory should exist");
        let pyproject = directory.path().join("pyproject.toml");
        fs::write(&pyproject, "[tool.genja.tasks\n").expect("project file should be written");
        let combined = CombinedTaskDescriptorSource::new(
            CompiledTaskDescriptorSource::new(),
            PythonTaskDescriptorSource::new("missing-python", &pyproject),
        );

        let error = combined
            .list_implementations()
            .expect_err("invalid configuration should fail discovery");
        assert!(error.to_string().contains("invalid TOML"), "{error}");
    }

    #[test]
    fn merges_in_deterministic_identity_and_language_order() {
        let implementations = merge_implementations(
            vec![
                implementation("acme.zeta", "2.0.0", TaskLanguage::Rust),
                implementation("acme.alpha", "1.0.0", TaskLanguage::Rust),
            ],
            vec![
                implementation("acme.alpha", "2.0.0", TaskLanguage::Python),
                implementation("acme.alpha", "1.0.0", TaskLanguage::Python),
            ],
        )
        .expect("cross-language identities should be allowed");

        let identities = implementations
            .iter()
            .map(|implementation| {
                (
                    implementation.descriptor().id.as_str(),
                    implementation.descriptor().version.as_str(),
                    implementation.language(),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            identities,
            vec![
                ("acme.alpha", "1.0.0", Some(TaskLanguage::Rust)),
                ("acme.alpha", "1.0.0", Some(TaskLanguage::Python)),
                ("acme.alpha", "2.0.0", Some(TaskLanguage::Python)),
                ("acme.zeta", "2.0.0", Some(TaskLanguage::Rust)),
            ]
        );
    }

    #[test]
    fn rejects_duplicate_language_qualified_identity() {
        let duplicate = implementation("acme.alpha", "1.0.0", TaskLanguage::Python);

        assert_eq!(
            merge_implementations(Vec::new(), vec![duplicate.clone(), duplicate]),
            Err(DiscoveryError::DuplicateImplementation {
                id: "acme.alpha".to_string(),
                version: "1.0.0".to_string(),
                language: TaskLanguage::Python,
            })
        );
    }

    #[test]
    fn descriptor_lookup_reports_cross_language_ambiguity() {
        let implementations = merge_implementations(
            vec![implementation("acme.alpha", "1.0.0", TaskLanguage::Rust)],
            vec![implementation("acme.alpha", "1.0.0", TaskLanguage::Python)],
        )
        .expect("cross-language identities should be allowed");

        assert_eq!(
            find_unique_descriptor(&implementations, "acme.alpha", "1.0.0"),
            Err(DiscoveryError::AmbiguousImplementation {
                id: "acme.alpha".to_string(),
                version: "1.0.0".to_string(),
                languages: vec![TaskLanguage::Rust, TaskLanguage::Python],
            })
        );
    }

    #[test]
    fn descriptor_lookup_returns_unique_match_or_not_found() {
        let implementation = implementation("acme.alpha", "1.0.0", TaskLanguage::Python);
        let implementations = vec![implementation.clone()];

        assert_eq!(
            find_unique_descriptor(&implementations, "acme.alpha", "1.0.0"),
            Ok(implementation.descriptor().clone())
        );
        assert_eq!(
            find_unique_descriptor(&implementations, "acme.alpha", "2.0.0"),
            Err(DiscoveryError::NotFound {
                id: "acme.alpha".to_string(),
                version: Some("2.0.0".to_string()),
            })
        );
    }

    #[cfg(any(unix, windows))]
    #[test]
    fn configured_source_keeps_matching_rust_and_python_implementations() {
        const ID: &str = "acme.tests.cli.discovery.compiled_alpha";
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

        let python_descriptor = implementation(ID, "1.0.0", TaskLanguage::Python);
        fs::write(
            directory.path().join("helper_stdout.txt"),
            serde_json::to_vec(&vec![python_descriptor.descriptor()])
                .expect("descriptor should serialize"),
        )
        .expect("helper output should be written");
        let combined = CombinedTaskDescriptorSource::new(
            CompiledTaskDescriptorSource::new(),
            PythonTaskDescriptorSource::new(&interpreter, &pyproject),
        );

        let implementations = combined
            .list_implementations()
            .expect("both sources should discover tasks");
        let languages = implementations
            .iter()
            .filter(|implementation| implementation.descriptor().id == ID)
            .map(|implementation| implementation.language())
            .collect::<Vec<_>>();
        assert_eq!(
            languages,
            vec![Some(TaskLanguage::Rust), Some(TaskLanguage::Python)]
        );

        let expected_error = DiscoveryError::AmbiguousImplementation {
            id: ID.to_string(),
            version: "1.0.0".to_string(),
            languages: vec![TaskLanguage::Rust, TaskLanguage::Python],
        };
        assert_eq!(
            combined.describe_task(&format!("{ID}@1.0.0")),
            Err(expected_error.clone())
        );
        let key = TaskRegistrationKey::parse(&format!("{ID}@1.0.0"))
            .expect("explicit identity should parse");
        assert_eq!(combined.describe_task_by_key(&key), Err(expected_error));
    }
}
