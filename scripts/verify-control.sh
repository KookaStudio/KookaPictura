#!/usr/bin/env bash
#
# verify-control.sh - live end-to-end check of the agentic control server.
#
# Launches a throwaway `pictura --headless --control` on a temporary local
# socket and drives the documented recipe over the real newline-delimited JSON
# protocol: status, a wand selection, a filter, a pixel readback, undo, and a
# screenshot. The in-process --self-test block exercises dispatch directly; this
# is the only check that goes through QLocalServer/QLocalSocket and a separate
# process. No display required (offscreen). No new dependency (python3 stdlib).

set -euo pipefail
cd "$(dirname "$0")/.."

BIN=./build/pictura
FIXTURE=crates/pictura-codec/tests/fixtures/two_layers.psd

if [ ! -x "$BIN" ]; then
    echo "verify-control: missing $BIN (build first: cmake --build build --parallel)" >&2
    exit 1
fi
if [ ! -f "$FIXTURE" ]; then
    echo "verify-control: missing fixture $FIXTURE" >&2
    exit 1
fi

tmp="$(mktemp -d)"
sock="$tmp/control.sock"
log="$tmp/app.log"
pid=""

cleanup() {
    if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
        kill "$pid" 2>/dev/null || true
        wait "$pid" 2>/dev/null || true
    fi
    rm -rf "$tmp"
}
trap cleanup EXIT

"$BIN" --headless --control --control-socket "$sock" --state-home "$tmp/state" \
    "$FIXTURE" >"$log" 2>&1 &
pid=$!

# Wait for the socket and its session token (bounded); fail loudly with the
# app log otherwise.
for _ in $(seq 1 100); do
    [ -S "$sock" ] && [ -s "$sock.token" ] && break
    if ! kill -0 "$pid" 2>/dev/null; then
        echo "verify-control: app exited before listening:" >&2
        cat "$log" >&2
        exit 1
    fi
    sleep 0.1
done
if [ ! -S "$sock" ] || [ ! -s "$sock.token" ]; then
    echo "verify-control: socket never appeared:" >&2
    cat "$log" >&2
    exit 1
fi

python3 - "$sock" <<'PY'
import base64
import json
import socket
import sys

sock_path = sys.argv[1]
with open(sock_path + ".token") as token_file:
    token = token_file.read().strip()
conn = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
conn.settimeout(20.0)
conn.connect(sock_path)
stream = conn.makefile("rwb")
next_id = 0


def fail(message):
    print(f"verify-control: FAIL {message}", file=sys.stderr)
    sys.exit(1)


def call(method, **params):
    global next_id
    next_id += 1
    request = {"id": next_id, "method": method, "params": params, "token": token}
    stream.write((json.dumps(request) + "\n").encode())
    stream.flush()
    line = stream.readline()
    if not line:
        fail(f"{method}: no response")
    response = json.loads(line)
    if not response.get("ok"):
        fail(f"{method}: {response.get('error')}")
    return response.get("result", {})


# 1. Exactly the fixture, 8x8.
status = call("status")
if status.get("documents") != 1:
    fail(f"expected 1 document, got {status.get('documents')}")
info = status["documents_info"][0]
if (info.get("width"), info.get("height")) != (8, 8):
    fail(f"expected 8x8, got {info.get('width')}x{info.get('height')}")

# 2. A rectangle over the left half of the active layer (the Blue layer's rect
#    is 4,4..8,8), so the selection is a strict subset of the layer: a filter
#    that ignored the mask would change pixels outside it.
selection = call("selection", op="rect", x=4, y=4, w=2, h=4)
if not selection.get("has_selection") or selection.get("count") != 8:
    fail(f"rect selection has={selection.get('has_selection')} count={selection.get('count')}")


def pixels():
    return {(x, y): call("get_pixel", x=x, y=y)["argb"] for y in range(8) for x in range(8)}


# 3. The filter must change something, and only inside the selection.
before = pixels()
call("filter", kind="add-noise")
after = pixels()
changed = {p for p in before if before[p] != after[p]}
if not changed:
    fail("filter changed no pixel")
outside = sorted(p for p in changed if not (4 <= p[0] < 6 and 4 <= p[1] < 8))
if outside:
    fail(f"filter changed pixels outside the selection: {outside}")

# 4. Undo restores every pixel.
call("edit", op="undo")
restored = pixels()
if restored != before:
    differing = sorted(p for p in before if before[p] != restored[p])
    fail(f"undo did not restore {differing}")

# 5. A document screenshot is a PNG at the source size.
shot = call("screenshot", scope="document")
if shot.get("mime") != "image/png":
    fail(f"screenshot mime is {shot.get('mime')}")
if (shot.get("source_width"), shot.get("source_height")) != (8, 8):
    fail(f"screenshot source is {shot.get('source_width')}x{shot.get('source_height')}")
png = base64.b64decode(shot["base64"])
if png[:8] != b"\x89PNG\r\n\x1a\n":
    fail("screenshot bytes are not a PNG")

print("verify-control: recipe OK")
PY

PICTURA_CONTROL_SOCKET="$sock" python3 - "$BIN" <<'PY'
import json
import os
import subprocess
import sys
import tempfile

binary = sys.argv[1]


def fail(message):
    print(f"verify-control: FAIL {message}", file=sys.stderr)
    sys.exit(1)


class Bridge:
    def __init__(self, *args, env=None):
        self.process = subprocess.Popen(
            [binary, "--mcp", *args], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
            text=True, env=env)
        self.next_id = 0

    def request(self, method, **params):
        self.next_id += 1
        message = {"jsonrpc": "2.0", "id": self.next_id, "method": method, "params": params}
        self.process.stdin.write(json.dumps(message) + "\n")
        self.process.stdin.flush()
        line = self.process.stdout.readline()
        if not line:
            fail(f"{method}: the bridge closed")
        return json.loads(line)

    def tool(self, name, **arguments):
        return self.request("tools/call", name=name, arguments=arguments)["result"]

    def close(self):
        self.process.stdin.close()
        if self.process.wait(timeout=20) != 0:
            fail(f"the bridge exited {self.process.returncode}")


# Attach to the instance the recipe above used.
attached = Bridge()
if "tools" not in attached.request("initialize", protocolVersion="2025-06-18")["result"]["capabilities"]:
    fail("initialize advertises no tools")
if len(attached.request("tools/list")["result"]["tools"]) < 20:
    fail("too few tools")
status = attached.tool("pictura_status")
if status.get("isError") or "documents" not in status["content"][0]["text"]:
    fail(f"attached status: {status}")
shot = attached.tool("pictura_screenshot", scope="document")
if shot["content"][0]["type"] != "image":
    fail("a screenshot is not image content")
attached.close()

# A private instance: restricted, and gone when the bridge goes.
with tempfile.TemporaryDirectory() as scratch:
    env = dict(os.environ, TMPDIR=scratch)
    env.pop("PICTURA_CONTROL_SOCKET")
    root = os.path.join(scratch, "root")
    os.mkdir(root)
    spawned = Bridge("--spawn", "--tier", "save", "--root", root, env=env)
    made = spawned.tool("pictura_document", op="new", width=8, height=8)
    if made.get("isError"):
        fail(f"spawned new: {made}")
    target = os.path.join(root, "out.png")
    if spawned.tool("pictura_export_png", path=target).get("isError") or not os.path.exists(target):
        fail("spawned export")
    outside = spawned.tool("pictura_export_png", path=os.path.join(scratch, "out.png"))
    if not outside.get("isError") or "refused" not in outside["content"][0]["text"]:
        fail(f"a write outside the root was not refused: {outside}")
    spawned.close()
    left = [name for name in os.listdir(scratch) if name.startswith("pictura-mcp-")]
    if left:
        fail(f"the bridge left {left}")

    readonly = Bridge("--spawn", env=env)
    denied = readonly.tool("pictura_document", op="new", width=8, height=8)
    if not denied.get("isError"):
        fail("the default tier allowed an edit")
    readonly.close()

print("verify-control: bridge OK")
PY

echo "verify-control: OK"
