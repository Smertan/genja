from pathlib import Path
import json

import genja


EXAMPLES_DIR = Path(__file__).resolve().parents[1]
HOSTS_FILE = EXAMPLES_DIR / "inventory" / "hosts.json"

with HOSTS_FILE.open() as hosts_file:
    hosts = json.load(hosts_file)

runtime = genja.Genja.from_hosts(hosts).with_runner("serial")
core_site = runtime.filter_by_key_value("data.site.name", "^core$")

print("Hosts in the core site:")
for host_id in core_site.host_ids():
    print(f"- {host_id}")

