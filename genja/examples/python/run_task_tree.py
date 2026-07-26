from pathlib import Path
import json

import genja as genja_lib
from genja.task import TaskSuccessResult, task


EXAMPLES_DIR = Path(__file__).resolve().parents[1]
HOSTS_FILE = EXAMPLES_DIR / "inventory" / "hosts.json"


@task(name="validate_config")
class ValidateConfig:
    def start(self, task, host, context):
        return TaskSuccessResult(
            summary=f"validated config for {host.hostname}",
            metadata={
                "task": task.name,
                "host": host.hostname,
                "valid": True,
            },
        )


@task(name="deploy_config", sub_tasks=[ValidateConfig])
class DeployConfig:
    def start(self, task, host, context):
        return TaskSuccessResult(
            summary=f"deployed config to {host.hostname}",
            metadata={
                "task": task.name,
                "host": host.hostname,
                "deployed": True,
            },
        )


with HOSTS_FILE.open() as hosts_file:
    hosts = json.load(hosts_file)

genja = genja_lib.Genja.from_hosts(hosts).with_runner("serial")
results = genja.run_task(
    DeployConfig,
    run_options=genja_lib.TaskRunOptions(max_depth=1),
)

print(results.to_json(pretty=True))
