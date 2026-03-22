//! Connection factory implementation for plugin-based connections.
//!
//! This module provides the infrastructure to integrate plugin-based connections
//! into the core inventory system's connection management. It bridges the gap
//! between the `PluginConnection` trait used by plugins and the `Connection` trait
//! expected by the inventory system.
//!
//! # Overview
//!
//! The module consists of two main components:
//!
//! 1. **[`PluginConnectionAdapter`]** - An adapter that wraps `PluginConnection`
//!    implementations and provides the `Connection` interface. It tracks connection
//!    lifecycle state and delegates operations to the underlying plugin.
//!
//! 2. **[`build_connection_factory`]** - A factory function that creates a
//!    `ConnectionFactory` closure. This factory looks up plugins by connection type
//!    and creates appropriate connection instances wrapped in adapters.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    Connection Manager                       │
//! │                  (genja_core::inventory)                    │
//! └─────────────────────────┬───────────────────────────────────┘
//!                           │
//!                           │ Uses ConnectionFactory
//!                           ▼
//! ┌─────────────────────────────────────────────────────────────┐
//! │              build_connection_factory()                     │
//! │         Returns: Arc<ConnectionFactory>                     │
//! └─────────────────────────┬───────────────────────────────────┘
//!                           │
//!                           │ Queries for plugins
//!                           ▼
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    PluginManager                            │
//! │              (Registered Connection Plugins)                │
//! └─────────────────────────┬───────────────────────────────────┘
//!                           │
//!                           │ Returns plugin instance
//!                           ▼
//! ┌─────────────────────────────────────────────────────────────┐
//! │              PluginConnectionAdapter                        │
//! │         Wraps: Box<dyn PluginConnection>                    │
//! │         Implements: Connection trait                        │
//! └─────────────────────────┬───────────────────────────────────┘
//!                           │
//!                           │ Delegates to
//!                           ▼
//! ┌─────────────────────────────────────────────────────────────┐
//! │              Plugin Implementation                          │
//! │         (e.g., SSH, Telnet, NETCONF)                        │
//! │         Implements: PluginConnection trait                  │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Usage
//!
//! ## Basic Setup
//!
//! ```no_run
//! use genja_plugin_manager::{PluginManager, connection_factory::build_connection_factory};
//! use genja_core::inventory::{ConnectionManager, ConnectionKey};
//! use std::sync::Arc;
//!
//! // 1. Create and configure plugin manager
//! let mut plugin_manager = PluginManager::new();
//! // Register connection plugins...
//!
//! // 2. Build connection factory
//! let factory = build_connection_factory(Arc::new(plugin_manager));
//!
//! // 3. Set factory in connection manager
//! let connection_manager = ConnectionManager::default();
//! connection_manager.set_connection_factory(factory);
//!
//! // 4. Use connection manager to create connections
//! let key = ConnectionKey::new("router1", "ssh");
//! // let connection = connection_manager.get_or_create(key);
//! ```
//!
//! ## Plugin Integration
//!
//! Connection plugins must implement the `PluginConnection` trait:
//!
//! ```no_run
//! use genja_plugin_manager::plugin_types::{Plugin, PluginConnection};
//! use genja_core::inventory::{ConnectionKey, ResolvedConnectionParams};
//!
//! struct MyConnectionPlugin {
//!     key: ConnectionKey,
//! }
//!
//! impl Plugin for MyConnectionPlugin {
//!     fn name(&self) -> String {
//!         "my_connection".to_string()
//!     }
//! }
//!
//! impl PluginConnection for MyConnectionPlugin {
//!     fn create(&self, key: &ConnectionKey) -> Box<dyn PluginConnection> {
//!         Box::new(MyConnectionPlugin { key: key.clone() })
//!     }
//!
//!     fn open(&mut self, params: &ResolvedConnectionParams) -> Result<(), String> {
//!         // Establish connection
//!         Ok(())
//!     }
//!
//!     fn close(&mut self) -> ConnectionKey {
//!         // Clean up connection
//!         self.key.clone()
//!     }
//!
//!     fn is_alive(&self) -> bool {
//!         // Check connection status
//!         true
//!     }
//! }
//! ```
//!
//! # Connection Lifecycle
//!
//! The adapter manages the connection lifecycle through the following states:
//!
//! 1. **Created** - Adapter is instantiated with `alive = false`
//! 2. **Opening** - `open()` is called with connection parameters
//! 3. **Open** - Connection established successfully, `alive = true`
//! 4. **Closing** - `close()` is called to tear down connection
//! 5. **Closed** - Connection terminated, `alive = false`
//!
//! ## State Transitions
//!
//! ```text
//! ┌─────────┐
//! │ Created │ (alive = false)
//! └────┬────┘
//!      │
//!      │ open() called
//!      ▼
//! ┌─────────┐
//! │ Opening │
//! └────┬────┘
//!      │
//!      ├─ Success ──► ┌──────┐
//!      │              │ Open │ (alive = true)
//!      │              └───┬──┘
//!      │                  │
//!      │                  │ close() called
//!      │                  ▼
//!      │              ┌────────┐
//!      └─ Failure ──► │ Closed │ (alive = false)
//!                     └────────┘
//! ```
//!
//! # Thread Safety
//!
//! All components in this module are designed for concurrent use:
//!
//! - The connection factory is wrapped in `Arc` and can be shared across threads
//! - Each connection adapter is wrapped in `Arc<Mutex<_>>` for safe mutation
//! - The `PluginManager` reference is shared via `Arc` in the factory closure
//!
//! # Error Handling
//!
//! The factory returns `Option<Arc<Mutex<dyn Connection>>>`:
//!
//! - `Some(connection)` - Plugin found and connection created successfully
//! - `None` - Plugin not found or not a connection plugin
//!
//! Connection operations return `Result<(), String>`:
//!
//! - `Ok(())` - Operation succeeded
//! - `Err(message)` - Operation failed with error description
//!
//! # Performance Considerations
//!
//! - **Connection Pooling**: The `ConnectionManager` handles connection reuse
//! - **Lazy Creation**: Connections are created only when needed
//! - **Plugin Lookup**: Plugin queries are O(1) hash map lookups
//! - **Lock Contention**: Each connection has its own mutex to minimize contention
//!
//! # Examples
//!
//! ## Complete Integration Example
//!
//! ```no_run
//! use genja_plugin_manager::{PluginManager, connection_factory::build_connection_factory};
//! use genja_core::inventory::{
//!     ConnectionManager, ConnectionKey, ResolvedConnectionParams,
//!     BaseBuilderHost, Host, Hosts, Inventory,
//! };
//! use std::sync::Arc;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Set up plugin manager with connection plugins
//! let mut plugin_manager = PluginManager::new();
//! // plugin_manager.load_plugins_from_directory("plugins")?;
//!
//! // Create connection factory
//! let factory = build_connection_factory(Arc::new(plugin_manager));
//!
//! // Build inventory with hosts
//! let mut hosts = Hosts::new();
//! hosts.add_host("router1", Host::builder()
//!     .hostname("10.0.0.1")
//!     .port(22)
//!     .username("admin")
//!     .platform("cisco_ios")
//!     .build());
//!
//! let inventory = Inventory::builder()
//!     .hosts(hosts)
//!     .build();
//!
//! // Set up connection manager
//! let connection_manager = inventory.connections();
//! connection_manager.set_connection_factory(factory);
//!
//! // Create and use connection
//! let key = ConnectionKey::new("router1", "ssh");

//! // Open connection
//! let params = ResolvedConnectionParams {
//!     hostname: "10.0.0.1".to_string(),
//!     port: Some(22),
//!     username: Some("admin".to_string()),
//!     password: Some("secret".to_string()),
//!     platform: Some("cisco_ios".to_string()),
//!     extras: None,
//! };
//! let connection = connection_manager
//!     .open_connection(&key, &params)?
//!     .expect("connection plugin not found");
//!
//! let conn = connection.lock().unwrap();
//!
//! // Use connection...
//! assert!(conn.is_alive());
//!
//! // Close connection
//! drop(conn);
//! connection_manager.close_connection(&key);
//! # Ok(())
//! # }
//! ```
//!
//! # See Also
//!
//! - [`PluginManager`] - Manages plugin registration and lookup
//! - [`PluginConnection`] - Plugin connection trait
//! - [`Connection`] - Core connection trait
//! - [`ConnectionFactory`] - Factory function type for creating connections

use crate::PluginManager;
use crate::plugin_types::{PluginConnection, Plugins};
use genja_core::inventory::{
    Connection, ConnectionFactory, ConnectionKey, ResolvedConnectionParams,
};
use std::sync::{Arc, Mutex};

/// Adapter that bridges `PluginConnection` trait objects to the `Connection` trait.
///
/// This adapter wraps a `PluginConnection` implementation and provides the `Connection`
/// interface expected by the inventory system. It maintains connection lifecycle state
/// and delegates operations to the underlying plugin.
///
/// # Purpose
///
/// The adapter serves two main purposes:
/// 1. **Trait Adaptation**: Converts `PluginConnection` trait objects into `Connection`
///    trait objects, allowing plugins to integrate with the core connection management system.
/// 2. **State Tracking**: Maintains the `alive` flag to track whether a connection has been
///    successfully opened, providing quick status checks without querying the plugin.
///
/// # Lifecycle
///
/// The adapter tracks connection state through the `alive` flag:
/// - Initially `false` when created
/// - Set to `true` after successful `open()` call
/// - Reset to `false` after `close()` call
///
/// # Thread Safety
///
/// This adapter is typically wrapped in `Arc<Mutex<_>>` by the connection factory,
/// ensuring thread-safe access to the underlying plugin connection.
///
/// # Examples
///
/// ```no_run
/// use genja_plugin_manager::connection_factory::PluginConnectionAdapter;
/// use genja_core::inventory::{Connection, ConnectionKey, ResolvedConnectionParams};
///
/// // Typically created by the connection factory, not directly
/// // let plugin_connection = ...; // Some PluginConnection implementation
/// // let adapter = PluginConnectionAdapter::new(plugin_connection);
/// ```
#[derive(Debug)]
#[doc(hidden)]
pub struct PluginConnectionAdapter {
    /// The underlying plugin connection implementation.
    ///
    /// This boxed trait object contains the actual connection logic provided
    /// by the plugin. All connection operations are delegated to this inner plugin.
    inner: Box<dyn PluginConnection>,

    /// Tracks whether the connection is currently alive.
    ///
    /// This flag is set to `true` when `open()` succeeds and reset to `false`
    /// when `close()` is called. It provides a quick way to check connection
    /// status without querying the plugin directly.
    alive: bool,
}

impl PluginConnectionAdapter {
    /// Creates a new adapter wrapping the given plugin connection.
    ///
    /// The adapter is initialized with `alive` set to `false`, indicating
    /// that no connection has been established yet.
    ///
    /// # Parameters
    ///
    /// * `inner` - The plugin connection implementation to wrap
    ///
    /// # Returns
    ///
    /// A new `PluginConnectionAdapter` instance ready to manage the connection lifecycle.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use genja_plugin_manager::connection_factory::PluginConnectionAdapter;
    ///
    /// // let plugin_connection = ...; // Some PluginConnection implementation
    /// // let adapter = PluginConnectionAdapter::new(plugin_connection);
    /// // assert!(!adapter.is_alive());
    /// ```
    fn new(inner: Box<dyn PluginConnection>) -> Self {
        Self {
            inner,
            alive: false,
        }
    }
}

impl Connection for PluginConnectionAdapter {
    /// Creates a new connection instance for the specified key.
    ///
    /// Delegates to the underlying plugin's `create()` method and wraps the result
    /// in a new adapter. This allows each host to have its own connection instance
    /// while maintaining consistent lifecycle management.
    ///
    /// # Parameters
    ///
    /// * `key` - The connection key identifying the host and connection type
    ///
    /// # Returns
    ///
    /// A boxed `Connection` trait object containing the new connection instance.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use genja_core::inventory::{Connection, ConnectionKey};
    ///
    /// // let adapter: Box<dyn Connection> = ...;
    /// // let key = ConnectionKey {
    /// //     hostname: "router1".to_string(),
    /// //     connection_type: "ssh".to_string(),
    /// // };
    /// // let new_connection = adapter.create(&key);
    /// ```
    fn create(&self, key: &ConnectionKey) -> Box<dyn Connection> {
        let instance = self.inner.create(key);
        Box::new(PluginConnectionAdapter::new(instance))
    }

    /// Checks if the connection is currently alive.
    ///
    /// Returns the value of the internal `alive` flag, which is set to `true`
    /// after a successful `open()` call and reset to `false` after `close()`.
    ///
    /// # Returns
    ///
    /// `true` if the connection has been opened and not yet closed, `false` otherwise.
    ///
    /// # Note
    ///
    /// This method returns cached state and does not verify the actual connection
    /// status with the underlying plugin. For real-time validation, plugins should
    /// implement additional health check mechanisms.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use genja_core::inventory::Connection;
    ///
    /// // let mut adapter: Box<dyn Connection> = ...;
    /// // assert!(!adapter.is_alive());
    /// // adapter.open(&params)?;
    /// // assert!(adapter.is_alive());
    /// ```
    fn is_alive(&self) -> bool {
        self.alive
    }

    /// Opens a connection using the provided parameters.
    ///
    /// Delegates the connection establishment to the underlying plugin. If the
    /// plugin successfully opens the connection, the `alive` flag is set to `true`.
    /// If the operation fails, the flag remains `false`.
    ///
    /// # Parameters
    ///
    /// * `params` - Resolved connection parameters including hostname, port,
    ///              credentials, and platform-specific settings
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Connection opened successfully
    /// * `Err(String)` - Connection failed with error message
    ///
    /// # State Changes
    ///
    /// On success, sets `alive` to `true`. On failure, `alive` remains `false`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use genja_core::inventory::{Connection, ResolvedConnectionParams};
    ///
    /// // let mut adapter: Box<dyn Connection> = ...;
    /// // let params = ResolvedConnectionParams {
    /// //     hostname: "10.0.0.1".to_string(),
    /// //     port: Some(22),
    /// //     username: Some("admin".to_string()),
    /// //     password: Some("secret".to_string()),
    /// //     platform: Some("linux".to_string()),
    /// //     extras: None,
    /// // };
    /// // adapter.open(&params)?;
    /// ```
    fn open(&mut self, params: &ResolvedConnectionParams) -> Result<(), String> {
        let result = self.inner.open(params);
        if result.is_ok() {
            self.alive = true;
        }
        result
    }

    /// Closes the connection and returns its key.
    ///
    /// Delegates the connection teardown to the underlying plugin and resets
    /// the `alive` flag to `false`. The connection key is returned for tracking
    /// or cleanup purposes.
    ///
    /// # Returns
    ///
    /// The `ConnectionKey` identifying the closed connection.
    ///
    /// # State Changes
    ///
    /// Always sets `alive` to `false`, regardless of the plugin's close operation result.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use genja_core::inventory::Connection;
    ///
    /// // let mut adapter: Box<dyn Connection> = ...;
    /// // adapter.open(&params)?;
    /// // let key = adapter.close();
    /// // assert!(!adapter.is_alive());
    /// ```
    fn close(&mut self) -> ConnectionKey {
        let key = self.inner.close();
        self.alive = false;
        key
    }
}

/// Builds a connection factory that creates connections from registered plugins.
///
/// This function creates a `ConnectionFactory` closure that looks up plugins by
/// connection type and creates appropriate connection instances. The factory
/// integrates plugin-based connections into the inventory's connection management
/// system.
///
/// # How It Works
///
/// 1. The factory receives a `ConnectionKey` specifying the connection type
/// 2. It queries the `PluginManager` for a plugin matching that type
/// 3. If a `Connection` plugin is found, it creates a new connection instance
/// 4. The instance is wrapped in a `PluginConnectionAdapter` for trait compatibility
/// 5. The adapter is wrapped in `Arc<Mutex<_>>` for thread-safe access
///
/// # Parameters
///
/// * `plugins` - Shared reference to the plugin manager containing registered plugins
///
/// # Returns
///
/// An `Arc<ConnectionFactory>` that can be used to create plugin-based connections.
///
/// # Plugin Requirements
///
/// The factory only works with plugins registered as `Plugins::Connection` variants.
/// Other plugin types are ignored and will result in `None` being returned.
///
/// # Thread Safety
///
/// The returned factory is thread-safe and can be shared across multiple threads.
/// Each created connection is wrapped in `Arc<Mutex<_>>` for safe concurrent access.
///
/// # Examples
///
/// ```no_run
/// use genja_plugin_manager::{PluginManager, connection_factory::build_connection_factory};
/// use genja_core::inventory::{ConnectionKey, ConnectionManager};
/// use std::sync::Arc;
///
/// // Create plugin manager and register plugins
/// let mut plugin_manager = PluginManager::new();
/// // plugin_manager.register_plugin(...);
///
/// // Build connection factory
/// let factory = build_connection_factory(Arc::new(plugin_manager));
///
/// // Use factory with connection manager
/// let connection_manager = ConnectionManager::default();
/// connection_manager.set_connection_factory(factory);
///
/// // Create connections through the manager
/// let key = ConnectionKey::new("router1", "ssh");
/// // let connection = connection_manager.get_or_create(key);
/// ```
///
/// # See Also
///
/// * [`PluginConnectionAdapter`] - The adapter that wraps plugin connections
/// * [`PluginManager`] - Manages registered plugins
/// * [`ConnectionFactory`] - The factory type returned by this function
pub fn build_connection_factory(plugins: Arc<PluginManager>) -> Arc<ConnectionFactory> {
    Arc::new(move |key: &ConnectionKey| {
        let plugin = plugins.get_plugin(&key.connection_type)?;
        match plugin {
            Plugins::Connection(connection) => {
                let instance = connection.create(key);
                let adapter = PluginConnectionAdapter::new(instance);
                Some(Arc::new(Mutex::new(adapter)) as Arc<Mutex<dyn Connection>>)
            }
            _ => None,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin_types::{Plugin, PluginConnection, PluginRunner};
    use genja_core::inventory::Connection;
    use genja_core::task::{Task, Tasks};
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Debug)]
    struct TestConnection {
        name: &'static str,
        key: ConnectionKey,
        create_calls: Arc<AtomicUsize>,
        open_calls: Arc<AtomicUsize>,
        close_calls: Arc<AtomicUsize>,
        alive: bool,
    }

    impl TestConnection {
        fn new(
            name: &'static str,
            key: ConnectionKey,
            create_calls: Arc<AtomicUsize>,
            open_calls: Arc<AtomicUsize>,
            close_calls: Arc<AtomicUsize>,
        ) -> Self {
            Self {
                name,
                key,
                create_calls,
                open_calls,
                close_calls,
                alive: false,
            }
        }
    }

    impl Plugin for TestConnection {
        fn name(&self) -> String {
            self.name.to_string()
        }
    }

    impl PluginConnection for TestConnection {
        fn create(&self, key: &ConnectionKey) -> Box<dyn PluginConnection> {
            self.create_calls.fetch_add(1, Ordering::SeqCst);
            Box::new(Self::new(
                self.name,
                key.clone(),
                Arc::clone(&self.create_calls),
                Arc::clone(&self.open_calls),
                Arc::clone(&self.close_calls),
            ))
        }

        fn open(&mut self, _params: &ResolvedConnectionParams) -> Result<(), String> {
            self.open_calls.fetch_add(1, Ordering::SeqCst);
            self.alive = true;
            Ok(())
        }

        fn close(&mut self) -> ConnectionKey {
            self.close_calls.fetch_add(1, Ordering::SeqCst);
            self.alive = false;
            self.key.clone()
        }

        fn is_alive(&self) -> bool {
            self.alive
        }
    }

    #[derive(Debug)]
    struct DummyRunner {
        name: &'static str,
    }

    impl Plugin for DummyRunner {
        fn name(&self) -> String {
            self.name.to_string()
        }
    }

    impl PluginRunner for DummyRunner {
        fn run(&self, _task: Task, _hosts: &genja_core::inventory::Hosts) {}

        fn run_tasks(&self, _tasks: Tasks, _hosts: &genja_core::inventory::Hosts) {}
    }

    fn default_params() -> ResolvedConnectionParams {
        ResolvedConnectionParams {
            hostname: "host1".to_string(),
            port: Some(22),
            username: Some("user".to_string()),
            password: Some("pass".to_string()),
            platform: Some("linux".to_string()),
            extras: None,
        }
    }

    #[test]
    fn adapter_open_close_updates_alive_and_returns_key() {
        let counters = (
            Arc::new(AtomicUsize::new(0)),
            Arc::new(AtomicUsize::new(0)),
            Arc::new(AtomicUsize::new(0)),
        );
        let key = ConnectionKey::new("host1", "ssh");
        let plugin = TestConnection::new(
            "ssh",
            key.clone(),
            Arc::clone(&counters.0),
            Arc::clone(&counters.1),
            Arc::clone(&counters.2),
        );

        let mut adapter = PluginConnectionAdapter::new(Box::new(plugin));
        assert!(!adapter.is_alive());

        adapter.open(&default_params()).unwrap();
        assert!(adapter.is_alive());

        let closed_key = adapter.close();
        assert_eq!(closed_key, key);
        assert!(!adapter.is_alive());

        assert_eq!(counters.1.load(Ordering::SeqCst), 1);
        assert_eq!(counters.2.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn adapter_create_uses_plugin_create_and_starts_dead() {
        let counters = (
            Arc::new(AtomicUsize::new(0)),
            Arc::new(AtomicUsize::new(0)),
            Arc::new(AtomicUsize::new(0)),
        );
        let key = ConnectionKey::new("host1", "ssh");
        let plugin = TestConnection::new(
            "ssh",
            key.clone(),
            Arc::clone(&counters.0),
            Arc::clone(&counters.1),
            Arc::clone(&counters.2),
        );
        let adapter = PluginConnectionAdapter::new(Box::new(plugin));

        let new_key = ConnectionKey::new("host2", "ssh");
        let new_conn = adapter.create(&new_key);
        assert!(!new_conn.is_alive());
        assert_eq!(counters.0.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn factory_returns_none_for_missing_or_non_connection_plugins() {
        let manager = Arc::new(PluginManager::new());
        let factory = build_connection_factory(Arc::clone(&manager));
        let key = ConnectionKey::new("host1", "ssh");
        assert!(factory(&key).is_none());

        let mut manager = PluginManager::new();
        manager.register_plugin(Plugins::Runner(Box::new(DummyRunner { name: "runner" })));
        let factory = build_connection_factory(Arc::new(manager));
        let key = ConnectionKey::new("host1", "runner");
        assert!(factory(&key).is_none());
    }

    #[test]
    fn factory_returns_adapter_for_connection_plugins() {
        let counters = (
            Arc::new(AtomicUsize::new(0)),
            Arc::new(AtomicUsize::new(0)),
            Arc::new(AtomicUsize::new(0)),
        );
        let key = ConnectionKey::new("host1", "ssh");
        let plugin = TestConnection::new(
            "ssh",
            key.clone(),
            Arc::clone(&counters.0),
            Arc::clone(&counters.1),
            Arc::clone(&counters.2),
        );

        let mut manager = PluginManager::new();
        manager.register_plugin(Plugins::Connection(Box::new(plugin)));

        let factory = build_connection_factory(Arc::new(manager));
        let connection = factory(&key).expect("expected connection plugin");

        {
            let mut guard = connection.lock().unwrap();
            assert!(!guard.is_alive());
            guard.open(&default_params()).unwrap();
            assert!(guard.is_alive());
            let closed_key = guard.close();
            assert_eq!(closed_key, key);
            assert!(!guard.is_alive());
        }

        assert_eq!(counters.0.load(Ordering::SeqCst), 1);
        assert_eq!(counters.1.load(Ordering::SeqCst), 1);
        assert_eq!(counters.2.load(Ordering::SeqCst), 1);
    }

    /// Tests that connections created by the factory are thread-safe and can handle concurrent access.
    ///
    /// This test verifies that the connection adapter wrapped in `Arc<Mutex<_>>` properly
    /// synchronizes access from multiple threads. It spawns two threads that concurrently
    /// attempt to open and close the same connection, ensuring that:
    ///
    /// 1. The mutex prevents data races on the connection state
    /// 2. Multiple threads can safely acquire and release the lock
    /// 3. Connection operations (open/close) are properly serialized
    /// 4. The operation counters reflect all concurrent operations
    ///
    /// # Test Setup
    ///
    /// - Creates a test connection plugin with atomic counters to track operations
    /// - Registers the plugin with a `PluginManager`
    /// - Builds a connection factory and creates a connection instance
    /// - Uses a barrier to synchronize thread execution for maximum contention
    ///
    /// # Test Execution
    ///
    /// - Thread A: Opens the connection
    /// - Thread B: Closes then reopens the connection
    /// - Main thread: Waits for both threads and performs final cleanup
    ///
    /// # Assertions
    ///
    /// - Verifies that at least 2 open operations occurred (one per thread)
    /// - Verifies that at least 1 close operation occurred (from thread B)
    ///
    /// # Panics
    ///
    /// This test will panic if:
    /// - The connection factory returns `None`
    /// - Any thread fails to acquire the mutex lock
    /// - Any connection operation fails
    /// - The expected minimum operation counts are not met
    #[test]
    fn factory_connection_is_thread_safe() {
        let counters = (
            Arc::new(AtomicUsize::new(0)),
            Arc::new(AtomicUsize::new(0)),
            Arc::new(AtomicUsize::new(0)),
        );
        let key = ConnectionKey::new("host1", "ssh");
        let plugin = TestConnection::new(
            "ssh",
            key.clone(),
            Arc::clone(&counters.0),
            Arc::clone(&counters.1),
            Arc::clone(&counters.2),
        );

        let mut manager = PluginManager::new();
        manager.register_plugin(Plugins::Connection(Box::new(plugin)));

        let factory = build_connection_factory(Arc::new(manager));
        let connection = factory(&key).expect("expected connection plugin");

        let barrier = Arc::new(std::sync::Barrier::new(3));
        let params = Arc::new(default_params());

        let conn_a = Arc::clone(&connection);
        let barrier_a = Arc::clone(&barrier);
        let params_a = Arc::clone(&params);
        let thread_a = std::thread::spawn(move || {
            barrier_a.wait();
            let mut guard = conn_a.lock().unwrap();
            guard.open(&params_a).unwrap();
        });

        let conn_b = Arc::clone(&connection);
        let barrier_b = Arc::clone(&barrier);
        let params_b = Arc::clone(&params);
        let thread_b = std::thread::spawn(move || {
            barrier_b.wait();
            let mut guard = conn_b.lock().unwrap();
            guard.close();
            guard.open(&params_b).unwrap();
        });

        barrier.wait();
        thread_a.join().unwrap();
        thread_b.join().unwrap();

        let mut guard = connection.lock().unwrap();
        guard.close();

        assert!(counters.1.load(Ordering::SeqCst) >= 2);
        assert!(counters.2.load(Ordering::SeqCst) >= 1);
    }

    #[test]
    fn adapter_open_error_keeps_alive_false() {
        #[derive(Debug)]
        struct FailingConnection;

        impl Plugin for FailingConnection {
            fn name(&self) -> String {
                "fail".to_string()
            }
        }

        impl PluginConnection for FailingConnection {
            fn create(&self, _key: &ConnectionKey) -> Box<dyn PluginConnection> {
                Box::new(Self)
            }

            fn open(&mut self, _params: &ResolvedConnectionParams) -> Result<(), String> {
                Err("boom".to_string())
            }

            fn close(&mut self) -> ConnectionKey {
                ConnectionKey::new("host1", "fail")
            }

            fn is_alive(&self) -> bool {
                false
            }
        }

        let mut adapter = PluginConnectionAdapter::new(Box::new(FailingConnection));
        assert!(!adapter.is_alive());
        let err = adapter.open(&default_params()).unwrap_err();
        assert_eq!(err, "boom");
        assert!(!adapter.is_alive());
    }

    #[test]
    fn adapter_create_can_be_called_multiple_times() {
        let counters = (
            Arc::new(AtomicUsize::new(0)),
            Arc::new(AtomicUsize::new(0)),
            Arc::new(AtomicUsize::new(0)),
        );
        let key = ConnectionKey::new("host1", "ssh");
        let plugin = TestConnection::new(
            "ssh",
            key,
            Arc::clone(&counters.0),
            Arc::clone(&counters.1),
            Arc::clone(&counters.2),
        );
        let adapter = PluginConnectionAdapter::new(Box::new(plugin));

        let key_a = ConnectionKey::new("host-a", "ssh");
        let key_b = ConnectionKey::new("host-b", "ssh");
        let conn_a = adapter.create(&key_a);
        let conn_b = adapter.create(&key_b);

        assert!(!conn_a.is_alive());
        assert!(!conn_b.is_alive());
        assert_eq!(counters.0.load(Ordering::SeqCst), 2);
    }
}
