//! Task registration and discovery descriptor types.
//!
//! This module defines the serializable task descriptor contract used by
//! local Rust task discovery and future catalog, provider manifest, MCP, and
//! Python registration integrations. The descriptor is metadata only; local
//! construction from JSON input is handled by later factory registry APIs.

use super::{RetryConfig, TaskExecutionMode};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::error::Error;
use std::fmt;

/// Errors returned by task registration, discovery, and construction APIs.
///
/// Factory and input errors identify the affected task but intentionally carry
/// sanitized messages instead of raw JSON input values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskRegistrationError {
    /// A stable explicit task ID failed validation.
    InvalidId {
        /// Invalid ID value.
        id: String,
        /// Human-readable validation failure.
        reason: String,
    },
    /// A task version failed semantic-version validation.
    InvalidVersion {
        /// Invalid version value.
        version: String,
        /// Human-readable validation failure.
        reason: String,
    },
    /// A rendered task identity failed `<task-id>@<task-version>` parsing.
    InvalidIdentity {
        /// Invalid identity value.
        identity: String,
        /// Human-readable parsing failure.
        reason: String,
    },
    /// A registry already contains the same task ID and version.
    DuplicateRegistration {
        /// Duplicate task ID.
        id: String,
        /// Duplicate task version.
        version: String,
    },
    /// No task matched the requested ID and optional version.
    NotFound {
        /// Requested task ID.
        id: String,
        /// Requested version, if the caller specified one.
        version: Option<String>,
    },
    /// A lookup by ID omitted the version and multiple versions are available.
    AmbiguousVersion {
        /// Requested task ID.
        id: String,
        /// Available versions for that ID.
        versions: Vec<String>,
    },
    /// The task is discoverable but cannot be constructed from local JSON input.
    NotConstructible {
        /// Task ID.
        id: String,
        /// Task version.
        version: String,
    },
    /// JSON-compatible task input failed validation or deserialization.
    InvalidInput {
        /// Task ID.
        id: String,
        /// Task version.
        version: String,
        /// Sanitized error message.
        message: String,
    },
    /// A task factory failed after receiving structurally valid input.
    FactoryFailed {
        /// Task ID.
        id: String,
        /// Task version.
        version: String,
        /// Sanitized error message.
        message: String,
    },
}

impl fmt::Display for TaskRegistrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidId { id, reason } => {
                write!(f, "invalid task id `{id}`: {reason}")
            }
            Self::InvalidVersion { version, reason } => {
                write!(f, "invalid task version `{version}`: {reason}")
            }
            Self::InvalidIdentity { identity, reason } => {
                write!(f, "invalid task identity `{identity}`: {reason}")
            }
            Self::DuplicateRegistration { id, version } => {
                write!(f, "duplicate task registration `{id}@{version}`")
            }
            Self::NotFound { id, version } => match version {
                Some(version) => write!(f, "registered task `{id}@{version}` was not found"),
                None => write!(f, "registered task `{id}` was not found"),
            },
            Self::AmbiguousVersion { id, versions } => {
                write!(
                    f,
                    "registered task `{id}` has multiple versions: {}",
                    versions.join(", ")
                )
            }
            Self::NotConstructible { id, version } => {
                write!(f, "registered task `{id}@{version}` is not constructible")
            }
            Self::InvalidInput {
                id,
                version,
                message,
            } => {
                write!(
                    f,
                    "invalid input for registered task `{id}@{version}`: {message}"
                )
            }
            Self::FactoryFailed {
                id,
                version,
                message,
            } => {
                write!(
                    f,
                    "factory failed for registered task `{id}@{version}`: {message}"
                )
            }
        }
    }
}

impl Error for TaskRegistrationError {}

/// Stable task registration identity.
///
/// A `TaskRegistrationKey` renders as `<task-id>@<task-version>`, for example
/// `acme.network.configure_acl@2.0.0`. It validates IDs using the explicit
/// stable task ID rules and versions using semantic-version parsing.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TaskRegistrationKey {
    id: String,
    version: String,
}

impl TaskRegistrationKey {
    /// Create a validated stable task registration key.
    pub fn new(
        id: impl Into<String>,
        version: impl Into<String>,
    ) -> Result<Self, TaskRegistrationError> {
        let id = id.into();
        let version = version.into();
        validate_explicit_task_id(&id)?;
        validate_task_version(&version)?;
        Ok(Self { id, version })
    }

    /// Parse a stable task identity in `<task-id>@<task-version>` form.
    pub fn parse(identity: &str) -> Result<Self, TaskRegistrationError> {
        if identity.is_empty() {
            return Err(TaskRegistrationError::InvalidIdentity {
                identity: identity.to_string(),
                reason: "identity must not be empty".to_string(),
            });
        }

        if identity.matches('@').count() != 1 {
            return Err(TaskRegistrationError::InvalidIdentity {
                identity: identity.to_string(),
                reason: "identity must contain exactly one `@` separator".to_string(),
            });
        }

        let (id, version) =
            identity
                .split_once('@')
                .ok_or_else(|| TaskRegistrationError::InvalidIdentity {
                    identity: identity.to_string(),
                    reason: "identity must contain exactly one `@` separator".to_string(),
                })?;

        if id.is_empty() {
            return Err(TaskRegistrationError::InvalidIdentity {
                identity: identity.to_string(),
                reason: "identity must include a task id before `@`".to_string(),
            });
        }

        if version.is_empty() {
            return Err(TaskRegistrationError::InvalidIdentity {
                identity: identity.to_string(),
                reason: "identity must include a task version after `@`".to_string(),
            });
        }

        Self::new(id, version)
    }

    /// Return the stable task ID.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Return the task contract version.
    pub fn version(&self) -> &str {
        &self.version
    }
}

impl fmt::Display for TaskRegistrationKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}@{}", self.id, self.version)
    }
}

/// Validate an explicitly authored stable task ID.
///
/// Explicit IDs are namespace-friendly identifiers made of one or more `.`
/// separated segments. Segment characters must be ASCII lowercase letters,
/// digits, `_`, or `-`; every segment must start with a letter or digit.
pub fn validate_explicit_task_id(id: &str) -> Result<(), TaskRegistrationError> {
    let invalid = |reason: &str| TaskRegistrationError::InvalidId {
        id: id.to_string(),
        reason: reason.to_string(),
    };

    if id.is_empty() {
        return Err(invalid("id must not be empty"));
    }

    if id.trim() != id {
        return Err(invalid("id must not have leading or trailing whitespace"));
    }

    if id.contains('@') {
        return Err(invalid("id must not contain `@`"));
    }

    for segment in id.split('.') {
        validate_task_id_segment(id, segment)?;
    }

    Ok(())
}

fn validate_task_id_segment(id: &str, segment: &str) -> Result<(), TaskRegistrationError> {
    let invalid = |reason: &str| TaskRegistrationError::InvalidId {
        id: id.to_string(),
        reason: reason.to_string(),
    };

    if segment.is_empty() {
        return Err(invalid("id segments must not be empty"));
    }

    let first = segment
        .bytes()
        .next()
        .expect("segment is known to be non-empty");
    if !first.is_ascii_lowercase() && !first.is_ascii_digit() {
        return Err(invalid(
            "id segments must start with an ASCII lowercase letter or digit",
        ));
    }

    if !segment.bytes().all(|byte| {
        byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_' || byte == b'-'
    }) {
        return Err(invalid(
            "id segments may contain only ASCII lowercase letters, digits, `_`, or `-`",
        ));
    }

    Ok(())
}

/// Validate a task contract version as a semantic version.
pub fn validate_task_version(version: &str) -> Result<(), TaskRegistrationError> {
    if version.is_empty() {
        return Err(TaskRegistrationError::InvalidVersion {
            version: version.to_string(),
            reason: "version must not be empty".to_string(),
        });
    }

    semver::Version::parse(version).map_err(|error| TaskRegistrationError::InvalidVersion {
        version: version.to_string(),
        reason: error.to_string(),
    })?;

    Ok(())
}

/// Indicates how a task descriptor's `id` was chosen.
///
/// Generated IDs are useful for local discovery but are derived from Rust
/// implementation details such as type and module paths, so they are not a
/// stable public contract. Explicit IDs are authored by the task provider and
/// are intended to remain stable across implementation refactors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskIdSource {
    /// The task ID was generated from implementation metadata.
    Generated,
    /// The task ID was explicitly declared by the task author.
    Explicit,
}

/// Serializable metadata describing a discoverable task.
///
/// `TaskDescriptor` is the canonical Rust-side shape for task discovery. It is
/// designed to be language-neutral so future Python registration, persistent
/// catalogs, provider manifests, and MCP tooling can consume equivalent JSON.
///
/// A descriptor's public identity is the pair of [`TaskDescriptor::id`] and
/// [`TaskDescriptor::version`], rendered as `<task-id>@<task-version>` by
/// higher-level catalog APIs. Generated IDs should be treated as local-only;
/// explicit IDs are the stable form intended for provider and remote catalog
/// workflows.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TaskDescriptor {
    /// Stable or generated task identifier.
    pub id: String,
    /// Whether the task ID was generated or explicitly authored.
    pub id_source: TaskIdSource,
    /// Human-readable task name reused from task metadata.
    pub name: String,
    /// Task contract version.
    pub version: String,
    /// Optional human-readable task description.
    pub description: Option<String>,
    /// Whether the task uses blocking or async execution.
    pub execution_mode: TaskExecutionMode,
    /// Connection plugin required by the task, if any.
    pub connection_plugin_name: Option<String>,
    /// Processor plugins selected by the task.
    pub processor_names: Vec<String>,
    /// Task-specific retry metadata, if configured.
    pub retry: Option<RetryConfig>,
    /// Optional JSON Schema describing accepted construction input.
    pub input_schema: Option<Value>,
    /// Whether this process can construct the task from JSON-compatible input.
    pub constructible: bool,
}

impl TaskDescriptor {
    /// Build a descriptor for a task discovered from implementation metadata.
    ///
    /// Generated descriptors are useful for local listing and inspection, but
    /// their IDs are derived from implementation details and should not be
    /// treated as stable public contracts.
    pub fn generated(
        id: impl Into<String>,
        version: impl Into<String>,
        metadata: TaskDescriptorMetadata,
    ) -> Self {
        Self::from_parts(id, TaskIdSource::Generated, version, metadata, None, false)
    }

    /// Build a descriptor for a task with an explicitly authored stable ID.
    ///
    /// Explicit descriptors represent the stable registration path used by
    /// future provider manifests, remote catalogs, MCP tooling, and JSON input
    /// construction.
    pub fn explicit(
        id: impl Into<String>,
        version: impl Into<String>,
        metadata: TaskDescriptorMetadata,
        input_schema: Option<Value>,
        constructible: bool,
    ) -> Self {
        Self::from_parts(
            id,
            TaskIdSource::Explicit,
            version,
            metadata,
            input_schema,
            constructible,
        )
    }

    fn from_parts(
        id: impl Into<String>,
        id_source: TaskIdSource,
        version: impl Into<String>,
        metadata: TaskDescriptorMetadata,
        input_schema: Option<Value>,
        constructible: bool,
    ) -> Self {
        Self {
            id: id.into(),
            id_source,
            name: metadata.name,
            version: version.into(),
            description: metadata.description,
            execution_mode: metadata.execution_mode,
            connection_plugin_name: metadata.connection_plugin_name,
            processor_names: metadata.processor_names,
            retry: metadata.retry,
            input_schema,
            constructible,
        }
    }
}

/// Task metadata shared by task runtime metadata and discovery descriptors.
///
/// `TaskDescriptorMetadata` keeps descriptor construction aligned with macro
/// generated [`super::TaskInfo`] metadata. Future macro metadata fields that are
/// part of task discovery should flow through this struct before they are mapped
/// into generated or explicit [`TaskDescriptor`] values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskDescriptorMetadata {
    /// Human-readable task name reused from task metadata.
    pub name: String,
    /// Optional human-readable task description.
    pub description: Option<String>,
    /// Whether the task uses blocking or async execution.
    pub execution_mode: TaskExecutionMode,
    /// Connection plugin required by the task, if any.
    pub connection_plugin_name: Option<String>,
    /// Processor plugins selected by the task.
    pub processor_names: Vec<String>,
    /// Task-specific retry metadata, if configured.
    pub retry: Option<RetryConfig>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn descriptor_metadata() -> TaskDescriptorMetadata {
        TaskDescriptorMetadata {
            name: "configure_acl".to_string(),
            description: Some("Configures an ACL on a network device".to_string()),
            execution_mode: TaskExecutionMode::Async,
            connection_plugin_name: Some("ssh".to_string()),
            processor_names: vec!["audit".to_string()],
            retry: Some(RetryConfig::builder().allow(true).max_attempts(3).build()),
        }
    }

    #[test]
    fn descriptor_serializes_to_canonical_field_names() {
        let descriptor = TaskDescriptor::explicit(
            "acme.network.configure_acl",
            "2.0.0",
            descriptor_metadata(),
            Some(json!({
                "type": "object",
                "properties": {
                    "acl_name": { "type": "string" }
                }
            })),
            true,
        );

        let serialized = serde_json::to_value(&descriptor).expect("descriptor serializes");

        assert_eq!(
            serialized,
            json!({
                "id": "acme.network.configure_acl",
                "id_source": "explicit",
                "name": "configure_acl",
                "version": "2.0.0",
                "description": "Configures an ACL on a network device",
                "execution_mode": "async",
                "connection_plugin_name": "ssh",
                "processor_names": ["audit"],
                "retry": {
                    "allow": true,
                    "max_attempts": 3,
                    "delay_ms": null
                },
                "input_schema": {
                    "type": "object",
                    "properties": {
                        "acl_name": { "type": "string" }
                    }
                },
                "constructible": true
            })
        );
    }

    #[test]
    fn descriptor_serializes_empty_optional_metadata_as_nulls_and_lists() {
        let descriptor = TaskDescriptor::generated(
            "auto:acme_network_tasks::network::ConfigureAcl",
            "2.0.0",
            TaskDescriptorMetadata {
                name: "configure_acl".to_string(),
                description: None,
                execution_mode: TaskExecutionMode::Blocking,
                connection_plugin_name: None,
                processor_names: Vec::new(),
                retry: None,
            },
        );

        let serialized = serde_json::to_value(&descriptor).expect("descriptor serializes");

        assert_eq!(
            serialized,
            json!({
                "id": "auto:acme_network_tasks::network::ConfigureAcl",
                "id_source": "generated",
                "name": "configure_acl",
                "version": "2.0.0",
                "description": null,
                "execution_mode": "blocking",
                "connection_plugin_name": null,
                "processor_names": [],
                "retry": null,
                "input_schema": null,
                "constructible": false
            })
        );
    }

    #[test]
    fn descriptor_constructors_map_shared_metadata() {
        let metadata = descriptor_metadata();

        let descriptor = TaskDescriptor::explicit(
            "acme.network.configure_acl",
            "2.0.0",
            metadata.clone(),
            None,
            true,
        );

        assert_eq!(descriptor.id, "acme.network.configure_acl");
        assert_eq!(descriptor.id_source, TaskIdSource::Explicit);
        assert_eq!(descriptor.version, "2.0.0");
        assert_eq!(descriptor.name, metadata.name);
        assert_eq!(descriptor.description, metadata.description);
        assert_eq!(descriptor.execution_mode, metadata.execution_mode);
        assert_eq!(
            descriptor.connection_plugin_name,
            metadata.connection_plugin_name
        );
        assert_eq!(descriptor.processor_names, metadata.processor_names);
        assert_eq!(descriptor.retry, metadata.retry);
        assert_eq!(descriptor.input_schema, None);
        assert!(descriptor.constructible);
    }

    #[test]
    fn generated_descriptor_is_not_constructible() {
        let descriptor = TaskDescriptor::generated(
            "auto:acme_network_tasks::network::ConfigureAcl",
            "2.0.0",
            descriptor_metadata(),
        );

        assert_eq!(descriptor.id_source, TaskIdSource::Generated);
        assert_eq!(descriptor.input_schema, None);
        assert!(!descriptor.constructible);
    }

    #[test]
    fn explicit_descriptor_keeps_schema_and_constructible_flag() {
        let input_schema = json!({
            "type": "object",
            "properties": {
                "acl_name": { "type": "string" }
            }
        });
        let descriptor = TaskDescriptor::explicit(
            "acme.network.configure_acl",
            "2.0.0",
            descriptor_metadata(),
            Some(input_schema.clone()),
            true,
        );

        assert_eq!(descriptor.id_source, TaskIdSource::Explicit);
        assert_eq!(descriptor.input_schema, Some(input_schema));
        assert!(descriptor.constructible);
    }

    #[test]
    fn descriptor_metadata_can_be_destructured_by_macro_generated_code() {
        let metadata = TaskDescriptorMetadata {
            name: "configure_acl".to_string(),
            description: None,
            execution_mode: TaskExecutionMode::Async,
            connection_plugin_name: Some("ssh".to_string()),
            processor_names: vec!["audit".to_string()],
            retry: None,
        };

        assert_eq!(
            (
                metadata.name.as_str(),
                metadata.description.as_deref(),
                metadata.execution_mode,
                metadata.connection_plugin_name.as_deref(),
                metadata.processor_names.as_slice(),
                metadata.retry,
            ),
            (
                "configure_acl",
                None,
                TaskExecutionMode::Async,
                Some("ssh"),
                &["audit".to_string()][..],
                None,
            )
        );
    }

    #[test]
    fn explicit_task_id_validation_accepts_namespace_friendly_ids() {
        for id in [
            "configure_acl",
            "acme.network.configure_acl",
            "io.github.acme.configure-acl",
            "a1.b2.c3",
            "1stage.configure_acl",
        ] {
            validate_explicit_task_id(id).expect("id should be valid");
        }
    }

    #[test]
    fn explicit_task_id_validation_rejects_unstable_or_malformed_ids() {
        for (id, expected_reason) in [
            ("", "id must not be empty"),
            (
                " acme.network.configure_acl",
                "id must not have leading or trailing whitespace",
            ),
            (
                "acme.network.configure_acl ",
                "id must not have leading or trailing whitespace",
            ),
            ("acme.network@configure_acl", "id must not contain `@`"),
            ("acme..network", "id segments must not be empty"),
            ("acme.network.", "id segments must not be empty"),
            (
                "Acme.network",
                "id segments must start with an ASCII lowercase letter or digit",
            ),
            (
                "_acme.network",
                "id segments must start with an ASCII lowercase letter or digit",
            ),
            (
                "acme.network/configure_acl",
                "id segments may contain only ASCII lowercase letters, digits, `_`, or `-`",
            ),
        ] {
            let error = validate_explicit_task_id(id).expect_err("id should be invalid");
            assert_eq!(
                error,
                TaskRegistrationError::InvalidId {
                    id: id.to_string(),
                    reason: expected_reason.to_string(),
                }
            );
        }
    }

    #[test]
    fn task_version_validation_accepts_semver_versions() {
        for version in ["1.0.0", "2.1.3", "1.0.0-alpha.1", "1.0.0+build.1"] {
            validate_task_version(version).expect("version should be valid");
        }
    }

    #[test]
    fn task_version_validation_rejects_non_semver_versions() {
        for version in ["", "1", "latest", "v1.0.0"] {
            let error = validate_task_version(version).expect_err("version should be invalid");
            assert!(matches!(
                error,
                TaskRegistrationError::InvalidVersion { .. }
            ));
            assert!(
                error.to_string().contains(version),
                "error should identify the invalid version"
            );
        }
    }

    #[test]
    fn registration_key_new_validates_and_exposes_parts() {
        let key = TaskRegistrationKey::new("acme.network.configure_acl", "2.0.0")
            .expect("key should be valid");

        assert_eq!(key.id(), "acme.network.configure_acl");
        assert_eq!(key.version(), "2.0.0");
        assert_eq!(key.to_string(), "acme.network.configure_acl@2.0.0");
    }

    #[test]
    fn registration_key_parse_round_trips_display() {
        let parsed = TaskRegistrationKey::parse("acme.network.configure_acl@2.0.0")
            .expect("identity should parse");

        assert_eq!(parsed.id(), "acme.network.configure_acl");
        assert_eq!(parsed.version(), "2.0.0");
        assert_eq!(
            TaskRegistrationKey::parse(&parsed.to_string()).expect("display should parse"),
            parsed
        );
    }

    #[test]
    fn registration_key_parse_rejects_invalid_separator_shapes() {
        for (identity, expected_reason) in [
            ("", "identity must not be empty"),
            (
                "acme.network.configure_acl",
                "identity must contain exactly one `@` separator",
            ),
            (
                "acme.network.configure_acl@2.0.0@extra",
                "identity must contain exactly one `@` separator",
            ),
            ("@2.0.0", "identity must include a task id before `@`"),
            (
                "acme.network.configure_acl@",
                "identity must include a task version after `@`",
            ),
        ] {
            let error = TaskRegistrationKey::parse(identity).expect_err("identity should fail");
            assert_eq!(
                error,
                TaskRegistrationError::InvalidIdentity {
                    identity: identity.to_string(),
                    reason: expected_reason.to_string(),
                }
            );
        }
    }

    #[test]
    fn registration_key_parse_reuses_id_and_version_validation() {
        assert!(matches!(
            TaskRegistrationKey::parse("Acme.network.configure_acl@2.0.0"),
            Err(TaskRegistrationError::InvalidId { .. })
        ));

        assert!(matches!(
            TaskRegistrationKey::parse("acme.network.configure_acl@latest"),
            Err(TaskRegistrationError::InvalidVersion { .. })
        ));
    }

    #[test]
    fn registration_error_display_identifies_task_without_raw_input() {
        let error = TaskRegistrationError::InvalidInput {
            id: "acme.network.configure_acl".to_string(),
            version: "2.0.0".to_string(),
            message: "missing field `acl_name`".to_string(),
        };

        assert_eq!(
            error.to_string(),
            "invalid input for registered task `acme.network.configure_acl@2.0.0`: missing field `acl_name`"
        );
    }

    #[test]
    fn registration_error_display_handles_lookup_and_registry_errors() {
        assert_eq!(
            TaskRegistrationError::DuplicateRegistration {
                id: "acme.network.configure_acl".to_string(),
                version: "2.0.0".to_string(),
            }
            .to_string(),
            "duplicate task registration `acme.network.configure_acl@2.0.0`"
        );
        assert_eq!(
            TaskRegistrationError::NotFound {
                id: "acme.network.configure_acl".to_string(),
                version: None,
            }
            .to_string(),
            "registered task `acme.network.configure_acl` was not found"
        );
        assert_eq!(
            TaskRegistrationError::AmbiguousVersion {
                id: "acme.network.configure_acl".to_string(),
                versions: vec!["1.0.0".to_string(), "2.0.0".to_string()],
            }
            .to_string(),
            "registered task `acme.network.configure_acl` has multiple versions: 1.0.0, 2.0.0"
        );
    }
}
