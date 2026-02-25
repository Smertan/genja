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
use std::fmt;
use std::sync::{Arc, Mutex};

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema)]
pub struct ConnectionOptions {
    pub hostname: Option<String>,
    pub port: Option<u16>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub platform: Option<String>,
    pub extras: Option<Extras>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedConnectionParams {
    pub hostname: String,
    pub port: Option<u16>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub platform: Option<String>,
    pub extras: Option<Extras>,
}

impl Default for ConnectionOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl ConnectionOptions {
    pub fn new() -> Self {
        ConnectionOptions {
            hostname: None,
            port: None,
            username: None,
            password: None,
            platform: None,
            extras: None,
        }
    }
}

impl DerefTarget for Extras {
    type Target = serde_json::Value;
}

/// The DataExtra struct is a wrapper for serde_json::Value, any json data is accepted.
#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema, DerefMacro, DerefMutMacro,
)]
pub struct Extras(serde_json::Value);

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
        if let Some(options) = self.connection_options.as_ref() {
            builder = builder.connection_options(options.clone());
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

    pub fn connection_options(
        mut self,
        options: CustomTreeMap<ConnectionOptions>,
    ) -> Self {
        self.connection_options = Some(options);
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
    #[serde(skip)]
    #[schemars(skip)]
    resolved_connection_params: CustomTreeMap<ResolvedConnectionParams>,
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
            resolved_connection_params: CustomTreeMap::new(),
        }
    }
    pub fn builder() -> HostBuilder {
        HostBuilder::new()
    }

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

    pub fn resolve_connection_params(
        &mut self,
        connection_type: &str,
    ) -> &ResolvedConnectionParams {
        if self
            .resolved_connection_params
            .get(connection_type)
            .is_none()
        {
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

            self.resolved_connection_params
                .insert(connection_type.to_string(), resolved);
        }

        self.resolved_connection_params
            .get(connection_type)
            .expect("resolved params should be present after insertion")
    }
}

impl BaseMethods for Host {}

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
            resolved_connection_params: CustomTreeMap::new(),
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

pub trait Transform: Send + Sync {
    fn transform_host(
        &self,
        host: &Host,
        _options: Option<&TransformFunctionOptions>,
    ) -> Host {
        host.clone()
    }

    fn transform_group(
        &self,
        group: &Group,
        _options: Option<&TransformFunctionOptions>,
    ) -> Group {
        group.clone()
    }

    fn transform_defaults(
        &self,
        defaults: &Defaults,
        _options: Option<&TransformFunctionOptions>,
    ) -> Defaults {
        defaults.clone()
    }
}

#[derive(Clone)]
pub struct TransformFunction(Arc<dyn Transform>);

impl TransformFunction {
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

    pub fn new_full<T>(transform: T) -> Self
    where
        T: Transform + 'static,
    {
        TransformFunction(Arc::new(transform))
    }

    pub fn transform_host(&self, host: &Host, options: Option<&TransformFunctionOptions>) -> Host {
        self.0.transform_host(host, options)
    }

    pub fn transform_group(
        &self,
        group: &Group,
        options: Option<&TransformFunctionOptions>,
    ) -> Group {
        self.0.transform_group(group, options)
    }

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
#[derive(
    Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, DerefMacro, DerefMutMacro,
)]
pub struct TransformFunctionOptions(serde_json::Value);

impl DerefTarget for TransformFunctionOptions {
    type Target = serde_json::Value;
}

pub trait Connection
where
    Self: Send + Sync + fmt::Debug,
{
    fn is_alive(&self) -> bool;

    fn open(&mut self, params: &ResolvedConnectionParams) -> Result<(), String>;

    fn close(&mut self) -> ConnectionKey;
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct ConnectionKey {
    pub hostname: String,
    pub connection_type: String,
}

impl ConnectionKey {
    pub fn new(hostname: impl Into<String>, connection_type: impl Into<String>) -> Self {
        Self {
            hostname: hostname.into(),
            connection_type: connection_type.into(),
        }
    }
}

// TODO: Write documentation the ConnectionManager struct and its methods.
#[derive(Debug, Default)]
pub struct ConnectionManager {
    connections_map: DashMap<ConnectionKey, Arc<Mutex<dyn Connection>>>,
}

impl ConnectionManager {
    pub fn get(&self, key: &ConnectionKey) -> Option<Arc<Mutex<dyn Connection>>> {
        self.connections_map
            .get(key)
            .map(|entry| entry.value().clone())
    }

    pub fn insert(&self, key: ConnectionKey, connection: Arc<Mutex<dyn Connection>>) {
        self.connections_map.insert(key, connection);
    }

    // TODO: Include the logic to use the pluginManager to load and create connections
    // with the use on the config held in the Nornir Struct.
    pub fn get_or_create<F, C>(&self, key: ConnectionKey, ctor: F) -> Arc<Mutex<dyn Connection>>
    where
        F: FnOnce() -> C,
        C: Connection + 'static,
    {
        if let Some(connection) = self.get(&key) {
            return connection;
        }

        let connection = Arc::new(Mutex::new(ctor())) as Arc<Mutex<dyn Connection>>;
        self.connections_map
            .entry(key)
            .or_insert_with(|| connection.clone());
        connection
    }

    /// Close the connection associated with the given key and remove
    /// it from `connections_map`.
    pub fn close_connection(&self, key: &ConnectionKey) {
        if let Some((_, connection)) = self.connections_map.remove(key) {
            if let Ok(mut connection) = connection.lock() {
                connection.close();
            }
        }
    }

    /// Close all connections in `connections_map` and then clear the map.
    pub fn close_all_connections(&self) {
        self.connections_map.iter().for_each(|entry| {
            if let Ok(mut connection) = entry.value().lock() {
                connection.close();
            }
        });
        self.connections_map.clear();
    }

    pub fn open_connection(&self, _key: &ConnectionKey) -> Option<Arc<Mutex<dyn Connection>>> {
        todo!()
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
}

impl BaseMethods for Inventory {}

impl Inventory {
    pub fn builder() -> InventoryBuilder {
        InventoryBuilder::new()
    }

    pub fn hosts(&self) -> HostsView<'_> {
        HostsView { inventory: self }
    }

    pub fn groups(&self) -> Option<GroupsView<'_>> {
        self.groups
            .as_ref()
            .map(|groups| GroupsView { inventory: self, groups })
    }

    pub fn defaults(&self) -> Option<Defaults> {
        self.defaults
            .as_ref()
            .map(|defaults| self.transform_defaults_value(defaults))
    }

    pub fn transform_function_options(&self) -> Option<&TransformFunctionOptions> {
        self.transform_function_options.as_ref()
    }

    pub fn connections(&self) -> &ConnectionManager {
        &self.connections
    }

    fn transform_host_value(&self, host: &Host) -> Host {
        let mut transformed = match &self.transform_function {
            Some(transform) => transform.transform_host(host, self.transform_function_options.as_ref()),
            None => host.clone(),
        };

        transformed
    }

    fn transform_group_value(&self, group: &Group) -> Group {
        let mut transformed = match &self.transform_function {
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
                .connection_options(String::from("Cisco"), ConnectionOptions::new())
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
                .connection_options(String::from("Juniper"), ConnectionOptions::new())
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

    // TODO: Create a test to verify the Host defaults deserialization
}
