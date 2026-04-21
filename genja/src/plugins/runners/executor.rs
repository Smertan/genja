use genja_core::inventory::{Host, Hosts};
use genja_core::task::{TaskDefinition, TaskInfo, TaskResults};
use genja_core::{GenjaError, NatString};
use std::time::SystemTime;

/// Shared execution helper for built-in runner plugins.
#[derive(Debug)]
pub(crate) struct TaskExecutor<'a> {
    hosts: &'a Hosts,
    max_depth: usize,
}

impl<'a> TaskExecutor<'a> {
    pub(crate) fn new(hosts: &'a Hosts, max_depth: usize) -> Self {
        Self { hosts, max_depth }
    }

    pub(crate) fn run_definition(
        &self,
        task_definition: &TaskDefinition,
    ) -> Result<TaskResults, GenjaError> {
        let started_at = SystemTime::now();
        let mut results = TaskResults::new(task_definition.name()).with_started_at(started_at);

        for (host_id, host) in self.hosts.iter() {
            self.run_host(task_definition, host_id, host, &mut results)?;
        }

        let finished_at = SystemTime::now();
        let duration_ns = finished_at
            .duration_since(started_at)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);

        Ok(results
            .with_finished_at(finished_at)
            .with_duration_ns(duration_ns))
    }

    fn run_host(
        &self,
        task_definition: &TaskDefinition,
        host_id: &NatString,
        host: &Host,
        results: &mut TaskResults,
    ) -> Result<(), GenjaError> {
        task_definition.start(host_id.as_str(), host, results, self.max_depth)
    }
}
