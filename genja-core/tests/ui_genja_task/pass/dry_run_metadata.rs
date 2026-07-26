use genja_core::genja_task;
use genja_core::inventory::Host;
use genja_core::task::{
    BlockingTaskRuntimeContext, HostTaskResult, TaskRuntimeContext, TaskSuccess,
};

struct AsyncDryRunTask;

#[genja_task(name = "async_dry_run", supports_dry_run = true)]
impl AsyncDryRunTask {
    async fn start_async(
        &self,
        _host: &Host,
        _context: &TaskRuntimeContext,
    ) -> Result<HostTaskResult, genja_core::task::TaskError> {
        Ok(HostTaskResult::passed(TaskSuccess::new()))
    }

    async fn dry_run_async(
        &self,
        _host: &Host,
        _context: &TaskRuntimeContext,
    ) -> Result<HostTaskResult, genja_core::task::TaskError> {
        Ok(HostTaskResult::passed(TaskSuccess::new()))
    }
}

struct BlockingDryRunTask;

#[genja_task(name = "blocking_dry_run", supports_dry_run = true)]
impl BlockingDryRunTask {
    fn start(
        &self,
        _host: &Host,
        _context: &BlockingTaskRuntimeContext,
    ) -> Result<HostTaskResult, genja_core::task::TaskError> {
        Ok(HostTaskResult::passed(TaskSuccess::new()))
    }

    fn dry_run(
        &self,
        _host: &Host,
        _context: &BlockingTaskRuntimeContext,
    ) -> Result<HostTaskResult, genja_core::task::TaskError> {
        Ok(HostTaskResult::passed(TaskSuccess::new()))
    }
}

fn main() {}
