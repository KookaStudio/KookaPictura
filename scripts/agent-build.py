#!/usr/bin/env python3
"""Run one managed verification job at a time across local worktrees."""

from __future__ import annotations

import contextlib
import fcntl
import json
import os
import re
import signal
import stat
import subprocess
import sys
import syslog
import tempfile
import time
import uuid
from dataclasses import asdict, dataclass
from pathlib import Path, PurePosixPath
from typing import Iterator, Sequence

sys.dont_write_bytecode = True

import agent_cpu_budget as budget
from agent_cpu_budget import BudgetError

JOB_UNIT = re.compile(r"pictura-agent-build-[0-9a-f]{32}\.scope")
POLL_SECONDS = 0.2
WAIT_SECONDS = 1800
WORKER_ENV_OVERRIDES = (
    "PICTURA_VERIFY_LOCK_HELD",
    "PICTURA_VERIFY_LOCK_ROOT",
    "PICTURA_CPU_BUDGET_APPLIED",
    "PICTURA_JOBS",
)


@dataclass(frozen=True)
class Job:
    unit: str
    worktree: str


def runtime_dir() -> Path:
    directory = Path("/run/user") / str(os.getuid()) / "pictura-agents"
    directory.mkdir(mode=0o700, exist_ok=True)
    info = directory.lstat()
    if not stat.S_ISDIR(info.st_mode) or info.st_uid != os.getuid() or info.st_mode & 0o077:
        raise BudgetError(f"{directory} must be a private, owned directory")
    return directory


@contextlib.contextmanager
def open_lock(directory: Path) -> Iterator[int]:
    path = directory / "build.lock"
    fd = os.open(path, os.O_CREAT | os.O_RDWR | os.O_NOFOLLOW, 0o600)
    try:
        info = os.fstat(fd)
        if not stat.S_ISREG(info.st_mode) or info.st_uid != os.getuid() or info.st_mode & 0o077:
            raise BudgetError(f"{path} must be a private, owned regular file")
        yield fd
    finally:
        os.close(fd)


def read_job(directory: Path) -> Job | None:
    record = directory / "build-job.json"
    try:
        data = json.loads(record.read_text(encoding="utf-8"))
        job = Job(**data)
        if not JOB_UNIT.fullmatch(job.unit) or not Path(job.worktree).is_absolute():
            raise ValueError("invalid unit or worktree")
    except FileNotFoundError:
        return None
    except (TypeError, ValueError) as error:
        raise BudgetError(f"invalid managed job record {record}: {error}") from error
    return job


def write_job(directory: Path, job: Job) -> None:
    # Readers can report the holder while another process has the queue lock.
    with tempfile.NamedTemporaryFile(mode="w", encoding="utf-8", dir=directory, delete=False) as file:
        path = Path(file.name)
        try:
            json.dump(asdict(job), file)
            file.close()
            path.replace(directory / "build-job.json")
        finally:
            path.unlink(missing_ok=True)


def current_job(directory: Path, membership: Path = budget.SELF_CGROUP) -> Job | None:
    for line in membership.read_text(encoding="utf-8").splitlines():
        if not line.startswith("0::"):
            continue
        path = PurePosixPath(line[3:])
        if path.root != "/" or ".." in path.parts or budget.SLICE not in path.parts:
            break
        components = path.parts[path.parts.index(budget.SLICE) + 1:]
        if not components or not JOB_UNIT.fullmatch(components[0]):
            break
        job = read_job(directory)
        if job is None or job.unit != components[0]:
            raise BudgetError(f"{components[0]} does not own the managed build slot")
        return job
    return None


def scope_populated(slice_dir: Path, unit: str) -> bool:
    path = slice_dir / unit / "cgroup.events"
    try:
        values = path.read_text(encoding="ascii").splitlines()
    except FileNotFoundError:
        return False
    for line in values:
        if line in ("populated 0", "populated 1"):
            return line == "populated 1"
    raise BudgetError(f"cannot determine whether {unit} is populated from {path}")


def wait_for_slot(fd: int, directory: Path, slice_dir: Path, cancelled: list[int],
                  *, wait_seconds: float = WAIT_SECONDS) -> bool:
    deadline = time.monotonic() + wait_seconds
    announced = None
    while not cancelled:
        try:
            fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            holder = read_job(directory)
        else:
            holder = read_job(directory)
            if holder is None or not scope_populated(slice_dir, holder.unit):
                return True
            fcntl.flock(fd, fcntl.LOCK_UN)

        details = f"{holder.unit} in {holder.worktree}" if holder else "a job starting up"
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            raise BudgetError(f"timed out after {wait_seconds:g}s waiting for {details}")
        if details != announced:
            print(f"agent-build: waiting for {details}", file=sys.stderr)
            announced = details
        time.sleep(min(POLL_SECONDS, remaining))
    return False


def start_scope(fd: int, unit: str, command: Sequence[str]) -> subprocess.Popen:
    env = dict(os.environ)
    for name in WORKER_ENV_OVERRIDES:
        env.pop(name, None)
    # A gate from an older worktree may still attempt the portable lock.
    env["PICTURA_VERIFY_LOCK"] = "0"
    argv = [
        "systemd-run", "--user", "--scope", "--quiet", "--collect", "--no-ask-password",
        f"--slice={budget.SLICE}", f"--unit={unit}", "--property=KillMode=control-group",
        "--property=TimeoutStopSec=10s", "--expand-environment=no", "--",
        sys.executable, str(Path(__file__).resolve()), "_worker", str(fd), unit, *command,
    ]
    # Keep the lock across startup if this client dies before systemd adds the
    # child to the scope. The worker closes its copy once membership is checked.
    return subprocess.Popen(argv, env=env, pass_fds=(fd,), start_new_session=True)


def stop_scope(unit: str) -> None:
    try:
        result = subprocess.run(
            ["systemctl", "--user", "--no-ask-password", "stop", unit],
            timeout=20, check=False,
        )
    except subprocess.TimeoutExpired as error:
        raise BudgetError(f"timed out stopping {unit}") from error
    if result.returncode != 0:
        raise BudgetError(f"systemctl failed to stop {unit} (exit {result.returncode})")


def finish_scope(process: subprocess.Popen, slice_dir: Path, unit: str) -> None:
    if process.poll() is None and not scope_populated(slice_dir, unit):
        # Cancel a starter that has not yet been placed in the scope.
        process.terminate()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=5)
    if scope_populated(slice_dir, unit):
        try:
            stop_scope(unit)
        except BudgetError:
            if scope_populated(slice_dir, unit):
                raise
        if scope_populated(slice_dir, unit):
            raise BudgetError(f"{unit} is still populated after stopping it")
    process.wait(timeout=5)


def log_job(event: str, job: Job, slice_dir: Path, status: int | None = None) -> None:
    try:
        memory_events = dict(
            line.split() for line in (slice_dir / "memory.events").read_text(encoding="ascii").splitlines()
        )
    except (OSError, ValueError):
        memory_events = {}
    syslog.syslog(syslog.LOG_INFO, json.dumps({
        "event": event,
        **asdict(job),
        "slice_memory_events": memory_events,
        "status": status,
    }))


def run_job(fd: int, directory: Path, slice_dir: Path, job: Job, command: Sequence[str],
            cancelled: list[int]) -> int:
    write_job(directory, job)
    print(f"agent-build: starting {job.unit} in {job.worktree}", file=sys.stderr)
    status = None
    process = None
    try:
        log_job("start", job, slice_dir)
        process = start_scope(fd, job.unit, command)
        while not cancelled:
            try:
                status = process.wait(timeout=POLL_SECONDS)
                break
            except subprocess.TimeoutExpired:
                pass
    finally:
        event = "cleanup-failed"
        try:
            if process is not None:
                finish_scope(process, slice_dir, job.unit)
            (directory / "build-job.json").unlink()
            event = "finish"
        finally:
            log_job(event, job, slice_dir, status)
    if cancelled:
        return 128 + cancelled[0]
    assert status is not None
    return 128 - status if status < 0 else status


def worker(directory: Path, fd_text: str, unit: str, command: Sequence[str]) -> None:
    job = current_job(directory)
    if job is None or job.unit != unit:
        raise BudgetError(f"worker is outside its queued scope {unit}")
    try:
        fd = int(fd_text)
        if fd <= 2 or not os.path.samestat(os.fstat(fd), (directory / "build.lock").stat()):
            raise ValueError("wrong lock descriptor")
    except (OSError, ValueError) as error:
        raise BudgetError(f"cannot verify the scope starter's lock: {error}") from error
    os.close(fd)
    helper = Path(__file__).with_name("cpu-budget.py").resolve()
    os.execv(sys.executable, [sys.executable, str(helper), "exec", "--", *command])


def main(argv: Sequence[str]) -> int:
    if argv != ["--inside"] and not (
        len(argv) >= 2 and argv[0] == "--" or len(argv) >= 4 and argv[0] == "_worker"
    ):
        print("usage: agent-build.py -- COMMAND [ARG ...]", file=sys.stderr)
        return 2
    try:
        slice_dir = budget.require_limits()[0]
        directory = runtime_dir()
        if argv[0] == "_worker":
            worker(directory, argv[1], argv[2], argv[3:])
            return 0

        inside = current_job(directory)
        if argv[0] == "--inside":
            if inside is None:
                raise BudgetError("not inside the current managed build job")
            return 0
        command = argv[1:]
        if inside is not None:
            os.execvp(command[0], command)
            return 0

        cancelled: list[int] = []

        def cancel(signum: int, _frame: object) -> None:
            if not cancelled:
                cancelled.append(signum)

        syslog.openlog("pictura-agent-build")
        previous_handlers = {
            sig: signal.signal(sig, cancel) for sig in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP)
        }
        try:
            with open_lock(directory) as fd:
                if wait_for_slot(fd, directory, slice_dir, cancelled) and not cancelled:
                    job = Job(f"pictura-agent-build-{uuid.uuid4().hex}.scope", str(Path.cwd()))
                    return run_job(fd, directory, slice_dir, job, command, cancelled)
            return 128 + cancelled[0]
        finally:
            for sig, previous in previous_handlers.items():
                signal.signal(sig, previous)
    except (BudgetError, OSError, ValueError, subprocess.TimeoutExpired) as error:
        print(f"agent-build: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
