use super::InventoryConfig;
use crate::inventory::{Defaults, Groups, Hosts};
use crate::{InventoryFileKind, InventoryLoadError};
use serde::de::DeserializeOwned;

impl InventoryConfig {
    /// Loads inventory data from configured file paths.
    ///
    /// This method reads and deserializes inventory files (hosts, groups, and defaults)
    /// based on the paths specified in the `options` field. If a file path is not
    /// provided for a particular inventory component, a default or empty value is used.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a tuple of:
    /// * `Hosts` - The loaded hosts inventory. If no hosts file is specified, returns
    ///   an empty `Hosts` instance.
    /// * `Option<Groups>` - The loaded groups inventory, or `None` if no groups file
    ///   is specified.
    /// * `Option<Defaults>` - The loaded defaults inventory, or `None` if no defaults
    ///   file is specified.
    ///
    /// # Errors
    ///
    /// Returns [`InventoryLoadError`] if:
    /// * Any specified file cannot be read
    /// * Any file contains invalid JSON or YAML syntax
    /// * A file has an unsupported format (not .json, .yaml, or .yml)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use genja_core::settings::InventoryConfig;
    ///
    /// let config = InventoryConfig::default();
    /// match config.load_inventory_files() {
    ///     Ok((hosts, groups, defaults)) => {
    ///         println!("Loaded {} hosts", hosts.len());
    ///     }
    ///     Err(e) => eprintln!("Failed to load inventory: {}", e),
    /// }
    /// ```
    pub fn load_inventory_files(
        &self,
    ) -> Result<(Hosts, Option<Groups>, Option<Defaults>), InventoryLoadError> {
        let hosts = match self.options.hosts_file.as_deref() {
            Some(path) => Self::load_from_file::<Hosts>(InventoryFileKind::Hosts, path)?,
            None => Hosts::new(),
        };

        let groups = match self.options.groups_file.as_deref() {
            Some(path) => Some(Self::load_from_file::<Groups>(InventoryFileKind::Groups, path)?),
            None => None,
        };

        let defaults = match self.options.defaults_file.as_deref() {
            Some(path) => {
                Some(Self::load_from_file::<Defaults>(InventoryFileKind::Defaults, path)?)
            }
            None => None,
        };

        Ok((hosts, groups, defaults))
    }

    /// Loads and deserializes data from a file.
    ///
    /// This helper method reads a file from the filesystem and deserializes its contents
    /// based on the file extension. Supports JSON (.json) and YAML (.yaml, .yml) formats.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The type to deserialize the file contents into. Must implement `DeserializeOwned`.
    ///
    /// # Parameters
    ///
    /// * `path` - The file path to read and deserialize. The file extension determines
    ///   the deserialization format.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the deserialized data of type `T`.
    ///
    /// # Errors
    ///
    /// Returns [`InventoryLoadError`] if:
    /// * The file cannot be read (e.g., doesn't exist, permission denied)
    /// * The file contents cannot be parsed as valid JSON or YAML
    /// * The file has an unsupported extension (not .json, .yaml, or .yml)
    fn load_from_file<T>(kind: InventoryFileKind, path: &str) -> Result<T, InventoryLoadError>
    where
        T: DeserializeOwned,
    {
        let contents = std::fs::read_to_string(path).map_err(|e| InventoryLoadError::Read {
            kind,
            path: path.to_string(),
            message: e.to_string(),
        })?;

        if path.ends_with(".json") {
            serde_json::from_str(&contents).map_err(|e| InventoryLoadError::ParseJson {
                kind,
                path: path.to_string(),
                message: e.to_string(),
            })
        } else if path.ends_with(".yaml") || path.ends_with(".yml") {
            serde_yaml::from_str(&contents).map_err(|e| InventoryLoadError::ParseYaml {
                kind,
                path: path.to_string(),
                message: e.to_string(),
            })
        } else {
            Err(InventoryLoadError::UnsupportedFormat {
                kind,
                path: path.to_string(),
            })
        }
    }
}
