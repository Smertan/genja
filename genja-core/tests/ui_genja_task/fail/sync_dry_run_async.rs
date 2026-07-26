#![allow(unused_imports)]

use genja_core::genja_task;
use genja_core::inventory::Host;
use genja_core::task::{HostTaskResult, TaskRuntimeContext, TaskSuccess};

struct BadTask;

#[genja_task(name = "bad")]
impl BadTask {
    async fn start_async(
        &self,
        _host: &Host,
        _context: &TaskRuntimeContext,
    ) -> Result<HostTaskResult, genja_core::task::TaskError> {
        Ok(HostTaskResult::passed(TaskSuccess::new()))
    }

    fn dry_run_async(
        &self,
        _host: &Host,
        _context: &TaskRuntimeContext,
    ) -> Result<HostTaskResult, genja_core::task::TaskError> {
        Ok(HostTaskResult::passed(TaskSuccess::new()))
    }
}

fn main() {}
