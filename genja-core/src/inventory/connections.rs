use super::ResolvedConnectionParams;
use async_trait::async_trait;
use dashmap::DashMap;
use std::any::Any;
use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, RwLock};
use tokio::sync::{Mutex, OwnedMutexGuard};

#[async_trait]
pub trait Connection
where
    Self: Any + Send + Sync + fmt::Debug,
{
    fn create(&self, key: &ConnectionKey) -> Box<dyn Connection>;

    fn is_alive(&self) -> bool;

    async fn open(&mut self, params: &ResolvedConnectionParams) -> Result<(), String>;

    async fn execute_command(&mut self, _command: &str) -> Result<String, String> {
        Err("connection does not implement execute_command".to_string())
    }

    fn close(&mut self) -> ConnectionKey;
}

/// A unique identifier for a connection in the connection manager.
///
/// `ConnectionKey` serves as a composite key for looking up and managing connections
/// in the `ConnectionManager`. It combines a hostname with a connection plugin name to
/// uniquely identify a specific connection instance. This allows the same host to have
/// multiple concurrent connections handled by different plugins (e.g., SSH, NETCONF, HTTP).
///
/// The struct implements `Hash` and `Eq` to enable its use as a key in hash-based
/// collections like `HashMap` and `DashMap`.
///
/// # Hash Function Behavior
///
/// When inserting a `ConnectionKey` into a hash-based collection (like `DashMap` in
/// `ConnectionManager`), the hash function is used to:
///
/// 1. **Compute Hash Value**: Both `hostname` and `plugin_name` fields are hashed
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
///    `plugin_name`, the hash function ensures the key maps to the same bucket,
///    and `Eq` confirms it's the same key, allowing the collection to update the
///    existing entry rather than creating a duplicate.
///
/// # Fields
///
/// * `hostname` - The hostname or IP address of the target device. This identifies
///   the remote endpoint for the connection.
/// * `plugin_name` - The connection plugin name (e.g., "ssh", "netconf", "http").
///   This distinguishes between different connection plugin types to the same host.
///
/// # Examples
///
/// ## Basic Usage
///
/// ```
/// # use genja_core::inventory::ConnectionKey;
/// let key = ConnectionKey::new("10.0.0.1", "ssh");
/// assert_eq!(key.hostname, "10.0.0.1");
/// assert_eq!(key.plugin_name, "ssh");
/// ```
///
/// ## Multiple Connection Plugins per Host
///
/// ```
/// # use genja_core::inventory::ConnectionKey;
/// use std::collections::HashMap;
///
/// let mut connections = HashMap::new();
/// let ssh_key = ConnectionKey::new("router1", "ssh");
/// let netconf_key = ConnectionKey::new("router1", "netconf");
///
/// // Same host can have different connection plugins
/// // Each key produces a different hash due to different plugin_name
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
/// // Both keys have the same hostname and plugin_name
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
    pub plugin_name: String,
}

impl ConnectionKey {
    /// Creates a new `ConnectionKey` from a hostname and plugin name.
    ///
    /// This constructor provides a convenient way to create a connection key by accepting
    /// any type that can be converted into a `String` for both the hostname and connection
    /// type parameters. This allows passing `&str`, `String`, or other string-like types
    /// without explicit conversion.
    ///
    /// The resulting key uniquely identifies a connection in the `ConnectionManager` by
    /// combining the target hostname with the connection plugin name.
    ///
    /// # Parameters
    ///
    /// * `hostname` - The hostname or IP address of the target device. Accepts any type
    ///   implementing `Into<String>`, such as `&str` or `String`. This identifies the
    ///   remote endpoint for the connection.
    /// * `plugin_name` - The connection plugin name (e.g., "ssh", "netconf", "http").
    ///   Accepts any type implementing `Into<String>`. This distinguishes between different
    ///   connection plugin names to the same host.
    ///
    /// # Returns
    ///
    /// Returns a new `ConnectionKey` instance with the provided hostname and plugin name.
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
    /// let plugin_name = String::from("netconf");
    /// let key2 = ConnectionKey::new(hostname, plugin_name);
    ///
    /// // Mixed types
    /// let key3 = ConnectionKey::new("10.0.0.2", String::from("http"));
    /// ```
    pub fn new(hostname: impl Into<String>, plugin_name: impl Into<String>) -> Self {
        Self {
            hostname: hostname.into(),
            plugin_name: plugin_name.into(),
        }
    }
}

pub type ConnectionFactory =
    dyn Fn(&ConnectionKey) -> Option<Arc<Mutex<dyn Connection>>> + Send + Sync;

/// Statistics tracking connection lifecycle operations per connection plugin name.
///
/// `ConnectionCounters` provides a simple counter-based mechanism for monitoring connection
/// operations in the `ConnectionManager`. Each connection plugin name (e.g., "ssh", "netconf", "http")
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
/// connection plugin names simultaneously without blocking each other.
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
/// # use async_trait::async_trait;
/// # use std::sync::Arc;
/// # use tokio::runtime::Builder;
/// # use tokio::sync::Mutex;
/// # use genja_core::inventory::{Connection, ConnectionKey, ConnectionManager, ResolvedConnectionParams};
/// # #[derive(Debug)]
/// # struct SshConnection { alive: bool }
/// # #[async_trait]
/// # impl Connection for SshConnection {
/// #     fn create(&self, _key: &ConnectionKey) -> Box<dyn Connection> {
/// #         Box::new(SshConnection { alive: false })
/// #     }
/// #     fn is_alive(&self) -> bool { self.alive }
/// #     async fn open(&mut self, _params: &ResolvedConnectionParams) -> Result<(), String> {
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
/// let runtime = Builder::new_current_thread().enable_all().build().unwrap();
/// runtime.block_on(async {
///     manager.open_connection(&key, &params).await?;
///     manager.open_connection(&key, &params).await?; // Reuses existing connection
///     Ok::<(), String>(())
/// })?;
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
/// # use async_trait::async_trait;
/// # use std::sync::Arc;
/// # use tokio::runtime::Builder;
/// # use tokio::sync::Mutex;
/// # use genja_core::inventory::{Connection, ConnectionKey, ConnectionManager, ResolvedConnectionParams};
/// # #[derive(Debug)]
/// # struct SshConnection { alive: bool }
/// # #[async_trait]
/// # impl Connection for SshConnection {
/// #     fn create(&self, _key: &ConnectionKey) -> Box<dyn Connection> {
/// #         Box::new(SshConnection { alive: false })
/// #     }
/// #     fn is_alive(&self) -> bool { self.alive }
/// #     async fn open(&mut self, _params: &ResolvedConnectionParams) -> Result<(), String> {
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
/// let runtime = Builder::new_current_thread().enable_all().build().unwrap();
/// for i in 1..=5 {
///     let key = ConnectionKey::new(format!("router{}", i), "ssh");
///     runtime.block_on(async { manager.open_connection(&key, &params).await })?;
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
/// # use async_trait::async_trait;
/// # use std::sync::Arc;
/// # use tokio::runtime::Builder;
/// # use tokio::sync::Mutex;
/// # use genja_core::inventory::{Connection, ConnectionKey, ConnectionManager, ResolvedConnectionParams};
/// # #[derive(Debug)]
/// # struct TestConnection { conn_type: String, alive: bool }
/// # #[async_trait]
/// # impl Connection for TestConnection {
/// #     fn create(&self, key: &ConnectionKey) -> Box<dyn Connection> {
/// #         Box::new(TestConnection { conn_type: key.plugin_name.clone(), alive: false })
/// #     }
/// #     fn is_alive(&self) -> bool { self.alive }
/// #     async fn open(&mut self, _params: &ResolvedConnectionParams) -> Result<(), String> {
/// #         self.alive = true; Ok(())
/// #     }
/// #     fn close(&mut self) -> ConnectionKey {
/// #         self.alive = false;
/// #         ConnectionKey::new("host", &self.conn_type)
/// #     }
/// # }
/// # let factory = Arc::new(|key: &ConnectionKey| {
/// #     Some(Arc::new(Mutex::new(TestConnection {
/// #         conn_type: key.plugin_name.clone(),
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
/// // Open different connection plugin names
/// let runtime = Builder::new_current_thread().enable_all().build().unwrap();
/// runtime.block_on(async {
///     manager.open_connection(&ConnectionKey::new("router1", "ssh"), &params).await?;
///     manager.open_connection(&ConnectionKey::new("router1", "netconf"), &params).await?;
///     Ok::<(), String>(())
/// })?;
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
/// to create connections dynamically based on connection plugin name, and maintains a pool of
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
///    keyed by `ConnectionKey` (hostname + connection plugin name). Connections are wrapped
///    in `Arc<Mutex<_>>` for thread-safe sharing and interior mutability.
///
/// 2. **Connection Factory** (`connection_factory`): An optional factory function that
///    creates new connections on demand. The factory is wrapped in `RwLock<Option<Arc<_>>>`
///    to allow runtime configuration while supporting concurrent reads.
///
/// 3. **Usage Counters** (`counters`): A `DashMap` tracking create, open, and close
///    operations per connection plugin name. Useful for monitoring, debugging, and testing.
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
/// - **Plugin-Based Architecture**: Different connection plugin names (SSH, NETCONF, HTTP)
///   can be registered dynamically via plugins.
///
/// - **Lazy Loading**: Connections are only created when needed, reducing startup time
///   and resource usage.
///
/// - **Graceful Degradation**: If no plugin is registered for a connection plugin name, the
///   factory returns `None` and the manager propagates this to the caller.
///
/// # Usage Counters
///
/// The manager tracks three types of operations per connection plugin name:
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
/// use async_trait::async_trait;
/// use std::sync::Arc;
/// use tokio::sync::Mutex;
/// use genja_core::inventory::{Connection, ConnectionKey, ConnectionManager};
///
/// #[derive(Debug)]
/// struct SshConnection {
///     alive: bool,
/// }
///
/// #[async_trait]
/// impl Connection for SshConnection {
///     fn create(&self, _key: &ConnectionKey) -> Box<dyn Connection> {
///         Box::new(SshConnection { alive: false })
///     }
///
///     fn is_alive(&self) -> bool {
///         self.alive
///     }
///
///     async fn open(&mut self, _params: &genja_core::inventory::ResolvedConnectionParams)
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
///     if key.plugin_name == "ssh" {
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
/// # use async_trait::async_trait;
/// # use std::sync::Arc;
/// # use tokio::sync::Mutex;
/// # use genja_core::inventory::{Connection, ConnectionKey, ConnectionManager};
/// # #[derive(Debug)]
/// # struct SshConnection { alive: bool }
/// # #[async_trait]
/// # impl Connection for SshConnection {
/// #     fn create(&self, _key: &ConnectionKey) -> Box<dyn Connection> {
/// #         Box::new(SshConnection { alive: false })
/// #     }
/// #     fn is_alive(&self) -> bool { self.alive }
/// #     async fn open(&mut self, _params: &genja_core::inventory::ResolvedConnectionParams)
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
/// # use async_trait::async_trait;
/// # use std::sync::Arc;
/// # use tokio::runtime::Builder;
/// # use tokio::sync::Mutex;
/// # use genja_core::inventory::{Connection, ConnectionKey, ConnectionManager, ResolvedConnectionParams};
/// # #[derive(Debug)]
/// # struct SshConnection { alive: bool }
/// # #[async_trait]
/// # impl Connection for SshConnection {
/// #     fn create(&self, _key: &ConnectionKey) -> Box<dyn Connection> {
/// #         Box::new(SshConnection { alive: false })
/// #     }
/// #     fn is_alive(&self) -> bool { self.alive }
/// #     async fn open(&mut self, _params: &ResolvedConnectionParams) -> Result<(), String> {
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
/// let runtime = Builder::new_current_thread().enable_all().build().unwrap();
/// runtime.block_on(async { manager.open_connection(&key, &params).await })?;
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
/// # use async_trait::async_trait;
/// # use std::sync::Arc;
/// # use tokio::sync::Mutex;
/// # use genja_core::inventory::{Connection, ConnectionKey, ConnectionManager};
/// # #[derive(Debug)]
/// # struct SshConnection { alive: bool }
/// # #[async_trait]
/// # impl Connection for SshConnection {
/// #     fn create(&self, _key: &ConnectionKey) -> Box<dyn Connection> {
/// #         Box::new(SshConnection { alive: false })
/// #     }
/// #     fn is_alive(&self) -> bool { self.alive }
/// #     async fn open(&mut self, _params: &genja_core::inventory::ResolvedConnectionParams)
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
    connection_locks: DashMap<ConnectionKey, Arc<Mutex<()>>>,
    connection_factory: RwLock<Option<Arc<ConnectionFactory>>>,
    counters: Arc<DashMap<String, ConnectionCounters>>,
}

impl fmt::Debug for ConnectionManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ConnectionManager")
            .field("connections_map_len", &self.connections_map.len())
            .field("connection_locks_len", &self.connection_locks.len())
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
            connection_locks: DashMap::new(),
            connection_factory: RwLock::new(Some(factory)),
            counters: Arc::new(DashMap::new()),
        }
    }

    /// Return the stable per-key lifecycle lock for cache ownership operations.
    ///
    /// The lock entry is intentionally retained for the manager lifetime once created.
    /// Removing it while another task still holds an `Arc` could allow two distinct locks
    /// for the same key and break replacement serialization.
    fn connection_lifecycle_lock(&self, key: &ConnectionKey) -> Arc<Mutex<()>> {
        self.connection_locks
            .entry(key.clone())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    /// Acquire exclusive lifecycle ownership for a connection key.
    ///
    /// Hold this guard across async cache operations that create, open, evict, replace,
    /// or insert `connections_map[key]`. The guard protects the cache slot; the connection
    /// object's own mutex still protects mutable plugin state on a specific instance.
    async fn lock_connection_lifecycle(&self, key: &ConnectionKey) -> OwnedMutexGuard<()> {
        self.connection_lifecycle_lock(key).lock_owned().await
    }

    /// Clone the configured connection factory or return the stable setup error.
    ///
    /// The returned `Arc` lets callers release the `RwLock` before invoking the factory,
    /// avoiding factory calls while the manager configuration lock is held.
    fn connection_factory(&self) -> Result<Arc<ConnectionFactory>, String> {
        let guard = self
            .connection_factory
            .read()
            .map_err(|_| "connection factory lock poisoned".to_string())?;
        guard
            .clone()
            .ok_or_else(|| "connection factory not set".to_string())
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
    /// This is a low-level synchronous helper and does not acquire the per-key
    /// lifecycle lock used by async lifecycle operations. Prefer
    /// [`open_connection`](Self::open_connection) when callers need a ready-to-use
    /// connection. New async methods that create, evict, replace, or open cached
    /// connections should acquire `lock_connection_lifecycle(...)` and then call the
    /// private `get_or_create_unlocked(...)` helper instead.
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
    /// * `key` - A `ConnectionKey` identifying the connection by hostname and connection plugin name.
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
    /// use async_trait::async_trait;
    /// use std::sync::Arc;
    /// use tokio::sync::Mutex;
    /// use genja_core::inventory::{Connection, ConnectionKey, ConnectionManager};
    ///
    /// #[derive(Debug)]
    /// struct DummyConnection;
    ///
    /// #[async_trait]
    /// impl Connection for DummyConnection {
    ///     fn create(&self, _key: &ConnectionKey) -> Box<dyn Connection> {
    ///         Box::new(DummyConnection)
    ///     }
    ///     fn is_alive(&self) -> bool { true }
    ///     async fn open(&mut self, _params: &genja_core::inventory::ResolvedConnectionParams)
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
        self.get_or_create_unlocked(key)
    }

    /// Return a cached connection or create one without taking the lifecycle lock.
    ///
    /// Call this only when the caller already holds `lock_connection_lifecycle(...)`
    /// for async lifecycle paths, or from legacy synchronous paths where taking the async
    /// lifecycle lock is not possible.
    fn get_or_create_unlocked(
        &self,
        key: ConnectionKey,
    ) -> Result<Option<Arc<Mutex<dyn Connection>>>, String> {
        let factory = self.connection_factory()?;

        match self.connections_map.entry(key) {
            dashmap::mapref::entry::Entry::Occupied(entry) => Ok(Some(entry.get().clone())),
            dashmap::mapref::entry::Entry::Vacant(entry) => {
                let key_for_factory = entry.key().clone();
                let connection_type = key_for_factory.plugin_name.clone();
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
            let mut connection = connection.blocking_lock();
            connection.close();
            self.increment_close(&key.plugin_name);
        }
    }

    /// Close all connections in `connections_map` and then clear the map.
    pub fn close_all_connections(&self) {
        self.connections_map.iter().for_each(|entry| {
            let mut connection = entry.value().blocking_lock();
            connection.close();
            self.increment_close(&entry.key().plugin_name);
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
    /// * `key` - A `ConnectionKey` identifying the connection by hostname and connection plugin name.
    ///   This key is used to look up or create the connection in the manager's map.
    /// * `params` - A `ResolvedConnectionParams` containing the connection parameters such as
    ///   hostname, port, username, password, and platform. These parameters are passed to the
    ///   connection's `open()` method if the connection needs to be established.
    ///
    /// # Thread Safety and Locking
    ///
    /// The method uses a layered locking strategy:
    ///
    /// 1. **Lifecycle Lock**: A per-key async mutex is acquired before the connection
    ///    cache is inspected or modified. The same lock is used by
    ///    [`replace_connection`](Self::replace_connection), preventing a replacement from
    ///    evicting a connection while another async opener recreates or retrieves the same
    ///    key. Different connection keys use different locks and can proceed concurrently.
    ///
    /// 2. **Factory Lock**: `get_or_create()` briefly acquires the factory's `RwLock` to clone
    ///    the `Arc<ConnectionFactory>`, then releases it before calling the factory function.
    ///    This prevents holding the factory lock during connection creation.
    ///
    /// 3. **Connection Lock**: After obtaining the connection, the method acquires its `Mutex`
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
    /// - The factory function returned `None` (e.g., no plugin registered for this connection plugin name)
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
    /// use async_trait::async_trait;
    /// use std::sync::Arc;
    /// use tokio::runtime::Builder;
    /// use tokio::sync::Mutex;
    /// use genja_core::inventory::{
    ///     Connection, ConnectionKey, ConnectionManager, ResolvedConnectionParams
    /// };
    ///
    /// #[derive(Debug)]
    /// struct SshConnection {
    ///     alive: bool,
    /// }
    ///
    /// #[async_trait]
    /// impl Connection for SshConnection {
    ///     fn create(&self, _key: &ConnectionKey) -> Box<dyn Connection> {
    ///         Box::new(SshConnection { alive: false })
    ///     }
    ///
    ///     fn is_alive(&self) -> bool {
    ///         self.alive
    ///     }
    ///
    ///     async fn open(&mut self, _params: &ResolvedConnectionParams) -> Result<(), String> {
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
    /// let runtime = Builder::new_current_thread().enable_all().build().unwrap();
    /// let connection = runtime.block_on(async { manager.open_connection(&key, &params).await })?;
    /// assert!(connection.is_some());
    ///
    /// // Second call reuses the existing connection without reopening
    /// let same_connection = runtime.block_on(async { manager.open_connection(&key, &params).await })?;
    /// assert!(Arc::ptr_eq(&connection.unwrap(), &same_connection.unwrap()));
    /// # Ok::<(), String>(())
    /// ```
    ///
    /// ## Handling Missing Plugins
    ///
    /// ```
    /// use std::sync::Arc;
    /// use tokio::runtime::Builder;
    /// use genja_core::inventory::{ConnectionKey, ConnectionManager, ResolvedConnectionParams};
    ///
    /// // Factory returns None for unknown connection plugin names
    /// let factory = Arc::new(|key: &ConnectionKey| {
    ///     if key.plugin_name == "ssh" {
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
    /// let runtime = Builder::new_current_thread().enable_all().build().unwrap();
    /// let result = runtime.block_on(async { manager.open_connection(&key, &params).await })?;
    /// assert!(result.is_none()); // No plugin available
    /// # Ok::<(), String>(())
    /// ```
    ///
    /// ## Thread-Safe Concurrent Access
    ///
    /// ```
    /// use async_trait::async_trait;
    /// use std::sync::Arc;
    /// use std::thread;
    /// use tokio::runtime::Builder;
    /// use tokio::sync::Mutex;
    /// use genja_core::inventory::{
    ///     Connection, ConnectionKey, ConnectionManager, ResolvedConnectionParams
    /// };
    ///
    /// # #[derive(Debug)]
    /// # struct SshConnection { alive: bool }
    /// # #[async_trait]
    /// # impl Connection for SshConnection {
    /// #     fn create(&self, _key: &ConnectionKey) -> Box<dyn Connection> {
    /// #         Box::new(SshConnection { alive: false })
    /// #     }
    /// #     fn is_alive(&self) -> bool { self.alive }
    /// #     async fn open(&mut self, _params: &ResolvedConnectionParams) -> Result<(), String> {
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
    /// let runtime = Arc::new(Builder::new_current_thread().enable_all().build().unwrap());
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
    ///         let runtime = Arc::clone(&runtime);
    ///         let key = key.clone();
    ///         let params = Arc::clone(&params);
    ///         thread::spawn(move || {
    ///             runtime.block_on(async { manager.open_connection(&key, &params).await })
    ///         })
    ///     })
    ///     .collect();
    ///
    /// for handle in handles {
    ///     let result = handle.join().unwrap();
    ///     assert!(result.is_ok());
    /// }
    /// ```
    pub async fn open_connection(
        &self,
        key: &ConnectionKey,
        params: &ResolvedConnectionParams,
    ) -> Result<Option<Arc<Mutex<dyn Connection>>>, String> {
        let _lifecycle_guard = self.lock_connection_lifecycle(key).await;

        let Some(connection) = self.get_or_create_unlocked(key.clone())? else {
            return Ok(None);
        };

        {
            let mut guard = connection.lock().await;
            if !guard.is_alive() {
                guard.open(params).await?;
                self.increment_open(&key.plugin_name);
            }
        }
        Ok(Some(connection))
    }

    /// Replace the cached connection for `key` with a freshly created and opened instance.
    ///
    /// Replacement is serialized with [`open_connection`](Self::open_connection) for the
    /// same [`ConnectionKey`] by acquiring a per-key lifecycle lock before touching the
    /// connection cache. That lock is separate from the connection object's own mutex:
    /// the lifecycle lock protects ownership of the cache slot while entries are removed,
    /// created, opened, and inserted; the connection mutex protects mutable plugin state
    /// on a specific connection instance.
    ///
    /// The old connection is evicted before the replacement is created. Closing the old
    /// connection is best-effort and uses the old connection object only after it has been
    /// removed from the cache. If the factory returns `None`, this method leaves the old
    /// connection evicted and returns `Ok(None)`. If opening the replacement fails, the
    /// replacement is not inserted and the open error is returned.
    pub async fn replace_connection(
        &self,
        key: &ConnectionKey,
        params: &ResolvedConnectionParams,
    ) -> Result<Option<Arc<Mutex<dyn Connection>>>, String> {
        let _lifecycle_guard = self.lock_connection_lifecycle(key).await;

        if let Some((_, old_connection)) = self.connections_map.remove(key) {
            let mut old_connection = old_connection.lock().await;
            old_connection.close();
            self.increment_close(&key.plugin_name);
        }

        let factory = self.connection_factory()?;
        let Some(new_connection) = factory(key) else {
            return Ok(None);
        };
        self.increment_create(&key.plugin_name);

        {
            let mut new_connection_guard = new_connection.lock().await;
            new_connection_guard.open(params).await?;
            self.increment_open(&key.plugin_name);
        }

        self.connections_map
            .insert(key.clone(), new_connection.clone());
        Ok(Some(new_connection))
    }
}

impl Default for ConnectionManager {
    fn default() -> Self {
        Self {
            connections_map: DashMap::new(),
            connection_locks: DashMap::new(),
            connection_factory: RwLock::new(None),
            counters: Arc::new(DashMap::new()),
        }
    }
}
