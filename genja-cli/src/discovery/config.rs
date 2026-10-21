//! Project configuration check for optional Python task discovery.

use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use super::{DiscoveryError, DiscoveryResult};

/// Return whether a project has Python task configuration in `pyproject.toml`.
///
/// A missing project file or absent `[tool.genja.tasks]` section means Python
/// discovery is not configured. A present section selects the Python helper,
/// which validates the section and its module names.
pub fn has_python_task_configuration(pyproject_path: &Path) -> DiscoveryResult<bool> {
    let contents = match fs::read_to_string(pyproject_path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(false),
        Err(error) => {
            return Err(DiscoveryError::source_failed(format!(
                "cannot read {}: {error}",
                pyproject_path.display()
            )));
        }
    };

    let project: toml::Value = toml::from_str(&contents).map_err(|error| {
        DiscoveryError::source_failed(format!(
            "invalid TOML in {}: {error}",
            pyproject_path.display()
        ))
    })?;

    let Some(tool) = project.get("tool") else {
        return Ok(false);
    };
    let tool = tool
        .as_table()
        .ok_or_else(|| invalid_field(pyproject_path, "[tool]"))?;

    let Some(genja) = tool.get("genja") else {
        return Ok(false);
    };
    let genja = genja
        .as_table()
        .ok_or_else(|| invalid_field(pyproject_path, "[tool.genja]"))?;

    Ok(genja.contains_key("tasks"))
}

fn invalid_field(pyproject_path: &Path, field: &str) -> DiscoveryError {
    DiscoveryError::source_failed(format!(
        "{}: {field} must be a table",
        pyproject_path.display()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_or_task_section_skips_python_discovery() {
        let directory = tempfile::tempdir().expect("temporary directory should exist");
        let pyproject = directory.path().join("pyproject.toml");

        assert_eq!(has_python_task_configuration(&pyproject), Ok(false));

        fs::write(&pyproject, "[project]\nname = 'example'\n")
            .expect("project file should be written");
        assert_eq!(has_python_task_configuration(&pyproject), Ok(false));

        fs::write(&pyproject, "[tool.genja]\n").expect("project file should be written");
        assert_eq!(has_python_task_configuration(&pyproject), Ok(false));
    }

    #[test]
    fn configured_task_modules_include_an_empty_list() {
        let directory = tempfile::tempdir().expect("temporary directory should exist");
        let pyproject = directory.path().join("pyproject.toml");

        for modules in ["[]", "['my_project.tasks', 'my_project.backups']"] {
            fs::write(
                &pyproject,
                format!("[tool.genja.tasks]\nmodules = {modules}\n"),
            )
            .expect("project file should be written");
            assert_eq!(has_python_task_configuration(&pyproject), Ok(true));
        }
    }

    #[test]
    fn present_but_invalid_task_section_still_selects_python_helper() {
        let directory = tempfile::tempdir().expect("temporary directory should exist");
        let pyproject = directory.path().join("pyproject.toml");

        for contents in [
            "[tool.genja]\ntasks = 'wrong'\n",
            "[tool.genja.tasks]\n",
            "[tool.genja.tasks]\nmodules = 'tasks'\n",
            "[tool.genja.tasks]\nmodules = ['tasks', 3]\n",
        ] {
            fs::write(&pyproject, contents).expect("project file should be written");
            assert_eq!(has_python_task_configuration(&pyproject), Ok(true));
        }
    }

    #[test]
    fn malformed_toml_is_a_discovery_error() {
        let directory = tempfile::tempdir().expect("temporary directory should exist");
        let pyproject = directory.path().join("pyproject.toml");
        fs::write(&pyproject, "[tool.genja.tasks\n").expect("project file should be written");

        let error =
            has_python_task_configuration(&pyproject).expect_err("malformed TOML should fail");
        assert!(error.to_string().contains("invalid TOML"), "{error}");
        assert!(
            error.to_string().contains(&pyproject.display().to_string()),
            "{error}"
        );
    }
}
