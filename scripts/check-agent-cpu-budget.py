#!/usr/bin/env python3
"""Exercise the opt-in launcher without starting local systemd units."""

from __future__ import annotations

import contextlib
import io
import os
import runpy
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

# These helpers run from linked worktrees without generating local cache files.
sys.dont_write_bytecode = True

import agent_cpu_budget as budget

SCRIPTS = Path(__file__).resolve().parent


class AgentQuotaChecks(unittest.TestCase):
    def test_unit(self):
        with mock.patch.object(budget.os, "cpu_count", return_value=16):
            for args, expected in (([], "1280%"), (["--percent", "50"], "800%")):
                with contextlib.redirect_stdout(io.StringIO()) as output:
                    self.assertEqual(budget.main(["unit", *args]), 0)
                self.assertIn(f"CPUQuota={expected}\n", output.getvalue())
                self.assertIn("MemoryHigh=38%\nMemoryMax=50%\nMemorySwapMax=2G\n", output.getvalue())
            with contextlib.redirect_stdout(io.StringIO()) as output:
                self.assertEqual(budget.main([
                    "unit", "--memory-high", "24G", "--memory-max", "32G", "--swap-max", "0",
                ]), 0)
            self.assertIn("MemoryHigh=24G\nMemoryMax=32G\nMemorySwapMax=0\n", output.getvalue())
            with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
                budget.main(["unit", "--percent", "101"])

    def test_resource_limits(self):
        with tempfile.TemporaryDirectory() as temp:
            slice_dir = Path(temp)
            limits = {"cpu.max": "1000000 100000", "memory.high": "24", "memory.max": "32",
                      "memory.swap.max": "0"}
            for filename, value in limits.items():
                (slice_dir / filename).write_text(value + "\n")
            with mock.patch.object(budget, "agent_slice", return_value=slice_dir):
                self.assertEqual(budget.require_limits(), (slice_dir, (1000000, 100000), 24, 32, 0))
                with contextlib.redirect_stdout(io.StringIO()) as output:
                    self.assertEqual(budget.main(["status"]), 0)
                for filename, value in limits.items():
                    self.assertIn(f"{filename}={value}\n", output.getvalue())

                for filename, invalid in (
                    ("cpu.max", "max 100000"), ("memory.high", "max"), ("memory.high", "33"),
                    ("memory.max", "0"), ("memory.swap.max", "max"), ("memory.swap.max", "-1"),
                ):
                    (slice_dir / filename).write_text(invalid)
                    with (
                        mock.patch.object(budget, "start_cache_server") as start_cache,
                        mock.patch.object(budget.os, "execvpe") as execute,
                        contextlib.redirect_stderr(io.StringIO()) as error,
                    ):
                        self.assertEqual(budget.main(["_exec", "never-run"]), 2)
                    self.assertIn(filename, error.getvalue())
                    start_cache.assert_not_called()
                    execute.assert_not_called()
                    (slice_dir / filename).write_text(limits[filename])

    def test_launcher_exports_canonical_policy(self):
        with (
            mock.patch.object(budget, "require_limits", return_value=(Path("/enrolled"), (1, 1), 1, 2, 0)),
            mock.patch.object(budget, "cache_environment", return_value={"PICTURA_AGENT_POLICY": "/stale"}),
            mock.patch.object(budget, "start_cache_server"),
            mock.patch.object(budget.os, "execvpe") as execute,
        ):
            budget.execute_in_slice(["opencode"])
        self.assertEqual(execute.call_args.args[2]["PICTURA_AGENT_POLICY"],
                         str(SCRIPTS / "heavy_command_policy.py"))

    def test_scope_command(self):
        args = ["/path with spaces/agent", "--arg", "${UNCHANGED}", "left;right"]
        with (
            mock.patch.object(budget, "agent_slice", return_value=None),
            mock.patch.object(budget.os, "execvp") as execvp,
        ):
            budget.main(["run", "--", *args])
        program, argv = execvp.call_args.args
        self.assertEqual(program, "systemd-run")
        self.assertEqual(argv[:7], [
            "systemd-run", "--user", "--scope", "--quiet", "--collect",
            "--slice=pictura-agents.slice", "--expand-environment=no",
        ])
        self.assertEqual(argv[7], "--")
        self.assertEqual(argv[-len(args):], args)

    def test_quota_validation(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            membership = root / "cgroup"
            prefix = "/user.slice/user@1000.service/pictura.slice/"
            scope = prefix + "pictura-agents.slice/probe.scope"
            for contents in ("1:cpu:/legacy", "0::/app.slice/shell.scope",
                             f"0::{scope.replace('agents.slice', 'agents-similar.slice')}",
                             "0::/../pictura-agents.slice/probe.scope"):
                membership.write_text(contents + "\n")
                self.assertIsNone(budget.agent_slice(membership, root))

            membership.write_text(f"0::{scope}\n")
            slice_dir = root / scope.strip("/").rsplit("/", 1)[0]
            slice_dir.mkdir(parents=True)
            quota_file = slice_dir / "cpu.max"
            self.assertEqual(budget.agent_slice(membership, root), slice_dir)
            with self.assertRaisesRegex(budget.BudgetError, "cpu.max"):
                budget.read_quota(slice_dir)
            for value in ("max 100000", "0 100000", "25000 0", "garbage"):
                quota_file.write_text(value + "\n")
                with self.assertRaises(budget.BudgetError):
                    budget.read_quota(slice_dir)
            quota_file.write_text("25000 100000\n")
            self.assertEqual(budget.read_quota(slice_dir), (25000, 100000))

    def test_unenrolled_execution_is_refused(self):
        with (
            mock.patch.object(budget, "agent_slice", return_value=None),
            mock.patch.object(budget.os, "execvpe") as execvpe,
            mock.patch.object(budget.os, "execvp", side_effect=FileNotFoundError("systemd-run")),
            contextlib.redirect_stderr(io.StringIO()) as error,
        ):
            self.assertEqual(budget.main(["_exec", "never-run"]), 2)
            self.assertEqual(budget.main(["run", "--", "never-run"]), 2)
            execvpe.assert_not_called()
        self.assertIn("outside pictura-agents.slice", error.getvalue())
        self.assertIn("systemd-run", error.getvalue())

    @unittest.skipUnless(hasattr(os, "getuid"), "requires POSIX ownership")
    def test_cache_environment(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            first = budget.cache_environment(root, root, {
                "XDG_RUNTIME_DIR": "/qt-test", "SCCACHE_SERVER_UDS": "/default",
                "SCCACHE_DIR": "/default-cache", "SCCACHE_SERVER_PORT": "4226",
            })
            second = budget.cache_environment(root, root, {
                "XDG_RUNTIME_DIR": "/another-qt-test", "XDG_CACHE_HOME": "/elsewhere",
            })
            self.assertEqual(first["SCCACHE_SERVER_UDS"],
                             str(root / "pictura-agents/sccache.sock"))
            self.assertEqual(first["SCCACHE_DIR"], str(root / ".cache/pictura-agents/sccache"))
            self.assertEqual(first["SCCACHE_SERVER_UDS"], second["SCCACHE_SERVER_UDS"])
            self.assertEqual(first["SCCACHE_DIR"], second["SCCACHE_DIR"])
            self.assertEqual(first["XDG_RUNTIME_DIR"], "/qt-test")
            self.assertEqual(first["SCCACHE_SERVER_PORT"], "0")
            self.assertEqual(first["SCCACHE_IDLE_TIMEOUT"], "0")
            self.assertEqual(first["SCCACHE_IGNORE_SERVER_IO_ERROR"], "1")

    def test_cache_startup(self):
        env = {"SCCACHE_SERVER_UDS": "/dedicated/sccache.sock"}
        slice_dir = Path("/enrolled")
        with (
            mock.patch.object(budget.shutil, "which", return_value="/bin/sccache"),
            mock.patch.object(budget, "cache_server_pid", side_effect=[None, 123]),
            mock.patch.object(budget, "agent_slice", return_value=slice_dir) as membership,
            mock.patch.object(budget.subprocess, "run") as start,
        ):
            budget.start_cache_server(env, slice_dir)
            self.assertEqual(start.call_args.args, (["/bin/sccache", "--start-server"],))
            self.assertEqual(start.call_args.kwargs["env"], env)
            membership.assert_called_once_with(Path("/proc/123/cgroup"))
        with (
            mock.patch.object(budget.shutil, "which", return_value="/bin/sccache"),
            mock.patch.object(budget, "cache_server_pid", return_value=123),
            mock.patch.object(budget, "agent_slice", return_value=Path("/unrelated")),
            mock.patch.object(budget.subprocess, "run") as start,
            self.assertRaisesRegex(budget.BudgetError, "outside the enrolled slice"),
        ):
            budget.start_cache_server(env, slice_dir)
        start.assert_not_called()

    def test_legacy_cache_boundary(self):
        legacy = runpy.run_path(SCRIPTS / "cpu-budget.py")
        entry = legacy["exec_command"]
        globals_ = entry.__globals__
        for enrolled in (True, False):
            scan, narrow = mock.Mock(return_value=[1]), mock.Mock()
            with (
                mock.patch.dict(globals_, {
                    "allowed_cpus": lambda: 16, "apply_slice": lambda _: True,
                    "agent_slice": lambda *_: Path("/slice") if enrolled else None,
                    "current_mask": lambda: [0, 1], "sccache_server_pids": scan,
                    "narrow_processes": narrow,
                }),
                mock.patch.dict(os.environ, {}, clear=True),
                mock.patch.object(os, "execvp", side_effect=OSError("probe")),
                contextlib.redirect_stderr(io.StringIO()),
            ):
                entry(["probe"])
                self.assertEqual(os.environ["PICTURA_JOBS"], "12")
            self.assertEqual(scan.called, not enrolled)
            self.assertEqual(narrow.called, not enrolled)

        discover = legacy["sccache_server_pids"]
        with (
            mock.patch.dict(discover.__globals__, {
                "agent_slice": lambda membership: Path("/slice")
                if membership.parts[-2] == "201" else None,
            }),
            mock.patch.object(os.path, "isdir", return_value=True),
            mock.patch.object(os, "listdir", return_value=["200", "201"]),
            mock.patch.object(Path, "read_bytes", return_value=b"sccache\0--server\0"),
        ):
            self.assertEqual(discover(), [200])

    def test_lock_path(self):
        env = {key: value for key, value in os.environ.items()
               if key != "PICTURA_VERIFY_LOCK_ROOT"}
        cmd = ["bash", "-c", '. "$1"; build_lock_dir_path', "probe",
               str(SCRIPTS / "build-lock.sh")]
        results = [subprocess.check_output(cmd, text=True, env={
            **env, "XDG_RUNTIME_DIR": f"/private/runtime/{index}", "TMPDIR": f"/tmp/{index}",
        }) for index in range(2)]
        self.assertEqual(results[0], results[1])
        self.assertTrue(results[0].endswith(".lock"))


if __name__ == "__main__":
    if sys.argv[1:] != ["--self-check"]:
        raise SystemExit("usage: check-agent-cpu-budget.py --self-check")
    unittest.main(argv=sys.argv[:1])
