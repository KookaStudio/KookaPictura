#!/usr/bin/env python3
"""psd-tools knockout pixel reference for `pictura-render`.

`psd-tools` ships an independent compositor that implements deep knockout
(`psd_tools/composite/composite.py`, verified against Photoshop). It is the
second implementation for the CPU knockout path, exactly as ImageMagick is for
the blend modes and psd-tools itself is for the codec.

`gen` opens a committed fixture, composites it, converts to RGBA, and writes raw
interleaved RGBA8 to the fixture's reference path, which
`tests/oracle/knockout_oracle.rs` reads without a codec dependency. The fixture is
chosen from the `--out` stem (`knockout_deep` -> `knockout.psd`,
`knockout_group` -> `knockout_group.psd`); pass `--fixture` to override.

Formats: raw interleaved RGBA8 (`r,g,b,a` per pixel, row major), 8x8.

Usage:
    python3 scripts/psd_knockout_reference.py gen [--out PATH] [--fixture PATH]
"""

from __future__ import annotations

import argparse
from pathlib import Path

from psd_tools import PSDImage

ROOT = Path(__file__).resolve().parent.parent
FIXTURE_DIR = ROOT / "crates" / "pictura-codec" / "tests" / "fixtures"
OUT_DIR = ROOT / "crates" / "pictura-render" / "tests" / "fixtures"
FIXTURES = {
    "knockout_deep": FIXTURE_DIR / "knockout.psd",
    "knockout_group": FIXTURE_DIR / "knockout_group.psd",
    "knockout_isolated_group": FIXTURE_DIR / "knockout_isolated_group.psd",
    "knockout_shallow_group": FIXTURE_DIR / "knockout_shallow_group.psd",
}
DEFAULT_OUT = OUT_DIR / "knockout_deep.rgba"


def gen(out: Path, fixture: Path | None = None) -> None:
    if fixture is None:
        fixture = FIXTURES.get(out.stem, FIXTURES["knockout_deep"])
    image = PSDImage.open(fixture).composite().convert("RGBA")
    out.write_bytes(image.tobytes())
    print(
        f"wrote {out.relative_to(ROOT)} from {fixture.name} "
        f"({image.width}x{image.height})"
    )


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="psd-tools knockout pixel reference")
    sub = parser.add_subparsers(dest="command", required=True)

    p_gen = sub.add_parser("gen", help="regenerate the committed reference")
    p_gen.add_argument("--out", default=str(DEFAULT_OUT))
    p_gen.add_argument(
        "--fixture",
        default=None,
        help="source PSD (default: inferred from --out stem)",
    )
    p_gen.set_defaults(
        func=lambda args: gen(
            Path(args.out).resolve(),
            Path(args.fixture).resolve() if args.fixture else None,
        )
    )

    args = parser.parse_args(argv)
    return args.func(args)


if __name__ == "__main__":
    raise SystemExit(main())
