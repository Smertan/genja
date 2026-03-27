//! Inventory models and helpers for Genja Core.
//!
//! This module defines the in-memory inventory model (hosts, groups, defaults),
//! plus helpers for building inventories and applying optional transforms.
//!
//! **Key points**
//! - Inventory is immutable from the public API. Use builders to construct it.
//! - Hosts and groups are stored in `CustomTreeMap` keyed by name.
//! - Defaults share the same fields as groups, minus `groups` and `defaults`.
//! - Transforms are applied lazily when accessing hosts, groups, or defaults.
//! - Builder methods are consistent across Host/Group/Defaults for shared fields; defaults
//!   intentionally omit `groups`.
//!
//! # Examples
//!
//! ## Build a minimal inventory
//! ```
//! use genja_core::inventory::{Host, Hosts, Inventory, BaseBuilderHost};
//!
//! let mut hosts = Hosts::new();
//! let host = Host::builder().hostname("10.0.0.1").build();
//! hosts.add_host("router1", host);
//!
//! let inventory = Inventory::builder().hosts(hosts).build();
//! assert_eq!(inventory.hosts().len(), 1);
//! ```
//!
//! ## Defaults
//! ```
//! use genja_core::inventory::Inventory;
//!
//! let inventory = Inventory::default();
//! assert_eq!(inventory.hosts().len(), 0);
//! ```
//!
//! ## Apply a transform
//! ```
//! use genja_core::inventory::{Inventory, TransformFunction};
//!
//! let transform = TransformFunction::new(|host, _options| host.clone());
//! let inventory = Inventory::builder().transform_function(transform).build();
//! let _ = inventory.hosts();
//! ```
use crate::{CustomTreeMap, NatString};
use dashmap::DashMap;
use genja_core_derive::{DerefMacro, DerefMutMacro};
use schemars::{schema_for, JsonSchema};
use serde::de::{Error, SeqAccess, Unexpected, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex, RwLock};

pub trait BaseMethods {
    fn schema() -> String
    where
        Self: Sized,
        Self: JsonSchema,
    {
        let schema = schema_for!(Self);
        serde_json::to_string_pretty(&schema).unwrap()
    }
}

pub trait BaseBuilderHost {
    type Output;

    // Updates the hostname and returns the updated builder.
    fn hostname<S>(self, hostname: S) -> Self
    where
        S: Into<String>;

    /// Updates the port and returns the updated builder.
    fn port(self, port: u16) -> Self;

    /// Updates the username and returns the updated builder.
    fn username<S>(self, username: S) -> Self
    where
        S: Into<String>;

    /// Updates the password and returns the updated builder.
    fn password<S>(self, password: S) -> Self
    where
        S: Into<String>;

    /// Updates the platform and returns the updated builder.
    fn platform<S>(self, platform: S) -> Self
    where
        S: Into<String>;

    /// Updates the groups and returns the updated builder.
    fn groups(self, groups: ParentGroups) -> Self;

    /// Updates the data and returns the updated builder.
    fn data(self, data: Data) -> Self;

    /// Updates the connection options and returns the updated builder.
    fn connection_options<S>(self, name: S, options: ConnectionOptions) -> Self
    where
        S: Into<String>;

    /// Builds the struct from the updated builder and returns final struct object.
    fn build(self) -> Self::Output;
}

// Required for the DerefMacro derive to satisfy the DerefTarget trait.
pub trait DerefTarget {
    type Target;
}

/// Connection-specific configuration options that can override base host settings.
///
/// This struct defines optional connection parameters that can be specified per connection type
/// (e.g., "ssh", "netconf", "http") to override the base connection settings defined at the host,
/// group, or defaults level. Connection options are stored in a map keyed by connection type name
/// and are applied during connection parameter resolution.
///
/// All fields are optional, allowing partial overrides. When resolving connection parameters,
/// these options take precedence over base settings at the same hierarchy level (host, group, or defaults).
///
/// # Fields
///
/// * `hostname` - Optional hostname or IP address override for this connection type.
///   When specified, overrides the base hostname for connections of this type.
///
/// * `port` - Optional port number override for this connection type.
///   When specified, overrides the base port for connections of this type.
///
/// * `username` - Optional username override for authentication.
///   When specified, overrides the base username for connections of this type.
///
/// * `password` - Optional password override for authentication.
///   When specified, overrides the base password for connections of this type.
///
/// * `platform` - Optional platform identifier override.
///   When specified, overrides the base platform for connections of this type.
///
/// * `extras` - Optional arbitrary JSON data for connection-specific configuration.
///   Allows storing additional connection parameters that don't fit the standard fields.
///
/// # Examples
///
/// ```
/// # use genja_core::inventory::ConnectionOptions;
/// let options = ConnectionOptions::builder()
///     .port(830)
///     .username("netconf_user")
///     .build();
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema)]
pub struct ConnectionOptions {
    hostname: Option<String>,
    port: Option<u16>,
    username: Option<String>,
    password: Option<String>,
    platform: Option<String>,
    extras: Option<Extras>,
}

impl Default for ConnectionOptions {
    fn default() -> Self {
        Self::builder().build()
    }
}

impl ConnectionOptions {
    pub fn builder() -> ConnectionOptionsBuilder {
        ConnectionOptionsBuilder::new()
    }

    pub fn hostname(&self) -> Option<&str> {
        self.hostname.as_deref()
    }

    pub fn port(&self) -> Option<u16> {
        self.port
    }

    pub fn username(&self) -> Option<&str> {
        self.username.as_deref()
    }

    pub fn password(&self) -> Option<&str> {
        self.password.as_deref()
    }

    pub fn platform(&self) -> Option<&str> {
        self.platform.as_deref()
    }

    pub fn extras(&self) -> Option<&Extras> {
        self.extras.as_ref()
    }

    /// Converts this `ConnectionOptions` instance into a builder for modification.
    ///
    /// This method creates a new `ConnectionOptionsBuilder` initialized with all the current
    /// values from this `ConnectionOptions` instance. This is useful when you need to create
    /// a modified copy of existing connection options while preserving most of the original
    /// configuration.
    ///
    /// # Returns
    ///
    /// Returns a `ConnectionOptionsBuilder` with all fields initialized to match the current
    /// `ConnectionOptions` instance. The builder can then be used to modify specific fields
    /// before calling `build()` to create a new `ConnectionOptions` instance.
    ///
    /// # Examples
    ///
    /// ```
    /// # use genja_core::inventory::ConnectionOptions;
    /// let options = ConnectionOptions::builder()
    ///     .port(830)
    ///     .username("netconf_user")
    ///     .build();
    ///
    /// let modified = options.to_builder()
    ///     .port(831)
    ///     .build();
    ///
    /// assert_eq!(modified.port(), Some(831));
    /// ```
    pub fn to_builder(&self) -> ConnectionOptionsBuilder {
        ConnectionOptionsBuilder {
            hostname: self.hostname.clone(),
            port: self.port,
            username: self.username.clone(),
            password: self.password.clone(),
            platform: self.platform.clone(),
            extras: self.extras.clone(),
        }
    }
}

/// Builder for constructing `ConnectionOptions` instances.
///
/// This builder provides a fluent interface for creating connection options with optional
/// field overrides. All fields start as `None` and can be set individually before calling
/// `build()` to create the final `ConnectionOptions` instance.
///
/// The builder is typically created via `ConnectionOptions::builder()` or by converting
/// an existing `ConnectionOptions` instance using `to_builder()`.
///
/// # Fields
///
/// * `hostname` - Optional hostname or IP address override for the connection type.
///   When set, this value will override the base hostname for connections of this type.
///
/// * `port` - Optional port number override for the connection type.
///   When set, this value will override the base port for connections of this type.
///
/// * `username` - Optional username override for authentication.
///   When set, this value will override the base username for connections of this type.
///
/// * `password` - Optional password override for authentication.
///   When set, this value will override the base password for connections of this type.
///
/// * `platform` - Optional platform identifier override.
///   When set, this value will override the base platform for connections of this type.
///
/// * `extras` - Optional arbitrary JSON data for connection-specific configuration.
///   Allows storing additional connection parameters that don't fit the standard fields.
///
/// # Examples
///
/// ```
/// # use genja_core::inventory::ConnectionOptions;
/// let options = ConnectionOptions::builder()
///     .hostname("10.0.0.1")
///     .port(830)
///     .username("netconf_user")
///     .build();
///
/// assert_eq!(options.hostname(), Some("10.0.0.1"));
/// assert_eq!(options.port(), Some(830));
/// ```
pub struct ConnectionOptionsBuilder {
    hostname: Option<String>,
    port: Option<u16>,
    username: Option<String>,
    password: Option<String>,
    platform: Option<String>,
    extras: Option<Extras>,
}

impl ConnectionOptionsBuilder {
    pub fn new() -> Self {
        Self {
            hostname: None,
            port: None,
            username: None,
            password: None,
            platform: None,
            extras: None,
        }
    }

    pub fn hostname<S>(mut self, hostname: S) -> Self
    where
        S: Into<String>,
    {
        self.hostname = Some(hostname.into());
        self
    }

    pub fn port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    pub fn username<S>(mut self, username: S) -> Self
    where
        S: Into<String>,
    {
        self.username = Some(username.into());
        self
    }

    pub fn password<S>(mut self, password: S) -> Self
    where
        S: Into<String>,
    {
        self.password = Some(password.into());
        self
    }

    pub fn platform<S>(mut self, platform: S) -> Self
    where
        S: Into<String>,
    {
        self.platform = Some(platform.into());
        self
    }

    pub fn extras(mut self, extras: Extras) -> Self {
        self.extras = Some(extras);
        self
    }

    pub fn build(self) -> ConnectionOptions {
        ConnectionOptions {
            hostname: self.hostname,
            port: self.port,
            username: self.username,
            password: self.password,
            platform: self.platform,
            extras: self.extras,
        }
    }
}

/// Fully resolved connection parameters for establishing a connection to a host.
///
/// This struct represents the final, merged connection configuration after applying
/// defaults, group settings, host-specific settings, and connection-type-specific
/// overrides. It contains all the information needed to establish a connection to
/// a target host using a specific connection type (e.g., SSH, NETCONF, HTTP).
///
/// The resolution process follows a hierarchical priority order where settings at
/// higher levels (host-specific) override settings at lower levels (defaults).
/// Connection-specific options can override base settings at each hierarchy level.
///
/// # Fields
///
/// * `hostname` - The resolved hostname or IP address for the connection.
///   This field is always present and defaults to an empty string if not specified
///   anywhere in the hierarchy. It represents the target address for the connection.
///
/// * `port` - Optional port number for the connection. If `None`, the connection
///   implementation should use its default port. When specified, it indicates the
///   TCP/UDP port to use for establishing the connection.
///
/// * `username` - Optional username for authentication. If `None`, the connection
///   may use other authentication methods or fail if credentials are required.
///   When specified, it provides the username for credential-based authentication.
///
/// * `password` - Optional password for authentication. If `None`, the connection
///   may use other authentication methods (e.g., SSH keys) or fail if a password
///   is required. When specified, it provides the password for authentication.
///
/// * `platform` - Optional platform identifier (e.g., "linux", "cisco_ios", "junos").
///   This helps connection implementations apply platform-specific behavior, command
///   syntax, or protocol variations. If `None`, the connection uses generic behavior.
///
/// * `extras` - Optional arbitrary JSON data for additional connection-specific
///   configuration. This allows passing custom parameters that don't fit the standard
///   fields, such as timeout values, retry settings, or protocol-specific options.
///
/// # Examples
///
/// ```
/// # use genja_core::inventory::ResolvedConnectionParams;
/// let params = ResolvedConnectionParams {
///     hostname: "10.0.0.1".to_string(),
///     port: Some(830),
///     username: Some("admin".to_string()),
///     password: Some("secret".to_string()),
///     platform: Some("junos".to_string()),
///     extras: None,
/// };
///
/// assert_eq!(params.hostname, "10.0.0.1");
/// assert_eq!(params.port, Some(830));
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedConnectionParams {
    pub hostname: String,
    pub port: Option<u16>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub platform: Option<String>,
    pub extras: Option<Extras>,
}

impl DerefTarget for Extras {
    type Target = serde_json::Value;
}

/// The DataExtra struct is a wrapper for serde_json::Value, any json data is accepted.
#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema, DerefMacro, DerefMutMacro,
)]
pub struct Extras(serde_json::Value);

impl Extras {
    pub fn new(value: serde_json::Value) -> Self {
        Extras(value)
    }
}

impl DerefTarget for ParentGroups {
    type Target = Vec<String>;
}

/// The ParentGroups struct is a wrapped vector of strings.
///
/// It stores a list of strings representing the groups the host
/// belongs to.
///
/// The ParentGroups struct implements Deref and DerefMut for easy
/// access to the underlying vector.
#[derive(Debug, Clone, Serialize, PartialEq, JsonSchema, DerefMacro, DerefMutMacro)]
pub struct ParentGroups(Vec<String>);

impl Default for ParentGroups {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentGroups {
    pub fn new() -> Self {
        ParentGroups(Vec::new())
    }
}

impl<'de> Deserialize<'de> for ParentGroups {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        match deserializer.deserialize_seq(ParentGroupsVisitor) {
            Ok(parent) => Ok(parent),
            Err(err) => {
                log::error!("{}", err);
                let err_msg = "Groups should be an array of strings for use with `ParentGroups`";
                log::error!("{err_msg}");
                Err(D::Error::custom(err_msg))
            }
        }
    }
}

struct ParentGroupsVisitor;

impl<'de> Visitor<'de> for ParentGroupsVisitor {
    type Value = ParentGroups;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a sequence of strings")
    }
    fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Err(Error::invalid_value(Unexpected::Str(s), &self))
    }

    /// This method is used to handle custom deserialization logic for
    /// sequences. It returns a list of unique strings from the sequence.
    ///
    /// The vector implementation ensures that duplicate strings are not added to the
    /// and preserves the order of the first occurrence of each string.
    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut groups = Vec::new();
        while let Some(value) = seq.next_element()? {
            if !groups.contains(&value) {
                groups.push(value);
            }
        }

        Ok(ParentGroups(groups.into_iter().collect()))
    }
}

/// Defaults configuration for inventory.
///
/// Schema: same fields as `Group`, minus `groups` and `defaults`.
/// This allows defaults to define connection details and data that apply broadly
/// without nesting or self-references.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct Defaults {
    pub(crate) hostname: Option<String>,
    pub(crate) port: Option<u16>,
    pub(crate) username: Option<String>,
    pub(crate) password: Option<String>,
    pub(crate) platform: Option<String>,
    pub(crate) data: Option<Data>,
    pub(crate) connection_options: Option<CustomTreeMap<ConnectionOptions>>,
}

impl DerefTarget for Data {
    type Target = serde_json::Value;
}

impl Defaults {
    pub fn builder() -> DefaultsBuilder {
        DefaultsBuilder::new()
    }

    /// Converts this `Defaults` instance into a builder for modification.
    ///
    /// This method creates a new `DefaultsBuilder` initialized with all the current
    /// values from this `Defaults` instance. This is useful when you need to create
    /// a modified copy of existing defaults while preserving most of the original
    /// configuration.
    ///
    /// # Returns
    ///
    /// Returns a `DefaultsBuilder` with all fields initialized to match the current
    /// `Defaults` instance. The builder can then be used to modify specific fields
    /// before calling `build()` to create a new `Defaults` instance.
    ///
    /// # Examples
    ///
    /// ```
    /// # use genja_core::inventory::Defaults;
    /// let defaults = Defaults::builder()
    ///     .username("admin")
    ///     .port(22)
    ///     .build();
    ///
    /// let modified = defaults.to_builder()
    ///     .port(2222)
    ///     .build();
    ///
    /// assert_eq!(modified.port(), Some(2222));
    /// assert_eq!(modified.username(), Some("admin"));
    /// ```
    pub fn to_builder(&self) -> DefaultsBuilder {
        let mut builder = Defaults::builder();
        if let Some(hostname) = self.hostname.as_deref() {
            builder = builder.hostname(hostname);
        }
        if let Some(port) = self.port {
            builder = builder.port(port);
        }
        if let Some(username) = self.username.as_deref() {
            builder = builder.username(username);
        }
        if let Some(password) = self.password.as_deref() {
            builder = builder.password(password);
        }
        if let Some(platform) = self.platform.as_deref() {
            builder = builder.platform(platform);
        }
        if let Some(data) = self.data.as_ref() {
            builder = builder.data(data.clone());
        }
        if let Some(options_map) = self.connection_options.as_ref() {
            for (name, options) in options_map.iter() {
                builder = builder.connection_options(name.to_string(), options.clone());
            }
        }
        builder
    }

    pub fn new() -> Self {
        Defaults {
            hostname: None,
            port: None,
            username: None,
            password: None,
            platform: None,
            data: None,
            connection_options: None,
        }
    }
    /// Returns true if all fields are None or empty
    pub fn is_empty(&self) -> bool {
        self.hostname.is_none()
            && self.port.is_none()
            && self.username.is_none()
            && self.password.is_none()
            && self.platform.is_none()
            && self.data.is_none()
            && self.connection_options.is_none()
    }

    pub fn hostname(&self) -> Option<&str> {
        self.hostname.as_deref()
    }

    pub fn port(&self) -> Option<u16> {
        self.port
    }

    pub fn username(&self) -> Option<&str> {
        self.username.as_deref()
    }

    pub fn password(&self) -> Option<&str> {
        self.password.as_deref()
    }

    pub fn platform(&self) -> Option<&str> {
        self.platform.as_deref()
    }

    pub fn data(&self) -> Option<&Data> {
        self.data.as_ref()
    }

    pub fn connection_options(&self) -> Option<&CustomTreeMap<ConnectionOptions>> {
        self.connection_options.as_ref()
    }
}

/// Builder for constructing `Defaults` instances.
///
/// This builder provides a fluent interface for creating inventory defaults with optional
/// configuration fields. All fields start as `None` and can be set individually using the
/// builder methods before calling `build()` to create the final `Defaults` instance.
///
/// Defaults define base configuration values that apply to all hosts and groups in the
/// inventory unless overridden at the group or host level. This allows for centralized
/// management of common connection parameters and data.
///
/// Unlike `Host` and `Group`, defaults do not support `groups` membership.
///
/// # Fields
///
/// * `hostname` - Optional default hostname or IP address. Applied to hosts/groups that
///   don't specify their own hostname.
///
/// * `port` - Optional default port number for connections. Applied to hosts/groups that
///   don't specify their own port.
///
/// * `username` - Optional default username for authentication. Applied to hosts/groups
///   that don't specify their own username.
///
/// * `password` - Optional default password for authentication. Applied to hosts/groups
///   that don't specify their own password.
///
/// * `platform` - Optional default platform identifier (e.g., "linux", "cisco_ios").
///   Applied to hosts/groups that don't specify their own platform.
///
/// * `data` - Optional arbitrary JSON data that applies to all hosts/groups by default.
///   Can be overridden or merged at the group or host level.
///
/// * `connection_options` - Optional map of connection-specific overrides keyed by
///   connection type. Allows per-connection-type customization of default parameters.
///
/// # Examples
///
/// ```
/// # use genja_core::inventory::Defaults;
/// let defaults = Defaults::builder()
///     .username("admin")
///     .port(22)
///     .platform("linux")
///     .build();
///
/// assert_eq!(defaults.username(), Some("admin"));
/// assert_eq!(defaults.port(), Some(22));
/// ```
pub struct DefaultsBuilder {
    hostname: Option<String>,
    port: Option<u16>,
    username: Option<String>,
    password: Option<String>,
    platform: Option<String>,
    data: Option<Data>,
    connection_options: Option<CustomTreeMap<ConnectionOptions>>,
}

impl DefaultsBuilder {
    pub fn new() -> Self {
        Self {
            hostname: None,
            port: None,
            username: None,
            password: None,
            platform: None,
            data: None,
            connection_options: None,
        }
    }

    pub fn hostname<S>(mut self, hostname: S) -> Self
    where
        S: Into<String>,
    {
        self.hostname = Some(hostname.into());
        self
    }

    pub fn port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    pub fn username<S>(mut self, username: S) -> Self
    where
        S: Into<String>,
    {
        self.username = Some(username.into());
        self
    }

    pub fn password<S>(mut self, password: S) -> Self
    where
        S: Into<String>,
    {
        self.password = Some(password.into());
        self
    }

    pub fn platform<S>(mut self, platform: S) -> Self
    where
        S: Into<String>,
    {
        self.platform = Some(platform.into());
        self
    }

    pub fn data(mut self, data: Data) -> Self {
        self.data = Some(data);
        self
    }

    /// Adds or updates connection-specific options for defaults.
    ///
    /// # Parameters
    ///
    /// * `name` - A string-like value identifying the connection type (e.g., "ssh", "netconf").
    /// * `options` - A `ConnectionOptions` instance containing connection-specific configuration.
    ///
    /// # Returns
    ///
    /// Returns `Self` with the connection options updated, allowing for method chaining.
    /// If no connection options map exists, one is created before inserting the new options.
    pub fn connection_options<S>(mut self, name: S, options: ConnectionOptions) -> Self
    where
        S: Into<String>,
    {
        if self.connection_options.is_none() {
            self.connection_options = Some(CustomTreeMap::new());
        }
        self.connection_options
            .as_mut()
            .unwrap()
            .insert(name.into(), options);
        self
    }


    pub fn build(self) -> Defaults {
        Defaults {
            hostname: self.hostname,
            port: self.port,
            username: self.username,
            password: self.password,
            platform: self.platform,
            data: self.data,
            connection_options: self.connection_options,
        }
    }
}

impl Default for Defaults {
    fn default() -> Self {
        Self::new()
    }
}
/// The Data struct is a wrapper for serde_json::Value, any json data is accepted.
#[derive(
    Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, DerefMacro, DerefMutMacro,
)]
pub struct Data(serde_json::Value);

impl Data {
    pub fn new(data: serde_json::Value) -> Self {
        Data(data)
    }
}

/// Represents a single host in the inventory with connection parameters and metadata.
///
/// A `Host` defines the configuration for connecting to and managing a single network device
/// or server. It contains optional connection parameters (hostname, port, credentials, platform),
/// group membership information, arbitrary data, and connection-specific overrides.
///
/// Hosts are the fundamental unit of the inventory system. They can inherit configuration from
/// groups and defaults through the inventory hierarchy, with host-level settings taking highest
/// precedence during parameter resolution.
///
/// # Fields
///
/// * `hostname` - Optional hostname or IP address for the host. This is the primary identifier
///   used for network connections. If not specified, it may be inherited from groups or defaults.
///
/// * `port` - Optional port number for connections. If not specified, defaults may be applied
///   during connection parameter resolution or connection implementations may use their default ports.
///
/// * `username` - Optional username for authentication. Used for establishing connections to
///   the host. Can be inherited from groups or defaults if not specified.
///
/// * `password` - Optional password for authentication. Used in conjunction with username for
///   connection authentication. Can be inherited from groups or defaults if not specified.
///
/// * `platform` - Optional platform identifier (e.g., "linux", "cisco_ios", "junos"). Used to
///   determine platform-specific behavior and connection handling. Can be inherited from groups
///   or defaults if not specified.
///
/// * `groups` - Optional parent group names that this host belongs to. Groups provide inherited
///   configuration through the inventory hierarchy. Multiple groups can be specified, and their
///   configurations are merged in order.
///
/// * `data` - Optional arbitrary JSON data associated with the host. Allows storing custom
///   metadata and configuration that doesn't fit standard fields. Can be merged with group
///   and default data during resolution.
///
/// * `connection_options` - Optional map of connection-specific overrides keyed by connection
///   type (e.g., "ssh", "netconf", "http"). Allows per-connection-type customization of
///   connection parameters, overriding base host settings for specific connection types.
///
/// # Deserialization
///
/// - Unknown fields are rejected via `#[serde(deny_unknown_fields)]` to catch configuration errors
/// - All fields are optional, allowing minimal host definitions
/// - Connection options accept arbitrary map keys for different connection types
///
/// # Examples
///
/// ```
/// # use genja_core::inventory::{Host, BaseBuilderHost};
/// let host = Host::builder()
///     .hostname("10.0.0.1")
///     .port(22)
///     .username("admin")
///     .platform("linux")
///     .build();
///
/// assert_eq!(host.hostname(), Some("10.0.0.1"));
/// assert_eq!(host.port(), Some(22));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Host {
    pub(crate) hostname: Option<String>,
    pub(crate) port: Option<u16>,
    pub(crate) username: Option<String>,
    pub(crate) password: Option<String>,
    pub(crate) platform: Option<String>,
    pub(crate) groups: Option<ParentGroups>,
    pub(crate) data: Option<Data>,
    pub(crate) connection_options: Option<CustomTreeMap<ConnectionOptions>>,
}
impl Host {
    pub fn new() -> Host {
        Host {
            hostname: None,
            port: None,
            username: None,
            password: None,
            platform: None,
            groups: None,
            data: None,
            connection_options: None,
        }
    }
    pub fn builder() -> HostBuilder {
        HostBuilder::new()
    }

    /// Converts this `Host` instance into a builder for modification.
    ///
    /// This method creates a new `HostBuilder` initialized with all the current
    /// values from this `Host` instance. This is useful when you need to create
    /// a modified copy of an existing host while preserving most of the original
    /// configuration.
    ///
    /// # Returns
    ///
    /// Returns a `HostBuilder` with all fields initialized to match the current
    /// `Host` instance. The builder can then be used to modify specific fields
    /// before calling `build()` to create a new `Host` instance.
    ///
    /// # Examples
    ///
    /// ```
    /// # use genja_core::inventory::{Host, BaseBuilderHost};
    /// let host = Host::builder()
    ///     .hostname("10.0.0.1")
    ///     .port(22)
    ///     .username("admin")
    ///     .build();
    ///
    /// let modified = host.to_builder()
    ///     .port(2222)
    ///     .build();
    ///
    /// assert_eq!(modified.hostname(), Some("10.0.0.1"));
    /// assert_eq!(modified.port(), Some(2222));
    /// assert_eq!(modified.username(), Some("admin"));
    /// ```
    pub fn to_builder(&self) -> HostBuilder {
        let mut builder = Host::builder();
        if let Some(hostname) = self.hostname() {
            builder = builder.hostname(hostname);
        }
        if let Some(port) = self.port() {
            builder = builder.port(port);
        }
        if let Some(username) = self.username() {
            builder = builder.username(username);
        }
        if let Some(password) = self.password() {
            builder = builder.password(password);
        }
        if let Some(platform) = self.platform() {
            builder = builder.platform(platform);
        }
        if let Some(groups) = self.groups() {
            builder = builder.groups(groups.clone());
        }
        if let Some(data) = self.data() {
            builder = builder.data(data.clone());
        }
        if let Some(options_map) = self.connection_options() {
            for (name, options) in options_map.iter() {
                builder = builder.connection_options(name.to_string(), options.clone());
            }
        }
        builder
    }

    pub fn hostname(&self) -> Option<&str> {
        self.hostname.as_deref()
    }

    pub fn port(&self) -> Option<u16> {
        self.port
    }

    pub fn username(&self) -> Option<&str> {
        self.username.as_deref()
    }

    pub fn password(&self) -> Option<&str> {
        self.password.as_deref()
    }

    pub fn platform(&self) -> Option<&str> {
        self.platform.as_deref()
    }

    pub fn groups(&self) -> Option<&ParentGroups> {
        self.groups.as_ref()
    }

    pub fn data(&self) -> Option<&Data> {
        self.data.as_ref()
    }

    pub fn connection_options(&self) -> Option<&CustomTreeMap<ConnectionOptions>> {
        self.connection_options.as_ref()
    }

    /// Resolves connection parameters for a specific connection type by merging host-level
    /// settings with connection-specific overrides.
    ///
    /// This method uses only the fields on this `Host`. It does not apply defaults or group
    /// inheritance. To include those, use `Inventory::resolve_connection_params` (see the second
    /// example below).
    ///
    /// This method creates a complete set of connection parameters by starting with the host's
    /// base connection fields (hostname, port, username, password, platform) and then applying
    /// any connection-specific overrides from the `connection_options` map. Connection-specific
    /// options take precedence over base host fields.
    ///
    /// # Parameters
    ///
    /// * `connection_type` - A string identifying the connection type to resolve parameters for
    ///   (e.g., "ssh", "netconf", "http"). This is used as the key to lookup connection-specific
    ///   options in the host's `connection_options` map.
    ///
    /// # Returns
    ///
    /// Returns a `ResolvedConnectionParams` struct containing the fully resolved connection
    /// parameters. If the host has connection-specific options for the given `connection_type`,
    /// those values override the corresponding base host fields. Fields not specified in either
    /// location will be `None` (except hostname, which defaults to an empty string if not set).
    ///
    /// # Examples
    ///
    /// ```
    /// # use genja_core::inventory::{Host, ConnectionOptions, BaseBuilderHost};
    /// let options = ConnectionOptions::builder().port(830).build();
    ///
    /// let host = Host::builder()
    ///     .hostname("10.0.0.1")
    ///     .port(22)
    ///     .connection_options("netconf", options)
    ///     .build();
    ///
    /// let params = host.resolve_connection_params("netconf");
    /// assert_eq!(params.hostname, "10.0.0.1");
    /// assert_eq!(params.port, Some(830)); // Connection-specific port overrides base port
    /// ```
    ///
    /// The following example shows how to resolve parameters through `Inventory`,
    /// which applies defaults and group inheritance before connection-specific overrides.
    ///
    /// ```
    /// # use genja_core::inventory::{Host, Hosts, Inventory, ConnectionOptions, BaseBuilderHost};
    /// let mut hosts = Hosts::new();
    /// let options = ConnectionOptions::builder().port(830).build();
    /// let host = Host::builder()
    ///     .hostname("10.0.0.1")
    ///     .port(22)
    ///     .connection_options("netconf", options)
    ///     .build();
    /// hosts.add_host("router1", host);
    /// let inventory = Inventory::builder().hosts(hosts).build();
    ///
    /// let params = inventory
    ///     .resolve_connection_params("router1", "netconf")
    ///     .expect("resolved params");
    /// assert_eq!(params.port, Some(830));
    /// ```
    pub fn resolve_connection_params(&self, connection_type: &str) -> ResolvedConnectionParams {
        let mut resolved = ResolvedConnectionParams {
            hostname: self.hostname.clone().unwrap_or_default(),
            port: self.port,
            username: self.username.clone(),
            password: self.password.clone(),
            platform: self.platform.clone(),
            extras: None,
        };

        if let Some(options_map) = &self.connection_options {
            if let Some(options) = options_map.get(connection_type) {
                if let Some(hostname) = options.hostname.clone() {
                    resolved.hostname = hostname;
                }
                if options.port.is_some() {
                    resolved.port = options.port;
                }
                if options.username.is_some() {
                    resolved.username = options.username.clone();
                }
                if options.password.is_some() {
                    resolved.password = options.password.clone();
                }
                if options.platform.is_some() {
                    resolved.platform = options.platform.clone();
                }
                if options.extras.is_some() {
                    resolved.extras = options.extras.clone();
                }
            }
        }

        resolved
    }
}

impl BaseMethods for Host {}

/// Builder for constructing `Host` instances.
///
/// This builder provides a fluent interface for creating hosts with optional configuration
/// fields. All fields start as `None` and can be set individually using the builder methods
/// before calling `build()` to create the final `Host` instance.
///
/// The builder implements the `BaseBuilderHost` trait, which provides standard methods for
/// setting connection parameters, group membership, and custom data. This allows for a
/// consistent interface across different inventory entity builders.
///
/// # Fields
///
/// * `hostname` - Optional hostname or IP address for the host. This is the primary identifier
///   used for network connections.
///
/// * `port` - Optional port number for connections. If not specified, defaults may be applied
///   during connection parameter resolution.
///
/// * `username` - Optional username for authentication. Used for establishing connections to
///   the host.
///
/// * `password` - Optional password for authentication. Used in conjunction with username for
///   connection authentication.
///
/// * `platform` - Optional platform identifier (e.g., "linux", "cisco_ios"). Used to determine
///   platform-specific behavior and connection handling.
///
/// * `groups` - Optional parent group names that this host belongs to. Groups provide inherited
///   configuration through the inventory hierarchy.
///
/// * `data` - Optional arbitrary JSON data associated with the host. Allows storing custom
///   metadata and configuration that doesn't fit standard fields.
///
/// * `connection_options` - Optional map of connection-specific overrides keyed by connection
///   type. Allows per-connection-type customization of connection parameters.
///
/// # Examples
///
/// ```
/// # use genja_core::inventory::{Host, BaseBuilderHost};
/// let host = Host::builder()
///     .hostname("10.0.0.1")
///     .port(22)
///     .username("admin")
///     .platform("linux")
///     .build();
///
/// assert_eq!(host.hostname(), Some("10.0.0.1"));
/// assert_eq!(host.port(), Some(22));
/// ```
pub struct HostBuilder {
    hostname: Option<String>,
    port: Option<u16>,
    username: Option<String>,
    password: Option<String>,
    platform: Option<String>,
    groups: Option<ParentGroups>,
    data: Option<Data>,
    connection_options: Option<CustomTreeMap<ConnectionOptions>>,
}

impl HostBuilder {
    pub fn new() -> Self {
        HostBuilder {
            hostname: None,
            port: None,
            username: None,
            password: None,
            platform: None,
            groups: None,
            data: None,
            connection_options: None,
        }
    }
}

impl BaseBuilderHost for HostBuilder {
    type Output = Host;

    fn hostname<S>(mut self, hostname: S) -> Self
    where
        S: Into<String>,
    {
        self.hostname = Some(hostname.into());
        self
    }

    fn port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    fn username<S>(mut self, username: S) -> Self
    where
        S: Into<String>,
    {
        self.username = Some(username.into());
        self
    }

    fn password<S>(mut self, password: S) -> Self
    where
        S: Into<String>,
    {
        self.password = Some(password.into());
        self
    }

    fn platform<S>(mut self, platform: S) -> Self
    where
        S: Into<String>,
    {
        self.platform = Some(platform.into());
        self
    }

    fn groups(mut self, groups: ParentGroups) -> Self {
        self.groups = Some(groups);
        self
    }

    fn data(mut self, data: Data) -> Self {
        self.data = Some(data);
        self
    }

    fn connection_options<S>(mut self, name: S, options: ConnectionOptions) -> Self
    where
        S: Into<String>,
    {
        if self.connection_options.is_none() {
            self.connection_options = Some(CustomTreeMap::new());
        }
        self.connection_options
            .as_mut()
            .unwrap()
            .insert(name.into(), options);
        self
    }

    fn build(self) -> Host {
        Host {
            hostname: self.hostname,
            port: self.port,
            username: self.username,
            password: self.password,
            platform: self.platform,
            groups: self.groups,
            data: self.data,
            connection_options: self.connection_options,
        }
    }
}

/// Group-level inventory entry that applies values to member hosts.
///
/// # Fields
///
/// Group fields mirror host fields and are merged during resolution.
/// Groups are stored in the `Groups` collection keyed by name. Use
/// `Groups::add_group(name, group)` to add a group entry under a name.
///
/// * `hostname` - Optional hostname or address applied to member hosts.
/// * `port` - Optional connection port applied to member hosts.
/// * `username` - Optional username applied to member hosts.
/// * `password` - Optional password applied to member hosts.
/// * `platform` - Optional platform identifier applied to member hosts.
/// * `groups` - Optional parent group names for group inheritance.
/// * `data` - Optional arbitrary data merged into member hosts.
/// * `connection_options` - Optional per-connection overrides.
/// * Defaults are applied globally via `Inventory`.
///
/// # Deserialization
///
/// - Unknown fields are rejected (via `#[serde(deny_unknown_fields)]`).
/// - Connection options accept arbitrary map keys.
///
/// # Examples
///
/// ```
/// use genja_core::inventory::{Group, Groups, BaseBuilderHost};
///
/// let mut groups = Groups::new();
/// let core_group = Group::builder()
///     .platform("linux")
///     .build();
///
/// groups.add_group("core", core_group);
/// assert_eq!(groups.len(), 1);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Group {
    pub(crate) hostname: Option<String>,
    pub(crate) port: Option<u16>,
    pub(crate) username: Option<String>,
    pub(crate) password: Option<String>,
    pub(crate) platform: Option<String>,
    pub(crate) groups: Option<ParentGroups>,
    pub(crate) data: Option<Data>,
    pub(crate) connection_options: Option<CustomTreeMap<ConnectionOptions>>,
}

impl Group {
    /// Returns a builder for creating group entries.
    ///
    /// Use the builder to set optional fields before calling `build()`.
    pub fn builder() -> GroupBuilder {
        GroupBuilder::new()
    }

    pub fn to_builder(&self) -> GroupBuilder {
        let mut builder = Group::builder();
        if let Some(hostname) = self.hostname() {
            builder = builder.hostname(hostname);
        }
        if let Some(port) = self.port() {
            builder = builder.port(port);
        }
        if let Some(username) = self.username() {
            builder = builder.username(username);
        }
        if let Some(password) = self.password() {
            builder = builder.password(password);
        }
        if let Some(platform) = self.platform() {
            builder = builder.platform(platform);
        }
        if let Some(groups) = self.groups() {
            builder = builder.groups(groups.clone());
        }
        if let Some(data) = self.data() {
            builder = builder.data(data.clone());
        }
        if let Some(options_map) = self.connection_options() {
            for (name, options) in options_map.iter() {
                builder = builder.connection_options(name.to_string(), options.clone());
            }
        }
        builder
    }

    pub fn hostname(&self) -> Option<&str> {
        self.hostname.as_deref()
    }

    pub fn port(&self) -> Option<u16> {
        self.port
    }

    pub fn username(&self) -> Option<&str> {
        self.username.as_deref()
    }

    pub fn password(&self) -> Option<&str> {
        self.password.as_deref()
    }

    pub fn platform(&self) -> Option<&str> {
        self.platform.as_deref()
    }

    pub fn groups(&self) -> Option<&ParentGroups> {
        self.groups.as_ref()
    }

    pub fn data(&self) -> Option<&Data> {
        self.data.as_ref()
    }

    pub fn connection_options(&self) -> Option<&CustomTreeMap<ConnectionOptions>> {
        self.connection_options.as_ref()
    }
}

/// Builder for constructing `Group` entries.
///
/// Use the `BaseBuilderHost` methods to populate optional fields, then call `build()`.
pub struct GroupBuilder {
    hostname: Option<String>,
    port: Option<u16>,
    username: Option<String>,
    password: Option<String>,
    platform: Option<String>,
    groups: Option<ParentGroups>,
    data: Option<Data>,
    connection_options: Option<CustomTreeMap<ConnectionOptions>>,
}

impl BaseBuilderHost for GroupBuilder {
    type Output = Group;

    /// Sets the hostname for the group.
    ///
    /// # Parameters
    ///
    /// * `hostname` - A string-like value containing the hostname or IP address to assign to the group.
    ///
    /// # Returns
    ///
    /// Returns `Self` with the hostname field updated, allowing for method chaining.
    fn hostname<S>(mut self, hostname: S) -> Self
    where
        S: Into<String>,
    {
        self.hostname = Some(hostname.into());
        self
    }

    /// Sets the connection port for the group.
    ///
    /// # Parameters
    ///
    /// * `port` - A 16-bit unsigned integer representing the port number to use for connections.
    ///
    /// # Returns
    ///
    /// Returns `Self` with the port field updated, allowing for method chaining.
    fn port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    /// Sets the username for authentication.
    ///
    /// # Parameters
    ///
    /// * `username` - A string-like value containing the username to use for authentication.
    ///
    /// # Returns
    ///
    /// Returns `Self` with the username field updated, allowing for method chaining.
    fn username<S>(mut self, username: S) -> Self
    where
        S: Into<String>,
    {
        self.username = Some(username.into());
        self
    }

    /// Sets the password for authentication.
    ///
    /// # Parameters
    ///
    /// * `password` - A string-like value containing the password to use for authentication.
    ///
    /// # Returns
    ///
    /// Returns `Self` with the password field updated, allowing for method chaining.
    fn password<S>(mut self, password: S) -> Self
    where
        S: Into<String>,
    {
        self.password = Some(password.into());
        self
    }

    /// Sets the platform identifier for the group.
    ///
    /// # Parameters
    ///
    /// * `platform` - A string-like value identifying the platform type (e.g., "linux", "windows", "cisco_ios").
    ///
    /// # Returns
    ///
    /// Returns `Self` with the platform field updated, allowing for method chaining.
    fn platform<S>(mut self, platform: S) -> Self
    where
        S: Into<String>,
    {
        self.platform = Some(platform.into());
        self
    }

    /// Sets the parent groups for this group.
    ///
    /// # Parameters
    ///
    /// * `groups` - A `ParentGroups` instance containing the names of parent groups this group belongs to.
    ///
    /// # Returns
    ///
    /// Returns `Self` with the groups field updated, allowing for method chaining.
    fn groups(mut self, groups: ParentGroups) -> Self {
        self.groups = Some(groups);
        self
    }

    /// Sets arbitrary data for the group.
    ///
    /// # Parameters
    ///
    /// * `data` - A `Data` instance containing arbitrary JSON data to associate with the group.
    ///
    /// # Returns
    ///
    /// Returns `Self` with the data field updated, allowing for method chaining.
    fn data(mut self, data: Data) -> Self {
        self.data = Some(data);
        self
    }

    /// Adds or updates connection-specific options for the group.
    ///
    /// # Parameters
    ///
    /// * `name` - A string-like value identifying the connection type (e.g., "ssh", "netconf").
    /// * `options` - A `ConnectionOptions` instance containing connection-specific configuration.
    ///
    /// # Returns
    ///
    /// Returns `Self` with the connection options updated, allowing for method chaining.
    /// If no connection options map exists, one is created before inserting the new options.
    fn connection_options<S>(mut self, name: S, options: ConnectionOptions) -> Self
    where
        S: Into<String>,
    {
        if self.connection_options.is_none() {
            self.connection_options = Some(CustomTreeMap::new());
        }
        self.connection_options
            .as_mut()
            .unwrap()
            .insert(name.into(), options);
        self
    }

    fn build(self) -> Group {
        Group {
            hostname: self.hostname,
            port: self.port,
            username: self.username,
            password: self.password,
            platform: self.platform,
            groups: self.groups,
            data: self.data,
            connection_options: self.connection_options,
        }
    }
}

impl GroupBuilder {
    pub fn new() -> Self {
        GroupBuilder {
            hostname: None,
            port: None,
            username: None,
            password: None,
            platform: None,
            groups: None,
            data: None,
            connection_options: None,
        }
    }
}

/// Internal storage type for `Hosts` (maps host name -> `Host`).
pub type HostsTarget = CustomTreeMap<Host>;

impl DerefTarget for Hosts {
    type Target = CustomTreeMap<Host>;
}

/// Collection of hosts keyed by name.
///
/// This type wraps a `CustomTreeMap<Host>` and is the primary container used
/// for host inventory data. The map keys are host names used for logging/output.
///
/// # Deserialization
///
/// - Unknown fields in individual `Host` entries are rejected (via `#[serde(deny_unknown_fields)]` on `Host`)
/// - The `Hosts` wrapper itself accepts any valid map structure
///
/// # Examples
///
/// ```
/// use genja_core::inventory::{Host, Hosts, BaseBuilderHost};
///
/// let mut hosts = Hosts::new();
/// let host = Host::builder().hostname("10.0.0.1").build();
/// hosts.add_host("router1", host);
/// assert_eq!(hosts.len(), 1);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, DerefMacro, DerefMutMacro)]
pub struct Hosts(HostsTarget);

impl Default for Hosts {
    fn default() -> Self {
        Self::new()
    }
}

impl Hosts {
    /// Creates an empty host collection.
    ///
    /// Use `add_host` or map insertion methods to populate it.
    pub fn new() -> Self {
        Hosts(CustomTreeMap::new())
    }

    /// Inserts a host into the collection under the provided name.
    ///
    /// If a host with the same name already exists, it will be replaced with the new host.
    /// The name serves as the unique identifier for the host and is used in logs and output.
    ///
    /// # Parameters
    ///
    /// * `name` - A string-like value that will be used as the unique identifier for the host.
    ///   This name is used in logs and output to reference the host.
    /// * `host` - The `Host` instance to insert into the collection.
    ///
    /// # Examples
    ///
    /// ```
    /// use genja_core::inventory::{Host, Hosts, BaseBuilderHost};
    ///
    /// let mut hosts = Hosts::new();
    /// let host = Host::builder().hostname("10.0.0.1").build();
    /// hosts.add_host("router1", host);
    /// assert_eq!(hosts.len(), 1);
    /// ```
    pub fn add_host<N>(&mut self, name: N, host: Host)
    where
        N: Into<String>,
    {
        self.insert(name.into(), host);
    }
}

impl BaseMethods for Hosts {}

/// Collection of groups keyed by name.
///
/// This type wraps a `CustomTreeMap<Group>` and is the primary container used
/// for group inventory data. The map keys are group names.
///
/// # Deserialization
///
/// - Unknown fields in individual `Group` entries are rejected (via `#[serde(deny_unknown_fields)]` on `Group`)
/// - The `Groups` wrapper itself accepts any valid map structure
///
/// # Examples
///
/// ```
/// use genja_core::inventory::{Group, Groups, BaseBuilderHost};
///
/// let mut groups = Groups::new();
/// let core_group = Group::builder().platform("linux").build();
/// groups.add_group("core", core_group);
/// assert_eq!(groups.len(), 1);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, DerefMacro, DerefMutMacro)]
pub struct Groups(CustomTreeMap<Group>);

impl DerefTarget for Groups {
    type Target = CustomTreeMap<Group>;
}

impl Groups {
    /// Creates an empty group collection.
    ///
    /// Use `add_group` or map insertion methods to populate it.
    pub fn new() -> Self {
        Groups(CustomTreeMap::new())
    }

    /// Inserts a group into the collection under the provided name.
    ///
    /// If a group with the same name already exists, it will be replaced.
    pub fn add_group<N>(&mut self, name: N, group: Group)
    where
        N: Into<String>,
    {
        self.insert(name.into(), group);
    }
}

impl Default for Groups {
    fn default() -> Self {
        Self::new()
    }
}

/// A trait for implementing custom transformation logic on inventory entities.
///
/// The `Transform` trait provides a flexible mechanism to modify hosts, groups, and defaults
/// in an inventory based on custom logic, external configuration, or runtime conditions.
/// Implementations of this trait can be wrapped in a `TransformFunction` and applied to an
/// inventory to dynamically alter entity properties without modifying the underlying data.
///
/// All methods in this trait have default implementations that return clones of the input
/// entities unchanged. Implementors only need to override the methods for entity types they
/// wish to transform.
///
/// # Thread Safety
///
/// Implementations must be `Send + Sync` to allow safe sharing across threads. The inventory
/// system uses `Arc` internally to share transform functions, so all transform logic must be
/// thread-safe.
///
/// # Transform Methods
///
/// The trait provides three transformation methods, one for each inventory entity type:
///
/// * `transform_host` - Transforms individual host configurations
/// * `transform_group` - Transforms group configurations
/// * `transform_defaults` - Transforms inventory-wide defaults
///
/// Each method receives a reference to the entity being transformed and optional configuration
/// through `TransformFunctionOptions`. The methods should return a new instance of the entity
/// with the desired modifications applied.
///
/// # When Transforms Are Applied
///
/// Transforms are applied lazily when accessing inventory entities:
/// - When calling `Inventory::hosts()` and accessing individual hosts
/// - When calling `Inventory::groups()` and accessing individual groups
/// - When calling `Inventory::defaults()`
/// - During host resolution via `Inventory::resolve_host()`
///
/// Results are cached to improve performance on subsequent accesses.
///
/// # Transform Options
///
/// The optional `TransformFunctionOptions` parameter provides a way to pass configuration
/// data to the transform function at runtime. This allows for flexible, data-driven
/// transformations without hardcoding values in the transform implementation.
///
/// Options are stored as JSON values and can contain any structured data needed by the
/// transform logic. Access the options using the `get()` method and JSON value accessors.
///
/// # Examples
///
/// ## Basic Host Transform
///
/// ```
/// # use genja_core::inventory::{Transform, TransformFunction, TransformFunctionOptions};
/// # use genja_core::inventory::{Host, Group, Defaults, BaseBuilderHost};
/// struct PortTransform {
///     default_port: u16,
/// }
///
/// impl Transform for PortTransform {
///     fn transform_host(&self, host: &Host, _options: Option<&TransformFunctionOptions>) -> Host {
///         // Apply default port if host doesn't have one
///         if host.port().is_none() {
///             host.to_builder()
///                 .port(self.default_port)
///                 .build()
///         } else {
///             host.clone()
///         }
///     }
/// }
///
/// let transform = TransformFunction::new_full(PortTransform { default_port: 2222 });
/// ```
///
/// ## Transform Using Options
///
/// ```
/// # use genja_core::inventory::{Transform, TransformFunction, TransformFunctionOptions};
/// # use genja_core::inventory::{Host, Group, Defaults, BaseBuilderHost};
/// struct PrefixTransform;
///
/// impl Transform for PrefixTransform {
///     fn transform_host(&self, host: &Host, options: Option<&TransformFunctionOptions>) -> Host {
///         // Get prefix from options
///         let prefix = options
///             .and_then(|opts| opts.get("hostname_prefix"))
///             .and_then(|v| v.as_str())
///             .unwrap_or("");
///
///         if !prefix.is_empty() {
///             if let Some(hostname) = host.hostname() {
///                 return host.to_builder()
///                     .hostname(format!("{}{}", prefix, hostname))
///                     .build();
///             }
///         }
///         host.clone()
///     }
/// }
///
/// let transform = TransformFunction::new_full(PrefixTransform);
/// let options = TransformFunctionOptions::new(
///     serde_json::json!({"hostname_prefix": "prod-"})
/// );
/// ```
///
/// ## Multi-Entity Transform
///
/// ```
/// # use genja_core::inventory::{Transform, TransformFunction, TransformFunctionOptions};
/// # use genja_core::inventory::{Host, Group, Defaults, BaseBuilderHost};
/// struct EnvironmentTransform {
///     environment: String,
/// }
///
/// impl Transform for EnvironmentTransform {
///     fn transform_host(&self, host: &Host, _options: Option<&TransformFunctionOptions>) -> Host {
///         // Add environment tag to hostname
///         if let Some(hostname) = host.hostname() {
///             host.to_builder()
///                 .hostname(format!("{}.{}", hostname, self.environment))
///                 .build()
///         } else {
///             host.clone()
///         }
///     }
///
///     fn transform_group(&self, group: &Group, _options: Option<&TransformFunctionOptions>) -> Group {
///         // Apply environment-specific group settings
///         if self.environment == "prod" {
///             // Production groups might need different settings
///             group.to_builder()
///                 .port(443)
///                 .build()
///         } else {
///             group.clone()
///         }
///     }
///
///     fn transform_defaults(&self, defaults: &Defaults, _options: Option<&TransformFunctionOptions>) -> Defaults {
///         // Apply environment-specific defaults
///         defaults.to_builder()
///             .username(format!("{}-user", self.environment))
///             .build()
///     }
/// }
///
/// let transform = TransformFunction::new_full(EnvironmentTransform {
///     environment: "prod".to_string(),
/// });
/// ```
///
/// ## IP Address Mapping Transform
///
/// ```
/// # use genja_core::inventory::{Transform, TransformFunction, TransformFunctionOptions};
/// # use genja_core::inventory::{Host, Group, Defaults, BaseBuilderHost};
/// struct IpMappingTransform;
///
/// impl Transform for IpMappingTransform {
///     fn transform_host(&self, host: &Host, options: Option<&TransformFunctionOptions>) -> Host {
///         // Get IP mapping from options
///         let mapping = options
///             .and_then(|opts| opts.get("ip_map"))
///             .and_then(|v| v.as_object());
///
///         let Some(mapping) = mapping else {
///             return host.clone();
///         };
///
///         let mut builder = host.to_builder();
///
///         // Map hostname if it exists in the mapping
///         if let Some(hostname) = host.hostname() {
///             if let Some(mapped) = mapping.get(hostname).and_then(|v| v.as_str()) {
///                 builder = builder.hostname(mapped);
///             }
///         }
///
///         builder.build()
///     }
/// }
///
/// let transform = TransformFunction::new_full(IpMappingTransform);
/// let options = TransformFunctionOptions::new(serde_json::json!({
///     "ip_map": {
///         "10-0-0-1": "10.0.0.1",
///         "10-0-0-2": "10.0.0.2"
///     }
/// }));
/// ```
pub trait Transform: Send + Sync {
    /// Transforms a host entity.
    ///
    /// This method is called when a host is accessed through the inventory's host view
    /// or during host resolution. The default implementation returns a clone of the
    /// input host unchanged.
    ///
    /// # Parameters
    ///
    /// * `host` - A reference to the host being transformed
    /// * `_options` - Optional configuration data for the transform
    ///
    /// # Returns
    ///
    /// Returns a new `Host` instance with the desired transformations applied.
    fn transform_host(&self, host: &Host, _options: Option<&TransformFunctionOptions>) -> Host {
        host.clone()
    }

    /// Transforms a group entity.
    ///
    /// This method is called when a group is accessed through the inventory's group view.
    /// The default implementation returns a clone of the input group unchanged.
    ///
    /// # Parameters
    ///
    /// * `group` - A reference to the group being transformed
    /// * `_options` - Optional configuration data for the transform
    ///
    /// # Returns
    ///
    /// Returns a new `Group` instance with the desired transformations applied.
    fn transform_group(&self, group: &Group, _options: Option<&TransformFunctionOptions>) -> Group {
        group.clone()
    }

    /// Transforms the inventory defaults.
    ///
    /// This method is called when defaults are accessed through `Inventory::defaults()`.
    /// The default implementation returns a clone of the input defaults unchanged.
    ///
    /// # Parameters
    ///
    /// * `defaults` - A reference to the defaults being transformed
    /// * `_options` - Optional configuration data for the transform
    ///
    /// # Returns
    ///
    /// Returns a new `Defaults` instance with the desired transformations applied.
    fn transform_defaults(
        &self,
        defaults: &Defaults,
        _options: Option<&TransformFunctionOptions>,
    ) -> Defaults {
        defaults.clone()
    }
}

/// A thread-safe wrapper around a transform function that can modify inventory entities.
///
/// `TransformFunction` encapsulates custom logic for dynamically transforming hosts, groups,
/// and defaults in an inventory. It provides a flexible mechanism to modify inventory data
/// based on runtime conditions, external configuration, or custom business logic without
/// altering the underlying inventory structure.
///
/// The wrapper uses `Arc` for thread-safe reference counting, enabling the transform function
/// to be shared across multiple threads and cloned efficiently. All clones share the same
/// underlying transform logic.
///
/// # Transform Types
///
/// There are two ways to create a `TransformFunction`:
///
/// 1. **Host-only transform** - Using `new()`, which accepts a closure that only transforms hosts.
///    Groups and defaults pass through unchanged.
///
/// 2. **Full transform** - Using `new_full()`, which accepts a type implementing the `Transform`
///    trait, allowing custom transformation of hosts, groups, and defaults.
///
/// # When Transforms Are Applied
///
/// Transforms are applied lazily when accessing inventory entities through:
/// - `Inventory::hosts()` - Returns a `HostsView` that applies transforms on access
/// - `Inventory::groups()` - Returns a `GroupsView` that applies transforms on access  
/// - `Inventory::defaults()` - Returns transformed defaults
/// - `Inventory::resolve_host()` - Applies transforms to the resolved host
///
/// Results are cached to improve performance on subsequent accesses.
///
/// # Thread Safety
///
/// The `Clone` implementation creates a new reference to the same underlying transform
/// function, not a deep copy. All clones share the same transform logic and can be safely
/// used across threads.
///
/// # Examples
///
/// ## Host-only Transform
///
/// ```
/// # use genja_core::inventory::{TransformFunction, Host, Inventory, Hosts, BaseBuilderHost};
/// // Create a transform that modifies the port for all hosts
/// let transform = TransformFunction::new(|host, _options| {
///     host.to_builder()
///         .port(2222)
///         .build()
/// });
///
/// let mut hosts = Hosts::new();
/// hosts.add_host("router1", Host::builder().hostname("10.0.0.1").port(22).build());
///
/// let inventory = Inventory::builder()
///     .hosts(hosts)
///     .transform_function(transform)
///     .build();
///
/// // Transform is applied when accessing the host
/// let host = inventory.hosts().get("router1").unwrap();
/// assert_eq!(host.port(), Some(2222));
/// ```
///
/// ## Full Transform with Options
///
/// ```
/// # use genja_core::inventory::{Transform, TransformFunction, TransformFunctionOptions};
/// # use genja_core::inventory::{Host, Group, Defaults, Inventory, Hosts, BaseBuilderHost};
/// struct CustomTransform;
///
/// impl Transform for CustomTransform {
///     fn transform_host(&self, host: &Host, options: Option<&TransformFunctionOptions>) -> Host {
///         // Access transform options if provided
///         if let Some(opts) = options {
///             if let Some(prefix) = opts.get("hostname_prefix").and_then(|v| v.as_str()) {
///                 if let Some(hostname) = host.hostname() {
///                     return host.to_builder()
///                         .hostname(format!("{}{}", prefix, hostname))
///                         .build();
///                 }
///             }
///         }
///         host.clone()
///     }
///
///     fn transform_group(&self, group: &Group, _options: Option<&TransformFunctionOptions>) -> Group {
///         // Custom group transformation logic
///         group.clone()
///     }
/// }
///
/// let transform = TransformFunction::new_full(CustomTransform);
/// let options = TransformFunctionOptions::new(
///     serde_json::json!({"hostname_prefix": "prod-"})
/// );
///
/// let mut hosts = Hosts::new();
/// hosts.add_host("router1", Host::builder().hostname("router1").build());
///
/// let inventory = Inventory::builder()
///     .hosts(hosts)
///     .transform_function(transform)
///     .transform_function_options(options)
///     .build();
///
/// let host = inventory.hosts().get("router1").unwrap();
/// assert_eq!(host.hostname(), Some("prod-router1"));
/// ```
///
/// ## Cloning and Sharing
///
/// ```
/// # use genja_core::inventory::{TransformFunction, Host};
/// let transform = TransformFunction::new(|host: &Host, _| host.clone());
///
/// // Cloning creates a new reference to the same transform
/// let transform_clone = transform.clone();
///
/// // Both can be used independently and share the same underlying logic
/// ```
#[derive(Clone)]
pub struct TransformFunction(Arc<dyn Transform>);

impl TransformFunction {
    /// Creates a new transform function that only modifies hosts.
    ///
    /// This is a convenience constructor for the common case where you only need to transform
    /// hosts. Groups and defaults will pass through unchanged. The provided closure receives
    /// a reference to the host and optional transform options, and should return a new `Host`
    /// instance with the desired modifications.
    ///
    /// # Type Parameters
    ///
    /// * `F` - A closure type that takes `(&Host, Option<&TransformFunctionOptions>)` and
    ///   returns a `Host`. The closure must be `Send + Sync + 'static` to allow thread-safe
    ///   sharing across the inventory.
    ///
    /// # Parameters
    ///
    /// * `func` - A closure that implements the host transformation logic. It receives:
    ///   - `&Host` - A reference to the host being transformed
    ///   - `Option<&TransformFunctionOptions>` - Optional configuration data for the transform
    ///
    /// # Returns
    ///
    /// Returns a new `TransformFunction` that applies the provided closure to hosts while
    /// leaving groups and defaults unchanged.
    ///
    /// # Examples
    ///
    /// ```
    /// # use genja_core::inventory::{TransformFunction, Host, BaseBuilderHost};
    /// // Simple transform that sets a default port
    /// let transform = TransformFunction::new(|host, _options| {
    ///     if host.port().is_none() {
    ///         host.to_builder().port(22).build()
    ///     } else {
    ///         host.clone()
    ///     }
    /// });
    /// ```
    ///
    /// ```
    /// # use genja_core::inventory::{TransformFunction, Host, BaseBuilderHost};
    /// // Transform using options
    /// let transform = TransformFunction::new(|host, options| {
    ///     if let Some(opts) = options {
    ///         if let Some(default_port) = opts.get("default_port").and_then(|v| v.as_u64()) {
    ///             if host.port().is_none() {
    ///                 return host.to_builder().port(default_port as u16).build();
    ///             }
    ///         }
    ///     }
    ///     host.clone()
    /// });
    /// ```
    pub fn new<F>(func: F) -> Self
    where
        F: Fn(&Host, Option<&TransformFunctionOptions>) -> Host + Send + Sync + 'static,
    {
        struct HostOnlyTransform<F> {
            func: F,
        }

        impl<F> Transform for HostOnlyTransform<F>
        where
            F: Fn(&Host, Option<&TransformFunctionOptions>) -> Host + Send + Sync,
        {
            fn transform_host(
                &self,
                host: &Host,
                options: Option<&TransformFunctionOptions>,
            ) -> Host {
                (self.func)(host, options)
            }
        }

        TransformFunction(Arc::new(HostOnlyTransform { func }))
    }

    /// Creates a new transform function from a type implementing the `Transform` trait.
    ///
    /// This constructor allows for full control over transformation of hosts, groups, and
    /// defaults. Use this when you need to implement custom transformation logic for all
    /// inventory entity types, or when you need to maintain state across transformations.
    ///
    /// # Type Parameters
    ///
    /// * `T` - A type implementing the `Transform` trait. The type must be `'static` to
    ///   allow it to be stored in the `Arc` wrapper.
    ///
    /// # Parameters
    ///
    /// * `transform` - An instance of a type implementing `Transform`. The instance will
    ///   be wrapped in an `Arc` for thread-safe sharing.
    ///
    /// # Returns
    ///
    /// Returns a new `TransformFunction` that applies the provided `Transform` implementation
    /// to hosts, groups, and defaults.
    ///
    /// # Examples
    ///
    /// ```
    /// # use genja_core::inventory::{Transform, TransformFunction, TransformFunctionOptions};
    /// # use genja_core::inventory::{Host, Group, Defaults, BaseBuilderHost};
    /// struct EnvironmentTransform {
    ///     environment: String,
    /// }
    ///
    /// impl Transform for EnvironmentTransform {
    ///     fn transform_host(&self, host: &Host, _options: Option<&TransformFunctionOptions>) -> Host {
    ///         // Prefix hostname with environment
    ///         if let Some(hostname) = host.hostname() {
    ///             host.to_builder()
    ///                 .hostname(format!("{}-{}", self.environment, hostname))
    ///                 .build()
    ///         } else {
    ///             host.clone()
    ///         }
    ///     }
    ///
    ///     fn transform_group(&self, group: &Group, _options: Option<&TransformFunctionOptions>) -> Group {
    ///         // Apply environment-specific group modifications
    ///         group.clone()
    ///     }
    ///
    ///     fn transform_defaults(&self, defaults: &Defaults, _options: Option<&TransformFunctionOptions>) -> Defaults {
    ///         // Apply environment-specific defaults
    ///         defaults.clone()
    ///     }
    /// }
    ///
    /// let transform = TransformFunction::new_full(EnvironmentTransform {
    ///     environment: "prod".to_string(),
    /// });
    /// ```
    pub fn new_full<T>(transform: T) -> Self
    where
        T: Transform + 'static,
    {
        TransformFunction(Arc::new(transform))
    }
    /// Applies the transform function to a host.
    ///
    /// This method delegates to the underlying `Transform` implementation to modify
    /// the provided host according to the transform logic. It's primarily used internally
    /// by the inventory system when accessing hosts through views or during resolution.
    ///
    /// # Parameters
    ///
    /// * `host` - A reference to the host to transform
    /// * `options` - Optional configuration data to pass to the transform function
    ///
    /// # Returns
    ///
    /// Returns a new `Host` instance with transformations applied. If no transform
    /// logic is defined for hosts, returns a clone of the input host.
    ///
    /// # Examples
    ///
    /// ```
    /// # use genja_core::inventory::{TransformFunction, Host, BaseBuilderHost};
    /// let transform = TransformFunction::new(|host, _| {
    ///     host.to_builder().port(2222).build()
    /// });
    ///
    /// let host = Host::builder().hostname("10.0.0.1").build();
    /// let transformed = transform.transform_host(&host, None);
    /// assert_eq!(transformed.port(), Some(2222));
    /// ```
    pub fn transform_host(&self, host: &Host, options: Option<&TransformFunctionOptions>) -> Host {
        self.0.transform_host(host, options)
    }

    /// Applies the transform function to a group.
    ///
    /// This method delegates to the underlying `Transform` implementation to modify
    /// the provided group according to the transform logic. It's primarily used internally
    /// by the inventory system when accessing groups through views.
    ///
    /// # Parameters
    ///
    /// * `group` - A reference to the group to transform
    /// * `options` - Optional configuration data to pass to the transform function
    ///
    /// # Returns
    ///
    /// Returns a new `Group` instance with transformations applied. If no transform
    /// logic is defined for groups, returns a clone of the input group.
    ///
    /// # Examples
    ///
    /// ```
    /// # use genja_core::inventory::{Transform, TransformFunction, TransformFunctionOptions, Group, BaseBuilderHost};
    /// struct GroupTransform;
    /// impl Transform for GroupTransform {
    ///     fn transform_group(&self, group: &Group, _options: Option<&TransformFunctionOptions>) -> Group {
    ///         group.to_builder().port(443).build()
    ///     }
    /// }
    ///
    /// let transform = TransformFunction::new_full(GroupTransform);
    /// let group = Group::builder().platform("linux").build();
    /// let transformed = transform.transform_group(&group, None);
    /// assert_eq!(transformed.port(), Some(443));
    /// ```
    pub fn transform_group(
        &self,
        group: &Group,
        options: Option<&TransformFunctionOptions>,
    ) -> Group {
        self.0.transform_group(group, options)
    }

    /// Applies the transform function to inventory defaults.
    ///
    /// This method delegates to the underlying `Transform` implementation to modify
    /// the provided defaults according to the transform logic. It's primarily used internally
    /// by the inventory system when accessing defaults through `Inventory::defaults()`.
    ///
    /// # Parameters
    ///
    /// * `defaults` - A reference to the defaults to transform
    /// * `options` - Optional configuration data to pass to the transform function
    ///
    /// # Returns
    ///
    /// Returns a new `Defaults` instance with transformations applied. If no transform
    /// logic is defined for defaults, returns a clone of the input defaults.
    ///
    /// # Examples
    ///
    /// ```
    /// # use genja_core::inventory::{Transform, TransformFunction, TransformFunctionOptions, Defaults, BaseBuilderHost};
    /// struct DefaultsTransform;
    /// impl Transform for DefaultsTransform {
    ///     fn transform_defaults(&self, defaults: &Defaults, _options: Option<&TransformFunctionOptions>) -> Defaults {
    ///         defaults.to_builder().username("admin").build()
    ///     }
    /// }
    ///
    /// let transform = TransformFunction::new_full(DefaultsTransform);
    /// let defaults = Defaults::builder().port(22).build();
    /// let transformed = transform.transform_defaults(&defaults, None);
    /// assert_eq!(transformed.username(), Some("admin"));
    /// ```
    pub fn transform_defaults(
        &self,
        defaults: &Defaults,
        options: Option<&TransformFunctionOptions>,
    ) -> Defaults {
        self.0.transform_defaults(defaults, options)
    }
}

impl fmt::Debug for TransformFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TransformFunction({:p})", Arc::as_ptr(&self.0))
    }
}

/// The TransformFunctionOptions struct is a wrapper for serde_json::Value, any json data is accepted.
/// Configuration options passed to transform functions when processing inventory entities.
///
/// `TransformFunctionOptions` is a wrapper around a JSON value that provides flexible,
/// schema-free configuration data for transform functions. It allows passing arbitrary
/// structured data to transforms without requiring predefined types or schemas.
///
/// The wrapper implements `Deref` and `DerefMut` to provide direct access to the underlying
/// `serde_json::Value`, enabling use of all JSON value methods for accessing and manipulating
/// the configuration data.
///
/// # Usage in Transforms
///
/// Transform functions receive an `Option<&TransformFunctionOptions>` parameter that can be
/// used to access configuration data. The options are typically set on the `Inventory` using
/// `InventoryBuilder::transform_function_options()`.
///
/// # JSON Structure
///
/// The underlying JSON value can be any valid JSON structure:
/// - Object: `{"key": "value", "nested": {"data": 123}}`
/// - Array: `["item1", "item2"]`
/// - Primitive: `"string"`, `42`, `true`, `null`
///
/// # Examples
///
/// ## Creating Options
///
/// ```
/// # use genja_core::inventory::TransformFunctionOptions;
/// // Simple key-value options
/// let options = TransformFunctionOptions::new(serde_json::json!({
///     "default_port": 2222,
///     "environment": "production"
/// }));
///
/// // Nested configuration
/// let options = TransformFunctionOptions::new(serde_json::json!({
///     "ssh": {
///         "port": 22,
///         "timeout": 30
///     },
///     "netconf": {
///         "port": 830,
///         "timeout": 60
///     }
/// }));
/// ```
///
/// ## Accessing Options in Transforms
///
/// ```
/// # use genja_core::inventory::{Transform, TransformFunctionOptions, Host, Group, Defaults, BaseBuilderHost};
/// struct PortTransform;
///
/// impl Transform for PortTransform {
///     fn transform_host(&self, host: &Host, options: Option<&TransformFunctionOptions>) -> Host {
///         // Access options using JSON value methods
///         if let Some(opts) = options {
///             if let Some(port) = opts.get("default_port").and_then(|v| v.as_u64()) {
///                 if host.port().is_none() {
///                     return host.to_builder().port(port as u16).build();
///                 }
///             }
///         }
///         host.clone()
///     }
/// }
/// ```
///
/// ## Using with Inventory
///
/// ```
/// # use genja_core::inventory::{Inventory, TransformFunction, TransformFunctionOptions, Host, Hosts, BaseBuilderHost};
/// let transform = TransformFunction::new(|host, options| {
///     if let Some(opts) = options {
///         if let Some(prefix) = opts.get("hostname_prefix").and_then(|v| v.as_str()) {
///             if let Some(hostname) = host.hostname() {
///                 return host.to_builder()
///                     .hostname(format!("{}{}", prefix, hostname))
///                     .build();
///             }
///         }
///     }
///     host.clone()
/// });
///
/// let options = TransformFunctionOptions::new(serde_json::json!({
///     "hostname_prefix": "prod-"
/// }));
///
/// let mut hosts = Hosts::new();
/// hosts.add_host("router1", Host::builder().hostname("router1").build());
///
/// let inventory = Inventory::builder()
///     .hosts(hosts)
///     .transform_function(transform)
///     .transform_function_options(options)
///     .build();
/// ```
#[derive(
    Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, DerefMacro, DerefMutMacro,
)]
pub struct TransformFunctionOptions(serde_json::Value);

impl DerefTarget for TransformFunctionOptions {
    type Target = serde_json::Value;
}

impl TransformFunctionOptions {
    /// Creates a new `TransformFunctionOptions` instance from a JSON value.
    ///
    /// This constructor wraps any valid JSON value in a `TransformFunctionOptions` struct,
    /// providing a flexible way to pass configuration data to transform functions. The JSON
    /// value can be of any type: object, array, string, number, boolean, or null.
    ///
    /// The options are typically accessed within transform function implementations using
    /// the `Deref` trait to access the underlying `serde_json::Value` methods like `get()`,
    /// `as_str()`, `as_object()`, etc.
    ///
    /// # Parameters
    ///
    /// * `options` - A `serde_json::Value` containing the configuration data to be passed
    ///   to transform functions. This can be any valid JSON structure created using the
    ///   `serde_json::json!` macro or parsed from JSON text.
    ///
    /// # Returns
    ///
    /// Returns a new `TransformFunctionOptions` instance wrapping the provided JSON value.
    ///
    /// # Examples
    ///
    /// ```
    /// # use genja_core::inventory::TransformFunctionOptions;
    /// // Create options with an object
    /// let options = TransformFunctionOptions::new(serde_json::json!({
    ///     "default_port": 2222,
    ///     "environment": "production"
    /// }));
    ///
    /// // Create options with an array
    /// let options = TransformFunctionOptions::new(serde_json::json!(["item1", "item2"]));
    ///
    /// // Create options with a primitive value
    /// let options = TransformFunctionOptions::new(serde_json::json!("simple_string"));
    /// ```
    pub fn new(options: serde_json::Value) -> Self {
        TransformFunctionOptions(options)
    }
}

pub trait Connection
where
    Self: Send + Sync + fmt::Debug,
{
    fn create(&self, key: &ConnectionKey) -> Box<dyn Connection>;

    fn is_alive(&self) -> bool;

    fn open(&mut self, params: &ResolvedConnectionParams) -> Result<(), String>;

    fn close(&mut self) -> ConnectionKey;
}

/// A unique identifier for a connection in the connection manager.
///
/// `ConnectionKey` serves as a composite key for looking up and managing connections
/// in the `ConnectionManager`. It combines a hostname with a connection type to uniquely
/// identify a specific connection instance. This allows the same host to have multiple
/// concurrent connections of different types (e.g., SSH, NETCONF, HTTP).
///
/// The struct implements `Hash` and `Eq` to enable its use as a key in hash-based
/// collections like `HashMap` and `DashMap`.
///
/// # Hash Function Behavior
///
/// When inserting a `ConnectionKey` into a hash-based collection (like `DashMap` in
/// `ConnectionManager`), the hash function is used to:
///
/// 1. **Compute Hash Value**: Both `hostname` and `connection_type` fields are hashed
///    together to produce a single hash value. This is done automatically by Rust's
///    derive macro for `Hash`, which hashes each field in declaration order.
///
/// 2. **Determine Bucket**: The hash value is used to determine which internal bucket
///    in the hash map should store this key-value pair. This enables O(1) average-case
///    lookup performance.
///
/// 3. **Handle Collisions**: If two different keys produce the same hash value (a hash
///    collision), the `Eq` implementation is used to distinguish between them. The
///    collection stores multiple entries in the same bucket and uses `Eq` to find the
///    exact match.
///
/// 4. **Enable Deduplication**: When inserting with the same `hostname` and
///    `connection_type`, the hash function ensures the key maps to the same bucket,
///    and `Eq` confirms it's the same key, allowing the collection to update the
///    existing entry rather than creating a duplicate.
///
/// # Fields
///
/// * `hostname` - The hostname or IP address of the target device. This identifies
///   the remote endpoint for the connection.
/// * `connection_type` - The type of connection protocol (e.g., "ssh", "netconf", "http").
///   This distinguishes between different connection types to the same host.
///
/// # Examples
///
/// ## Basic Usage
///
/// ```
/// # use genja_core::inventory::ConnectionKey;
/// let key = ConnectionKey::new("10.0.0.1", "ssh");
/// assert_eq!(key.hostname, "10.0.0.1");
/// assert_eq!(key.connection_type, "ssh");
/// ```
///
/// ## Multiple Connection Types per Host
///
/// ```
/// # use genja_core::inventory::ConnectionKey;
/// use std::collections::HashMap;
///
/// let mut connections = HashMap::new();
/// let ssh_key = ConnectionKey::new("router1", "ssh");
/// let netconf_key = ConnectionKey::new("router1", "netconf");
///
/// // Same host can have different connection types
/// // Each key produces a different hash due to different connection_type
/// connections.insert(ssh_key, "SSH connection");
/// connections.insert(netconf_key, "NETCONF connection");
/// assert_eq!(connections.len(), 2);
/// ```
///
/// ## Key Equality and Deduplication
///
/// ```
/// # use genja_core::inventory::ConnectionKey;
/// use std::collections::HashMap;
///
/// let mut connections = HashMap::new();
/// let key1 = ConnectionKey::new("router1", "ssh");
/// let key2 = ConnectionKey::new("router1", "ssh");
///
/// // Both keys have the same hostname and connection_type
/// // They produce the same hash and are equal via Eq
/// connections.insert(key1, "First connection");
/// connections.insert(key2, "Second connection"); // Replaces first
/// assert_eq!(connections.len(), 1);
/// assert_eq!(connections.values().next(), Some(&"Second connection"));
/// ```
///
/// ## Hash-Based Lookup in ConnectionManager
///
/// ```
/// # use genja_core::inventory::{ConnectionKey, ConnectionManager};
/// let manager = ConnectionManager::default();
/// let key = ConnectionKey::new("router1", "ssh");
///
/// // The hash function enables fast lookup:
/// // 1. Hash is computed from key
/// // 2. Hash determines which bucket to search
/// // 3. Eq is used to find exact match in bucket
/// if let Some(connection) = manager.get(&key) {
///     println!("Found existing connection");
/// }
/// ```
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct ConnectionKey {
    pub hostname: String,
    pub connection_type: String,
}

impl ConnectionKey {
    /// Creates a new `ConnectionKey` from a hostname and connection type.
    ///
    /// This constructor provides a convenient way to create a connection key by accepting
    /// any type that can be converted into a `String` for both the hostname and connection
    /// type parameters. This allows passing `&str`, `String`, or other string-like types
    /// without explicit conversion.
    ///
    /// The resulting key uniquely identifies a connection in the `ConnectionManager` by
    /// combining the target hostname with the connection protocol type.
    ///
    /// # Parameters
    ///
    /// * `hostname` - The hostname or IP address of the target device. Accepts any type
    ///   implementing `Into<String>`, such as `&str` or `String`. This identifies the
    ///   remote endpoint for the connection.
    /// * `connection_type` - The type of connection protocol (e.g., "ssh", "netconf", "http").
    ///   Accepts any type implementing `Into<String>`. This distinguishes between different
    ///   connection types to the same host.
    ///
    /// # Returns
    ///
    /// Returns a new `ConnectionKey` instance with the provided hostname and connection type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use genja_core::inventory::ConnectionKey;
    /// // Using string slices
    /// let key1 = ConnectionKey::new("10.0.0.1", "ssh");
    ///
    /// // Using owned strings
    /// let hostname = String::from("router1");
    /// let conn_type = String::from("netconf");
    /// let key2 = ConnectionKey::new(hostname, conn_type);
    ///
    /// // Mixed types
    /// let key3 = ConnectionKey::new("10.0.0.2", String::from("http"));
    /// ```
    pub fn new(hostname: impl Into<String>, connection_type: impl Into<String>) -> Self {
        Self {
            hostname: hostname.into(),
            connection_type: connection_type.into(),
        }
    }
}

pub type ConnectionFactory =
    dyn Fn(&ConnectionKey) -> Option<Arc<Mutex<dyn Connection>>> + Send + Sync;

/// Statistics tracking connection lifecycle operations per connection type.
///
/// `ConnectionCounters` provides a simple counter-based mechanism for monitoring connection
/// operations in the `ConnectionManager`. Each connection type (e.g., "ssh", "netconf", "http")
/// has its own set of counters that track how many times connections of that type have been
/// created, opened, and closed.
///
/// These counters are useful for:
/// - **Performance Monitoring**: Identify connection pool efficiency and reuse patterns
/// - **Debugging**: Detect connection leaks, excessive creation, or improper cleanup
/// - **Testing**: Verify connection lifecycle behavior in unit and integration tests
/// - **Metrics**: Export connection statistics for observability systems
///
/// # Counter Semantics
///
/// * `create_calls` - Incremented when a new connection instance is created by the factory.
///   This happens on the first call to `get_or_create()` for a unique `ConnectionKey`.
///   Multiple calls with the same key do not increment this counter.
///
/// * `open_calls` - Incremented when `open()` is called on a connection. This happens when
///   `open_connection()` is called and the connection's `is_alive()` returns `false`.
///   Calling `open_connection()` on an already-alive connection does not increment this counter.
///
/// * `close_calls` - Incremented when a connection is closed via `close_connection()` or
///   `close_all_connections()`. Each connection is counted only once when it's removed from
///   the pool.
///
/// # Thread Safety
///
/// The counters are stored in a `DashMap<String, ConnectionCounters>` in the `ConnectionManager`,
/// providing thread-safe concurrent access. Multiple threads can increment counters for different
/// connection types simultaneously without blocking each other.
///
/// # Usage Patterns
///
/// ## Ideal Pattern (Efficient Connection Reuse)
/// ```text
/// create_calls: 1
/// open_calls:   1
/// close_calls:  1
/// ```
/// This indicates a connection was created once, opened once, and properly cleaned up.
/// Multiple operations reused the same connection without reopening it.
///
/// ## Connection Leak Pattern
/// ```text
/// create_calls: 5
/// open_calls:   5
/// close_calls:  0
/// ```
/// This indicates connections are being created but never closed, suggesting a resource leak.
///
/// ## Excessive Recreation Pattern
/// ```text
/// create_calls: 100
/// open_calls:   100
/// close_calls:  100
/// ```
/// This indicates connections are being created and destroyed repeatedly instead of being
/// reused, suggesting inefficient connection pooling.
///
/// # Examples
///
/// ## Monitoring Connection Usage
///
/// ```
/// # use std::sync::{Arc, Mutex};
/// # use genja_core::inventory::{Connection, ConnectionKey, ConnectionManager, ResolvedConnectionParams};
/// # #[derive(Debug)]
/// # struct SshConnection { alive: bool }
/// # impl Connection for SshConnection {
/// #     fn create(&self, _key: &ConnectionKey) -> Box<dyn Connection> {
/// #         Box::new(SshConnection { alive: false })
/// #     }
/// #     fn is_alive(&self) -> bool { self.alive }
/// #     fn open(&mut self, _params: &ResolvedConnectionParams) -> Result<(), String> {
/// #         self.alive = true; Ok(())
/// #     }
/// #     fn close(&mut self) -> ConnectionKey {
/// #         self.alive = false;
/// #         ConnectionKey::new("router1", "ssh")
/// #     }
/// # }
/// # let factory = Arc::new(|_key: &ConnectionKey| {
/// #     Some(Arc::new(Mutex::new(SshConnection { alive: false })) as Arc<Mutex<dyn Connection>>)
/// # });
/// let manager = ConnectionManager::with_connection_factory(factory);
/// let key = ConnectionKey::new("router1", "ssh");
/// let params = ResolvedConnectionParams {
///     hostname: "10.0.0.1".to_string(),
///     port: Some(22),
///     username: Some("admin".to_string()),
///     password: None,
///     platform: None,
///     extras: None,
/// };
///
/// // Perform operations
/// manager.open_connection(&key, &params)?;
/// manager.open_connection(&key, &params)?; // Reuses existing connection
/// manager.close_connection(&key);
///
/// // Check counters
/// let counters = manager.connection_counters_for("ssh").unwrap();
/// assert_eq!(counters.create_calls, 1); // Created once
/// assert_eq!(counters.open_calls, 1);   // Opened once (second call reused)
/// assert_eq!(counters.close_calls, 1);  // Closed once
/// # Ok::<(), String>(())
/// ```
///
/// ## Detecting Connection Leaks in Tests
///
/// ```
/// # use std::sync::{Arc, Mutex};
/// # use genja_core::inventory::{Connection, ConnectionKey, ConnectionManager, ResolvedConnectionParams};
/// # #[derive(Debug)]
/// # struct SshConnection { alive: bool }
/// # impl Connection for SshConnection {
/// #     fn create(&self, _key: &ConnectionKey) -> Box<dyn Connection> {
/// #         Box::new(SshConnection { alive: false })
/// #     }
/// #     fn is_alive(&self) -> bool { self.alive }
/// #     fn open(&mut self, _params: &ResolvedConnectionParams) -> Result<(), String> {
/// #         self.alive = true; Ok(())
/// #     }
/// #     fn close(&mut self) -> ConnectionKey {
/// #         self.alive = false;
/// #         ConnectionKey::new("router1", "ssh")
/// #     }
/// # }
/// # let factory = Arc::new(|_key: &ConnectionKey| {
/// #     Some(Arc::new(Mutex::new(SshConnection { alive: false })) as Arc<Mutex<dyn Connection>>)
/// # });
/// let manager = ConnectionManager::with_connection_factory(factory);
/// let params = ResolvedConnectionParams {
///     hostname: "10.0.0.1".to_string(),
///     port: Some(22),
///     username: Some("admin".to_string()),
///     password: None,
///     platform: None,
///     extras: None,
/// };
///
/// // Open multiple connections
/// for i in 1..=5 {
///     let key = ConnectionKey::new(format!("router{}", i), "ssh");
///     manager.open_connection(&key, &params)?;
/// }
///
/// // Verify all connections were created
/// let counters = manager.connection_counters_for("ssh").unwrap();
/// assert_eq!(counters.create_calls, 5);
/// assert_eq!(counters.open_calls, 5);
///
/// // Clean up and verify no leaks
/// manager.close_all_connections();
/// let counters = manager.connection_counters_for("ssh").unwrap();
/// assert_eq!(counters.close_calls, 5); // All connections closed
/// # Ok::<(), String>(())
/// ```
///
/// ## Comparing Multiple Connection Types
///
/// ```
/// # use std::sync::{Arc, Mutex};
/// # use genja_core::inventory::{Connection, ConnectionKey, ConnectionManager, ResolvedConnectionParams};
/// # #[derive(Debug)]
/// # struct TestConnection { conn_type: String, alive: bool }
/// # impl Connection for TestConnection {
/// #     fn create(&self, key: &ConnectionKey) -> Box<dyn Connection> {
/// #         Box::new(TestConnection { conn_type: key.connection_type.clone(), alive: false })
/// #     }
/// #     fn is_alive(&self) -> bool { self.alive }
/// #     fn open(&mut self, _params: &ResolvedConnectionParams) -> Result<(), String> {
/// #         self.alive = true; Ok(())
/// #     }
/// #     fn close(&mut self) -> ConnectionKey {
/// #         self.alive = false;
/// #         ConnectionKey::new("host", &self.conn_type)
/// #     }
/// # }
/// # let factory = Arc::new(|key: &ConnectionKey| {
/// #     Some(Arc::new(Mutex::new(TestConnection {
/// #         conn_type: key.connection_type.clone(),
/// #         alive: false
/// #     })) as Arc<Mutex<dyn Connection>>)
/// # });
/// let manager = ConnectionManager::with_connection_factory(factory);
/// let params = ResolvedConnectionParams {
///     hostname: "10.0.0.1".to_string(),
///     port: Some(22),
///     username: Some("admin".to_string()),
///     password: None,
///     platform: None,
///     extras: None,
/// };
///
/// // Open different connection types
/// manager.open_connection(&ConnectionKey::new("router1", "ssh"), &params)?;
/// manager.open_connection(&ConnectionKey::new("router1", "netconf"), &params)?;
///
/// // Get snapshot of all counters
/// let snapshot = manager.connection_counters_snapshot();
/// let ssh_counters = snapshot.get("ssh").unwrap();
/// let netconf_counters = snapshot.get("netconf").unwrap();
///
/// assert_eq!(ssh_counters.create_calls, 1);
/// assert_eq!(netconf_counters.create_calls, 1);
/// # Ok::<(), String>(())
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ConnectionCounters {
    pub create_calls: usize,
    pub open_calls: usize,
    pub close_calls: usize,
}
/// Thread-safe manager for connection lifecycle and pooling.
///
/// `ConnectionManager` provides centralized management of connections to remote hosts,
/// handling connection creation, caching, opening, and closing. It uses a factory pattern
/// to create connections dynamically based on connection type, and maintains a pool of
/// active connections for reuse across multiple operations.
///
/// The manager is designed for concurrent access and uses lock-free data structures
/// (`DashMap`) for the connection pool and counters, with an `RwLock` for the factory
/// to minimize contention.
///
/// # Architecture
///
/// The manager consists of four main components:
///
/// 1. **Connection Pool** (`connections_map`): A `DashMap` storing active connections
///    keyed by `ConnectionKey` (hostname + connection type). Connections are wrapped
///    in `Arc<Mutex<_>>` for thread-safe sharing and interior mutability.
///
/// 2. **Connection Factory** (`connection_factory`): An optional factory function that
///    creates new connections on demand. The factory is wrapped in `RwLock<Option<Arc<_>>>`
///    to allow runtime configuration while supporting concurrent reads.
///
/// 3. **Usage Counters** (`counters`): A `DashMap` tracking create, open, and close
///    operations per connection type. Useful for monitoring, debugging, and testing.
///
/// 4. **Caching Strategy**: Connections are created lazily on first access and cached
///    for subsequent use. The same connection instance is reused until explicitly closed.
///
/// # Connection Lifecycle
///
/// 1. **Creation**: When `get_or_create()` is called with a new key, the factory is
///    invoked to create a connection. The connection is inserted into the pool and
///    the `create_calls` counter is incremented.
///
/// 2. **Opening**: The `open_connection()` method checks if a connection is alive
///    before calling `open()`. Only actual open operations increment the `open_calls`
///    counter.
///
/// 3. **Reuse**: Subsequent calls with the same key return the cached connection
///    without creating a new one or reopening it if it's still alive.
///
/// 4. **Closing**: Connections can be closed individually via `close_connection()` or
///    all at once via `close_all_connections()`. Closed connections are removed from
///    the pool and the `close_calls` counter is incremented.
///
/// # Thread Safety
///
/// The manager is fully thread-safe and designed for concurrent access:
///
/// - **Lock-Free Pool**: `DashMap` provides concurrent access to the connection pool
///   without requiring a global lock. Different threads can access different connections
///   simultaneously.
///
/// - **Per-Connection Locking**: Each connection is wrapped in `Mutex`, allowing
///   fine-grained locking. Only the thread actively using a connection holds its lock.
///
/// - **Factory Configuration**: The factory uses `RwLock` to allow multiple concurrent
///   reads (connection creation) while serializing writes (factory updates).
///
/// - **Lock Ordering**: Methods acquire locks in a consistent order (factory → connection)
///   and release them promptly to prevent deadlocks.
///
/// # Factory Pattern
///
/// The connection factory is a function that takes a `ConnectionKey` and returns an
/// optional connection. This design allows:
///
/// - **Plugin-Based Architecture**: Different connection types (SSH, NETCONF, HTTP)
///   can be registered dynamically via plugins.
///
/// - **Lazy Loading**: Connections are only created when needed, reducing startup time
///   and resource usage.
///
/// - **Graceful Degradation**: If no plugin is registered for a connection type, the
///   factory returns `None` and the manager propagates this to the caller.
///
/// # Usage Counters
///
/// The manager tracks three types of operations per connection type:
///
/// - `create_calls`: Number of times a new connection was created
/// - `open_calls`: Number of times `open()` was called on a connection
/// - `close_calls`: Number of times a connection was closed
///
/// These counters are useful for:
/// - Monitoring connection pool efficiency
/// - Debugging connection leaks or excessive creation
/// - Testing connection lifecycle behavior
///
/// # Examples
///
/// ## Basic Setup with Factory
///
/// ```
/// use std::sync::{Arc, Mutex};
/// use genja_core::inventory::{Connection, ConnectionKey, ConnectionManager};
///
/// #[derive(Debug)]
/// struct SshConnection {
///     alive: bool,
/// }
///
/// impl Connection for SshConnection {
///     fn create(&self, _key: &ConnectionKey) -> Box<dyn Connection> {
///         Box::new(SshConnection { alive: false })
///     }
///
///     fn is_alive(&self) -> bool {
///         self.alive
///     }
///
///     fn open(&mut self, _params: &genja_core::inventory::ResolvedConnectionParams)
///         -> Result<(), String> {
///         self.alive = true;
///         Ok(())
///     }
///
///     fn close(&mut self) -> ConnectionKey {
///         self.alive = false;
///         ConnectionKey::new("router1", "ssh")
///     }
/// }
///
/// // Create a factory that returns SSH connections
/// let factory = Arc::new(|key: &ConnectionKey| {
///     if key.connection_type == "ssh" {
///         Some(Arc::new(Mutex::new(SshConnection { alive: false })) as Arc<Mutex<dyn Connection>>)
///     } else {
///         None
///     }
/// });
///
/// let manager = ConnectionManager::with_connection_factory(factory);
/// ```
///
/// ## Connection Reuse
///
/// ```
/// # use std::sync::{Arc, Mutex};
/// # use genja_core::inventory::{Connection, ConnectionKey, ConnectionManager};
/// # #[derive(Debug)]
/// # struct SshConnection { alive: bool }
/// # impl Connection for SshConnection {
/// #     fn create(&self, _key: &ConnectionKey) -> Box<dyn Connection> {
/// #         Box::new(SshConnection { alive: false })
/// #     }
/// #     fn is_alive(&self) -> bool { self.alive }
/// #     fn open(&mut self, _params: &genja_core::inventory::ResolvedConnectionParams)
/// #         -> Result<(), String> { self.alive = true; Ok(()) }
/// #     fn close(&mut self) -> ConnectionKey {
/// #         self.alive = false;
/// #         ConnectionKey::new("router1", "ssh")
/// #     }
/// # }
/// # let factory = Arc::new(|_key: &ConnectionKey| {
/// #     Some(Arc::new(Mutex::new(SshConnection { alive: false })) as Arc<Mutex<dyn Connection>>)
/// # });
/// let manager = ConnectionManager::with_connection_factory(factory);
/// let key = ConnectionKey::new("router1", "ssh");
///
/// // First access creates the connection
/// let conn1 = manager.get_or_create(key.clone())?.unwrap();
///
/// // Second access returns the same connection
/// let conn2 = manager.get_or_create(key)?.unwrap();
///
/// assert!(Arc::ptr_eq(&conn1, &conn2));
/// # Ok::<(), String>(())
/// ```
///
/// ## Monitoring Connection Usage
///
/// ```
/// # use std::sync::{Arc, Mutex};
/// # use genja_core::inventory::{Connection, ConnectionKey, ConnectionManager, ResolvedConnectionParams};
/// # #[derive(Debug)]
/// # struct SshConnection { alive: bool }
/// # impl Connection for SshConnection {
/// #     fn create(&self, _key: &ConnectionKey) -> Box<dyn Connection> {
/// #         Box::new(SshConnection { alive: false })
/// #     }
/// #     fn is_alive(&self) -> bool { self.alive }
/// #     fn open(&mut self, _params: &ResolvedConnectionParams) -> Result<(), String> {
/// #         self.alive = true; Ok(())
/// #     }
/// #     fn close(&mut self) -> ConnectionKey {
/// #         self.alive = false;
/// #         ConnectionKey::new("router1", "ssh")
/// #     }
/// # }
/// # let factory = Arc::new(|_key: &ConnectionKey| {
/// #     Some(Arc::new(Mutex::new(SshConnection { alive: false })) as Arc<Mutex<dyn Connection>>)
/// # });
/// let manager = ConnectionManager::with_connection_factory(factory);
/// let key = ConnectionKey::new("router1", "ssh");
/// let params = ResolvedConnectionParams {
///     hostname: "10.0.0.1".to_string(),
///     port: Some(22),
///     username: Some("admin".to_string()),
///     password: None,
///     platform: None,
///     extras: None,
/// };
///
/// manager.open_connection(&key, &params)?;
///
/// // Check counters
/// let counters = manager.connection_counters_for("ssh").unwrap();
/// assert_eq!(counters.create_calls, 1);
/// assert_eq!(counters.open_calls, 1);
/// # Ok::<(), String>(())
/// ```
///
/// ## Cleanup
///
/// ```
/// # use std::sync::{Arc, Mutex};
/// # use genja_core::inventory::{Connection, ConnectionKey, ConnectionManager};
/// # #[derive(Debug)]
/// # struct SshConnection { alive: bool }
/// # impl Connection for SshConnection {
/// #     fn create(&self, _key: &ConnectionKey) -> Box<dyn Connection> {
/// #         Box::new(SshConnection { alive: false })
/// #     }
/// #     fn is_alive(&self) -> bool { self.alive }
/// #     fn open(&mut self, _params: &genja_core::inventory::ResolvedConnectionParams)
/// #         -> Result<(), String> { self.alive = true; Ok(()) }
/// #     fn close(&mut self) -> ConnectionKey {
/// #         self.alive = false;
/// #         ConnectionKey::new("router1", "ssh")
/// #     }
/// # }
/// # let factory = Arc::new(|_key: &ConnectionKey| {
/// #     Some(Arc::new(Mutex::new(SshConnection { alive: false })) as Arc<Mutex<dyn Connection>>)
/// # });
/// let manager = ConnectionManager::with_connection_factory(factory);
/// let key1 = ConnectionKey::new("router1", "ssh");
/// let key2 = ConnectionKey::new("router2", "ssh");
///
/// manager.get_or_create(key1.clone())?;
/// manager.get_or_create(key2.clone())?;
///
/// // Close specific connection
/// manager.close_connection(&key1);
///
/// // Close all remaining connections
/// manager.close_all_connections();
///
/// let counters = manager.connection_counters_for("ssh").unwrap();
/// assert_eq!(counters.close_calls, 2);
/// # Ok::<(), String>(())
/// ```
pub struct ConnectionManager {
    connections_map: DashMap<ConnectionKey, Arc<Mutex<dyn Connection>>>,
    connection_factory: RwLock<Option<Arc<ConnectionFactory>>>,
    counters: Arc<DashMap<String, ConnectionCounters>>,
}

impl fmt::Debug for ConnectionManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ConnectionManager")
            .field("connections_map_len", &self.connections_map.len())
            .field(
                "has_connection_factory",
                &self
                    .connection_factory
                    .read()
                    .map(|factory| factory.is_some())
                    .unwrap_or(false),
            )
            .finish()
    }
}

impl ConnectionManager {
    pub fn with_connection_factory(factory: Arc<ConnectionFactory>) -> Self {
        Self {
            connections_map: DashMap::new(),
            connection_factory: RwLock::new(Some(factory)),
            counters: Arc::new(DashMap::new()),
        }
    }

    fn increment_create(&self, connection_type: &str) {
        let mut entry = self
            .counters
            .entry(connection_type.to_string())
            .or_default();
        entry.create_calls += 1;
    }

    fn increment_open(&self, connection_type: &str) {
        let mut entry = self
            .counters
            .entry(connection_type.to_string())
            .or_default();
        entry.open_calls += 1;
    }

    fn increment_close(&self, connection_type: &str) {
        let mut entry = self
            .counters
            .entry(connection_type.to_string())
            .or_default();
        entry.close_calls += 1;
    }

    pub fn connection_counters_for(&self, connection_type: &str) -> Option<ConnectionCounters> {
        self.counters.get(connection_type).map(|entry| *entry)
    }

    pub fn connection_counters_snapshot(&self) -> HashMap<String, ConnectionCounters> {
        self.counters
            .iter()
            .map(|entry| (entry.key().clone(), *entry.value()))
            .collect()
    }

    pub fn get(&self, key: &ConnectionKey) -> Option<Arc<Mutex<dyn Connection>>> {
        self.connections_map
            .get(key)
            .map(|entry| entry.value().clone())
    }

    pub fn insert(&self, key: ConnectionKey, connection: Arc<Mutex<dyn Connection>>) {
        self.connections_map.insert(key, connection);
    }

    /// Retrieves an existing connection or creates a new one using the configured factory.
    ///
    /// This method provides thread-safe, lazy initialization of connections. It first checks
    /// for an existing connection in the map, and if missing, it uses the connection factory
    /// to create one and inserts it.
    ///
    /// # Concurrency and Race Behavior
    ///
    /// - Creation uses `DashMap::entry`, so only one connection is created per unique key,
    ///   even under concurrent access.
    /// - The factory is called while holding the entry lock for that key’s shard. This avoids
    ///   race conditions but means a slow factory can temporarily block other operations on the
    ///   same shard.
    /// - If the factory returns `None`, no entry is inserted and subsequent calls may retry.
    ///
    /// # Parameters
    ///
    /// * `key` - A `ConnectionKey` identifying the connection by hostname and connection type.
    ///
    /// # Returns
    ///
    /// - `Ok(Some(connection))` if a connection exists or was created
    /// - `Ok(None)` if the factory returns `None` (e.g., no plugin registered)
    /// - `Err(...)` if the factory lock is poisoned or not configured
    ///
    /// # Errors
    ///
    /// - `"connection factory not set"` if no factory is configured
    /// - `"connection factory lock poisoned"` if the factory lock is poisoned
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::{Arc, Mutex};
    /// use genja_core::inventory::{Connection, ConnectionKey, ConnectionManager};
    ///
    /// #[derive(Debug)]
    /// struct DummyConnection;
    ///
    /// impl Connection for DummyConnection {
    ///     fn create(&self, _key: &ConnectionKey) -> Box<dyn Connection> {
    ///         Box::new(DummyConnection)
    ///     }
    ///     fn is_alive(&self) -> bool { true }
    ///     fn open(&mut self, _params: &genja_core::inventory::ResolvedConnectionParams)
    ///         -> Result<(), String> { Ok(()) }
    ///     fn close(&mut self) -> ConnectionKey {
    ///         ConnectionKey::new("router1", "ssh")
    ///     }
    /// }
    ///
    /// let factory = Arc::new(|_key: &ConnectionKey| {
    ///     Some(Arc::new(Mutex::new(DummyConnection)) as Arc<Mutex<dyn Connection>>)
    /// });
    /// let manager = ConnectionManager::with_connection_factory(factory);
    ///
    /// let key = ConnectionKey::new("router1", "ssh");
    /// let connection = manager.get_or_create(key)?;
    /// assert!(connection.is_some());
    /// # Ok::<(), String>(())
    /// ```
    pub fn get_or_create(
        &self,
        key: ConnectionKey,
    ) -> Result<Option<Arc<Mutex<dyn Connection>>>, String> {
        let factory = {
            let guard = self
                .connection_factory
                .read()
                .map_err(|_| "connection factory lock poisoned".to_string())?;
            guard
                .clone()
                .ok_or_else(|| "connection factory not set".to_string())?
        };

        match self.connections_map.entry(key) {
            dashmap::mapref::entry::Entry::Occupied(entry) => Ok(Some(entry.get().clone())),
            dashmap::mapref::entry::Entry::Vacant(entry) => {
                let key_for_factory = entry.key().clone();
                let connection_type = key_for_factory.connection_type.clone();
                let Some(connection) = factory(&key_for_factory) else {
                    return Ok(None);
                };
                self.increment_create(&connection_type);
                entry.insert(connection.clone());
                Ok(Some(connection))
            }
        }
    }

    pub fn set_connection_factory(&self, factory: Arc<ConnectionFactory>) {
        if let Ok(mut slot) = self.connection_factory.write() {
            *slot = Some(factory);
        }
    }

    /// Close the connection associated with the given key and remove
    /// it from `connections_map`.
    pub fn close_connection(&self, key: &ConnectionKey) {
        if let Some((_, connection)) = self.connections_map.remove(key) {
            if let Ok(mut connection) = connection.lock() {
                connection.close();
                self.increment_close(&key.connection_type);
            }
        }
    }

    /// Close all connections in `connections_map` and then clear the map.
    pub fn close_all_connections(&self) {
        self.connections_map.iter().for_each(|entry| {
            if let Ok(mut connection) = entry.value().lock() {
                connection.close();
                self.increment_close(&entry.key().connection_type);
            }
        });
        self.connections_map.clear();
    }

    /// Opens a connection for the specified key, creating it if necessary.
    ///
    /// This method provides a high-level interface for establishing connections to remote hosts.
    /// It combines connection retrieval/creation with automatic opening, ensuring the connection
    /// is ready for use before returning. The method handles the full lifecycle:
    ///
    /// 1. **Retrieve or Create**: Calls `get_or_create()` to obtain a connection from the map
    ///    or create a new one using the configured factory
    /// 2. **Check Alive Status**: Acquires the connection's mutex and checks if it's already open
    /// 3. **Open if Needed**: If the connection is not alive, calls `open()` with the provided
    ///    parameters and increments the open counter
    /// 4. **Return Ready Connection**: Returns the connection wrapped in `Arc<Mutex<_>>` for
    ///    thread-safe access
    ///
    /// # Parameters
    ///
    /// * `key` - A `ConnectionKey` identifying the connection by hostname and connection type.
    ///   This key is used to look up or create the connection in the manager's map.
    /// * `params` - A `ResolvedConnectionParams` containing the connection parameters such as
    ///   hostname, port, username, password, and platform. These parameters are passed to the
    ///   connection's `open()` method if the connection needs to be established.
    ///
    /// # Thread Safety and Locking
    ///
    /// The method uses a two-phase locking strategy to prevent deadlocks:
    ///
    /// 1. **Factory Lock**: `get_or_create()` briefly acquires the factory's `RwLock` to clone
    ///    the `Arc<ConnectionFactory>`, then releases it before calling the factory function.
    ///    This prevents holding the factory lock during connection creation.
    ///
    /// 2. **Connection Lock**: After obtaining the connection, the method acquires its `Mutex`
    ///    in a scoped block (`{ ... }`). The lock is automatically released when the scope ends,
    ///    before returning the connection. This allows the caller to acquire the lock again
    ///    without deadlock.
    ///
    /// **Why the scoped lock?**
    /// ```text
    /// Without scope:                    With scope:
    /// ---------------                   -----------
    /// let mut guard = conn.lock();      {
    /// guard.open(params)?;                  let mut guard = conn.lock();
    /// // guard still held                   guard.open(params)?;
    /// Ok(Some(conn))                    } // guard dropped here
    /// // Caller tries conn.lock()       Ok(Some(conn))
    /// // DEADLOCK! 💥                   // Caller can lock successfully ✓
    /// ```
    ///
    /// # Connection Lifecycle
    ///
    /// The method respects the connection's alive state:
    /// - If `is_alive()` returns `true`, the connection is already open and no action is taken
    /// - If `is_alive()` returns `false`, `open()` is called to establish the connection
    /// - The `open_calls` counter is incremented only when `open()` is actually called
    ///
    /// This prevents unnecessary reconnection attempts and tracks actual connection operations.
    ///
    /// # Returns
    ///
    /// Returns `Ok(Some(Arc<Mutex<dyn Connection>>))` if:
    /// - The connection was successfully retrieved or created, AND
    /// - The connection was already alive OR was successfully opened
    ///
    /// Returns `Ok(None)` if:
    /// - The factory function returned `None` (e.g., no plugin registered for this connection type)
    ///
    /// Returns `Err(String)` if:
    /// - The connection factory is not configured: `"connection factory not set"`
    /// - The factory lock is poisoned: `"connection factory lock poisoned"`
    /// - The connection lock is poisoned: `"connection lock poisoned"`
    /// - The connection's `open()` method returns an error (error message from the connection)
    ///
    /// # Examples
    ///
    /// ## Basic Usage
    ///
    /// ```
    /// use std::sync::{Arc, Mutex};
    /// use genja_core::inventory::{
    ///     Connection, ConnectionKey, ConnectionManager, ResolvedConnectionParams
    /// };
    ///
    /// #[derive(Debug)]
    /// struct SshConnection {
    ///     alive: bool,
    /// }
    ///
    /// impl Connection for SshConnection {
    ///     fn create(&self, _key: &ConnectionKey) -> Box<dyn Connection> {
    ///         Box::new(SshConnection { alive: false })
    ///     }
    ///
    ///     fn is_alive(&self) -> bool {
    ///         self.alive
    ///     }
    ///
    ///     fn open(&mut self, _params: &ResolvedConnectionParams) -> Result<(), String> {
    ///         self.alive = true;
    ///         Ok(())
    ///     }
    ///
    ///     fn close(&mut self) -> ConnectionKey {
    ///         self.alive = false;
    ///         ConnectionKey::new("router1", "ssh")
    ///     }
    /// }
    ///
    /// let factory = Arc::new(|_key: &ConnectionKey| {
    ///     Some(Arc::new(Mutex::new(SshConnection { alive: false })) as Arc<Mutex<dyn Connection>>)
    /// });
    /// let manager = ConnectionManager::with_connection_factory(factory);
    ///
    /// let key = ConnectionKey::new("router1", "ssh");
    /// let params = ResolvedConnectionParams {
    ///     hostname: "10.0.0.1".to_string(),
    ///     port: Some(22),
    ///     username: Some("admin".to_string()),
    ///     password: None,
    ///     platform: None,
    ///     extras: None,
    /// };
    ///
    /// // First call creates and opens the connection
    /// let connection = manager.open_connection(&key, &params)?;
    /// assert!(connection.is_some());
    ///
    /// // Second call reuses the existing connection without reopening
    /// let same_connection = manager.open_connection(&key, &params)?;
    /// assert!(Arc::ptr_eq(&connection.unwrap(), &same_connection.unwrap()));
    /// # Ok::<(), String>(())
    /// ```
    ///
    /// ## Handling Missing Plugins
    ///
    /// ```
    /// use std::sync::Arc;
    /// use genja_core::inventory::{ConnectionKey, ConnectionManager, ResolvedConnectionParams};
    ///
    /// // Factory returns None for unknown connection types
    /// let factory = Arc::new(|key: &ConnectionKey| {
    ///     if key.connection_type == "ssh" {
    ///         // ... return SSH connection
    ///         None // simplified for example
    ///     } else {
    ///         None // No plugin for this type
    ///     }
    /// });
    /// let manager = ConnectionManager::with_connection_factory(factory);
    ///
    /// let key = ConnectionKey::new("router1", "telnet");
    /// let params = ResolvedConnectionParams {
    ///     hostname: "10.0.0.1".to_string(),
    ///     port: None,
    ///     username: None,
    ///     password: None,
    ///     platform: None,
    ///     extras: None,
    /// };
    ///
    /// let result = manager.open_connection(&key, &params)?;
    /// assert!(result.is_none()); // No plugin available
    /// # Ok::<(), String>(())
    /// ```
    ///
    /// ## Thread-Safe Concurrent Access
    ///
    /// ```
    /// use std::sync::{Arc, Mutex};
    /// use std::thread;
    /// use genja_core::inventory::{
    ///     Connection, ConnectionKey, ConnectionManager, ResolvedConnectionParams
    /// };
    ///
    /// # #[derive(Debug)]
    /// # struct SshConnection { alive: bool }
    /// # impl Connection for SshConnection {
    /// #     fn create(&self, _key: &ConnectionKey) -> Box<dyn Connection> {
    /// #         Box::new(SshConnection { alive: false })
    /// #     }
    /// #     fn is_alive(&self) -> bool { self.alive }
    /// #     fn open(&mut self, _params: &ResolvedConnectionParams) -> Result<(), String> {
    /// #         self.alive = true;
    /// #         Ok(())
    /// #     }
    /// #     fn close(&mut self) -> ConnectionKey {
    /// #         self.alive = false;
    /// #         ConnectionKey::new("router1", "ssh")
    /// #     }
    /// # }
    /// let factory = Arc::new(|_key: &ConnectionKey| {
    ///     Some(Arc::new(Mutex::new(SshConnection { alive: false })) as Arc<Mutex<dyn Connection>>)
    /// });
    /// let manager = Arc::new(ConnectionManager::with_connection_factory(factory));
    ///
    /// let key = ConnectionKey::new("router1", "ssh");
    /// let params = Arc::new(ResolvedConnectionParams {
    ///     hostname: "10.0.0.1".to_string(),
    ///     port: None,
    ///     username: None,
    ///     password: None,
    ///     platform: None,
    ///     extras: None,
    /// });
    ///
    /// // Multiple threads can safely open the same connection
    /// let handles: Vec<_> = (0..3)
    ///     .map(|_| {
    ///         let manager = Arc::clone(&manager);
    ///         let key = key.clone();
    ///         let params = Arc::clone(&params);
    ///         thread::spawn(move || {
    ///             manager.open_connection(&key, &params)
    ///         })
    ///     })
    ///     .collect();
    ///
    /// for handle in handles {
    ///     let result = handle.join().unwrap();
    ///     assert!(result.is_ok());
    /// }
    /// ```
    pub fn open_connection(
        &self,
        key: &ConnectionKey,
        params: &ResolvedConnectionParams,
    ) -> Result<Option<Arc<Mutex<dyn Connection>>>, String> {
        let Some(connection) = self.get_or_create(key.clone())? else {
            return Ok(None);
        };

        {
            let mut guard = connection
                .lock()
                .map_err(|_| "connection lock poisoned".to_string())?;
            if !guard.is_alive() {
                guard.open(params)?;
                self.increment_open(&key.connection_type);
            }
        }
        Ok(Some(connection))
    }
}

impl Default for ConnectionManager {
    fn default() -> Self {
        Self {
            connections_map: DashMap::new(),
            connection_factory: RwLock::new(None),
            counters: Arc::new(DashMap::new()),
        }
    }
}

/// In-memory inventory container.
///
/// Aggregates hosts, groups, defaults, and optional transform settings.
/// This struct is deserializable and is the primary shape used by the
/// inventory loader and runtime.
///
/// Transforms are applied lazily when accessing hosts, groups, or defaults
/// via the view accessors (e.g., `hosts()`).
///
/// # Deserialization
///
/// - Missing fields use their default values (see `Default` impl)
/// - Unknown fields are rejected for nested host/group items (see `Hosts` and `Groups`)
///
/// # Examples
///
/// ```
/// use genja_core::inventory::{Inventory, Hosts, Host};
/// use genja_core::inventory::BaseBuilderHost;
///
/// let mut hosts = Hosts::new();
/// hosts.add_host("router1", Host::builder().hostname("10.0.0.1").build());
///
/// let inventory = Inventory::builder().hosts(hosts).build();
/// assert_eq!(inventory.hosts().len(), 1);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Inventory {
    pub(crate) hosts: Hosts,
    pub(crate) groups: Option<Groups>,
    pub(crate) defaults: Option<Defaults>,
    #[serde(skip)]
    transform_function: Option<TransformFunction>,
    transform_function_options: Option<TransformFunctionOptions>,
    #[serde(skip)]
    #[schemars(skip)]
    connections: Arc<ConnectionManager>,
    #[serde(skip)]
    #[schemars(skip)]
    host_cache: DashMap<NatString, Host>,
    #[serde(skip)]
    #[schemars(skip)]
    group_cache: DashMap<NatString, Group>,
    #[serde(skip)]
    #[schemars(skip)]
    resolved_host_cache: DashMap<NatString, Host>,
    #[serde(skip)]
    #[schemars(skip)]
    resolved_params_cache: DashMap<(NatString, String), ResolvedConnectionParams>,
}

impl BaseMethods for Inventory {}

impl Inventory {
    /// Creates a new builder for constructing an `Inventory` instance.
    ///
    /// This method provides a fluent interface for building an `Inventory` with custom
    /// configuration. The builder allows you to set optional hosts, groups, defaults,
    /// transform functions, and connection managers before calling `build()` to create
    /// the final inventory.
    ///
    /// # Returns
    ///
    /// Returns a new `InventoryBuilder` instance with all fields initialized to `None`.
    /// Use the builder's methods to configure the inventory before calling `build()`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use genja_core::inventory::{Inventory, Hosts, Host, BaseBuilderHost};
    /// let mut hosts = Hosts::new();
    /// hosts.add_host("router1", Host::builder().hostname("10.0.0.1").build());
    ///
    /// let inventory = Inventory::builder()
    ///     .hosts(hosts)
    ///     .build();
    ///
    /// assert_eq!(inventory.hosts().len(), 1);
    /// ```
    pub fn builder() -> InventoryBuilder {
        InventoryBuilder::new()
    }

    /// Returns a view of the inventory's hosts collection with transform functions applied.
    ///
    /// This method provides access to the inventory's hosts through a `HostsView` wrapper
    /// that applies any configured transform function when accessing individual hosts.
    /// The view provides read-only access to the hosts and caches transformed results
    /// for improved performance on subsequent accesses.
    ///
    /// # Returns
    ///
    /// Returns a `HostsView` containing a view of the hosts collection. The view allows
    /// iteration over hosts and lookup by name, with transforms applied lazily on access.
    ///
    /// # Examples
    ///
    /// ```
    /// # use genja_core::inventory::{Inventory, Hosts, Host, BaseBuilderHost};
    /// let mut hosts = Hosts::new();
    /// hosts.add_host("router1", Host::builder().hostname("10.0.0.1").build());
    ///
    /// let inventory = Inventory::builder()
    ///     .hosts(hosts)
    ///     .build();
    ///
    /// let hosts_view = inventory.hosts();
    /// assert_eq!(hosts_view.len(), 1);
    /// if let Some(host) = hosts_view.get("router1") {
    ///     assert_eq!(host.hostname(), Some("10.0.0.1"));
    /// }
    /// ```
    pub fn hosts(&self) -> HostsView<'_> {
        HostsView { inventory: self }
    }

    /// Returns a reference to the raw hosts collection without applying transforms.
    ///
    /// This accessor provides direct, read-only access to the underlying `Hosts`
    /// data stored in the inventory. No transform function is applied, and no
    /// cache is populated. This is useful for debugging, inspection, or when you
    /// explicitly need the original, unmodified host data.
    ///
    /// # Returns
    ///
    /// Returns a reference to the raw `Hosts` collection.
    ///
    /// # Examples
    ///
    /// ```
    /// # use genja_core::inventory::{Inventory, Hosts, Host, BaseBuilderHost};
    /// let mut hosts = Hosts::new();
    /// hosts.add_host("router1", Host::builder().hostname("10.0.0.1").build());
    ///
    /// let inventory = Inventory::builder()
    ///     .hosts(hosts)
    ///     .build();
    ///
    /// let raw_hosts = inventory.hosts_raw();
    /// assert_eq!(raw_hosts.len(), 1);
    /// ```
    pub fn hosts_raw(&self) -> &Hosts {
        &self.hosts
    }

    /// Returns a view of the inventory's groups collection with transform functions applied.
    ///
    /// This method provides access to the inventory's groups through a `GroupsView` wrapper
    /// that applies any configured transform function when accessing individual groups.
    /// The view provides read-only access to the groups and caches transformed results
    /// for improved performance on subsequent accesses.
    ///
    /// # Returns
    ///
    /// Returns `Some(GroupsView)` containing a view of the groups collection if groups
    /// are configured in the inventory. Returns `None` if no groups are present.
    ///
    /// # Examples
    ///
    /// ```
    /// # use genja_core::inventory::{Inventory, Groups, Group, BaseBuilderHost};
    /// let mut groups = Groups::new();
    /// groups.add_group("core", Group::builder().platform("linux").build());
    ///
    /// let inventory = Inventory::builder()
    ///     .groups(groups)
    ///     .build();
    ///
    /// if let Some(groups_view) = inventory.groups() {
    ///     assert_eq!(groups_view.len(), 1);
    ///     if let Some(group) = groups_view.get("core") {
    ///         assert_eq!(group.platform(), Some("linux"));
    ///     }
    /// }
    /// ```
    pub fn groups(&self) -> Option<GroupsView<'_>> {
        self.groups.as_ref().map(|groups| GroupsView {
            inventory: self,
            groups,
        })
    }

    /// Returns a reference to the raw groups collection without applying transforms.
    ///
    /// This accessor provides direct, read-only access to the underlying `Groups`
    /// data stored in the inventory. No transform function is applied, and no
    /// cache is populated. This is useful for debugging, inspection, or when you
    /// explicitly need the original, unmodified group data.
    ///
    /// # Returns
    ///
    /// Returns `Some(&Groups)` if groups are configured in the inventory, or `None`
    /// if no groups are present.
    ///
    /// # Examples
    ///
    /// ```
    /// # use genja_core::inventory::{Inventory, Groups, Group, BaseBuilderHost};
    /// let mut groups = Groups::new();
    /// groups.add_group("core", Group::builder().platform("linux").build());
    ///
    /// let inventory = Inventory::builder()
    ///     .groups(groups)
    ///     .build();
    ///
    /// let raw_groups = inventory.groups_raw().expect("groups exist");
    /// assert_eq!(raw_groups.len(), 1);
    /// ```
    pub fn groups_raw(&self) -> Option<&Groups> {
        self.groups.as_ref()
    }

    /// Returns the inventory's default configuration after applying any configured transform function.
    ///
    /// This method provides access to the inventory-wide defaults that apply to all hosts and groups.
    /// If a transform function is configured on the inventory, it will be applied to the defaults
    /// before returning them. The transform allows for dynamic modification of default values based
    /// on custom logic or external configuration.
    ///
    /// # Returns
    ///
    /// Returns `Some(Defaults)` containing the default configuration (potentially transformed) if
    /// defaults are configured in the inventory. Returns `None` if no defaults are set.
    ///
    /// # Examples
    ///
    /// ```
    /// # use genja_core::inventory::{Inventory, Defaults};
    /// let defaults = Defaults::builder()
    ///     .username("admin")
    ///     .port(22)
    ///     .build();
    ///
    /// let inventory = Inventory::builder()
    ///     .defaults(defaults)
    ///     .build();
    ///
    /// if let Some(defaults) = inventory.defaults() {
    ///     assert_eq!(defaults.username(), Some("admin"));
    ///     assert_eq!(defaults.port(), Some(22));
    /// }
    /// ```
    pub fn defaults(&self) -> Option<Defaults> {
        self.defaults
            .as_ref()
            .map(|defaults| self.transform_defaults_value(defaults))
    }

    /// Returns a reference to the raw defaults configuration without applying transforms.
    ///
    /// This accessor provides direct, read-only access to the underlying `Defaults`
    /// data stored in the inventory. No transform function is applied. This is useful
    /// for debugging, inspection, or when you explicitly need the original defaults.
    ///
    /// # Returns
    ///
    /// Returns `Some(&Defaults)` if defaults are configured in the inventory, or `None`
    /// if no defaults are set.
    ///
    /// # Examples
    ///
    /// ```
    /// # use genja_core::inventory::{Inventory, Defaults};
    /// let defaults = Defaults::builder()
    ///     .username("admin")
    ///     .port(22)
    ///     .build();
    ///
    /// let inventory = Inventory::builder()
    ///     .defaults(defaults)
    ///     .build();
    ///
    /// let raw_defaults = inventory.defaults_raw().expect("defaults exist");
    /// assert_eq!(raw_defaults.username(), Some("admin"));
    /// ```
    pub fn defaults_raw(&self) -> Option<&Defaults> {
        self.defaults.as_ref()
    }

    /// Returns a reference to the transform function options configured for this inventory.
    ///
    /// Transform function options provide additional configuration data that is passed to
    /// the transform function when it processes hosts, groups, or defaults. These options
    /// allow for dynamic customization of the transform behavior without modifying the
    /// transform function itself.
    ///
    /// The options are stored as a `TransformFunctionOptions` wrapper around a JSON value,
    /// allowing for flexible, schema-free configuration data.
    ///
    /// # Returns
    ///
    /// Returns `Some(&TransformFunctionOptions)` containing a reference to the configured
    /// options if they are set. Returns `None` if no transform function options have been
    /// configured for this inventory.
    ///
    /// # Examples
    ///
    /// ```
    /// # use genja_core::inventory::{Inventory, TransformFunctionOptions};
    /// let options = TransformFunctionOptions::new(serde_json::json!({"key": "value"}));
    /// let inventory = Inventory::builder()
    ///     .transform_function_options(options)
    ///     .build();
    ///
    /// if let Some(opts) = inventory.transform_function_options() {
    ///     println!("Transform options configured");
    /// }
    /// ```
    pub fn transform_function_options(&self) -> Option<&TransformFunctionOptions> {
        self.transform_function_options.as_ref()
    }

    pub fn connections(&self) -> &ConnectionManager {
        &self.connections
    }

    #[cfg(test)]
    pub(crate) fn resolved_host_cache_len(&self) -> usize {
        self.resolved_host_cache.len()
    }

    #[cfg(test)]
    pub(crate) fn resolved_params_cache_len(&self) -> usize {
        self.resolved_params_cache.len()
    }

    /// Resolves a host by applying defaults, group settings, and host-specific configuration.
    ///
    /// This method performs hierarchical resolution of host configuration by merging settings
    /// from multiple sources in priority order. The resolution follows this sequence:
    ///
    /// 1. Start with an empty host configuration
    /// 2. Apply inventory defaults (if present)
    /// 3. Apply parent group settings recursively (in order of group declaration)
    /// 4. Apply host-specific settings
    /// 5. Apply transform function (if configured)
    ///
    /// The result is cached to improve performance on subsequent calls for the same host.
    /// Group resolution handles inheritance chains and prevents circular references.
    ///
    /// # Parameters
    ///
    /// * `name` - The name of the host to resolve. This should match a key in the inventory's
    ///   hosts collection. The name is used for both lookup and cache key generation.
    ///
    /// # Returns
    ///
    /// Returns `Some(Host)` containing the fully resolved host configuration if the host exists
    /// in the inventory. Returns `None` if the host is not found.
    ///
    /// # Examples
    ///
    /// ```
    /// # use genja_core::inventory::{Inventory, Host, Hosts, BaseBuilderHost};
    /// let mut hosts = Hosts::new();
    /// hosts.add_host("router1", Host::builder().hostname("10.0.0.1").build());
    /// let inventory = Inventory::builder().hosts(hosts).build();
    ///
    /// if let Some(resolved) = inventory.resolve_host("router1") {
    ///     println!("Resolved hostname: {:?}", resolved.hostname());
    /// }
    /// ```
    pub fn resolve_host(&self, name: &str) -> Option<Host> {
        let key = NatString::new(name.to_string());
        if let Some(entry) = self.resolved_host_cache.get(&key) {
            return Some(entry.value().clone());
        }

        let host = self.hosts.get(name)?;
        let mut resolved = Host::new();

        if let Some(defaults) = self.defaults.as_ref() {
            merge_defaults_into_host(&mut resolved, defaults);
        }

        let mut group_stack = std::collections::HashSet::new();
        let mut group_cache = std::collections::HashMap::new();
        if let Some(groups) = host.groups.as_ref() {
            for group_name in groups.iter() {
                if let Some(group) =
                    self.resolve_group_internal(group_name, &mut group_stack, &mut group_cache)
                {
                    merge_group_into_host(&mut resolved, &group);
                }
            }
        }

        merge_host_into_host(&mut resolved, host);

        let resolved = self.transform_host_value(&resolved);
        self.resolved_host_cache.insert(key, resolved.clone());
        Some(resolved)
    }

    /// Resolves connection parameters for a specific host and connection type.
    ///
    /// This method combines defaults, group settings, and host-specific configuration
    /// to produce a complete set of connection parameters. The resolution follows a
    /// hierarchical priority order where each level can have both base fields and
    /// connection-specific overrides:
    ///
    /// **Priority Order (lowest to highest):**
    /// 1. `defaults` base fields
    /// 2. `defaults.connection_options[connection_type]`
    /// 3. `groups` base fields (applied in order for each parent group)
    /// 4. `groups.connection_options[connection_type]` (applied in order for each parent group)
    /// 5. `host` base fields
    /// 6. `host.connection_options[connection_type]`
    ///
    /// At each level, connection-specific options override the base fields for that level.
    /// The final result is a complete set of connection parameters with all fields resolved
    /// according to this cascading priority system.
    ///
    /// Results are cached to improve performance on subsequent calls with the same parameters.
    ///
    /// # Parameters
    ///
    /// * `name` - The name of the host to resolve connection parameters for. This should
    ///   match a key in the inventory's hosts collection.
    /// * `connection_type` - The type of connection to resolve parameters for (e.g., "ssh",
    ///   "netconf", "http"). This determines which connection_options entry to apply.
    ///
    /// # Returns
    ///
    /// Returns `Some(ResolvedConnectionParams)` containing the fully resolved connection
    /// parameters if the host exists in the inventory. Returns `None` if the host is not
    /// found or cannot be resolved.
    ///
    /// # Examples
    ///
    /// ```
    /// # use genja_core::inventory::{Inventory, Host, Hosts, BaseBuilderHost};
    /// let mut hosts = Hosts::new();
    /// hosts.add_host("router1", Host::builder().hostname("10.0.0.1").build());
    /// let inventory = Inventory::builder().hosts(hosts).build();
    ///
    /// if let Some(params) = inventory.resolve_connection_params("router1", "ssh") {
    ///     println!("Hostname: {}", params.hostname);
    /// }
    /// ```
    ///
    /// # Resolution Example
    ///
    /// ```text
    /// Given:
    ///   defaults:
    ///     port: 22
    ///     connection_options:
    ///       netconf: { port: 830 }
    ///
    ///   groups["cisco"]:
    ///     port: 2200
    ///     connection_options:
    ///       netconf: { port: 831 }
    ///
    ///   host["router1.lab"]:
    ///     groups: ["cisco"]
    ///     port: 2201
    ///     connection_options:
    ///       netconf: { port: 832 }
    ///
    /// Resolution for connection_type "netconf":
    ///   1. defaults.port = 22
    ///   2. defaults.connection_options["netconf"].port = 830 (overrides step 1)
    ///   3. groups["cisco"].port = 2200 (overrides step 2)
    ///   4. groups["cisco"].connection_options["netconf"].port = 831 (overrides step 3)
    ///   5. host.port = 2201 (overrides step 4)
    ///   6. host.connection_options["netconf"].port = 832 (overrides step 5)
    ///
    /// Final result: port = 832
    /// ```
    pub fn resolve_connection_params(
        &self,
        name: &str,
        connection_type: &str,
    ) -> Option<ResolvedConnectionParams> {
        let key = (
            NatString::new(name.to_string()),
            connection_type.to_string(),
        );
        if let Some(entry) = self.resolved_params_cache.get(&key) {
            return Some(entry.value().clone());
        }

        let host = self.resolve_host(name)?;
        let resolved = host.resolve_connection_params(connection_type);
        self.resolved_params_cache.insert(key, resolved.clone());
        Some(resolved)
    }

    /// Recursively resolves a group by applying parent group settings and handling inheritance chains.
    ///
    /// This internal method performs hierarchical resolution of group configuration by merging settings
    /// from parent groups. It uses memoization to cache resolved groups and a stack to detect and prevent
    /// circular references in the group hierarchy.
    ///
    /// The resolution process:
    /// 1. Checks the memo cache for previously resolved groups
    /// 2. Detects circular references using the stack
    /// 3. Recursively resolves parent groups
    /// 4. Merges parent group settings into the current group
    /// 5. Caches the result for future lookups
    ///
    /// # Parameters
    ///
    /// * `name` - The name of the group to resolve. This should match a key in the inventory's
    ///   groups collection.
    /// * `stack` - A mutable reference to a HashSet tracking the current resolution path. Used to
    ///   detect circular references in the group hierarchy. Groups already in the stack indicate
    ///   a circular dependency and will cause the method to return `None`.
    /// * `memo` - A mutable reference to a HashMap caching previously resolved groups. This improves
    ///   performance by avoiding redundant resolution of the same group during recursive traversal.
    ///
    /// # Returns
    ///
    /// Returns `Some(Group)` containing the fully resolved group configuration with all parent
    /// settings merged. Returns `None` if:
    /// - The group does not exist in the inventory
    /// - A circular reference is detected in the group hierarchy
    /// - The inventory has no groups collection
    fn resolve_group_internal(
        &self,
        name: &str,
        stack: &mut std::collections::HashSet<String>,
        memo: &mut std::collections::HashMap<String, Group>,
    ) -> Option<Group> {
        if let Some(cached) = memo.get(name) {
            return Some(cached.clone());
        }

        if !stack.insert(name.to_string()) {
            return None;
        }

        let group = self.groups.as_ref()?.get(name)?;
        let mut resolved = empty_group();

        if let Some(parent_groups) = group.groups.as_ref() {
            for parent in parent_groups.iter() {
                if let Some(parent_group) = self.resolve_group_internal(parent, stack, memo) {
                    merge_group_into_group(&mut resolved, &parent_group);
                }
            }
        }

        merge_group_into_group(&mut resolved, group);

        stack.remove(name);
        memo.insert(name.to_string(), resolved.clone());
        Some(resolved)
    }

    fn transform_host_value(&self, host: &Host) -> Host {
        let transformed = match &self.transform_function {
            Some(transform) => {
                transform.transform_host(host, self.transform_function_options.as_ref())
            }
            None => host.clone(),
        };

        transformed
    }

    fn transform_group_value(&self, group: &Group) -> Group {
        let transformed = match &self.transform_function {
            Some(transform) => {
                transform.transform_group(group, self.transform_function_options.as_ref())
            }
            None => group.clone(),
        };

        transformed
    }

    fn cached_host_value(&self, key: &NatString, host: &Host) -> Host {
        if let Some(entry) = self.host_cache.get(key) {
            return entry.value().clone();
        }

        let transformed = self.transform_host_value(host);
        self.host_cache.insert(key.clone(), transformed.clone());
        transformed
    }

    fn cached_group_value(&self, key: &NatString, group: &Group) -> Group {
        if let Some(entry) = self.group_cache.get(key) {
            return entry.value().clone();
        }

        let transformed = self.transform_group_value(group);
        self.group_cache.insert(key.clone(), transformed.clone());
        transformed
    }

    fn transform_defaults_value(&self, defaults: &Defaults) -> Defaults {
        match &self.transform_function {
            Some(transform) => {
                transform.transform_defaults(defaults, self.transform_function_options.as_ref())
            }
            None => defaults.clone(),
        }
    }
}

fn empty_group() -> Group {
    Group {
        hostname: None,
        port: None,
        username: None,
        password: None,
        platform: None,
        groups: None,
        data: None,
        connection_options: None,
    }
}

fn merge_defaults_into_host(target: &mut Host, defaults: &Defaults) {
    merge_option(&mut target.hostname, &defaults.hostname);
    merge_option(&mut target.port, &defaults.port);
    merge_option(&mut target.username, &defaults.username);
    merge_option(&mut target.password, &defaults.password);
    merge_option(&mut target.platform, &defaults.platform);
    merge_data(&mut target.data, &defaults.data);
    merge_connection_options(&mut target.connection_options, &defaults.connection_options);
}

fn merge_group_into_host(target: &mut Host, group: &Group) {
    merge_option(&mut target.hostname, &group.hostname);
    merge_option(&mut target.port, &group.port);
    merge_option(&mut target.username, &group.username);
    merge_option(&mut target.password, &group.password);
    merge_option(&mut target.platform, &group.platform);
    merge_data(&mut target.data, &group.data);
    merge_connection_options(&mut target.connection_options, &group.connection_options);
}

fn merge_host_into_host(target: &mut Host, host: &Host) {
    merge_option(&mut target.hostname, &host.hostname);
    merge_option(&mut target.port, &host.port);
    merge_option(&mut target.username, &host.username);
    merge_option(&mut target.password, &host.password);
    merge_option(&mut target.platform, &host.platform);
    merge_data(&mut target.data, &host.data);
    merge_connection_options(&mut target.connection_options, &host.connection_options);
    if host.groups.is_some() {
        target.groups = host.groups.clone();
    }
}

fn merge_group_into_group(target: &mut Group, group: &Group) {
    merge_option(&mut target.hostname, &group.hostname);
    merge_option(&mut target.port, &group.port);
    merge_option(&mut target.username, &group.username);
    merge_option(&mut target.password, &group.password);
    merge_option(&mut target.platform, &group.platform);
    merge_data(&mut target.data, &group.data);
    merge_connection_options(&mut target.connection_options, &group.connection_options);
    if group.groups.is_some() {
        target.groups = group.groups.clone();
    }
}

fn merge_option<T: Clone>(target: &mut Option<T>, source: &Option<T>) {
    if let Some(value) = source.as_ref() {
        *target = Some(value.clone());
    }
}

/// Merges data from a source `Data` option into a target `Data` option.
///
/// This function performs intelligent merging of JSON data structures with the following behavior:
///
/// 1. **Object Merging**: When both target and source contain JSON objects, the function merges
///    their key-value pairs. Keys present in the source object will overwrite corresponding keys
///    in the target object, while keys unique to either object are preserved.
///
/// 2. **Non-Object Replacement**: When the target is not a JSON object (e.g., array, string, number)
///    but the source is an object, the entire target is replaced with the source object rather than
///    attempting to merge incompatible types.
///
/// 3. **Initialization**: When the target is `None` and the source contains data, the target is
///    initialized with a clone of the source data.
///
/// 4. **No-Op Cases**: When the source is `None`, the target remains unchanged regardless of its state.
///
/// This function is used internally during host and group resolution to merge data fields from
/// defaults, parent groups, and host-specific configurations in the proper priority order.
///
/// # Parameters
///
/// * `target` - A mutable reference to an optional `Data` value that will be modified in place.
///   This represents the destination for the merge operation. If `None`, it may be initialized
///   with the source data. If `Some`, its contents may be merged with or replaced by the source.
///
/// * `source` - A reference to an optional `Data` value containing the data to merge into the target.
///   This represents the source of new or overriding values. If `None`, no changes are made to the
///   target. If `Some`, its contents are merged into or replace the target based on their types.
///
/// # Examples
///
/// See the unit test `merge_data_merges_objects_and_replaces_non_objects` in the unit tests
/// for a comprehensive example of how this function is used in practice during inventory resolution.
fn merge_data(target: &mut Option<Data>, source: &Option<Data>) {
    match (target.as_mut(), source.as_ref()) {
        (Some(target_data), Some(source_data)) => {
            if let (Some(target_obj), Some(source_obj)) =
                (target_data.as_object_mut(), source_data.as_object())
            {
                for (key, value) in source_obj {
                    target_obj.insert(key.clone(), value.clone());
                }
            } else {
                *target = Some(source_data.clone());
            }
        }
        (None, Some(source_data)) => {
            *target = Some(source_data.clone());
        }
        _ => {}
    }
}

fn merge_connection_options(
    target: &mut Option<CustomTreeMap<ConnectionOptions>>,
    source: &Option<CustomTreeMap<ConnectionOptions>>,
) {
    let Some(source_map) = source.as_ref() else {
        return;
    };

    if target.is_none() {
        *target = Some(CustomTreeMap::new());
    }

    let target_map = target.as_mut().expect("target map initialized");
    for (name, options) in source_map.iter() {
        if let Some(existing) = target_map.get_mut(name.as_str()) {
            merge_connection_options_fields(existing, options);
        } else {
            target_map.insert(name.as_str(), options.clone());
        }
    }
}

fn merge_connection_options_fields(target: &mut ConnectionOptions, source: &ConnectionOptions) {
    if source.hostname.is_some() {
        target.hostname = source.hostname.clone();
    }
    if source.port.is_some() {
        target.port = source.port;
    }
    if source.username.is_some() {
        target.username = source.username.clone();
    }
    if source.password.is_some() {
        target.password = source.password.clone();
    }
    if source.platform.is_some() {
        target.platform = source.platform.clone();
    }
    if source.extras.is_some() {
        target.extras = source.extras.clone();
    }
}

/// A view over the hosts collection in an inventory that applies transform functions on access.
///
/// This struct provides a read-only view of the hosts stored in an `Inventory`. When accessing
/// individual hosts through this view, any configured transform function is automatically applied.
/// The view caches transformed results to improve performance on subsequent accesses to the same host.
///
/// The view does not own the inventory data; it holds a reference to the parent `Inventory` and
/// provides methods to iterate over hosts, look up hosts by name, and query collection metadata.
///
/// # Lifetime
///
/// * `'a` - The lifetime of the reference to the parent `Inventory`. The view cannot outlive
///   the inventory it references.
///
/// # Examples
///
/// ```
/// # use genja_core::inventory::{Inventory, Host, Hosts, BaseBuilderHost};
/// let mut hosts = Hosts::new();
/// hosts.add_host("router1", Host::builder().hostname("10.0.0.1").build());
/// let inventory = Inventory::builder().hosts(hosts).build();
///
/// let hosts_view = inventory.hosts();
/// assert_eq!(hosts_view.len(), 1);
///
/// if let Some(host) = hosts_view.get("router1") {
///     assert_eq!(host.hostname(), Some("10.0.0.1"));
/// }
///
/// for (name, host) in hosts_view.iter() {
///     println!("Host: {}", name);
/// }
/// ```
pub struct HostsView<'a> {
    inventory: &'a Inventory,
}

impl<'a> HostsView<'a> {
    pub fn len(&self) -> usize {
        self.inventory.hosts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inventory.hosts.is_empty()
    }

    pub fn keys(&self) -> impl Iterator<Item = &'a NatString> {
        self.inventory.hosts.keys()
    }

    pub fn get(&self, name: &str) -> Option<Host> {
        let key = NatString::new(name.to_string());
        if let Some(entry) = self.inventory.host_cache.get(&key) {
            return Some(entry.value().clone());
        }

        self.inventory
            .hosts
            .get(name)
            .map(|host| self.inventory.cached_host_value(&key, host))
    }

    pub fn iter(&self) -> impl Iterator<Item = (&'a NatString, Host)> {
        self.inventory
            .hosts
            .iter()
            .map(|(id, host)| (id, self.inventory.cached_host_value(id, host)))
    }
}

/// A view over the groups collection in an inventory that applies transform functions on access.
///
/// This struct provides a read-only view of the groups stored in an `Inventory`. When accessing
/// individual groups through this view, any configured transform function is automatically applied.
/// The view caches transformed results to improve performance on subsequent accesses to the same group.
///
/// The view does not own the inventory data; it holds references to both the parent `Inventory` and
/// the underlying `Groups` collection. It provides methods to iterate over groups, look up groups by
/// name, and query collection metadata.
///
/// # Lifetime
///
/// * `'a` - The lifetime of the references to the parent `Inventory` and `Groups` collection. The view
///   cannot outlive either the inventory or groups it references.
///
/// # Examples
///
/// ```
/// # use genja_core::inventory::{Inventory, Group, Groups, BaseBuilderHost};
/// let mut groups = Groups::new();
/// groups.add_group("core", Group::builder().platform("linux").build());
/// let inventory = Inventory::builder().groups(groups).build();
///
/// if let Some(groups_view) = inventory.groups() {
///     assert_eq!(groups_view.len(), 1);
///
///     if let Some(group) = groups_view.get("core") {
///         assert_eq!(group.platform(), Some("linux"));
///     }
///
///     for (name, group) in groups_view.iter() {
///         println!("Group: {}", name);
///     }
/// }
/// ```
pub struct GroupsView<'a> {
    inventory: &'a Inventory,
    groups: &'a Groups,
}

impl<'a> GroupsView<'a> {
    pub fn len(&self) -> usize {
        self.groups.len()
    }

    pub fn is_empty(&self) -> bool {
        self.groups.is_empty()
    }

    pub fn keys(&self) -> impl Iterator<Item = &'a NatString> {
        self.groups.keys()
    }

    pub fn get(&self, name: &str) -> Option<Group> {
        let key = NatString::new(name.to_string());
        if let Some(entry) = self.inventory.group_cache.get(&key) {
            return Some(entry.value().clone());
        }

        self.groups
            .get(name)
            .map(|group| self.inventory.cached_group_value(&key, group))
    }

    pub fn iter(&self) -> impl Iterator<Item = (&'a NatString, Group)> {
        self.groups
            .iter()
            .map(|(id, group)| (id, self.inventory.cached_group_value(id, group)))
    }
}

impl Default for Inventory {
    fn default() -> Self {
        Inventory {
            hosts: Hosts::new(),
            groups: None,
            defaults: None,
            transform_function: None,
            transform_function_options: None,
            connections: Arc::new(ConnectionManager::default()),
            host_cache: DashMap::new(),
            group_cache: DashMap::new(),
            resolved_host_cache: DashMap::new(),
            resolved_params_cache: DashMap::new(),
        }
    }
}
/// Builder for constructing `Inventory` instances with custom configuration.
///
/// This builder provides a fluent interface for creating `Inventory` objects
/// with optional hosts, groups, defaults, and transform settings. Fields that
/// are not explicitly set will use their default values when `build()` is called.
///
/// # Fields
///
/// * `hosts` - Optional hosts map. When set to `Some(hosts)`, the provided hosts
///   are used. When `None`, an empty `Hosts` map is used.
/// * `groups` - Optional groups map. When set, the provided groups are used.
/// * `defaults` - Optional defaults object. When set, the provided defaults are used.
/// * `transform_function` - Optional transform function applied lazily on access.
/// * `transform_function_options` - Optional JSON options passed to the transform.
/// * `connections` - Optional connection manager. When `None`, a default
///   `ConnectionManager` is created.
///
/// # Examples
///
/// ```
/// use genja_core::inventory::{Host, Hosts, Inventory, BaseBuilderHost};
///
/// let mut hosts = Hosts::new();
/// let host = Host::builder().hostname("10.0.0.1").build();
/// hosts.add_host("router1", host);
///
/// let inventory = Inventory::builder()
///     .hosts(hosts)
///     .build();
/// ```
pub struct InventoryBuilder {
    pub hosts: Option<Hosts>,
    pub groups: Option<Groups>,
    pub defaults: Option<Defaults>,
    pub transform_function: Option<TransformFunction>,
    pub transform_function_options: Option<TransformFunctionOptions>,
    pub connections: Option<Arc<ConnectionManager>>,
}

impl InventoryBuilder {
    pub fn new() -> InventoryBuilder {
        InventoryBuilder {
            hosts: None,
            groups: None,
            defaults: None,
            transform_function: None,
            transform_function_options: None,
            connections: None,
        }
    }

    pub fn hosts(mut self, hosts: Hosts) -> Self {
        self.hosts = Some(hosts);
        self
    }

    pub fn groups(mut self, groups: Groups) -> Self {
        self.groups = Some(groups);
        self
    }

    pub fn defaults(mut self, defaults: Defaults) -> Self {
        self.defaults = Some(defaults);
        self
    }

    pub fn transform_function(mut self, transform: TransformFunction) -> Self {
        self.transform_function = Some(transform);
        self
    }

    pub fn transform_function_options(mut self, options: TransformFunctionOptions) -> Self {
        self.transform_function_options = Some(options);
        self
    }

    pub fn connections(mut self, connections: ConnectionManager) -> Self {
        self.connections = Some(Arc::new(connections));
        self
    }

    pub fn build(self) -> Inventory {
        Inventory {
            hosts: self.hosts.unwrap_or_default(),
            groups: self.groups,
            defaults: self.defaults,
            transform_function: self.transform_function,
            transform_function_options: self.transform_function_options,
            connections: self
                .connections
                .unwrap_or_else(|| Arc::new(ConnectionManager::default())),
            host_cache: DashMap::new(),
            group_cache: DashMap::new(),
            resolved_host_cache: DashMap::new(),
            resolved_params_cache: DashMap::new(),
        }
    }
}

impl Default for InventoryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn create_dummy_hosts() -> Result<Hosts, std::io::Error> {
        let mut hosts = Hosts(CustomTreeMap::new());
        // hosts.insert("hosts".to_string(), CustomTreeMap::new());
        for i in 1..=10 {
            let name = format!("host{}.example.com", i);
            let mut groups = ParentGroups::new();
            groups.push("cisco".to_string());
            let host = Host::builder()
                .hostname(&name)
                .port(2200 + i as u16)
                .username(&format!("user{}", i))
                .password(&format!("password{}", i))
                .platform(if i % 2 == 0 { "linux" } else { "windows" })
                .data(Data(serde_json::json!(vec![format!(
                    "data for host {}",
                    i
                )])))
                .groups(groups)
                .connection_options(String::from("Cisco"), ConnectionOptions::builder().build())
                .build();
            hosts.insert(name, host);
        }

        Ok(hosts)
    }

    #[test]
    fn test_host_new() {
        let host = Host::new();
        assert_eq!(host.hostname, None);
        assert_eq!(host.port, None);
        assert_eq!(host.username, None);
        assert_eq!(host.password, None);
        assert_eq!(host.platform, None);
        assert_eq!(host.groups, None);
        assert_eq!(host.data, None);
        assert_eq!(host.connection_options, None);
    }

    #[test]
    fn test_hosts_new() {
        let mut hosts = Hosts::new();

        // Add 10 hosts to the hosts map with dummy data
        for i in 1..=10 {
            let name = format!("host{}.example.com", i);
            let host = Host::builder()
                .hostname(&name)
                .port(2200 + i as u16)
                .username(&format!("user{}", i))
                .password(&format!("password{}", i))
                .platform(if i % 2 == 0 { "linux" } else { "windows" })
                .data(Data(serde_json::json!(vec![format!(
                    "data for host {}",
                    i
                )])))
                .connection_options(
                    String::from("Juniper"),
                    ConnectionOptions::builder().build(),
                )
                .build();

            hosts.add_host(name, host);
        }
        assert_eq!(hosts.len(), 10);
    }

    #[test]
    fn test_build_hosts() {
        let hosts = create_dummy_hosts();
        assert_eq!(hosts.unwrap().len(), 10);
    }

    #[test]
    fn test_connection_options_builder() {
        let extras = Extras::new(serde_json::json!({ "env": "lab", "tier": "core" }));

        let options = ConnectionOptions::builder()
            .hostname("192.0.2.55")
            .port(830)
            .username("netconf")
            .password("secret")
            .platform("iosxr")
            .extras(extras.clone())
            .build();

        assert_eq!(options.hostname(), Some("192.0.2.55"));
        assert_eq!(options.port(), Some(830));
        assert_eq!(options.username(), Some("netconf"));
        assert_eq!(options.password(), Some("secret"));
        assert_eq!(options.platform(), Some("iosxr"));
        assert_eq!(options.extras(), Some(&extras));
    }

    #[test]
    fn test_parent_groups() {
        let groups = vec![
            "cisco".to_string(),
            "Juniper".to_string(),
            "arista".to_string(),
        ];
        let serialized = serde_json::to_string(&groups).unwrap();
        assert_eq!(serialized, "[\"cisco\",\"Juniper\",\"arista\"]");
        let mut deserialized: ParentGroups = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.sort(), ParentGroups(groups).sort());
    }

    #[test]
    fn test_parent_groups_deduplication() {
        // Test that duplicate groups are removed during deserialization
        let groups_with_duplicates = vec![
            "cisco".to_string(),
            "juniper".to_string(),
            "cisco".to_string(), // duplicate
            "arista".to_string(),
            "juniper".to_string(), // duplicate
            "cisco".to_string(),   // duplicate
        ];

        let serialized = serde_json::to_string(&groups_with_duplicates).unwrap();
        let deserialized: ParentGroups = serde_json::from_str(&serialized).unwrap();

        // Should only contain unique values in order of first occurrence
        assert_eq!(deserialized.len(), 3);
        assert_eq!(deserialized[0], "cisco");
        assert_eq!(deserialized[1], "juniper");
        assert_eq!(deserialized[2], "arista");
    }

    #[test]
    fn test_parent_groups_preserves_order() {
        // Test that the order of first occurrence is preserved
        let groups = vec![
            "zebra".to_string(),
            "apple".to_string(),
            "zebra".to_string(), // duplicate
            "banana".to_string(),
        ];

        let serialized = serde_json::to_string(&groups).unwrap();
        let deserialized: ParentGroups = serde_json::from_str(&serialized).unwrap();

        // Should preserve order of first occurrence
        assert_eq!(deserialized.len(), 3);
        assert_eq!(deserialized[0], "zebra");
        assert_eq!(deserialized[1], "apple");
        assert_eq!(deserialized[2], "banana");
    }

    /// Tests the ParentGroups deserialization with an error.
    ///
    /// The error message is expected to be "Groups should be an array of strings for use with `ParentGroups`"
    #[test]
    fn test_parent_groups_err() {
        let name = String::from("name");
        let deserialized: Result<ParentGroups, serde_json::Error> = serde_json::from_str(&name);
        match deserialized {
            Ok(_) => panic!("Expected an error, but deserialization succeeded"),
            Err(err) => {
                assert_eq!(
                    err.to_string(),
                    "Groups should be an array of strings for use with `ParentGroups`"
                );
            }
        }
    }

    #[test]
    fn test_inventory_builder_defaults() {
        let inventory = Inventory::builder().build();
        assert_eq!(inventory.hosts().len(), 0);
        assert!(inventory.groups().is_none());
        assert!(inventory.defaults().is_none());
        assert!(inventory.transform_function_options().is_none());
    }

    #[test]
    fn resolve_connection_params_uses_cache() {
        let defaults: Defaults = serde_json::from_value(serde_json::json!({
            "connection_options": {
                "netconf": {
                    "hostname": "default-netconf",
                    "port": 2001
                }
            }
        }))
        .expect("defaults should deserialize");

        let mut hosts = Hosts::new();
        hosts.add_host("router1.lab", Host::builder().hostname("host-host").build());

        let inventory = Inventory::builder()
            .hosts(hosts)
            .defaults(defaults)
            .connections(ConnectionManager::default())
            .build();

        assert_eq!(inventory.resolved_host_cache_len(), 0);
        assert_eq!(inventory.resolved_params_cache_len(), 0);
        let _ = inventory
            .resolve_connection_params("router1.lab", "netconf")
            .expect("resolved params should exist");
        assert_eq!(inventory.resolved_host_cache_len(), 1);
        assert_eq!(inventory.resolved_params_cache_len(), 1);
        let _ = inventory
            .resolve_connection_params("router1.lab", "netconf")
            .expect("resolved params should exist");
        assert_eq!(inventory.resolved_host_cache_len(), 1);
        assert_eq!(inventory.resolved_params_cache_len(), 1);
    }

    /// Tests the internal group resolution logic for proper merging and cycle detection.
    ///
    /// This test verifies two critical aspects of the `resolve_group_internal` method:
    ///
    /// 1. **Hierarchical Merging**: Tests that group inheritance correctly merges settings
    ///    from parent groups in the proper order. Creates a three-level hierarchy (a -> b -> c)
    ///    and verifies that settings cascade properly, with child groups overriding parent
    ///    settings as expected.
    ///
    /// 2. **Cycle Detection**: Tests that circular group references are detected and handled
    ///    gracefully without causing infinite loops. Creates a two-group cycle (cycle-a -> cycle-b -> cycle-a)
    ///    and verifies that resolution completes successfully without hanging.
    ///
    /// # Test Structure
    ///
    /// The test is divided into two parts:
    ///
    /// ## Part 1: Hierarchical Merging
    /// - Creates groups with inheritance chain: a (username) -> b (hostname, port) -> c (port, platform)
    /// - Resolves group "a" and verifies all inherited settings are present
    /// - Validates that closer ancestors override more distant ones (b's port overrides c's port)
    ///
    /// ## Part 2: Cycle Detection
    /// - Creates a circular reference: cycle-a -> cycle-b -> cycle-a
    /// - Attempts to resolve cycle-a
    /// - Verifies that resolution completes without infinite loop
    /// - Confirms that the resolved group maintains its structure
    ///
    /// # Assertions
    ///
    /// For hierarchical merging:
    /// - `hostname` should be "b-host" (from group b)
    /// - `port` should be 2002 (from group b, overriding c's 2001)
    /// - `platform` should be "linux" (from group c)
    /// - `username` should be "a-user" (from group a)
    ///
    /// For cycle detection:
    /// - Resolution should complete without panic or timeout
    /// - Resolved group should maintain its groups field
    #[test]
    fn resolve_group_internal_merges_and_detects_cycle() {
        let mut groups = Groups::new();

        let mut a_parents = ParentGroups::new();
        a_parents.push("b".to_string());
        let group_a = Group::builder()
            .username("a-user")
            .groups(a_parents)
            .build();

        let mut b_parents = ParentGroups::new();
        b_parents.push("c".to_string());
        let group_b = Group::builder()
            .hostname("b-host")
            .port(2002)
            .groups(b_parents)
            .build();

        let group_c = Group::builder().port(2001).platform("linux").build();

        groups.add_group("a", group_a);
        groups.add_group("b", group_b);
        groups.add_group("c", group_c);

        let inventory = Inventory::builder().groups(groups).build();

        let mut stack = std::collections::HashSet::new();
        let mut memo = std::collections::HashMap::new();
        let resolved = inventory
            .resolve_group_internal("a", &mut stack, &mut memo)
            .expect("group should resolve");

        assert_eq!(resolved.hostname(), Some("b-host"));
        assert_eq!(resolved.port(), Some(2002));
        assert_eq!(resolved.platform(), Some("linux"));
        assert_eq!(resolved.username(), Some("a-user"));

        let mut cycle_groups = Groups::new();
        let mut c1 = ParentGroups::new();
        c1.push("cycle-b".to_string());
        let mut c2 = ParentGroups::new();
        c2.push("cycle-a".to_string());
        cycle_groups.add_group("cycle-a", Group::builder().groups(c1).build());
        cycle_groups.add_group("cycle-b", Group::builder().groups(c2).build());

        let cycle_inventory = Inventory::builder().groups(cycle_groups).build();
        let mut cycle_stack = std::collections::HashSet::new();
        let mut cycle_memo = std::collections::HashMap::new();
        let cycle_resolved = cycle_inventory
            .resolve_group_internal("cycle-a", &mut cycle_stack, &mut cycle_memo)
            .expect("cycle should not infinite loop");
        assert!(cycle_resolved.groups().is_some());
    }

    /// Tests that transform functions are applied to hosts and results are cached in HostsView.
    ///
    /// This test verifies two critical behaviors of the `HostsView` implementation:
    ///
    /// 1. **Transform Application**: Confirms that the configured transform function is
    ///    applied when accessing hosts through the view, modifying host properties as expected.
    ///
    /// 2. **Caching Behavior**: Validates that transformed results are cached, ensuring the
    ///    transform function is called only once per unique host regardless of how many times
    ///    that host is accessed.
    ///
    /// The test creates a transform function that increments each host's port number by 1
    /// and tracks the number of times it's called. It then verifies that:
    /// - Multiple accesses to the same host return the same transformed result
    /// - The transform is only called once per host (cached after first access)
    /// - Iteration over the view also uses the cache when available
    ///
    /// # Test Structure
    ///
    /// 1. Creates a transform function that increments port numbers and counts invocations
    /// 2. Builds an inventory with two hosts (ports 10 and 20)
    /// 3. Accesses the same host twice via `get()` and verifies caching
    /// 4. Iterates over all hosts and verifies the transform was called exactly twice total
    ///
    /// # Assertions
    ///
    /// - First access to "h1" returns port 11 (10 + 1)
    /// - Second access to "h1" returns port 11 (cached, transform not called again)
    /// - Transform function called exactly once after two accesses to same host
    /// - Transform function called exactly twice total after iterating all hosts
    #[test]
    fn transform_and_cache_hosts_view() {
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_clone = Arc::clone(&calls);
        let transform = TransformFunction::new(move |host, _| {
            calls_clone.fetch_add(1, Ordering::SeqCst);
            host.to_builder().port(host.port().unwrap_or(0) + 1).build()
        });

        let mut hosts = Hosts::new();
        hosts.add_host("h1", Host::builder().port(10).build());
        hosts.add_host("h2", Host::builder().port(20).build());

        let inventory = Inventory::builder()
            .hosts(hosts)
            .transform_function(transform)
            .build();

        let view = inventory.hosts();
        let first = view.get("h1").expect("host exists");
        let second = view.get("h1").expect("host exists");
        assert_eq!(first.port(), Some(11));
        assert_eq!(second.port(), Some(11));
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        // Iterate over all hosts which triggers the transform function again
        // as h2 has not been accessed yet. The transform function should be called twice.
        let _ = view.iter().collect::<Vec<_>>();
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    /// Tests the `merge_data` function's behavior when merging JSON objects and replacing non-objects.
    ///
    /// This test verifies two critical behaviors of the `merge_data` function:
    ///
    /// 1. **Object Merging**: When both target and source are JSON objects, the function should
    ///    merge their key-value pairs. Keys present in both objects should be overwritten with
    ///    the source's values, while keys unique to either object should be preserved.
    ///
    /// 2. **Non-Object Replacement**: When the target is not a JSON object (e.g., an array) but
    ///    the source is an object, the entire target should be replaced with the source object
    ///    rather than attempting to merge incompatible types.
    ///
    /// # Test Structure
    ///
    /// The test is divided into two parts:
    ///
    /// ## Part 1: Object Merging
    /// - Creates a target object with keys "a" and "b"
    /// - Creates a source object with keys "b" and "c"
    /// - Merges source into target
    /// - Verifies that:
    ///   - Key "a" retains its original value (1)
    ///   - Key "b" is overwritten with source's value (3)
    ///   - Key "c" is added from source (4)
    ///
    /// ## Part 2: Non-Object Replacement
    /// - Creates a target that is a JSON array
    /// - Creates a source that is a JSON object
    /// - Merges source into target
    /// - Verifies that the target is completely replaced and is now an object
    ///
    /// # Assertions
    ///
    /// For object merging:
    /// - `target["a"]` should equal 1 (preserved from original)
    /// - `target["b"]` should equal 3 (overwritten by source)
    /// - `target["c"]` should equal 4 (added from source)
    ///
    /// For non-object replacement:
    /// - Target should be a JSON object after merge (not an array)
    #[test]
    fn merge_data_merges_objects_and_replaces_non_objects() {
        let mut target = Some(Data::new(serde_json::json!({
            "a": 1,
            "b": 2
        })));
        let source = Some(Data::new(serde_json::json!({
            "b": 3,
            "c": 4
        })));

        merge_data(&mut target, &source);
        let target_obj = target
            .as_ref()
            .and_then(|data| data.as_object())
            .expect("target should be object");
        assert_eq!(target_obj.get("a").and_then(|v| v.as_i64()), Some(1));
        assert_eq!(target_obj.get("b").and_then(|v| v.as_i64()), Some(3));
        assert_eq!(target_obj.get("c").and_then(|v| v.as_i64()), Some(4));

        let mut non_object_target = Some(Data::new(serde_json::json!(["x", "y"])));
        let object_source = Some(Data::new(serde_json::json!({ "k": "v" })));
        merge_data(&mut non_object_target, &object_source);
        assert!(non_object_target
            .as_ref()
            .and_then(|data| data.as_object())
            .is_some());
    }

    #[test]
    fn merge_connection_options_fields_overrides_only_present_values() {
        let mut target = ConnectionOptions::builder()
            .hostname("primary")
            .port(22)
            .username("user")
            .platform("linux")
            .build();

        let source = ConnectionOptions::builder()
            .port(2222)
            .password("secret")
            .extras(Extras::new(serde_json::json!({ "tier": "core" })))
            .build();

        merge_connection_options_fields(&mut target, &source);

        assert_eq!(target.hostname(), Some("primary"));
        assert_eq!(target.port(), Some(2222));
        assert_eq!(target.username(), Some("user"));
        assert_eq!(target.password(), Some("secret"));
        assert_eq!(target.platform(), Some("linux"));
        assert_eq!(
            target
                .extras()
                .and_then(|v| v.get("tier"))
                .and_then(|v| v.as_str()),
            Some("core")
        );
    }

    #[test]
    fn base_methods_schema_returns_json() {
        let schema = Host::schema();
        let parsed: serde_json::Value =
            serde_json::from_str(&schema).expect("schema should be valid JSON");
        assert!(parsed.get("$schema").is_some());
    }

    #[test]
    fn connection_options_default_and_to_builder_round_trip() {
        let options = ConnectionOptions::default();
        assert_eq!(options.hostname(), None);
        assert_eq!(options.port(), None);
        assert_eq!(options.username(), None);
        assert_eq!(options.password(), None);
        assert_eq!(options.platform(), None);
        assert_eq!(options.extras(), None);

        let rebuilt = options.to_builder().build();
        assert_eq!(options, rebuilt);
    }

    #[test]
    fn defaults_builder_to_builder_and_accessors() {
        let defaults = Defaults::builder()
            .hostname("default-host")
            .port(22)
            .username("admin")
            .password("secret")
            .platform("linux")
            .data(Data::new(serde_json::json!({"env": "lab"})))
            .build();

        assert!(!defaults.is_empty());
        assert_eq!(defaults.hostname(), Some("default-host"));
        assert_eq!(defaults.port(), Some(22));
        assert_eq!(defaults.username(), Some("admin"));
        assert_eq!(defaults.password(), Some("secret"));
        assert_eq!(defaults.platform(), Some("linux"));
        assert!(defaults.data().is_some());

        let modified = defaults.to_builder().port(2222).build();
        assert_eq!(modified.port(), Some(2222));
        assert_eq!(modified.username(), Some("admin"));
    }

    #[test]
    fn host_and_group_to_builder_preserve_connection_options() {
        let opts1 = ConnectionOptions::builder().port(22).build();
        let opts2 = ConnectionOptions::builder().port(830).build();

        let host = Host::builder()
            .hostname("h1")
            .connection_options("ssh", opts1.clone())
            .connection_options("netconf", opts2.clone())
            .build();
        let host_round = host.to_builder().build();
        assert_eq!(
            host_round
                .connection_options()
                .and_then(|m| m.get("ssh"))
                .and_then(|o| o.port()),
            Some(22)
        );

        let group = Group::builder()
            .hostname("g1")
            .connection_options("ssh", opts1)
            .build();
        let group_round = group.to_builder().build();
        assert_eq!(
            group_round
                .connection_options()
                .and_then(|m| m.get("ssh"))
                .and_then(|o| o.port()),
            Some(22)
        );
    }

    #[test]
    fn groups_default_is_empty() {
        let groups = Groups::default();
        assert!(groups.is_empty());
    }

    #[test]
    fn transform_function_group_and_defaults_methods() {
        struct CountTransform {
            group_calls: Arc<AtomicUsize>,
            defaults_calls: Arc<AtomicUsize>,
        }

        impl Transform for CountTransform {
            fn transform_group(
                &self,
                group: &Group,
                _options: Option<&TransformFunctionOptions>,
            ) -> Group {
                self.group_calls.fetch_add(1, Ordering::SeqCst);
                group.to_builder().port(443).build()
            }

            fn transform_defaults(
                &self,
                defaults: &Defaults,
                _options: Option<&TransformFunctionOptions>,
            ) -> Defaults {
                self.defaults_calls.fetch_add(1, Ordering::SeqCst);
                defaults.to_builder().username("admin").build()
            }
        }

        let group_calls = Arc::new(AtomicUsize::new(0));
        let defaults_calls = Arc::new(AtomicUsize::new(0));
        let transform = TransformFunction::new_full(CountTransform {
            group_calls: Arc::clone(&group_calls),
            defaults_calls: Arc::clone(&defaults_calls),
        });

        let group = Group::builder().platform("linux").build();
        let defaults = Defaults::builder().port(22).build();

        let transformed_group = transform.transform_group(&group, None);
        let transformed_defaults = transform.transform_defaults(&defaults, None);

        assert_eq!(transformed_group.port(), Some(443));
        assert_eq!(transformed_defaults.username(), Some("admin"));
        assert_eq!(group_calls.load(Ordering::SeqCst), 1);
        assert_eq!(defaults_calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn transform_function_options_new() {
        let options = TransformFunctionOptions::new(serde_json::json!({"k": "v"}));
        assert_eq!(options.get("k").and_then(|v| v.as_str()), Some("v"));
    }

    #[test]
    fn connection_manager_insert_get_and_close() {
        #[derive(Debug)]
        struct TestConnection {
            closes: Arc<AtomicUsize>,
            key: ConnectionKey,
        }

        impl Connection for TestConnection {
            fn create(&self, key: &ConnectionKey) -> Box<dyn Connection> {
                Box::new(TestConnection {
                    closes: Arc::clone(&self.closes),
                    key: key.clone(),
                })
            }

            fn is_alive(&self) -> bool {
                true
            }

            fn open(&mut self, _params: &ResolvedConnectionParams) -> Result<(), String> {
                Ok(())
            }

            fn close(&mut self) -> ConnectionKey {
                self.closes.fetch_add(1, Ordering::SeqCst);
                self.key.clone()
            }
        }

        let manager = ConnectionManager::default();
        let closes = Arc::new(AtomicUsize::new(0));
        let key = ConnectionKey::new("h1", "ssh");
        let connection = Arc::new(Mutex::new(TestConnection {
            closes: Arc::clone(&closes),
            key: key.clone(),
        })) as Arc<Mutex<dyn Connection>>;

        manager.insert(key.clone(), Arc::clone(&connection));
        let fetched = manager.get(&key).expect("connection should exist");
        assert!(Arc::ptr_eq(&connection, &fetched));

        manager.close_connection(&key);
        assert_eq!(closes.load(Ordering::SeqCst), 1);
        assert!(manager.get(&key).is_none());

        let key2 = ConnectionKey::new("h2", "netconf");
        let connection2 = Arc::new(Mutex::new(TestConnection {
            closes: Arc::clone(&closes),
            key: key2.clone(),
        })) as Arc<Mutex<dyn Connection>>;
        manager.insert(key.clone(), connection);
        manager.insert(key2.clone(), connection2);
        manager.close_all_connections();
        assert_eq!(closes.load(Ordering::SeqCst), 3);
        assert!(manager.get(&key).is_none());
        assert!(manager.get(&key2).is_none());
    }

    #[test]
    fn inventory_connections_and_resolve_host_cache_hit() {
        let mut hosts = Hosts::new();
        hosts.add_host("router1", Host::builder().hostname("10.0.0.1").build());
        let inventory = Inventory::builder().hosts(hosts).build();

        let _ = inventory.connections();
        assert_eq!(inventory.resolved_host_cache_len(), 0);
        let _ = inventory.resolve_host("router1");
        assert_eq!(inventory.resolved_host_cache_len(), 1);
        let _ = inventory.resolve_host("router1");
        assert_eq!(inventory.resolved_host_cache_len(), 1);
    }

    #[test]
    fn groups_view_applies_transform_and_accessors_work() {
        let group_calls = Arc::new(AtomicUsize::new(0));
        let group_calls_clone = Arc::clone(&group_calls);
        struct GroupOnlyTransform {
            calls: Arc<AtomicUsize>,
        }

        impl Transform for GroupOnlyTransform {
            fn transform_group(
                &self,
                group: &Group,
                _options: Option<&TransformFunctionOptions>,
            ) -> Group {
                self.calls.fetch_add(1, Ordering::SeqCst);
                group.to_builder().port(1234).build()
            }
        }

        let transform = TransformFunction::new_full(GroupOnlyTransform {
            calls: group_calls_clone,
        });

        let mut groups = Groups::new();
        groups.add_group("core", Group::builder().platform("linux").build());
        groups.add_group("edge", Group::builder().platform("linux").build());

        let inventory = Inventory::builder()
            .groups(groups)
            .transform_function(transform)
            .build();

        let view = inventory.groups().expect("groups view exists");
        assert_eq!(view.len(), 2);
        assert!(!view.is_empty());
        assert_eq!(view.keys().count(), 2);

        let core = view.get("core").expect("group exists");
        assert_eq!(core.port(), Some(1234));

        let _ = view.iter().collect::<Vec<_>>();
        assert_eq!(group_calls.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn merge_connection_options_merges_maps_and_overrides_fields() {
        let mut target: Option<CustomTreeMap<ConnectionOptions>> = Some(CustomTreeMap::new());
        let mut source: Option<CustomTreeMap<ConnectionOptions>> = Some(CustomTreeMap::new());

        if let Some(map) = target.as_mut() {
            map.insert(
                "ssh",
                ConnectionOptions::builder().hostname("t").port(22).build(),
            );
        }
        if let Some(map) = source.as_mut() {
            map.insert(
                "ssh",
                ConnectionOptions::builder()
                    .port(2222)
                    .username("u")
                    .build(),
            );
            map.insert("netconf", ConnectionOptions::builder().port(830).build());
        }

        merge_connection_options(&mut target, &source);
        let merged = target.expect("target should exist");
        let ssh = merged.get("ssh").expect("ssh should exist");
        assert_eq!(ssh.hostname(), Some("t"));
        assert_eq!(ssh.port(), Some(2222));
        assert_eq!(ssh.username(), Some("u"));
        assert!(merged.get("netconf").is_some());
    }

    #[test]
    fn resolve_host_applies_defaults_groups_then_host_in_order() {
        let defaults = Defaults::builder().port(10).build();

        let group_a = Group::builder().port(20).build();
        let group_b = Group::builder().port(30).build();

        let mut groups = Groups::new();
        groups.add_group("a", group_a);
        groups.add_group("b", group_b);

        let mut hosts = Hosts::new();
        let mut parents = ParentGroups::new();
        parents.push("a".to_string());
        parents.push("b".to_string());
        let host = Host::builder().port(40).groups(parents).build();
        hosts.add_host("h1", host);

        let inventory = Inventory::builder()
            .hosts(hosts)
            .groups(groups)
            .defaults(defaults)
            .build();

        let resolved = inventory.resolve_host("h1").expect("host should resolve");
        assert_eq!(resolved.port(), Some(40));
    }

    #[test]
    fn resolve_connection_params_applies_priority_order() {
        let defaults = Defaults::builder()
            .port(10)
            .connection_options(
                "ssh",
                ConnectionOptions::builder().port(11).build(),
            )
            .build();

        let group = Group::builder()
            .port(20)
            .connection_options(
                "ssh",
                ConnectionOptions::builder().port(21).build(),
            )
            .build();

        let mut groups = Groups::new();
        groups.add_group("g1", group);

        let mut parents = ParentGroups::new();
        parents.push("g1".to_string());
        let host = Host::builder()
            .port(30)
            .groups(parents)
            .connection_options(
                "ssh",
                ConnectionOptions::builder().port(31).build(),
            )
            .build();

        let mut hosts = Hosts::new();
        hosts.add_host("h1", host);

        let inventory = Inventory::builder()
            .hosts(hosts)
            .groups(groups)
            .defaults(defaults)
            .build();

        let resolved = inventory
            .resolve_connection_params("h1", "ssh")
            .expect("params should resolve");
        assert_eq!(resolved.port, Some(31));
    }

    #[test]
    fn connection_manager_get_or_create_errors_without_factory() {
        let manager = ConnectionManager::default();
        let key = ConnectionKey::new("h1", "ssh");
        let err = manager.get_or_create(key).unwrap_err();
        assert_eq!(err, "connection factory not set");
    }

    #[test]
    fn connection_manager_open_connection_propagates_open_error() {
        #[derive(Debug)]
        struct FailingConnection;

        impl Connection for FailingConnection {
            fn create(&self, _key: &ConnectionKey) -> Box<dyn Connection> {
                Box::new(FailingConnection)
            }

            fn is_alive(&self) -> bool {
                false
            }

            fn open(&mut self, _params: &ResolvedConnectionParams) -> Result<(), String> {
                Err("boom".to_string())
            }

            fn close(&mut self) -> ConnectionKey {
                ConnectionKey::new("h1", "ssh")
            }
        }

        let factory = Arc::new(|_key: &ConnectionKey| {
            Some(Arc::new(Mutex::new(FailingConnection)) as Arc<Mutex<dyn Connection>>)
        });
        let manager = ConnectionManager::with_connection_factory(factory);
        let key = ConnectionKey::new("h1", "ssh");
        let params = ResolvedConnectionParams {
            hostname: "h1".to_string(),
            port: Some(22),
            username: None,
            password: None,
            platform: None,
            extras: None,
        };

        let err = manager.open_connection(&key, &params).unwrap_err();
        assert_eq!(err, "boom");
        let counters = manager.connection_counters_for("ssh").unwrap();
        assert_eq!(counters.create_calls, 1);
        assert_eq!(counters.open_calls, 0);
    }

    #[test]
    fn connection_manager_open_connection_returns_none_when_factory_returns_none() {
        let factory = Arc::new(|_key: &ConnectionKey| None);
        let manager = ConnectionManager::with_connection_factory(factory);
        let key = ConnectionKey::new("h1", "ssh");
        let params = ResolvedConnectionParams {
            hostname: "h1".to_string(),
            port: Some(22),
            username: None,
            password: None,
            platform: None,
            extras: None,
        };

        let result = manager.open_connection(&key, &params).unwrap();
        assert!(result.is_none());
        assert!(manager.connection_counters_for("ssh").is_none());
    }

    #[test]
    fn inventory_applies_transform_options_to_hosts_groups_and_defaults() {
        struct OptTransform;

        impl Transform for OptTransform {
            fn transform_host(
                &self,
                host: &Host,
                options: Option<&TransformFunctionOptions>,
            ) -> Host {
                let port = options
                    .and_then(|opts| opts.get("port").and_then(|v| v.as_u64()))
                    .unwrap_or(0) as u16;
                host.to_builder().port(port).build()
            }

            fn transform_group(
                &self,
                group: &Group,
                options: Option<&TransformFunctionOptions>,
            ) -> Group {
                let username = options
                    .and_then(|opts| opts.get("username").and_then(|v| v.as_str()))
                    .unwrap_or("default");
                group.to_builder().username(username).build()
            }

            fn transform_defaults(
                &self,
                defaults: &Defaults,
                options: Option<&TransformFunctionOptions>,
            ) -> Defaults {
                let hostname = options
                    .and_then(|opts| opts.get("hostname").and_then(|v| v.as_str()))
                    .unwrap_or("defaults");
                defaults.to_builder().hostname(hostname).build()
            }
        }

        let transform = TransformFunction::new_full(OptTransform);
        let options = TransformFunctionOptions::new(serde_json::json!({
            "port": 2022,
            "username": "opt-user",
            "hostname": "opt-defaults"
        }));

        let mut hosts = Hosts::new();
        hosts.add_host("h1", Host::builder().build());

        let mut groups = Groups::new();
        groups.add_group("g1", Group::builder().build());

        let defaults = Defaults::builder().build();

        let inventory = Inventory::builder()
            .hosts(hosts)
            .groups(groups)
            .defaults(defaults)
            .transform_function(transform)
            .transform_function_options(options)
            .build();

        let host = inventory.hosts().get("h1").expect("host exists");
        assert_eq!(host.port(), Some(2022));

        let group = inventory.groups().expect("groups exist").get("g1").unwrap();
        assert_eq!(group.username(), Some("opt-user"));

        let defaults = inventory.defaults().expect("defaults exist");
        assert_eq!(defaults.hostname(), Some("opt-defaults"));
    }

    #[test]
    fn host_deserializes_with_missing_fields_as_none() {
        let host: Host = serde_json::from_value(serde_json::json!({}))
            .expect("host should deserialize from empty object");
        assert_eq!(host.hostname(), None);
        assert_eq!(host.port(), None);
        assert_eq!(host.username(), None);
        assert_eq!(host.password(), None);
        assert_eq!(host.platform(), None);
        assert!(host.groups().is_none());
        assert!(host.data().is_none());
        assert!(host.connection_options().is_none());
    }
}
