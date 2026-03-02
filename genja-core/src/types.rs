use natord::compare;
use pyo3::prelude::*;
use schemars::{JsonSchema, Schema, SchemaGenerator};
// use serde::ser::SerializeMap;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fmt;
use std::ops::{Deref, DerefMut};
// pub mod inventory

pub trait DerefTarget {
    type Target;
}

/// A wrapper type for strings that implements natural (alphanumeric) ordering.
///
/// `NatString` wraps a `String` and provides custom ordering behavior where
/// numeric portions of strings are compared numerically rather than lexicographically.
/// For example, "item2" will be ordered before "item10" (natural order) instead of
/// after it (lexicographic order).
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
/// // s1 < s2 in natural order (2 < 10)
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

impl NatString {
    pub fn new(s: String) -> Self {
        NatString(s)
    }

    pub fn as_str(&self) -> &str {
        &self.0
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
        compare(&self.0, &other.0)
    }
}

impl PartialOrd for NatString {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// A wrapper around `BTreeMap` that uses natural ordering for string keys.
///
/// `MyTree` provides a map data structure where keys are automatically sorted
/// using natural (alphanumeric) ordering instead of lexicographic ordering.
/// For example, "host2" will come before "host10" in the natural order.
///
/// ## Fields
///
/// * `0` - The underlying `BTreeMap` with `NatString` keys and `String` values.
///
/// ## Examples
///
/// ```
/// # use genja_core::CustomTreeMap;
/// let mut tree = CustomTreeMap::new();
/// tree.insert("host1", "value1".to_string());
/// tree.insert("host10", "value10".to_string());
/// // Keys will be ordered naturally: host1, host10
/// ```
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)] // JsonSchema
pub struct CustomTreeMap<V>(BTreeMap<NatString, V>);

impl<V> Deref for CustomTreeMap<V> {
    // Specify the Target type, which is a reference to T
    type Target = BTreeMap<NatString, V>;

    // Implement the deref method, returning an immutable reference
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<V> DerefMut for CustomTreeMap<V> {
    // Implement the deref_mut method, returning a mutable reference
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

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

impl<V: fmt::Display + fmt::Debug> fmt::Display for CustomTreeMap<V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Use write! to format the fields directly without the struct wrapper
        write!(f, "{:?}", self.0)
    }
}

impl<V> CustomTreeMap<V> {
    pub fn new() -> Self {
        CustomTreeMap(BTreeMap::new())
    }

    /// Inserts a key-value pair into the map.
    ///
    /// The where statement allows for string-like types
    /// (&str, String, `Cow<str>`, etc.) including `numbers` that
    /// can be turned into strings using the `ToString` trait. It
    /// makes the insertion process more flexible and easier to use.
    pub fn insert<K>(&mut self, key: K, value: V)
    where
        K: ToString,
    {
        self.0.insert(NatString::new(key.to_string()), value);
    }

    pub fn get(&self, key: &str) -> Option<&V> {
        self.0.get(&NatString::new(key.to_string()))
    }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut V> {
        self.0.get_mut(&NatString::new(key.to_string()))
    }

    pub fn remove(&mut self, key: &str) -> Option<V> {
        self.0.remove(&NatString::new(key.to_string()))
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<V> Default for CustomTreeMap<V> {
    fn default() -> Self {
        Self::new()
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

/// Formats the sum of two numbers as string.
#[pyfunction]
fn sum_as_string(a: usize, b: usize) -> PyResult<String> {
    Ok((a + b).to_string())
}

/// A Python module implemented in Rust.
#[pymodule]
fn genja_core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(sum_as_string, m)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nat_string_ordering() {
        let s1 = NatString::new("file2".to_string());
        let s2 = NatString::new("file10".to_string());
        assert!(s1 < s2);
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
    }
}
