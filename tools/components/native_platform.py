"""Explicit native component target identity shared by offline builders."""
import platform


def native_platform():
    systems = {"Darwin": "macos", "Linux": "linux"}
    machines = {"arm64": "aarch64", "aarch64": "aarch64", "x86_64": "x86_64", "AMD64": "x86_64"}
    system, machine = platform.system(), platform.machine()
    if system not in systems or machine not in machines:
        raise RuntimeError(f"unsupported native component platform: {system}/{machine}")
    return {"os": systems[system], "arch": machines[machine]}


def native_link_flags(target, map_path):
    if target["os"] == "macos":
        return ["-Wl,-dead_strip", "-Wl,-map," + str(map_path)]
    if target["os"] == "linux":
        return ["-Wl,--gc-sections", "-Wl,-Map," + str(map_path), "-pthread", "-ldl"]
    raise RuntimeError("unsupported native linker platform")
