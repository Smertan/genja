use crate::CustomTreeMap;
use genja_core_derive::{DerefMacro, DerefMutMacro};
use schemars::{JsonSchema, schema_for};
use serde::de::{Error, SeqAccess, Unexpected, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;

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
/// This struct defines optional connection parameters that can be specified per connection plugin name
/// (e.g., "ssh", "netconf", "http") to override the base connection settings defined at the host,
/// group, or defaults level. Connection options are stored in a map keyed by connection plugin name
/// and are applied during connection parameter resolution.
///
/// All fields are optional, allowing partial overrides. When resolving connection parameters,
/// these options take precedence over base settings at the same hierarchy level (host, group, or defaults).
///
/// # Fields
///
/// * `hostname` - Optional hostname or IP address override for this connection plugin name.
///   When specified, overrides the base hostname for connections of this type.
///
/// * `port` - Optional port number override for this connection plugin name.
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
    pub(crate) hostname: Option<String>,
    pub(crate) port: Option<u16>,
    pub(crate) username: Option<String>,
    pub(crate) password: Option<String>,
    pub(crate) platform: Option<String>,
    pub(crate) extras: Option<Extras>,
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
/// * `hostname` - Optional hostname or IP address override for the connection plugin name.
///   When set, this value will override the base hostname for connections of this type.
///
/// * `port` - Optional port number override for the connection plugin name.
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

impl Default for ConnectionOptionsBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Fully resolved connection parameters for establishing a connection to a host.
///
/// This struct represents the final, merged connection configuration after applying
/// defaults, group settings, host-specific settings, and connection-plugin-name-specific
/// overrides. It contains all the information needed to establish a connection to
/// a target host using a specific connection plugin name (e.g., SSH, NETCONF, HTTP).
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
pub struct ParentGroups(pub(crate) Vec<String>);

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
                log::error!("{err}");
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
///   connection plugin name. Allows per-connection-plugin-name customization of default parameters.
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
    /// * `name` - A string-like value identifying the connection plugin name (e.g., "ssh", "netconf").
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
        self.connection_options
            .get_or_insert_with(CustomTreeMap::new)
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

impl Default for DefaultsBuilder {
    fn default() -> Self {
        Self::new()
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
pub struct Data(pub(crate) serde_json::Value);

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
///   type (e.g., "ssh", "netconf", "http"). Allows per-connection-plugin-name customization of
///   connection parameters, overriding base host settings for specific connection plugin names.
///
/// # Deserialization
///
/// - Unknown fields are rejected via `#[serde(deny_unknown_fields)]` to catch configuration errors
/// - All fields are optional, allowing minimal host definitions
/// - Connection options accept arbitrary map keys for different connection plugin names
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

    /// Resolves connection parameters for a specific connection plugin name by merging host-level
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
    /// * `connection_type` - A string identifying the connection plugin name to resolve parameters for
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

        if let Some(options_map) = &self.connection_options
            && let Some(options) = options_map.get(connection_type)
        {
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

        resolved
    }
}

impl Default for Host {
    fn default() -> Self {
        Self::new()
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
///   type. Allows per-connection-plugin-name customization of connection parameters.
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

impl Default for HostBuilder {
    fn default() -> Self {
        Self::new()
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
        self.connection_options
            .get_or_insert_with(CustomTreeMap::new)
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
    /// * `name` - A string-like value identifying the connection plugin name (e.g., "ssh", "netconf").
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
        self.connection_options
            .get_or_insert_with(CustomTreeMap::new)
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

impl Default for GroupBuilder {
    fn default() -> Self {
        Self::new()
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
pub struct Hosts(pub(crate) HostsTarget);

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
