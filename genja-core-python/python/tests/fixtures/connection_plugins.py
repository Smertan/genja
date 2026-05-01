class TestConnection:
    def __init__(self, key):
        self.key = key
        self.alive = False
        self.opened_with = None

    def open(self, params):
        self.opened_with = params.to_dict()
        self.alive = True

    def close(self):
        self.alive = False
        return self.key

    def is_alive(self):
        return self.alive


class ConnectionPlugin:
    def name(self):
        return "ssh"

    def group(self):
        return "ConnectionPlugin"

    def create(self, key):
        return TestConnection(key)
