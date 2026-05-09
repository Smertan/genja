class HostnameSuffixTransformPlugin:
    def name(self) -> str:
        return "python_transform"

    def group(self) -> str:
        return "TransformFunctionPlugin"

    def transform_host(self, host, options):
        suffix = ""
        if options:
            suffix = options.get("suffix", "")
        return {
            **host,
            "hostname": f"{host['hostname']}{suffix}",
        }

    def transform_group(self, group, options):
        data = dict(group.get("data") or {})
        if options:
            data["transform_suffix"] = options.get("suffix")
        return {
            **group,
            "data": data,
        }


class HostOnlyTransformPlugin:
    def name(self) -> str:
        return "python_host_only_transform"

    def group(self) -> str:
        return "TransformFunctionPlugin"

    def transform_host(self, host, options):
        suffix = ""
        if options:
            suffix = options.get("suffix", "")
        return {
            **host,
            "hostname": f"{host['hostname']}{suffix}",
        }
