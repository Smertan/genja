use crate::{inventory::ConnectionKey, types::NatString};
use dashmap::DashMap;
use log::warn;

/// Per-host execution state for the current Genja instance.
///
/// This state is internal to the runtime and is used to exclude failed hosts
/// from host views until they are explicitly put back in scope.
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

    /// Mark a host as failed (out of scope).
    pub fn mark_failed<K>(&self, name: K)
    where
        K: Into<NatString>,
    {
        let name = name.into();
        warn!("host '{}' marked as failed", name);
        self.host_status.insert(name, HostStatus::Failed);
    }

    /// Mark a host as back in scope.
    pub fn mark_in_scope<K>(&self, name: K)
    where
        K: Into<NatString>,
    {
        self.host_status.insert(name.into(), HostStatus::InScope);
    }

    /// Mark a host as back in scope using a key.
    pub fn mark_in_scope_key(&self, key: &NatString) {
        self.host_status.insert(key.clone(), HostStatus::InScope);
    }

    /// Returns `true` if the host is currently in scope.
    pub fn is_in_scope<K>(&self, name: K) -> bool
    where
        K: Into<NatString>,
    {
        let key = name.into();
        self.is_in_scope_key(&key)
    }

    /// Return the tracked host scope status for a host, if present.
    pub fn host_status<K>(&self, name: K) -> Option<HostStatus>
    where
        K: Into<NatString>,
    {
        let key = name.into();
        self.host_status_key(&key)
    }

    /// Return the tracked host scope status for an existing key, if present.
    pub fn host_status_key(&self, key: &NatString) -> Option<HostStatus> {
        self.host_status.get(key).map(|entry| *entry.value())
    }

    /// Returns `true` if the host is currently in scope.
    pub fn is_in_scope_key(&self, key: &NatString) -> bool {
        match self.host_status.get(key) {
            Some(status) => *status.value() == HostStatus::InScope,
            None => true,
        }
    }

    /// Return the raw host scope state map.
    pub fn host_statuses(&self) -> &DashMap<NatString, HostStatus> {
        &self.host_status
    }

    /// Set the current connection attempt state for a host and plugin.
    pub fn set_connection_state(
        &self,
        host: impl Into<String>,
        plugin_name: impl Into<String>,
        state: ConnectionAttemptState,
    ) {
        self.connection_state
            .insert(ConnectionKey::new(host, plugin_name), state);
    }

    /// Set the current connection attempt state using an existing key.
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

    /// Return the current connection attempt state for a host and plugin.
    pub fn connection_state(
        &self,
        host: &str,
        plugin_name: &str,
    ) -> Option<ConnectionAttemptState> {
        let key = ConnectionKey::new(host, plugin_name);
        self.connection_state.get(&key).map(|entry| entry.value().clone())
    }

    /// Return the current connection attempt state for an existing key.
    pub fn connection_state_key(&self, key: &ConnectionKey) -> Option<ConnectionAttemptState> {
        self.connection_state.get(key).map(|entry| entry.value().clone())
    }

    /// Return the raw connection state map.
    pub fn connection_states(&self) -> &DashMap<ConnectionKey, ConnectionAttemptState> {
        &self.connection_state
    }

    /// Set the current task attempt state for a host and task.
    pub fn set_task_state(
        &self,
        host: impl Into<String>,
        task_name: impl Into<String>,
        state: TaskAttemptState,
    ) {
        self.task_state
            .insert(TaskExecutionKey::new(host, task_name), state);
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
}
