from pathlib import Path
import json

import genja as genja_lib


EXAMPLES_DIR = Path(__file__).resolve().parents[1]
HOSTS_FILE = EXAMPLES_DIR / "inventory" / "hosts.json"

with HOSTS_FILE.open() as hosts_file:
    hosts = json.load(hosts_file)

genja = genja_lib.Genja.from_hosts(hosts).with_runner("serial")

print("Loaded hosts:")
for host_id in genja.host_ids():
    print(f"- {host_id}")
