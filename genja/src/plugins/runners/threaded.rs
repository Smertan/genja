use super::executor::TaskExecutor;
use genja_core::GenjaError;
use genja_core::NatString;
use genja_core::inventory::{Host, Hosts};
use genja_core::task::{TaskDefinition, TaskInfo, TaskResults, Tasks};
use genja_plugin_manager::plugin_types::{Plugin, PluginRunner};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::SystemTime;

/// Built-in threaded runner placeholder.
///
/// This currently reuses the shared executor path until a concurrent executor
/// is implemented, but it preserves the configured default runner name.
pub struct ThreadedRunnerPlugin;

impl Plugin for ThreadedRunnerPlugin {
    fn name(&self) -> String {
        "threaded".to_string()
    }
}

impl PluginRunner for ThreadedRunnerPlugin {
    fn run(
        &self,
        task: &TaskDefinition,
        hosts: &Hosts,
        max_depth: usize,
    ) -> Result<TaskResults, GenjaError> {
        if hosts.is_empty() {
            let started_at = SystemTime::now();
            return Ok(TaskResults::new(task.name())
                .with_started_at(started_at)
                .with_finished_at(started_at)
                .with_duration_ns(0));
        }

        let started_at = SystemTime::now();
        let worker_count = worker_count_for(hosts.len());
        let jobs = Arc::new(Mutex::new(collect_jobs(hosts)));
        let (tx, rx) = mpsc::channel();
        let mut handles = Vec::with_capacity(worker_count);

        for _ in 0..worker_count {
            let jobs = Arc::clone(&jobs);
            let tx = tx.clone();
            let task = task.clone();

            handles.push(thread::spawn(move || -> Result<(), GenjaError> {
                loop {
                    let next_job = {
                        let mut guard = jobs.lock().map_err(|_| {
                            GenjaError::Message("threaded runner queue lock poisoned".to_string())
                        })?;
                        guard.pop_front()
                    };

                    let Some((host_id, host)) = next_job else {
                        break;
                    };

                    let host_results = TaskExecutor::run_host(&task, &host_id, &host, max_depth)?;

                    tx.send(host_results).map_err(|err| {
                        GenjaError::Message(format!(
                            "threaded runner failed to send host result: {}",
                            err
                        ))
                    })?;
                }

                Ok(())
            }));
        }

        drop(tx);

        let mut results = TaskResults::new(task.name()).with_started_at(started_at);
        for host_results in rx {
            results.merge(host_results);
        }

        for handle in handles {
            match handle.join() {
                Ok(Ok(())) => {}
                Ok(Err(err)) => return Err(err),
                Err(_) => {
                    return Err(GenjaError::Message(
                        "threaded runner worker panicked".to_string(),
                    ));
                }
            }
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

    fn run_tasks(
        &self,
        tasks: &Tasks,
        hosts: &Hosts,
        max_depth: usize,
    ) -> Result<Vec<TaskResults>, GenjaError> {
        tasks
            .iter()
            .map(|task| self.run(task, hosts, max_depth))
            .collect()
    }
}

fn worker_count_for(host_count: usize) -> usize {
    let available = thread::available_parallelism()
        .map(|parallelism| parallelism.get())
        .unwrap_or(1);
    available.max(1).min(host_count.max(1))
}

fn collect_jobs(hosts: &Hosts) -> VecDeque<(NatString, Host)> {
    hosts
        .iter()
        .map(|(host_id, host)| (host_id.clone(), host.clone()))
        .collect()
}
