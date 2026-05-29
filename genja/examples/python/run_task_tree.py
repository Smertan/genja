from pathlib import Path
import json

import genja as genja_lib
from genja.task import TaskSuccessResult, task


EXAMPLES_DIR = Path(__file__).resolve().parents[1]
HOSTS_FILE = EXAMPLES_DIR / "inventory" / "hosts.json"


@task(name="validate_config")
class ValidateConfig:
    def run(self, task, host, context):
        return TaskSuccessResult(
            summary=f"validated config for {host.hostname}",
            metadata={
                "task": task.name,
                "host": host.hostname,
                "depth": context.current_depth,
                "valid": True,
            },
        )


@task(name="deploy_config", sub_task=ValidateConfig)
class DeployConfig:
    def run(self, task, host, context):
        return TaskSuccessResult(
            summary=f"deployed config to {host.hostname}",
            metadata={
                "task": task.name,
                "host": host.hostname,
                "depth": context.current_depth,
                "deployed": True,
            },
        )


with HOSTS_FILE.open() as hosts_file:
    hosts = json.load(hosts_file)

genja = genja_lib.Genja.from_hosts(hosts).with_runner("serial")
results = genja.run_task(DeployConfig, max_depth=1)

print(results.to_json(pretty=True))
