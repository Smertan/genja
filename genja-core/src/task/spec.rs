//! Declarative task invocation specs.
//!
//! `TaskSpec` is the minimal data model for constructing one registered task
//! from a stable task identity and JSON-compatible input. It intentionally does
//! not run tasks, define task lists, apply execution overrides, or provide a
//! workflow language.

use super::TaskRegistrationKey;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::error::Error;
use std::fmt;

/// Errors returned while validating declarative task specs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskSpecError {
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
            Self::InvalidTaskIdentity { identity, reason } => {
                write!(f, "invalid task spec identity `{identity}`: {reason}")
            }
        }
    }
}

impl Error for TaskSpecError {}

/// Declarative spec for constructing one registered task.
///
/// `task` must be a rendered registration identity in
/// `<task-id>@<task-version>` form. `input` is passed to the registered task
/// factory as JSON-compatible construction input. When `input` is omitted, it
/// defaults to an empty object.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TaskSpec {
    /// Stable task identity in `<task-id>@<task-version>` form.
    pub task: String,
    /// JSON-compatible task construction input.
    #[serde(default = "empty_input")]
    pub input: Value,
}

impl TaskSpec {
    /// Create and validate a single-task declarative spec.
    pub fn new(task: impl Into<String>, input: Value) -> Result<Self, TaskSpecError> {
        let spec = Self {
            task: task.into(),
            input,
        };
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
        Ok(())
    }
}

fn empty_input() -> Value {
    Value::Object(Map::new())
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
    fn task_spec_rejects_unknown_fields() {
        let error = serde_json::from_value::<TaskSpec>(json!({
            "task": "acme.examples.backup_config@1.0.0",
            "input": {},
            "overrides": {},
        }))
        .expect_err("unknown fields should be rejected");

        assert!(error.to_string().contains("unknown field `overrides`"));
    }
}
