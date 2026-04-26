//! Host filtering functionality for Genja.
//!
//! This module provides filtering capabilities for hosts based on key existence and key-value matching
//! with regex patterns. It supports both simple keys and nested paths using dot notation, and can
//! traverse through JSON objects and arrays.
//!
//! # Overview
//!
//! The module offers two main filter types:
//!
//! - [`KeyFilter`] - Matches hosts based on the presence of a specific key
//! - [`ValueFilter`] - Matches hosts based on key existence and value pattern matching
//!
//! Both filters serialize hosts to JSON and perform recursive searches through the data structure,
//! supporting nested paths with dot notation (e.g., `"data.metadata.owner.name"`) and traversal
//! through arrays.
//!
//! # Function Call Chain
//!
//! ```text
//! KeyFilter::matches()
//!   └─> json_has_key()
//!       ├─> json_contains_key() [for simple keys]
//!       │   └─> (recursive)
//!       └─> json_contains_path() [for dot paths]
//!           ├─> path_exists()
//!           │   └─> (recursive)
//!           └─> (recursive)
//!
//! ValueFilter::matches()
//!   └─> json_matches_key_value()
//!       ├─> json_matches_key() [for simple keys]
//!       │   ├─> json_value_match_text()
//!       │   └─> (recursive)
//!       └─> json_matches_path() [for dot paths]
//!           ├─> path_value_matches()
//!           │   ├─> json_value_match_text()
//!           │   └─> (recursive)
//!           └─> (recursive)
//! ```
//!
//! # Examples
//!
//! ## Key Filtering
//!
//! ```rust,ignore
//! use genja::filter::KeyFilter;
//! use genja_core::inventory::{Host, BaseBuilderHost, Data};
//! use serde_json::json;
//!
//! let host = Host::builder()
//!     .hostname("router1")
//!     .data(Data::new(json!({
//!         "site": {
//!             "name": "lab-a"
//!         }
//!     })))
//!     .build();
//!
//! // Match by simple key
//! let filter = KeyFilter::new("hostname");
//! assert!(filter.matches(&host));
//!
//! // Match by nested path
//! let filter = KeyFilter::new("data.site.name");
//! assert!(filter.matches(&host));
//! ```
//!
//! ## Value Filtering
//!
//! ```rust,ignore
//! use genja::filter::ValueFilter;
//! use genja_core::inventory::{Host, BaseBuilderHost, Data};
//! use serde_json::json;
//!
//! let host = Host::builder()
//!     .hostname("router1")
//!     .platform("ios-xe")
//!     .data(Data::new(json!({
//!         "role": "core"
//!     })))
//!     .build();
//!
//! // Match by regex pattern
//! let filter = ValueFilter::new("platform", "^ios").unwrap();
//! assert!(filter.matches(&host));
//!
//! // Match nested value
//! let filter = ValueFilter::new("role", "^(core|distribution)$").unwrap();
//! assert!(filter.matches(&host));
//! ```
//!
//! # Path Traversal
//!
//! The filters support two traversal strategies:
//!
//! 1. **Direct path traversal** - Follows path segments sequentially through object keys
//! 2. **Recursive search** - Searches for the path starting from any nested value
//!
//! This allows matching keys/values at any nesting level, including within arrays:
//!
//! ```rust,ignore
//! use genja::filter::KeyFilter;
//! use genja_core::inventory::{Host, BaseBuilderHost, Data};
//! use serde_json::json;
//!
//! let host = Host::builder()
//!     .hostname("router1")
//!     .data(Data::new(json!({
//!         "devices": [
//!             {"role": "core"},
//!             {"role": "edge"}
//!         ]
//!     })))
//!     .build();
//!
//! // Matches keys inside arrays
//! let filter = KeyFilter::new("devices.role");
//! assert!(filter.matches(&host));
//! ```

use genja_core::GenjaError;
use genja_core::inventory::Host;
use regex::Regex;
use serde_json::Value;

/// A filter that matches hosts based on whether a specific key exists and its value matches a regex pattern.
///
/// This filter serializes a host to JSON and searches for a key-value pair where the key matches
/// the specified path (supporting dot notation for nested fields) and the value matches the provided regex.
pub(crate) struct ValueFilter {
    key: String,
    value_regex: Regex,
}

/// A filter that matches hosts based on the presence of a specific key.
///
/// This filter serializes a host to JSON and searches for the existence of a key,
/// supporting dot notation for nested field paths and traversing arrays.
pub(crate) struct KeyFilter {
    key: String,
}

impl KeyFilter {
    /// Creates a new `KeyFilter` with the specified key path.
    ///
    /// # Parameters
    ///
    /// * `key` - The key or dot-separated path to search for in the host data.
    ///           Supports nested paths like "data.metadata.owner.name" and paths through arrays.
    ///
    /// # Returns
    ///
    /// A new `KeyFilter` instance configured to match the specified key.
    pub(crate) fn new(key: &str) -> Self {
        Self {
            key: key.to_string(),
        }
    }

    /// Checks whether the specified key exists in the host's data structure.
    ///
    /// The host is serialized to JSON and the key is searched at any nesting level,
    /// including within arrays and nested objects. Supports dot notation for path traversal.
    ///
    /// # Parameters
    ///
    /// * `host` - A reference to the `Host` to check for the key's existence.
    ///
    /// # Returns
    ///
    /// `true` if the key exists anywhere in the host's data structure, `false` otherwise.
    /// Returns `false` if the host cannot be serialized to JSON.
    pub(crate) fn matches(&self, host: &Host) -> bool {
        let Ok(host_value) = serde_json::to_value(host) else {
            return false;
        };

        json_has_key(&host_value, &self.key)
    }
}

impl ValueFilter {
    /// Creates a new `ValueFilter` with the specified key path and value pattern.
    ///
    /// # Parameters
    ///
    /// * `key` - The key or dot-separated path to search for in the host data.
    ///           Supports nested paths like "data.metadata.owner.name" and paths through arrays.
    /// * `value_pattern` - A regular expression pattern that the value associated with the key must match.
    ///
    /// # Returns
    ///
    /// * `Ok(ValueFilter)` - A new `ValueFilter` instance configured with the specified key and pattern.
    /// * `Err(GenjaError)` - If the provided `value_pattern` is not a valid regular expression.
    pub(crate) fn new(key: &str, value_pattern: &str) -> Result<Self, GenjaError> {
        let value_regex = Regex::new(value_pattern)
            .map_err(|err| GenjaError::Message(format!("invalid value regex: {err}")))?;

        Ok(Self {
            key: key.to_string(),
            value_regex,
        })
    }

    /// Checks whether the specified key exists in the host's data structure and its value matches the regex pattern.
    ///
    /// The host is serialized to JSON and the key-value pair is searched at any nesting level,
    /// including within arrays and nested objects. Supports dot notation for path traversal.
    ///
    /// # Parameters
    ///
    /// * `host` - A reference to the `Host` to check for the key-value match.
    ///
    /// # Returns
    ///
    /// `true` if the key exists anywhere in the host's data structure and its associated value
    /// matches the configured regex pattern, `false` otherwise.
    /// Returns `false` if the host cannot be serialized to JSON.
    pub(crate) fn matches(&self, host: &Host) -> bool {
        let Ok(host_value) = serde_json::to_value(host) else {
            return false;
        };

        json_matches_key_value(&host_value, &self.key, &self.value_regex)
    }
}

/// Checks whether a key or dot-separated path exists in a JSON value.
///
/// This function supports both simple keys and nested paths using dot notation.
/// For simple keys (no dots), it searches recursively through the entire JSON structure.
/// For paths with dots, it attempts to traverse the structure following the path segments.
///
/// # Parameters
///
/// * `value` - A reference to the JSON `Value` to search within.
/// * `key` - The key or dot-separated path to search for (e.g., "name" or "metadata.owner.name").
///           Empty path segments are filtered out.
///
/// # Returns
///
/// `true` if the key or path exists in the JSON structure, `false` otherwise.
/// Returns `false` if the key is empty or contains only dots.
fn json_has_key(value: &Value, key: &str) -> bool {
    let path: Vec<&str> = key.split('.').filter(|part| !part.is_empty()).collect();
    if path.is_empty() {
        return false;
    }

    if path.len() == 1 {
        return json_contains_key(value, path[0]);
    }

    json_contains_path(value, &path)
}

/// Checks whether a key or dot-separated path exists in a JSON value and its associated value matches a regex pattern.
///
/// This function supports both simple keys and nested paths using dot notation.
/// For simple keys (no dots), it searches recursively through the entire JSON structure.
/// For paths with dots, it attempts to traverse the structure following the path segments.
///
/// # Parameters
///
/// * `value` - A reference to the JSON `Value` to search within.
/// * `key` - The key or dot-separated path to search for (e.g., "name" or "metadata.owner.name").
///           Empty path segments are filtered out.
/// * `value_regex` - A reference to the compiled `Regex` pattern that the value must match.
///
/// # Returns
///
/// `true` if the key or path exists in the JSON structure and its associated value matches
/// the regex pattern, `false` otherwise. Returns `false` if the key is empty or contains only dots.
fn json_matches_key_value(value: &Value, key: &str, value_regex: &Regex) -> bool {
    let path: Vec<&str> = key.split('.').filter(|part| !part.is_empty()).collect();
    if path.is_empty() {
        return false;
    }

    if path.len() == 1 {
        return json_matches_key(value, path[0], value_regex);
    }

    json_matches_path(value, &path, value_regex)
}

/// Recursively searches for a key in a JSON value structure.
///
/// This function performs a depth-first search through the JSON structure, checking
/// if the specified key exists at any level. It traverses both objects and arrays,
/// searching through all nested values until the key is found or all paths are exhausted.
///
/// # Parameters
///
/// * `value` - A reference to the JSON `Value` to search within. Can be any JSON type
///             (object, array, string, number, boolean, or null).
/// * `key` - The exact key name to search for. Must match a key in a JSON object exactly
///           (case-sensitive, no partial matches).
///
/// # Returns
///
/// `true` if the key exists as a direct key in any JSON object within the structure,
/// `false` otherwise. Returns `false` for non-object, non-array values.
fn json_contains_key(value: &Value, key: &str) -> bool {
    match value {
        Value::Object(map) => map.iter().any(|(candidate_key, candidate_value)| {
            candidate_key == key || json_contains_key(candidate_value, key)
        }),
        Value::Array(values) => values.iter().any(|value| json_contains_key(value, key)),
        _ => false,
    }
}

/// Recursively searches for a key in a JSON value structure and checks if its associated value matches a regex pattern.
///
/// This function performs a depth-first search through the JSON structure, checking
/// if the specified key exists at any level and whether its associated value matches
/// the provided regex pattern. It traverses both objects and arrays, searching through
/// all nested values until a matching key-value pair is found or all paths are exhausted.
///
/// # Parameters
///
/// * `value` - A reference to the JSON `Value` to search within. Can be any JSON type
///             (object, array, string, number, boolean, or null).
/// * `key` - The exact key name to search for. Must match a key in a JSON object exactly
///           (case-sensitive, no partial matches).
/// * `value_regex` - A reference to the compiled `Regex` pattern that the value associated
///                   with the key must match.
///
/// # Returns
///
/// `true` if the key exists as a direct key in any JSON object within the structure and
/// its associated value matches the regex pattern, `false` otherwise. Returns `false` for
/// non-object, non-array values.
fn json_matches_key(value: &Value, key: &str, value_regex: &Regex) -> bool {
    match value {
        Value::Object(map) => map.iter().any(|(candidate_key, candidate_value)| {
            (candidate_key == key && value_regex.is_match(&json_value_match_text(candidate_value)))
                || json_matches_key(candidate_value, key, value_regex)
        }),
        Value::Array(values) => values
            .iter()
            .any(|value| json_matches_key(value, key, value_regex)),
        _ => false,
    }
}

/// Recursively searches for a dot-separated path in a JSON value structure.
///
/// This function performs a depth-first search through the JSON structure, attempting
/// to follow the specified path through nested objects and arrays. It tries two strategies:
/// 1. Direct path traversal: Following the path segments sequentially through object keys
/// 2. Recursive search: Searching for the path starting from any nested value
///
/// # Parameters
///
/// * `value` - A reference to the JSON `Value` to search within. Can be any JSON type
///             (object, array, string, number, boolean, or null).
/// * `path` - A slice of path segments representing the dot-separated path to search for.
///            Each segment represents a key to look up in objects. Empty path indicates
///            the target has been reached.
///
/// # Returns
///
/// `true` if the complete path exists in the JSON structure (either through direct traversal
/// or by finding it nested within the structure), `false` otherwise. Returns `false` for
/// non-object, non-array values.
fn json_contains_path(value: &Value, path: &[&str]) -> bool {
    match value {
        Value::Object(map) => {
            path_exists(map.get(path[0]), &path[1..])
                || map.values().any(|value| json_contains_path(value, path))
        }
        Value::Array(values) => values.iter().any(|value| json_contains_path(value, path)),
        _ => false,
    }
}

/// Recursively searches for a dot-separated path in a JSON value structure and checks if the final value matches a regex pattern.
///
/// This function performs a depth-first search through the JSON structure, attempting
/// to follow the specified path through nested objects and arrays while checking if the
/// final value matches the provided regex pattern. It tries two strategies:
/// 1. Direct path traversal: Following the path segments sequentially through object keys
///    and checking if the final value matches the regex
/// 2. Recursive search: Searching for the path starting from any nested value
///
/// # Parameters
///
/// * `value` - A reference to the JSON `Value` to search within. Can be any JSON type
///             (object, array, string, number, boolean, or null).
/// * `path` - A slice of path segments representing the dot-separated path to search for.
///            Each segment represents a key to look up in objects. Empty path indicates
///            the target has been reached.
/// * `value_regex` - A reference to the compiled `Regex` pattern that the value associated
///                   with the path must match.
///
/// # Returns
///
/// `true` if the complete path exists in the JSON structure and its associated value matches
/// the regex pattern (either through direct traversal or by finding it nested within the structure),
/// `false` otherwise. Returns `false` for non-object, non-array values.
fn json_matches_path(value: &Value, path: &[&str], value_regex: &Regex) -> bool {
    match value {
        Value::Object(map) => {
            path_value_matches(map.get(path[0]), &path[1..], value_regex)
                || map
                    .values()
                    .any(|value| json_matches_path(value, path, value_regex))
        }
        Value::Array(values) => values
            .iter()
            .any(|value| json_matches_path(value, path, value_regex)),
        _ => false,
    }
}

/// Recursively traverses a JSON value following a path to check if the path exists.
///
/// This function follows a specific path through a JSON structure by consuming path segments
/// one at a time. When the path is exhausted, it returns `true` indicating the path exists.
/// It handles both objects (by looking up keys) and arrays (by checking all elements).
///
/// # Parameters
///
/// * `value` - An optional reference to the JSON `Value` to traverse. If `None`, the function
///             returns `false` immediately.
/// * `path` - A slice of path segments to follow through the JSON structure. Each segment
///            represents a key to look up in objects. When empty, indicates the target value
///            has been reached.
///
/// # Returns
///
/// `true` if the path can be successfully traversed to completion, `false` otherwise.
/// Returns `false` if the value is `None` or the path cannot be followed through the structure.
fn path_exists(value: Option<&Value>, path: &[&str]) -> bool {
    let Some(value) = value else {
        return false;
    };

    if path.is_empty() {
        return true;
    }

    match value {
        Value::Object(map) => path_exists(map.get(path[0]), &path[1..]),
        Value::Array(values) => values.iter().any(|value| path_exists(Some(value), path)),
        _ => false,
    }
}

/// Recursively traverses a JSON value following a path and checks if the final value matches a regex pattern.
///
/// This function follows a specific path through a JSON structure by consuming path segments
/// one at a time. When the path is exhausted, it checks if the reached value matches the
/// provided regex pattern. It handles both objects (by looking up keys) and arrays (by
/// checking all elements).
///
/// # Parameters
///
/// * `value` - An optional reference to the JSON `Value` to traverse. If `None`, the function
///             returns `false` immediately.
/// * `path` - A slice of path segments to follow through the JSON structure. Each segment
///            represents a key to look up in objects. When empty, indicates the target value
///            has been reached.
/// * `value_regex` - A reference to the compiled `Regex` pattern that the final value must match.
///
/// # Returns
///
/// `true` if the path can be successfully traversed and the final value matches the regex pattern,
/// `false` otherwise. Returns `false` if the value is `None`, the path cannot be followed, or
/// the final value doesn't match the pattern.
fn path_value_matches(value: Option<&Value>, path: &[&str], value_regex: &Regex) -> bool {
    let Some(value) = value else {
        return false;
    };

    if path.is_empty() {
        return value_regex.is_match(&json_value_match_text(value));
    }

    match value {
        Value::Object(map) => path_value_matches(map.get(path[0]), &path[1..], value_regex),
        Value::Array(values) => values
            .iter()
            .any(|value| path_value_matches(Some(value), path, value_regex)),
        _ => false,
    }
}

/// Converts a JSON value to a string representation suitable for regex matching.
///
/// This function extracts a textual representation from various JSON value types
/// to enable pattern matching with regular expressions. For primitive types, it
/// returns their natural string representation. For complex types (arrays and objects),
/// it returns their JSON serialization.
///
/// # Parameters
///
/// * `value` - A reference to the JSON `Value` to convert to text. Can be any JSON type
///             (string, null, boolean, number, array, or object).
///
/// # Returns
///
/// A `String` containing the textual representation of the value:
/// - For `String`: Returns a clone of the string value
/// - For `Null`: Returns the literal string "null"
/// - For `Bool`: Returns "true" or "false"
/// - For `Number`: Returns the numeric value as a string
/// - For `Array` or `Object`: Returns the JSON serialization of the structure
fn json_value_match_text(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        Value::Null => "null".to_string(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::Array(_) | Value::Object(_) => value.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::{KeyFilter, ValueFilter};
    use genja_core::inventory::{BaseBuilderHost, Data, Host};
    use serde_json::json;

    fn matching_host() -> Host {
        Host::builder()
            .hostname("10.0.0.1")
            .platform("ios-xe")
            .data(Data::new(json!({
                "site": {
                    "name": "lab-a",
                    "devices": [
                        {"role": "core"},
                        {"role": "edge"}
                    ]
                },
                "metadata": {
                    "owner": {
                        "name": "network-team"
                    },
                    "tag": null
                }
            })))
            .build()
    }

    #[test]
    fn matches_fixed_host_field() {
        let filter = ValueFilter::new("platform", "^ios").expect("regex should compile");

        assert!(filter.matches(&matching_host()));
    }

    #[test]
    fn key_filter_matches_fixed_host_field() {
        let filter = KeyFilter::new("platform");

        assert!(filter.matches(&matching_host()));
    }

    #[test]
    fn key_filter_matches_nested_data_key_at_any_level() {
        let filter = KeyFilter::new("role");

        assert!(filter.matches(&matching_host()));
    }

    #[test]
    fn key_filter_matches_nested_dot_path() {
        let filter = KeyFilter::new("data.metadata.owner.name");

        assert!(filter.matches(&matching_host()));
    }

    #[test]
    fn key_filter_matches_dot_path_inside_arrays() {
        let filter = KeyFilter::new("site.devices.role");

        assert!(filter.matches(&matching_host()));
    }

    #[test]
    fn key_filter_matches_null_value() {
        let filter = KeyFilter::new("metadata.tag");

        assert!(filter.matches(&matching_host()));
    }

    #[test]
    fn key_filter_does_not_match_missing_key() {
        let filter = KeyFilter::new("missing");

        assert!(!filter.matches(&matching_host()));
    }

    #[test]
    fn matches_nested_data_key_at_any_level() {
        let filter =
            ValueFilter::new("role", "^(core|distribution)$").expect("regex should compile");

        assert!(filter.matches(&matching_host()));
    }

    #[test]
    fn matches_nested_dot_path() {
        let filter = ValueFilter::new("data.metadata.owner.name", "network-team")
            .expect("regex should compile");

        assert!(filter.matches(&matching_host()));
    }

    #[test]
    fn matches_dot_path_inside_arrays() {
        let filter = ValueFilter::new("site.devices.role", "edge").expect("regex should compile");

        assert!(filter.matches(&matching_host()));
    }

    #[test]
    fn does_not_match_wrong_value() {
        let filter = ValueFilter::new("role", "access").expect("regex should compile");

        assert!(!filter.matches(&matching_host()));
    }
}
