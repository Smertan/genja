use genja_core::inventory::Host;
use genja_core::GenjaError;
use regex::Regex;
use serde_json::Value;

pub(crate) struct ValueFilter {
    key: String,
    value_regex: Regex,
}

pub(crate) struct KeyFilter {
    key: String,
}

impl KeyFilter {
    pub(crate) fn new(key: &str) -> Self {
        Self {
            key: key.to_string(),
        }
    }

    pub(crate) fn matches(&self, host: &Host) -> bool {
        let Ok(host_value) = serde_json::to_value(host) else {
            return false;
        };

        json_has_key(&host_value, &self.key)
    }
}

impl ValueFilter {
    pub(crate) fn new(key: &str, value_pattern: &str) -> Result<Self, GenjaError> {
        let value_regex = Regex::new(value_pattern)
            .map_err(|err| GenjaError::Message(format!("invalid value regex: {err}")))?;

        Ok(Self {
            key: key.to_string(),
            value_regex,
        })
    }

    pub(crate) fn matches(&self, host: &Host) -> bool {
        let Ok(host_value) = serde_json::to_value(host) else {
            return false;
        };

        json_matches_key_value(&host_value, &self.key, &self.value_regex)
    }
}

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

fn json_contains_key(value: &Value, key: &str) -> bool {
    match value {
        Value::Object(map) => map
            .iter()
            .any(|(candidate_key, candidate_value)| {
                candidate_key == key || json_contains_key(candidate_value, key)
            }),
        Value::Array(values) => values.iter().any(|value| json_contains_key(value, key)),
        _ => false,
    }
}

fn json_matches_key(value: &Value, key: &str, value_regex: &Regex) -> bool {
    match value {
        Value::Object(map) => map.iter().any(|(candidate_key, candidate_value)| {
            (candidate_key == key
                && value_regex.is_match(&json_value_match_text(candidate_value)))
                || json_matches_key(candidate_value, key, value_regex)
        }),
        Value::Array(values) => values
            .iter()
            .any(|value| json_matches_key(value, key, value_regex)),
        _ => false,
    }
}

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
        let filter =
            ValueFilter::new("site.devices.role", "edge").expect("regex should compile");

        assert!(filter.matches(&matching_host()));
    }

    #[test]
    fn does_not_match_wrong_value() {
        let filter = ValueFilter::new("role", "access").expect("regex should compile");

        assert!(!filter.matches(&matching_host()));
    }
}
