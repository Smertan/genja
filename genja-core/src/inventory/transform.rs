use super::{Defaults, DerefTarget, Group, Host};
use genja_core_derive::{DerefMacro, DerefMutMacro};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::Arc;

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
