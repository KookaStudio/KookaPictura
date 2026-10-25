#!/usr/bin/env python3
"""Regenerate the profile-bearing CMYK fixture from the committed base fixture.

The base `cmyk.psd` is profile-free. This appends image resource 1039 with the
bundled CC0 coated CMYK profile, keeping every other byte stable. The result is
committed so the lcms2 oracle can read a real file.
"""
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BASE = ROOT / "crates/pictura-codec/tests/fixtures/cmyk.psd"
ICC = ROOT / "assets/icc/photocraft-coated-cmyk.icc"
OUT = ROOT / "crates/pictura-codec/tests/fixtures/cmyk_profile.psd"


def block(data: bytes) -> bytes:
    out = b"8BIM" + (1039).to_bytes(2, "big") + b"\x00\x00"
    out += len(data).to_bytes(4, "big") + data
    if len(data) % 2:
        out += b"\x00"
    return out


def main() -> None:
    data = BASE.read_bytes()
    assert data[:4] == b"8BPS", "base fixture is a PSD"
    color_len = int.from_bytes(data[26:30], "big")
    at = 30 + color_len
    res_len = int.from_bytes(data[at : at + 4], "big")
    section = data[at + 4 : at + 4 + res_len]
    profile = ICC.read_bytes()
    new_section = section + block(profile)
    out = data[:at] + len(new_section).to_bytes(4, "big") + new_section + data[at + 4 + res_len :]
    OUT.write_bytes(out)
    print(f"wrote {OUT.relative_to(ROOT)} ({len(out)} bytes)")


if __name__ == "__main__":
    main()
