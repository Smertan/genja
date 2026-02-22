//! Inventory models and helpers for Genja Core.
//!
//! This module defines the in-memory inventory model (hosts, groups, defaults),
//! plus helpers for building inventories and applying optional transforms.
//!
//! **Key points**
//! - Inventory is immutable from the public API. Use builders to construct it.
//! - Hosts and groups are stored in `CustomTreeMap` keyed by name.
//! - Defaults share the same fields as groups, minus `groups` and `defaults`.
//! - `Inventory::apply_transform` runs an optional transform over the inventory.
//!
//! # Examples
//!
//! ## Build a minimal inventory
//! ```
//! use genja_core::inventory::{Host, Hosts, Inventory, BaseBuilderHost};
//!
//! let mut hosts = Hosts::new();
//! let host = Host::builder("router1").hostname("10.0.0.1").build();
//! hosts.add_host(host);
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
//! let transform = TransformFunction::new(|_inventory, _options| {
//!     // mutate inventory in place
//! });
//! let mut inventory = Inventory::builder().transform_function(transform).build();
//! inventory.apply_transform();
//! ```
use crate::CustomTreeMap;
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
    fn hostname(self, hostname: &str) -> Self;

    /// Updates the port and returns the updated builder.
    fn port(self, port: u16) -> Self;

    /// Updates the username and returns the updated builder.
    fn username(self, username: &str) -> Self;

    /// Updates the password and returns the updated builder.
    fn password(self, password: &str) -> Self;

    /// Updates the platform and returns the updated builder.
    fn platform(self, platform: &str) -> Self;

    /// Updates the groups and returns the updated builder.
    fn groups(self, groups: ParentGroups) -> Self;

    /// Updates the data and returns the updated builder.
    fn data(self, data: Data) -> Self;

    /// Updates the connection options and returns the updated builder.
    fn connection_options(self, name: String, options: ConnectionOptions) -> Self;

    /// Updates the defaults and returns the updated builder.
    fn defaults(self, defaults: &Arc<Defaults>) -> Self;

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
    pub hostname: Option<String>,
    pub port: Option<u16>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub platform: Option<String>,
    pub data: Option<Data>,
    pub connection_options: Option<CustomTreeMap<ConnectionOptions>>,
}

impl DerefTarget for Data {
    type Target = serde_json::Value;
}

impl Defaults {
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
    pub name: String,
    pub hostname: Option<String>,
    pub port: Option<u16>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub platform: Option<String>,
    pub groups: Option<ParentGroups>,
    pub data: Option<Data>,
    pub connection_options: Option<CustomTreeMap<ConnectionOptions>>,
    pub defaults: Option<Arc<Defaults>>,
    #[serde(skip)]
    #[schemars(skip)]
    pub resolved_connection_params: CustomTreeMap<ResolvedConnectionParams>,
}

impl Host {
    pub fn new(name: &str) -> Host {
        Host {
            name: name.to_string(),
            hostname: None,
            port: None,
            username: None,
            password: None,
            platform: None,
            groups: None,
            data: None,
            connection_options: None,
            defaults: None,
            resolved_connection_params: CustomTreeMap::new(),
        }
    }
    pub fn builder(name: &str) -> HostBuilder {
        HostBuilder::new(name)
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
                hostname: self.hostname.clone().unwrap_or_else(|| self.name.clone()),
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
    name: String,
    hostname: Option<String>,
    port: Option<u16>,
    username: Option<String>,
    password: Option<String>,
    platform: Option<String>,
    groups: Option<ParentGroups>,
    data: Option<Data>,
    connection_options: Option<CustomTreeMap<ConnectionOptions>>,
    defaults: Option<Arc<Defaults>>,
}

impl HostBuilder {
    pub fn new(name: &str) -> Self {
        HostBuilder {
            name: name.to_string(),
            hostname: None,
            port: None,
            username: None,
            password: None,
            platform: None,
            groups: None,
            data: None,
            connection_options: None,
            defaults: None,
        }
    }
}

impl BaseBuilderHost for HostBuilder {
    type Output = Host;

    fn hostname(mut self, hostname: &str) -> Self {
        self.hostname = Some(hostname.to_string());
        self
    }

    fn port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    fn username(mut self, username: &str) -> Self {
        self.username = Some(username.to_string());
        self
    }

    fn password(mut self, password: &str) -> Self {
        self.password = Some(password.to_string());
        self
    }

    fn platform(mut self, platform: &str) -> Self {
        self.platform = Some(platform.to_string());
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

    fn connection_options(mut self, name: String, options: ConnectionOptions) -> Self {
        if self.connection_options.is_none() {
            self.connection_options = Some(CustomTreeMap::new());
        }
        self.connection_options
            .as_mut()
            .unwrap()
            .insert(name, options);
        self
    }

    fn defaults(mut self, defaults: &Arc<Defaults>) -> Self {
        self.defaults = Some(Arc::clone(defaults));
        self
    }

    fn build(self) -> Host {
        Host {
            name: self.name,
            hostname: self.hostname,
            port: self.port,
            username: self.username,
            password: self.password,
            platform: self.platform,
            groups: self.groups,
            data: self.data,
            connection_options: self.connection_options,
            defaults: self.defaults,
            resolved_connection_params: CustomTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Group {
    pub hostname: Option<String>,
    pub port: Option<u16>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub platform: Option<String>,
    pub groups: Option<ParentGroups>,
    pub data: Option<Data>,
    pub connection_options: Option<CustomTreeMap<ConnectionOptions>>,
    pub defaults: Option<Arc<Defaults>>,
}

impl Default for Group {
    fn default() -> Group {
        Group::new()
    }
}

impl Group {
    pub fn new() -> Group {
        Group {
            hostname: None,
            port: None,
            username: None,
            password: None,
            platform: None,
            groups: None,
            data: None,
            connection_options: None,
            defaults: None,
        }
    }
    pub fn builder(hostname: &str) -> GroupBuilder {
        GroupBuilder::new(hostname)
    }
}

pub struct GroupBuilder {
    hostname: Option<String>,
    port: Option<u16>,
    username: Option<String>,
    password: Option<String>,
    platform: Option<String>,
    groups: Option<ParentGroups>,
    data: Option<Data>,
    connection_options: Option<CustomTreeMap<ConnectionOptions>>,
    defaults: Option<Arc<Defaults>>,
}

impl BaseBuilderHost for GroupBuilder {
    type Output = Group;

    fn hostname(mut self, hostname: &str) -> Self {
        self.hostname = Some(hostname.to_string());
        self
    }
    fn port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    fn username(mut self, username: &str) -> Self {
        self.username = Some(username.to_string());
        self
    }

    fn password(mut self, password: &str) -> Self {
        self.password = Some(password.to_string());
        self
    }
    fn platform(mut self, platform: &str) -> Self {
        self.platform = Some(platform.to_string());
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
    fn connection_options(mut self, name: String, options: ConnectionOptions) -> Self {
        if self.connection_options.is_none() {
            self.connection_options = Some(CustomTreeMap::new());
        }
        self.connection_options
            .as_mut()
            .unwrap()
            .insert(name, options);
        self
    }
    fn defaults(mut self, defaults: &Arc<Defaults>) -> Self {
        self.defaults = Some(Arc::clone(defaults));
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
            defaults: self.defaults,
        }
    }
}

impl GroupBuilder {
    pub fn new(hostname: &str) -> Self {
        GroupBuilder {
            hostname: Some(hostname.to_string()),
            port: None,
            username: None,
            password: None,
            platform: None,
            groups: None,
            data: None,
            connection_options: None,
            defaults: None,
        }
    }
}

pub type HostsTarget = CustomTreeMap<Host>;

impl DerefTarget for Hosts {
    type Target = CustomTreeMap<Host>;
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, DerefMacro, DerefMutMacro)]
#[serde(deny_unknown_fields)]
pub struct Hosts(HostsTarget);

impl Default for Hosts {
    fn default() -> Self {
        Self::new()
    }
}

impl Hosts {
    pub fn new() -> Self {
        Hosts(CustomTreeMap::new())
    }

    pub fn add_host(&mut self, host: Host) {
        let name = host.name.clone();
        self.insert(name, host);
    }
}

impl BaseMethods for Hosts {}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, DerefMacro, DerefMutMacro)]
pub struct Groups(CustomTreeMap<Group>);

impl DerefTarget for Groups {
    type Target = CustomTreeMap<Group>;
}

impl Groups {
    pub fn new() -> Self {
        Groups(CustomTreeMap::new())
    }
}

impl Default for Groups {
    fn default() -> Self {
        Self::new()
    }
}

type TransformFunctionType =
    Arc<dyn Fn(&mut Inventory, Option<&TransformFunctionOptions>) + Send + Sync>;

#[derive(Clone)]
pub struct TransformFunction(TransformFunctionType);

impl TransformFunction {
    pub fn new<F>(func: F) -> Self
    where
        F: Fn(&mut Inventory, Option<&TransformFunctionOptions>) + Send + Sync + 'static,
    {
        TransformFunction(Arc::new(func))
    }

    /// `(self.0)(...)` - The parentheses around self.0 explicitly
    /// call the function pointer. gives us the Arc<dyn Fn(...)>
    /// stored inside.
    ///
    /// `self.0(...)` could also be used.
    pub fn call(&self, inventory: &mut Inventory, options: Option<&TransformFunctionOptions>) {
        (self.0)(inventory, options);
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
/// hosts.add_host(Host::builder("router1").hostname("10.0.0.1").build());
///
/// let inventory = Inventory::builder().hosts(hosts).build();
/// assert_eq!(inventory.hosts().len(), 1);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Inventory {
    hosts: Hosts,
    groups: Option<Groups>,
    defaults: Option<Defaults>,
    #[serde(skip)]
    transform_function: Option<TransformFunction>,
    transform_function_options: Option<TransformFunctionOptions>,
    #[serde(skip)]
    #[schemars(skip)]
    connections: Arc<ConnectionManager>,
}

impl BaseMethods for Inventory {}

impl Inventory {
    pub fn builder() -> InventoryBuilder {
        InventoryBuilder::new()
    }

    pub fn hosts(&self) -> &Hosts {
        &self.hosts
    }

    pub fn groups(&self) -> Option<&Groups> {
        self.groups.as_ref()
    }

    pub fn defaults(&self) -> Option<&Defaults> {
        self.defaults.as_ref()
    }

    pub fn transform_function_options(&self) -> Option<&TransformFunctionOptions> {
        self.transform_function_options.as_ref()
    }

    pub fn connections(&self) -> &ConnectionManager {
        &self.connections
    }

    pub fn hosts_mut(&mut self) -> &mut Hosts {
        &mut self.hosts
    }

    /// Apply the transform function if one is set, passing the transform options
    pub fn apply_transform(&mut self) {
        if let Some(transform) = self.transform_function.clone() {
            let options = self.transform_function_options.clone();
            transform.call(self, options.as_ref());
        }
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
/// * `transform_function` - Optional transform function applied via
///   `Inventory::apply_transform`.
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
/// let host = Host::builder("router1").hostname("10.0.0.1").build();
/// hosts.add_host(host);
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
            let mut groups = ParentGroups::new();
            groups.push("cisco".to_string());
            let host = Host::builder(&format!("host{}.example.com", i))
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

            let hostname = host.name.clone();

            hosts.insert(hostname, host);
        }

        Ok(hosts)
    }

    #[test]
    fn test_host_new() {
        let host = Host::new("example.com");
        assert_eq!(host.hostname, None);
        assert_eq!(host.port, None);
        assert_eq!(host.username, None);
        assert_eq!(host.password, None);
        assert_eq!(host.platform, None);
        assert_eq!(host.groups, None);
        assert_eq!(host.data, None);
        assert_eq!(host.connection_options, None);
        assert_eq!(host.defaults.as_ref(), None);
    }

    #[test]
    fn test_hosts_new() {
        let mut hosts = Hosts::new();

        // Add 10 hosts to the hosts map with dummy data
        for i in 1..=10 {
            let host = Host::builder(&format!("host{}.example.com", i))
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

            hosts.add_host(host);
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

    // TODO: Create a test to verify the Host defaults deserialization
}
