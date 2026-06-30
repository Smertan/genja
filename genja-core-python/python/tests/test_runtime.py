import asyncio

import genja
from genja.inventory import Defaults, Group, Host as InventoryHost, Inventory
from genja.task import (
    Host,
    TaskMessage,
    TaskMessageLevel,
    TaskSuccessResult,
    task,
)
from tests.fixtures.connection_plugins import ConnectionPlugin, TestConnection


def test_genja_from_settings_file_rejects_invalid_path():
    try:
        genja.Genja.from_settings_file("/nonexistent/path/settings.yaml")
    except ValueError as err:
        assert "failed to build Genja runtime from settings file" in str(err)
    else:
        raise AssertionError("invalid settings path should fail")


def test_genja_from_settings_file_rejects_malformed_content(tmp_path):
    settings_path = tmp_path / "settings.yaml"
    settings_path.write_text("invalid: yaml: content: [")

    try:
        genja.Genja.from_settings_file(str(settings_path))
    except ValueError as err:
        assert "failed to build Genja runtime from settings file" in str(err)
    else:
        raise AssertionError("malformed settings content should fail")


@task(name="runtime_backup")
class RuntimeBackupTask:
    def start(self, task, host, context):
        return TaskSuccessResult(
            changed=True,
            summary=f"runtime handled {host.hostname}",
            messages=[TaskMessage(level=TaskMessageLevel.INFO, text=task.name)],
            metadata={"platform": host.platform},
        )


@task(name="runtime_async_backup")
class RuntimeAsyncBackupTask:
    async def start_async(self, task, host, context):
        await asyncio.sleep(0)
        return TaskSuccessResult(
            summary=f"async runtime handled {host.hostname}",
            metadata={"has_connection": context.has_connection()},
        )


@task(name="runtime_child")
class RuntimeChildTask:
    def start(self, task, host, context):
        return TaskSuccessResult(
            summary=f"child handled {host.hostname}",
            metadata={"has_connection": context.has_connection()},
        )


@task(
    name="runtime_parent",
    sub_tasks=[RuntimeChildTask],
)
class RuntimeParentTask:
    def start(self, task, host, context):
        return TaskSuccessResult(
            summary=f"parent handled {host.hostname}",
            metadata={"has_connection": context.has_connection()},
        )


@task(name="runtime_connection", connection_plugin_name="ssh")
class RuntimeConnectionTask:
    def start(self, task, host, context):
        assert isinstance(context.connection(), TestConnection)
        connection = context.connection()
        return TaskSuccessResult(
            summary=f"connected to {host.hostname}",
            metadata={
                "connection_alive": connection.is_alive(),
                "connection_hostname": connection.key.hostname,
                "opened_with": connection.opened_with,
            },
        )


def test_genja_runtime_runs_python_task_definition():
    runtime = genja.Genja.from_hosts({
        "router1": Host(hostname="10.0.0.1", platform="ios"),
        "router2": Host(hostname="10.0.0.2", platform="ios"),
    }).with_runner("serial")
    results = runtime.run_task(RuntimeBackupTask)
    data = results.to_dict()
    summary = results.host_summary()

    assert results.task_name == "runtime_backup"
    assert results.passed_hosts == ["router1", "router2"]
    assert results.failed_hosts == []
    assert results.skipped_hosts == []
    assert summary == {"passed": 2, "failed": 0, "skipped": 0, "total": 2}
    assert data["task_name"] == "runtime_backup"
    assert (
        data["hosts"]["router1"]["outcome"]["Passed"]["summary"]
        == "runtime handled 10.0.0.1"
    )
    assert (
        data["hosts"]["router2"]["outcome"]["Passed"]["metadata"]["platform"] == "ios"
    )


def test_genja_runtime_run_task_async_awaits_async_python_task():
    async def run_case():
        runtime = genja.Genja.from_hosts({
            "router1": Host(hostname="10.0.0.1", platform="ios"),
            "router2": Host(hostname="10.0.0.2", platform="ios"),
        }).with_runner("serial")
        return await runtime.run_task_async(RuntimeAsyncBackupTask)

    results = asyncio.run(run_case())

    assert results.task_name == "runtime_async_backup"
    assert results.passed_hosts == ["router1", "router2"]
    assert results.failed_hosts == []
    assert results.skipped_hosts == []
    data = results.to_dict()
    assert (
        data["hosts"]["router1"]["outcome"]["Passed"]["summary"]
        == "async runtime handled 10.0.0.1"
    )


def test_genja_runtime_run_tasks_async_preserves_order():
    async def run_case():
        runtime = genja.Genja.from_hosts({
            "router1": Host(hostname="10.0.0.1", platform="ios"),
            "router2": Host(hostname="10.0.0.2", platform="ios"),
        }).with_runner("serial")
        tasks = genja.Tasks()
        tasks.add_task(RuntimeBackupTask)
        tasks.add_task(RuntimeAsyncBackupTask)
        return await runtime.run_tasks_async(tasks, max_depth=1)

    results = asyncio.run(run_case())

    assert [result.task_name for result in results] == [
        "runtime_backup",
        "runtime_async_backup",
    ]
    assert results[0].passed_hosts == ["router1", "router2"]
    assert results[1].passed_hosts == ["router1", "router2"]


def test_genja_runtime_run_task_async_supports_asyncio_gather():
    async def run_case():
        runtime = genja.Genja.from_hosts({
            "router1": Host(hostname="10.0.0.1", platform="ios"),
        }).with_runner("serial")
        first, second = await asyncio.gather(
            runtime.run_task_async(RuntimeBackupTask),
            runtime.run_task_async(RuntimeAsyncBackupTask),
        )
        return first, second

    first, second = asyncio.run(run_case())

    assert first.task_name == "runtime_backup"
    assert second.task_name == "runtime_async_backup"
    assert first.passed_hosts == ["router1"]
    assert second.passed_hosts == ["router1"]


def test_genja_runtime_runs_ordered_task_list_with_nested_subtasks():
    runtime = genja.Genja.from_hosts({
        "router1": Host(hostname="10.0.0.1", platform="ios"),
        "router2": Host(hostname="10.0.0.2", platform="ios"),
    }).with_runner("serial")
    tasks = genja.Tasks()
    tasks.add_task(RuntimeBackupTask)
    tasks.add_task(RuntimeParentTask)

    assert len(tasks) == 2
    assert tasks[0].name == "runtime_backup"
    assert tasks[-1].name == "runtime_parent"
    assert [task.name for task in tasks.to_list()] == [
        "runtime_backup",
        "runtime_parent",
    ]

    results = runtime.run_tasks(tasks, max_depth=1)

    assert [result.task_name for result in results] == [
        "runtime_backup",
        "runtime_parent",
    ]
    assert results[0].passed_hosts == ["router1", "router2"]
    assert results[1].passed_hosts == ["router1", "router2"]

    parent_data = results[1].to_dict(raw=True)
    assert "runtime_child" in parent_data["sub_tasks"]
    assert (
        parent_data["sub_tasks"]["runtime_child"]["hosts"]["router1"]["outcome"]["Passed"][
            "metadata"
        ]
        == {"has_connection": False}
    )


def test_genja_runtime_run_tasks_rejects_plain_task_iterable():
    runtime = genja.Genja.from_hosts({
        "router1": Host(hostname="10.0.0.1", platform="ios"),
    }).with_runner("serial")

    try:
        runtime.run_tasks([RuntimeBackupTask, RuntimeParentTask], max_depth=1)
    except ValueError as err:
        assert "tasks must be a genja.Tasks instance" in str(err)
    else:
        raise AssertionError("plain task iterables should be rejected")


def test_task_results_to_dict_normalizes_non_raw_and_preserves_raw_shape():
    runtime = genja.Genja.from_hosts({
        "router1": Host(hostname="10.0.0.1", platform="ios"),
    }).with_runner("serial")
    results = runtime.run_task(RuntimeBackupTask)

    normalized = results.to_dict()
    raw = results.to_dict(raw=True)

    assert (
        normalized["hosts"]["router1"]["outcome"]["Passed"]["summary"]
        == "runtime handled 10.0.0.1"
    )
    assert (
        raw["hosts"]["router1"]["outcome"]["Passed"]["summary"]
        == "runtime handled 10.0.0.1"
    )


def test_genja_inventory_accessors_expose_host_payloads():
    runtime = genja.Genja.from_hosts({
        "router1": Host(hostname="10.0.0.1", platform="ios"),
        "router2": Host(hostname="10.0.0.2", port=2222, platform="nxos"),
    })

    inventory = runtime.inventory()
    raw_hosts = runtime.hosts_raw()
    inventory_hosts = runtime.iter_inventory_hosts()

    assert inventory["router1"]["hostname"] == "10.0.0.1"
    assert raw_hosts["router2"]["platform"] == "nxos"
    assert inventory_hosts == [
        ("router1", inventory["router1"]),
        ("router2", inventory["router2"]),
    ]


def test_inventory_models_serialize_to_expected_python_payloads():
    inventory = Inventory(
        hosts={
            "router1": InventoryHost(
                hostname="10.0.0.1",
                platform="ios",
                groups=["core"],
                data={"site": "a"},
            )
        },
        groups={
            "core": Group(
                platform="ios",
                data={"role": "core"},
            )
        },
        defaults=Defaults(
            username="admin",
            port=22,
        ),
    )

    data = inventory.to_dict()

    assert data["hosts"]["router1"]["groups"] == ["core"]
    assert data["groups"]["core"]["data"] == {"role": "core"}
    assert data["defaults"] == {"username": "admin", "port": 22}


def test_genja_from_inventory_preserves_groups_and_defaults():
    runtime = genja.Genja.from_inventory(
        Inventory(
            hosts={
                "router1": InventoryHost(
                    hostname="10.0.0.1",
                    groups=["core"],
                )
            },
            groups={
                "core": Group(
                    platform="ios",
                    data={"role": "core"},
                )
            },
            defaults=Defaults(
                username="admin",
                port=22,
            ),
        )
    )

    inventory_full = runtime.inventory_full()
    inventory_raw = runtime.inventory_raw()

    assert inventory_full["hosts"]["router1"]["groups"] == ["core"]
    assert inventory_full["groups"]["core"]["platform"] == "ios"
    assert inventory_raw["defaults"]["username"] == "admin"
    assert inventory_raw["defaults"]["port"] == 22


def test_genja_filter_accessors_and_execution_respect_selected_hosts():
    runtime = genja.Genja.from_hosts({
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
    }).with_runner("serial")

    filtered = runtime.filter_by_key_value("data.site.role", "^core$")
    results = filtered.run_task(RuntimeBackupTask)

    assert results.passed_hosts == ["router1"]
    assert filtered.inventory()["router2"]["hostname"] == "10.0.0.2"
    assert filtered.hosts_raw()["router1"]["platform"] == "ios"


def test_genja_runtime_hides_depth_from_python_task_context():
    runtime = genja.Genja.from_hosts({
        "router1": Host(hostname="10.0.0.1", platform="ios"),
    }).with_runner("serial")
    results = runtime.run_task(RuntimeParentTask, max_depth=1)
    data = results.to_dict(raw=True)

    assert data["hosts"]["router1"]["outcome"]["Passed"]["metadata"] == {
        "has_connection": False
    }

    child_results = data["sub_tasks"]["runtime_child"]
    assert child_results["hosts"]["router1"]["outcome"]["Passed"]["metadata"] == {
        "has_connection": False
    }


def test_genja_runtime_passes_python_connection_into_runtime_context():
    plugins = genja.PluginManager()
    plugins.register_plugin(ConnectionPlugin())

    runtime = genja.Genja.from_hosts(
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

    assert data["hosts"]["router1"]["outcome"]["Passed"]["metadata"] == {
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
        genja.Genja
        .builder({
            "router1": Host(
                hostname="10.0.0.1",
                port=22,
                username="admin",
                password="secret",
                platform="ios",
            ),
        })
        .with_plugin(ConnectionPlugin())
        .with_runner("serial")
        .build()
    )

    results = runtime.run_task(RuntimeConnectionTask)
    assert results.passed_hosts == ["router1"]
