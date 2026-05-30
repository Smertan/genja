class FirstHostOnlyRunnerPlugin:
    name = "python_runner"

    group = "RunnerPlugin"

    def run_task(self, task, hosts, connection_resolver, runner_config, max_depth):
        first_host_id, first_host = next(iter(hosts.items()))
        return task.run_on_host(
            first_host,
            connection_resolver=connection_resolver,
            max_depth=max_depth,
        )


class BatchRunnerPlugin:
    name = "python_batch_runner"

    group = "RunnerPlugin"

    def run_task(self, task, hosts, connection_resolver, runner_config, max_depth):
        return task.run_on_hosts(
            hosts,
            connection_resolver=connection_resolver,
            max_depth=max_depth,
        )

    def run_tasks(self, tasks, hosts, connection_resolver, runner_config, max_depth):
        return [
            task.run_on_hosts(
                hosts,
                connection_resolver=connection_resolver,
                max_depth=max_depth,
            )
            for task in tasks
        ]
