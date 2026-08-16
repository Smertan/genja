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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn descriptor_serializes_to_canonical_field_names() {
        let descriptor = TaskDescriptor {
            id: "acme.network.configure_acl".to_string(),
            id_source: TaskIdSource::Explicit,
            name: "configure_acl".to_string(),
            version: "2.0.0".to_string(),
            description: Some("Configures an ACL on a network device".to_string()),
            execution_mode: TaskExecutionMode::Async,
            connection_plugin_name: Some("ssh".to_string()),
            processor_names: vec!["audit".to_string()],
            retry: Some(RetryConfig::builder().allow(true).max_attempts(3).build()),
            input_schema: Some(json!({
                "type": "object",
                "properties": {
                    "acl_name": { "type": "string" }
                }
            })),
            constructible: true,
        };

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
        let descriptor = TaskDescriptor {
            id: "auto:acme_network_tasks::network::ConfigureAcl".to_string(),
            id_source: TaskIdSource::Generated,
            name: "configure_acl".to_string(),
            version: "2.0.0".to_string(),
            description: None,
            execution_mode: TaskExecutionMode::Blocking,
            connection_plugin_name: None,
            processor_names: Vec::new(),
            retry: None,
            input_schema: None,
            constructible: false,
        };

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
}
