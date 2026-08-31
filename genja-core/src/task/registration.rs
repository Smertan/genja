//! Task registration and discovery descriptor types.
//!
//! This module defines the serializable task descriptor contract used by
//! local Rust task discovery and future catalog, provider manifest, MCP, and
//! Python registration integrations. Descriptors can be stored by themselves
//! for discovery-only catalogs or paired with factories that construct local
//! task instances from JSON-compatible input.
//!
//! # Registration model
//!
//! Task discovery is descriptor-first. A [`TaskDescriptor`] contains the
//! language-neutral metadata needed to list and inspect a task without running
//! it. A [`RegisteredTaskFactory`] is optional local code that can construct a
//! [`TaskDefinition`] from JSON-compatible input. [`TaskCatalog`] covers the
//! descriptor-only side; [`TaskFactoryRegistry`] covers local construction.
//!
//! Rust tasks normally enter the compiled registry through the
//! `#[genja_task]` macro. A task without `registration(...)` is still
//! discoverable with a generated local ID, but it is not constructible from
//! JSON input. A task with `registration(id = "...")` gets an explicit stable
//! ID and a generated factory.
//!
//! # Task identity
//!
//! A registered task's public identity is:
//!
//! ```text
//! <task-id>@<task-version>
//! ```
//!
//! For example:
//!
//! ```text
//! acme.network.configure_acl@2.0.0
//! ```
//!
//! Explicit task IDs are namespace-friendly strings made of `.` separated
//! segments. Versions must be semantic versions. When a macro registration
//! omits `version`, it uses the provider crate's `CARGO_PKG_VERSION`.
//! Duplicate `id + version` registrations are rejected while the registry is
//! built.
//!
//! # Listing compiled tasks
//!
//! ```no_run
//! use genja_core::task::list_compiled_tasks;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! for descriptor in list_compiled_tasks()? {
//!     println!(
//!         "{}@{} {}",
//!         descriptor.id,
//!         descriptor.version,
//!         descriptor.name
//!     );
//! }
//! # Ok(())
//! # }
//! ```
//!
//! Generated IDs start with `auto:` and are intended for local discovery only.
//! Provider manifests, CLIs, MCP servers, and remote catalogs should use
//! explicit IDs from `registration(id = "...")`.
//!
//! # Looking up and constructing a compiled task
//!
//! Use descriptor lookup when you only need metadata:
//!
//! ```no_run
//! use genja_core::task::get_compiled_task_descriptor_by_identity;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let descriptor =
//!     get_compiled_task_descriptor_by_identity("acme.network.configure_acl@2.0.0")?;
//!
//! if let Some(schema) = &descriptor.input_schema {
//!     println!("{}", serde_json::to_string_pretty(schema)?);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! Use the factory registry path when a caller supplied JSON input and you need
//! a runnable [`TaskDefinition`]:
//!
//! ```no_run
//! use genja_core::task::{TaskInfo, create_compiled_task_by_identity};
//! use serde_json::json;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let task = create_compiled_task_by_identity(
//!     "acme.network.configure_acl@2.0.0",
//!     json!({
//!         "acl_name": "edge-inbound",
//!         "rules": []
//!     }),
//! )?;
//!
//! assert_eq!(task.name(), "configure_acl");
//! # Ok(())
//! # }
//! ```
//!
//! Factory and validation errors identify the affected task identity but should
//! not expose raw input values or decoded secret material.
//!
//! # Descriptor JSON
//!
//! Descriptors serialize with stable field names suitable for future provider
//! manifests and cross-language compatibility tests:
//!
//! ```json
//! {
//!   "id": "acme.network.configure_acl",
//!   "id_source": "explicit",
//!   "name": "configure_acl",
//!   "version": "2.0.0",
//!   "description": "Configures an ACL on a network device",
//!   "execution_mode": "async",
//!   "connection_plugin_name": "ssh",
//!   "processor_names": [],
//!   "retry": null,
//!   "input_schema": null,
//!   "constructible": true
//! }
//! ```

use super::{RetryConfig, TaskDefinition, TaskExecutionMode};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::sync::Arc;

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

/// A task registration entry collected from compiled Rust code.
///
/// `#[genja_task]` uses this type to submit descriptor builders into the
/// process-wide compiled task inventory. It stores function pointers rather
/// than fully built descriptors so macro-generated descriptors can use owned
/// `String`, `Vec`, and JSON values without requiring const allocation.
pub struct CompiledTaskRegistration {
    descriptor: fn() -> TaskDescriptor,
    create: Option<fn(Value) -> Result<TaskDefinition, TaskRegistrationError>>,
}

impl CompiledTaskRegistration {
    /// Create a descriptor-only compiled task registration.
    ///
    /// Descriptor-only registrations are discoverable through [`TaskCatalog`]
    /// but are not constructible through [`TaskFactoryRegistry`].
    pub const fn descriptor_only(descriptor: fn() -> TaskDescriptor) -> Self {
        Self {
            descriptor,
            create: None,
        }
    }

    /// Create a constructible compiled task registration.
    ///
    /// Constructible registrations are discoverable through [`TaskCatalog`] and
    /// can be created from JSON-compatible input through
    /// [`TaskFactoryRegistry::create`].
    pub const fn constructible(
        descriptor: fn() -> TaskDescriptor,
        create: fn(Value) -> Result<TaskDefinition, TaskRegistrationError>,
    ) -> Self {
        Self {
            descriptor,
            create: Some(create),
        }
    }

    /// Build this registration's task descriptor.
    pub fn descriptor(&self) -> TaskDescriptor {
        (self.descriptor)()
    }
}

inventory_crate::collect!(CompiledTaskRegistration);

struct CompiledTaskFactory {
    descriptor: TaskDescriptor,
    create: fn(Value) -> Result<TaskDefinition, TaskRegistrationError>,
}

impl RegisteredTaskFactory for CompiledTaskFactory {
    fn descriptor(&self) -> &TaskDescriptor {
        &self.descriptor
    }

    fn create(&self, input: Value) -> Result<TaskDefinition, TaskRegistrationError> {
        (self.create)(input)
    }
}

/// Build an in-memory registry from all task registrations linked into this process.
///
/// The resulting registry validates descriptor versions, validates explicit
/// IDs, rejects duplicate `id + version` pairs, and returns deterministic
/// listing/lookup behavior. Inventory iteration order is intentionally ignored.
pub fn compiled_task_registry() -> Result<InMemoryTaskRegistry, TaskRegistrationError> {
    let mut registry = InMemoryTaskRegistry::new();
    for registration in inventory_crate::iter::<CompiledTaskRegistration> {
        let descriptor = registration.descriptor();
        if let Some(create) = registration.create {
            registry.register_factory(CompiledTaskFactory { descriptor, create })?;
        } else {
            registry.register_descriptor(descriptor)?;
        }
    }
    Ok(registry)
}

/// List descriptors for all task registrations linked into this process.
pub fn list_compiled_tasks() -> Result<Vec<TaskDescriptor>, TaskRegistrationError> {
    compiled_task_registry()?.list()
}

/// Look up a compiled task descriptor by ID and optional version.
///
/// When `version` is omitted, lookup succeeds only if exactly one version is
/// available for `id`.
pub fn get_compiled_task_descriptor(
    id: &str,
    version: Option<&str>,
) -> Result<TaskDescriptor, TaskRegistrationError> {
    compiled_task_registry()?.get(id, version)
}

/// Look up a compiled task descriptor by rendered `<task-id>@<task-version>` identity.
///
/// This is the most convenient lookup form for CLIs and APIs that accept a
/// single task identity string. The identity is parsed with
/// [`TaskRegistrationKey::parse`] before querying the compiled catalog.
pub fn get_compiled_task_descriptor_by_identity(
    identity: &str,
) -> Result<TaskDescriptor, TaskRegistrationError> {
    compiled_task_registry()?.get_by_identity(identity)
}

/// Construct a compiled task by rendered `<task-id>@<task-version>` identity.
///
/// This helper validates the identity, finds the compiled factory, and passes
/// `input` to the task's selected construction strategy. Descriptor-only tasks
/// and generated local discovery entries return
/// [`TaskRegistrationError::NotConstructible`].
pub fn create_compiled_task_by_identity(
    identity: &str,
    input: Value,
) -> Result<TaskDefinition, TaskRegistrationError> {
    compiled_task_registry()?.create_by_identity(identity, input)
}

/// Read-only catalog of task descriptors.
///
/// Catalog implementations provide metadata listing and lookup. They do not
/// need access to local Rust factories, which lets future SQLite, provider
/// manifest, remote catalog, and MCP metadata sources implement this trait
/// without supporting in-process task construction.
pub trait TaskCatalog {
    /// Return all known task descriptors in deterministic order.
    ///
    /// Implementations should order descriptors by task ID and version so CLI,
    /// manifest, and test output is stable.
    fn list(&self) -> Result<Vec<TaskDescriptor>, TaskRegistrationError>;

    /// Look up a descriptor by ID and optional version.
    ///
    /// When `version` is `None`, the lookup succeeds only if exactly one
    /// version exists for `id`; multiple matches return
    /// [`TaskRegistrationError::AmbiguousVersion`].
    fn get(&self, id: &str, version: Option<&str>)
    -> Result<TaskDescriptor, TaskRegistrationError>;

    /// Look up a descriptor by validated task registration key.
    ///
    /// Use this when the caller has already parsed or validated
    /// `<task-id>@<task-version>`.
    fn get_by_key(
        &self,
        key: &TaskRegistrationKey,
    ) -> Result<TaskDescriptor, TaskRegistrationError> {
        self.get(key.id(), Some(key.version()))
    }

    /// Parse `<task-id>@<task-version>` and look up the matching descriptor.
    ///
    /// Use this form for user-facing inputs such as CLI arguments and MCP tool
    /// parameters.
    fn get_by_identity(&self, identity: &str) -> Result<TaskDescriptor, TaskRegistrationError> {
        let key = TaskRegistrationKey::parse(identity)?;
        self.get_by_key(&key)
    }
}

/// Local factory registry for constructing tasks from JSON-compatible input.
///
/// This trait is intentionally separate from [`TaskCatalog`] because persistent
/// catalogs can store descriptors but cannot store Rust function pointers or
/// closures. Local registries, including the compiled registry, can implement
/// both traits.
pub trait TaskFactoryRegistry {
    /// Construct a local task definition by ID, optional version, and input.
    ///
    /// When `version` is `None`, construction succeeds only if exactly one
    /// version exists for `id`. If the descriptor is present but not backed by
    /// a factory, the registry returns [`TaskRegistrationError::NotConstructible`].
    fn create(
        &self,
        id: &str,
        version: Option<&str>,
        input: Value,
    ) -> Result<TaskDefinition, TaskRegistrationError>;

    /// Construct a local task definition by validated task registration key.
    ///
    /// Use this when the caller already has a [`TaskRegistrationKey`].
    fn create_by_key(
        &self,
        key: &TaskRegistrationKey,
        input: Value,
    ) -> Result<TaskDefinition, TaskRegistrationError> {
        self.create(key.id(), Some(key.version()), input)
    }

    /// Parse `<task-id>@<task-version>` and construct the matching task.
    ///
    /// This is the most direct form for CLI, MCP, and API entrypoints that
    /// receive a task identity string plus JSON input.
    fn create_by_identity(
        &self,
        identity: &str,
        input: Value,
    ) -> Result<TaskDefinition, TaskRegistrationError> {
        let key = TaskRegistrationKey::parse(identity)?;
        self.create_by_key(&key, input)
    }
}

/// Type-erased factory for one registered task.
///
/// Macro-generated registrations will implement this trait for each
/// constructible task. The registry only calls this common interface; the
/// factory implementation owns the task-specific construction strategy. Manual
/// implementations should validate input without including raw secrets in
/// returned error messages.
pub trait RegisteredTaskFactory: Send + Sync {
    /// Return the descriptor for the task this factory constructs.
    fn descriptor(&self) -> &TaskDescriptor;

    /// Construct a task definition from JSON-compatible input.
    ///
    /// Implementations should return [`TaskRegistrationError::InvalidInput`]
    /// for caller-supplied input that does not match the task contract and
    /// [`TaskRegistrationError::FactoryFailed`] for construction failures after
    /// input validation.
    fn create(&self, input: Value) -> Result<TaskDefinition, TaskRegistrationError>;
}

#[derive(Clone)]
enum InMemoryTaskEntry {
    Descriptor(Box<TaskDescriptor>),
    Factory(Arc<dyn RegisteredTaskFactory>),
}

impl InMemoryTaskEntry {
    fn descriptor(&self) -> &TaskDescriptor {
        match self {
            Self::Descriptor(descriptor) => descriptor,
            Self::Factory(factory) => factory.descriptor(),
        }
    }
}

/// In-memory task catalog and local factory registry.
///
/// `InMemoryTaskRegistry` is useful for tests, manually assembled local
/// registries, and as the behavior model for the future compiled registry. It
/// stores descriptor-only entries as well as factory-backed constructible
/// entries, rejects duplicate `id + version` pairs, and performs deterministic
/// version lookup.
#[derive(Clone, Default)]
pub struct InMemoryTaskRegistry {
    entries: BTreeMap<(String, String), InMemoryTaskEntry>,
}

impl InMemoryTaskRegistry {
    /// Create an empty in-memory task registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Build a registry from descriptor-only entries.
    pub fn from_descriptors<I>(descriptors: I) -> Result<Self, TaskRegistrationError>
    where
        I: IntoIterator<Item = TaskDescriptor>,
    {
        let mut registry = Self::new();
        for descriptor in descriptors {
            registry.register_descriptor(descriptor)?;
        }
        Ok(registry)
    }

    /// Add a descriptor-only task entry.
    ///
    /// Descriptor-only entries can be listed and looked up but cannot be
    /// constructed through [`TaskFactoryRegistry::create`].
    pub fn register_descriptor(
        &mut self,
        descriptor: TaskDescriptor,
    ) -> Result<(), TaskRegistrationError> {
        validate_descriptor(&descriptor)?;
        self.insert_entry(InMemoryTaskEntry::Descriptor(Box::new(descriptor)))
    }

    /// Add a constructible factory-backed task entry.
    pub fn register_factory<F>(&mut self, factory: F) -> Result<(), TaskRegistrationError>
    where
        F: RegisteredTaskFactory + 'static,
    {
        self.register_factory_arc(Arc::new(factory))
    }

    /// Add a shared constructible factory-backed task entry.
    pub fn register_factory_arc(
        &mut self,
        factory: Arc<dyn RegisteredTaskFactory>,
    ) -> Result<(), TaskRegistrationError> {
        validate_descriptor(factory.descriptor())?;
        self.insert_entry(InMemoryTaskEntry::Factory(factory))
    }

    fn insert_entry(&mut self, entry: InMemoryTaskEntry) -> Result<(), TaskRegistrationError> {
        let descriptor = entry.descriptor();
        let key = (descriptor.id.clone(), descriptor.version.clone());

        if self.entries.contains_key(&key) {
            return Err(TaskRegistrationError::DuplicateRegistration {
                id: key.0,
                version: key.1,
            });
        }

        self.entries.insert(key, entry);
        Ok(())
    }

    fn resolve_entry(
        &self,
        id: &str,
        version: Option<&str>,
    ) -> Result<&InMemoryTaskEntry, TaskRegistrationError> {
        if let Some(version) = version {
            return self
                .entries
                .get(&(id.to_string(), version.to_string()))
                .ok_or_else(|| TaskRegistrationError::NotFound {
                    id: id.to_string(),
                    version: Some(version.to_string()),
                });
        }

        let mut matches = self
            .entries
            .iter()
            .filter(|((entry_id, _), _)| entry_id == id)
            .map(|((_, entry_version), entry)| (entry_version.clone(), entry));

        let Some((first_version, first_entry)) = matches.next() else {
            return Err(TaskRegistrationError::NotFound {
                id: id.to_string(),
                version: None,
            });
        };

        let mut versions = vec![first_version];
        for (entry_version, _) in matches {
            versions.push(entry_version);
        }

        if versions.len() > 1 {
            return Err(TaskRegistrationError::AmbiguousVersion {
                id: id.to_string(),
                versions,
            });
        }

        Ok(first_entry)
    }
}

impl TaskCatalog for InMemoryTaskRegistry {
    fn list(&self) -> Result<Vec<TaskDescriptor>, TaskRegistrationError> {
        Ok(self
            .entries
            .values()
            .map(|entry| entry.descriptor().clone())
            .collect())
    }

    fn get(
        &self,
        id: &str,
        version: Option<&str>,
    ) -> Result<TaskDescriptor, TaskRegistrationError> {
        Ok(self.resolve_entry(id, version)?.descriptor().clone())
    }
}

impl TaskFactoryRegistry for InMemoryTaskRegistry {
    fn create(
        &self,
        id: &str,
        version: Option<&str>,
        input: Value,
    ) -> Result<TaskDefinition, TaskRegistrationError> {
        let entry = self.resolve_entry(id, version)?;
        let descriptor = entry.descriptor();

        if !descriptor.constructible {
            return Err(TaskRegistrationError::NotConstructible {
                id: descriptor.id.clone(),
                version: descriptor.version.clone(),
            });
        }

        match entry {
            InMemoryTaskEntry::Descriptor(descriptor) => {
                Err(TaskRegistrationError::NotConstructible {
                    id: descriptor.id.clone(),
                    version: descriptor.version.clone(),
                })
            }
            InMemoryTaskEntry::Factory(factory) => factory.create(input),
        }
    }
}

fn validate_descriptor(descriptor: &TaskDescriptor) -> Result<(), TaskRegistrationError> {
    if descriptor.id_source == TaskIdSource::Explicit {
        validate_explicit_task_id(&descriptor.id)?;
    }
    validate_task_version(&descriptor.version)?;
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
    use crate::async_trait;
    use crate::inventory::Host;
    use crate::task::{
        BlockingTaskRuntimeContext, HostTaskResult, Task, TaskError, TaskInfo, TaskRuntimeContext,
        TaskSuccess,
    };
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

    fn explicit_descriptor(id: &str, version: &str, constructible: bool) -> TaskDescriptor {
        TaskDescriptor::explicit(id, version, descriptor_metadata(), None, constructible)
    }

    fn generated_descriptor(id: &str, version: &str) -> TaskDescriptor {
        TaskDescriptor::generated(id, version, descriptor_metadata())
    }

    struct RegistryTestTask {
        name: &'static str,
    }

    impl TaskInfo for RegistryTestTask {
        fn name(&self) -> &str {
            self.name
        }
    }

    #[async_trait]
    impl Task for RegistryTestTask {
        fn start(
            &self,
            _host: &Host,
            _context: &BlockingTaskRuntimeContext,
        ) -> Result<HostTaskResult, TaskError> {
            Ok(HostTaskResult::passed(TaskSuccess::new()))
        }

        async fn start_async(
            &self,
            _host: &Host,
            _context: &TaskRuntimeContext,
        ) -> Result<HostTaskResult, TaskError> {
            Ok(HostTaskResult::passed(TaskSuccess::new()))
        }

        fn execution_mode(&self) -> TaskExecutionMode {
            TaskExecutionMode::Async
        }
    }

    struct RegistryTestFactory {
        descriptor: TaskDescriptor,
    }

    impl RegistryTestFactory {
        fn new(id: &str, version: &str) -> Self {
            Self {
                descriptor: explicit_descriptor(id, version, true),
            }
        }
    }

    impl RegisteredTaskFactory for RegistryTestFactory {
        fn descriptor(&self) -> &TaskDescriptor {
            &self.descriptor
        }

        fn create(&self, input: Value) -> Result<TaskDefinition, TaskRegistrationError> {
            if input != json!({ "ok": true }) {
                return Err(TaskRegistrationError::InvalidInput {
                    id: self.descriptor.id.clone(),
                    version: self.descriptor.version.clone(),
                    message: "expected ok flag".to_string(),
                });
            }

            Ok(TaskDefinition::new(RegistryTestTask {
                name: "registry_test_task",
            }))
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

    #[test]
    fn in_memory_registry_lists_descriptors_in_deterministic_order() {
        let registry = InMemoryTaskRegistry::from_descriptors([
            explicit_descriptor("zeta.task", "1.0.0", false),
            explicit_descriptor("acme.task", "2.0.0", false),
            explicit_descriptor("acme.task", "1.0.0", false),
        ])
        .expect("descriptors should register");

        let listed = registry.list().expect("list should succeed");
        let identities = listed
            .iter()
            .map(|descriptor| format!("{}@{}", descriptor.id, descriptor.version))
            .collect::<Vec<_>>();

        assert_eq!(
            identities,
            vec!["acme.task@1.0.0", "acme.task@2.0.0", "zeta.task@1.0.0"]
        );
    }

    #[test]
    fn in_memory_registry_gets_exact_version() {
        let registry = InMemoryTaskRegistry::from_descriptors([
            explicit_descriptor("acme.task", "1.0.0", false),
            explicit_descriptor("acme.task", "2.0.0", false),
        ])
        .expect("descriptors should register");

        let descriptor = registry
            .get("acme.task", Some("2.0.0"))
            .expect("descriptor should exist");

        assert_eq!(descriptor.id, "acme.task");
        assert_eq!(descriptor.version, "2.0.0");
    }

    #[test]
    fn in_memory_registry_get_without_version_succeeds_for_single_match() {
        let registry = InMemoryTaskRegistry::from_descriptors([
            explicit_descriptor("acme.task", "1.0.0", false),
            explicit_descriptor("zeta.task", "1.0.0", false),
        ])
        .expect("descriptors should register");

        let descriptor = registry
            .get("acme.task", None)
            .expect("single version should resolve");

        assert_eq!(descriptor.id, "acme.task");
        assert_eq!(descriptor.version, "1.0.0");
    }

    #[test]
    fn in_memory_registry_get_without_version_rejects_ambiguity() {
        let registry = InMemoryTaskRegistry::from_descriptors([
            explicit_descriptor("acme.task", "1.0.0", false),
            explicit_descriptor("acme.task", "2.0.0", false),
        ])
        .expect("descriptors should register");

        let error = registry
            .get("acme.task", None)
            .expect_err("multiple versions should be ambiguous");

        assert_eq!(
            error,
            TaskRegistrationError::AmbiguousVersion {
                id: "acme.task".to_string(),
                versions: vec!["1.0.0".to_string(), "2.0.0".to_string()],
            }
        );
    }

    #[test]
    fn in_memory_registry_get_returns_not_found() {
        let registry = InMemoryTaskRegistry::from_descriptors([explicit_descriptor(
            "acme.task",
            "1.0.0",
            false,
        )])
        .expect("descriptors should register");

        assert_eq!(
            registry
                .get("missing.task", None)
                .expect_err("missing id should fail"),
            TaskRegistrationError::NotFound {
                id: "missing.task".to_string(),
                version: None,
            }
        );
        assert_eq!(
            registry
                .get("acme.task", Some("2.0.0"))
                .expect_err("missing version should fail"),
            TaskRegistrationError::NotFound {
                id: "acme.task".to_string(),
                version: Some("2.0.0".to_string()),
            }
        );
    }

    #[test]
    fn in_memory_registry_get_by_identity_parses_task_key() {
        let registry = InMemoryTaskRegistry::from_descriptors([explicit_descriptor(
            "acme.task",
            "1.0.0",
            false,
        )])
        .expect("descriptors should register");

        let descriptor = registry
            .get_by_identity("acme.task@1.0.0")
            .expect("identity lookup should succeed");

        assert_eq!(descriptor.id, "acme.task");
        assert_eq!(descriptor.version, "1.0.0");
    }

    #[test]
    fn in_memory_registry_get_by_identity_rejects_invalid_identity() {
        let registry = InMemoryTaskRegistry::from_descriptors([explicit_descriptor(
            "acme.task",
            "1.0.0",
            false,
        )])
        .expect("descriptors should register");

        assert_eq!(
            registry
                .get_by_identity("acme.task")
                .expect_err("invalid identity should fail"),
            TaskRegistrationError::InvalidIdentity {
                identity: "acme.task".to_string(),
                reason: "identity must contain exactly one `@` separator".to_string(),
            }
        );
    }

    #[test]
    fn in_memory_registry_rejects_duplicate_id_and_version() {
        let error = InMemoryTaskRegistry::from_descriptors([
            explicit_descriptor("acme.task", "1.0.0", false),
            explicit_descriptor("acme.task", "1.0.0", false),
        ])
        .err()
        .expect("duplicate descriptors should fail");

        assert_eq!(
            error,
            TaskRegistrationError::DuplicateRegistration {
                id: "acme.task".to_string(),
                version: "1.0.0".to_string(),
            }
        );
    }

    #[test]
    fn in_memory_registry_validates_explicit_descriptors_only_as_explicit_ids() {
        let generated =
            generated_descriptor("auto:acme_network_tasks::network::ConfigureAcl", "1.0.0");
        InMemoryTaskRegistry::from_descriptors([generated])
            .expect("generated ids should not use explicit id validation");

        let explicit = explicit_descriptor(
            "auto:acme_network_tasks::network::ConfigureAcl",
            "1.0.0",
            false,
        );
        assert!(matches!(
            InMemoryTaskRegistry::from_descriptors([explicit]),
            Err(TaskRegistrationError::InvalidId { .. })
        ));
    }

    #[test]
    fn in_memory_registry_validates_descriptor_versions() {
        let descriptor =
            generated_descriptor("auto:acme_network_tasks::network::ConfigureAcl", "latest");

        assert!(matches!(
            InMemoryTaskRegistry::from_descriptors([descriptor]),
            Err(TaskRegistrationError::InvalidVersion { .. })
        ));
    }

    #[test]
    fn in_memory_registry_creates_factory_backed_task() {
        let mut registry = InMemoryTaskRegistry::new();
        registry
            .register_factory(RegistryTestFactory::new("acme.task", "1.0.0"))
            .expect("factory should register");

        let task = registry
            .create("acme.task", None, json!({ "ok": true }))
            .expect("factory should create task");

        assert_eq!(task.as_task().name(), "registry_test_task");
    }

    #[test]
    fn in_memory_registry_create_by_identity_parses_task_key() {
        let mut registry = InMemoryTaskRegistry::new();
        registry
            .register_factory(RegistryTestFactory::new("acme.task", "1.0.0"))
            .expect("factory should register");

        let task = registry
            .create_by_identity("acme.task@1.0.0", json!({ "ok": true }))
            .expect("identity create should succeed");

        assert_eq!(task.as_task().name(), "registry_test_task");
    }

    #[test]
    fn in_memory_registry_propagates_factory_errors_without_raw_input() {
        let mut registry = InMemoryTaskRegistry::new();
        registry
            .register_factory(RegistryTestFactory::new("acme.task", "1.0.0"))
            .expect("factory should register");

        let error = registry
            .create("acme.task", Some("1.0.0"), json!({ "secret": "value" }))
            .expect_err("factory should reject input");

        assert_eq!(
            error,
            TaskRegistrationError::InvalidInput {
                id: "acme.task".to_string(),
                version: "1.0.0".to_string(),
                message: "expected ok flag".to_string(),
            }
        );
        assert!(!error.to_string().contains("secret"));
        assert!(!error.to_string().contains("value"));
    }

    #[test]
    fn in_memory_registry_rejects_descriptor_only_create() {
        let registry = InMemoryTaskRegistry::from_descriptors([explicit_descriptor(
            "acme.task",
            "1.0.0",
            false,
        )])
        .expect("descriptor should register");

        let error = registry
            .create("acme.task", None, Value::Null)
            .expect_err("descriptor-only entry is not constructible");

        assert_eq!(
            error,
            TaskRegistrationError::NotConstructible {
                id: "acme.task".to_string(),
                version: "1.0.0".to_string(),
            }
        );
    }
}
