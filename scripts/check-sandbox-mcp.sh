#!/usr/bin/env bash
# Prove an installed package serves agents: a real window, enabled through the
# stored session as a person would leave it, answers the command the Preferences
# page advertises, run from outside the sandbox. Qt Test can't see this, because
# it depends on two instances of a real sandbox meeting on one path.
#
#   scripts/check-sandbox-mcp.sh flatpak|snap
set -euo pipefail

exec python3 - "${1:?usage: check-sandbox-mcp.sh flatpak|snap}" <<'PY'
import json
import os
import subprocess
import sys
import time

FLATPAK_ID = "io.github.KookaStudio.KookaPictura"
SNAP_APP = "kooka-pictura.pictura"
STATE_HOME = 'printf %s "${XDG_STATE_HOME:-$HOME/.local/state}"'
START_TIMEOUT = 90

kind = sys.argv[1]
if kind == "flatpak":
    shell = ["flatpak", "run", "--command=sh", FLATPAK_ID, "-c"]
    window = ["flatpak", "run", "--env=QT_QPA_PLATFORM=offscreen", FLATPAK_ID]
    client = ["flatpak", "run", FLATPAK_ID, "--mcp"]
elif kind == "snap":
    shell = ["snap", "run", "--shell", SNAP_APP, "-c"]
    window = ["env", "QT_QPA_PLATFORM=offscreen", "snap", "run", SNAP_APP]
    client = ["snap", "run", SNAP_APP, "--mcp"]
else:
    sys.exit(f"unknown package: {kind}")


def fail(message):
    print(f"check-sandbox-mcp: FAIL {message}", file=sys.stderr)
    sys.exit(1)


def exchange():
    """The replies to one client session, or why there are none yet."""
    requests = [
        ("initialize", {"protocolVersion": "2025-06-18"}),
        ("tools/list", {}),
        ("tools/call", {"name": "pictura_status", "arguments": {}}),
    ]
    lines = [
        json.dumps({"jsonrpc": "2.0", "id": i + 1, "method": method, "params": params})
        for i, (method, params) in enumerate(requests)
    ]
    done = subprocess.run(client, input="\n".join(lines) + "\n", capture_output=True,
                          text=True, timeout=30)
    # Anything but protocol on stdout would corrupt the client's stream.
    replies = [json.loads(line) for line in done.stdout.splitlines()]
    if len(replies) != len(requests) or any("result" not in reply for reply in replies):
        return None, done.stderr.strip() or done.stdout.strip()
    return replies, ""


# Ask the sandbox, so this seeds the path the app really reads.
state_home = subprocess.run([*shell, STATE_HOME], capture_output=True, text=True, check=True)
state_dir = os.path.join(state_home.stdout, "kooka-pictura")
os.makedirs(state_dir, exist_ok=True)
with open(os.path.join(state_dir, "state.json"), "w", encoding="utf-8") as state:
    json.dump({"agentControlEnabled": True}, state)

running = subprocess.Popen(window, stdin=subprocess.DEVNULL)
try:
    deadline = time.monotonic() + START_TIMEOUT
    replies, why = None, ""
    while replies is None and time.monotonic() < deadline:
        if running.poll() is not None:
            fail(f"the window exited {running.returncode}")
        replies, why = exchange()
        if replies is None:
            time.sleep(1)
    if replies is None:
        fail(f"no answer in {START_TIMEOUT}s: {why}")
    if len(replies[1]["result"]["tools"]) < 20:
        fail("too few tools")
    status = replies[2]["result"]
    if status.get("isError") or "documents" not in status["content"][0]["text"]:
        fail(f"status: {status}")
finally:
    running.terminate()
    try:
        running.wait(timeout=15)
    except subprocess.TimeoutExpired:
        running.kill()

print(f"check-sandbox-mcp: {kind} OK")
PY
