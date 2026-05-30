class TestConnection:
    __test__ = False

    def __init__(self, key):
        self.key = key
        self.alive = False
        self.opened_with: dict[str, object] | None = None

    def open(self, params):
        self.opened_with = params.to_dict()
        self.alive = True

    def close(self):
        self.alive = False
        return self.key

    def is_alive(self):
        return self.alive


class ConnectionPlugin:
    name = "ssh"

    group = "ConnectionPlugin"

    def create(self, key):
        return TestConnection(key)


class AsyncConnection:
    def __init__(self, key):
        self.key = key
        self.alive = False
        self.opened_with: dict[str, object] | None = None

    async def open(self, params):
        self.opened_with = params.to_dict()
        self.alive = True

    async def execute_command(self, command):
        assert self.opened_with is not None
        return f"{self.opened_with['hostname']}:{command}"

    async def close(self):
        self.alive = False
        return self.key

    async def is_alive(self):
        return self.alive


class AsyncConnectionPlugin:
    name = "async_ssh"

    group = "ConnectionPlugin"

    async def create(self, key):
        return AsyncConnection(key)
