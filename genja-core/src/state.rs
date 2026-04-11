//! Runtime state management for Genja execution.
//!
//! This module provides thread-safe state tracking for the Genja automation framework,
//! managing the lifecycle and status of hosts, connections, and task executions throughout
//! a playbook run. The state system uses concurrent data structures to enable safe access
//! from multiple threads while maintaining consistency across all tracked entities.
//!
//! # Overview
//!
//! The state module centers around the [`State`] structure, which maintains three primary
//! categories of runtime information:
//!
//! 1. **Host Status** - Tracks whether hosts are in scope (available for operations) or
//!    have been marked as failed and should be excluded from further operations.
//!
//! 2. **Connection State** - Records the status of connection attempts for each host/plugin
//!    combination, including the number of attempts made, current connection status, and
//!    error information from failed attempts.
//!
//! 3. **Task Execution State** - Maintains the execution status of tasks on individual hosts,
//!    tracking whether tasks are pending, running, succeeded, failed, or skipped, along with
//!    attempt counts and error details.
//!
//! All state tracking is designed to be thread-safe, allowing concurrent access from multiple
//! execution threads without requiring external synchronization.
//!
//! # Core Types
//!
//! ## State Management
//!
//! - [`State`] - The main state container that tracks all runtime information using concurrent
//!   hash maps. Provides methods for querying and updating host status, connection state, and
//!   task execution state.
//!
//! ## Host Status
//!
//! - [`HostStatus`] - Indicates whether a host is in scope or has been marked as failed.
//!   Hosts default to `InScope` and can be transitioned to `Failed` when errors occur.
//!
//! ## Connection Tracking
//!
//! - [`ConnectionAttemptState`] - Tracks the state of connection attempts for a specific
//!   host/plugin pair, including status, attempt count, and error information.
//!
//! - [`ConnectionStatus`] - Represents the current status of a connection attempt, from
//!   initial connection through success, retry, or failure states.
//!
//! - [`ConnectionFailureKind`] - Categorizes connection failures into specific types
//!   (timeout, authentication, DNS, etc.) for targeted error handling.
//!
//! ## Task Execution Tracking
//!
//! - [`TaskAttemptState`] - Tracks the state of task execution attempts for a specific
//!   host/task pair, including status, attempt count, and error information.
//!
//! - [`TaskExecutionKey`] - A composite key that uniquely identifies a task execution
//!   by combining hostname and task name.
//!
//! - [`TaskStatus`] - Represents the current status of a task execution, from pending
//!   through running, succeeded, failed, or skipped states.
//!
//! - [`TaskFailureKind`] - Categorizes task execution failures into specific types
//!   (command failed, parse failed, validation failed, etc.) for targeted error handling.
//!
//! # Usage Patterns
//!
//! ## Basic State Creation
//!
//! ```rust
//! use genja_core::state::State;
//!
//! let state = State::new();
//! ```
//!
//! ## Host Status Management
//!
//! ```rust
//! # use genja_core::state::{State, HostStatus};
//! # let state = State::new();
//! // Check if a host is in scope (defaults to true)
//! assert!(state.is_in_scope("router1"));
//!
//! // Mark a host as failed
//! state.mark_failed("router1");
//! assert_eq!(state.host_status("router1"), Some(HostStatus::Failed));
//!
//! // Restore a host to in-scope status
//! state.mark_in_scope("router1");
//! assert_eq!(state.host_status("router1"), Some(HostStatus::InScope));
//! ```
//!
//! ## Connection State Tracking
//!
//! ```rust
//! # use genja_core::state::{State, ConnectionAttemptState, ConnectionStatus, ConnectionFailureKind};
//! # let state = State::new();
//! // Begin a connection attempt
//! state.begin_connection_attempt("router1", "ssh");
//!
//! // Mark the connection as successful
//! state.mark_connection_connected("router1", "ssh");
//!
//! // Or mark it for retry with an error message
//! state.mark_connection_retry_pending("router1", "ssh", "connection timed out");
//!
//! // Or mark it as permanently failed
//! state.mark_connection_failed(
//!     "router1",
//!     "ssh",
//!     ConnectionFailureKind::Timeout,
//!     "connection timed out after 30 seconds"
//! );
//!
//! // Query the current connection state
//! if let Some(conn_state) = state.connection_state("router1", "ssh") {
//!     println!("Connection status: {:?}", conn_state.status);
//!     println!("Attempts made: {}", conn_state.attempts);
//!     if let Some(error) = conn_state.last_error {
//!         println!("Last error: {}", error);
//!     }
//! }
//! ```
//!
//! ## Task Execution State Tracking
//!
//! ```rust
//! # use genja_core::state::{State, TaskAttemptState, TaskStatus, TaskFailureKind};
//! # let state = State::new();
//! // Record a successful task execution
//! let task_state = TaskAttemptState::new(TaskStatus::Succeeded)
//!     .with_attempts(1);
//! state.set_task_state("router1", "show_version", task_state);
//!
//! // Record a failed task execution with error details
//! let failed_state = TaskAttemptState::new(
//!     TaskStatus::Failed(TaskFailureKind::ParseFailed)
//! )
//! .with_attempts(2)
//! .with_last_error("failed to parse command output");
//! state.set_task_state("router1", "configure_interface", failed_state);
//!
//! // Query task execution state
//! if let Some(task_state) = state.task_state("router1", "show_version") {
//!     println!("Task status: {:?}", task_state.status);
//!     println!("Attempts made: {}", task_state.attempts);
//! }
//! ```
//!
//! ## Using Keys for Efficient Lookups
//!
//! When you need to perform multiple operations on the same host/plugin or host/task
//! combination, using the `_key` variants of methods can be more efficient:
//!
//! ```rust
//! # use genja_core::state::State;
//! # use genja_core::inventory::ConnectionKey;
//! # let state = State::new();
//! // Create a key once
//! let key = ConnectionKey::new("router1", "ssh");
//!
//! // Use it for multiple operations
//! state.begin_connection_attempt_key(key.clone());
//! state.mark_connection_connected_key(key.clone());
//!
//! // Query using the same key
//! if let Some(conn_state) = state.connection_state_key(&key) {
//!     println!("Connection established after {} attempts", conn_state.attempts);
//! }
//! ```
//!
//! # Thread Safety
//!
//! All state tracking structures use `DashMap` internally, which provides concurrent
//! access without requiring external locks. This allows multiple threads to safely
//! update and query state simultaneously:
//!
//! ```rust
//! # use genja_core::state::State;
//! # use std::sync::Arc;
//! # use std::thread;
//! let state = Arc::new(State::new());
//!
//! let mut handles = vec![];
//! for i in 0..4 {
//!     let state = Arc::clone(&state);
//!     handles.push(thread::spawn(move || {
//!         let host = format!("router{}", i);
//!         state.begin_connection_attempt(&host, "ssh");
//!         state.mark_connection_connected(&host, "ssh");
//!     }));
//! }
//!
//! for handle in handles {
//!     handle.join().unwrap();
//! }
//! ```
//!
//! # Design Considerations
//!
//! ## Builder Pattern
//!
//! State structures like [`ConnectionAttemptState`] and [`TaskAttemptState`] use the
//! builder pattern for convenient construction and method chaining:
//!
//! ```rust
//! # use genja_core::state::{ConnectionAttemptState, ConnectionStatus, ConnectionFailureKind};
//! let state = ConnectionAttemptState::new(
//!     ConnectionStatus::Failed(ConnectionFailureKind::Timeout)
//! )
//! .with_attempts(3)
//! .with_last_error("connection timed out after 30 seconds");
//! ```
//!
//! ## Attempt Counting
//!
//! Connection and task attempt counters are preserved across state transitions,
//! allowing you to track the total number of attempts made even after a connection
//! succeeds or a task completes:
//!
//! ```rust
//! # use genja_core::state::State;
//! # let state = State::new();
//! state.begin_connection_attempt("router1", "ssh"); // attempts = 1
//! state.begin_connection_attempt("router1", "ssh"); // attempts = 2
//! state.mark_connection_connected("router1", "ssh"); // attempts still = 2
//! ```
//!
//! ## Error Tracking
//!
//! Error messages from failed attempts are preserved in the state, allowing for
//! detailed diagnostics and logging:
//!
//! ```rust
//! # use genja_core::state::{State, ConnectionFailureKind};
//! # let state = State::new();
//! state.begin_connection_attempt("router1", "ssh");
//! state.mark_connection_failed(
//!     "router1",
//!     "ssh",
//!     ConnectionFailureKind::Auth,
//!     "authentication failed: invalid credentials"
//! );
//!
//! if let Some(conn_state) = state.connection_state("router1", "ssh") {
//!     if let Some(error) = conn_state.last_error {
//!         eprintln!("Connection failed: {}", error);
//!     }
//! }
//! ```
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

    /// Marks a connection as pending retry using an existing `ConnectionKey` while preserving the attempt count and recording the error.
    ///
    /// This is a more efficient variant of [`mark_connection_retry_pending`](Self::mark_connection_retry_pending)
    /// when you already have a `ConnectionKey`, as it avoids constructing a new key from separate
    /// host and plugin name components.
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
    /// * `key` - The `ConnectionKey` identifying the host and plugin combination for which to
    ///   mark the connection as pending retry.
    ///
    /// * `last_error` - The error message from the failed connection attempt that triggered the
    ///   retry. Can be any type that converts into a `String`, such as `&str`, `String`, or other
    ///   string-like types.
    ///
    /// # Examples
    ///
    /// ```
    /// # use genja_core::state::{State, ConnectionStatus};
    /// # use genja_core::inventory::ConnectionKey;
    /// let state = State::new();
    /// let key = ConnectionKey::new("router1", "ssh");
    ///
    /// // Record a connection attempt and mark as pending retry
    /// state.begin_connection_attempt_key(key.clone());
    /// state.mark_connection_retry_pending_key(key.clone(), "connection timed out");
    ///
    /// // Verify the connection is marked as retry pending with error
    /// let connection_state = state.connection_state_key(&key).unwrap();
    /// assert_eq!(connection_state.status, ConnectionStatus::RetryPending);
    /// assert_eq!(connection_state.attempts, 1);
    /// assert_eq!(connection_state.last_error, Some("connection timed out".to_string()));
    /// ```
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

    /// Marks a connection as terminally failed while preserving the attempt count and recording the error.
    ///
    /// This method updates the connection state to `ConnectionStatus::Failed` with the specified
    /// failure kind, indicating that a connection attempt has failed and will not be retried. The
    /// attempt counter is preserved from any previous connection attempts, and the provided error
    /// message is stored for diagnostic purposes.
    ///
    /// If no previous connection attempts were recorded, the attempt count will be set to 0.
    ///
    /// # Parameters
    ///
    /// * `host` - The hostname for which to mark the connection as failed. Can be any type that
    ///   converts into a `String`, such as `&str`, `String`, or other string-like types.
    ///
    /// * `plugin_name` - The name of the connection plugin for which the connection failed. Can be
    ///   any type that converts into a `String`, such as `&str`, `String`, or other string-like types.
    ///
    /// * `kind` - The `ConnectionFailureKind` classifying the type of failure (e.g., timeout,
    ///   authentication failure, DNS error).
    ///
    /// * `last_error` - The error message from the failed connection attempt. Can be any type that
    ///   converts into a `String`, such as `&str`, `String`, or other string-like types.
    ///
    /// # Examples
    ///
    /// ```
    /// # use genja_core::state::{State, ConnectionStatus, ConnectionFailureKind};
    /// let state = State::new();
    ///
    /// // Record connection attempts and mark as failed
    /// state.begin_connection_attempt("router1", "ssh");
    /// state.begin_connection_attempt("router1", "ssh");
    /// state.mark_connection_failed(
    ///     "router1",
    ///     "ssh",
    ///     ConnectionFailureKind::Timeout,
    ///     "connection timed out after 30 seconds"
    /// );
    ///
    /// // Verify the connection is marked as failed with error details
    /// let connection_state = state.connection_state("router1", "ssh").unwrap();
    /// assert_eq!(connection_state.status, ConnectionStatus::Failed(ConnectionFailureKind::Timeout));
    /// assert_eq!(connection_state.attempts, 2);
    /// assert_eq!(connection_state.last_error, Some("connection timed out after 30 seconds".to_string()));
    /// ```
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

    /// Marks a connection as terminally failed using an existing `ConnectionKey` while preserving the attempt count and recording the error.
    ///
    /// This is a more efficient variant of [`mark_connection_failed`](Self::mark_connection_failed)
    /// when you already have a `ConnectionKey`, as it avoids constructing a new key from separate
    /// host and plugin name components.
    ///
    /// This method updates the connection state to `ConnectionStatus::Failed` with the specified
    /// failure kind, indicating that a connection attempt has failed and will not be retried. The
    /// attempt counter is preserved from any previous connection attempts, and the provided error
    /// message is stored for diagnostic purposes.
    ///
    /// If no previous connection attempts were recorded, the attempt count will be set to 0.
    ///
    /// # Parameters
    ///
    /// * `key` - The `ConnectionKey` identifying the host and plugin combination for which to
    ///   mark the connection as failed.
    ///
    /// * `kind` - The `ConnectionFailureKind` classifying the type of failure (e.g., timeout,
    ///   authentication failure, DNS error).
    ///
    /// * `last_error` - The error message from the failed connection attempt. Can be any type that
    ///   converts into a `String`, such as `&str`, `String`, or other string-like types.
    ///
    /// # Examples
    ///
    /// ```
    /// # use genja_core::state::{State, ConnectionStatus, ConnectionFailureKind};
    /// # use genja_core::inventory::ConnectionKey;
    /// let state = State::new();
    /// let key = ConnectionKey::new("router1", "ssh");
    ///
    /// // Record connection attempts and mark as failed
    /// state.begin_connection_attempt_key(key.clone());
    /// state.begin_connection_attempt_key(key.clone());
    /// state.mark_connection_failed_key(
    ///     key.clone(),
    ///     ConnectionFailureKind::Timeout,
    ///     "connection timed out after 30 seconds"
    /// );
    ///
    /// // Verify the connection is marked as failed with error details
    /// let connection_state = state.connection_state_key(&key).unwrap();
    /// assert_eq!(connection_state.status, ConnectionStatus::Failed(ConnectionFailureKind::Timeout));
    /// assert_eq!(connection_state.attempts, 2);
    /// assert_eq!(connection_state.last_error, Some("connection timed out after 30 seconds".to_string()));
    /// ```
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

    /// Sets the task execution state for a specific host and task combination.
    ///
    /// This method records the current state of a task execution, including the
    /// task status, number of attempts made, and any error information. The state
    /// is stored using a `TaskExecutionKey` composed of the host and task name.
    ///
    /// If the task state indicates a failure, a warning will be logged with details
    /// about the failure kind and any associated error message.
    ///
    /// # Parameters
    ///
    /// * `host` - The hostname for which to set the task state. Can be any type that
    ///   converts into a `String`, such as `&str`, `String`, or other string-like types.
    ///
    /// * `task_name` - The name of the task being executed. Can be any type that
    ///   converts into a `String`, such as `&str`, `String`, or other string-like types.
    ///
    /// * `state` - The `TaskAttemptState` to record, containing the task status,
    ///   attempt count, and optional error information.
    ///
    /// # Examples
    ///
    /// ```
    /// # use genja_core::state::{State, TaskAttemptState, TaskStatus};
    /// let state = State::new();
    ///
    /// // Record a successful task execution
    /// let task_state = TaskAttemptState::new(TaskStatus::Succeeded)
    ///     .with_attempts(1);
    /// state.set_task_state("router1", "show_version", task_state);
    ///
    /// // Verify the state was recorded
    /// assert_eq!(
    ///     state.task_state("router1", "show_version").map(|s| s.status),
    ///     Some(TaskStatus::Succeeded)
    /// );
    /// ```
    pub fn set_task_state(
        &self,
        host: impl Into<String>,
        task_name: impl Into<String>,
        state: TaskAttemptState,
    ) {
        self.set_task_state_key(TaskExecutionKey::new(host, task_name), state);
    }

    /// Sets the task execution state using an existing `TaskExecutionKey`.
    ///
    /// This is a more efficient variant of [`set_task_state`](Self::set_task_state)
    /// when you already have a `TaskExecutionKey`, as it avoids constructing a new key from
    /// separate host and task name components.
    ///
    /// If the task state indicates a failure, a warning will be logged with details
    /// about the failure kind and any associated error message.
    ///
    /// # Parameters
    ///
    /// * `key` - The `TaskExecutionKey` identifying the host and task combination for which
    ///   to set the task state.
    ///
    /// * `state` - The `TaskAttemptState` to record, containing the task status,
    ///   attempt count, and optional error information.
    ///
    /// # Examples
    ///
    /// ```
    /// # use genja_core::state::{State, TaskAttemptState, TaskStatus, TaskExecutionKey};
    /// let state = State::new();
    /// let key = TaskExecutionKey::new("router1", "show_version");
    ///
    /// // Record a successful task execution
    /// let task_state = TaskAttemptState::new(TaskStatus::Succeeded)
    ///     .with_attempts(1);
    /// state.set_task_state_key(key.clone(), task_state);
    ///
    /// // Verify the state was recorded
    /// assert_eq!(
    ///     state.task_state_key(&key).map(|s| s.status),
    ///     Some(TaskStatus::Succeeded)
    /// );
    /// ```
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

    /// Retrieves the current task execution state for a specific host and task combination.
    ///
    /// This method looks up the task state using the provided host and task name,
    /// returning the current state if it exists. The state includes information about the
    /// task status, number of attempts made, and any error from the last failed attempt.
    ///
    /// # Parameters
    ///
    /// * `host` - The hostname for which to retrieve the task state.
    ///
    /// * `task_name` - The name of the task being queried.
    ///
    /// # Returns
    ///
    /// Returns `Some(TaskAttemptState)` if a task state has been recorded for
    /// the given host and task combination, or `None` if no task execution has been
    /// tracked for this combination.
    ///
    /// # Examples
    ///
    /// ```
    /// # use genja_core::state::{State, TaskAttemptState, TaskStatus};
    /// let state = State::new();
    ///
    /// // No state recorded yet
    /// assert_eq!(state.task_state("router1", "show_version"), None);
    ///
    /// // After recording a task execution
    /// let task_state = TaskAttemptState::new(TaskStatus::Running).with_attempts(1);
    /// state.set_task_state("router1", "show_version", task_state.clone());
    /// assert_eq!(state.task_state("router1", "show_version"), Some(task_state));
    /// ```
    pub fn task_state(&self, host: &str, task_name: &str) -> Option<TaskAttemptState> {
        let key = TaskExecutionKey::new(host, task_name);
        self.task_state.get(&key).map(|entry| entry.value().clone())
    }

    /// Retrieves the current task execution state using an existing `TaskExecutionKey`.
    ///
    /// This is a more efficient variant of [`task_state`](Self::task_state) when you
    /// already have a `TaskExecutionKey`, as it avoids constructing a new key from separate
    /// host and task name components.
    ///
    /// # Parameters
    ///
    /// * `key` - A reference to the `TaskExecutionKey` identifying the host and task combination
    ///   for which to retrieve the task state.
    ///
    /// # Returns
    ///
    /// Returns `Some(TaskAttemptState)` if a task state has been recorded for
    /// the given key, or `None` if no task execution has been tracked for this
    /// host and task combination.
    ///
    /// # Examples
    ///
    /// ```
    /// # use genja_core::state::{State, TaskAttemptState, TaskStatus, TaskExecutionKey};
    /// let state = State::new();
    /// let key = TaskExecutionKey::new("router1", "show_version");
    ///
    /// // No state recorded yet
    /// assert_eq!(state.task_state_key(&key), None);
    ///
    /// // After recording a task execution
    /// let task_state = TaskAttemptState::new(TaskStatus::Running).with_attempts(1);
    /// state.set_task_state_key(key.clone(), task_state.clone());
    /// assert_eq!(state.task_state_key(&key), Some(task_state));
    /// ```
    pub fn task_state_key(&self, key: &TaskExecutionKey) -> Option<TaskAttemptState> {
        self.task_state.get(key).map(|entry| entry.value().clone())
    }

    // TODO: Remove direct access and use the accessors instead.
    // /// Return the raw task state map.
    // pub fn task_states(&self) -> &DashMap<TaskExecutionKey, TaskAttemptState> {
    //     &self.task_state
    // }
}

/// Represents the operational status of a host within the Genja runtime.
///
/// This enum tracks whether a host is currently available for operations or has
/// been marked as failed. Hosts can transition between these states using the
/// [`State::mark_failed`] and [`State::mark_in_scope`] methods.
///
/// # Variants
///
/// * `InScope` - The host is available and can participate in operations. This is
///   the default state for hosts that have not been explicitly marked as failed.
///
/// * `Failed` - The host has been marked as failed and will be excluded from
///   operations until it is explicitly restored to the `InScope` state.
///
/// # Examples
///
/// ```
/// # use genja_core::state::{State, HostStatus};
/// let state = State::new();
///
/// // Check the status of a host
/// assert_eq!(state.host_status("router1"), None); // Untracked hosts return None
///
/// // Mark a host as failed
/// state.mark_failed("router1");
/// assert_eq!(state.host_status("router1"), Some(HostStatus::Failed));
///
/// // Restore the host to in-scope status
/// state.mark_in_scope("router1");
/// assert_eq!(state.host_status("router1"), Some(HostStatus::InScope));
/// ```
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

/// Tracks the state of connection attempts for a specific host and plugin combination.
///
/// This structure maintains comprehensive information about connection attempts, including
/// the current connection status, the number of attempts that have been made, and any error
/// message from the most recent failed attempt. It is used by the [`State`] structure to
/// track connection history and current state for each host/plugin pair.
///
/// # Fields
///
/// * `status` - The current [`ConnectionStatus`] indicating whether the connection is in
///   progress, successfully established, pending retry, or has failed. This field provides
///   high-level information about the connection state.
///
/// * `attempts` - The total number of connection attempts that have been made for this
///   host/plugin combination. This counter is incremented each time a connection attempt
///   begins and is preserved across status changes, allowing you to track how many attempts
///   were needed before a connection succeeded or ultimately failed.
///
/// * `last_error` - An optional error message from the most recent failed connection attempt.
///   This field is `None` if no error has occurred or if the connection is currently in
///   progress. When present, it contains diagnostic information about why the last connection
///   attempt failed, which can be useful for troubleshooting and logging.
///
/// # Examples
///
/// ```
/// # use genja_core::state::{ConnectionAttemptState, ConnectionStatus, ConnectionFailureKind};
/// // Create a new connection state indicating a successful connection
/// let state = ConnectionAttemptState::new(ConnectionStatus::Connected)
///     .with_attempts(2);
///
/// assert_eq!(state.status, ConnectionStatus::Connected);
/// assert_eq!(state.attempts, 2);
/// assert_eq!(state.last_error, None);
///
/// // Create a connection state with a failure and error message
/// let failed_state = ConnectionAttemptState::new(
///     ConnectionStatus::Failed(ConnectionFailureKind::Timeout)
/// )
/// .with_attempts(3)
/// .with_last_error("connection timed out after 30 seconds");
///
/// assert_eq!(failed_state.attempts, 3);
/// assert_eq!(
///     failed_state.last_error,
///     Some("connection timed out after 30 seconds".to_string())
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionAttemptState {
    pub status: ConnectionStatus,
    pub attempts: usize,
    pub last_error: Option<String>,
}

impl ConnectionAttemptState {
    /// Creates a new `ConnectionAttemptState` with the specified connection status.
    ///
    /// This constructor initializes a connection attempt state with the given status,
    /// setting the attempt counter to 0 and leaving the error field empty. This is
    /// typically used when first recording a connection attempt or when transitioning
    /// to a new connection state.
    ///
    /// # Parameters
    ///
    /// * `status` - The initial [`ConnectionStatus`] for this connection attempt state.
    ///   This indicates whether the connection is in progress, successfully established,
    ///   pending retry, or has failed.
    ///
    /// # Returns
    ///
    /// Returns a new `ConnectionAttemptState` instance with the specified status,
    /// zero attempts, and no error message.
    ///
    /// # Examples
    ///
    /// ```
    /// # use genja_core::state::{ConnectionAttemptState, ConnectionStatus};
    /// // Create a state for a new connection attempt
    /// let state = ConnectionAttemptState::new(ConnectionStatus::Connecting);
    /// assert_eq!(state.status, ConnectionStatus::Connecting);
    /// assert_eq!(state.attempts, 0);
    /// assert_eq!(state.last_error, None);
    /// ```
    pub fn new(status: ConnectionStatus) -> Self {
        Self {
            status,
            attempts: 0,
            last_error: None,
        }
    }

    /// Sets the number of connection attempts for this connection state.
    ///
    /// This builder method allows you to specify how many connection attempts have been
    /// made for a particular host/plugin combination. The attempt count is typically used
    /// to track retry behavior and can be helpful for implementing exponential backoff
    /// strategies or determining when to give up on a connection.
    ///
    /// This method consumes `self` and returns a new instance with the updated attempt
    /// count, following the builder pattern for convenient method chaining.
    ///
    /// # Parameters
    ///
    /// * `attempts` - The number of connection attempts to record. This should represent
    ///   the total number of attempts made, not just the increment. A value of 0 indicates
    ///   no attempts have been made yet, while higher values indicate multiple retry attempts.
    ///
    /// # Returns
    ///
    /// Returns `self` with the `attempts` field updated to the specified value, allowing
    /// for method chaining with other builder methods like [`with_last_error`](Self::with_last_error).
    ///
    /// # Examples
    ///
    /// ```
    /// # use genja_core::state::{ConnectionAttemptState, ConnectionStatus};
    /// // Create a connection state with 3 attempts
    /// let state = ConnectionAttemptState::new(ConnectionStatus::Connecting)
    ///     .with_attempts(3);
    ///
    /// assert_eq!(state.attempts, 3);
    /// ```
    ///
    /// ```
    /// # use genja_core::state::{ConnectionAttemptState, ConnectionStatus, ConnectionFailureKind};
    /// // Chain multiple builder methods together
    /// let state = ConnectionAttemptState::new(
    ///     ConnectionStatus::Failed(ConnectionFailureKind::Timeout)
    /// )
    /// .with_attempts(5)
    /// .with_last_error("connection timed out after 30 seconds");
    ///
    /// assert_eq!(state.attempts, 5);
    /// assert_eq!(state.last_error, Some("connection timed out after 30 seconds".to_string()));
    /// ```
    pub fn with_attempts(mut self, attempts: usize) -> Self {
        self.attempts = attempts;
        self
    }

    /// Sets the error message from the most recent failed connection attempt.
    ///
    /// This builder method allows you to record diagnostic information about why a
    /// connection attempt failed. The error message is typically set when marking a
    /// connection as retry pending or failed, and can be used for logging, debugging,
    /// or displaying error information to users.
    ///
    /// This method consumes `self` and returns a new instance with the error message
    /// set, following the builder pattern for convenient method chaining.
    ///
    /// # Parameters
    ///
    /// * `last_error` - The error message to record. Can be any type that converts into
    ///   a `String`, such as `&str`, `String`, error types that implement `Display`, or
    ///   other string-like types. The error message should provide meaningful diagnostic
    ///   information about what went wrong during the connection attempt.
    ///
    /// # Returns
    ///
    /// Returns `self` with the `last_error` field set to `Some(error_message)`, allowing
    /// for method chaining with other builder methods like [`with_attempts`](Self::with_attempts).
    ///
    /// # Examples
    ///
    /// ```
    /// # use genja_core::state::{ConnectionAttemptState, ConnectionStatus, ConnectionFailureKind};
    /// // Create a connection state with an error message
    /// let state = ConnectionAttemptState::new(
    ///     ConnectionStatus::Failed(ConnectionFailureKind::Timeout)
    /// )
    /// .with_last_error("connection timed out after 30 seconds");
    ///
    /// assert_eq!(
    ///     state.last_error,
    ///     Some("connection timed out after 30 seconds".to_string())
    /// );
    /// ```
    ///
    /// ```
    /// # use genja_core::state::{ConnectionAttemptState, ConnectionStatus};
    /// // Chain multiple builder methods together
    /// let state = ConnectionAttemptState::new(ConnectionStatus::RetryPending)
    ///     .with_attempts(2)
    ///     .with_last_error("authentication failed: invalid credentials");
    ///
    /// assert_eq!(state.attempts, 2);
    /// assert_eq!(
    ///     state.last_error,
    ///     Some("authentication failed: invalid credentials".to_string())
    /// );
    /// ```
    ///
    /// ```
    /// # use genja_core::state::{ConnectionAttemptState, ConnectionStatus, ConnectionFailureKind};
    /// // Use with String type
    /// let error_msg = String::from("network unreachable");
    /// let state = ConnectionAttemptState::new(
    ///     ConnectionStatus::Failed(ConnectionFailureKind::Transport)
    /// )
    /// .with_last_error(error_msg);
    ///
    /// assert_eq!(state.last_error, Some("network unreachable".to_string()));
    /// ```
    pub fn with_last_error(mut self, last_error: impl Into<String>) -> Self {
        self.last_error = Some(last_error.into());
        self
    }
}

/// Represents the current status of a connection attempt for a host/plugin pair.
///
/// This enum tracks the lifecycle of a connection attempt, from the initial state
/// through various stages of connection establishment, including success, retry,
/// and failure states. It is used within [`ConnectionAttemptState`] to provide
/// high-level information about the current state of a connection.
///
/// # Variants
///
/// * `NeverTried` - No connection attempt has been made yet for this host/plugin
///   combination. This is the initial state before any connection activity.
///
/// * `Connecting` - A connection attempt is currently in progress. This state is
///   set when [`State::begin_connection_attempt`] is called and indicates that
///   the connection plugin is actively attempting to establish a connection.
///
/// * `Connected` - The connection has been successfully established. This state
///   is set when [`State::mark_connection_connected`] is called and indicates
///   that the connection is ready for use.
///
/// * `RetryPending` - A connection attempt has failed but will be retried. This
///   state is set when [`State::mark_connection_retry_pending`] is called and
///   indicates that the connection will be attempted again after a delay or
///   under different conditions.
///
/// * `Failed(ConnectionFailureKind)` - The connection attempt has failed and will
///   not be retried. This state is set when [`State::mark_connection_failed`] is
///   called and includes a [`ConnectionFailureKind`] that classifies the type of
///   failure that occurred (e.g., timeout, authentication failure, DNS error).
///
/// # Examples
///
/// ```
/// # use genja_core::state::{ConnectionStatus, ConnectionFailureKind};
/// // Check different connection states
/// let connecting = ConnectionStatus::Connecting;
/// let connected = ConnectionStatus::Connected;
/// let failed = ConnectionStatus::Failed(ConnectionFailureKind::Timeout);
///
/// assert_eq!(connecting, ConnectionStatus::Connecting);
/// assert_eq!(connected, ConnectionStatus::Connected);
/// assert!(matches!(failed, ConnectionStatus::Failed(_)));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionStatus {
    NeverTried,
    Connecting,
    Connected,
    RetryPending,
    Failed(ConnectionFailureKind),
}

/// Classifies the type of failure that occurred during a connection attempt.
///
/// This enum categorizes connection failures into distinct types, allowing for
/// more targeted error handling, retry logic, and diagnostic reporting. Each
/// variant represents a different category of connection failure that may require
/// different handling strategies.
///
/// # Variants
///
/// * `Timeout` - The connection attempt exceeded the configured timeout period.
///   This typically indicates network latency issues, an unresponsive host, or
///   firewall rules blocking the connection.
///
/// * `Refused` - The connection was actively refused by the remote host. This
///   usually means the host is reachable but the service is not running or is
///   not accepting connections on the specified port.
///
/// * `Auth` - Authentication failed during the connection attempt. This indicates
///   that the credentials provided were invalid, expired, or insufficient for
///   establishing the connection.
///
/// * `Dns` - DNS resolution failed for the hostname. This means the hostname
///   could not be resolved to an IP address, possibly due to DNS server issues
///   or an invalid hostname.
///
/// * `Transport` - A transport-layer error occurred during the connection attempt.
///   This includes network unreachable errors, connection reset by peer, and
///   other low-level network failures.
///
/// * `Unknown` - The failure type could not be determined or does not fit into
///   any of the other categories. This is used as a fallback for unexpected or
///   unclassified errors.
///
/// # Examples
///
/// ```
/// # use genja_core::state::{ConnectionFailureKind, ConnectionStatus};
/// // Create different failure kinds
/// let timeout = ConnectionFailureKind::Timeout;
/// let auth = ConnectionFailureKind::Auth;
/// let dns = ConnectionFailureKind::Dns;
///
/// // Use in connection status
/// let failed_status = ConnectionStatus::Failed(ConnectionFailureKind::Timeout);
/// assert!(matches!(failed_status, ConnectionStatus::Failed(_)));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionFailureKind {
    Timeout,
    Refused,
    Auth,
    Dns,
    Transport,
    Unknown,
}

/// A unique identifier for tracking task execution state for a specific host and task combination.
///
/// This structure serves as a composite key used by the [`State`] structure to track the
/// execution state of tasks on individual hosts. Each `TaskExecutionKey` uniquely identifies
/// a task execution by combining the hostname with the task name, allowing the state tracking
/// system to maintain separate execution histories for different task/host pairs.
///
/// The key is used internally by methods like [`State::set_task_state`], [`State::task_state`],
/// and their `_key` variants to store and retrieve [`TaskAttemptState`] information.
///
/// # Fields
///
/// * `host` - A [`NatString`] representing the hostname on which the task is being executed.
///   Using `NatString` provides efficient string handling with natural sorting capabilities,
///   which can be useful when displaying or organizing task execution results by hostname.
///
/// * `task_name` - The name of the task being executed. This is a standard `String` that
///   identifies the specific task or operation being performed on the host. Task names should
///   be unique within a playbook or execution context to ensure proper state tracking.
///
/// # Examples
///
/// ```
/// # use genja_core::state::TaskExecutionKey;
/// # use genja_core::types::NatString;
/// // Create a task execution key for a specific host and task
/// let key = TaskExecutionKey::new("router1", "show_version");
/// assert_eq!(key.host, NatString::from("router1"));
/// assert_eq!(key.task_name, "show_version");
///
/// // Keys can be used to track task state
/// # use genja_core::state::{State, TaskAttemptState, TaskStatus};
/// let state = State::new();
/// let task_state = TaskAttemptState::new(TaskStatus::Running).with_attempts(1);
/// state.set_task_state_key(key.clone(), task_state.clone());
/// assert_eq!(state.task_state_key(&key), Some(task_state));
/// ```
///
/// ```
/// # use genja_core::state::TaskExecutionKey;
/// // Keys with the same host and task name are equal
/// let key1 = TaskExecutionKey::new("router1", "show_version");
/// let key2 = TaskExecutionKey::new("router1", "show_version");
/// assert_eq!(key1, key2);
///
/// // Keys with different hosts or task names are not equal
/// let key3 = TaskExecutionKey::new("router2", "show_version");
/// assert_ne!(key1, key3);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TaskExecutionKey {
    pub host: NatString,
    pub task_name: String,
}

impl TaskExecutionKey {
    /// Creates a new `TaskExecutionKey` from a host and task name.
    ///
    /// This constructor method creates a unique identifier for tracking task execution state
    /// by combining a hostname with a task name. The resulting key can be used with methods
    /// like [`State::set_task_state_key`] and [`State::task_state_key`] to store and retrieve
    /// task execution information.
    ///
    /// The host parameter is converted into a [`NatString`] for efficient string handling with
    /// natural sorting capabilities, while the task name is stored as a standard `String`.
    ///
    /// # Parameters
    ///
    /// * `host` - The hostname for which the task is being executed. Can be any type that
    ///   converts into a `String`, such as `&str`, `String`, or other string-like types.
    ///   This will be stored as a `NatString` in the resulting key.
    ///
    /// * `task_name` - The name of the task being executed. Can be any type that converts
    ///   into a `String`, such as `&str`, `String`, or other string-like types. Task names
    ///   should be unique within a playbook or execution context to ensure proper state tracking.
    ///
    /// # Returns
    ///
    /// Returns a new `TaskExecutionKey` instance with the specified host and task name,
    /// ready to be used for tracking task execution state.
    ///
    /// # Examples
    ///
    /// ```
    /// # use genja_core::state::TaskExecutionKey;
    /// # use genja_core::types::NatString;
    /// // Create a key using string slices
    /// let key = TaskExecutionKey::new("router1", "show_version");
    /// assert_eq!(key.host, NatString::from("router1"));
    /// assert_eq!(key.task_name, "show_version");
    /// ```
    ///
    /// ```
    /// # use genja_core::state::TaskExecutionKey;
    /// // Create a key using owned Strings
    /// let host = String::from("router2");
    /// let task = String::from("configure_interface");
    /// let key = TaskExecutionKey::new(host, task);
    /// assert_eq!(key.task_name, "configure_interface");
    /// ```
    pub fn new(host: impl Into<String>, task_name: impl Into<String>) -> Self {
        Self {
            host: NatString::new(host.into()),
            task_name: task_name.into(),
        }
    }
}


/// Tracks the state of task execution attempts for a specific host and task combination.
///
/// This structure maintains comprehensive information about task execution attempts, including
/// the current task status, the number of attempts that have been made, and any error message
/// from the most recent failed attempt. It is used by the [`State`] structure to track task
/// execution history and current state for each host/task pair.
///
/// # Fields
///
/// * `status` - The current [`TaskStatus`] indicating whether the task is pending, running,
///   succeeded, pending retry, failed, or skipped. This field provides high-level information
///   about the task execution state.
///
/// * `attempts` - The total number of task execution attempts that have been made for this
///   host/task combination. This counter tracks how many times the task has been attempted
///   and is preserved across status changes, allowing you to track how many attempts were
///   needed before the task succeeded or ultimately failed.
///
/// * `last_error` - An optional error message from the most recent failed task execution attempt.
///   This field is `None` if no error has occurred or if the task is currently running or has
///   succeeded. When present, it contains diagnostic information about why the last task
///   execution attempt failed, which can be useful for troubleshooting and logging.
///
/// # Examples
///
/// ```
/// # use genja_core::state::{TaskAttemptState, TaskStatus, TaskFailureKind};
/// // Create a new task state indicating a successful execution
/// let state = TaskAttemptState::new(TaskStatus::Succeeded)
///     .with_attempts(1);
///
/// assert_eq!(state.status, TaskStatus::Succeeded);
/// assert_eq!(state.attempts, 1);
/// assert_eq!(state.last_error, None);
///
/// // Create a task state with a failure and error message
/// let failed_state = TaskAttemptState::new(
///     TaskStatus::Failed(TaskFailureKind::ParseFailed)
/// )
/// .with_attempts(2)
/// .with_last_error("failed to parse command output");
///
/// assert_eq!(failed_state.attempts, 2);
/// assert_eq!(
///     failed_state.last_error,
///     Some("failed to parse command output".to_string())
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskAttemptState {
    pub status: TaskStatus,
    pub attempts: usize,
    pub last_error: Option<String>,
}

impl TaskAttemptState {
    /// Creates a new `TaskAttemptState` with the specified task status.
    ///
    /// This constructor initializes a task attempt state with the given status,
    /// setting the attempt counter to 0 and leaving the error field empty. This is
    /// typically used when first recording a task execution or when transitioning
    /// to a new task state.
    ///
    /// # Parameters
    ///
    /// * `status` - The initial [`TaskStatus`] for this task attempt state.
    ///   This indicates whether the task is pending, running, succeeded, pending retry,
    ///   failed, or skipped.
    ///
    /// # Returns
    ///
    /// Returns a new `TaskAttemptState` instance with the specified status,
    /// zero attempts, and no error message.
    ///
    /// # Examples
    ///
    /// ```
    /// # use genja_core::state::{TaskAttemptState, TaskStatus};
    /// // Create a state for a new task execution
    /// let state = TaskAttemptState::new(TaskStatus::Running);
    /// assert_eq!(state.status, TaskStatus::Running);
    /// assert_eq!(state.attempts, 0);
    /// assert_eq!(state.last_error, None);
    /// ```
    pub fn new(status: TaskStatus) -> Self {
        Self {
            status,
            attempts: 0,
            last_error: None,
        }
    }

    /// Sets the number of task execution attempts for this task state.
    ///
    /// This builder method allows you to specify how many task execution attempts have been
    /// made for a particular host/task combination. The attempt count is typically used
    /// to track retry behavior and can be helpful for implementing retry strategies or
    /// determining when to give up on a task execution.
    ///
    /// This method consumes `self` and returns a new instance with the updated attempt
    /// count, following the builder pattern for convenient method chaining.
    ///
    /// # Parameters
    ///
    /// * `attempts` - The number of task execution attempts to record. This should represent
    ///   the total number of attempts made, not just the increment. A value of 0 indicates
    ///   no attempts have been made yet, while higher values indicate multiple retry attempts.
    ///
    /// # Returns
    ///
    /// Returns `self` with the `attempts` field updated to the specified value, allowing
    /// for method chaining with other builder methods like [`with_last_error`](Self::with_last_error).
    ///
    /// # Examples
    ///
    /// ```
    /// # use genja_core::state::{TaskAttemptState, TaskStatus};
    /// // Create a task state with 2 attempts
    /// let state = TaskAttemptState::new(TaskStatus::Running)
    ///     .with_attempts(2);
    ///
    /// assert_eq!(state.attempts, 2);
    /// ```
    ///
    /// ```
    /// # use genja_core::state::{TaskAttemptState, TaskStatus, TaskFailureKind};
    /// // Chain multiple builder methods together
    /// let state = TaskAttemptState::new(
    ///     TaskStatus::Failed(TaskFailureKind::ParseFailed)
    /// )
    /// .with_attempts(3)
    /// .with_last_error("failed to parse command output");
    ///
    /// assert_eq!(state.attempts, 3);
    /// assert_eq!(state.last_error, Some("failed to parse command output".to_string()));
    /// ```
    pub fn with_attempts(mut self, attempts: usize) -> Self {
        self.attempts = attempts;
        self
    }

    /// Sets the error message from the most recent failed task execution attempt.
    ///
    /// This builder method allows you to record diagnostic information about why a
    /// task execution attempt failed. The error message is typically set when marking a
    /// task as retry pending or failed, and can be used for logging, debugging,
    /// or displaying error information to users.
    ///
    /// This method consumes `self` and returns a new instance with the error message
    /// set, following the builder pattern for convenient method chaining.
    ///
    /// # Parameters
    ///
    /// * `last_error` - The error message to record. Can be any type that converts into
    ///   a `String`, such as `&str`, `String`, error types that implement `Display`, or
    ///   other string-like types. The error message should provide meaningful diagnostic
    ///   information about what went wrong during the task execution attempt.
    ///
    /// # Returns
    ///
    /// Returns `self` with the `last_error` field set to `Some(error_message)`, allowing
    /// for method chaining with other builder methods like [`with_attempts`](Self::with_attempts).
    ///
    /// # Examples
    ///
    /// ```
    /// # use genja_core::state::{TaskAttemptState, TaskStatus, TaskFailureKind};
    /// // Create a task state with an error message
    /// let state = TaskAttemptState::new(
    ///     TaskStatus::Failed(TaskFailureKind::ParseFailed)
    /// )
    /// .with_last_error("failed to parse show version output");
    ///
    /// assert_eq!(
    ///     state.last_error,
    ///     Some("failed to parse show version output".to_string())
    /// );
    /// ```
    ///
    /// ```
    /// # use genja_core::state::{TaskAttemptState, TaskStatus};
    /// // Chain multiple builder methods together
    /// let state = TaskAttemptState::new(TaskStatus::RetryPending)
    ///     .with_attempts(1)
    ///     .with_last_error("command execution timed out");
    ///
    /// assert_eq!(state.attempts, 1);
    /// assert_eq!(
    ///     state.last_error,
    ///     Some("command execution timed out".to_string())
    /// );
    /// ```
    ///
    /// ```
    /// # use genja_core::state::{TaskAttemptState, TaskStatus, TaskFailureKind};
    /// // Use with String type
    /// let error_msg = String::from("validation failed: invalid interface name");
    /// let state = TaskAttemptState::new(
    ///     TaskStatus::Failed(TaskFailureKind::ValidationFailed)
    /// )
    /// .with_last_error(error_msg);
    ///
    /// assert_eq!(state.last_error, Some("validation failed: invalid interface name".to_string()));
    /// ```
    pub fn with_last_error(mut self, last_error: impl Into<String>) -> Self {
        self.last_error = Some(last_error.into());
        self
    }
}

/// Represents the current status of a task execution attempt for a host/task pair.
///
/// This enum tracks the lifecycle of a task execution, from the initial pending state
/// through various stages of execution, including success, retry, and failure states.
/// It is used within [`TaskAttemptState`] to provide high-level information about the
/// current state of a task execution.
///
/// # Variants
///
/// * `Pending` - The task has been scheduled but has not yet started execution. This is
///   the initial state before any task execution activity begins.
///
/// * `Running` - The task is currently being executed. This state indicates that the task
///   execution is actively in progress.
///
/// * `Succeeded` - The task has completed successfully. This state indicates that the task
///   execution finished without errors and achieved its intended outcome.
///
/// * `RetryPending` - A task execution attempt has failed but will be retried. This state
///   indicates that the task will be attempted again after a delay or under different
///   conditions.
///
/// * `Failed(TaskFailureKind)` - The task execution has failed and will not be retried.
///   This state includes a [`TaskFailureKind`] that classifies the type of failure that
///   occurred (e.g., command failed, parse failed, validation failed, timeout).
///
/// * `Skipped` - The task was skipped and not executed. This typically occurs when task
///   conditions are not met or when the task is explicitly configured to be skipped.
///
/// # Examples
///
/// ```
/// # use genja_core::state::{TaskStatus, TaskFailureKind};
/// // Check different task states
/// let pending = TaskStatus::Pending;
/// let running = TaskStatus::Running;
/// let succeeded = TaskStatus::Succeeded;
/// let failed = TaskStatus::Failed(TaskFailureKind::ParseFailed);
///
/// assert_eq!(pending, TaskStatus::Pending);
/// assert_eq!(running, TaskStatus::Running);
/// assert_eq!(succeeded, TaskStatus::Succeeded);
/// assert!(matches!(failed, TaskStatus::Failed(_)));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskStatus {
    Pending,
    Running,
    Succeeded,
    RetryPending,
    Failed(TaskFailureKind),
    Skipped,
}

/// Classifies the type of failure that occurred during a task execution attempt.
///
/// This enum categorizes task execution failures into distinct types, allowing for
/// more targeted error handling, retry logic, and diagnostic reporting. Each variant
/// represents a different category of task failure that may require different handling
/// strategies.
///
/// # Variants
///
/// * `CommandFailed` - The command or operation executed by the task failed. This
///   typically indicates that the command returned a non-zero exit code or encountered
///   an error during execution.
///
/// * `ParseFailed` - Parsing of the task output or result failed. This indicates that
///   the task executed successfully but the output could not be parsed into the expected
///   format or structure.
///
/// * `ValidationFailed` - Validation of the task result or output failed. This indicates
///   that the task executed and was parsed successfully, but the result did not meet
///   the expected validation criteria.
///
/// * `Timeout` - The task execution exceeded the configured timeout period. This
///   typically indicates that the task took too long to complete or became unresponsive.
///
/// * `DependencyFailed` - A dependency required by the task failed or was not available.
///   This indicates that the task could not execute because a prerequisite task or
///   resource was not in the expected state.
///
/// * `Unknown` - The failure type could not be determined or does not fit into any of
///   the other categories. This is used as a fallback for unexpected or unclassified
///   errors.
///
/// # Examples
///
/// ```
/// # use genja_core::state::{TaskFailureKind, TaskStatus};
/// // Create different failure kinds
/// let command_failed = TaskFailureKind::CommandFailed;
/// let parse_failed = TaskFailureKind::ParseFailed;
/// let timeout = TaskFailureKind::Timeout;
///
/// // Use in task status
/// let failed_status = TaskStatus::Failed(TaskFailureKind::ParseFailed);
/// assert!(matches!(failed_status, TaskStatus::Failed(_)));
/// ```
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
