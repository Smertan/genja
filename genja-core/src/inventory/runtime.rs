use super::{
    BaseMethods, ConnectionManager, ConnectionOptions, Data, Defaults, Group, Groups, Host,
    Hosts, ResolvedConnectionParams, TransformFunction, TransformFunctionOptions,
};
use crate::{CustomTreeMap, NatString, State};
use dashmap::DashMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

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
    #[serde(skip)]
    #[schemars(skip)]
    state: Arc<State>,
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

    /// Returns the global runtime state for the current Genja instance.
    pub fn state(&self) -> &State {
        self.state.as_ref()
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

    /// Resolves connection parameters for a specific host and connection plugin name.
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
    pub(crate) fn resolve_group_internal(
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

trait OverlayFields {
    fn hostname(&self) -> &Option<String>;
    fn port(&self) -> &Option<u16>;
    fn username(&self) -> &Option<String>;
    fn password(&self) -> &Option<String>;
    fn platform(&self) -> &Option<String>;
    fn data(&self) -> &Option<Data>;
    fn connection_options(&self) -> &Option<CustomTreeMap<ConnectionOptions>>;
}

impl OverlayFields for Defaults {
    fn hostname(&self) -> &Option<String> {
        &self.hostname
    }

    fn port(&self) -> &Option<u16> {
        &self.port
    }

    fn username(&self) -> &Option<String> {
        &self.username
    }

    fn password(&self) -> &Option<String> {
        &self.password
    }

    fn platform(&self) -> &Option<String> {
        &self.platform
    }

    fn data(&self) -> &Option<Data> {
        &self.data
    }

    fn connection_options(&self) -> &Option<CustomTreeMap<ConnectionOptions>> {
        &self.connection_options
    }
}

impl OverlayFields for Group {
    fn hostname(&self) -> &Option<String> {
        &self.hostname
    }

    fn port(&self) -> &Option<u16> {
        &self.port
    }

    fn username(&self) -> &Option<String> {
        &self.username
    }

    fn password(&self) -> &Option<String> {
        &self.password
    }

    fn platform(&self) -> &Option<String> {
        &self.platform
    }

    fn data(&self) -> &Option<Data> {
        &self.data
    }

    fn connection_options(&self) -> &Option<CustomTreeMap<ConnectionOptions>> {
        &self.connection_options
    }
}

impl OverlayFields for Host {
    fn hostname(&self) -> &Option<String> {
        &self.hostname
    }

    fn port(&self) -> &Option<u16> {
        &self.port
    }

    fn username(&self) -> &Option<String> {
        &self.username
    }

    fn password(&self) -> &Option<String> {
        &self.password
    }

    fn platform(&self) -> &Option<String> {
        &self.platform
    }

    fn data(&self) -> &Option<Data> {
        &self.data
    }

    fn connection_options(&self) -> &Option<CustomTreeMap<ConnectionOptions>> {
        &self.connection_options
    }
}

fn merge_overlay_into_host<T: OverlayFields>(target: &mut Host, source: &T) {
    merge_option(&mut target.hostname, source.hostname());
    merge_option(&mut target.port, source.port());
    merge_option(&mut target.username, source.username());
    merge_option(&mut target.password, source.password());
    merge_option(&mut target.platform, source.platform());
    merge_data(&mut target.data, source.data());
    merge_connection_options(&mut target.connection_options, source.connection_options());
}

fn merge_overlay_into_group<T: OverlayFields>(target: &mut Group, source: &T) {
    merge_option(&mut target.hostname, source.hostname());
    merge_option(&mut target.port, source.port());
    merge_option(&mut target.username, source.username());
    merge_option(&mut target.password, source.password());
    merge_option(&mut target.platform, source.platform());
    merge_data(&mut target.data, source.data());
    merge_connection_options(&mut target.connection_options, source.connection_options());
}

fn merge_defaults_into_host(target: &mut Host, defaults: &Defaults) {
    merge_overlay_into_host(target, defaults);
}

fn merge_group_into_host(target: &mut Host, group: &Group) {
    merge_overlay_into_host(target, group);
}

fn merge_host_into_host(target: &mut Host, host: &Host) {
    merge_overlay_into_host(target, host);
    if host.groups.is_some() {
        target.groups = host.groups.clone();
    }
}

fn merge_group_into_group(target: &mut Group, group: &Group) {
    merge_overlay_into_group(target, group);
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
pub(crate) fn merge_data(target: &mut Option<Data>, source: &Option<Data>) {
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

pub(crate) fn merge_connection_options(
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

pub(crate) fn merge_connection_options_fields(
    target: &mut ConnectionOptions,
    source: &ConnectionOptions,
) {
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
        self.inventory
            .hosts
            .keys()
            .filter(|key| self.inventory.state.is_in_scope_key(key))
            .count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn keys(&self) -> impl Iterator<Item = &'a NatString> {
        self.inventory
            .hosts
            .keys()
            .filter(|key| self.inventory.state.is_in_scope_key(key))
    }

    pub fn get(&self, name: &str) -> Option<Host> {
        if !self.inventory.state.is_in_scope(name) {
            return None;
        }
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
        self.inventory.hosts.iter().filter_map(|(id, host)| {
            if self.inventory.state.is_in_scope_key(id) {
                Some((id, self.inventory.cached_host_value(id, host)))
            } else {
                None
            }
        })
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
            state: Arc::new(State::new()),
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
        let hosts = self.hosts.unwrap_or_default();
        let state = State::new();
        for key in hosts.keys() {
            state.mark_in_scope_key(key);
        }

        Inventory {
            hosts,
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
            state: Arc::new(state),
        }
    }
}

impl Default for InventoryBuilder {
    fn default() -> Self {
        Self::new()
    }
}
