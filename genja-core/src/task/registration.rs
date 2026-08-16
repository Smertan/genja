//! Task registration and discovery descriptor types.
//!
//! This module defines the serializable task descriptor contract used by
//! local Rust task discovery and future catalog, provider manifest, MCP, and
//! Python registration integrations. The descriptor is metadata only; local
//! construction from JSON input is handled by later factory registry APIs.

use super::{RetryConfig, TaskExecutionMode};
use serde::{Deserialize, Serialize};
use serde_json::Value;

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
}
