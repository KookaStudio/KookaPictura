#!/usr/bin/env python3
"""profile-tools.py - time tools end to end in the real app (issue #719).

Launches `pictura --headless --control` on a throwaway socket, opens a document
(an image file, or a new one of --size), and drives each scenario's gesture
through the control server the way a user's mouse would: a `pointer` `down`, one
`move` request per motion event, and an `up`. Every request is timed on the
client, so a move's time is the GUI thread's work for that event plus whatever
it deferred (the frame flush, the repaint) before it read the next request. A
trailing `status` request catches work deferred past the release.

Each scenario then undoes its own change, so every scenario starts from the same
document. Results print as a table; --save writes them as JSON and --compare
prints a before/after table against an earlier --save:

    python3 scripts/profile-tools.py --size 4000x3000 --save /tmp/before.json
    # ...change the code, rebuild...
    python3 scripts/profile-tools.py --size 4000x3000 --compare /tmp/before.json

--timing-log keeps the app's PICTURA_PAINT_TIMING per-phase breakdown. --metrics
runs the app with PICTURA_METRICS=1, resets the metrics before each scenario,
and prints the scenario's per-phase totals (including Qt-side phases such as
the Layers panel refresh) from the control server's `metrics` snapshot.

The new-layer, new-group, duplicate, move-copy and delete-copy scenarios use
`layer_op` instead of driving a tool. rss_mb is the app's resident size after
the scenario.
Numbers are for one machine and one build; compare runs on the same machine.
No dependency beyond python3's standard library.

Memory (#1210): --peak resets the app's peak resident size before each scenario
(5 -> /proc/<pid>/clear_refs) and reports peak_mb (VmHWM), peak_added_mb and
rss_mb. The memory scenarios M1-M14 (scripts/memory_scenarios.py; --scenarios
memory for all) default to 17000x17000 and run each in a fresh app, with their
prerequisites run before the reset. --memory-max 4G runs the app under
`systemd-run --user --scope -p MemoryMax=4G -p MemorySwapMax=0`; a SIGKILL is the
row's `oom` outcome, not a harness failure. A limit above the memory available
now less 1.5 GiB is refused without --force: to make room for the app the kernel
would swap out everything else, which can freeze a desktop. Memory scenarios
without --memory-max run under that default limit; --force lifts both.

    python3 scripts/profile-tools.py --scenarios memory --memory-max 4G --save /tmp/4g.json
    python3 scripts/profile-tools.py --scenarios M5 --compare docs/dev/benchmarks/memory-baseline.json --compare-run 4G

--self-check runs the harness's own checks and every M scenario at 512x512.
"""

import argparse
import json
import math
import os
import shutil
import socket
import statistics
import subprocess
import sys
import tempfile
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import memory_scenarios  # noqa: E402

# name -> (tool, tool_options patch, gesture). Sizes are document pixels; a
# gesture's geometry is in fractions of the document, so a scenario means the
# same thing at every document size.
SCENARIOS = {
    "brush-40": ("brush", {"brush": {"size": 40, "hardness": 100, "opacity": 100, "flow": 100}}, "stroke"),
    "brush-500": ("brush", {"brush": {"size": 500, "hardness": 0, "opacity": 100, "flow": 100}}, "stroke"),
    "brush-1940": ("brush", {"brush": {"size": 1940, "hardness": 0, "opacity": 100, "flow": 100}}, "stroke"),
    "pencil-1940": ("pencil", {"brush": {"size": 1940, "opacity": 100}}, "stroke"),
    "eraser-1940": ("eraser", {"brush": {"size": 1940, "hardness": 0, "opacity": 100, "flow": 100}}, "stroke"),
    "dodge-40": ("dodge", {"brush": {"size": 40, "hardness": 100}}, "stroke"),
    "dodge-500": ("dodge", {"brush": {"size": 500, "hardness": 0}}, "stroke"),
    "dodge-1940": ("dodge", {"brush": {"size": 1940, "hardness": 0}}, "stroke"),
    "sponge-1940": ("sponge", {"brush": {"size": 1940, "hardness": 0}}, "stroke"),
    "rectangle-shape": ("rectangle", {"shape": {"mode": 0, "geometry": 0}}, "box"),
    "ellipse-shape": ("ellipse", {"shape": {"mode": 0, "geometry": 0}}, "box"),
    "rectangle-pixels": ("rectangle", {"shape": {"mode": 2, "geometry": 0}}, "box"),
    "marquee": ("marquee", {}, "box"),
    "gradient": ("gradient", {}, "line"),
    "paintbucket": ("paintbucket", {}, "click"),
    # The rest of the toolbox (#1185), one scenario per tool. A fourth element
    # adds `setup` (calls made before timing, each undone after it when it
    # leaves history), `mods` (held for the gesture), `finish` (a key sequence
    # timed after the release, e.g. a Crop commit) and `undo: False` for tools
    # that leave no history.
    "lasso": ("lasso", {}, "stroke"),
    "lasso-feather": ("lasso", {"selection": {"feather": 20}}, "stroke"),
    "lasso-add": ("lasso", {}, "stroke", {"setup": ["selection"], "mods": "shift"}),
    "lasso-subtract": ("lasso", {}, "stroke", {"setup": ["selection"], "mods": "alt"}),
    "polygonal-lasso": ("polygonallasso", {}, "clicks"),
    "magnetic-lasso": ("magneticlasso", {}, "hover"),
    "magic-wand": ("magicwand", {}, "click"),
    "quick-selection": ("quickselection", {}, "stroke"),
    "crop": ("crop", {}, "box", {"finish": "enter"}),
    "eyedropper": ("eyedropper", {}, "click", {"undo": False}),
    "color-sampler": ("colorsampler", {}, "click"),
    "ruler": ("ruler", {}, "line", {"undo": False}),
    "count": ("count", {}, "click"),
    "spot-healing": ("spothealingbrush", {"brush": {"size": 200}}, "stroke"),
    "healing": ("healingbrush", {"brush": {"size": 200}}, "stroke", {"setup": ["source"]}),
    "patch": ("patch", {}, "patch", {"setup": ["selection"]}),
    "red-eye": ("redeye", {}, "box"),
    "color-replacement": ("colorreplacement", {"brush": {"size": 200}}, "stroke"),
    "mixer-brush": ("mixerbrush", {"brush": {"size": 200}}, "stroke"),
    "clone-stamp": ("clonestamp", {"brush": {"size": 200}}, "stroke", {"setup": ["source"]}),
    "pattern-stamp": ("patternstamp", {"brush": {"size": 200}}, "stroke"),
    "history-brush": ("historybrush", {"brush": {"size": 200}}, "stroke"),
    "art-history": ("arthistorybrush", {"brush": {"size": 200}}, "stroke"),
    "background-eraser": ("backgrounderaser", {"brush": {"size": 200}}, "stroke"),
    "magic-eraser": ("magiceraser", {}, "click"),
    "blur": ("blur", {"brush": {"size": 200}}, "stroke"),
    "sharpen": ("sharpen", {"brush": {"size": 200}}, "stroke"),
    "smudge": ("smudge", {"brush": {"size": 200}}, "stroke"),
    "burn": ("burn", {"brush": {"size": 200}}, "stroke"),
    "pen": ("pen", {}, "clicks"),
    "freeform-pen": ("freeformpen", {}, "stroke"),
    "type": ("horizontaltype", {}, "click", {"finish": "escape"}),
    "rounded-rect-shape": ("roundedrectangle", {"shape": {"mode": 0, "geometry": 0}}, "box"),
    "polygon-shape": ("polygon", {"shape": {"mode": 0, "geometry": 0}}, "box"),
    "line-shape": ("line", {"shape": {"mode": 0, "geometry": 0}}, "line"),
    "custom-shape": ("customshape", {"shape": {"mode": 0, "geometry": 0}}, "box"),
    "rectangle-path": ("rectangle", {"shape": {"mode": 1, "geometry": 0}}, "box"),
    "ellipse-pixels": ("ellipse", {"shape": {"mode": 2, "geometry": 0}}, "box"),
    "shape-move": ("move", {}, "drag", {"setup": ["shape"]}),
    "shape-stroked-move": ("move", {}, "drag", {"setup": ["stroked-shape"]}),
    "shape-path-drag": ("pathselection", {}, "drag", {"setup": ["shape"]}),
    "shape-direct-drag": ("directselection", {}, "drag", {"setup": ["shape"]}),
    "hand": ("hand", {}, "line", {"undo": False}),
    "zoom": ("zoom", {}, "click", {"undo": False}),
}
# name -> (layer_op op, its params, setup duplicates of the bottom layer).
LAYER_SCENARIOS = {
    "new-layer": ("add", {"index": 0}, 0),
    "new-group": ("add_group", {"index": 0}, 0),
    "duplicate": ("duplicate", {"index": 0}, 0),
    "duplicate-copy": ("duplicate", {"index": 1}, 1),
    "move-copy": ("translate", {"index": 1, "dx": 40, "dy": 40}, 1),
    "delete-copy": ("delete", {"index": 1}, 1),
}
# name -> (type_op op, its params, type_op add params), on a type layer added
# first. Sizes are multiples of the document's short side / 56, so the layer has
# the same share of the document at every size. Undo reports what undoing the
# edit costs, which for a size change crosses a history segment.
TYPE_SCENARIOS = {
    "type-size": ("set", {"size": 1.2}, {}),
    "type-font": ("set", {"font": "Liberation Serif"}, {}),
    "type-align": ("set", {"justify": 2}, {}),
    "type-color": ("set", {"color": 0xFFC02030}, {}),
    "type-paragraph": ("set", {"space_before": 12.0}, {}),
    "type-color-preview": ("preview", {"color": 0xFFC02030}, {}),
    "type-warp-preview": ("warp_preview", {"style": "arc", "bend": 50.0}, {}),
    "type-huge-box": ("set", {"size": 1.2}, {"box": True}),
}
ALL_SCENARIOS = [*SCENARIOS, *LAYER_SCENARIOS, *TYPE_SCENARIOS]
DEFAULT_SCENARIOS = ",".join(ALL_SCENARIOS)


class AppError(RuntimeError):
    """The app answered a request with an error: it is alive and declined."""


class Client:
    def __init__(self, path, timeout):
        self.conn = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.conn.settimeout(timeout)
        self.conn.connect(path)
        self.stream = self.conn.makefile("rwb")
        self.next_id = 0
        with open(path + ".token") as token_file:
            self.token = token_file.read().strip()

    def call(self, method, **params):
        """Return (result, seconds); raise on an error response."""
        self.next_id += 1
        request = {"id": self.next_id, "method": method, "params": params, "token": self.token}
        line = json.dumps(request) + "\n"
        start = time.perf_counter()
        self.stream.write(line.encode())
        self.stream.flush()
        reply = self.stream.readline()
        elapsed = time.perf_counter() - start
        if not reply:
            raise RuntimeError(f"{method}: no response (did the app crash?)")
        response = json.loads(reply)
        if not response.get("ok"):
            raise AppError(f"{method}: {response.get('error')}")
        return response.get("result", {}), elapsed


def ms(seconds):
    return round(seconds * 1000.0, 2)


def stroke_points(w, h, moves):
    """A zigzag across the middle of the document: long enough that a large
    brush lays many dabs, turning so the dirty region is not one rectangle."""
    points = []
    for i in range(moves + 1):
        t = i / moves
        wave = abs((t * 3.0) % 2.0 - 1.0)  # triangle wave, three legs
        points.append((round(w * (0.15 + 0.7 * t), 1), round(h * (0.35 + 0.3 * wave), 1)))
    return points


def ring_points(w, h, sides=8):
    """Vertices of a polygon inscribed in the box gesture's ellipse."""
    cx, cy, rx, ry = w * 0.45, h * 0.425, w * 0.25, h * 0.225
    return [(round(cx + rx * math.cos(2 * math.pi * i / sides), 1),
             round(cy + ry * math.sin(2 * math.pi * i / sides), 1)) for i in range(sides)]


def hover_points(w, h, moves):
    """The ring walked edge by edge: about `moves` hover events in all."""
    ring = ring_points(w, h)
    per_side = max(1, moves // len(ring))
    points = []
    for i, (x0, y0) in enumerate(ring):
        x1, y1 = ring[(i + 1) % len(ring)]
        points += [(round(x0 + (x1 - x0) * k / per_side, 1), round(y0 + (y1 - y0) * k / per_side, 1))
                   for k in range(1, per_side + 1)]
    return points


def gesture_points(gesture, w, h, moves):
    if gesture == "stroke":
        return stroke_points(w, h, moves)
    if gesture == "drag":
        # From inside the box gesture's shape, a fifth of the document right.
        x0, y0, x1, y1 = w * 0.45, h * 0.425, w * 0.65, h * 0.5
    elif gesture == "patch":
        x0, y0, x1, y1 = w * 0.4, h * 0.4, w * 0.6, h * 0.45
    elif gesture == "box":
        x0, y0, x1, y1 = w * 0.2, h * 0.2, w * 0.7, h * 0.65
    elif gesture == "line":
        x0, y0, x1, y1 = w * 0.1, h * 0.5, w * 0.9, h * 0.5
    else:
        return [(w * 0.5, h * 0.5)]
    steps = max(2, min(moves, 16))
    return [(round(x0 + (x1 - x0) * i / steps, 1), round(y0 + (y1 - y0) * i / steps, 1)) for i in range(steps + 1)]


SELECTION_BOX = (0.3, 0.3, 0.35, 0.3)  # x, y, w, h in document fractions


def run_setup(client, step, w, h):
    """Run one setup step; return how many history states it added."""
    if step == "selection":
        x, y, sw, sh = SELECTION_BOX
        client.call("selection", op="rect", x=round(w * x), y=round(h * y), w=round(w * sw), h=round(h * sh))
        return 1
    if step == "source":
        client.call("pointer", op="click", space="image", x=w * 0.1, y=h * 0.1, modifiers="alt")
        return 0
    if step in ("shape", "stroked-shape"):
        shape = {"mode": 0, "geometry": 0, "strokeEnabled": step == "stroked-shape", "strokeWidth": 20}
        client.call("set_tool", tool="rectangle")
        client.call("tool_options", options={"shape": shape})
        (x0, y0), (x1, y1) = gesture_points("box", w, h, 2)[::2][:2]
        client.call("pointer", op="drag", space="image", x=x0, y=y0, x2=x1, y2=y1, steps=4)
        client.call("tool_options", options={"shape": {"strokeEnabled": False}})
        return 1
    raise RuntimeError(f"unknown setup step {step}")


def click(client, x, y, **extra):
    took = client.call("pointer", op="down", space="image", x=x, y=y, **extra)[1]
    return took + client.call("pointer", op="up", space="image", x=x, y=y, **extra)[1]


def run_gesture(client, gesture, w, h, moves, row, mods):
    """Drive the gesture; return the per-event times between press and release."""
    extra = {"modifiers": mods} if mods else {}
    times = []
    if gesture in ("clicks", "hover"):
        # A path of clicks closed on its first vertex (Polygonal Lasso, Pen);
        # the Magnetic Lasso hovers between them. down is the first click, up
        # the closing one.
        ring = ring_points(w, h)
        row["down_ms"] = click(client, *ring[0], **extra)
        if gesture == "clicks":
            times = [click(client, x, y) for x, y in ring[1:]]
        else:
            times = [client.call("pointer", op="move", space="image", x=x, y=y)[1]
                     for x, y in hover_points(w, h, moves)[:-1]]
        row["up_ms"] = click(client, *ring[0])
        return times
    points = gesture_points(gesture, w, h, moves)
    _, row["down_ms"] = client.call("pointer", op="down", space="image", x=points[0][0], y=points[0][1], **extra)
    for x, y in points[1:]:
        times.append(client.call("pointer", op="move", space="image", x=x, y=y, **extra)[1])
    last = points[-1]
    _, row["up_ms"] = client.call("pointer", op="up", space="image", x=last[0], y=last[1], **extra)
    return times


def run_scenario(client, name, w, h, moves, before_undo):
    tool, options, gesture, *rest = SCENARIOS[name]
    extra = rest[0] if rest else {}
    setup_states = sum(run_setup(client, step, w, h) for step in extra.get("setup", []))
    client.call("set_tool", tool=tool)
    if options:
        client.call("tool_options", options=options)
    client.call("status")  # settle the tool switch before timing
    row = {"tool": tool, "gesture": gesture}
    total = time.perf_counter()
    move_times = run_gesture(client, gesture, w, h, moves, row, extra.get("mods"))
    row["moves"] = len(move_times)
    if extra.get("finish"):
        _, row["finish_ms"] = client.call("key", sequence=extra["finish"])
        row["finish_ms"] = ms(row["finish_ms"])
    _, row["settle_ms"] = client.call("status")
    row["total_ms"] = time.perf_counter() - total
    row["down_ms"] = ms(row["down_ms"])
    row["up_ms"] = ms(row["up_ms"])
    row["settle_ms"] = ms(row["settle_ms"])
    row["total_ms"] = ms(row["total_ms"])
    if move_times:
        ordered = sorted(move_times)
        row["move_p50_ms"] = ms(statistics.median(ordered))
        row["move_p95_ms"] = ms(ordered[min(len(ordered) - 1, int(round(0.95 * (len(ordered) - 1))))])
        row["move_max_ms"] = ms(ordered[-1])
        row["move_sum_ms"] = ms(sum(ordered))
    before_undo(row)
    if extra.get("undo", True):
        _, undo = client.call("edit", op="undo")
        _, settle = client.call("status")
        row["undo_ms"] = ms(undo + settle)
    for _ in range(setup_states):
        client.call("edit", op="undo")
    client.call("status")
    return row


def run_layer_scenario(client, name, before_undo, metrics):
    op, params, setup = LAYER_SCENARIOS[name]
    for _ in range(setup):
        client.call("layer_op", op="duplicate", index=0)
    client.call("status")
    if metrics:
        client.call("metrics", reset=True)
    row = {"tool": "layer_op", "gesture": op}
    total = time.perf_counter()
    _, row["op_ms"] = client.call("layer_op", op=op, **params)
    _, row["settle_ms"] = client.call("status")
    row["total_ms"] = ms(time.perf_counter() - total)
    row["idle_ms"] = ms(wait_idle(client, total))
    row["op_ms"] = ms(row["op_ms"])
    row["settle_ms"] = ms(row["settle_ms"])
    before_undo(row)
    _, undo = client.call("edit", op="undo")
    _, settle = client.call("status")
    row["undo_ms"] = ms(undo + settle)
    for _ in range(setup):
        client.call("edit", op="undo")
    client.call("status")
    return row


def run_type_scenario(client, name, w, h, before_undo, metrics):
    op, params, setup = TYPE_SCENARIOS[name]
    base = max(24.0, min(w, h) / 56.0)
    add = {"op": "add", "size": base, "x": w * 0.35, "y": h * 0.5}
    if setup.get("box"):
        add.update(x=w * 0.02, y=h * 0.02, box=[0.0, 0.0, w * 0.96, h * 0.96])
    added, _ = client.call("type_op", **add)
    client.call("status")
    if metrics:
        client.call("metrics", reset=True)
    edit = {"path": added["path"], **params}
    if "size" in edit:
        edit["size"] = base * edit["size"]
    row = {"tool": "type_op", "gesture": op}
    total = time.perf_counter()
    _, row["op_ms"] = client.call("type_op", op=op, **edit)
    _, row["settle_ms"] = client.call("status")
    row["total_ms"] = ms(time.perf_counter() - total)
    row["idle_ms"] = ms(wait_idle(client, total))
    row["op_ms"] = ms(row["op_ms"])
    row["settle_ms"] = ms(row["settle_ms"])
    before_undo(row)
    if op == "set":
        _, undo = client.call("edit", op="undo")
        _, settle = client.call("status")
        row["undo_ms"] = ms(undo + settle)
    elif op == "warp_preview":
        _, undo = client.call("type_op", op="warp_cancel")
        row["undo_ms"] = ms(undo)
    client.call("edit", op="undo")
    client.call("status")
    return row


def status_kib(pid, key):
    with open(f"/proc/{pid}/status") as status:
        for line in status:
            if line.startswith(key + ":"):
                return int(line.split()[1])
    return 0


def rss_mb(pid):
    return status_kib(pid, "VmRSS") // 1024


def reset_peak(pid):
    """Reset VmHWM to the current VmRSS (Linux 4.0+)."""
    with open(f"/proc/{pid}/clear_refs", "w") as refs:
        refs.write("5")


def settle_workers(pid, window=0.5, tolerance_kib=1024, limit=120.0):
    """Wait until the resident size holds within `tolerance_kib` for `window`
    seconds: deferred composites and pyramid builds run on workers after the
    GUI thread is idle."""
    start = time.perf_counter()
    last, since = status_kib(pid, "VmRSS"), time.perf_counter()
    while time.perf_counter() - start < limit:
        time.sleep(0.05)
        now = status_kib(pid, "VmRSS")
        if abs(now - last) > tolerance_kib:
            last, since = now, time.perf_counter()
        elif time.perf_counter() - since >= window:
            return


def wait_idle(client, start, window=0.4, quiet_s=0.005):
    """Seconds from `start` until the GUI thread finished its deferred work.

    Polls `status` every 20 ms for at least `window` seconds (past the frame's
    120 ms panel-refresh debounce). A reply slower than `quiet_s` means the GUI
    thread was busy, so the idle point is the end of the last slow reply."""
    busy_until = time.perf_counter()
    quiet = 0
    while True:
        _, took = client.call("status")
        now = time.perf_counter()
        if took > quiet_s:
            busy_until = now
            quiet = 0
        else:
            quiet += 1
        if quiet >= 3 and now - start > window and now - busy_until > 0.1:
            return busy_until - start
        time.sleep(0.02)


def run_any(client, name, w, h, moves, metrics, pid, peak=False):
    # The snapshot is read before the scenario's undo, whose full restore
    # would otherwise add composite phases the scenario itself never ran.
    rss_before = status_kib(pid, "VmRSS")
    if peak:
        reset_peak(pid)

    def before_undo(row):
        row["rss_mb"] = rss_mb(pid)
        if peak:
            row["peak_mb"] = status_kib(pid, "VmHWM") // 1024
            row["peak_added_mb"] = row["peak_mb"] - rss_before // 1024
        if metrics:
            snapshot = client.call("metrics")[0]
            row["phases"] = {label: phase["total_ms"] for label, phase in snapshot.get("phases", {}).items()}

    if metrics:
        client.call("metrics", reset=True)
    if name in LAYER_SCENARIOS:
        return run_layer_scenario(client, name, before_undo, metrics)
    if name in TYPE_SCENARIOS:
        return run_type_scenario(client, name, w, h, before_undo, metrics)
    return run_scenario(client, name, w, h, moves, before_undo)


def print_phases(results):
    for name, row in results.items():
        phases = row.get("phases")
        if not phases:
            continue
        print(f"{name}:")
        for label, total in sorted(phases.items(), key=lambda item: -item[1]):
            print(f"    {label:<34}{total:>10.2f}")
    print("(per-phase totals in milliseconds, from PICTURA_METRICS)")


COLUMNS = ["op_ms", "idle_ms", "down_ms", "move_p50_ms", "move_p95_ms", "move_max_ms", "up_ms", "finish_ms", "settle_ms", "total_ms", "undo_ms", "rss_mb", "peak_mb"]


# --- Memory scenarios (#1210) -------------------------------------------------

MEMORY_COLUMNS = ["outcome", "peak_added_mb", "peak_mb", "rss_before_mb", "rss_mb", "total_ms"]
OOM_CODES = (-9, 137)  # SIGKILL from Popen, or through a shell


def app_command(app, sock, state, memory_max=None):
    """The app's command line, inside a memory-limited scope when asked."""
    cmd = [app, "--headless", "--control", "--control-socket", sock, "--state-home", state]
    if not memory_max:
        return cmd
    return ["systemd-run", "--user", "--scope", "--quiet",
            "-p", f"MemoryMax={memory_max}", "-p", "MemorySwapMax=0", "--", *cmd]


LIMIT_MARGIN_MIB = 1536  # what the rest of the desktop keeps beside a limited app


def size_mib(text):
    """A systemd size (`4G`, `6475M`, `512K`, bytes) in MiB."""
    units = {"K": 1 / 1024, "M": 1, "G": 1024, "T": 1024 * 1024}
    text = text.strip().upper()
    if text and text[-1] in units:
        return float(text[:-1]) * units[text[-1]]
    return float(text) / (1024 * 1024)


def available_mib():
    with open("/proc/meminfo") as meminfo:
        for line in meminfo:
            if line.startswith("MemAvailable:"):
                return int(line.split()[1]) // 1024
    return 0


def limit_problem(limit, available, margin=LIMIT_MARGIN_MIB):
    """Why `limit` would starve the machine, or None. With swap off inside the
    scope, the kernel makes room for the app by swapping everything else out:
    a limit above what is free now, less a margin, can freeze a desktop."""
    wanted = size_mib(limit)
    if wanted > available - margin:
        return (f"--memory-max {limit} ({wanted:.0f} MiB) exceeds the memory available now "
                f"({available} MiB) less {margin} MiB for the rest of the machine; close other "
                f"programs, lower the limit, or pass --force")
    return None


def default_limit(available, margin=LIMIT_MARGIN_MIB):
    """The limit a memory run gets without --memory-max: what is free less the margin."""
    return f"{max(256, available - margin)}M"


def outcome_of(returncode, phase, answered=False):
    """A failed scenario's outcome: `oom` for a SIGKILL; `refused` when the
    live app answered a step with an error (an open over the image budget, a
    gate); else `crash` (an exit, or no answer)."""
    if returncode in OOM_CODES:
        return {"outcome": "oom", "oom_phase": phase}
    if answered and returncode is None:
        return {"outcome": "refused", "phase": phase}
    return {"outcome": "crash", "phase": phase, "exit": returncode}


def launch(cmd, sock, env, log):
    """Start the app and wait for its control socket; return the process."""
    app = subprocess.Popen(cmd, stdout=log, stderr=subprocess.STDOUT, env=env)
    for _ in range(300):
        token = sock + ".token"
        if os.path.exists(sock) and os.path.exists(token) and os.path.getsize(token) > 0:
            return app
        if app.poll() is not None:
            raise RuntimeError(f"app exited ({app.returncode}) before listening; see {log.name}")
        time.sleep(0.1)
    app.kill()
    raise RuntimeError(f"socket never appeared; see {log.name}")


def stop(app):
    if app.poll() is None:
        app.terminate()
        try:
            app.wait(timeout=10)
        except subprocess.TimeoutExpired:
            app.kill()
            app.wait()


def fixture_path(directory, w, h):
    """The M12 PSD for this size, generated once into `directory`."""
    # The control server opens absolute paths only.
    path = os.path.abspath(os.path.join(directory, f"memory-fixture-{w}x{h}-seed1210.psd"))
    if not os.path.exists(path):
        generator = os.path.join(os.path.dirname(os.path.abspath(__file__)), "gen-large-fixtures.py")
        subprocess.run([sys.executable, generator, "--size", f"{w}x{h}", "--out", path],
                       check=True, stdout=subprocess.DEVNULL)
    return path


def settle(client, pid):
    client.call("status")
    wait_idle(client, time.perf_counter())
    settle_workers(pid)


def run_memory_scenario(binary, sid, w, h, memory_max, fixture_dir, timeout, env):
    """Run one M scenario in a fresh app; return its row."""
    gesture, setup, measured, needs_fixture = memory_scenarios.SCENARIOS[sid]
    row = {"gesture": gesture}
    tmp = tempfile.mkdtemp(prefix="pictura-memory-")
    sock = os.path.join(tmp, "control.sock")
    fixture = fixture_path(fixture_dir or tmp, w, h) if needs_fixture else None
    phase = "launch"
    app = None
    try:
        with open(os.path.join(tmp, "app.log"), "w") as log:
            app = launch(app_command(binary, sock, os.path.join(tmp, "state"), memory_max), sock, env, log)
        client = Client(sock, timeout)
        ctx = memory_scenarios.Ctx(client, w, h, tmp, fixture)
        phase = "setup"
        for step in setup:
            step(ctx)
        settle(client, app.pid)
        rss_before = status_kib(app.pid, "VmRSS")
        reset_peak(app.pid)
        phase = "scenario"
        start = time.perf_counter()
        for step in measured:
            step(ctx)
        settle(client, app.pid)
        peak = status_kib(app.pid, "VmHWM")
        row.update(outcome="ok", total_ms=ms(time.perf_counter() - start),
                   rss_before_mb=rss_before // 1024, peak_mb=peak // 1024,
                   peak_added_mb=(peak - rss_before) // 1024, rss_mb=rss_mb(app.pid))
    except (RuntimeError, OSError, ValueError) as err:
        returncode = None
        if app is not None:
            try:
                returncode = app.wait(timeout=5)
            except subprocess.TimeoutExpired:
                pass
        row.update(outcome_of(returncode, phase, isinstance(err, AppError)), error=str(err))
    finally:
        if app is not None:
            stop(app)
        shutil.rmtree(tmp, ignore_errors=True)
    return row


def machine_info():
    """The reference machine, for a saved baseline."""
    def first(cmd):
        try:
            return subprocess.run(cmd, capture_output=True, text=True, check=False).stdout.strip()
        except OSError:
            return ""
    # A big.LITTLE CPU lists one model per core type.
    cpu = " + ".join(line.split(":", 1)[1].strip() for line in first(["lscpu"]).splitlines()
                     if line.strip().startswith("Model name"))
    mem_kib = 0
    with open("/proc/meminfo") as meminfo:
        for line in meminfo:
            if line.startswith("MemTotal:"):
                mem_kib = int(line.split()[1])
    return {"cpu": cpu, "cores": os.cpu_count(), "mem_total_mib": mem_kib // 1024,
            "kernel": os.uname().release, "arch": os.uname().machine,
            "qt": first(["qmake6", "-query", "QT_VERSION"]),
            "commit": first(["git", "-C", os.path.dirname(os.path.abspath(__file__)), "rev-parse", "HEAD"])}


def print_memory_table(results):
    header = f"{'scenario':<9}" + "".join(f"{c.replace('_mb', ''):>15}" for c in MEMORY_COLUMNS)
    print(header)
    print("-" * len(header))
    for name, row in results.items():
        cells = "".join(f"{str(row.get(c, '')):>15}" for c in MEMORY_COLUMNS)
        note = f"  ({row.get('oom_phase') or row.get('phase')}: {row.get('error', '')})" if row.get("outcome") != "ok" else ""
        print(f"{name:<9}{cells}{note}")
    print("(MiB; peak_added is the peak above the resident size at the reset; total is milliseconds)")


def memory_compare_lines(before, after):
    """Per scenario: peak and outcome before -> after."""
    lines = [f"{'scenario':<9}{'outcome':>16}{'peak_mb':>26}{'peak_added_mb':>26}"]
    for name, row in after.items():
        base = before.get(name)
        if base is None:
            continue
        cells = [f"{base.get('outcome', '-')} -> {row.get('outcome', '-')}"]
        for key in ("peak_mb", "peak_added_mb"):
            b, a = base.get(key), row.get(key)
            if b is None or a is None:
                cells.append("")
                continue
            ratio = f"{b / a:.1f}x" if a > 0 else "-"
            cells.append(f"{b} -> {a} ({ratio})")
        lines.append(f"{name:<9}{cells[0]:>16}{cells[1]:>26}{cells[2]:>26}")
    lines.append("(before -> after, MiB; the ratio is the saving)")
    return lines


def load_results(path, run=None):
    """A --save file's results; a baseline holds several runs under `runs`."""
    with open(path) as f:
        data = json.load(f)
    if "runs" in data:
        if run not in data["runs"]:
            raise SystemExit(f"profile-tools: {path} has runs {', '.join(data['runs'])}; pick one with --compare-run")
        data = data["runs"][run]
    return data


# --- Self-check -----------------------------------------------------------------

def check_peak_row(name, row):
    assert row.get("outcome") == "ok", f"{name}: {row}"
    for key in ("peak_mb", "peak_added_mb", "rss_mb", "rss_before_mb", "total_ms"):
        assert isinstance(row.get(key), (int, float)), f"{name}: {key} missing in {row}"
    assert row["peak_mb"] >= row["rss_before_mb"], f"{name}: peak below the reset's RSS"
    assert row["peak_added_mb"] >= 0, f"{name}: negative peak added"


def check_limit_command():
    plain = app_command("app", "s", "st")
    assert plain == ["app", "--headless", "--control", "--control-socket", "s", "--state-home", "st"]
    limited = app_command("app", "s", "st", "4G")
    assert limited[:4] == ["systemd-run", "--user", "--scope", "--quiet"]
    assert "MemoryMax=4G" in limited and "MemorySwapMax=0" in limited
    assert limited[limited.index("--") + 1:] == plain


def check_limit_guard():
    assert size_mib("4G") == 4096 and size_mib("6475M") == 6475 and size_mib("512K") == 0.5
    assert limit_problem("4G", 6000) is None
    assert "exceeds the memory available" in limit_problem("4G", 5000)
    assert limit_problem("2G", 3600) is None
    assert default_limit(5000) == "3464M" and limit_problem(default_limit(5000), 5000) is None
    assert default_limit(1000) == "256M"


def check_outcomes():
    assert outcome_of(-9, "setup") == {"outcome": "oom", "oom_phase": "setup"}
    assert outcome_of(137, "scenario") == {"outcome": "oom", "oom_phase": "scenario"}
    assert outcome_of(1, "scenario")["outcome"] == "crash"
    assert outcome_of(None, "scenario")["outcome"] == "crash"
    assert outcome_of(None, "scenario", answered=True) == {"outcome": "refused", "phase": "scenario"}
    assert outcome_of(-9, "scenario", answered=True)["outcome"] == "oom"


def check_compare():
    before = {"M4": {"outcome": "ok", "peak_mb": 400, "peak_added_mb": 300}, "M5": {"outcome": "ok"}}
    after = {"M4": {"outcome": "ok", "peak_mb": 200, "peak_added_mb": 100}, "M5": {"outcome": "oom"}}
    lines = memory_compare_lines(before, after)
    assert "400 -> 200 (2.0x)" in lines[1] and "300 -> 100 (3.0x)" in lines[1], lines
    assert "ok -> oom" in lines[2], lines


def self_check_catalogue(binary, side=512):
    tmp = tempfile.mkdtemp(prefix="pictura-memory-check-")
    env = dict(os.environ)
    env.pop("PICTURA_METRICS", None)
    try:
        failed = []
        for sid in memory_scenarios.SCENARIOS:
            row = run_memory_scenario(binary, sid, side, side, None, tmp, 120.0, env)
            try:
                check_peak_row(sid, row)
                print(f"profile-tools: self-check {sid} ok (peak +{row['peak_added_mb']} MiB)")
            except AssertionError as err:
                failed.append(str(err))
        assert not failed, "\n".join(failed)
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


def self_check(binary):
    check_limit_command()
    check_limit_guard()
    check_outcomes()
    check_compare()
    print("profile-tools: self-check helpers ok")
    if not os.access(binary, os.X_OK):
        print(f"profile-tools: SKIP catalogue (no app at {binary})")
        return
    self_check_catalogue(binary)
    print("profile-tools: self-check OK")


def print_table(results):
    header = f"{'scenario':<20}" + "".join(f"{c.replace('_ms', ''):>11}" for c in COLUMNS)
    print(header)
    print("-" * len(header))
    for name, row in results.items():
        if "error" in row:
            print(f"{name:<20}  error: {row['error']}")
            continue
        print(f"{name:<20}" + "".join(f"{row.get(c, ''):>11}" for c in COLUMNS))
    print("(milliseconds; move_* are per motion event, total is down through settle)")


def print_compare(before, after):
    keys = ["op_ms", "idle_ms", "move_p95_ms", "move_max_ms", "up_ms", "total_ms", "undo_ms"]
    header = f"{'scenario':<20}" + "".join(f"{k.replace('_ms', ''):>26}" for k in keys)
    print(header)
    print("-" * len(header))
    for name, row in after.items():
        base = before.get(name)
        if base is None or "error" in row or "error" in base:
            continue
        cells = []
        for k in keys:
            b, a = base.get(k), row.get(k)
            if b is None or a is None:
                cells.append(f"{'':>26}")
                continue
            ratio = f"{b / a:.1f}x" if a > 0 else "-"
            cells.append(f"{f'{b:.1f} -> {a:.1f} ({ratio})':>26}")
        print(f"{name:<20}" + "".join(cells))
    print("(before -> after, milliseconds; the ratio is the speed-up)")


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--bin", default="./build/pictura")
    source = parser.add_mutually_exclusive_group()
    source.add_argument("--image", help="open this file as the document")
    source.add_argument("--size", help="new white RGB document WxH (default 4000x3000; 17000x17000 for M scenarios)")
    memory_ids = list(memory_scenarios.SCENARIOS)
    parser.add_argument("--scenarios", default=DEFAULT_SCENARIOS,
                        help=f"comma list, 'all' (the tools), or 'memory' (of: {', '.join(ALL_SCENARIOS + memory_ids)})")
    parser.add_argument("--moves", type=int, default=60, help="motion events per stroke (default 60)")
    parser.add_argument("--gpu", choices=["on", "off", "default"], default="default",
                        help="force GPU compute on/off for the document (default: the app's own)")
    parser.add_argument("--save", help="write the results as JSON")
    parser.add_argument("--compare", help="print a before/after table against this --save")
    parser.add_argument("--compare-run", help="the run to compare against in a multi-run baseline")
    parser.add_argument("--timing-log", help="keep the app's stderr with PICTURA_PAINT_TIMING=1")
    parser.add_argument("--metrics", action="store_true",
                        help="run the app with PICTURA_METRICS=1 and print each scenario's phases")
    parser.add_argument("--peak", action="store_true",
                        help="reset the peak resident size before each scenario and report it (always on for M scenarios)")
    parser.add_argument("--memory-max", help="run the app under a systemd MemoryMax (e.g. 4G), swap off")
    parser.add_argument("--force", action="store_true",
                        help="allow a --memory-max above the memory available now less 1.5 GiB")
    parser.add_argument("--fixture-dir", help="keep generated fixtures here between runs")
    parser.add_argument("--self-check", action="store_true", help="check the harness, then exit")
    parser.add_argument("--timeout", type=float, default=600.0, help="per-request timeout in seconds")
    args = parser.parse_args()

    if args.self_check:
        self_check(args.bin)
        return
    known = ALL_SCENARIOS + memory_ids
    if args.scenarios == "all":
        names = ALL_SCENARIOS
    elif args.scenarios == "memory":
        names = memory_ids
    else:
        names = [s.strip() for s in args.scenarios.split(",") if s.strip()]
    unknown = [n for n in names if n not in known]
    if unknown:
        parser.error(f"unknown scenario(s): {', '.join(unknown)}")
    memory = [n for n in names if n in memory_ids]
    if memory and len(memory) != len(names):
        parser.error("M scenarios run in their own apps; do not mix them with tool scenarios")
    if memory and args.image:
        parser.error("M scenarios make their own documents; drop --image")
    if not os.access(args.bin, os.X_OK):
        sys.exit(f"profile-tools: missing {args.bin} (build first: cmake --build build --parallel)")
    if memory and not args.memory_max and not args.force:
        # An unlimited run can take the machine down as surely as a limit too
        # large: by default a memory run keeps the margin free.
        args.memory_max = default_limit(available_mib())
        print(f"profile-tools: no --memory-max; limiting the app to {args.memory_max} "
              f"(available less {LIMIT_MARGIN_MIB} MiB; --force runs unlimited)")
    if args.memory_max and not args.force:
        problem = limit_problem(args.memory_max, available_mib())
        if problem:
            sys.exit(f"profile-tools: {problem}")

    env = dict(os.environ)
    if args.timing_log:
        env["PICTURA_PAINT_TIMING"] = "1"
    if args.metrics:
        env["PICTURA_METRICS"] = "1"
    else:
        env.pop("PICTURA_METRICS", None)
    if memory:
        w, h = (int(v) for v in (args.size or "17000x17000").lower().split("x"))
        meta = {"document": f"new {w}x{h}", "width": w, "height": h, "memory_max": args.memory_max,
                "machine": machine_info()}
        print(f"profile-tools: memory scenarios at {w}x{h}" + (f" under MemoryMax={args.memory_max}" if args.memory_max else ""))
        results = {}
        for name in memory:
            results[name] = run_memory_scenario(os.path.abspath(args.bin), name, w, h, args.memory_max,
                                                args.fixture_dir, args.timeout, env)
            print(f"profile-tools: {name} {results[name].get('outcome')}", flush=True)
        print_memory_table(results)
        finish(args, meta, results)
        return

    tmp = tempfile.mkdtemp(prefix="pictura-profile-")
    sock = os.path.join(tmp, "control.sock")
    log_path = args.timing_log or os.path.join(tmp, "app.log")
    app = None
    try:
        with open(log_path, "w") as log:
            try:
                app = launch(app_command(args.bin, sock, os.path.join(tmp, "state"), args.memory_max), sock, env, log)
            except RuntimeError as err:
                sys.exit(f"profile-tools: {err}")
        client = Client(sock, args.timeout)

        if args.image:
            _, open_s = client.call("document", op="open", path=os.path.abspath(args.image))
        else:
            w, h = (int(v) for v in (args.size or "4000x3000").lower().split("x"))
            _, open_s = client.call("document", op="new", width=w, height=h, background="white")
        _, settle_s = client.call("status")
        status = client.call("status")[0]
        info = status["documents_info"][status["active"]]
        w, h = info["width"], info["height"]
        meta = {"document": args.image or f"new {w}x{h}", "width": w, "height": h,
                "open_ms": ms(open_s + settle_s), "moves": args.moves}
        if args.gpu != "default":
            gpu = client.call("set_gpu_compute", on=args.gpu == "on")[0]
            meta["gpu_compute"], meta["backend"] = gpu.get("gpu_compute"), gpu.get("backend")
        print(f"profile-tools: {meta['document']} ({w}x{h}), opened in {meta['open_ms']} ms"
              + (f", gpu_compute={meta.get('gpu_compute')} ({meta.get('backend')})" if "backend" in meta else ""))

        results = {}
        for name in names:
            try:
                results[name] = run_any(client, name, w, h, args.moves, args.metrics, app.pid, args.peak)
            except (RuntimeError, OSError) as err:
                results[name] = {"error": str(err)}
                if app.poll() is not None:
                    results[name].update(outcome_of(app.returncode, "scenario"))
                    break
        print_table(results)
        if args.metrics:
            print()
            print_phases(results)
        finish(args, meta, results)
    finally:
        if app is not None:
            stop(app)
        if not args.timing_log:
            shutil.rmtree(tmp, ignore_errors=True)


def finish(args, meta, results):
    if args.save:
        with open(args.save, "w") as out:
            json.dump({"meta": meta, "results": results}, out, indent=2)
        print(f"profile-tools: saved {args.save}")
    if args.compare:
        before = load_results(args.compare, args.compare_run)
        print()
        print(f"compare against {args.compare} ({before['meta'].get('document')})")
        if any("outcome" in row for row in results.values()):
            print("\n".join(memory_compare_lines(before["results"], results)))
        else:
            print_compare(before["results"], results)


if __name__ == "__main__":
    main()
