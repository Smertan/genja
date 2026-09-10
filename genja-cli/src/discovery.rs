//! Shared task descriptor discovery boundary for terminal interfaces.
//!
//! This module owns CLI/TUI-facing discovery concepts while reusing the
//! canonical descriptor identity and metadata types from `genja-core`.

use std::error::Error;
use std::fmt;

use genja_core::task::TaskRegistrationError;
pub use genja_core::task::{TaskDescriptor, TaskRegistrationKey};

/// Result type for task descriptor discovery operations.
pub type DiscoveryResult<T> = Result<T, DiscoveryError>;

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
        let key = parse_task_identity(identity)?;
        self.describe_task_by_key(&key)
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
