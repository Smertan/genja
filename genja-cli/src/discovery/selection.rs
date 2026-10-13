//! Select the descriptor source for a project configuration.

use std::path::PathBuf;

use super::combined::CombinedTaskDescriptorSource;
use super::config::has_python_task_configuration;
use super::python::PythonTaskDescriptorSource;
use super::rust::CompiledTaskDescriptorSource;
use super::{DiscoveryResult, TaskDescriptorSource};

/// Select compiled or combined discovery for a project file.
///
/// A missing project file or absent `[tool.genja.tasks]` section selects only
/// compiled Rust tasks. When the section is present, the selected Python
/// interpreter is used when descriptors are requested. Invalid project TOML
/// is reported during selection.
pub fn select_task_descriptor_source(
    python: impl Into<PathBuf>,
    pyproject: impl Into<PathBuf>,
) -> DiscoveryResult<Box<dyn TaskDescriptorSource>> {
    select_task_descriptor_source_with_python(pyproject, || Ok(python.into()))
}

/// Resolve the interpreter only when the project configures Python tasks.
pub(crate) fn select_task_descriptor_source_with_python(
    pyproject: impl Into<PathBuf>,
    python: impl FnOnce() -> DiscoveryResult<PathBuf>,
) -> DiscoveryResult<Box<dyn TaskDescriptorSource>> {
    let pyproject = pyproject.into();
    let compiled = CompiledTaskDescriptorSource::new();

    if has_python_task_configuration(&pyproject)? {
        let python = PythonTaskDescriptorSource::new(python()?, pyproject);
        Ok(Box::new(CombinedTaskDescriptorSource::new(
            compiled, python,
        )))
    } else {
        Ok(Box::new(compiled))
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn rust_only_project_does_not_start_python() {
        let directory = tempfile::tempdir().expect("temporary directory should exist");
        let pyproject = directory.path().join("pyproject.toml");
        fs::write(&pyproject, "[project]\nname = 'rust-only'\n")
            .expect("project file should be written");

        let source =
            select_task_descriptor_source(directory.path().join("missing-python"), &pyproject)
                .expect("Rust-only source should be selected");
        source
            .list_implementations()
            .expect("Rust-only discovery should not launch Python");
    }

    #[test]
    fn rust_only_project_does_not_resolve_python() {
        let directory = tempfile::tempdir().expect("temporary directory should exist");
        let pyproject = directory.path().join("pyproject.toml");
        fs::write(&pyproject, "[project]\nname = 'rust-only'\n")
            .expect("project file should be written");

        select_task_descriptor_source_with_python(&pyproject, || {
            panic!("Rust-only discovery must not resolve a Python interpreter")
        })
        .expect("Rust-only source should be selected");
    }

    #[test]
    fn configured_project_selects_python_discovery() {
        let directory = tempfile::tempdir().expect("temporary directory should exist");
        let pyproject = directory.path().join("pyproject.toml");
        fs::write(&pyproject, "[tool.genja.tasks]\nmodules = []\n")
            .expect("project file should be written");
        let missing = directory.path().join("missing-python");

        let source = select_task_descriptor_source(&missing, &pyproject)
            .expect("combined source should be selected");
        let error = source
            .list_implementations()
            .expect_err("configured source should launch Python");
        assert!(error.to_string().contains("missing-python"), "{error}");
    }
}
