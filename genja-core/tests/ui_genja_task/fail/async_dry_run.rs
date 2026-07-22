#![allow(unused_imports)]

use genja_core::genja_task;
use genja_core::inventory::Host;
use genja_core::task::{BlockingTaskRuntimeContext, HostTaskResult, TaskSuccess};

struct BadTask;

#[genja_task(name = "bad")]
impl BadTask {
    fn start(
        &self,
        _host: &Host,
        _context: &BlockingTaskRuntimeContext,
    ) -> Result<HostTaskResult, genja_core::task::TaskError> {
        Ok(HostTaskResult::passed(TaskSuccess::new()))
    }

    async fn dry_run(
        &self,
        _host: &Host,
        _context: &BlockingTaskRuntimeContext,
    ) -> Result<HostTaskResult, genja_core::task::TaskError> {
        Ok(HostTaskResult::passed(TaskSuccess::new()))
    }
}

fn main() {}
