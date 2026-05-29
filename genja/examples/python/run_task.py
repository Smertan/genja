from pathlib import Path
import json

import genja
from genja.task import TaskSuccessResult, task


EXAMPLES_DIR = Path(__file__).resolve().parents[1]
HOSTS_FILE = EXAMPLES_DIR / "inventory" / "hosts.json"


@task(name="collect_facts")
class CollectFacts:
    def run(self, task, host, context):
        return TaskSuccessResult(
            summary=f"collected facts from {host.hostname}",
            metadata={
                "hostname": host.hostname,
                "platform": host.platform,
                "facts_collected": True,
            },
        )


with HOSTS_FILE.open() as hosts_file:
    hosts = json.load(hosts_file)

runtime = genja.Genja.from_hosts(hosts).with_runner("serial")
results = runtime.run_task(CollectFacts, max_depth=1)

print(results.to_json(pretty=True))

