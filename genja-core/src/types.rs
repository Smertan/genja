//! Common types used across Genja Core.
//!
//! This module provides shared utility types like `NatString` for natural
//! string ordering and `CustomTreeMap` for maps keyed by `NatString`.

use natord::compare;
use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::btree_map::{IntoIter, Iter, Keys, Values};
use std::collections::BTreeMap;
use std::fmt;
use std::ops::{Deref, DerefMut};

/// A wrapper type for strings that implements natural (alphanumeric) ordering.
///
/// `NatString` wraps a `String` and provides custom ordering behavior where
/// numeric portions of strings are compared numerically rather than lexicographically.
/// For example, "item2" will be ordered before "item10" (natural order) instead of
/// after it (lexicographic order).
///
/// If the natural comparator considers two distinct strings equal, `NatString`
/// falls back to standard string ordering. This keeps ordering consistent with
/// equality, which is required for safe use as a `BTreeMap` key.
///
/// This type is typically used as a key in ordered collections like `BTreeMap`
/// when natural sorting of string keys is desired.
///
/// # Examples
///
/// ```
/// # use genja_core::NatString;
/// let s1 = NatString::new("file2".to_string());
/// let s2 = NatString::new("file10".to_string());
/// assert!(s1 < s2);
///
/// let compact = NatString::from("host1");
/// let spaced = NatString::from("host 1");
/// assert_ne!(compact, spaced);
/// assert_ne!(compact.cmp(&spaced), std::cmp::Ordering::Equal);
/// ```
#[derive(PartialEq, Eq, Clone, Hash, JsonSchema, Serialize, Deserialize)]
pub struct NatString(String);

impl Deref for NatString {
    type Target = String;

    // Implement the deref method, returning an immutable reference
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for NatString {
    // Implement the deref method, returning an immutable reference
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<NatString> for String {
    fn from(value: NatString) -> Self {
        value.0
    }
}

impl From<&NatString> for String {
    fn from(value: &NatString) -> Self {
        value.0.clone()
    }
}

impl From<String> for NatString {
    fn from(value: String) -> Self {
        NatString(value)
    }
}

impl From<&str> for NatString {
    fn from(value: &str) -> Self {
        NatString(value.to_string())
    }
}

impl From<&String> for NatString {
    fn from(value: &String) -> Self {
        NatString(value.clone())
    }
}

impl From<&NatString> for NatString {
    fn from(value: &NatString) -> Self {
        value.clone()
    }
}

impl NatString {
    /// Creates a new `NatString` from a `String`.
    pub fn new(s: String) -> Self {
        NatString(s)
    }

    /// Returns the inner string as `&str`.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for NatString {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Debug for NatString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Use write! to format the fields directly without the struct wrapper
        write!(f, "{}", self.0)
    }
}

impl fmt::Display for NatString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl Ord for NatString {
    fn cmp(&self, other: &Self) -> Ordering {
        match compare(&self.0, &other.0) {
            Ordering::Equal => self.0.cmp(&other.0),
            ordering => ordering,
        }
    }
}

impl PartialOrd for NatString {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// A wrapper around `BTreeMap` that uses natural ordering for string keys.
///
/// `CustomTreeMap` provides a map data structure where keys are automatically sorted
/// using natural (alphanumeric) ordering instead of lexicographic ordering.
/// For example, "host2" will come before "host10" in the natural order.
///
/// The underlying storage is intentionally private. Use the explicit methods on
/// this type to insert, retrieve, remove, and iterate entries without depending
/// on its internal representation.
///
/// ## Examples
///
/// ```
/// # use genja_core::CustomTreeMap;
/// let mut tree = CustomTreeMap::new();
/// tree.insert("host1", "value1".to_string());
/// tree.insert("host2", "value2".to_string());
/// tree.insert("host10", "value10".to_string());
///
/// let keys: Vec<&str> = tree.keys().map(|key| key.as_str()).collect();
/// assert_eq!(keys, vec!["host1", "host2", "host10"]);
/// ```
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)] // JsonSchema
pub struct CustomTreeMap<V>(BTreeMap<NatString, V>);

impl<V: fmt::Debug> fmt::Debug for CustomTreeMap<V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            // pretty print the map using the debug_struct builder pattern
            f.debug_struct("CustomTreeMap")
                .field("BTreeMap", &self.0)
                .finish()
        } else {
            // Use write! to format the fields directly without the struct wrapper
            write!(f, "{:?}", self.0)
        }
    }
}

impl<V: fmt::Debug> fmt::Display for CustomTreeMap<V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Use write! to format the fields directly without the struct wrapper
        write!(f, "{:?}", self.0)
    }
}

impl<V> CustomTreeMap<V> {
    /// Creates an empty map.
    pub fn new() -> Self {
        CustomTreeMap(BTreeMap::new())
    }

    /// Inserts a key-value pair into the map.
    ///
    /// Keys are converted into [`NatString`] values and stored in natural order.
    pub fn insert<K>(&mut self, key: K, value: V)
    where
        K: Into<NatString>,
    {
        self.0.insert(key.into(), value);
    }

    /// Returns a reference to the value for the given key, if present.
    pub fn get<K>(&self, key: K) -> Option<&V>
    where
        K: AsRef<str>,
    {
        self.0.get(&NatString::new(key.as_ref().to_string()))
    }

    /// Returns a mutable reference to the value for the given key, if present.
    pub fn get_mut<K>(&mut self, key: K) -> Option<&mut V>
    where
        K: AsRef<str>,
    {
        self.0.get_mut(&NatString::new(key.as_ref().to_string()))
    }

    /// Removes the entry for the given key and returns its value, if present.
    pub fn remove<K>(&mut self, key: K) -> Option<V>
    where
        K: AsRef<str>,
    {
        self.0.remove(&NatString::new(key.as_ref().to_string()))
    }

    /// Returns `true` if the map contains a value for the given key.
    pub fn contains_key<K>(&self, key: K) -> bool
    where
        K: AsRef<str>,
    {
        self.0
            .contains_key(&NatString::new(key.as_ref().to_string()))
    }

    /// Returns an iterator over the key-value pairs in natural key order.
    pub fn iter(&self) -> Iter<'_, NatString, V> {
        self.0.iter()
    }

    /// Returns an iterator over the keys in natural order.
    pub fn keys(&self) -> Keys<'_, NatString, V> {
        self.0.keys()
    }

    /// Returns an iterator over the values in natural key order.
    pub fn values(&self) -> Values<'_, NatString, V> {
        self.0.values()
    }

    /// Returns the number of entries in the map.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if the map contains no entries.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<V> Default for CustomTreeMap<V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<V> IntoIterator for CustomTreeMap<V> {
    type Item = (NatString, V);
    type IntoIter = IntoIter<NatString, V>;

    /// Consumes the map and returns entries in natural key order.
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<V> JsonSchema for CustomTreeMap<V>
where
    V: JsonSchema,
{
    fn schema_name() -> Cow<'static, str> {
        format!("{}", V::schema_name()).into()
    }

    fn json_schema(gen: &mut SchemaGenerator) -> Schema {
        <BTreeMap<String, V>>::json_schema(gen)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use schemars::schema_for;
    use serde_json::json;

    #[test]
    fn test_nat_string_ordering() {
        let s1 = NatString::new("file2".to_string());
        let s2 = NatString::new("file10".to_string());
        assert!(s1 < s2);
    }

    #[test]
    fn test_nat_string_ordering_distinguishes_whitespace() {
        let compact = NatString::from("host1");
        let spaced = NatString::from("host 1");

        assert_ne!(compact, spaced);
        assert_ne!(compact.cmp(&spaced), Ordering::Equal);
    }

    #[test]
    fn test_custom_tree_map_ordering() {
        let mut tree = CustomTreeMap::new();
        tree.insert("host1", "one".to_string());
        tree.insert("host2", "two".to_string());
        tree.insert("host10", "three10".to_string());
        tree.insert("host4", "four1".to_string());
        tree.insert("host100", "100".to_string());
        assert_eq!(tree.get("host1").unwrap(), "one");
        assert_eq!(tree.get("host10").unwrap(), "three10");

        let keys: Vec<&str> = tree.keys().map(NatString::as_str).collect();
        assert_eq!(keys, vec!["host1", "host2", "host4", "host10", "host100"]);
    }

    #[test]
    fn test_custom_tree_map_preserves_distinct_whitespace_keys() {
        let mut tree = CustomTreeMap::new();
        tree.insert("host1", "compact");
        tree.insert("host 1", "spaced");

        assert_eq!(tree.len(), 2);
        assert_eq!(tree.get("host1"), Some(&"compact"));
        assert_eq!(tree.get("host 1"), Some(&"spaced"));

        assert_eq!(tree.remove("host1"), Some("compact"));
        assert_eq!(tree.get("host 1"), Some(&"spaced"));
    }

    #[test]
    fn test_custom_tree_map_explicit_read_api() {
        let mut tree = CustomTreeMap::new();
        tree.insert("host10", "ten");
        tree.insert("host2", "two");
        tree.insert("host1", "one");

        assert!(tree.contains_key("host2"));
        assert!(!tree.contains_key("host3"));

        let entries: Vec<(&str, &str)> = tree
            .iter()
            .map(|(key, value)| (key.as_str(), *value))
            .collect();
        assert_eq!(
            entries,
            vec![("host1", "one"), ("host2", "two"), ("host10", "ten")]
        );

        let values: Vec<&str> = tree.values().copied().collect();
        assert_eq!(values, vec!["one", "two", "ten"]);
    }

    #[test]
    fn test_custom_tree_map_accepts_supported_key_forms() {
        let mut tree = CustomTreeMap::new();
        let string_key = "host1".to_string();
        let nat_key = NatString::from("host2");

        tree.insert(string_key.clone(), "string");
        tree.insert(&nat_key, "nat");

        assert_eq!(tree.get(&string_key), Some(&"string"));
        assert!(tree.contains_key(nat_key.clone()));

        *tree.get_mut(&nat_key).expect("nat key exists") = "updated";
        assert_eq!(tree.remove(nat_key), Some("updated"));
    }

    #[test]
    fn test_custom_tree_map_into_iter_preserves_natural_order() {
        let mut tree = CustomTreeMap::new();
        tree.insert("host10", "ten");
        tree.insert("host2", "two");
        tree.insert("host1", "one");

        let entries: Vec<(String, &str)> = tree
            .into_iter()
            .map(|(key, value)| (key.into(), value))
            .collect();

        assert_eq!(
            entries,
            vec![
                ("host1".to_string(), "one"),
                ("host2".to_string(), "two"),
                ("host10".to_string(), "ten"),
            ]
        );
    }

    #[test]
    fn test_nat_string_json_and_yaml_round_trip_as_string() {
        let key = NatString::from("host10");

        let json = serde_json::to_string(&key).expect("NatString should serialize to JSON");
        assert_eq!(json, "\"host10\"");
        let from_json: NatString =
            serde_json::from_str(&json).expect("NatString should deserialize from JSON");
        assert_eq!(from_json, key);

        let yaml = serde_yaml::to_string(&key).expect("NatString should serialize to YAML");
        let from_yaml: NatString =
            serde_yaml::from_str(&yaml).expect("NatString should deserialize from YAML");
        assert_eq!(from_yaml, key);
    }

    #[test]
    fn test_custom_tree_map_json_round_trip_uses_string_keyed_object() {
        let mut tree = CustomTreeMap::new();
        tree.insert("host10", "ten");
        tree.insert("host2", "two");
        tree.insert("host1", "one");

        let json = serde_json::to_string(&tree).expect("CustomTreeMap should serialize to JSON");
        assert_eq!(json, r#"{"host1":"one","host2":"two","host10":"ten"}"#);

        let round_trip: CustomTreeMap<String> =
            serde_json::from_str(&json).expect("CustomTreeMap should deserialize from JSON");
        let entries: Vec<(&str, &str)> = round_trip
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str()))
            .collect();
        assert_eq!(
            entries,
            vec![("host1", "one"), ("host2", "two"), ("host10", "ten")]
        );
    }

    #[test]
    fn test_custom_tree_map_yaml_round_trip_preserves_keys_and_ordering() {
        let yaml = r#"
host10: ten
host2: two
host1: one
"#;

        let tree: CustomTreeMap<String> =
            serde_yaml::from_str(yaml).expect("CustomTreeMap should deserialize from YAML");
        let keys: Vec<&str> = tree.keys().map(NatString::as_str).collect();
        assert_eq!(keys, vec!["host1", "host2", "host10"]);

        let serialized = serde_yaml::to_string(&tree).expect("CustomTreeMap should serialize YAML");
        let round_trip: CustomTreeMap<String> =
            serde_yaml::from_str(&serialized).expect("serialized YAML should round trip");
        assert_eq!(round_trip, tree);
    }

    #[test]
    fn test_custom_tree_map_json_schema_matches_string_keyed_map_shape() {
        let schema = schema_for!(CustomTreeMap<String>);
        let schema_json =
            serde_json::to_value(&schema).expect("CustomTreeMap schema should serialize");

        assert_eq!(schema_json["type"], json!("object"));
        assert_eq!(schema_json["additionalProperties"]["type"], json!("string"));
    }
}
