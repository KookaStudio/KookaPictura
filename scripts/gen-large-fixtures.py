#!/usr/bin/env python3
"""gen-large-fixtures.py - write the memory benchmark's large PSD (issue #1210).

The M12 scenario opens an 8-bit RGB PSD with a white Background and four
densely painted layers. The file is generated, never committed (CONTRIBUTING
asset rules), and the bytes depend only on --size and --seed:

    python3 scripts/gen-large-fixtures.py --size 17000 --out /tmp/m12.psd
    python3 scripts/gen-large-fixtures.py --self-check

Each painted layer is a seeded set of opaque discs over transparency, so every
row is a short list of colour runs. The rows are PackBits (RLE) encoded straight
from the runs: a 17000² file stays far below classic PSD's 32-bit section
lengths, and pure Python writes it in minutes rather than hours. The merged
image is the exact composite of the same runs. Standard library only.
"""

import argparse
import hashlib
import io
import os
import random
import struct
import sys

LAYERS = 4
DISCS_PER_LAYER = 60
WHITE = (255, 255, 255)


def discs(rng, w, h):
    """One layer's discs, bottom first: (cx, cy, r, (r, g, b))."""
    side = min(w, h)
    out = []
    for _ in range(DISCS_PER_LAYER):
        r = side * (1 / 40 + rng.random() * (1 / 12 - 1 / 40))
        cx, cy = rng.random() * w, rng.random() * h
        colour = tuple(int(rng.random() * 256) for _ in range(3))
        out.append((cx, cy, r, colour))
    return out


def row_runs(layer, y, w):
    """The row as runs [(x0, x1, colour or None)] covering 0..w, later discs on top."""
    spans = []
    yc = y + 0.5
    for cx, cy, r, colour in layer:
        dy = yc - cy
        if abs(dy) >= r:
            continue
        half = (r * r - dy * dy) ** 0.5
        x0, x1 = max(0, int(cx - half + 0.5)), min(w, int(cx + half + 0.5))
        if x0 < x1:
            spans.append((x0, x1, colour))
    return paint(spans, w)


def paint(spans, w):
    """Flatten `spans` (later wins) into disjoint runs over 0..w; None is empty."""
    cuts = sorted({0, w, *(s[0] for s in spans), *(s[1] for s in spans)})
    runs = []
    for a, b in zip(cuts, cuts[1:]):
        colour = None
        for x0, x1, c in reversed(spans):
            if x0 <= a and b <= x1:
                colour = c
                break
        if runs and runs[-1][2] == colour:
            runs[-1] = (runs[-1][0], b, colour)
        else:
            runs.append((a, b, colour))
    return runs


def packbits_runs(runs):
    """PackBits for a row given as [(length, byte)]."""
    out = bytearray()
    for n, value in runs:
        while n > 0:
            k = min(n, 128)
            out.append(0 if k == 1 else (1 - k) & 0xFF)
            out.append(value)
            n -= k
    return bytes(out)


def channel_rows(runs, channel):
    """One channel's (length, byte) runs; channel 3 is the alpha."""
    if channel == 3:
        return [(b - a, 0 if c is None else 255) for a, b, c in runs]
    return [(b - a, 0 if c is None else c[channel]) for a, b, c in runs]


def merge_runs(lower, upper):
    """`upper`'s opaque runs over `lower`'s; both are disjoint runs covering the row."""
    out = []
    i = j = x = 0
    w = lower[-1][1]
    while x < w:
        while lower[i][1] <= x:
            i += 1
        while upper[j][1] <= x:
            j += 1
        end = min(lower[i][1], upper[j][1])
        colour = upper[j][2] if upper[j][2] is not None else lower[i][2]
        if out and out[-1][2] == colour:
            out[-1] = (out[-1][0], end, colour)
        else:
            out.append((x, end, colour))
        x = end
    return out


def rle_channel(rows):
    """A channel's compression tag, row byte counts and data, from packed rows."""
    counts = b"".join(struct.pack(">H", len(r)) for r in rows)
    return counts, b"".join(rows)


def pascal4(name):
    raw = name.encode("ascii")
    s = bytes([len(raw)]) + raw
    return s + b"\0" * (-len(s) % 4)


def layer_record(name, w, h, channel_ids, lengths):
    rec = struct.pack(">iiiiH", 0, 0, h, w, len(channel_ids))
    for cid, length in zip(channel_ids, lengths):
        rec += struct.pack(">hI", cid, length)
    extra = struct.pack(">II", 0, 0) + pascal4(name)
    rec += b"8BIMnorm" + bytes([255, 0, 0, 0]) + struct.pack(">I", len(extra)) + extra
    return rec


def write_psd(out, w, h, seed):
    rng = random.Random(seed)
    painted = [discs(rng, w, h) for _ in range(LAYERS)]
    white_row = packbits_runs([(w, 255)])

    # Per layer and channel, the packed rows. The Background is white RGB.
    records = []
    data = []
    bg = [white_row] * h
    bg_channels = [rle_channel(bg) for _ in range(3)]
    bg_lengths = [2 + len(c) + len(d) for c, d in bg_channels]
    records.append(layer_record("Background", w, h, [0, 1, 2], bg_lengths))
    data.append(bg_channels)

    merged = [[(0, w, WHITE)] for _ in range(h)]
    for index, layer in enumerate(painted):
        rows = [[] for _ in range(4)]
        for y in range(h):
            runs = row_runs(layer, y, w)
            for c in range(4):
                rows[c].append(packbits_runs(channel_rows(runs, c)))
            merged[y] = merge_runs(merged[y], runs)
        # PSD stores the alpha (-1) first.
        channels = [rle_channel(rows[3])] + [rle_channel(rows[c]) for c in range(3)]
        lengths = [2 + len(c) + len(d) for c, d in channels]
        records.append(layer_record(f"Paint {index + 1}", w, h, [-1, 0, 1, 2], lengths))
        data.append(channels)

    info = io.BytesIO()
    info.write(struct.pack(">h", len(records)))
    for rec in records:
        info.write(rec)
    for channels in data:
        for counts, payload in channels:
            info.write(struct.pack(">H", 1))
            info.write(counts)
            info.write(payload)
    layer_info = info.getvalue()
    layer_info += b"\0" * (len(layer_info) % 2)

    out.write(b"8BPS" + struct.pack(">H", 1) + b"\0" * 6)
    out.write(struct.pack(">HIIHH", 3, h, w, 8, 3))
    out.write(struct.pack(">I", 0))  # colour mode data
    out.write(struct.pack(">I", 0))  # image resources
    out.write(struct.pack(">I", 4 + len(layer_info) + 4))
    out.write(struct.pack(">I", len(layer_info)))
    out.write(layer_info)
    out.write(struct.pack(">I", 0))  # global layer mask info

    # Merged image: one compression tag, every channel's row counts, then data.
    planes = [[packbits_runs(channel_rows(runs, c)) for runs in merged] for c in range(3)]
    out.write(struct.pack(">H", 1))
    for rows in planes:
        out.write(b"".join(struct.pack(">H", len(r)) for r in rows))
    for rows in planes:
        out.write(b"".join(rows))


def unpackbits(data, width):
    out = bytearray()
    i = 0
    while len(out) < width:
        n = data[i]
        i += 1
        if n < 128:
            out += data[i:i + n + 1]
            i += n + 1
        elif n > 128:
            out += bytes([data[i]]) * (257 - n)
            i += 1
    return bytes(out)


def parse(blob):
    """Check the PSD structure; return (w, h, layer count)."""
    assert blob[:4] == b"8BPS", "signature"
    version, = struct.unpack_from(">H", blob, 4)
    channels, h, w, depth, mode = struct.unpack_from(">HIIHH", blob, 12)
    assert (version, channels, depth, mode) == (1, 3, 8, 3), "header"
    pos = 26
    for _ in range(2):
        length, = struct.unpack_from(">I", blob, pos)
        pos += 4 + length
    pos += 4  # layer and mask section length
    info_len, = struct.unpack_from(">I", blob, pos)
    pos += 4
    end = pos + info_len
    count, = struct.unpack_from(">h", blob, pos)
    pos += 2
    layers = []
    for _ in range(count):
        top, left, bottom, right, n = struct.unpack_from(">iiiiH", blob, pos)
        pos += 18
        chans = [struct.unpack_from(">hI", blob, pos + 6 * i) for i in range(n)]
        pos += 6 * n + 12
        extra, = struct.unpack_from(">I", blob, pos)
        pos += 4 + extra
        layers.append((bottom - top, right - left, chans))
    for lh, lw, chans in layers:
        for _, length in chans:
            compression, = struct.unpack_from(">H", blob, pos)
            assert compression == 1, "RLE"
            counts = struct.unpack_from(f">{lh}H", blob, pos + 2)
            assert 2 + 2 * lh + sum(counts) == length, "row counts match the data"
            first = blob[pos + 2 + 2 * lh:pos + 2 + 2 * lh + counts[0]]
            assert len(unpackbits(first, lw)) == lw, "a row decodes to its width"
            pos += length
    assert pos <= end
    return w, h, count


def self_check():
    blobs = []
    for _ in range(2):
        out = io.BytesIO()
        write_psd(out, 64, 64, 1210)
        blobs.append(out.getvalue())
    assert hashlib.sha256(blobs[0]).digest() == hashlib.sha256(blobs[1]).digest(), "deterministic"
    assert parse(blobs[0]) == (64, 64, LAYERS + 1)
    other = io.BytesIO()
    write_psd(other, 64, 64, 7)
    assert other.getvalue() != blobs[0], "the seed matters"
    assert packbits_runs([(130, 9), (1, 4)]) == bytes([129, 9, 255, 9, 0, 4])
    assert unpackbits(packbits_runs([(130, 9), (1, 4)]), 131) == bytes([9] * 130 + [4])
    print("gen-large-fixtures: self-check OK")


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--size", default="17000", help="side, or WxH (default 17000)")
    parser.add_argument("--seed", type=int, default=1210)
    parser.add_argument("--out", help="the PSD to write")
    parser.add_argument("--self-check", action="store_true", help="check determinism and the format, then exit")
    args = parser.parse_args()
    if args.self_check:
        self_check()
        return 0
    if not args.out:
        parser.error("--out is required")
    w, _, h = args.size.lower().partition("x")
    w = int(w)
    h = int(h) if h else w
    tmp = args.out + ".part"
    with open(tmp, "wb") as out:
        write_psd(out, w, h, args.seed)
    os.replace(tmp, args.out)
    print(f"gen-large-fixtures: wrote {args.out} ({w}x{h}, seed {args.seed})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
