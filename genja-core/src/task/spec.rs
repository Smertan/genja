//! Declarative task invocation specs.
//!
//! `TaskSpec` is the minimal data model for constructing one registered task
//! from a stable task identity and JSON-compatible input. It can also carry
//! narrow per-run runtime policy overrides for retry and session verification.
//! It intentionally does not run tasks, define task lists, change processors,
//! or provide a workflow language.

use super::{
    RetryConfig, SessionVerificationConfig, TaskDefinition, TaskRegistrationError,
    TaskRegistrationKey, create_compiled_task_by_identity,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::error::Error;
use std::fmt;
use std::str::FromStr;

/// Text format used when parsing a declarative task spec.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskSpecFormat {
    /// Try JSON first, then YAML.
    Auto,
    /// Parse the source text as JSON.
    Json,
    /// Parse the source text as YAML.
    Yaml,
}

/// Errors returned while validating declarative task specs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskSpecError {
    /// Automatic task spec parsing failed for every supported text format.
    InvalidAuto {
        /// Human-readable JSON parse failure.
        json_message: String,
        /// Human-readable YAML parse failure.
        yaml_message: String,
    },
    /// YAML task spec parsing failed.
    InvalidYaml {
        /// Human-readable parse failure.
        message: String,
    },
    /// JSON task spec parsing failed.
    InvalidJson {
        /// Human-readable parse failure.
        message: String,
    },
    /// The source parsed successfully but does not have the task spec shape.
    InvalidShape {
        /// Human-readable shape failure.
        message: String,
    },
    /// A task spec override is unsupported or invalid.
    ///
    /// Only `retry` and `session_verification` are currently supported under
    /// `overrides`.
    InvalidOverride {
        /// Override field path.
        field: String,
        /// Human-readable validation failure.
        message: String,
    },
    /// The `task` identity failed `<task-id>@<task-version>` validation.
    InvalidTaskIdentity {
        /// Invalid identity value.
        identity: String,
        /// Human-readable validation failure.
        reason: String,
    },
}

impl fmt::Display for TaskSpecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidAuto {
                json_message,
                yaml_message,
            } => {
                write!(
                    f,
                    "could not parse task spec as JSON or YAML; JSON parser reported: {json_message}; YAML parser reported: {yaml_message}"
                )
            }
            Self::InvalidYaml { message } => {
                write!(f, "invalid YAML task spec: {message}")
            }
            Self::InvalidJson { message } => {
                write!(f, "invalid JSON task spec: {message}")
            }
            Self::InvalidShape { message } => {
                write!(f, "invalid task spec: {message}")
            }
            Self::InvalidOverride { field, message } => {
                write!(f, "invalid task spec override `{field}`: {message}")
            }
            Self::InvalidTaskIdentity { identity, reason } => {
                write!(f, "invalid task spec identity `{identity}`: {reason}")
            }
        }
    }
}

impl Error for TaskSpecError {}

/// Errors returned while constructing a compiled task from a declarative spec.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskSpecConstructionError {
    /// The task spec could not be parsed or validated.
    InvalidSpec(TaskSpecError),
    /// The spec was valid, but compiled task lookup or construction failed.
    Registration(TaskRegistrationError),
}

impl fmt::Display for TaskSpecConstructionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSpec(error) => write!(f, "{error}"),
            Self::Registration(error) => write!(f, "{error}"),
        }
    }
}

impl Error for TaskSpecConstructionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidSpec(error) => Some(error),
            Self::Registration(error) => Some(error),
        }
    }
}

impl From<TaskSpecError> for TaskSpecConstructionError {
    fn from(error: TaskSpecError) -> Self {
        Self::InvalidSpec(error)
    }
}

impl From<TaskRegistrationError> for TaskSpecConstructionError {
    fn from(error: TaskRegistrationError) -> Self {
        Self::Registration(error)
    }
}

/// Per-run runtime policy overrides for a declarative task spec.
///
/// Overrides are intentionally narrow. They can tune runtime policy for a
/// single constructed task, but they do not rewrite authored task behavior,
/// change processors, change connection plugins, or alter registration
/// metadata. The supported fields are `retry` and `session_verification`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TaskSpecOverrides {
    /// Optional retry policy override for this task construction.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry: Option<RetryConfig>,
    /// Optional post-change session verification override.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_verification: Option<SessionVerificationConfig>,
}

impl TaskSpecOverrides {
    /// Validate override values.
    pub fn validate(&self) -> Result<(), TaskSpecError> {
        if let Some(config) = self.session_verification
            && config.max_attempts() == 0
        {
            return Err(TaskSpecError::InvalidOverride {
                field: "overrides.session_verification.max_attempts".to_string(),
                message: "must be greater than 0".to_string(),
            });
        }

        Ok(())
    }
}

/// Declarative spec for constructing one registered task.
///
/// `task` must be a rendered registration identity in
/// `<task-id>@<task-version>` form. `input` is passed to the registered task
/// factory as JSON-compatible construction input. When `input` is omitted, it
/// defaults to an empty object. `overrides` can tune narrow runtime policy for
/// the constructed task without changing the authored task behavior.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TaskSpec {
    /// Stable task identity in `<task-id>@<task-version>` form.
    pub task: String,
    /// JSON-compatible task construction input.
    #[serde(default = "empty_input")]
    pub input: Value,
    /// Optional per-run runtime policy overrides.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub overrides: Option<TaskSpecOverrides>,
}

impl TaskSpec {
    /// Create and validate a single-task declarative spec.
    pub fn new(task: impl Into<String>, input: Value) -> Result<Self, TaskSpecError> {
        let spec = Self {
            task: task.into(),
            input,
            overrides: None,
        };
        spec.validate()?;
        Ok(spec)
    }

    /// Parse a single-task spec from text using an explicit format.
    pub fn parse(source: &str, format: TaskSpecFormat) -> Result<Self, TaskSpecError> {
        match format {
            TaskSpecFormat::Auto => Self::parse_auto(source),
            TaskSpecFormat::Json => Self::from_json_str(source),
            TaskSpecFormat::Yaml => Self::from_yaml_str(source),
        }
    }

    /// Parse a single-task spec from text as JSON or YAML.
    ///
    /// JSON is tried first. If it fails to parse, YAML is tried. If neither
    /// format parses, the error reports both parser failures. If either format
    /// parses but the document is not a task spec object, a format-neutral
    /// shape error is returned.
    pub fn parse_auto(source: &str) -> Result<Self, TaskSpecError> {
        match parse_json_value(source) {
            Ok(value) => Self::from_value(value),
            Err(json_message) => match parse_yaml_value(source) {
                Ok(value) => Self::from_value(value),
                Err(yaml_message) => Err(TaskSpecError::InvalidAuto {
                    json_message,
                    yaml_message,
                }),
            },
        }
    }

    /// Parse a single-task spec from YAML text.
    ///
    /// Use [`TaskSpec::parse_auto`] for a parser that accepts the common JSON
    /// and YAML file shapes through one entry point.
    pub fn from_yaml_str(source: &str) -> Result<Self, TaskSpecError> {
        let value =
            parse_yaml_value(source).map_err(|message| TaskSpecError::InvalidYaml { message })?;
        Self::from_value(value)
    }

    /// Parse a single-task spec from JSON text.
    ///
    /// Use [`TaskSpec::parse_auto`] for a parser that accepts the common JSON
    /// and YAML file shapes through one entry point.
    pub fn from_json_str(source: &str) -> Result<Self, TaskSpecError> {
        let value =
            parse_json_value(source).map_err(|message| TaskSpecError::InvalidJson { message })?;
        Self::from_value(value)
    }

    fn from_value(value: Value) -> Result<Self, TaskSpecError> {
        let object = value.as_object().ok_or_else(expected_shape_error)?;
        if !object.contains_key("task") {
            return Err(TaskSpecError::InvalidShape {
                message:
                    "missing required field `task`; expected an object with `task`, optional `input`, and optional `overrides`"
                        .to_string(),
            });
        }
        validate_overrides_shape(object)?;

        let spec: Self =
            serde_json::from_value(value).map_err(|error| TaskSpecError::InvalidShape {
                message: error.to_string(),
            })?;
        spec.validate()?;
        Ok(spec)
    }

    /// Validate this spec's task identity.
    pub fn validate(&self) -> Result<(), TaskSpecError> {
        TaskRegistrationKey::parse(&self.task).map_err(|error| {
            TaskSpecError::InvalidTaskIdentity {
                identity: self.task.clone(),
                reason: error.to_string(),
            }
        })?;
        if let Some(overrides) = &self.overrides {
            overrides.validate()?;
        }
        Ok(())
    }
}

fn parse_json_value(source: &str) -> Result<Value, String> {
    serde_json::from_str(source).map_err(|error| error.to_string())
}

fn parse_yaml_value(source: &str) -> Result<Value, String> {
    serde_yaml::from_str(source).map_err(|error| error.to_string())
}

fn expected_shape_error() -> TaskSpecError {
    TaskSpecError::InvalidShape {
        message: "expected an object with `task`, optional `input`, and optional `overrides`"
            .to_string(),
    }
}

fn validate_overrides_shape(object: &Map<String, Value>) -> Result<(), TaskSpecError> {
    let Some(overrides) = object.get("overrides") else {
        return Ok(());
    };
    let overrides = overrides
        .as_object()
        .ok_or_else(|| TaskSpecError::InvalidOverride {
            field: "overrides".to_string(),
            message: "expected an object".to_string(),
        })?;

    for key in overrides.keys() {
        if key != "retry" && key != "session_verification" {
            return Err(TaskSpecError::InvalidOverride {
                field: format!("overrides.{key}"),
                message: "unsupported override field".to_string(),
            });
        }
    }

    Ok(())
}

impl FromStr for TaskSpec {
    type Err = TaskSpecError;

    /// Parse a task spec with the same deterministic format detection as
    /// [`TaskSpec::parse_auto`].
    fn from_str(source: &str) -> Result<Self, Self::Err> {
        Self::parse_auto(source)
    }
}

/// Construct a compiled task from a validated declarative task spec.
///
/// This helper uses the compiled task registry and passes the spec's
/// JSON-compatible `input` to the registered task factory selected by
/// `spec.task`.
pub fn create_compiled_task_from_spec(
    spec: TaskSpec,
) -> Result<TaskDefinition, TaskSpecConstructionError> {
    let task = create_compiled_task_by_identity(&spec.task, spec.input)?;
    Ok(apply_overrides(task, spec.overrides))
}

/// Parse a task spec string with auto JSON/YAML parsing and construct its task.
pub fn create_compiled_task_from_spec_str(
    source: &str,
) -> Result<TaskDefinition, TaskSpecConstructionError> {
    create_compiled_task_from_spec(TaskSpec::parse_auto(source)?)
}

/// Parse a task spec string with an explicit format and construct its task.
pub fn create_compiled_task_from_spec_str_with_format(
    source: &str,
    format: TaskSpecFormat,
) -> Result<TaskDefinition, TaskSpecConstructionError> {
    create_compiled_task_from_spec(TaskSpec::parse(source, format)?)
}

fn empty_input() -> Value {
    Value::Object(Map::new())
}

fn apply_overrides(task: TaskDefinition, overrides: Option<TaskSpecOverrides>) -> TaskDefinition {
    let Some(overrides) = overrides else {
        return task;
    };
    let task = if let Some(retry) = overrides.retry {
        task.with_retry_config_override(retry)
    } else {
        task
    };
    if let Some(session_verification) = overrides.session_verification {
        task.with_session_verification_config_override(session_verification)
    } else {
        task
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn task_spec_new_accepts_valid_identity_and_input() {
        let spec = TaskSpec::new(
            "acme.examples.backup_config@1.0.0",
            json!({
                "backup_path": "/tmp/configs",
                "compress": true,
            }),
        )
        .expect("task spec should be valid");

        assert_eq!(spec.task, "acme.examples.backup_config@1.0.0");
        assert_eq!(
            spec.input,
            json!({
                "backup_path": "/tmp/configs",
                "compress": true,
            })
        );
    }

    #[test]
    fn task_spec_new_rejects_invalid_identity() {
        let error = TaskSpec::new("acme.examples.backup_config", json!({}))
            .expect_err("identity should be required");

        assert_eq!(
            error,
            TaskSpecError::InvalidTaskIdentity {
                identity: "acme.examples.backup_config".to_string(),
                reason: "invalid task identity `acme.examples.backup_config`: identity must contain exactly one `@` separator".to_string(),
            }
        );
    }

    #[test]
    fn task_spec_deserializes_json_shape() {
        let spec: TaskSpec = serde_json::from_value(json!({
            "task": "acme.examples.backup_config@1.0.0",
            "input": {
                "backup_path": "/tmp/configs",
                "compress": true,
            }
        }))
        .expect("task spec should deserialize");

        assert_eq!(spec.task, "acme.examples.backup_config@1.0.0");
        assert_eq!(
            spec.input,
            json!({
                "backup_path": "/tmp/configs",
                "compress": true,
            })
        );
    }

    #[test]
    fn task_spec_deserializes_missing_input_as_empty_object() {
        let spec: TaskSpec = serde_json::from_value(json!({
            "task": "acme.examples.collect_facts@1.0.0",
        }))
        .expect("task spec should deserialize");

        assert_eq!(spec.input, json!({}));
    }

    #[test]
    fn task_spec_parses_yaml_source() {
        let spec = TaskSpec::from_yaml_str(
            r#"
task: acme.examples.backup_config@1.0.0
input:
  backup_path: /tmp/configs
  compress: true
  rules:
    - path: /etc/network
      recursive: true
"#,
        )
        .expect("YAML task spec should parse");

        assert_eq!(spec.task, "acme.examples.backup_config@1.0.0");
        assert_eq!(
            spec.input,
            json!({
                "backup_path": "/tmp/configs",
                "compress": true,
                "rules": [
                    {
                        "path": "/etc/network",
                        "recursive": true,
                    }
                ],
            })
        );
    }

    #[test]
    fn task_spec_parses_json_source() {
        let spec = TaskSpec::from_json_str(
            r#"
{
  "task": "acme.examples.backup_config@1.0.0",
  "input": {
    "backup_path": "/tmp/configs",
    "compress": true
  }
}
"#,
        )
        .expect("JSON task spec should parse");

        assert_eq!(spec.task, "acme.examples.backup_config@1.0.0");
        assert_eq!(
            spec.input,
            json!({
                "backup_path": "/tmp/configs",
                "compress": true,
            })
        );
    }

    #[test]
    fn task_spec_parse_uses_explicit_format() {
        let auto = TaskSpec::parse(
            "task: acme.examples.collect_facts@1.0.0\n",
            TaskSpecFormat::Auto,
        )
        .expect("auto task spec should parse");
        let yaml = TaskSpec::parse(
            "task: acme.examples.collect_facts@1.0.0\n",
            TaskSpecFormat::Yaml,
        )
        .expect("YAML task spec should parse");
        let json = TaskSpec::parse(
            r#"{ "task": "acme.examples.collect_facts@1.0.0" }"#,
            TaskSpecFormat::Json,
        )
        .expect("JSON task spec should parse");

        assert_eq!(auto.task, "acme.examples.collect_facts@1.0.0");
        assert_eq!(yaml.task, "acme.examples.collect_facts@1.0.0");
        assert_eq!(json.task, "acme.examples.collect_facts@1.0.0");
    }

    #[test]
    fn task_spec_parse_auto_accepts_yaml_and_json() {
        let yaml = TaskSpec::parse_auto("task: acme.examples.collect_facts@1.0.0\n")
            .expect("YAML task spec should parse");
        let json = TaskSpec::parse_auto(
            r#"
{
  "task": "acme.examples.backup_config@1.0.0",
  "input": {
    "backup_path": "/tmp/configs"
  }
}
"#,
        )
        .expect("JSON task spec should parse");

        assert_eq!(yaml.task, "acme.examples.collect_facts@1.0.0");
        assert_eq!(json.task, "acme.examples.backup_config@1.0.0");
        assert_eq!(
            json.input,
            json!({
                "backup_path": "/tmp/configs",
            })
        );
    }

    #[test]
    fn task_spec_parse_auto_rejects_text_that_is_neither_json_nor_yaml() {
        let error = TaskSpec::parse_auto(
            r#"
## Starting

* bullets garbage
* points dnd
"#,
        )
        .expect_err("Markdown-like text should not parse as a task spec");

        assert!(matches!(error, TaskSpecError::InvalidAuto { .. }));
        assert!(
            error
                .to_string()
                .starts_with("could not parse task spec as JSON or YAML;")
        );
    }

    #[test]
    fn task_spec_parse_auto_rejects_parsed_scalar_with_shape_error() {
        let error = TaskSpec::parse_auto("\"I am a string, not a task spec.\"")
            .expect_err("scalar text should not be a task spec");

        assert_eq!(
            error,
            TaskSpecError::InvalidShape {
                message:
                    "expected an object with `task`, optional `input`, and optional `overrides`"
                        .to_string(),
            }
        );
    }

    #[test]
    fn task_spec_parse_auto_rejects_object_without_task_with_shape_error() {
        let error = TaskSpec::parse_auto("input:\n  name: router1\n")
            .expect_err("task field should be required");

        assert_eq!(
            error,
            TaskSpecError::InvalidShape {
                message:
                    "missing required field `task`; expected an object with `task`, optional `input`, and optional `overrides`"
                        .to_string(),
            }
        );
    }

    #[test]
    fn task_spec_parses_retry_and_session_verification_overrides() {
        let spec = TaskSpec::from_yaml_str(
            r#"
task: acme.examples.backup_config@1.0.0
input:
  backup_path: /tmp/configs
overrides:
  retry:
    allow: true
    max_attempts: 4
    delay_ms: 250
  session_verification:
    max_attempts: 2
    delay_ms: 1000
"#,
        )
        .expect("task spec overrides should parse");

        let overrides = spec.overrides.expect("overrides should be present");
        let retry = overrides.retry.expect("retry override should be present");
        assert_eq!(retry.allow(), Some(true));
        assert_eq!(retry.max_attempts(), Some(4));
        assert_eq!(retry.delay_ms(), Some(250));
        let session_verification = overrides
            .session_verification
            .expect("session verification override should be present");
        assert_eq!(session_verification.max_attempts(), 2);
        assert_eq!(session_verification.delay_ms(), 1000);
    }

    #[test]
    fn task_spec_rejects_unsupported_processor_override() {
        let error = TaskSpec::from_yaml_str(
            r#"
task: acme.examples.backup_config@1.0.0
overrides:
  processors: ["audit"]
"#,
        )
        .expect_err("processor override should be unsupported");

        assert_eq!(
            error,
            TaskSpecError::InvalidOverride {
                field: "overrides.processors".to_string(),
                message: "unsupported override field".to_string(),
            }
        );
    }

    #[test]
    fn task_spec_rejects_invalid_session_verification_override() {
        let error = TaskSpec::from_yaml_str(
            r#"
task: acme.examples.backup_config@1.0.0
overrides:
  session_verification:
    max_attempts: 0
    delay_ms: 1000
"#,
        )
        .expect_err("invalid session verification override should be rejected");

        assert_eq!(
            error,
            TaskSpecError::InvalidOverride {
                field: "overrides.session_verification.max_attempts".to_string(),
                message: "must be greater than 0".to_string(),
            }
        );
    }

    #[test]
    fn task_spec_from_str_uses_auto_detection() {
        let spec = "task: acme.examples.collect_facts@1.0.0\n"
            .parse::<TaskSpec>()
            .expect("YAML task spec should parse");

        assert_eq!(spec.task, "acme.examples.collect_facts@1.0.0");
    }

    #[test]
    fn task_spec_parsers_default_missing_input_to_empty_object() {
        let yaml = TaskSpec::from_yaml_str("task: acme.examples.collect_facts@1.0.0\n")
            .expect("YAML task spec should parse");
        let json = TaskSpec::from_json_str(
            r#"
{
  "task": "acme.examples.collect_facts@1.0.0"
}
"#,
        )
        .expect("JSON task spec should parse");

        assert_eq!(yaml.input, json!({}));
        assert_eq!(json.input, json!({}));
        assert_eq!(yaml.overrides, None);
        assert_eq!(json.overrides, None);
    }

    #[test]
    fn task_spec_from_yaml_str_rejects_malformed_yaml() {
        let error = TaskSpec::from_yaml_str("task: [").expect_err("YAML should be invalid");

        assert!(matches!(error, TaskSpecError::InvalidYaml { .. }));
        assert!(error.to_string().starts_with("invalid YAML task spec:"));
    }

    #[test]
    fn task_spec_from_json_str_rejects_malformed_json() {
        let error = TaskSpec::from_json_str(r#"{ "task": "#).expect_err("JSON should be invalid");

        assert!(matches!(error, TaskSpecError::InvalidJson { .. }));
        assert!(error.to_string().starts_with("invalid JSON task spec:"));
    }

    #[test]
    fn explicit_parsers_reject_parsed_scalar_with_shape_error() {
        let yaml_error =
            TaskSpec::from_yaml_str("plain text").expect_err("YAML scalar should not be a spec");
        let json_error = TaskSpec::from_json_str("\"plain text\"")
            .expect_err("JSON scalar should not be a spec");

        assert!(matches!(yaml_error, TaskSpecError::InvalidShape { .. }));
        assert!(matches!(json_error, TaskSpecError::InvalidShape { .. }));
    }

    #[test]
    fn task_spec_parsers_validate_identity_after_parse() {
        let yaml_error = TaskSpec::from_yaml_str("task: acme.examples.backup_config\n")
            .expect_err("identity should be invalid");
        let json_error = TaskSpec::from_json_str(r#"{ "task": "acme.examples.backup_config" }"#)
            .expect_err("identity should be invalid");

        assert!(matches!(
            yaml_error,
            TaskSpecError::InvalidTaskIdentity { .. }
        ));
        assert!(matches!(
            json_error,
            TaskSpecError::InvalidTaskIdentity { .. }
        ));
    }

    #[test]
    fn task_spec_rejects_unknown_fields() {
        let error = serde_json::from_value::<TaskSpec>(json!({
            "task": "acme.examples.backup_config@1.0.0",
            "input": {},
            "metadata": {},
        }))
        .expect_err("unknown fields should be rejected");

        assert!(error.to_string().contains("unknown field `metadata`"));
    }
}
