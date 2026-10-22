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

echo "verify-control: OK"
