import genja_core
from genja_core.task import Host, TaskMessage, TaskSuccessResult, task
from tests.fixtures.connection_plugins import ConnectionPlugin, TestConnection


@task(name="runtime_backup")
class RuntimeBackupTask:
    def run(self, task, host, context):
        return TaskSuccessResult(
            changed=True,
            summary=f"runtime handled {host.hostname}",
            messages=[TaskMessage(level="info", text=task.name)],
            metadata={"platform": host.platform},
        )


@task(name="runtime_child")
class RuntimeChildTask:
    def run(self, task, host, context):
        return TaskSuccessResult(
            summary=f"child handled {host.hostname}",
            metadata={
                "current_depth": context.current_depth,
                "max_depth": context.max_depth,
            },
        )


@task(
    name="runtime_parent",
    sub_task=RuntimeChildTask,
)
class RuntimeParentTask:
    def run(self, task, host, context):
        return TaskSuccessResult(
            summary=f"parent handled {host.hostname}",
            metadata={
                "current_depth": context.current_depth,
                "max_depth": context.max_depth,
            },
        )


@task(name="runtime_connection", connection_plugin_name="ssh")
class RuntimeConnectionTask:
    def run(self, task, host, context):
        assert isinstance(context.connection, TestConnection)
        return TaskSuccessResult(
            summary=f"connected to {host.hostname}",
            metadata={
                "connection_alive": context.connection.is_alive(),
                "connection_hostname": context.connection.key.hostname,
                "opened_with": context.connection.opened_with,
            },
        )


def test_genja_runtime_runs_python_task_definition():
    runtime = genja_core.Genja.from_hosts(
        {
            "router1": Host(hostname="10.0.0.1", platform="ios"),
            "router2": Host(hostname="10.0.0.2", platform="ios"),
        }
    ).with_runner("serial")
    results = runtime.run_task(RuntimeBackupTask)
    data = results.to_dict()
    summary = results.host_summary()

    assert results.task_name == "runtime_backup"
    assert results.passed_hosts == ["router1", "router2"]
    assert results.failed_hosts == []
    assert results.skipped_hosts == []
    assert summary == {"passed": 2, "failed": 0, "skipped": 0, "total": 2}
    assert data["task_name"] == "runtime_backup"
    assert data["hosts"]["router1"]["status"] == "passed"
    assert data["hosts"]["router1"]["summary"] == "runtime handled 10.0.0.1"
    assert data["hosts"]["router2"]["status"] == "passed"
    assert data["hosts"]["router2"]["metadata"]["platform"] == "ios"


def test_genja_inventory_accessors_expose_host_payloads():
    runtime = genja_core.Genja.from_hosts(
        {
            "router1": Host(hostname="10.0.0.1", platform="ios"),
            "router2": Host(hostname="10.0.0.2", port=2222, platform="nxos"),
        }
    )

    inventory = runtime.inventory()
    raw_hosts = runtime.hosts_raw()
    inventory_hosts = runtime.iter_inventory_hosts()

    assert inventory["router1"]["hostname"] == "10.0.0.1"
    assert raw_hosts["router2"]["platform"] == "nxos"
    assert inventory_hosts == [
        ("router1", inventory["router1"]),
        ("router2", inventory["router2"]),
    ]


def test_genja_filter_accessors_and_execution_respect_selected_hosts():
    runtime = genja_core.Genja.from_hosts(
        {
            "router1": Host(
                hostname="10.0.0.1",
                platform="ios",
                data={"site": {"role": "core"}},
            ),
            "router2": Host(
                hostname="10.0.0.2",
                platform="nxos",
                data={"site": {"role": "edge"}},
            ),
        }
    ).with_runner("serial")

    filtered = runtime.filter_by_key_value("data.site.role", "^core$")
    results = filtered.run_task(RuntimeBackupTask)

    assert results.passed_hosts == ["router1"]
    assert filtered.inventory()["router2"]["hostname"] == "10.0.0.2"
    assert filtered.hosts_raw()["router1"]["platform"] == "ios"


def test_genja_runtime_passes_real_task_context_depth():
    runtime = genja_core.Genja.from_hosts(
        {
            "router1": Host(hostname="10.0.0.1", platform="ios"),
        }
    ).with_runner("serial")
    results = runtime.run_task(RuntimeParentTask, max_depth=1)
    data = results.to_dict(raw=True)

    assert data["hosts"]["router1"]["Passed"]["metadata"] == {
        "current_depth": 0,
        "max_depth": 1,
    }

    child_results = data["sub_tasks"]["runtime_child"]
    assert child_results["hosts"]["router1"]["Passed"]["metadata"] == {
        "current_depth": 1,
        "max_depth": 1,
    }


def test_genja_runtime_passes_python_connection_into_runtime_context():
    plugins = genja_core.PluginManager()
    plugins.register_plugin(ConnectionPlugin())

    runtime = genja_core.Genja.from_hosts(
        {
            "router1": Host(
                hostname="10.0.0.1",
                port=22,
                username="admin",
                password="secret",
                platform="ios",
            ),
        },
        plugin_manager=plugins,
    ).with_runner("serial")
    results = runtime.run_task(RuntimeConnectionTask)
    data = results.to_dict()

    assert data["hosts"]["router1"]["status"] == "passed"
    assert data["hosts"]["router1"]["metadata"] == {
        "connection_alive": True,
        "connection_hostname": "router1",
        "opened_with": {
            "hostname": "10.0.0.1",
            "port": 22,
            "username": "admin",
            "password": "secret",
            "platform": "ios",
            "extras": None,
        },
    }


def test_genja_builder_registers_plugin_and_builds_runtime():
    runtime = (
        genja_core.Genja.builder(
            {
                "router1": Host(
                    hostname="10.0.0.1",
                    port=22,
                    username="admin",
                    password="secret",
                    platform="ios",
                ),
            }
        )
        .with_plugin(ConnectionPlugin())
        .with_runner("serial")
        .build()
    )

    results = runtime.run_task(RuntimeConnectionTask)
    assert results.passed_hosts == ["router1"]
