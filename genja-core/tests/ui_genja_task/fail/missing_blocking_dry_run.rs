#![allow(unused_imports)]

use genja_core::genja_task;
use genja_core::inventory::Host;
use genja_core::task::{BlockingTaskRuntimeContext, HostTaskResult, TaskSuccess};

struct BadTask;

#[genja_task(name = "bad", supports_dry_run = true)]
impl BadTask {
    fn start(
        &self,
        _host: &Host,
        _context: &BlockingTaskRuntimeContext,
    ) -> Result<HostTaskResult, genja_core::task::TaskError> {
        Ok(HostTaskResult::passed(TaskSuccess::new()))
    }
}

fn main() {}
