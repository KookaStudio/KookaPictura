#!/usr/bin/env python3
"""Fit colour matrices for Panasonic RW2 (#1542) and Canon CR3 (#1574) bodies
from their own camera JPEGs.

No openly licensed spectral sensitivities exist for most of these bodies, so by
owner decision each body's matrices are fitted from one sample raw file: the
3 x 3 map from white-balanced camera RGB to linear sRGB that best reproduces
the JPEG the camera rendered and embedded in the same file (an RW2's
`JpgFromRaw`, a CR3's full-size JPEG track). That JPEG carries the camera
maker's own colour rendering (its saturation and hue choices included), so the
result is approximate, not a colorimetric calibration. No Adobe, dcraw or
LibRaw matrix is used.

The matrix (rows constrained so camera white maps to sRGB white) is fitted by
least squares on the chromaticities of unclipped blocks, in linear light, so
exposure and the JPEG's tone curve drop out; the worst-fitting blocks are
trimmed and the rest refitted.

The DNG-style `ColorMatrix` pair comes from the body's own white-balance
presets, read from the same file: the camera-to-XYZ map under a preset's light
is `Bradford(D65 -> light) * sRGB-to-XYZ * M * diag(1 / preset neutral)`
(a von Kries change of white in camera space), inverted and scaled like
`gen-camera-matrices.py` scales its matrices. The daylight preset (D55, else
Fine Weather) and the tungsten preset make the pair; a body without a tungsten
preset gets the daylight matrix alone. Canon records each preset with its own
temperature (Daylight 5200 K, Cloudy 6000 K, Tungsten 3200 K), so its D55
neutral is interpolated in mired between Daylight and Cloudy, and Tungsten is
ISO studio tungsten (code 24). A pair whose lights the colour stage would not
read back (its as-shot solve, within 2 %) keeps the daylight matrix alone.

Regenerate (samples from https://raw.pixls.us/data/Panasonic/ and
https://raw.pixls.us/data/Canon/, one directory per body; the file names and
SHA-256 sums used are in the output):
  cargo build --release -p pictura-codec --example raw_jpeg_blocks
  python3 scripts/fit-camera-jpeg-matrices.py SAMPLES_DIR \\
      target/release/examples/raw_jpeg_blocks
  python3 scripts/gen-camera-matrices.py rawtoaces-data CIE_std_illum_D65.csv
"""

from __future__ import annotations

import hashlib
import json
import math
import struct
import subprocess
import sys
import tempfile
from pathlib import Path

OUT = Path(__file__).resolve().parent / "camera-jpeg-fits.json"

COLS, ROWS = 32, 24
MARGIN = 0.08
ITERATIONS = 4
TRIM = 0.15

# Acceptance: enough usable blocks, enough colour in the scene to constrain
# the matrix, and a close enough reproduction of the camera's own rendering.
MIN_BLOCKS = 60
MIN_CHROMA_SPREAD = 6.0
MAX_MEDIAN_DE = 4.0

SRGB_TO_XYZ = [
    [0.4124564, 0.3575761, 0.1804375],
    [0.2126729, 0.7151522, 0.0721750],
    [0.0193339, 0.1191920, 0.9503041],
]
D65_XY = (0.31271, 0.32902)
BRADFORD = [
    [0.8951, 0.2664, -0.1614],
    [-0.7502, 1.7135, 0.0367],
    [0.0389, -0.0685, 1.0296],
]

# Exif model names one camera is also sold under, with their source: these
# share the body's fit.
SAME_CAMERA = {
    # Panasonic's regional names of one compact (DMC-ZS100 in the Americas).
    "DMC-ZS100": ["DMC-TZ100", "DMC-TZ101", "DMC-TZ110"],
    # The GM1S is the GM1 sold with another kit.
    "DMC-GM1S": ["DMC-GM1"],
    # Canon's regional names (ExifTool's Canon model ID table pairs them).
    "Canon EOS M50": ["Canon EOS Kiss M"],
    "Canon EOS M50m2": ["Canon EOS Kiss M2"],
    "Canon EOS 850D": ["Canon EOS Rebel T8i", "Canon EOS Kiss X10i"],
}

# EXIF LightSource codes the presets carry, best first.
DAYLIGHT = (20, 9, 1)
TUNGSTEN = (3, 24)
TEMPERATURE = {1: 5500.0, 9: 5500.0, 20: 5503.0, 3: 2850.0, 24: 3200.0}

RAW_MAGIC = b"IIU\0"
TAG_MAKE, TAG_MODEL = 0x010F, 0x0110
TAG_WB_INFO, TAG_WB_INFO2 = 0x0013, 0x0027
TAG_JPG_FROM_RAW = 0x002E
TYPE_SIZE = {1: 1, 2: 1, 3: 2, 4: 4, 7: 1}


def mat_mul(a, b):
    return [[sum(a[r][k] * b[k][c] for k in range(3)) for c in range(3)] for r in range(3)]


def mat_vec(m, v):
    return [sum(m[r][k] * v[k] for k in range(3)) for r in range(3)]


def invert(m):
    a, b, c = m[0]
    d, e, f = m[1]
    g, h, i = m[2]
    det = a * (e * i - f * h) - b * (d * i - f * g) + c * (d * h - e * g)
    if abs(det) < 1e-12:
        raise ValueError("singular matrix")
    return [
        [(e * i - f * h) / det, (c * h - b * i) / det, (b * f - c * e) / det],
        [(f * g - d * i) / det, (a * i - c * g) / det, (c * d - a * f) / det],
        [(d * h - e * g) / det, (b * g - a * h) / det, (a * e - b * d) / det],
    ]


def diag(v):
    return [[v[0], 0, 0], [0, v[1], 0], [0, 0, v[2]]]


def xy_to_xyz(xy):
    x, y = xy
    return [x / y, 1.0, (1 - x - y) / y]


def white_xy(temperature: float):
    """CIE daylight locus at 4000 K and above, else the Planckian locus (Kim
    et al. 2002 cubic fit)."""
    t = temperature
    if t >= 4000:
        x = (
            -4.6070e9 / t**3 + 2.9678e6 / t**2 + 0.09911e3 / t + 0.244063
            if t <= 7000
            else -2.0064e9 / t**3 + 1.9018e6 / t**2 + 0.24748e3 / t + 0.237040
        )
        return x, -3.000 * x * x + 2.870 * x - 0.275
    x = -0.2661239e9 / t**3 - 0.2343589e6 / t**2 + 0.8776956e3 / t + 0.179910
    if t <= 2222:
        y = -1.1063814 * x**3 - 1.34811020 * x**2 + 2.18555832 * x - 0.20219683
    else:
        y = -0.9549476 * x**3 - 1.37418593 * x**2 + 2.09137015 * x - 0.16748867
    return x, y


def bradford(src_xy, dst_xy):
    inv = invert(BRADFORD)
    s = mat_vec(BRADFORD, xy_to_xyz(src_xy))
    d = mat_vec(BRADFORD, xy_to_xyz(dst_xy))
    return mat_mul(inv, mat_mul(diag([d[i] / s[i] for i in range(3)]), BRADFORD))


# --- RW2 container --------------------------------------------------------


def read_ifd0(data: bytes) -> dict:
    """IFD0's entries as tag -> (type, count, value bytes)."""
    if not data.startswith(RAW_MAGIC):
        raise ValueError("not a Panasonic raw file")
    return ifd_entries(data, "<")


def ifd_entries(data: bytes, order: str) -> dict:
    """The first IFD's entries of a TIFF-shaped block, offsets from its start."""
    (at,) = struct.unpack_from(order + "I", data, 4)
    (n,) = struct.unpack_from(order + "H", data, at)
    tags = {}
    for i in range(n):
        tag, typ, count, inline = struct.unpack_from(order + "HHI4s", data, at + 2 + 12 * i)
        size = TYPE_SIZE.get(typ, 1) * count
        if size <= 4:
            value = inline[:size]
        else:
            (off,) = struct.unpack(order + "I", inline)
            value = data[off : off + size]
        tags[tag] = (typ, count, value)
    return tags


def ascii_tag(tags: dict, tag: int) -> str:
    return tags[tag][2].split(b"\0")[0].decode("ascii").strip()


def presets(tags: dict) -> dict:
    """White-balance presets as LightSource code -> camera neutral (G = 1)."""
    out = {}
    if TAG_WB_INFO2 in tags:
        words = struct.unpack(f"<{len(tags[TAG_WB_INFO2][2]) // 2}H", tags[TAG_WB_INFO2][2])
        for i in range(min(words[0], (len(words) - 1) // 4)):
            code, r, g, b = words[1 + 4 * i : 5 + 4 * i]
            if r and g and b:
                out.setdefault(code, [g / r, 1.0, g / b])
    elif TAG_WB_INFO in tags:
        # Red and blue levels against a green of 256.
        words = struct.unpack(f"<{len(tags[TAG_WB_INFO][2]) // 2}H", tags[TAG_WB_INFO][2])
        for i in range(min(words[0], (len(words) - 1) // 3)):
            code, r, b = words[1 + 3 * i : 4 + 3 * i]
            if r and b:
                out.setdefault(code, [256 / r, 1.0, 256 / b])
    return out


def rw2_meta(data: bytes):
    tags = read_ifd0(data)
    jpeg = tags[TAG_JPG_FROM_RAW][2] if TAG_JPG_FROM_RAW in tags else None
    return ascii_tag(tags, TAG_MAKE), ascii_tag(tags, TAG_MODEL), jpeg, presets(tags)


# --- CR3 container --------------------------------------------------------

CR3_MAGIC = b"ftypcrx "
CANON_UUID = bytes.fromhex("85c0b687820f11e08111f4ce462b6a48")
TAG_MAKER_NOTE, TAG_COLOR_DATA = 0x927C, 0x4001
# ColorData version -> word index of WB_RGGBLevelsDaylight. Shade, Cloudy and
# Tungsten follow it, each four levels and the temperature Canon records
# (ExifTool's Canon ColorData9-12 tables; version 66, the R6 Mark III's, has
# version 64's layout on the sample).
CANON_DAYLIGHT = {16: 136, 17: 136, 18: 136, 19: 136, 32: 150, 33: 150, 34: 205, 48: 205}
CANON_DAYLIGHT |= {64: 110, 65: 110, 66: 110}
D55 = 5503.0


def boxes(data: bytes, start: int, end: int):
    """(name, content start, content end) of the ISO media boxes in a range."""
    at = start
    while at + 8 <= end:
        size, name = struct.unpack_from(">I4s", data, at)
        head = 8
        if size == 1:
            (size,) = struct.unpack_from(">Q", data, at + 8)
            head = 16
        elif size == 0:
            size = end - at
        if size < head or at + size > end:
            raise ValueError("bad box size")
        yield name, at + head, at + size
        at += size


def box(data: bytes, span, name: bytes):
    return next(((s, e) for n, s, e in boxes(data, *span) if n == name), None)


def first_sample(data: bytes, trak) -> bytes:
    stbl = box(data, box(data, box(data, trak, b"mdia"), b"minf"), b"stbl")
    stsz, co64 = box(data, stbl, b"stsz"), box(data, stbl, b"co64")
    (size,) = struct.unpack_from(">I", data, stsz[0] + 4)
    if size == 0:
        (size,) = struct.unpack_from(">I", data, stsz[0] + 12)
    (offset,) = struct.unpack_from(">Q", data, co64[0] + 8)
    return data[offset : offset + size]


def ctmd_notes(sample: bytes):
    """Maker notes in a CTMD sample: records of a size and a type, those of
    types 7 to 9 holding tagged TIFF blocks."""
    at = 0
    while at + 6 < len(sample):
        size, kind = struct.unpack_from("<IH", sample, at)
        if size == 0 or at + size > len(sample):
            return
        inner = at + 12
        while 7 <= kind <= 9 and inner + 8 < at + size:
            length, tag = struct.unpack_from("<II", sample, inner)
            if length < 8:
                break
            if tag == TAG_MAKER_NOTE:
                yield sample[inner + 8 : inner + length]
            inner += length
        at += size


def tiff_tags(block: bytes) -> dict:
    return ifd_entries(block, "<" if block[:2] == b"II" else ">")


def canon_presets(color_data: bytes) -> dict:
    """D55 (code 20), interpolated in mired between the Daylight and Cloudy
    presets, and Tungsten (code 24) when Canon recorded it at 3200 K."""
    words = struct.unpack(f"<{len(color_data) // 2}H", color_data)
    at = CANON_DAYLIGHT.get(words[0])
    if at is None or at + 20 > len(words):
        return {}

    def level(i):
        r, g1, g2, b, kelvin = words[i : i + 5]
        g = (g1 + g2) / 2
        return ([g / r, 1.0, g / b], kelvin) if r and b and kelvin else (None, kelvin)

    (day, t_day), (cloudy, t_cloudy), (tungsten, t_tungsten) = (
        level(at),
        level(at + 10),
        level(at + 15),
    )
    out = {}
    if day and cloudy and t_day < D55 < t_cloudy:
        f = (1 / t_day - 1 / D55) / (1 / t_day - 1 / t_cloudy)
        out[20] = [d + f * (c - d) for d, c in zip(day, cloudy)]
    if tungsten and t_tungsten == 3200:
        out[24] = tungsten
    return out


def cr3_meta(data: bytes):
    moov = box(data, (0, len(data)), b"moov")
    ifd0, samples = {}, []
    for name, start, end in boxes(data, *moov):
        if name == b"uuid" and data[start : start + 16] == CANON_UUID:
            cmt1 = box(data, (start + 16, end), b"CMT1")
            ifd0 = tiff_tags(data[cmt1[0] : cmt1[1]])
        elif name == b"trak":
            samples.append(first_sample(data, (start, end)))
    jpegs = [s for s in samples if s.startswith(b"\xff\xd8")]
    jpeg = max(jpegs, key=len) if jpegs else None
    found = {}
    for note in (n for s in samples for n in ctmd_notes(s)):
        tags = tiff_tags(note)
        if TAG_COLOR_DATA in tags:
            found = canon_presets(tags[TAG_COLOR_DATA][2])
            break
    return ascii_tag(ifd0, TAG_MAKE), ascii_tag(ifd0, TAG_MODEL), jpeg, found


# --- fit ------------------------------------------------------------------


def blocks(raw: Path, jpeg: bytes, tool: str):
    with tempfile.NamedTemporaryFile(suffix=".jpg") as f:
        f.write(jpeg)
        f.flush()
        lines = subprocess.run(
            [tool, str(raw), f.name, str(COLS), str(ROWS)],
            check=True,
            capture_output=True,
            text=True,
        ).stdout.splitlines()
    head = lines[0].split()
    neutral = None if head[1] == "none" else [float(v) for v in head[1:]]
    rows = []
    for line in lines[1:]:
        v = line.split()
        col, row = int(v[0]), int(v[1])
        cam = [float(x) for x in v[2:5]]
        jpg = [float(x) for x in v[5:8]]
        rows.append((col, row, cam, jpg, float(v[8])))
    return neutral, rows


def usable(rows, neutral):
    out = []
    for col, row, cam, jpg, clipped in rows:
        u, v = (col + 0.5) / COLS, (row + 0.5) / ROWS
        if min(u, v, 1 - u, 1 - v) < MARGIN or clipped > 0:
            continue
        if max(cam) > 0.9 or min(cam) < 0.002:
            continue
        if max(jpg) > 0.8 or min(jpg) < 0.004:
            continue
        out.append(([cam[i] / neutral[i] for i in range(3)], jpg))
    return out


def fit_rows(xs, ts):
    """Rows `m` minimizing sum (m.x - t)^2 with m.(1,1,1) = 1."""
    a = [[sum(x[r] * x[c] for x in xs) for c in range(3)] for r in range(3)]
    ainv = invert(a)
    ones = [1.0, 1.0, 1.0]
    u = mat_vec(ainv, ones)
    m = []
    for ch in range(3):
        b = [sum(x[r] * t[ch] for x, t in zip(xs, ts)) for r in range(3)]
        m0 = mat_vec(ainv, b)
        lam = (1 - sum(m0)) / sum(u)
        m.append([m0[k] + lam * u[k] for k in range(3)])
    return m


def lab(xyz):
    w = xy_to_xyz(D65_XY)

    def f(t):
        return t ** (1 / 3) if t > 216 / 24389 else (24389 / 27 * t + 16) / 116

    fx, fy, fz = (f(xyz[i] / w[i]) for i in range(3))
    return 116 * fy - 16, 500 * (fx - fy), 200 * (fy - fz)


def chroma_error(m, xs, ts):
    """Median and 90th percentile CIELAB difference of chromaticity at the
    target's luminance, and the target colours' a*b* standard deviation."""
    errors, ab = [], []
    for x, t in zip(xs, ts):
        p = mat_vec(SRGB_TO_XYZ, mat_vec(m, x))
        q = mat_vec(SRGB_TO_XYZ, t)
        if p[1] <= 0 or q[1] <= 0:
            errors.append(99.0)
            continue
        lp = lab([v / p[1] * 0.18 for v in p])
        lq = lab([v / q[1] * 0.18 for v in q])
        errors.append(math.hypot(lp[1] - lq[1], lp[2] - lq[2]))
        ab.append(lq[1:])
    errors.sort()
    n = len(ab)
    ma, mb = sum(a for a, _ in ab) / n, sum(b for _, b in ab) / n
    spread = math.sqrt(sum((a - ma) ** 2 + (b - mb) ** 2 for a, b in ab) / n)
    return errors[len(errors) // 2], errors[int(len(errors) * 0.9)], spread


def fit(samples):
    """The white-preserving matrix that best maps each block's camera
    chromaticity to its JPEG chromaticity. Normalizing every block to unit sum
    leaves out exposure and the JPEG's tone curve; the worst blocks (texture
    the two renderings place differently) are dropped and the rest refitted."""
    xs = [[v / sum(x) for v in x] for x, _ in samples]
    ts = [[v / sum(t) for v in t] for _, t in samples]
    keep = list(range(len(xs)))
    for _ in range(ITERATIONS):
        m = fit_rows([xs[i] for i in keep], [ts[i] for i in keep])
        residual = {
            i: sum((a - b) ** 2 for a, b in zip(mat_vec(m, xs[i]), ts[i])) for i in range(len(xs))
        }
        keep = sorted(range(len(xs)), key=residual.get)[: int(len(xs) * (1 - TRIM))]
    return m, chroma_error(m, [xs[i] for i in keep], [ts[i] for i in keep])


def color_matrix(m, neutral, code):
    """XYZ -> camera under the preset light `code` whose neutral is `neutral`."""
    w = white_xy(TEMPERATURE[code])
    cam_to_xyz = mat_mul(
        bradford(D65_XY, w), mat_mul(SRGB_TO_XYZ, mat_mul(m, diag([1 / v for v in neutral])))
    )
    cm = invert(cam_to_xyz)
    cam = mat_vec(cm, xy_to_xyz(w))
    scale = 1 / max(cam)
    return [round(v * scale, 4) for row in cm for v in row]


def cct(xyz) -> float:
    """McCamy's approximation of a white's correlated colour temperature."""
    x, y = xyz[0] / sum(xyz), xyz[1] / sum(xyz)
    n = (x - 0.3320) / (0.1858 - y)
    return 449 * n**3 + 3525 * n**2 + 6823.3 * n + 5520.33


def reads_back(matrices) -> bool:
    """Whether the colour stage's as-shot solve (the pair interpolated in
    mired, iterated from 5003 K) reads each matrix's own light, as its
    response to that light's white, within 2 %."""
    (lo, m_lo), (hi, m_hi) = sorted(((TEMPERATURE[c], m) for c, m in matrices), key=lambda p: p[0])

    def camera_to_xyz(t):
        g = min(max((1 / t - 1 / hi) / (1 / lo - 1 / hi), 0.0), 1.0)
        m = [a * g + b * (1 - g) for a, b in zip(m_lo, m_hi)]
        return invert([m[0:3], m[3:6], m[6:9]])

    for want, m in ((lo, m_lo), (hi, m_hi)):
        neutral = mat_vec([m[0:3], m[3:6], m[6:9]], xy_to_xyz(white_xy(want)))
        t = 5003.0
        for _ in range(30):
            t = cct(mat_vec(camera_to_xyz(t), neutral))
        if abs(t - want) > 0.02 * want:
            return False
    return True


def pick(found: dict, codes):
    return next(((c, found[c]) for c in codes if c in found), None)


def body(raw: Path, tool: str) -> dict:
    data = raw.read_bytes()
    make, model, jpeg, found = (cr3_meta if data[4:12] == CR3_MAGIC else rw2_meta)(data)
    # A sample written under a regional name stands for its body.
    model = next(
        (body for body, names in SAME_CAMERA.items() if model.upper() in map(str.upper, names)),
        model,
    )
    entry = {
        "make": make,
        "model": model,
        "aliases": SAME_CAMERA.get(model, []),
        "sample": raw.name,
        "sha256": hashlib.sha256(data).hexdigest(),
    }
    if jpeg is None:
        return entry | {"rejected": "no embedded JPEG"}
    neutral, rows = blocks(raw, jpeg, tool)
    if neutral is None:
        return entry | {"rejected": "no as-shot white balance"}
    daylight = pick(found, DAYLIGHT)
    if daylight is None:
        return entry | {"rejected": "no daylight white-balance preset"}
    samples = usable(rows, neutral)
    if len(samples) < MIN_BLOCKS:
        return entry | {"rejected": f"{len(samples)} usable blocks"}
    m, (median, p90, spread) = fit(samples)
    entry |= {
        "blocks": len(samples),
        "median_de": round(median, 2),
        "p90_de": round(p90, 2),
        "chroma_spread": round(spread, 1),
    }
    if spread < MIN_CHROMA_SPREAD:
        return entry | {"rejected": "too little colour in the scene"}
    if median > MAX_MEDIAN_DE:
        return entry | {"rejected": "matrix does not reproduce the camera JPEG"}
    pair = [daylight] + ([t] if (t := pick(found, TUNGSTEN)) else [])
    matrices = [(code, color_matrix(m, n, code)) for code, n in pair]
    if len(matrices) == 2 and not reads_back(matrices):
        # The fit and the tungsten preset disagree; the daylight matrix alone
        # still reads its own light.
        matrices = matrices[:1]
        entry["note"] = "tungsten matrix dropped: the pair does not read back its lights"
    return entry | {
        "illuminants": [code for code, _ in matrices],
        "matrices": [cm for _, cm in matrices],
    }


def main() -> None:
    if len(sys.argv) != 3:
        sys.exit(__doc__)
    root, tool = Path(sys.argv[1]), sys.argv[2]
    files = sorted(
        p for p in root.rglob("*") if p.suffix.upper() in (".RW2", ".RWL", ".RAW", ".CR3")
    )
    # Fits of samples not in this run are kept, so one maker can be refitted alone.
    names = {p.name for p in files}
    old = json.loads(OUT.read_text()) if OUT.exists() else []
    fits = [e for e in old if e["sample"] not in names]
    for raw in files:
        try:
            entry = body(raw, tool)
        except (ValueError, subprocess.CalledProcessError, KeyError, struct.error) as e:
            entry = {"sample": raw.name, "rejected": f"unreadable: {e}"}
        print(json.dumps(entry), file=sys.stderr)
        fits.append(entry)
    fits.sort(key=lambda e: (e.get("make", ""), e.get("model", ""), e["sample"]))
    OUT.write_text(json.dumps(fits, indent=1) + "\n")


if __name__ == "__main__":
    main()
