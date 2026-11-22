#!/usr/bin/env python3
"""Generate a user slice and launch agents under its cgroup v2 resource limits."""

from __future__ import annotations

import argparse
import os
import re
import shutil
import socket
import stat
import struct
import subprocess
import sys
from pathlib import Path, PurePosixPath
from typing import Mapping, Sequence

SLICE = "pictura-agents.slice"
CGROUP_ROOT = Path("/sys/fs/cgroup")
SELF_CGROUP = Path("/proc/self/cgroup")


class BudgetError(Exception):
    """The managed resource limits could not be established."""


def agent_slice(
    membership: Path = SELF_CGROUP, root: Path = CGROUP_ROOT,
) -> Path | None:
    try:
        lines = membership.read_text(encoding="utf-8").splitlines()
    except (OSError, UnicodeError):
        return None
    for line in lines:
        if not line.startswith("0::"):
            continue
        path = PurePosixPath(line[3:])
        if path.root != "/" or ".." in path.parts or SLICE not in path.parts:
            return None
        return root.joinpath(*path.parts[1:path.parts.index(SLICE) + 1])
    return None


def read_quota(slice_dir: Path) -> tuple[int, int]:
    quota_file = slice_dir / "cpu.max"
    try:
        quota, period = map(int, quota_file.read_text(encoding="ascii").split())
    except OSError as error:
        raise BudgetError(f"cannot read quota from {quota_file}: {error}") from error
    except ValueError as error:
        raise BudgetError(f"{quota_file} has no finite CPU quota") from error
    if quota <= 0 or period <= 0:
        raise BudgetError(f"{quota_file} has an invalid CPU quota")
    return quota, period


def read_memory_limit(slice_dir: Path, name: str, minimum: int) -> int:
    limit_file = slice_dir / name
    try:
        raw = limit_file.read_text(encoding="ascii").strip()
    except (OSError, UnicodeError) as error:
        raise BudgetError(f"cannot read limit from {limit_file}: {error}") from error
    if not raw.isdecimal() or int(raw) < minimum:
        raise BudgetError(f"{limit_file} has no valid finite limit")
    return int(raw)


def require_limits(
    membership: Path = SELF_CGROUP,
) -> tuple[Path, tuple[int, int], int, int, int]:
    slice_dir = agent_slice(membership)
    if slice_dir is None:
        raise BudgetError(f"{membership} is outside {SLICE} or cannot be read")
    cpu = read_quota(slice_dir)
    high = read_memory_limit(slice_dir, "memory.high", 1)
    maximum = read_memory_limit(slice_dir, "memory.max", 1)
    swap = read_memory_limit(slice_dir, "memory.swap.max", 0)
    if high > maximum:
        raise BudgetError(f"{slice_dir / 'memory.high'} exceeds memory.max")
    return slice_dir, cpu, high, maximum, swap


def percent_value(raw: str) -> int:
    if raw.isascii() and raw.isdecimal():
        percent = int(raw)
        if 1 <= percent <= 100:
            return percent
    raise argparse.ArgumentTypeError("percentage must be a decimal integer from 1 to 100")


def memory_size(raw: str) -> str:
    if re.fullmatch(r"[1-9][0-9]*(?:[KMGT]|%)?", raw):
        if not raw.endswith("%") or int(raw[:-1]) <= 100:
            return raw
    raise argparse.ArgumentTypeError("expected a positive size such as 24G or 50%")


def swap_size(raw: str) -> str:
    return raw if raw == "0" else memory_size(raw)


def cache_environment(
    runtime: Path, home: Path, environment: Mapping[str, str],
) -> dict[str, str]:
    cache_runtime = runtime / "pictura-agents"
    cache_runtime.mkdir(mode=0o700, exist_ok=True)
    info = cache_runtime.lstat()
    if (not stat.S_ISDIR(info.st_mode) or info.st_uid != os.getuid()
            or info.st_mode & 0o077):
        raise BudgetError(f"{cache_runtime} must be a private, owned directory")
    env = dict(environment)
    # Older clients that ignore the Unix socket must never use the default server.
    env["SCCACHE_SERVER_PORT"] = "0"
    env["SCCACHE_SERVER_UDS"] = str(cache_runtime / "sccache.sock")
    env["SCCACHE_DIR"] = str(home / ".cache/pictura-agents/sccache")
    env["SCCACHE_IDLE_TIMEOUT"] = "0"
    env.setdefault("SCCACHE_IGNORE_SERVER_IO_ERROR", "1")
    return env


def cache_server_pid(socket_path: str) -> int | None:
    try:
        with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as peer:
            peer.connect(socket_path)
            credentials = peer.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, struct.calcsize("3i"))
    except (FileNotFoundError, ConnectionRefusedError):
        return None
    return struct.unpack("3i", credentials)[0]


def start_cache_server(env: dict[str, str], slice_dir: Path) -> None:
    sccache = shutil.which("sccache", path=env.get("PATH"))
    if sccache is None:
        return
    socket_path = env["SCCACHE_SERVER_UDS"]
    pid = cache_server_pid(socket_path)
    if pid is None:
        try:
            started = subprocess.run([sccache, "--start-server"], env=env,
                                     capture_output=True, text=True, timeout=10)
        except subprocess.TimeoutExpired as error:
            raise BudgetError(f"sccache startup timed out for {socket_path}") from error
        pid = cache_server_pid(socket_path)
        if pid is None:
            raise BudgetError(
                f"sccache did not create {socket_path} (exit {started.returncode}); "
                "install a version with SCCACHE_SERVER_UDS support"
            )
    if pid <= 0 or agent_slice(Path(f"/proc/{pid}/cgroup")) != slice_dir:
        raise BudgetError(f"sccache server at {socket_path} is outside the enrolled slice")


def execute_in_slice(command: Sequence[str]) -> None:
    slice_dir, _, _, _, _ = require_limits()
    env = cache_environment(Path("/run/user") / str(os.getuid()), Path.home(), os.environ)
    env["PICTURA_AGENT_POLICY"] = str(Path(__file__).resolve().with_name("heavy_command_policy.py"))
    start_cache_server(env, slice_dir)
    os.execvpe(command[0], list(command), env)


def run_in_scope(command: Sequence[str]) -> None:
    if agent_slice() is not None:
        execute_in_slice(command)
        return
    argv = [
        "systemd-run", "--user", "--scope", "--quiet", "--collect",
        f"--slice={SLICE}", "--expand-environment=no", "--",
        sys.executable, str(Path(__file__).resolve()), "_exec", *command,
    ]
    os.execvp(argv[0], argv)


def main(argv: Sequence[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="action", required=True)
    unit = commands.add_parser("unit", help="print the slice unit for manual installation")
    unit.add_argument("--percent", type=percent_value, default=80, help="share of online CPUs")
    unit.add_argument("--memory-high", type=memory_size, default="38%", help="memory pressure threshold")
    unit.add_argument("--memory-max", type=memory_size, default="50%", help="hard memory ceiling")
    unit.add_argument("--swap-max", type=swap_size, default="2G", help="hard swap ceiling")
    run = commands.add_parser("run", help="start a command under the configured user slice")
    run.add_argument("command", nargs=argparse.REMAINDER, metavar="-- COMMAND [ARG ...]")
    status = commands.add_parser("status", help="verify a process belongs to the resource-limited slice")
    status.add_argument("--pid", type=int, help="inspect this PID instead of the status process")

    try:
        if argv and argv[0] == "_exec":
            if len(argv) == 1:
                parser.error("missing command")
            execute_in_slice(argv[1:])
            return 0
        args = parser.parse_args(argv)
        if args.action == "unit":
            count = os.cpu_count()
            if count is None:
                raise BudgetError("cannot determine the number of online CPUs")
            print(f"[Unit]\nDescription=Pictura coding agents\n\n"
                  f"[Slice]\nCPUQuota={count * args.percent}%\n"
                  f"MemoryHigh={args.memory_high}\nMemoryMax={args.memory_max}\n"
                  f"MemorySwapMax={args.swap_max}")
        elif args.action == "run":
            command = args.command
            if command and command[0] == "--":
                command = command[1:]
            if not command:
                run.error("missing command")
            run_in_scope(command)
        else:
            if args.pid is not None and args.pid <= 0:
                status.error("--pid must be positive")
            membership = SELF_CGROUP if args.pid is None else Path(f"/proc/{args.pid}/cgroup")
            slice_dir, (quota, period), high, maximum, swap = require_limits(membership)
            print(f"cgroup={slice_dir}\ncpu.max={quota} {period}\n"
                  f"memory.high={high}\nmemory.max={maximum}\nmemory.swap.max={swap}")
    except (BudgetError, OSError) as error:
        print(f"agent-cpu-budget: {error}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
