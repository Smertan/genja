//! Shared task descriptor discovery boundary for terminal interfaces.
//!
//! This module owns CLI/TUI-facing discovery concepts while reusing the
//! canonical descriptor identity and metadata types from `genja-core`.

pub mod rust;

use std::error::Error;
use std::fmt;

pub use genja_core::task::{TaskDescriptor, TaskRegistrationKey};
use genja_core::task::{TaskRegistrationError, validate_task_version};

/// Result type for task descriptor discovery operations.
pub type DiscoveryResult<T> = Result<T, DiscoveryError>;

/// Re-exports for source implementations and consumers that prefer an explicit
/// source namespace.
pub mod source {
    pub use super::{
        DiscoveryError, DiscoveryResult, TaskDescriptorIdentity, TaskDescriptorSource,
    };
}

/// Source of task descriptors for terminal interfaces.
///
/// Implementations may read descriptors from compiled Rust registries, Python
/// modules, provider manifests, MCP tools, or other future backends. This trait
/// is intentionally limited to descriptor discovery and does not cover task
/// construction or execution.
pub trait TaskDescriptorSource {
    /// Return all known task descriptors.
    fn list_tasks(&self) -> DiscoveryResult<Vec<TaskDescriptor>>;

    /// Describe a task by rendered `<task-id>@<task-version>` identity.
    fn describe_task(&self, identity: &str) -> DiscoveryResult<TaskDescriptor> {
        let identity = parse_task_descriptor_identity(identity)?;
        let descriptors = self.list_tasks()?;
        find_task_descriptor_by_identity(descriptors.iter(), &identity)
    }

    /// Describe a task by a parsed registration key.
    ///
    /// Implementations may override this method when their backend supports a
    /// direct descriptor lookup. The default implementation lists descriptors
    /// and applies the shared exact identity matching rules.
    fn describe_task_by_key(&self, key: &TaskRegistrationKey) -> DiscoveryResult<TaskDescriptor> {
        let descriptors = self.list_tasks()?;
        find_task_descriptor(descriptors.iter(), key)
    }
}

/// Parsed descriptor identity used for discovery lookup.
///
/// Unlike [`TaskRegistrationKey`], this type only validates the rendered
/// identity shape and semantic version. It deliberately allows generated
/// descriptor IDs such as `auto:crate::module::Task`, so CLI users can describe
/// any identity previously emitted by `genja task list`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskDescriptorIdentity {
    id: String,
    version: String,
}

impl TaskDescriptorIdentity {
    /// Return the descriptor ID.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Return the descriptor version.
    pub fn version(&self) -> &str {
        &self.version
    }
}

/// Find a descriptor by exact parsed task registration key.
pub fn find_task_descriptor<'a, I>(
    descriptors: I,
    key: &TaskRegistrationKey,
) -> DiscoveryResult<TaskDescriptor>
where
    I: IntoIterator<Item = &'a TaskDescriptor>,
{
    descriptors
        .into_iter()
        .find(|descriptor| descriptor.id == key.id() && descriptor.version == key.version())
        .cloned()
        .ok_or_else(|| DiscoveryError::NotFound {
            id: key.id().to_string(),
            version: Some(key.version().to_string()),
        })
}

/// Find a descriptor by exact parsed descriptor identity.
pub fn find_task_descriptor_by_identity<'a, I>(
    descriptors: I,
    identity: &TaskDescriptorIdentity,
) -> DiscoveryResult<TaskDescriptor>
where
    I: IntoIterator<Item = &'a TaskDescriptor>,
{
    descriptors
        .into_iter()
        .find(|descriptor| descriptor.id == identity.id && descriptor.version == identity.version)
        .cloned()
        .ok_or_else(|| DiscoveryError::NotFound {
            id: identity.id.clone(),
            version: Some(identity.version.clone()),
        })
}

/// Errors returned by task descriptor discovery sources.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiscoveryError {
    /// A rendered `<task-id>@<task-version>` identity could not be parsed.
    InvalidIdentity {
        /// Invalid identity value.
        identity: String,
        /// Human-readable parsing or validation failure.
        reason: String,
    },
    /// No descriptor matched the requested task identity.
    NotFound {
        /// Requested task ID.
        id: String,
        /// Requested task version, when specified.
        version: Option<String>,
    },
    /// A lookup by ID omitted the version and multiple versions were available.
    AmbiguousVersion {
        /// Requested task ID.
        id: String,
        /// Available versions for the requested task ID.
        versions: Vec<String>,
    },
    /// The descriptor source failed independently of caller input.
    SourceFailed {
        /// Human-readable source failure.
        message: String,
    },
}

impl DiscoveryError {
    /// Create a source failure from a displayable backend error.
    pub fn source_failed(error: impl fmt::Display) -> Self {
        Self::SourceFailed {
            message: error.to_string(),
        }
    }
}

impl fmt::Display for DiscoveryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIdentity { identity, reason } => {
                write!(f, "invalid task identity `{identity}`: {reason}")
            }
            Self::NotFound { id, version } => match version {
                Some(version) => write!(f, "task descriptor `{id}@{version}` was not found"),
                None => write!(f, "task descriptor `{id}` was not found"),
            },
            Self::AmbiguousVersion { id, versions } => {
                write!(
                    f,
                    "task descriptor `{id}` has multiple versions: {}",
                    versions.join(", ")
                )
            }
            Self::SourceFailed { message } => {
                write!(f, "task descriptor source failed: {message}")
            }
        }
    }
}

impl Error for DiscoveryError {}

impl From<TaskRegistrationError> for DiscoveryError {
    fn from(error: TaskRegistrationError) -> Self {
        match error {
            TaskRegistrationError::InvalidIdentity { identity, reason } => {
                Self::InvalidIdentity { identity, reason }
            }
            TaskRegistrationError::InvalidId { id, reason } => Self::InvalidIdentity {
                identity: id,
                reason,
            },
            TaskRegistrationError::InvalidVersion { version, reason } => Self::InvalidIdentity {
                identity: version,
                reason,
            },
            TaskRegistrationError::NotFound { id, version } => Self::NotFound { id, version },
            TaskRegistrationError::AmbiguousVersion { id, versions } => {
                Self::AmbiguousVersion { id, versions }
            }
            // Discovery is descriptor-only. Construction, input, and factory
            // failures should not normally occur here, so preserve them as
            // backend/source failures.
            error => Self::source_failed(error),
        }
    }
}

/// Parse a rendered `<task-id>@<task-version>` task identity.
///
/// This delegates validation to [`TaskRegistrationKey`], so task IDs must
/// follow the explicit registration rules and versions must be semantic
/// versions. When either part is invalid, the returned discovery error keeps
/// the original rendered identity so terminal interfaces can report the exact
/// user-provided value.
pub fn parse_task_identity(identity: &str) -> DiscoveryResult<TaskRegistrationKey> {
    TaskRegistrationKey::parse(identity).map_err(|error| match error {
        TaskRegistrationError::InvalidIdentity { identity, reason } => {
            DiscoveryError::InvalidIdentity { identity, reason }
        }
        TaskRegistrationError::InvalidId { reason, .. }
        | TaskRegistrationError::InvalidVersion { reason, .. } => DiscoveryError::InvalidIdentity {
            identity: identity.to_string(),
            reason,
        },
        error => DiscoveryError::source_failed(error),
    })
}

/// Parse a rendered descriptor identity in `<descriptor-id>@<semver-version>` form.
///
/// This parser is intentionally less strict than [`parse_task_identity`]. It is
/// intended for descriptor lookup and accepts generated IDs emitted by compiled
/// discovery, while still requiring a non-empty ID and semantic version.
pub fn parse_task_descriptor_identity(identity: &str) -> DiscoveryResult<TaskDescriptorIdentity> {
    if identity.is_empty() {
        return Err(DiscoveryError::InvalidIdentity {
            identity: identity.to_string(),
            reason: "identity must not be empty".to_string(),
        });
    }

    if identity.matches('@').count() != 1 {
        return Err(DiscoveryError::InvalidIdentity {
            identity: identity.to_string(),
            reason: "identity must contain exactly one `@` separator".to_string(),
        });
    }

    let (id, version) =
        identity
            .split_once('@')
            .ok_or_else(|| DiscoveryError::InvalidIdentity {
                identity: identity.to_string(),
                reason: "identity must contain exactly one `@` separator".to_string(),
            })?;

    if id.is_empty() {
        return Err(DiscoveryError::InvalidIdentity {
            identity: identity.to_string(),
            reason: "identity must include a task id before `@`".to_string(),
        });
    }

    if id.trim() != id {
        return Err(DiscoveryError::InvalidIdentity {
            identity: identity.to_string(),
            reason: "task id must not have leading or trailing whitespace".to_string(),
        });
    }

    if version.is_empty() {
        return Err(DiscoveryError::InvalidIdentity {
            identity: identity.to_string(),
            reason: "identity must include a task version after `@`".to_string(),
        });
    }

    validate_task_version(version).map_err(|error| match error {
        TaskRegistrationError::InvalidVersion { reason, .. } => DiscoveryError::InvalidIdentity {
            identity: identity.to_string(),
            reason,
        },
        error => DiscoveryError::source_failed(error),
    })?;

    Ok(TaskDescriptorIdentity {
        id: id.to_string(),
        version: version.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use genja_core::task::{TaskDescriptorMetadata, TaskExecutionMode};

    #[derive(Debug)]
    struct StaticTaskDescriptorSource {
        descriptors: Vec<TaskDescriptor>,
    }

    impl TaskDescriptorSource for StaticTaskDescriptorSource {
        fn list_tasks(&self) -> DiscoveryResult<Vec<TaskDescriptor>> {
            Ok(self.descriptors.clone())
        }
    }

    fn descriptor(id: &str, version: &str) -> TaskDescriptor {
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
        )
    }

    fn generated_descriptor(id: &str, version: &str) -> TaskDescriptor {
        TaskDescriptor::generated(
            id,
            version,
            TaskDescriptorMetadata {
                name: id.replace([':', '.'], "_"),
                description: None,
                execution_mode: TaskExecutionMode::Async,
                connection_plugin_name: None,
                processor_names: Vec::new(),
                retry: None,
            },
        )
    }

    #[test]
    fn list_tasks_returns_descriptors_from_source() {
        let descriptors = vec![descriptor("acme.deploy", "1.0.0")];
        let source = StaticTaskDescriptorSource {
            descriptors: descriptors.clone(),
        };

        assert_eq!(source.list_tasks(), Ok(descriptors));
    }

    #[test]
    fn describe_task_uses_default_identity_lookup() {
        let expected = descriptor("acme.deploy", "1.0.0");
        let source = StaticTaskDescriptorSource {
            descriptors: vec![
                descriptor("acme.deploy", "0.9.0"),
                expected.clone(),
                descriptor("acme.rollback", "1.0.0"),
            ],
        };

        assert_eq!(source.describe_task("acme.deploy@1.0.0"), Ok(expected));
    }

    #[test]
    fn describe_task_uses_default_lookup_for_generated_descriptor_ids() {
        let expected = generated_descriptor("auto:use_genja::tasks::DeployChangesTask", "0.1.0");
        let source = StaticTaskDescriptorSource {
            descriptors: vec![expected.clone()],
        };

        assert_eq!(
            source.describe_task("auto:use_genja::tasks::DeployChangesTask@0.1.0"),
            Ok(expected)
        );
    }

    #[test]
    fn describe_task_rejects_invalid_identity_before_listing() {
        let source = StaticTaskDescriptorSource {
            descriptors: vec![descriptor("acme.deploy", "1.0.0")],
        };

        assert!(matches!(
            source.describe_task("acme.deploy"),
            Err(DiscoveryError::InvalidIdentity { identity, reason })
                if identity == "acme.deploy"
                    && reason == "identity must contain exactly one `@` separator"
        ));
    }

    #[test]
    fn find_task_descriptor_returns_not_found_for_missing_key() {
        let descriptors = vec![descriptor("acme.deploy", "1.0.0")];
        let key = TaskRegistrationKey::parse("acme.deploy@2.0.0").expect("key should parse");

        assert_eq!(
            find_task_descriptor(descriptors.iter(), &key),
            Err(DiscoveryError::NotFound {
                id: "acme.deploy".to_string(),
                version: Some("2.0.0".to_string()),
            })
        );
    }

    #[test]
    fn find_task_descriptor_by_identity_accepts_generated_descriptor_ids() {
        let descriptors = vec![generated_descriptor(
            "auto:use_genja::tasks::DeployChangesTask",
            "0.1.0",
        )];
        let identity =
            parse_task_descriptor_identity("auto:use_genja::tasks::DeployChangesTask@0.1.0")
                .expect("generated descriptor identity should parse");

        assert_eq!(
            find_task_descriptor_by_identity(descriptors.iter(), &identity),
            Ok(descriptors[0].clone())
        );
    }

    #[test]
    fn parse_task_identity_preserves_original_identity_for_invalid_id() {
        // TaskRegistrationKey validates the task ID before the version. This
        // should surface an ID validation reason while preserving the complete
        // identity string that the caller supplied.
        assert!(matches!(
            parse_task_identity("Acme.deploy@1.0.0"),
            Err(DiscoveryError::InvalidIdentity { identity, reason })
                if identity == "Acme.deploy@1.0.0"
                    && reason == "id segments must start with an ASCII lowercase letter or digit"
        ));
    }

    #[test]
    fn parse_task_descriptor_identity_accepts_generated_descriptor_ids() {
        let identity =
            parse_task_descriptor_identity("auto:use_genja::tasks::DeployChangesTask@0.1.0")
                .expect("generated descriptor identity should parse");

        assert_eq!(identity.id(), "auto:use_genja::tasks::DeployChangesTask");
        assert_eq!(identity.version(), "0.1.0");
    }

    #[test]
    fn parse_task_descriptor_identity_rejects_invalid_versions() {
        assert!(matches!(
            parse_task_descriptor_identity("auto:use_genja::tasks::DeployChangesTask@latest"),
            Err(DiscoveryError::InvalidIdentity { identity, reason })
                if identity == "auto:use_genja::tasks::DeployChangesTask@latest"
                    && reason.contains("unexpected character")
        ));
    }

    #[test]
    fn parse_task_identity_preserves_original_identity_for_invalid_version() {
        // Once the task ID is valid, invalid semver should map to the same
        // discovery variant with the original rendered identity.
        assert!(matches!(
            parse_task_identity("acme.deploy@latest"),
            Err(DiscoveryError::InvalidIdentity { identity, reason })
                if identity == "acme.deploy@latest"
                    && reason.contains("unexpected character")
        ));
    }

    #[test]
    fn task_registration_lookup_errors_map_to_discovery_errors() {
        assert_eq!(
            DiscoveryError::from(TaskRegistrationError::NotFound {
                id: "acme.deploy".to_string(),
                version: Some("1.0.0".to_string()),
            }),
            DiscoveryError::NotFound {
                id: "acme.deploy".to_string(),
                version: Some("1.0.0".to_string()),
            }
        );

        assert_eq!(
            DiscoveryError::from(TaskRegistrationError::AmbiguousVersion {
                id: "acme.deploy".to_string(),
                versions: vec!["1.0.0".to_string(), "2.0.0".to_string()],
            }),
            DiscoveryError::AmbiguousVersion {
                id: "acme.deploy".to_string(),
                versions: vec!["1.0.0".to_string(), "2.0.0".to_string()],
            }
        );
    }

    #[test]
    fn construction_registration_errors_map_to_source_failures() {
        assert_eq!(
            DiscoveryError::from(TaskRegistrationError::NotConstructible {
                id: "acme.deploy".to_string(),
                version: "1.0.0".to_string(),
            }),
            DiscoveryError::SourceFailed {
                message: "registered task `acme.deploy@1.0.0` is not constructible".to_string(),
            }
        );
    }
}
