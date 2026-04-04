use crate::{inventory::ConnectionKey, types::NatString};
use dashmap::DashMap;
use log::warn;



/// Per-host execution state for the current Genja instance.
///
/// This structure maintains thread-safe state tracking for hosts, connections, and tasks
/// within a Genja runtime. It uses concurrent hash maps ([`DashMap`]) to allow safe
/// concurrent access from multiple threads without requiring external synchronization.
///
/// # State Categories
///
/// The state is organized into three primary categories:
///
/// * **Host Status** - Tracks whether hosts are in scope or have failed, allowing the
///   runtime to exclude failed hosts from operations until they are explicitly restored.
///
/// * **Connection State** - Records connection attempt history for each host/plugin pair,
///   including the current connection status, number of attempts, and any error messages
///   from failed attempts.
///
/// * **Task State** - Tracks task execution state for each host/task pair, including
///   execution status, attempt counts, and error information.
///
/// # Thread Safety
///
/// All state maps use [`DashMap`] internally, which provides lock-free concurrent access
/// for most operations. This allows multiple threads to safely read and update state
/// without explicit locking.
///
/// # Examples
///
/// ```
/// # use genja_core::state::State;
/// let state = State::new();
///
/// // Track host failures
/// state.mark_failed("router1");
/// assert!(!state.is_in_scope("router1"));
///
/// // Track connection attempts
/// state.begin_connection_attempt("router2", "ssh");
/// state.mark_connection_connected("router2", "ssh");
///
/// // Restore failed hosts
/// state.mark_in_scope("router1");
/// assert!(state.is_in_scope("router1"));
/// ```
#[derive(Debug, Default)]
pub struct State {
    host_status: DashMap<NatString, HostStatus>,
    connection_state: DashMap<ConnectionKey, ConnectionAttemptState>,
    task_state: DashMap<TaskExecutionKey, TaskAttemptState>,
}

impl State {
    /// Create an empty state store.
    pub fn new() -> Self {
        Self {
            host_status: DashMap::new(),
            connection_state: DashMap::new(),
            task_state: DashMap::new(),
        }
    }

    /// Marks a host as failed and removes it from the active scope.
    ///
    /// When a host is marked as failed, it will be excluded from host views and
    /// operations until it is explicitly restored using [`mark_in_scope`](Self::mark_in_scope)
    /// or [`mark_in_scope_key`](Self::mark_in_scope_key).
    ///
    /// # Parameters
    ///
    /// * `name` - The hostname to mark as failed. Can be any type that converts into a `NatString`,
    ///   such as `&str`, `String`, or `NatString`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use genja_core::state::State;
    /// let state = State::new();
    /// state.mark_failed("router1");
    /// assert!(!state.is_in_scope("router1"));
    /// ```
    pub fn mark_failed<K>(&self, name: K)
    where
        K: Into<NatString>,
    {
        let name = name.into();
        warn!("host '{}' marked as failed", name);
        self.host_status.insert(name, HostStatus::Failed);
    }

    /// Marks a host as back in scope and restores it to the active host view.
    ///
    /// When a host is marked as in scope, it will be included in host views and
    /// operations. This is typically used to restore a host that was previously
    /// marked as failed using [`mark_failed`](Self::mark_failed).
    ///
    /// # Parameters
    ///
    /// * `name` - The hostname to mark as in scope. Can be any type that converts into a `NatString`,
    ///   such as `&str`, `String`, or `NatString`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use genja_core::state::State;
    /// let state = State::new();
    /// state.mark_failed("router1");
    /// state.mark_in_scope("router1");
    /// assert!(state.is_in_scope("router1"));
    /// ```
    pub fn mark_in_scope<K>(&self, name: K)
    where
        K: Into<NatString>,
    {
        self.host_status.insert(name.into(), HostStatus::InScope);
    }

    /// Marks a host as back in scope using an existing `NatString` key.
    ///
    /// This is a more efficient variant of [`mark_in_scope`](Self::mark_in_scope) when you
    /// already have a `NatString` reference, as it avoids an additional conversion.
    ///
    /// # Parameters
    ///
    /// * `key` - A reference to the `NatString` key representing the hostname to mark as in scope.
    ///
    /// # Examples
    ///
    /// ```
    /// # use genja_core::state::State;
    /// # use genja_core::types::NatString;
    /// let state = State::new();
    /// let host = NatString::from("router1");
    /// state.mark_failed(host.clone());
    /// state.mark_in_scope_key(&host);
    /// assert!(state.is_in_scope_key(&host));
    /// ```
    pub fn mark_in_scope_key(&self, key: &NatString) {
        self.host_status.insert(key.clone(), HostStatus::InScope);
    }

    /// Checks if a host is currently in scope and available for operations.
    ///
    /// A host is considered in scope unless it has been explicitly marked as failed
    /// using [`mark_failed`](Self::mark_failed). Hosts that have never been tracked
    /// are considered in scope by default.
    ///
    /// # Parameters
    ///
    /// * `name` - The hostname to check. Can be any type that converts into a `NatString`,
    ///   such as `&str`, `String`, or `NatString`.
    ///
    /// # Returns
    ///
    /// Returns `true` if the host is in scope (either explicitly marked as in scope or
    /// never tracked), `false` if the host has been marked as failed.
    ///
    /// # Examples
    ///
    /// ```
    /// # use genja_core::state::State;
    /// let state = State::new();
    ///
    /// // Untracked hosts are in scope by default
    /// assert!(state.is_in_scope("router1"));
    ///
    /// // Failed hosts are not in scope
    /// state.mark_failed("router1");
    /// assert!(!state.is_in_scope("router1"));
    ///
    /// // Restored hosts are back in scope
    /// state.mark_in_scope("router1");
    /// assert!(state.is_in_scope("router1"));
    /// ```
    pub fn is_in_scope<K>(&self, name: K) -> bool
    where
        K: Into<NatString>,
    {
        let key = name.into();
        self.is_in_scope_key(&key)
    }

    /// Returns the tracked host status for a host, if it has been explicitly set.
    ///
    /// This method retrieves the current status of a host if it has been tracked
    /// (i.e., marked as failed or explicitly set as in scope). If the host has never
    /// been tracked, this method returns `None`.
    ///
    /// # Parameters
    ///
    /// * `name` - The hostname to query. Can be any type that converts into a `NatString`,
    ///   such as `&str`, `String`, or `NatString`.
    ///
    /// # Returns
    ///
    /// Returns `Some(HostStatus)` if the host has been explicitly tracked, or `None`
    /// if the host has never been marked with a status. Note that returning `None`
    /// does not mean the host is out of scope; untracked hosts are considered in
    /// scope by default (see [`is_in_scope`](Self::is_in_scope)).
    ///
    /// # Examples
    ///
    /// ```
    /// # use genja_core::state::{State, HostStatus};
    /// let state = State::new();
    ///
    /// // Untracked hosts return None
    /// assert_eq!(state.host_status("router1"), None);
    ///
    /// // Tracked hosts return their status
    /// state.mark_failed("router1");
    /// assert_eq!(state.host_status("router1"), Some(HostStatus::Failed));
    ///
    /// state.mark_in_scope("router1");
    /// assert_eq!(state.host_status("router1"), Some(HostStatus::InScope));
    /// ```
    pub fn host_status<K>(&self, name: K) -> Option<HostStatus>
    where
        K: Into<NatString>,
    {
        let key = name.into();
        self.host_status_key(&key)
    }

    /// Returns the tracked host status for a host using an existing `NatString` key.
    ///
    /// This is a more efficient variant of [`host_status`](Self::host_status) when you
    /// already have a `NatString` reference, as it avoids an additional conversion.
    ///
    /// # Parameters
    ///
    /// * `key` - A reference to the `NatString` key representing the hostname to query.
    ///
    /// # Returns
    ///
    /// Returns `Some(HostStatus)` if the host has been explicitly tracked, or `None`
    /// if the host has never been marked with a status. Note that returning `None`
    /// does not mean the host is out of scope; untracked hosts are considered in
    /// scope by default (see [`is_in_scope`](Self::is_in_scope)).
    ///
    /// # Examples
    ///
    /// ```
    /// # use genja_core::state::{State, HostStatus};
    /// # use genja_core::types::NatString;
    /// let state = State::new();
    /// let host = NatString::from("router1");
    ///
    /// // Untracked hosts return None
    /// assert_eq!(state.host_status_key(&host), None);
    ///
    /// // Tracked hosts return their status
    /// state.mark_failed(host.clone());
    /// assert_eq!(state.host_status_key(&host), Some(HostStatus::Failed));
    ///
    /// state.mark_in_scope_key(&host);
    /// assert_eq!(state.host_status_key(&host), Some(HostStatus::InScope));
    /// ```
    pub fn host_status_key(&self, key: &NatString) -> Option<HostStatus> {
        self.host_status.get(key).map(|entry| *entry.value())
    }

    /// Checks if a host is currently in scope using an existing `NatString` key.
    ///
    /// This is a more efficient variant of [`is_in_scope`](Self::is_in_scope) when you
    /// already have a `NatString` reference, as it avoids an additional conversion.
    ///
    /// A host is considered in scope unless it has been explicitly marked as failed
    /// using [`mark_failed`](Self::mark_failed). Hosts that have never been tracked
    /// are considered in scope by default.
    ///
    /// # Parameters
    ///
    /// * `key` - A reference to the `NatString` key representing the hostname to check.
    ///
    /// # Returns
    ///
    /// Returns `true` if the host is in scope (either explicitly marked as in scope or
    /// never tracked), `false` if the host has been marked as failed.
    ///
    /// # Examples
    ///
    /// ```
    /// # use genja_core::state::State;
    /// # use genja_core::types::NatString;
    /// let state = State::new();
    /// let host = NatString::from("router1");
    ///
    /// // Untracked hosts are in scope by default
    /// assert!(state.is_in_scope_key(&host));
    ///
    /// // Failed hosts are not in scope
    /// state.mark_failed(host.clone());
    /// assert!(!state.is_in_scope_key(&host));
    ///
    /// // Restored hosts are back in scope
    /// state.mark_in_scope_key(&host);
    /// assert!(state.is_in_scope_key(&host));
    /// ```
    pub fn is_in_scope_key(&self, key: &NatString) -> bool {
        match self.host_status.get(key) {
            Some(status) => *status.value() == HostStatus::InScope,
            None => true,
        }
    }

    // TODO: Remove this method and use the accessors instead i.e., failed_hosts() and in_scope_hosts()
    // /// Return the raw host scope state map.
    // pub fn host_statuses(&self) -> &DashMap<NatString, HostStatus> {
    //     &self.host_status
    // }

    /// Sets the connection attempt state for a specific host and plugin combination.
    ///
    /// This method records the current state of a connection attempt, including the
    /// connection status, number of attempts made, and any error information. The state
    /// is stored using a `ConnectionKey` composed of the host and plugin name.
    ///
    /// If the connection state indicates a failure, a warning will be logged with details
    /// about the failure kind and any associated error message.
    ///
    /// # Parameters
    ///
    /// * `host` - The hostname for which to set the connection state. Can be any type that
    ///   converts into a `String`, such as `&str`, `String`, or other string-like types.
    ///
    /// * `plugin_name` - The name of the connection plugin being used. Can be any type that
    ///   converts into a `String`, such as `&str`, `String`, or other string-like types.
    ///
    /// * `state` - The `ConnectionAttemptState` to record, containing the connection status,
    ///   attempt count, and optional error information.
    ///
    /// # Examples
    ///
    /// ```
    /// # use genja_core::state::{State, ConnectionAttemptState, ConnectionStatus};
    /// let state = State::new();
    ///
    /// // Record a successful connection
    /// let connection_state = ConnectionAttemptState::new(ConnectionStatus::Connected)
    ///     .with_attempts(1);
    /// state.set_connection_state("router1", "ssh", connection_state);
    ///
    /// // Verify the state was recorded
    /// assert_eq!(
    ///     state.connection_state("router1", "ssh").map(|s| s.status),
    ///     Some(ConnectionStatus::Connected)
    /// );
    /// ```
    pub fn set_connection_state(
        &self,
        host: impl Into<String>,
        plugin_name: impl Into<String>,
        state: ConnectionAttemptState,
    ) {
        self.set_connection_state_key(ConnectionKey::new(host, plugin_name), state);
    }

    /// Sets the connection attempt state using an existing `ConnectionKey`.
    ///
    /// This is a more efficient variant of [`set_connection_state`](Self::set_connection_state)
    /// when you already have a `ConnectionKey`, as it avoids constructing a new key from
    /// separate host and plugin name components.
    ///
    /// If the connection state indicates a failure, a warning will be logged with details
    /// about the failure kind and any associated error message.
    ///
    /// # Parameters
    ///
    /// * `key` - The `ConnectionKey` identifying the host and plugin combination for which
    ///   to set the connection state.
    ///
    /// * `state` - The `ConnectionAttemptState` to record, containing the connection status,
    ///   attempt count, and optional error information.
    ///
    /// # Examples
    ///
    /// ```
    /// # use genja_core::state::{State, ConnectionAttemptState, ConnectionStatus};
    /// # use genja_core::inventory::ConnectionKey;
    /// let state = State::new();
    /// let key = ConnectionKey::new("router1", "ssh");
    ///
    /// // Record a successful connection
    /// let connection_state = ConnectionAttemptState::new(ConnectionStatus::Connected)
    ///     .with_attempts(1);
    /// state.set_connection_state_key(key.clone(), connection_state);
    ///
    /// // Verify the state was recorded
    /// assert_eq!(
    ///     state.connection_state_key(&key).map(|s| s.status),
    ///     Some(ConnectionStatus::Connected)
    /// );
    /// ```
    pub fn set_connection_state_key(&self, key: ConnectionKey, state: ConnectionAttemptState) {
        if let ConnectionStatus::Failed(kind) = &state.status {
            match &state.last_error {
                Some(error) => warn!(
                    "connection failed for host '{}' via plugin '{}' ({kind:?}): {error}",
                    key.hostname, key.plugin_name
                ),
                None => warn!(
                    "connection failed for host '{}' via plugin '{}' ({kind:?})",
                    key.hostname, key.plugin_name
                ),
            }
        }
        self.connection_state.insert(key, state);
    }

    /// Retrieves the current connection attempt state for a specific host and plugin combination.
    ///
    /// This method looks up the connection state using the provided host and plugin name,
    /// returning the current state if it exists. The state includes information about the
    /// connection status, number of attempts made, and any error from the last failed attempt.
    ///
    /// # Parameters
    ///
    /// * `host` - The hostname for which to retrieve the connection state.
    ///
    /// * `plugin_name` - The name of the connection plugin being used.
    ///
    /// # Returns
    ///
    /// Returns `Some(ConnectionAttemptState)` if a connection state has been recorded for
    /// the given host and plugin combination, or `None` if no connection attempts have been
    /// tracked for this combination.
    ///
    /// # Examples
    ///
    /// ```
    /// # use genja_core::state::{State, ConnectionAttemptState, ConnectionStatus};
    /// let state = State::new();
    ///
    /// // No state recorded yet
    /// assert_eq!(state.connection_state("router1", "ssh"), None);
    ///
    /// // After recording a connection attempt
    /// state.begin_connection_attempt("router1", "ssh");
    /// let connection_state = state.connection_state("router1", "ssh");
    /// assert!(connection_state.is_some());
    /// assert_eq!(connection_state.unwrap().status, ConnectionStatus::Connecting);
    /// ```
    pub fn connection_state(
        &self,
        host: &str,
        plugin_name: &str,
    ) -> Option<ConnectionAttemptState> {
        let key = ConnectionKey::new(host, plugin_name);
        self.connection_state.get(&key).map(|entry| entry.value().clone())
    }

    /// Retrieves the current connection attempt state using an existing `ConnectionKey`.
    ///
    /// This is a more efficient variant of [`connection_state`](Self::connection_state) when you
    /// already have a `ConnectionKey`, as it avoids constructing a new key from separate host
    /// and plugin name components.
    ///
    /// # Parameters
    ///
    /// * `key` - A reference to the `ConnectionKey` identifying the host and plugin combination
    ///   for which to retrieve the connection state.
    ///
    /// # Returns
    ///
    /// Returns `Some(ConnectionAttemptState)` if a connection state has been recorded for
    /// the given key, or `None` if no connection attempts have been tracked for this
    /// host and plugin combination.
    ///
    /// # Examples
    ///
    /// ```
    /// # use genja_core::state::{State, ConnectionAttemptState, ConnectionStatus};
    /// # use genja_core::inventory::ConnectionKey;
    /// let state = State::new();
    /// let key = ConnectionKey::new("router1", "ssh");
    ///
    /// // No state recorded yet
    /// assert_eq!(state.connection_state_key(&key), None);
    ///
    /// // After recording a connection attempt
    /// state.begin_connection_attempt_key(key.clone());
    /// let connection_state = state.connection_state_key(&key);
    /// assert!(connection_state.is_some());
    /// assert_eq!(connection_state.unwrap().status, ConnectionStatus::Connecting);
    /// ```
    pub fn connection_state_key(&self, key: &ConnectionKey) -> Option<ConnectionAttemptState> {
        self.connection_state.get(key).map(|entry| entry.value().clone())
    }

    // TODO: Remove direct access and create an iterator, i.e., failed_connections(), open_connections(), etc.
    // /// Return the raw connection state map.
    // pub fn connection_states(&self) -> &DashMap<ConnectionKey, ConnectionAttemptState> {
    //     &self.connection_state
    // }

    /// Records the start of a connection attempt and increments the attempt counter.
    ///
    /// This method marks the beginning of a new connection attempt for a specific host and
    /// plugin combination. It automatically increments the attempt counter, tracking how many
    /// times a connection has been attempted for this host/plugin pair. The connection status
    /// is set to `ConnectionStatus::Connecting`.
    ///
    /// If this is the first connection attempt for the given host and plugin, the attempt
    /// counter starts at 1. For subsequent attempts, the counter is incremented from its
    /// previous value.
    ///
    /// # Parameters
    ///
    /// * `host` - The hostname for which to record the connection attempt. Can be any type that
    ///   converts into a `String`, such as `&str`, `String`, or other string-like types.
    ///
    /// * `plugin_name` - The name of the connection plugin being used for the attempt. Can be
    ///   any type that converts into a `String`, such as `&str`, `String`, or other string-like types.
    ///
    /// # Examples
    ///
    /// ```
    /// # use genja_core::state::{State, ConnectionStatus};
    /// let state = State::new();
    ///
    /// // First connection attempt
    /// state.begin_connection_attempt("router1", "ssh");
    /// let connection_state = state.connection_state("router1", "ssh").unwrap();
    /// assert_eq!(connection_state.status, ConnectionStatus::Connecting);
    /// assert_eq!(connection_state.attempts, 1);
    ///
    /// // Second connection attempt
    /// state.begin_connection_attempt("router1", "ssh");
    /// let connection_state = state.connection_state("router1", "ssh").unwrap();
    /// assert_eq!(connection_state.attempts, 2);
    /// ```
    pub fn begin_connection_attempt(
        &self,
        host: impl Into<String>,
        plugin_name: impl Into<String>,
    ) {
        self.begin_connection_attempt_key(ConnectionKey::new(host, plugin_name));
    }

    /// Records the start of a connection attempt using an existing `ConnectionKey` and increments the attempt counter.
    ///
    /// This is a more efficient variant of [`begin_connection_attempt`](Self::begin_connection_attempt)
    /// when you already have a `ConnectionKey`, as it avoids constructing a new key from separate
    /// host and plugin name components.
    ///
    /// This method marks the beginning of a new connection attempt for the specified connection key.
    /// It automatically increments the attempt counter, tracking how many times a connection has been
    /// attempted for this host/plugin pair. The connection status is set to `ConnectionStatus::Connecting`.
    ///
    /// If this is the first connection attempt for the given key, the attempt counter starts at 1.
    /// For subsequent attempts, the counter is incremented from its previous value.
    ///
    /// # Parameters
    ///
    /// * `key` - The `ConnectionKey` identifying the host and plugin combination for which to record
    ///   the connection attempt.
    ///
    /// # Examples
    ///
    /// ```
    /// # use genja_core::state::{State, ConnectionStatus};
    /// # use genja_core::inventory::ConnectionKey;
    /// let state = State::new();
    /// let key = ConnectionKey::new("router1", "ssh");
    ///
    /// // First connection attempt
    /// state.begin_connection_attempt_key(key.clone());
    /// let connection_state = state.connection_state_key(&key).unwrap();
    /// assert_eq!(connection_state.status, ConnectionStatus::Connecting);
    /// assert_eq!(connection_state.attempts, 1);
    ///
    /// // Second connection attempt
    /// state.begin_connection_attempt_key(key.clone());
    /// let connection_state = state.connection_state_key(&key).unwrap();
    /// assert_eq!(connection_state.attempts, 2);
    /// ```
    pub fn begin_connection_attempt_key(&self, key: ConnectionKey) {
        let attempts = self
            .connection_state_key(&key)
            .map(|state| state.attempts + 1)
            .unwrap_or(1);

        self.set_connection_state_key(
            key,
            ConnectionAttemptState::new(ConnectionStatus::Connecting).with_attempts(attempts),
        );
    }

    /// Marks a connection as successfully established while preserving the attempt count.
    ///
    /// This method updates the connection state to `ConnectionStatus::Connected`, indicating
    /// that a connection has been successfully established for the specified host and plugin
    /// combination. The attempt counter is preserved from any previous connection attempts,
    /// allowing you to track how many attempts were needed before the connection succeeded.
    ///
    /// If no previous connection attempts were recorded, the attempt count will be set to 0.
    ///
    /// # Parameters
    ///
    /// * `host` - The hostname for which to mark the connection as connected. Can be any type
    ///   that converts into a `String`, such as `&str`, `String`, or other string-like types.
    ///
    /// * `plugin_name` - The name of the connection plugin that successfully established the
    ///   connection. Can be any type that converts into a `String`, such as `&str`, `String`,
    ///   or other string-like types.
    ///
    /// # Examples
    ///
    /// ```
    /// # use genja_core::state::{State, ConnectionStatus};
    /// let state = State::new();
    ///
    /// // Record connection attempts and then mark as connected
    /// state.begin_connection_attempt("router1", "ssh");
    /// state.begin_connection_attempt("router1", "ssh");
    /// state.mark_connection_connected("router1", "ssh");
    ///
    /// // Verify the connection is marked as connected with preserved attempt count
    /// let connection_state = state.connection_state("router1", "ssh").unwrap();
    /// assert_eq!(connection_state.status, ConnectionStatus::Connected);
    /// assert_eq!(connection_state.attempts, 2);
    /// ```
    pub fn mark_connection_connected(
        &self,
        host: impl Into<String>,
        plugin_name: impl Into<String>,
    ) {
        self.mark_connection_connected_key(ConnectionKey::new(host, plugin_name));
    }

    /// Marks a connection as successfully established using an existing `ConnectionKey` while preserving the attempt count.
    ///
    /// This is a more efficient variant of [`mark_connection_connected`](Self::mark_connection_connected)
    /// when you already have a `ConnectionKey`, as it avoids constructing a new key from separate
    /// host and plugin name components.
    ///
    /// This method updates the connection state to `ConnectionStatus::Connected`, indicating
    /// that a connection has been successfully established. The attempt counter is preserved from
    /// any previous connection attempts, allowing you to track how many attempts were needed
    /// before the connection succeeded.
    ///
    /// If no previous connection attempts were recorded, the attempt count will be set to 0.
    ///
    /// # Parameters
    ///
    /// * `key` - The `ConnectionKey` identifying the host and plugin combination for which to
    ///   mark the connection as connected.
    ///
    /// # Examples
    ///
    /// ```
    /// # use genja_core::state::{State, ConnectionStatus};
    /// # use genja_core::inventory::ConnectionKey;
    /// let state = State::new();
    /// let key = ConnectionKey::new("router1", "ssh");
    ///
    /// // Record connection attempts and then mark as connected
    /// state.begin_connection_attempt_key(key.clone());
    /// state.begin_connection_attempt_key(key.clone());
    /// state.mark_connection_connected_key(key.clone());
    ///
    /// // Verify the connection is marked as connected with preserved attempt count
    /// let connection_state = state.connection_state_key(&key).unwrap();
    /// assert_eq!(connection_state.status, ConnectionStatus::Connected);
    /// assert_eq!(connection_state.attempts, 2);
    /// ```
    pub fn mark_connection_connected_key(&self, key: ConnectionKey) {
        let attempts = self
            .connection_state_key(&key)
            .map(|state| state.attempts)
            .unwrap_or(0);

        self.set_connection_state_key(
            key,
            ConnectionAttemptState::new(ConnectionStatus::Connected).with_attempts(attempts),
        );
    }

    /// Marks a connection as pending retry while preserving the attempt count and recording the error.
    ///
    /// This method updates the connection state to `ConnectionStatus::RetryPending`, indicating
    /// that a connection attempt has failed but will be retried. The attempt counter is preserved
    /// from any previous connection attempts, and the provided error message is stored for
    /// diagnostic purposes.
    ///
    /// If no previous connection attempts were recorded, the attempt count will be set to 0.
    ///
    /// # Parameters
    ///
    /// * `host` - The hostname for which to mark the connection as pending retry. Can be any type
    ///   that converts into a `String`, such as `&str`, `String`, or other string-like types.
    ///
    /// * `plugin_name` - The name of the connection plugin for which the retry is pending. Can be
    ///   any type that converts into a `String`, such as `&str`, `String`, or other string-like types.
    ///
    /// * `last_error` - The error message from the failed connection attempt that triggered the
    ///   retry. Can be any type that converts into a `String`, such as `&str`, `String`, or other
    ///   string-like types.
    ///
    /// # Examples
    ///
    /// ```
    /// # use genja_core::state::{State, ConnectionStatus};
    /// let state = State::new();
    ///
    /// // Record a connection attempt and mark as pending retry
    /// state.begin_connection_attempt("router1", "ssh");
    /// state.mark_connection_retry_pending("router1", "ssh", "connection timed out");
    ///
    /// // Verify the connection is marked as retry pending with error
    /// let connection_state = state.connection_state("router1", "ssh").unwrap();
    /// assert_eq!(connection_state.status, ConnectionStatus::RetryPending);
    /// assert_eq!(connection_state.attempts, 1);
    /// assert_eq!(connection_state.last_error, Some("connection timed out".to_string()));
    /// ```
    pub fn mark_connection_retry_pending(
        &self,
        host: impl Into<String>,
        plugin_name: impl Into<String>,
        last_error: impl Into<String>,
    ) {
        self.mark_connection_retry_pending_key(
            ConnectionKey::new(host, plugin_name),
            last_error,
        );
    }

    /// Mark a connection as pending retry while preserving the attempt count.
    pub fn mark_connection_retry_pending_key(
        &self,
        key: ConnectionKey,
        last_error: impl Into<String>,
    ) {
        let attempts = self
            .connection_state_key(&key)
            .map(|state| state.attempts)
            .unwrap_or(0);

        self.set_connection_state_key(
            key,
            ConnectionAttemptState::new(ConnectionStatus::RetryPending)
                .with_attempts(attempts)
                .with_last_error(last_error),
        );
    }

    /// Mark a connection as terminally failed while preserving the attempt count.
    pub fn mark_connection_failed(
        &self,
        host: impl Into<String>,
        plugin_name: impl Into<String>,
        kind: ConnectionFailureKind,
        last_error: impl Into<String>,
    ) {
        self.mark_connection_failed_key(
            ConnectionKey::new(host, plugin_name),
            kind,
            last_error,
        );
    }

    /// Mark a connection as terminally failed while preserving the attempt count.
    pub fn mark_connection_failed_key(
        &self,
        key: ConnectionKey,
        kind: ConnectionFailureKind,
        last_error: impl Into<String>,
    ) {
        let attempts = self
            .connection_state_key(&key)
            .map(|state| state.attempts)
            .unwrap_or(0);

        self.set_connection_state_key(
            key,
            ConnectionAttemptState::new(ConnectionStatus::Failed(kind))
                .with_attempts(attempts)
                .with_last_error(last_error),
        );
    }

    /// Set the current task attempt state for a host and task.
    pub fn set_task_state(
        &self,
        host: impl Into<String>,
        task_name: impl Into<String>,
        state: TaskAttemptState,
    ) {
        self.set_task_state_key(TaskExecutionKey::new(host, task_name), state);
    }

    /// Set the current task attempt state using an existing key.
    pub fn set_task_state_key(&self, key: TaskExecutionKey, state: TaskAttemptState) {
        if let TaskStatus::Failed(kind) = &state.status {
            match &state.last_error {
                Some(error) => warn!(
                    "task failed for host '{}' in task '{}' ({kind:?}): {error}",
                    key.host, key.task_name
                ),
                None => warn!(
                    "task failed for host '{}' in task '{}' ({kind:?})",
                    key.host, key.task_name
                ),
            }
        }
        self.task_state.insert(key, state);
    }

    /// Return the current task attempt state for a host and task.
    pub fn task_state(&self, host: &str, task_name: &str) -> Option<TaskAttemptState> {
        let key = TaskExecutionKey::new(host, task_name);
        self.task_state.get(&key).map(|entry| entry.value().clone())
    }

    /// Return the current task attempt state for an existing key.
    pub fn task_state_key(&self, key: &TaskExecutionKey) -> Option<TaskAttemptState> {
        self.task_state.get(key).map(|entry| entry.value().clone())
    }

    /// Return the raw task state map.
    pub fn task_states(&self) -> &DashMap<TaskExecutionKey, TaskAttemptState> {
        &self.task_state
    }
}

/// Host state within the current Genja runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostStatus {
    InScope,
    Failed,
}

impl Default for HostStatus {
    fn default() -> Self {
        HostStatus::InScope
    }
}

/// Runtime status of a connection attempt for a host/plugin pair.
///
/// This structure tracks the current state of connection attempts, including
/// the connection status, number of attempts made, and any error message from
/// the last failed attempt.
///
/// # Fields
///
/// * `status` - The current connection status (e.g., connecting, connected, failed).
/// * `attempts` - The number of connection attempts that have been made.
/// * `last_error` - An optional error message from the most recent failed connection attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionAttemptState {
    pub status: ConnectionStatus,
    pub attempts: usize,
    pub last_error: Option<String>,
}

impl ConnectionAttemptState {
    pub fn new(status: ConnectionStatus) -> Self {
        Self {
            status,
            attempts: 0,
            last_error: None,
        }
    }

    pub fn with_attempts(mut self, attempts: usize) -> Self {
        self.attempts = attempts;
        self
    }

    pub fn with_last_error(mut self, last_error: impl Into<String>) -> Self {
        self.last_error = Some(last_error.into());
        self
    }
}

/// High-level connection execution state for a host/plugin pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionStatus {
    NeverTried,
    Connecting,
    Connected,
    RetryPending,
    Failed(ConnectionFailureKind),
}

/// Classified connection failure kinds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionFailureKind {
    Timeout,
    Refused,
    Auth,
    Dns,
    Transport,
    Unknown,
}

/// Identifies a task execution for a specific host.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TaskExecutionKey {
    pub host: NatString,
    pub task_name: String,
}

impl TaskExecutionKey {
    pub fn new(host: impl Into<String>, task_name: impl Into<String>) -> Self {
        Self {
            host: NatString::new(host.into()),
            task_name: task_name.into(),
        }
    }
}

/// Runtime status of a task attempt for a host/task pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskAttemptState {
    pub status: TaskStatus,
    pub attempts: usize,
    pub last_error: Option<String>,
}

impl TaskAttemptState {
    pub fn new(status: TaskStatus) -> Self {
        Self {
            status,
            attempts: 0,
            last_error: None,
        }
    }

    pub fn with_attempts(mut self, attempts: usize) -> Self {
        self.attempts = attempts;
        self
    }

    pub fn with_last_error(mut self, last_error: impl Into<String>) -> Self {
        self.last_error = Some(last_error.into());
        self
    }
}

/// High-level task execution state for a host/task pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskStatus {
    Pending,
    Running,
    Succeeded,
    RetryPending,
    Failed(TaskFailureKind),
    Skipped,
}

/// Classified task failure kinds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskFailureKind {
    CommandFailed,
    ParseFailed,
    ValidationFailed,
    Timeout,
    DependencyFailed,
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{sync::Arc, thread};

    #[test]
    fn host_scope_defaults_to_in_scope() {
        let state = State::new();

        assert!(state.is_in_scope("router1"));
    }

    #[test]
    fn host_scope_can_be_marked_failed_and_restored() {
        let state = State::new();

        state.mark_failed("router1");
        assert!(!state.is_in_scope("router1"));
        assert_eq!(state.host_status("router1"), Some(HostStatus::Failed));

        state.mark_in_scope("router1");
        assert!(state.is_in_scope("router1"));
        assert_eq!(state.host_status("router1"), Some(HostStatus::InScope));
    }

    #[test]
    fn host_scope_accepts_natstring_inputs() {
        let state = State::new();
        let host = NatString::from("router1");

        state.mark_failed(host.clone());

        assert_eq!(state.host_status(&host), Some(HostStatus::Failed));
        assert!(!state.is_in_scope(host));
    }

    #[test]
    fn host_statuses_and_mark_in_scope_key_reflect_latest_scope() {
        let state = State::new();
        let host = NatString::from("router1");

        state.mark_failed(host.clone());
        assert_eq!(
            state.host_statuses().get(&host).map(|entry| *entry.value()),
            Some(HostStatus::Failed)
        );

        state.mark_in_scope_key(&host);
        assert_eq!(
            state.host_statuses().get(&host).map(|entry| *entry.value()),
            Some(HostStatus::InScope)
        );
    }

    #[test]
    fn stores_connection_state_by_host_and_plugin() {
        let state = State::new();
        let connection_state = ConnectionAttemptState::new(ConnectionStatus::Failed(
            ConnectionFailureKind::Timeout,
        ))
        .with_attempts(3)
        .with_last_error("timed out");

        state.set_connection_state("router1", "ssh", connection_state.clone());

        assert_eq!(
            state.connection_state("router1", "ssh"),
            Some(connection_state)
        );
    }

    #[test]
    fn stores_connection_state_by_key() {
        let state = State::new();
        let key = ConnectionKey::new("router1", "ssh");
        let connection_state = ConnectionAttemptState::new(ConnectionStatus::Connected)
            .with_attempts(2);

        state.set_connection_state_key(key.clone(), connection_state.clone());

        assert_eq!(state.connection_state_key(&key), Some(connection_state));
    }

    #[test]
    fn connection_states_accessor_exposes_stored_entries() {
        let state = State::new();
        let key = ConnectionKey::new("router1", "ssh");
        let connection_state = ConnectionAttemptState::new(ConnectionStatus::RetryPending)
            .with_attempts(1)
            .with_last_error("timed out");

        state.set_connection_state_key(key.clone(), connection_state.clone());

        assert_eq!(
            state
                .connection_states()
                .get(&key)
                .map(|entry| entry.value().clone()),
            Some(connection_state)
        );
    }

    #[test]
    fn begin_connection_attempt_sets_connecting_and_increments_attempts() {
        let state = State::new();
        let key = ConnectionKey::new("router1", "ssh");

        state.begin_connection_attempt_key(key.clone());
        assert_eq!(
            state.connection_state_key(&key),
            Some(ConnectionAttemptState::new(ConnectionStatus::Connecting).with_attempts(1))
        );

        state.begin_connection_attempt_key(key.clone());
        assert_eq!(
            state.connection_state_key(&key),
            Some(ConnectionAttemptState::new(ConnectionStatus::Connecting).with_attempts(2))
        );
    }

    #[test]
    fn mark_connection_connected_preserves_attempt_count() {
        let state = State::new();
        let key = ConnectionKey::new("router1", "ssh");

        state.begin_connection_attempt_key(key.clone());
        state.begin_connection_attempt_key(key.clone());
        state.mark_connection_connected_key(key.clone());

        assert_eq!(
            state.connection_state_key(&key),
            Some(ConnectionAttemptState::new(ConnectionStatus::Connected).with_attempts(2))
        );
    }

    #[test]
    fn mark_connection_retry_pending_preserves_attempts_and_error() {
        let state = State::new();
        let key = ConnectionKey::new("router1", "ssh");

        state.begin_connection_attempt_key(key.clone());
        state.mark_connection_retry_pending_key(key.clone(), "timed out");

        assert_eq!(
            state.connection_state_key(&key),
            Some(
                ConnectionAttemptState::new(ConnectionStatus::RetryPending)
                    .with_attempts(1)
                    .with_last_error("timed out")
            )
        );
    }

    #[test]
    fn mark_connection_failed_preserves_attempts_and_sets_failed_status() {
        let state = State::new();
        let key = ConnectionKey::new("router1", "ssh");

        state.begin_connection_attempt_key(key.clone());
        state.begin_connection_attempt_key(key.clone());
        state.mark_connection_failed_key(
            key.clone(),
            ConnectionFailureKind::Timeout,
            "timed out",
        );

        assert_eq!(
            state.connection_state_key(&key),
            Some(
                ConnectionAttemptState::new(ConnectionStatus::Failed(
                    ConnectionFailureKind::Timeout,
                ))
                .with_attempts(2)
                .with_last_error("timed out")
            )
        );
    }

    #[test]
    fn mark_connection_connected_without_prior_attempts_uses_zero_attempts() {
        let state = State::new();
        let key = ConnectionKey::new("router1", "ssh");

        state.mark_connection_connected_key(key.clone());

        assert_eq!(
            state.connection_state_key(&key),
            Some(ConnectionAttemptState::new(ConnectionStatus::Connected).with_attempts(0))
        );
    }

    #[test]
    fn mark_connection_retry_pending_without_prior_attempts_uses_zero_attempts() {
        let state = State::new();
        let key = ConnectionKey::new("router1", "ssh");

        state.mark_connection_retry_pending_key(key.clone(), "timed out");

        assert_eq!(
            state.connection_state_key(&key),
            Some(
                ConnectionAttemptState::new(ConnectionStatus::RetryPending)
                    .with_attempts(0)
                    .with_last_error("timed out")
            )
        );
    }

    #[test]
    fn mark_connection_failed_without_prior_attempts_uses_zero_attempts() {
        let state = State::new();
        let key = ConnectionKey::new("router1", "ssh");

        state.mark_connection_failed_key(
            key.clone(),
            ConnectionFailureKind::Timeout,
            "timed out",
        );

        assert_eq!(
            state.connection_state_key(&key),
            Some(
                ConnectionAttemptState::new(ConnectionStatus::Failed(
                    ConnectionFailureKind::Timeout,
                ))
                .with_attempts(0)
                .with_last_error("timed out")
            )
        );
    }

    #[test]
    fn begin_connection_attempt_wrapper_increments_attempts() {
        let state = State::new();

        state.begin_connection_attempt("router1", "ssh");
        state.begin_connection_attempt("router1", "ssh");

        assert_eq!(
            state.connection_state("router1", "ssh"),
            Some(ConnectionAttemptState::new(ConnectionStatus::Connecting).with_attempts(2))
        );
    }

    #[test]
    fn mark_connection_connected_wrapper_preserves_attempt_count() {
        let state = State::new();

        state.begin_connection_attempt("router1", "ssh");
        state.begin_connection_attempt("router1", "ssh");
        state.mark_connection_connected("router1", "ssh");

        assert_eq!(
            state.connection_state("router1", "ssh"),
            Some(ConnectionAttemptState::new(ConnectionStatus::Connected).with_attempts(2))
        );
    }

    #[test]
    fn mark_connection_retry_pending_wrapper_preserves_attempts_and_error() {
        let state = State::new();

        state.begin_connection_attempt("router1", "ssh");
        state.mark_connection_retry_pending("router1", "ssh", "timed out");

        assert_eq!(
            state.connection_state("router1", "ssh"),
            Some(
                ConnectionAttemptState::new(ConnectionStatus::RetryPending)
                    .with_attempts(1)
                    .with_last_error("timed out")
            )
        );
    }

    #[test]
    fn mark_connection_failed_wrapper_preserves_attempts_and_error() {
        let state = State::new();

        state.begin_connection_attempt("router1", "ssh");
        state.begin_connection_attempt("router1", "ssh");
        state.mark_connection_failed(
            "router1",
            "ssh",
            ConnectionFailureKind::Timeout,
            "timed out",
        );

        assert_eq!(
            state.connection_state("router1", "ssh"),
            Some(
                ConnectionAttemptState::new(ConnectionStatus::Failed(
                    ConnectionFailureKind::Timeout,
                ))
                .with_attempts(2)
                .with_last_error("timed out")
            )
        );
    }

    #[test]
    fn stores_task_state_by_host_and_task() {
        let state = State::new();
        let task_state = TaskAttemptState::new(TaskStatus::Failed(TaskFailureKind::ParseFailed))
            .with_attempts(1)
            .with_last_error("failed to parse show version output");

        state.set_task_state("router1", "show_version", task_state.clone());

        assert_eq!(state.task_state("router1", "show_version"), Some(task_state));
    }

    #[test]
    fn stores_task_state_by_key() {
        let state = State::new();
        let key = TaskExecutionKey::new("router1", "show_version");
        let task_state = TaskAttemptState::new(TaskStatus::Succeeded).with_attempts(1);

        state.set_task_state_key(key.clone(), task_state.clone());

        assert_eq!(state.task_state_key(&key), Some(task_state));
    }

    #[test]
    fn task_states_accessor_exposes_stored_entries() {
        let state = State::new();
        let key = TaskExecutionKey::new("router1", "show_version");
        let task_state = TaskAttemptState::new(TaskStatus::Running).with_attempts(1);

        state.set_task_state_key(key.clone(), task_state.clone());

        assert_eq!(
            state.task_states().get(&key).map(|entry| entry.value().clone()),
            Some(task_state)
        );
    }

    #[test]
    fn supports_concurrent_updates_across_maps() {
        const THREADS: usize = 8;
        const ATTEMPTS_PER_THREAD: usize = 200;

        let state = Arc::new(State::new());
        let mut handles = Vec::with_capacity(THREADS);

        for i in 0..THREADS {
            let state = Arc::clone(&state);
            handles.push(thread::spawn(move || {
                let host = format!("router{i}");

                for _ in 0..ATTEMPTS_PER_THREAD {
                    state.begin_connection_attempt(host.clone(), "ssh");
                }

                state.mark_failed(host.clone());
                state.mark_in_scope(host.clone());
                state.mark_connection_connected(host.clone(), "ssh");
                state.set_task_state(
                    host.clone(),
                    "show_version",
                    TaskAttemptState::new(TaskStatus::Succeeded).with_attempts(1),
                );
            }));
        }

        for handle in handles {
            handle.join().expect("worker thread panicked");
        }

        for i in 0..THREADS {
            let host = format!("router{i}");
            let connection = state
                .connection_state(&host, "ssh")
                .expect("missing connection state");

            assert_eq!(connection.status, ConnectionStatus::Connected);
            assert_eq!(connection.attempts, ATTEMPTS_PER_THREAD);
            assert!(state.is_in_scope(host.as_str()));
            assert_eq!(
                state.task_state(&host, "show_version"),
                Some(TaskAttemptState::new(TaskStatus::Succeeded).with_attempts(1))
            );
        }
    }
}
