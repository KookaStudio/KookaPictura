#!/usr/bin/env python3
"""Test the managed verification queue without creating systemd scopes."""

from __future__ import annotations

import contextlib
import io
import json
import os
import runpy
import signal
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

sys.dont_write_bytecode = True

if sys.platform == "linux":
    queue = runpy.run_path(str(Path(__file__).with_name("agent-build.py")))
    Job = queue["Job"]
    implementation = queue["main"].__globals__


@unittest.skipUnless(sys.platform == "linux", "requires Linux queue primitives")
class AgentBuildChecks(unittest.TestCase):
    def setUp(self):
        storage = tempfile.TemporaryDirectory()
        self.addCleanup(storage.cleanup)
        root = Path(storage.name)
        self.runtime = root / "runtime"
        self.runtime.mkdir(mode=0o700)
        self.slice = root / "pictura-agents.slice"
        self.slice.mkdir()
        self.job = Job("pictura-agent-build-" + "a" * 32 + ".scope", "/first worktree")

    def populate(self, value: int):
        directory = self.slice / self.job.unit
        directory.mkdir(exist_ok=True)
        (directory / "cgroup.events").write_text(f"populated {value}\nfrozen 0\n", encoding="ascii")

    def test_queue(self):
        with queue["open_lock"](self.runtime) as first, queue["open_lock"](self.runtime) as second:
            self.assertTrue(queue["wait_for_slot"](first, self.runtime, self.slice, [], wait_seconds=0))
            queue["write_job"](self.runtime, self.job)
            with mock.patch.dict(os.environ, {
                "PICTURA_VERIFY_LOCK": "0",
                "PICTURA_VERIFY_LOCK_HELD": "1",
                "PICTURA_VERIFY_LOCK_ROOT": "/private/other",
            }), self.assertRaisesRegex(queue["BudgetError"], "/first worktree"):
                queue["wait_for_slot"](second, self.runtime, self.slice, [], wait_seconds=0)
        with queue["open_lock"](self.runtime) as second:
            self.assertTrue(queue["wait_for_slot"](second, self.runtime, self.slice, [], wait_seconds=0))

    def test_nested_job(self):
        membership = self.runtime / "membership"
        path = f"/user.slice/pictura.slice/pictura-agents.slice/{self.job.unit}"
        queue["write_job"](self.runtime, self.job)
        membership.write_text(f"0::{path}\n", encoding="ascii")
        self.assertEqual(queue["current_job"](self.runtime, membership), self.job)
        membership.write_text("0::/pictura-agents.slice/some-other.scope\n", encoding="ascii")
        with mock.patch.dict(os.environ, {"PICTURA_VERIFY_LOCK_HELD": "1"}):
            self.assertIsNone(queue["current_job"](self.runtime, membership))
        membership.write_text(f"0::{path.replace('a' * 32, 'b' * 32)}\n", encoding="ascii")
        with self.assertRaisesRegex(queue["BudgetError"], "does not own"):
            queue["current_job"](self.runtime, membership)

        with (
            mock.patch.object(queue["budget"], "require_limits", return_value=(self.slice,)),
            mock.patch.dict(implementation, {
                "runtime_dir": lambda: self.runtime,
                "current_job": lambda _: self.job,
                "wait_for_slot": mock.Mock(side_effect=AssertionError("nested jobs do not queue")),
            }),
            mock.patch.object(os, "execvp") as execute,
        ):
            self.assertEqual(queue["main"](["--", "true"]), 0)
        execute.assert_called_once_with("true", ["true"])

    def test_abandoned_job(self):
        queue["write_job"](self.runtime, self.job)
        self.populate(1)
        with queue["open_lock"](self.runtime) as lock:
            with self.assertRaisesRegex(queue["BudgetError"], self.job.unit):
                queue["wait_for_slot"](lock, self.runtime, self.slice, [], wait_seconds=0)
            self.populate(0)
            self.assertTrue(queue["wait_for_slot"](lock, self.runtime, self.slice, [], wait_seconds=0))

    def test_cancellation(self):
        self.populate(1)
        process = mock.Mock()
        process.poll.return_value = None
        process.wait.return_value = -signal.SIGTERM
        stop = mock.Mock(side_effect=lambda _: self.populate(0))
        with (
            mock.patch.dict(implementation, {
                "start_scope": mock.Mock(return_value=process),
                "stop_scope": stop,
                "log_job": mock.Mock(),
            }),
            queue["open_lock"](self.runtime) as lock,
            contextlib.redirect_stderr(io.StringIO()),
        ):
            rc = queue["run_job"](lock, self.runtime, self.slice, self.job, ["true"], [signal.SIGTERM])
        self.assertEqual(rc, 128 + signal.SIGTERM)
        stop.assert_called_once_with(self.job.unit)
        self.assertIsNone(queue["read_job"](self.runtime))

    def test_cleanup_failure_leaves_live_job_record(self):
        self.populate(1)
        process = mock.Mock()
        process.poll.return_value = 0
        process.wait.return_value = 0
        with (
            mock.patch.dict(implementation, {
                "start_scope": mock.Mock(return_value=process),
                "stop_scope": mock.Mock(),
                "log_job": mock.Mock(),
            }),
            queue["open_lock"](self.runtime) as lock,
            contextlib.redirect_stderr(io.StringIO()),
            self.assertRaisesRegex(queue["BudgetError"], "still populated"),
        ):
            queue["run_job"](lock, self.runtime, self.slice, self.job, ["true"], [])
        self.assertEqual(queue["read_job"](self.runtime), self.job)

    def test_managed_shell_refuses_to_fall_back_outside_slice(self):
        if queue["budget"].agent_slice() is not None:
            self.skipTest("requires a shell outside the managed slice")
        lock_script = Path(__file__).with_name("build-lock.sh")
        command = [
            "bash", "-c", '. "$1"; build_lock_enter_managed "$2" true', "probe",
            str(lock_script), str(Path(__file__).with_name("agent-build.py")),
        ]
        env = {name: value for name, value in os.environ.items() if name != "PICTURA_AGENT_POLICY"}
        ordinary = subprocess.run(command, env=env, check=False, capture_output=True, text=True)
        self.assertEqual(ordinary.returncode, 0, ordinary.stderr)
        managed = subprocess.run(command, env={**env, "PICTURA_AGENT_POLICY": "/host/policy.py"},
                                 check=False, capture_output=True, text=True)
        self.assertEqual(managed.returncode, 2)
        self.assertIn("managed shell is outside pictura-agents.slice", managed.stderr)

    def test_portable_wrapper_budget(self):
        if queue["budget"].agent_slice() is not None:
            self.skipTest("requires a shell outside the managed slice")
        wrapper = Path(__file__).with_name("with-build-lock.sh").resolve()
        probe = (
            "import json, os, sys;"
            "print(json.dumps([os.getcwd(), os.environ['PICTURA_CPU_BUDGET_APPLIED'],"
            "os.environ['PICTURA_JOBS'], os.environ['CARGO_BUILD_JOBS']]));"
            "sys.exit(7)"
        )
        cleared = {"PICTURA_AGENT_POLICY", "PICTURA_CPU_BUDGET_APPLIED", "PICTURA_JOBS", "CARGO_BUILD_JOBS"}
        env = {name: value for name, value in os.environ.items() if name not in cleared}
        env.update(PICTURA_CPU_BUDGET_PERCENT="50", PICTURA_VERIFY_LOCK="0")
        child = subprocess.run(
            ["bash", str(wrapper), "--", sys.executable, "-c", probe],
            cwd=self.runtime, env=env, check=False, capture_output=True, text=True,
        )
        jobs = str(max(1, len(os.sched_getaffinity(0)) // 2))
        self.assertEqual(child.returncode, 7, child.stderr)
        self.assertEqual(json.loads(child.stdout), [str(self.runtime), "1", jobs, jobs])

    def test_scope_inherits_the_lock_until_worker_checks_membership(self):
        with (
            mock.patch.object(queue["subprocess"], "Popen") as popen,
            queue["open_lock"](self.runtime) as lock,
        ):
            queue["start_scope"](lock, self.job.unit, ["printf", "%s", "${verbatim}"])
        args, kwargs = popen.call_args
        self.assertIn(f"--unit={self.job.unit}", args[0])
        self.assertIn("--expand-environment=no", args[0])
        self.assertEqual(args[0][-4:], [self.job.unit, "printf", "%s", "${verbatim}"])
        self.assertEqual(kwargs["pass_fds"], (lock,))
        self.assertEqual(kwargs["env"]["PICTURA_VERIFY_LOCK"], "0")
        self.assertNotIn("PICTURA_VERIFY_LOCK_HELD", kwargs["env"])

    def test_worker_refuses_unverified_scope(self):
        with (
            mock.patch.dict(implementation, {"current_job": lambda _: None}),
            mock.patch.object(os, "execv") as execute,
            self.assertRaisesRegex(queue["BudgetError"], "outside its queued scope"),
        ):
            queue["worker"](self.runtime, "3", self.job.unit, ["true"])
        execute.assert_not_called()


if __name__ == "__main__":
    if sys.argv[1:] != ["--self-check"]:
        raise SystemExit("usage: check-agent-build.py --self-check")
    unittest.main(argv=sys.argv[:1])
