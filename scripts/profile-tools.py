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

--timing-log keeps the app's PICTURA_PAINT_TIMING per-phase breakdown.
Numbers are for one machine and one build; compare runs on the same machine.
No dependency beyond python3's standard library.
"""

import argparse
import json
import os
import shutil
import socket
import statistics
import subprocess
import sys
import tempfile
import time

# name -> (tool, tool_options patch, gesture). Sizes are document pixels; a
# gesture's geometry is in fractions of the document, so a scenario means the
# same thing at every document size.
SCENARIOS = {
    "brush-40": ("brush", {"brush": {"size": 40, "hardness": 100, "opacity": 100, "flow": 100}}, "stroke"),
    "brush-500": ("brush", {"brush": {"size": 500, "hardness": 0, "opacity": 100, "flow": 100}}, "stroke"),
    "brush-1940": ("brush", {"brush": {"size": 1940, "hardness": 0, "opacity": 100, "flow": 100}}, "stroke"),
    "pencil-1940": ("pencil", {"brush": {"size": 1940, "opacity": 100}}, "stroke"),
    "eraser-1940": ("eraser", {"brush": {"size": 1940, "hardness": 0, "opacity": 100, "flow": 100}}, "stroke"),
    "rectangle-shape": ("rectangle", {"shape": {"mode": 0, "geometry": 0}}, "box"),
    "ellipse-shape": ("ellipse", {"shape": {"mode": 0, "geometry": 0}}, "box"),
    "rectangle-pixels": ("rectangle", {"shape": {"mode": 2, "geometry": 0}}, "box"),
    "marquee": ("marquee", {}, "box"),
    "gradient": ("gradient", {}, "line"),
    "paintbucket": ("paintbucket", {}, "click"),
}
DEFAULT_SCENARIOS = ",".join(SCENARIOS)


class Client:
    def __init__(self, path, timeout):
        self.conn = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.conn.settimeout(timeout)
        self.conn.connect(path)
        self.stream = self.conn.makefile("rwb")
        self.next_id = 0

    def call(self, method, **params):
        """Return (result, seconds); raise on an error response."""
        self.next_id += 1
        line = json.dumps({"id": self.next_id, "method": method, "params": params}) + "\n"
        start = time.perf_counter()
        self.stream.write(line.encode())
        self.stream.flush()
        reply = self.stream.readline()
        elapsed = time.perf_counter() - start
        if not reply:
            raise RuntimeError(f"{method}: no response (did the app crash?)")
        response = json.loads(reply)
        if not response.get("ok"):
            raise RuntimeError(f"{method}: {response.get('error')}")
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


def gesture_points(gesture, w, h, moves):
    if gesture == "stroke":
        return stroke_points(w, h, moves)
    if gesture == "box":
        x0, y0, x1, y1 = w * 0.2, h * 0.2, w * 0.7, h * 0.65
    elif gesture == "line":
        x0, y0, x1, y1 = w * 0.1, h * 0.5, w * 0.9, h * 0.5
    else:
        return [(w * 0.5, h * 0.5)]
    steps = max(2, min(moves, 16))
    return [(round(x0 + (x1 - x0) * i / steps, 1), round(y0 + (y1 - y0) * i / steps, 1)) for i in range(steps + 1)]


def run_scenario(client, name, w, h, moves):
    tool, options, gesture = SCENARIOS[name]
    client.call("set_tool", tool=tool)
    if options:
        client.call("tool_options", options=options)
    client.call("status")  # settle the tool switch before timing
    points = gesture_points(gesture, w, h, moves)
    row = {"tool": tool, "gesture": gesture, "moves": max(0, len(points) - 1)}
    total = time.perf_counter()
    _, row["down_ms"] = client.call("pointer", op="down", space="image", x=points[0][0], y=points[0][1])
    move_times = []
    for x, y in points[1:]:
        move_times.append(client.call("pointer", op="move", space="image", x=x, y=y)[1])
    last = points[-1]
    _, row["up_ms"] = client.call("pointer", op="up", space="image", x=last[0], y=last[1])
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
    _, undo = client.call("edit", op="undo")
    _, settle = client.call("status")
    row["undo_ms"] = ms(undo + settle)
    return row


COLUMNS = ["down_ms", "move_p50_ms", "move_p95_ms", "move_max_ms", "up_ms", "settle_ms", "total_ms", "undo_ms"]


def print_table(results):
    header = f"{'scenario':<18}" + "".join(f"{c.replace('_ms', ''):>11}" for c in COLUMNS)
    print(header)
    print("-" * len(header))
    for name, row in results.items():
        if "error" in row:
            print(f"{name:<18}  error: {row['error']}")
            continue
        print(f"{name:<18}" + "".join(f"{row.get(c, ''):>11}" for c in COLUMNS))
    print("(milliseconds; move_* are per motion event, total is down through settle)")


def print_compare(before, after):
    keys = ["move_p95_ms", "move_max_ms", "up_ms", "total_ms", "undo_ms"]
    header = f"{'scenario':<18}" + "".join(f"{k.replace('_ms', ''):>26}" for k in keys)
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
        print(f"{name:<18}" + "".join(cells))
    print("(before -> after, milliseconds; the ratio is the speed-up)")


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--bin", default="./build/pictura")
    source = parser.add_mutually_exclusive_group()
    source.add_argument("--image", help="open this file as the document")
    source.add_argument("--size", default="4000x3000", help="new white RGB document WxH (default 4000x3000)")
    parser.add_argument("--scenarios", default=DEFAULT_SCENARIOS,
                        help=f"comma list, or 'all' (of: {', '.join(SCENARIOS)})")
    parser.add_argument("--moves", type=int, default=60, help="motion events per stroke (default 60)")
    parser.add_argument("--gpu", choices=["on", "off", "default"], default="default",
                        help="force GPU compute on/off for the document (default: the app's own)")
    parser.add_argument("--save", help="write the results as JSON")
    parser.add_argument("--compare", help="print a before/after table against this --save")
    parser.add_argument("--timing-log", help="keep the app's stderr with PICTURA_PAINT_TIMING=1")
    parser.add_argument("--timeout", type=float, default=600.0, help="per-request timeout in seconds")
    args = parser.parse_args()

    names = list(SCENARIOS) if args.scenarios == "all" else [s.strip() for s in args.scenarios.split(",") if s.strip()]
    unknown = [n for n in names if n not in SCENARIOS]
    if unknown:
        parser.error(f"unknown scenario(s): {', '.join(unknown)}")
    if not os.access(args.bin, os.X_OK):
        sys.exit(f"profile-tools: missing {args.bin} (build first: cmake --build build --parallel)")

    tmp = tempfile.mkdtemp(prefix="pictura-profile-")
    sock = os.path.join(tmp, "control.sock")
    log_path = args.timing_log or os.path.join(tmp, "app.log")
    env = dict(os.environ)
    if args.timing_log:
        env["PICTURA_PAINT_TIMING"] = "1"
    with open(log_path, "w") as log:
        app = subprocess.Popen(
            [args.bin, "--headless", "--control", "--control-socket", sock, "--state-home", os.path.join(tmp, "state")],
            stdout=log, stderr=subprocess.STDOUT, env=env)
    try:
        for _ in range(300):
            if os.path.exists(sock):
                break
            if app.poll() is not None:
                sys.exit(f"profile-tools: app exited before listening; see {log_path}")
            time.sleep(0.1)
        else:
            sys.exit(f"profile-tools: socket never appeared; see {log_path}")
        client = Client(sock, args.timeout)

        if args.image:
            _, open_s = client.call("document", op="open", path=os.path.abspath(args.image))
        else:
            w, h = (int(v) for v in args.size.lower().split("x"))
            _, open_s = client.call("document", op="new", width=w, height=h, background="white")
        _, settle = client.call("status")
        status = client.call("status")[0]
        info = status["documents_info"][status["active"]]
        w, h = info["width"], info["height"]
        meta = {"document": args.image or f"new {w}x{h}", "width": w, "height": h,
                "open_ms": ms(open_s + settle), "moves": args.moves}
        if args.gpu != "default":
            gpu = client.call("set_gpu_compute", on=args.gpu == "on")[0]
            meta["gpu_compute"], meta["backend"] = gpu.get("gpu_compute"), gpu.get("backend")
        print(f"profile-tools: {meta['document']} ({w}x{h}), opened in {meta['open_ms']} ms"
              + (f", gpu_compute={meta.get('gpu_compute')} ({meta.get('backend')})" if "backend" in meta else ""))

        results = {}
        for name in names:
            try:
                results[name] = run_scenario(client, name, w, h, args.moves)
            except (RuntimeError, OSError) as err:
                results[name] = {"error": str(err)}
                if app.poll() is not None:
                    break
        print_table(results)
        if args.save:
            with open(args.save, "w") as out:
                json.dump({"meta": meta, "results": results}, out, indent=2)
            print(f"profile-tools: saved {args.save}")
        if args.compare:
            with open(args.compare) as f:
                before = json.load(f)
            print()
            print(f"compare against {args.compare} ({before['meta'].get('document')})")
            print_compare(before["results"], results)
    finally:
        if app.poll() is None:
            app.terminate()
            try:
                app.wait(timeout=10)
            except subprocess.TimeoutExpired:
                app.kill()
        if not args.timing_log:
            shutil.rmtree(tmp, ignore_errors=True)


if __name__ == "__main__":
    main()
