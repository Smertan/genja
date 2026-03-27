use crate::types::NatString;
use dashmap::DashMap;

/// Per-host execution state for the current Genja instance.
///
/// This state is internal to the runtime and is used to exclude failed hosts
/// from host views until they are explicitly put back in scope.
#[derive(Debug, Default)]
pub struct State {
    host_status: DashMap<NatString, HostStatus>,
}

impl State {
    /// Create an empty state store.
    pub fn new() -> Self {
        Self {
            host_status: DashMap::new(),
        }
    }

    /// Mark a host as failed (out of scope).
    pub fn mark_failed(&self, name: impl Into<String>) {
        self.host_status
            .insert(NatString::new(name.into()), HostStatus::Failed);
    }

    /// Mark a host as back in scope.
    pub fn mark_in_scope(&self, name: impl Into<String>) {
        self.host_status
            .insert(NatString::new(name.into()), HostStatus::InScope);
    }

    /// Mark a host as back in scope using a key.
    pub fn mark_in_scope_key(&self, key: &NatString) {
        self.host_status.insert(key.clone(), HostStatus::InScope);
    }

    /// Returns `true` if the host is currently in scope.
    pub fn is_in_scope(&self, name: &str) -> bool {
        let key = NatString::new(name.to_string());
        self.is_in_scope_key(&key)
    }

    /// Returns `true` if the host is currently in scope.
    pub fn is_in_scope_key(&self, key: &NatString) -> bool {
        match self.host_status.get(key) {
            Some(status) => *status.value() == HostStatus::InScope,
            None => true,
        }
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
