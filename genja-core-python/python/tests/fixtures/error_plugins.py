def raise_type_error():
    raise TypeError("bad hook shape")


async def _yield_once():
    return None


class FailingInventoryPlugin:
    name = "failing_inventory"

    group = "InventoryPlugin"

    def load(self, settings, plugins):
        raise RuntimeError("inventory load exploded")


class FailingRunnerPlugin:
    name = "failing_runner"

    group = "RunnerPlugin"

    def run_task(self, task, hosts, connection_resolver, runner_config, run_options):
        raise TypeError("runner run_task exploded")


class AsyncFailingRunnerPlugin:
    name = "async_failing_runner"

    group = "RunnerPlugin"

    async def run_task(
        self, task, hosts, connection_resolver, runner_config, run_options
    ):
        await _yield_once()
        raise RuntimeError("async runner run_task exploded")


class AsyncFailingBatchRunnerPlugin:
    name = "async_failing_batch_runner"

    group = "RunnerPlugin"

    async def run_task(
        self, task, hosts, connection_resolver, runner_config, run_options
    ):
        return task.run_on_hosts(
            hosts,
            connection_resolver=connection_resolver,
            run_options=run_options,
        )

    async def run_tasks(
        self, tasks, hosts, connection_resolver, runner_config, run_options
    ):
        await _yield_once()
        raise RuntimeError("async runner run_tasks exploded")


class InvalidResultRunnerPlugin:
    name = "invalid_result_runner"

    group = "RunnerPlugin"

    def run_task(self, task, hosts, connection_resolver, runner_config, run_options):
        return "not task results"


class FailingCommandConnection:
    def __init__(self, key):
        self.key = key
        self.alive = False

    def open(self, params):
        self.alive = True

    def execute_command(self, command):
        raise RuntimeError("connection command exploded")

    def close(self):
        self.alive = False
        return self.key

    def is_alive(self):
        return self.alive


class FailingCommandConnectionPlugin:
    name = "failing_command"

    group = "ConnectionPlugin"

    def create(self, key):
        return FailingCommandConnection(key)


class FailingProcessorPlugin:
    name = "failing_processor"

    group = "ProcessorPlugin"

    def on_task_start(self, context, results):
        raise RuntimeError("processor on_task_start exploded")
