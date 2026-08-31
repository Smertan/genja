class FirstHostOnlyRunnerPlugin:
    name = "python_runner"

    group = "RunnerPlugin"

    def run_task(self, task, hosts, connection_resolver, runner_config, run_options):
        first_host_id, first_host = next(iter(hosts.items()))
        return task.run_on_host(
            first_host,
            connection_resolver=connection_resolver,
            run_options=run_options,
        )


class BatchRunnerPlugin:
    name = "python_batch_runner"

    group = "RunnerPlugin"

    def run_task(self, task, hosts, connection_resolver, runner_config, run_options):
        return task.run_on_hosts(
            hosts,
            connection_resolver=connection_resolver,
            run_options=run_options,
        )

    def run_tasks(self, tasks, hosts, connection_resolver, runner_config, run_options):
        return [
            task.run_on_hosts(
                hosts,
                connection_resolver=connection_resolver,
                run_options=run_options,
            )
            for task in tasks
        ]
