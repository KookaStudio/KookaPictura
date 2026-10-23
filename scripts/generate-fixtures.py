#!/usr/bin/env python3
"""Author deterministic PSD fixtures with the independent `psd-tools` library.

These fixtures are the oracle for `pictura-codec`: a Rust reader is only
trustworthy if it agrees with a second, independent implementation of the
format. Every pixel value, layer name, and offset below is fixed, and
`psd-tools`/PIL emit byte-identical output on repeated runs, so regenerating
does not churn the working tree.

Regenerate with:  python3 scripts/generate-fixtures.py
"""

from __future__ import annotations

import io
import struct
from pathlib import Path

from PIL import Image
from psd_tools import PSDImage
from psd_tools.constants import (
    BlendMode,
    ChannelID,
    ColorMode,
    ColorSpaceID,
    EffectOSType,
    Resource,
    Tag,
)
from psd_tools.psd.adjustments import (
    BrightnessContrast,
    ColorLookup,
    ColorStop,
    GradientMap,
    LevelRecord,
    Levels,
    PhotoFilter,
    SelectiveColor,
    TransparencyStop,
)
from psd_tools.psd.base import EmptyElement, ShortIntegerElement
from psd_tools.psd.color import Color
from psd_tools.psd.color_mode_data import ColorModeData
from psd_tools.psd.descriptor import (
    Bool,
    Descriptor,
    DescriptorBlock,
    DescriptorBlock2,
    Double,
    Enumerated,
    List,
    RawData,
    String,
    UnitFloat,
)
from psd_tools.psd.engine_data import EngineData
from psd_tools.psd.effects_layer import (
    CommonStateInfo,
    EffectsLayer,
    OuterGlowInfo,
    ShadowInfo,
)
from psd_tools.psd.layer_and_mask import ChannelDataList
from psd_tools.psd.image_resources import ImageResource
from psd_tools.psd.patterns import (
    Pattern,
    VirtualMemoryArray,
    VirtualMemoryArrayList,
)
from psd_tools.psd.tagged_blocks import TaggedBlock, TaggedBlocks
from psd_tools.psd.vector import (
    ClosedKnotLinked,
    ClosedPath,
    Path as VectorPath,
    PathFillRule,
    VectorMaskSetting,
    VectorStrokeContentSetting,
)
from psd_tools.terminology import Enum, Key, Klass, Type, Unit

ROOT = Path(__file__).resolve().parent.parent
FIXTURE_DIR = ROOT / "crates" / "pictura-codec" / "tests" / "fixtures"

WIDTH = HEIGHT = 8

# Depth-16 and depth-32 narrowing samples. Chosen so the big-endian stored
# values exercise `v >> 8` and `clamp(trunc(f * 256))` at their boundaries and
# match psd-tools' composite exactly (its I;16B / F;32BF scaling).
DEPTH16_SAMPLES = [0, 1, 255, 256, 257, 32768, 65534, 65535]
DEPTH32_SAMPLES = [0.0, 0.001, 0.5, 1.0, 1.5, -0.5, 0.99609375, 255.0]

# The identity cube embedded in `color_lookup.psd`; must match
# `pictura_render::identity_cube()` byte-for-byte.
CUBE_IDENTITY = (
    b'TITLE "Identity"\nLUT_3D_SIZE 2\n'
    b"0.0 0.0 0.0\n1.0 0.0 0.0\n0.0 1.0 0.0\n1.0 1.0 0.0\n"
    b"0.0 0.0 1.0\n1.0 0.0 1.0\n0.0 1.0 1.0\n1.0 1.0 1.0\n"
)

# Fixed image-resource payloads for `image_resources.psd`: an EXIF blob and a
# small XMP packet (both raw bytes, so psd-tools' save does not try to apply
# them as a colour profile). Only their bytes matter to the parser oracle.
EXIF_BYTES = bytes((i * 7 + 3) % 256 for i in range(128))
XMP_BYTES = b'<x:xmpmeta xmlns:x="adobe:ns:meta/"></x:xmpmeta>'

# The `metadata.psd` resources: a real little-endian EXIF IFD (IFD0 + Exif
# sub-IFD), an IPTC-IIM stream, and an XMP packet. Fixed bytes so the fixture is
# byte-stable and `exiftool` can decode it as an independent oracle.
METADATA_XMP = (
    b'<?xpacket begin="\xef\xbb\xbf" id="W5M0MpCehiHzreSzNTczkc9d"?>\n'
    b'<x:xmpmeta xmlns:x="adobe:ns:meta/">'
    b'<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">'
    b'<rdf:Description xmlns:dc="http://purl.org/dc/elements/1.1/" '
    b'xmlns:photoshop="http://ns.adobe.com/photoshop/1.0/" '
    b'xmlns:xmpRights="http://ns.adobe.com/xap/1.0/rights/" '
    b'xmlns:acme="http://example.com/acme/1.0/" '
    b'dc:format="image/psd" acme:Marker="keep-me">'
    b"<dc:title><rdf:Alt><rdf:li xml:lang=\"x-default\">Fixture Title</rdf:li></rdf:Alt></dc:title>"
    b"<dc:creator><rdf:Seq><rdf:li>Ada Lovelace</rdf:li></rdf:Seq></dc:creator>"
    b"<dc:description><rdf:Alt><rdf:li xml:lang=\"x-default\">A fixture caption.</rdf:li></rdf:Alt></dc:description>"
    b"<dc:rights><rdf:Alt><rdf:li xml:lang=\"x-default\">(c) 2026 Kooka Pictura</rdf:li></rdf:Alt></dc:rights>"
    b"<photoshop:Credit>Kooka Pictura</photoshop:Credit>"
    b"<photoshop:Source>Test Suite</photoshop:Source>"
    b"<acme:Note>keep-me-too</acme:Note>"
    b"</rdf:Description>"
    b'</rdf:RDF></x:xmpmeta>\n<?xpacket end="w"?>'
)


def _metadata_exif() -> bytes:
    """A little-endian TIFF: IFD0 (Make/Model/Orientation/Software/DateTime)
    plus an Exif sub-IFD (ExposureTime/FNumber/ISO/DateTimeOriginal/FocalLength).
    """
    make = b"ACME Cameras\x00"
    model = b"ACME One\x00"
    software = b"Kooka Pictura Fixtures\x00"
    date_time = b"2026:09:22 12:00:00\x00"
    date_orig = b"2026:09:22 11:59:00\x00"
    exposure = struct.pack("<II", 1, 125)
    f_number = struct.pack("<II", 28, 10)
    focal = struct.pack("<II", 50, 1)

    blobs = [make, model, software, date_time, exposure, f_number, focal, date_orig]
    ifd0_size = 2 + 6 * 12 + 4
    exif_size = 2 + 5 * 12 + 4
    offsets = []
    cursor = 8 + ifd0_size + exif_size
    for blob in blobs:
        offsets.append(cursor)
        cursor += len(blob)
    make_off, model_off, software_off, dt_off, exp_off, fnum_off, focal_off, dto_off = (
        offsets
    )

    def pointer(tag: int, typ: int, count: int, offset: int) -> bytes:
        return struct.pack("<HHI", tag, typ, count) + struct.pack("<I", offset)

    def inline(tag: int, typ: int, value: int) -> bytes:
        return struct.pack("<HHI", tag, typ, 1) + struct.pack("<H", value) + b"\x00\x00"

    ifd0 = struct.pack("<H", 6)
    ifd0 += pointer(271, 2, len(make), make_off)
    ifd0 += pointer(272, 2, len(model), model_off)
    ifd0 += inline(274, 3, 1)
    ifd0 += pointer(305, 2, len(software), software_off)
    ifd0 += pointer(306, 2, len(date_time), dt_off)
    ifd0 += pointer(0x8769, 4, 1, 8 + ifd0_size)
    ifd0 += struct.pack("<I", 0)

    exif = struct.pack("<H", 5)
    exif += pointer(33434, 5, 1, exp_off)
    exif += pointer(33437, 5, 1, fnum_off)
    exif += inline(34855, 3, 200)
    exif += pointer(36867, 2, len(date_orig), dto_off)
    exif += pointer(37386, 5, 1, focal_off)
    exif += struct.pack("<I", 0)

    header = b"II" + struct.pack("<H", 42) + struct.pack("<I", 8)
    return header + ifd0 + exif + b"".join(blobs)


def _metadata_iptc() -> bytes:
    """An IPTC-IIM stream of core caption/credit records."""

    def record(number: int, dataset: int, value: bytes) -> bytes:
        return b"\x1c" + bytes([number, dataset]) + struct.pack(">H", len(value)) + value

    return b"".join(
        [
            record(2, 5, b"Fixture Title"),
            record(2, 80, b"Ada Lovelace"),
            record(2, 85, b"Engineer"),
            record(2, 110, b"Kooka Pictura"),
            record(2, 115, b"Test Suite"),
            record(2, 116, b"(c) 2026 Kooka Pictura"),
            record(2, 120, b"A fixture caption."),
        ]
    )


def _solid(size: tuple[int, int], color: tuple[int, int, int]) -> Image.Image:
    return Image.new("RGB", size, color)


def two_layers() -> PSDImage:
    """RGB, two named pixel layers with distinct solid colors."""
    psd = PSDImage.new("RGB", (WIDTH, HEIGHT), color=(0, 0, 0))
    psd.create_pixel_layer(
        _solid((4, 4), (255, 0, 0)), name="Red", left=0, top=0
    )
    psd.create_pixel_layer(
        _solid((4, 4), (0, 0, 255)), name="Blue", left=4, top=4
    )
    return psd


def group() -> PSDImage:
    """RGB, a group containing two named pixel layers."""
    psd = PSDImage.new("RGB", (WIDTH, HEIGHT), color=(0, 0, 0))
    inner_green = psd.create_pixel_layer(
        _solid((WIDTH, HEIGHT), (0, 255, 0)), name="Inner Green"
    )
    inner_yellow = psd.create_pixel_layer(
        _solid((WIDTH, HEIGHT), (255, 255, 0)), name="Inner Yellow"
    )
    psd.create_group([inner_green, inner_yellow], name="Group A")
    return psd


def pass_through_vector_mask() -> PSDImage:
    """RGB, a blue Backdrop plus a Pass Through group whose red child is cut by
    an inverted group vector mask over ``(0,0)-(6,6)``.

    The group is visible only outside the rectangle: the fixture exercises the
    pass-through fast path falling back to isolation when a vector mask is
    present. Without that fallback the child paints the whole canvas red.
    """
    psd = PSDImage.new("RGB", (WIDTH, HEIGHT), color=(0, 0, 255))
    psd.create_pixel_layer(
        Image.new("RGBA", (WIDTH, HEIGHT), (0, 0, 255, 255)),
        name="Backdrop",
        left=0,
        top=0,
    )
    child = psd.create_pixel_layer(
        Image.new("RGBA", (WIDTH, HEIGHT), (255, 0, 0, 255)),
        name="Inner Red",
        left=0,
        top=0,
    )
    group = psd.create_group([child], name="Group A")
    group._record.tagged_blocks[Tag.VECTOR_MASK_SETTING1] = TaggedBlock(
        key=Tag.VECTOR_MASK_SETTING1,
        data=VectorMaskSetting(
            version=3, flags=0x01, path=_closed_rect_path(0, 0, 6, 6)
        ),
    )
    return psd


def knockout() -> PSDImage:
    """RGB, a red Background, an intermediate green layer, and a half-fill blue
    layer carrying `knko = Deep` (2) that punches the green through.

    The bottom layer is a genuine Background -- opaque by construction, with no
    transparency channel -- which is what psd-tools' compositor
    (`_is_background_layer`) and the CPU's bottom-layer assumption both use as the
    deep-knockout target. `PSDImage.new(..., color=...)` alone only sets
    psd-tools' `_background_color` and the merged composite; it emits no layer
    record, so a channel-stripped pixel layer is authored explicitly.

    `knko` is a `ByteElement`, so it is authored through `set_data`; assigning a
    raw `TaggedBlock(data=b"\\x02\\x00\\x00\\x00")` is re-encoded to two bytes and
    the value is lost.
    """
    psd = PSDImage.new("RGB", (WIDTH, HEIGHT), color=(255, 0, 0))
    background = psd.create_pixel_layer(
        _solid((WIDTH, HEIGHT), (255, 0, 0)), name="Background"
    )
    # psd-tools' `create_pixel_layer` always appends a transparency channel; a
    # real Background has none, and that absence is how its own compositor
    # identifies the knockout target. Keep channel_info and data aligned.
    rec = background._record
    channels = [
        (info, data)
        for info, data in zip(rec.channel_info, background._channels)
        if info.id != ChannelID.TRANSPARENCY_MASK
    ]
    rec.channel_info = [info for info, _ in channels]
    background._channels = ChannelDataList([data for _, data in channels])

    psd.create_pixel_layer(_solid((WIDTH, HEIGHT), (0, 255, 0)), name="Green")
    blue = psd.create_pixel_layer(_solid((WIDTH, HEIGHT), (0, 0, 255)), name="Blue")
    blue.opacity = 128
    blue._record.tagged_blocks.set_data(Tag.KNOCKOUT_SETTING, 2)
    return psd


def knockout_group() -> PSDImage:
    """RGB, a red Background plus a pass-through group of green and a half-fill
    blue carrying `knko = Deep` (2) that punches the green through.

    The top layer's knockout punches through the grouped layers to the layer
    below the group (`docs/05-layers/layer-groups.md:81`). psd-tools defaults a
    `create_group` folder to blend mode ``pass`` (`lsct``), which is exactly the
    pass-through context the CPU compositor now recurses; the same
    channel-stripped Background as `knockout()` is the stopping point.
    """
    psd = PSDImage.new("RGB", (WIDTH, HEIGHT), color=(255, 0, 0))
    background = psd.create_pixel_layer(
        _solid((WIDTH, HEIGHT), (255, 0, 0)), name="Background"
    )
    rec = background._record
    channels = [
        (info, data)
        for info, data in zip(rec.channel_info, background._channels)
        if info.id != ChannelID.TRANSPARENCY_MASK
    ]
    rec.channel_info = [info for info, _ in channels]
    background._channels = ChannelDataList([data for _, data in channels])

    green = psd.create_pixel_layer(_solid((WIDTH, HEIGHT), (0, 255, 0)), name="Green")
    blue = psd.create_pixel_layer(_solid((WIDTH, HEIGHT), (0, 0, 255)), name="Blue")
    blue.opacity = 128
    blue._record.tagged_blocks.set_data(Tag.KNOCKOUT_SETTING, 2)
    psd.create_group([green, blue], name="Group")
    return psd


def knockout_isolated_group() -> PSDImage:
    """RGB, a red Background, a yellow layer, and an isolated (Normal) group of
    green and a half-fill blue carrying `knko = Deep` (2).

    psd-tools' compositor stops deep knockout at an isolated group
    (`composite.py`: it "escapes pass-through groups but stops at an isolated
    one"), so the blue punches the group's green through to the group's initial
    backdrop, then the group composites over the yellow layer below. The same
    channel-stripped Background as `knockout()` is the document base; the group
    is made isolated by writing an explicit non-`pass` blend.
    """
    psd = PSDImage.new("RGB", (WIDTH, HEIGHT), color=(255, 0, 0))
    background = psd.create_pixel_layer(
        _solid((WIDTH, HEIGHT), (255, 0, 0)), name="Background"
    )
    rec = background._record
    channels = [
        (info, data)
        for info, data in zip(rec.channel_info, background._channels)
        if info.id != ChannelID.TRANSPARENCY_MASK
    ]
    rec.channel_info = [info for info, _ in channels]
    background._channels = ChannelDataList([data for _, data in channels])

    psd.create_pixel_layer(_solid((WIDTH, HEIGHT), (255, 255, 0)), name="Yellow")
    green = psd.create_pixel_layer(_solid((WIDTH, HEIGHT), (0, 255, 0)), name="Green")
    blue = psd.create_pixel_layer(_solid((WIDTH, HEIGHT), (0, 0, 255)), name="Blue")
    blue.opacity = 128
    blue._record.tagged_blocks.set_data(Tag.KNOCKOUT_SETTING, 2)
    isolated = psd.create_group([green, blue], name="Group")
    isolated.blend_mode = BlendMode.NORMAL
    return psd


def knockout_shallow_group() -> PSDImage:
    """RGB, a red Background, an opaque yellow intervening layer, then a
    pass-through group of green and a half-fill blue carrying `knko = Shallow`
    (1).

    Shallow knockout stops at the enclosing group's entry backdrop
    (`docs/05-layers/layer-groups.md:81`), unlike `knockout_group()`'s Deep,
    which reaches the document Background. The red-plus-yellow backdrop current
    when the group began is the stopping point, so the blue punches the group's
    green through while the yellow below the group still contributes. The same
    channel-stripped Background as `knockout()` is the document base.
    """
    psd = PSDImage.new("RGB", (WIDTH, HEIGHT), color=(255, 0, 0))
    background = psd.create_pixel_layer(
        _solid((WIDTH, HEIGHT), (255, 0, 0)), name="Background"
    )
    rec = background._record
    channels = [
        (info, data)
        for info, data in zip(rec.channel_info, background._channels)
        if info.id != ChannelID.TRANSPARENCY_MASK
    ]
    rec.channel_info = [info for info, _ in channels]
    background._channels = ChannelDataList([data for _, data in channels])

    psd.create_pixel_layer(_solid((WIDTH, HEIGHT), (255, 255, 0)), name="Yellow")
    green = psd.create_pixel_layer(_solid((WIDTH, HEIGHT), (0, 255, 0)), name="Green")
    blue = psd.create_pixel_layer(_solid((WIDTH, HEIGHT), (0, 0, 255)), name="Blue")
    blue.opacity = 128
    blue._record.tagged_blocks.set_data(Tag.KNOCKOUT_SETTING, 1)
    psd.create_group([green, blue], name="Group")
    return psd


def masked() -> PSDImage:
    """RGB, one pixel layer with a raster layer mask."""
    psd = PSDImage.new("RGB", (WIDTH, HEIGHT), color=(0, 0, 0))
    layer = psd.create_pixel_layer(
        _solid((WIDTH, HEIGHT), (128, 64, 32)), name="Masked"
    )
    mask = Image.new("L", (WIDTH, HEIGHT), 0)
    for y in range(HEIGHT):
        for x in range(WIDTH):
            mask.putpixel((x, y), 255 if x >= y else 0)
    layer.create_mask(mask)
    return psd


def gray() -> PSDImage:
    """Grayscale, one named pixel layer."""
    psd = PSDImage.new("L", (WIDTH, HEIGHT), color=0)
    psd.create_pixel_layer(
        Image.new("L", (WIDTH, HEIGHT), 200), name="Gray"
    )
    return psd


def _set_composite(psd: PSDImage, planes: list[bytes]) -> None:
    """Overwrite the image-data section with hand-built per-channel planes."""
    psd._record.image_data.set_data([bytes(p) for p in planes], psd._record.header)


def indexed() -> PSDImage:
    """Indexed, an 8x8 index plane plus a 768-byte non-interleaved palette."""
    psd = PSDImage.new("RGB", (WIDTH, HEIGHT), color=(0, 0, 0))
    psd._record.header.color_mode = ColorMode.INDEXED
    psd._record.header.channels = 1
    palette = bytearray(768)
    for i in range(256):
        palette[i] = i
        palette[256 + i] = 255 - i
        palette[512 + i] = (i * 7) % 256
    psd._record.color_mode_data = ColorModeData(bytes(palette))
    indices = bytes(i % 8 for i in range(WIDTH * HEIGHT))
    _set_composite(psd, [indices])
    return psd


def cmyk() -> PSDImage:
    """CMYK, varied known planes plus one constant pixel layer.

    The stored planes are `0 = full ink, 255 = no ink`, so the profile-free
    formula maps `(128, 64, 32, 200)` to RGB `(100, 50, 25)`. psd-tools/Pillow
    rounds `color * black / 255` while the engine floors it, so the varied
    samples make that <=1 difference visible; the oracle compares within 1.
    """
    psd = PSDImage.new("CMYK", (WIDTH, HEIGHT), color=(0, 0, 0, 0))
    psd.create_pixel_layer(
        Image.new("CMYK", (WIDTH, HEIGHT), (128, 64, 32, 200)), name="Ink"
    )
    samples = [
        (128, 64, 32, 200),
        (50, 200, 10, 90),
        (0, 0, 0, 0),
        (255, 255, 255, 255),
        (10, 20, 30, 40),
        (200, 10, 90, 150),
        (77, 88, 99, 111),
        (250, 5, 128, 63),
    ]
    pixels = [samples[i % len(samples)] for i in range(WIDTH * HEIGHT)]
    _set_composite(
        psd,
        [
            bytes(p[0] for p in pixels),
            bytes(p[1] for p in pixels),
            bytes(p[2] for p in pixels),
            bytes(p[3] for p in pixels),
        ],
    )
    psd._updated = False
    return psd


def lab() -> PSDImage:
    """Lab, eight non-neutral in-gamut colors repeated across the image.

    Each sample is a `(L, a, b)` triple. The oracle compares the decoded RGB to
    lcms2's exact (unoptimized) LAB->sRGB transform within 1 LSB; the optimized
    color LUT that psd-tools' `.convert("RGB")` applies is not the reference.
    """
    psd = PSDImage.new("LAB", (WIDTH, HEIGHT), color=(0, 0, 0))
    samples = [
        (225, 82, 114),
        (200, 110, 100),
        (150, 110, 110),
        (120, 160, 140),
        (90, 120, 160),
        (160, 90, 120),
        (110, 100, 160),
        (200, 140, 160),
    ]
    pixels = [samples[i % len(samples)] for i in range(WIDTH * HEIGHT)]
    _set_composite(
        psd,
        [
            bytes(p[0] for p in pixels),
            bytes(p[1] for p in pixels),
            bytes(p[2] for p in pixels),
        ],
    )
    return psd


def bitmap() -> PSDImage:
    """Depth-1 Bitmap, an 8x8 image of alternating black/white pixels (0xAA)."""
    psd = PSDImage.new("BITMAP", (WIDTH, HEIGHT), color=0)
    psd._record.header.depth = 1
    _set_composite(psd, [bytes([0xAA]) * HEIGHT])
    return psd


def rgb16() -> PSDImage:
    """Flat depth-16 RGB, big-endian `u16` planes from `DEPTH16_SAMPLES`."""
    psd = PSDImage.new("RGB", (WIDTH, HEIGHT), depth=16)
    row = b"".join(struct.pack(">H", DEPTH16_SAMPLES[x]) for x in range(WIDTH))
    _set_composite(psd, [row * HEIGHT] * 3)
    return psd


def rgb32() -> PSDImage:
    """Flat depth-32 RGB, big-endian `f32` planes from `DEPTH32_SAMPLES`."""
    psd = PSDImage.new("RGB", (WIDTH, HEIGHT), depth=32)
    row = b"".join(struct.pack(">f", DEPTH32_SAMPLES[x]) for x in range(WIDTH))
    _set_composite(psd, [row * HEIGHT] * 3)
    return psd


def _depth_layered(depth: int) -> bytes:
    """A depth-16/32 RGB PSD that also exercises the layer and extra-channel
    paths: a composite extra channel and one pixel layer carrying color,
    transparency, a mask, and an unmodeled spot channel.

    Hand-authored because psd-tools cannot write 16/32-bit layer channels. The
    independent decoder still opens it (see `main`).
    """
    samples = DEPTH16_SAMPLES if depth == 16 else DEPTH32_SAMPLES

    def plane(shift: int) -> bytes:
        rotated = list(samples[shift:]) + list(samples[:shift])
        if depth == 16:
            row = b"".join(struct.pack(">H", v) for v in rotated)
        else:
            row = b"".join(struct.pack(">f", v) for v in rotated)
        return row * HEIGHT

    color = [plane(0), plane(1), plane(2)]
    extra = plane(3)
    channel_ids = [0, 1, 2, -1, -2, 3]
    channel_planes = [color[0], color[1], color[2], plane(4), plane(5), plane(6)]

    rec = bytearray()
    for v in (0, 0, HEIGHT, WIDTH):
        rec += struct.pack(">i", v)
    rec += struct.pack(">H", len(channel_ids))
    for cid, cp in zip(channel_ids, channel_planes):
        rec += struct.pack(">h", cid)
        rec += struct.pack(">I", 2 + len(cp))
    rec += b"8BIMnorm" + bytes([255, 0, 0, 0])
    mask = struct.pack(">I", 18) + struct.pack(">iiii", 0, 0, HEIGHT, WIDTH) + bytes([0, 0])
    extra_block = mask + struct.pack(">I", 0) + bytes([1, ord("L"), 0, 0])
    rec += struct.pack(">I", len(extra_block)) + extra_block

    info = bytearray(struct.pack(">h", 1) + rec)
    for cp in channel_planes:
        info += struct.pack(">H", 0) + cp
    while len(info) % 4:
        info += b"\x00"

    out = bytearray(b"8BPS" + struct.pack(">H", 1) + bytes(6))
    out += struct.pack(">H", 4)  # channels: three color + one extra
    out += struct.pack(">II", HEIGHT, WIDTH)
    out += struct.pack(">H", depth)
    out += struct.pack(">H", 3)  # RGB
    out += struct.pack(">I", 0)  # color mode data
    out += struct.pack(">I", 0)  # image resources
    out += struct.pack(">I", 4 + len(info) + 4)  # layer/mask section length
    out += struct.pack(">I", len(info))
    out += info
    out += struct.pack(">I", 0)  # global layer mask
    out += struct.pack(">H", 0)  # raw composite
    for cp in color + [extra]:
        out += cp
    return bytes(out)


def rgb16_layered() -> bytes:
    """Depth-16 RGB with a composite extra channel and a multi-channel layer."""
    return _depth_layered(16)


def rgb32_layered() -> bytes:
    """Depth-32 RGB with a composite extra channel and a multi-channel layer."""
    return _depth_layered(32)


def _grouped_psd(depth: int) -> bytes:
    """A depth-16/32 RGB PSD with a group of two pixel layers.

    Hand-authored because psd-tools cannot write 16/32-bit layer channels; the
    independent decoder still opens it (see `main`). A folder record carries
    placeholder channels (declared length 2, empty plane), so the read path must
    not keep them as source samples or a group would desync from its (empty)
    saved channels.
    """
    samples = DEPTH16_SAMPLES if depth == 16 else DEPTH32_SAMPLES

    def plane(shift: int) -> bytes:
        rotated = list(samples[shift:]) + list(samples[:shift])
        if depth == 16:
            row = b"".join(struct.pack(">H", v) for v in rotated)
        else:
            row = b"".join(struct.pack(">f", v) for v in rotated)
        return row * HEIGHT

    def record(name: str, rect, channels, section=None):
        rec = bytearray()
        for v in rect:
            rec += struct.pack(">i", v)
        rec += struct.pack(">H", len(channels))
        for cid, cp in channels:
            rec += struct.pack(">h", cid)
            rec += struct.pack(">I", 2 + len(cp))
        rec += b"8BIMnorm" + bytes([255, 0, 0, 0])
        extra = bytearray(struct.pack(">I", 0) + struct.pack(">I", 0))
        nb = name.encode()
        extra += bytes([len(nb)]) + nb
        while len(extra) % 4:
            extra += b"\x00"
        if section is not None:
            lsct = struct.pack(">I", section) + b"8BIMnorm"
            extra += b"8BIM" + b"lsct" + struct.pack(">I", len(lsct)) + lsct
        rec += struct.pack(">I", len(extra)) + extra
        return bytes(rec), channels

    inner = [(0, plane(0)), (1, plane(1)), (2, plane(2))]
    # Photoshop folder/divider records carry placeholder channels: a declared
    # length of 2 (the compression word only) on a zero-area rect. The read path
    # must not retain these as source samples. `Plain` carries an ordinary
    # `lsct=0` (SectionDivider::OTHER), which `build_tree` keeps, so it must keep
    # its retained samples.
    placeholders = [(-1, b""), (0, b""), (1, b""), (2, b"")]
    recs = [
        record("Plain", (0, 0, HEIGHT, WIDTH), inner, section=0),
        record("</Layer group>", (0, 0, 0, 0), placeholders, section=3),
        record("Inner Green", (0, 0, HEIGHT, WIDTH), inner),
        record("Inner Yellow", (0, 0, HEIGHT, WIDTH), inner),
        record("Group A", (0, 0, 0, 0), placeholders, section=1),
    ]

    info = bytearray(struct.pack(">h", len(recs)))
    for rec, _ in recs:
        info += rec
    for _, channels in recs:
        for _, cp in channels:
            info += struct.pack(">H", 0) + cp
    while len(info) % 4:
        info += b"\x00"

    out = bytearray(b"8BPS" + struct.pack(">H", 1) + bytes(6))
    out += struct.pack(">H", 3)
    out += struct.pack(">II", HEIGHT, WIDTH)
    out += struct.pack(">H", depth)
    out += struct.pack(">H", 3)  # RGB
    out += struct.pack(">I", 0) + struct.pack(">I", 0)
    out += struct.pack(">I", 4 + len(info) + 4)
    out += struct.pack(">I", len(info))
    out += info
    out += struct.pack(">I", 0)  # global layer mask
    out += struct.pack(">H", 0)  # raw composite
    for p in (plane(0), plane(1), plane(2)):
        out += p
    return bytes(out)


def rgb16_grouped() -> bytes:
    """Depth-16 RGB with a group of two pixel layers."""
    return _grouped_psd(16)


def rgb32_grouped() -> bytes:
    """Depth-32 RGB with a group of two pixel layers."""
    return _grouped_psd(32)


def _adj_layer(psd: PSDImage, key: Tag, name: str, data) -> None:
    """Turn a fresh empty pixel layer into an adjustment layer for `key`."""
    layer = psd.create_pixel_layer(Image.new("RGBA", (2, 2), (0, 0, 0, 0)), name=name)
    rec = layer._record
    layer._channels = ChannelDataList([])
    rec.channel_info = []
    rec.bottom = rec.top
    rec.right = rec.left
    rec.mask_data = None
    rec.tagged_blocks[Tag(key)] = TaggedBlock(key=Tag(key), data=data)


def image_resources() -> PSDImage:
    """RGB, a base layer plus EXIF (1058) and XMP (1060) image resources."""
    psd = PSDImage.new("RGB", (WIDTH, HEIGHT), color=(200, 100, 50))
    psd.create_pixel_layer(
        Image.new("RGBA", (WIDTH, HEIGHT), (200, 100, 50, 255)), name="Base"
    )
    psd._record.image_resources[Resource.EXIF_DATA_1] = ImageResource(
        key=Resource.EXIF_DATA_1, data=EXIF_BYTES
    )
    psd._record.image_resources[Resource.XMP_METADATA] = ImageResource(
        key=Resource.XMP_METADATA, data=XMP_BYTES
    )
    return psd


def icc_profile() -> PSDImage:
    """RGB, a base layer with an embedded synthesized Adobe-RGB ICC profile.

    The profile bytes are `pictura-color`'s `Profile::adobe_rgb().to_icc()`,
    regenerated by `cargo run -p pictura-color --example dump_adobe_rgb`.
    """
    profile = (FIXTURE_DIR / "psd_icc_rgb.icc").read_bytes()
    psd = PSDImage.new("RGB", (WIDTH, HEIGHT), color=(200, 100, 50))
    psd.create_pixel_layer(
        Image.new("RGBA", (WIDTH, HEIGHT), (200, 100, 50, 255)), name="Base"
    )
    psd._record.image_resources[Resource.ICC_PROFILE] = ImageResource(
        key=Resource.ICC_PROFILE, data=profile
    )
    return psd


def metadata() -> PSDImage:
    """RGB, a base layer plus a real EXIF IFD, an IPTC-IIM block, and XMP."""
    psd = PSDImage.new("RGB", (WIDTH, HEIGHT), color=(200, 100, 50))
    psd.create_pixel_layer(
        Image.new("RGBA", (WIDTH, HEIGHT), (200, 100, 50, 255)), name="Base"
    )
    psd._record.image_resources[Resource.EXIF_DATA_1] = ImageResource(
        key=Resource.EXIF_DATA_1, data=_metadata_exif()
    )
    psd._record.image_resources[Resource.IPTC_NAA] = ImageResource(
        key=Resource.IPTC_NAA, data=_metadata_iptc()
    )
    psd._record.image_resources[Resource.XMP_METADATA] = ImageResource(
        key=Resource.XMP_METADATA, data=METADATA_XMP
    )
    return psd


def adjustment() -> PSDImage:
    """RGB, a Base pixel layer plus the adjustment layers the renderer decodes."""
    psd = PSDImage.new("RGB", (WIDTH, HEIGHT), color=(200, 100, 50))
    psd.create_pixel_layer(
        Image.new("RGBA", (WIDTH, HEIGHT), (200, 100, 50, 255)), name="Base"
    )
    _adj_layer(psd, Tag.INVERT, "Invert", EmptyElement())
    _adj_layer(psd, Tag.POSTERIZE, "Posterize", ShortIntegerElement(4))
    _adj_layer(psd, Tag.THRESHOLD, "Threshold", ShortIntegerElement(128))
    _adj_layer(
        psd,
        Tag.BRIGHTNESS_AND_CONTRAST,
        "BrightnessContrast",
        BrightnessContrast(brightness=10, contrast=20, mean=0, lab_only=0),
    )
    recs = [LevelRecord(5, 250, 10, 240, 120)] * 29
    _adj_layer(
        psd, Tag.LEVELS, "Levels", Levels(version=2, extra_version=None, items=recs)
    )
    _adj_layer(
        psd,
        Tag.PHOTO_FILTER,
        "PhotoFilter",
        PhotoFilter(
            version=2,
            color_space=0,
            color_components=(255, 180, 80, 0),
            density=25,
            luminosity=1,
        ),
    )
    return psd


def _mixr_data(monochrome, red, green, blue, gray) -> bytes:
    """Hand-build an ag-psd-shaped `mixr` block.

    Each channel argument is a `(rgb, constant)` pair. Non-monochrome writes
    red/green/blue then the gray channel; monochrome writes only the gray
    channel followed by 30 zero bytes, so both forms are 44 bytes (the decoder
    ignores trailing bytes and only needs the channels it reads).
    """
    data = struct.pack(">HH", 1, 1 if monochrome else 0)
    channels = (gray,) if monochrome else (red, green, blue, gray)
    for rgb, constant in channels:
        data += struct.pack(">hhh", *rgb) + b"\x00\x00" + struct.pack(">h", constant)
    if monochrome:
        data += b"\x00" * 30
    return data


def channel_mixer() -> PSDImage:
    """RGB, a Base pixel layer plus non-monochrome and monochrome `mixr` layers."""
    psd = PSDImage.new("RGB", (WIDTH, HEIGHT), color=(200, 100, 50))
    psd.create_pixel_layer(
        Image.new("RGBA", (WIDTH, HEIGHT), (200, 100, 50, 255)), name="Base"
    )
    _adj_layer(
        psd,
        Tag.CHANNEL_MIXER,
        "Channel Mixer",
        _mixr_data(
            False,
            ((30, -10, 50), 5),
            ((10, 90, 0), -20),
            ((0, 20, 110), 40),
            ((100, 0, 0), 0),
        ),
    )
    _adj_layer(
        psd,
        Tag.CHANNEL_MIXER,
        "Channel Mixer Mono",
        _mixr_data(True, None, None, None, ((20, 40, 60), -15)),
    )
    return psd


def _curv_channel(points: list[tuple[int, int]]) -> bytes:
    """One `curv` channel: a `u16` node count then `(output, input)` nodes.

    `points` are model `(input, output)` pairs; the block stores them in the
    on-disk `(output, input)` order.
    """
    data = struct.pack(">H", len(points))
    for input_value, output_value in points:
        data += struct.pack(">hh", output_value, input_value)
    return data


def _curv_data(channels: list[tuple[int, list[tuple[int, int]]]]) -> bytes:
    """Hand-build an ag-psd-shaped `curv` block.

    `channels` is a list of `(bitmask_bit, model_points)` in the rgb/red/green/
    blue order. A version-4 duplicate `Crv ` section is appended so the
    decoder's ignore path is exercised; passing plain `bytes` as
    `TaggedBlock.data` stores the block verbatim.
    """
    bitmask = 0
    for bit, _ in channels:
        bitmask |= bit
    data = struct.pack(">BHHH", 0, 1, 0, bitmask)
    for _, points in channels:
        data += _curv_channel(points)
    index = {1: 0, 2: 1, 4: 2, 8: 3}
    data += b"Crv " + struct.pack(">HHH", 4, 0, len(channels))
    for bit, points in channels:
        data += struct.pack(">H", index[bit]) + _curv_channel(points)
    return data


def curves() -> PSDImage:
    """RGB, a Base pixel layer plus composite-only and per-channel `curv` layers."""
    psd = PSDImage.new("RGB", (WIDTH, HEIGHT), color=(200, 100, 50))
    psd.create_pixel_layer(
        Image.new("RGBA", (WIDTH, HEIGHT), (200, 100, 50, 255)), name="Base"
    )
    _adj_layer(
        psd,
        Tag.CURVES,
        "Curves",
        _curv_data([(1, [(0, 0), (64, 32), (192, 224), (255, 255)])]),
    )
    _adj_layer(
        psd,
        Tag.CURVES,
        "Curves Channels",
        _curv_data(
            [
                (1, [(0, 0), (255, 255)]),
                (2, [(0, 0), (128, 255), (255, 255)]),
                (4, [(0, 0), (64, 16), (255, 255)]),
                (8, [(0, 255), (255, 0)]),
            ]
        ),
    )
    return psd


def selective_color() -> PSDImage:
    """RGB, a Base layer plus relative and absolute `selc` layers.

    Each layer carries ten plates (cyan, magenta, yellow, black): a zero
    reserved plate 0 then the nine named ranges reds, yellows, greens, cyans,
    blues, magentas, whites, neutrals, blacks. psd-tools stores the
    ``SelectiveColor`` element verbatim.
    """
    psd = PSDImage.new("RGB", (WIDTH, HEIGHT), color=(200, 100, 50))
    psd.create_pixel_layer(
        Image.new("RGBA", (WIDTH, HEIGHT), (200, 100, 50, 255)), name="Base"
    )
    relative = [
        (0, 0, 0, 0),
        (10, -20, 30, 0),
        (0, 0, 0, 5),
        (-10, 0, 0, 0),
        (0, 15, 0, 0),
        (0, 0, -25, 0),
        (5, 0, 0, 0),
        (0, 0, 0, 0),
        (20, -10, 0, 0),
        (0, 0, 0, -40),
    ]
    _adj_layer(
        psd,
        Tag.SELECTIVE_COLOR,
        "Selective Color",
        SelectiveColor(version=1, method=0, data=relative),
    )
    absolute = [(0, 0, 0, 0)] + [
        tuple(4 * i + j for j in range(1, 5)) for i in range(9)
    ]
    _adj_layer(
        psd,
        Tag.SELECTIVE_COLOR,
        "Selective Color Abs",
        SelectiveColor(version=1, method=1, data=absolute),
    )
    return psd


def color_lookup() -> PSDImage:
    """RGB, a Base layer plus a `clrL` Color Lookup embedding an identity cube.

    psd-tools stores the ``ColorLookup`` descriptor verbatim, so the renderer
    decodes the same `lookupType`/`LUTFormat`/`LUT3DFileData` fields ag-psd
    reads.
    """
    psd = PSDImage.new("RGB", (WIDTH, HEIGHT), color=(200, 100, 50))
    psd.create_pixel_layer(
        Image.new("RGBA", (WIDTH, HEIGHT), (200, 100, 50, 255)), name="Base"
    )
    lookup = ColorLookup(version=1, data_version=16)
    lookup[b"lookupType"] = Enumerated(typeID=b"3DLUT", enum=b"3DLUT")
    lookup[b"Nm  "] = String("Identity.CUBE")
    lookup[b"Dthr"] = Bool(False)
    lookup[b"LUTFormat"] = Enumerated(
        typeID=b"LUTFormatCUBE", enum=b"LUTFormatCUBE"
    )
    lookup[b"dataOrder"] = Enumerated(typeID=b"rgbOrder", enum=b"rgbOrder")
    lookup[b"tableOrder"] = Enumerated(typeID=b"rgbOrder", enum=b"rgbOrder")
    lookup[b"LUT3DFileName"] = String("Identity.CUBE")
    lookup[b"LUT3DFileData"] = RawData(CUBE_IDENTITY)
    _adj_layer(psd, Tag.COLOR_LOOKUP, "Color Lookup", lookup)
    return psd


def gradient_map() -> PSDImage:
    """RGB, a Base pixel layer plus a black-to-white Gradient Map adjustment."""
    psd = PSDImage.new("RGB", (WIDTH, HEIGHT), color=(200, 100, 50))
    psd.create_pixel_layer(
        Image.new("RGBA", (WIDTH, HEIGHT), (200, 100, 50, 255)), name="Base"
    )
    _adj_layer(
        psd,
        Tag.GRADIENT_MAP,
        "Gradient Map",
        GradientMap(
            version=1,
            is_reversed=0,
            is_dithered=0,
            name="Black to White",
            color_stops=[
                ColorStop(location=0, midpoint=50, mode=0, color=(0, 0, 0, 0)),
                ColorStop(
                    location=4096,
                    midpoint=50,
                    mode=0,
                    color=(65535, 65535, 65535, 0),
                ),
            ],
            transparency_stops=[
                TransparencyStop(location=0, midpoint=50, opacity=255),
                TransparencyStop(location=4096, midpoint=50, opacity=255),
            ],
            expansion=2,
            interpolation=4096,
            length=32,
            mode=0,
            random_seed=0,
            show_transparency=0,
            use_vector_color=0,
            roughness=0,
            color_model=0,
            minimum_color=[0, 0, 0, 0],
            maximum_color=[0, 0, 0, 0],
        ),
    )
    return psd


def solid_fill() -> PSDImage:
    """RGB, a Base pixel layer plus a SoCo tagged fill layer.

    The tagged-block key is ``SoCo``, but psd-tools' ``DescriptorBlock`` keeps
    its own default class id, so the serialized top-level descriptor class is
    ``null``. pictura's encoder writes ``SoCo`` as the class id, which psd-tools
    accepts identically.
    """
    psd = PSDImage.new("RGB", (WIDTH, HEIGHT), color=(200, 100, 50))
    psd.create_pixel_layer(
        Image.new("RGBA", (WIDTH, HEIGHT), (200, 100, 50, 255)), name="Base"
    )
    data = DescriptorBlock(
        Descriptor(
            {
                b"Clr ": Descriptor(
                    {
                        b"Rd  ": Double(10.0),
                        b"Grn ": Double(20.0),
                        b"Bl  ": Double(30.0),
                    },
                    classID=b"RGBC",
                )
            },
            classID=b"SoCo",
        )
    )
    _adj_layer(psd, Tag.SOLID_COLOR_SHEET_SETTING, "Solid Fill", data)
    return psd


def _solid_color_block(rgb: tuple[int, int, int]) -> DescriptorBlock:
    return DescriptorBlock(
        Descriptor(
            {
                b"Clr ": Descriptor(
                    {
                        b"Rd  ": Double(float(rgb[0])),
                        b"Grn ": Double(float(rgb[1])),
                        b"Bl  ": Double(float(rgb[2])),
                    },
                    classID=b"RGBC",
                )
            },
            classID=b"SoCo",
        )
    )


def _shape_fill_layer(psd: PSDImage, name: str, rgb: tuple[int, int, int]):
    """A document-sized solid-color fill layer (`SoCo`, no pixel channels)."""
    layer = psd.create_pixel_layer(
        Image.new("RGBA", (2, 2), (0, 0, 0, 0)), name=name
    )
    rec = layer._record
    layer._channels = ChannelDataList([])
    rec.channel_info = []
    rec.top, rec.left, rec.bottom, rec.right = 0, 0, HEIGHT, WIDTH
    rec.mask_data = None
    rec.tagged_blocks[Tag.SOLID_COLOR_SHEET_SETTING] = TaggedBlock(
        key=Tag.SOLID_COLOR_SHEET_SETTING, data=_solid_color_block(rgb)
    )
    return layer


def _closed_rect_path(
    x0: int, y0: int, x1: int, y1: int, fill_rule: int = 1
) -> VectorPath:
    """A closed rectangle subpath in normalized 8.24 document coordinates."""
    corners = [(y0, x0), (y0, x1), (y1, x1), (y1, x0)]
    knots = [
        ClosedKnotLinked(
            preceding=(y / HEIGHT, x / WIDTH),
            anchor=(y / HEIGHT, x / WIDTH),
            leaving=(y / HEIGHT, x / WIDTH),
        )
        for (y, x) in corners
    ]
    return VectorPath(
        [ClosedPath(knots, operation=1, unknown1=fill_rule), PathFillRule()]
    )


def vector_mask() -> PSDImage:
    """RGB, a Base layer plus a SoCo fill layer clipped by a closed-rectangle
    ``vmsk``, and a second SoCo layer whose ``vmsk`` sets the invert flag.

    `Shape Inverted` shows outside ``(0,0)-(6,6)``; `Shape` shows inside
    ``(1,1)-(3,3)``, so compositing bottom-first proves the mask clips a fill
    layer and that the invert flag flips it.
    """
    psd = PSDImage.new("RGB", (WIDTH, HEIGHT), color=(200, 100, 50))
    psd.create_pixel_layer(
        Image.new("RGBA", (WIDTH, HEIGHT), (200, 100, 50, 255)), name="Base"
    )
    inverted = _shape_fill_layer(psd, "Shape Inverted", (0, 0, 255))
    inverted._record.tagged_blocks[Tag.VECTOR_MASK_SETTING1] = TaggedBlock(
        key=Tag.VECTOR_MASK_SETTING1,
        data=VectorMaskSetting(
            version=3, flags=0x01, path=_closed_rect_path(0, 0, 6, 6)
        ),
    )
    shape = _shape_fill_layer(psd, "Shape", (255, 0, 0))
    shape._record.tagged_blocks[Tag.VECTOR_MASK_SETTING1] = TaggedBlock(
        key=Tag.VECTOR_MASK_SETTING1,
        data=VectorMaskSetting(
            version=3, flags=0, path=_closed_rect_path(1, 1, 3, 3)
        ),
    )
    return psd


def vector_fill() -> PSDImage:
    """RGB, a Base layer plus a ``vscg`` solid-fill shape layer clipped by a
    closed-rectangle ``vmsk``.

    The ``Shape`` layer carries no pixel channels; its fill content lives in the
    additional-layer-info ``vscg`` block -- a 4-byte ``SoCo`` key followed by a
    version-16 descriptor -- and its ``vmsk`` is a closed ``(1,1)-(5,5)``
    rectangle. The inner ``Clr `` object must be ``RGBC`` or psd-tools' own
    compositor rejects it.
    """
    psd = PSDImage.new("RGB", (WIDTH, HEIGHT), color=(200, 100, 50))
    psd.create_pixel_layer(
        Image.new("RGBA", (WIDTH, HEIGHT), (200, 100, 50, 255)), name="Base"
    )
    shape = psd.create_pixel_layer(Image.new("RGBA", (2, 2), (0, 0, 0, 0)), name="Shape")
    rec = shape._record
    shape._channels = ChannelDataList([])
    rec.channel_info = []
    rec.top, rec.left, rec.bottom, rec.right = 0, 0, HEIGHT, WIDTH
    rec.mask_data = None
    rec.flags.pixel_data_irrelevant = True
    rec.tagged_blocks[Tag.VECTOR_STROKE_CONTENT_DATA] = TaggedBlock(
        key=Tag.VECTOR_STROKE_CONTENT_DATA,
        data=VectorStrokeContentSetting(
            key=b"SoCo",
            version=16,
            items={
                b"Clr ": Descriptor(
                    {
                        b"Rd  ": Double(255.0),
                        b"Grn ": Double(0.0),
                        b"Bl  ": Double(0.0),
                    },
                    classID=b"RGBC",
                )
            },
        ),
    )
    rec.tagged_blocks[Tag.VECTOR_MASK_SETTING1] = TaggedBlock(
        key=Tag.VECTOR_MASK_SETTING1,
        data=VectorMaskSetting(version=3, flags=0, path=_closed_rect_path(1, 1, 5, 5)),
    )
    return psd


def _gradient_stop(location: int, rgb: tuple[int, int, int]) -> Descriptor:
    """One custom gradient stop for a `GdFl` descriptor's `Clrs` list."""
    return Descriptor(
        {
            Key.Color: Descriptor(
                {
                    Key.Red: Double(float(rgb[0])),
                    Key.Green: Double(float(rgb[1])),
                    Key.Blue: Double(float(rgb[2])),
                },
                classID=b"RGBC",
            ),
            Key.Type: Enumerated(Type.ColorStopType, Enum.UserStop),
            Key.Location: Double(float(location)),
            Key.Midpoint: Double(50.0),
        },
        classID=b"Clrt",
    )


def gradient_fill() -> PSDImage:
    """RGB, a Base pixel layer plus a channel-stripped document-sized GdFl fill.

    The fill layer keeps a document-sized rect so psd-tools composites the ramp
    (a black-to-white 8-pixel row) rather than collapsing it to zero.
    """
    psd = PSDImage.new("RGB", (WIDTH, HEIGHT), color=(200, 100, 50))
    psd.create_pixel_layer(
        Image.new("RGBA", (WIDTH, HEIGHT), (200, 100, 50, 255)), name="Base"
    )
    layer = psd.create_pixel_layer(
        Image.new("RGBA", (WIDTH, HEIGHT), (0, 0, 0, 0)), name="Gradient Fill"
    )
    rec = layer._record
    layer._channels = ChannelDataList([])
    rec.channel_info = []
    rec.mask_data = None
    data = DescriptorBlock(
        Descriptor(
            {
                Key.Angle: Double(0.0),
                Key.Type: Enumerated(Type.GradientType, Enum.Linear),
                Key.Gradient: Descriptor(
                    {
                        Key.Name: String("Black to White"),
                        Type.GradientForm: Enumerated(
                            Type.GradientForm, Enum.CustomStops
                        ),
                        b"Intr": Enumerated(Type.Interpolation, b"Lnr "),
                        Key.Colors: List(
                            [
                                _gradient_stop(0, (0, 0, 0)),
                                _gradient_stop(4096, (255, 255, 255)),
                            ]
                        ),
                    },
                    classID=b"Grdn",
                ),
            },
            classID=b"GdFl",
        )
    )
    rec.tagged_blocks[Tag.GRADIENT_FILL_SETTING] = TaggedBlock(
        key=Tag.GRADIENT_FILL_SETTING, data=data
    )
    return psd


def _pattern_channel(data: list[int], depth: int = 8) -> VirtualMemoryArray:
    """One written 2x2 channel for the fixture pattern."""
    channel = VirtualMemoryArray()
    channel.set_data((2, 2), bytes(data), depth, 0)
    return channel


def _fixture_pattern() -> Pattern:
    """A 2x2 RGB pattern: red, green / blue, white, opaque.

    The channel list carries Photoshop's fixed slot layout: three written colour
    slots (``num_channels = 3``), then the two-slot alpha region, the last of
    which holds a written opaque plane. psd-tools' ``get_pattern_color_channels``
    reads the three leading written slots as colour and the trailing one as
    alpha, which is the boundary the Rust decoder follows.
    """
    return Pattern(
        version=1,
        image_mode=3,  # ColorMode.RGB
        point=(2, 2),
        name="Pictura",
        pattern_id="pictura-pattern",
        data=VirtualMemoryArrayList(
            version=3,
            rectangle=(0, 0, 2, 2),
            channels=[
                _pattern_channel([255, 0, 0, 255]),  # R: red, red
                _pattern_channel([0, 255, 0, 255]),  # G: green, white
                _pattern_channel([0, 0, 255, 255]),  # B: blue, white
                VirtualMemoryArray(),  # unwritten alpha slot
                _pattern_channel([255, 255, 255, 255]),  # transparency
            ],
        ),
    )


def _fixture_pattern_16bit() -> Pattern:
    """The same 2x2 RGB pattern with 16-bit channel planes.

    The decoder only reads 8-bit pattern planes, so this pattern is skipped and
    the fill renders the placeholder; the fixture proves a 16-bit payload is not
    misdecoded as 8-bit pixels.
    """
    return Pattern(
        version=1,
        image_mode=3,
        point=(2, 2),
        name="Pictura",
        pattern_id="pictura-pattern-16",
        data=VirtualMemoryArrayList(
            version=3,
            rectangle=(0, 0, 2, 2),
            channels=[
                _pattern_channel([255, 0, 0, 0, 0, 0, 0, 255], 16),
                _pattern_channel([0, 0, 255, 0, 0, 0, 0, 255], 16),
                _pattern_channel([0, 0, 0, 0, 255, 0, 0, 255], 16),
                VirtualMemoryArray(),
                _pattern_channel([255, 0, 255, 0, 255, 0, 255, 0], 16),
            ],
        ),
    )


def _pattern_fill(name: str, pattern: Pattern) -> PSDImage:
    """Base layer plus a document-sized PtFl fill referencing `pattern`.

    The pattern's pixels live in the global `Patt` tagged block
    (`Tag.PATTERNS1`), not in an image resource; the `PtFl` descriptor
    references the pattern by id. The fill layer keeps a document-sized rect so
    psd-tools composites the tile.
    """
    psd = PSDImage.new("RGB", (WIDTH, HEIGHT), color=(200, 100, 50))
    psd.create_pixel_layer(
        Image.new("RGBA", (WIDTH, HEIGHT), (200, 100, 50, 255)), name="Base"
    )
    layer = psd.create_pixel_layer(
        Image.new("RGBA", (WIDTH, HEIGHT), (0, 0, 0, 0)), name=name
    )
    rec = layer._record
    layer._channels = ChannelDataList([])
    rec.channel_info = []
    rec.mask_data = None
    data = DescriptorBlock(
        Descriptor(
            {
                b"Ptrn": Descriptor(
                    {
                        b"Nm  ": String("Pictura\x00"),
                        b"Idnt": String(pattern.pattern_id + "\x00"),
                    },
                    classID=b"Ptrn",
                ),
                b"Scl ": UnitFloat(100.0, Unit.Percent),
                b"Algn": Bool(True),
            },
            classID=b"PtFl",
        )
    )
    rec.tagged_blocks[Tag.PATTERN_FILL_SETTING] = TaggedBlock(
        key=Tag.PATTERN_FILL_SETTING, data=data
    )
    psd._record.layer_and_mask_information.tagged_blocks = TaggedBlocks()
    psd.tagged_blocks.set_data(Tag.PATTERNS1, [pattern])
    return psd


def pattern_fill() -> PSDImage:
    """RGB, a Base pixel layer plus a document-sized 8-bit PtFl pattern fill."""
    return _pattern_fill("Pattern Fill", _fixture_pattern())


def pattern_fill_16bit() -> PSDImage:
    """RGB, a Base layer plus a PtFl fill whose pattern planes are 16-bit.

    The decoder skips the 16-bit pattern, so the fill renders the placeholder.
    """
    return _pattern_fill("Pattern Fill 16", _fixture_pattern_16bit())


def drop_shadow() -> PSDImage:
    """RGB, a Base pixel layer plus a `DrSh` drop-shadow pixel layer.

    The effect is the standard object-based `lfx2` `DescriptorBlock2`: a
    top-level `masterFXSwitch` and a `DrSh` object whose keys mirror
    psd-tools' `DropShadow` accessors. `uglg` is off, so the authored local
    `lagl` of 120 is the effective angle.
    """
    psd = PSDImage.new("RGB", (WIDTH, HEIGHT), color=(200, 100, 50))
    psd.create_pixel_layer(
        Image.new("RGBA", (WIDTH, HEIGHT), (200, 100, 50, 255)), name="Base"
    )
    layer = psd.create_pixel_layer(
        Image.new("RGBA", (4, 4), (255, 0, 0, 255)), name="Shadowed", left=0, top=0
    )
    drsh = Descriptor(
        {
            Key.Enabled: Bool(True),
            b"present": Bool(True),
            b"showInDialog": Bool(True),
            Key.Mode: Enumerated(b"BlnM", b"Mltp"),
            Key.Color: Descriptor(
                {
                    b"Rd  ": Double(10.0),
                    b"Grn ": Double(20.0),
                    b"Bl  ": Double(30.0),
                },
                classID=b"RGBC",
            ),
            Key.Opacity: UnitFloat(75.0, Unit.Percent),
            Key.UseGlobalAngle: Bool(False),
            Key.LocalLightingAngle: UnitFloat(120.0, Unit.Angle),
            Key.Distance: UnitFloat(5.0, Unit.Pixels),
            Key.ChokeMatte: UnitFloat(0.0, Unit.Percent),
            Key.Blur: UnitFloat(5.0, Unit.Pixels),
            Key.TransferSpec: Descriptor(
                {Key.Name: String("Linear")}, classID=b"TrnS"
            ),
            Key.AntiAlias: Bool(True),
            Key.Noise: UnitFloat(0.0, Unit.Percent),
            b"layerConceals": Bool(False),
        },
        classID=b"DrSh",
    )
    layer._record.tagged_blocks[Tag.OBJECT_BASED_EFFECTS_LAYER_INFO] = TaggedBlock(
        key=Tag.OBJECT_BASED_EFFECTS_LAYER_INFO,
        data=DescriptorBlock2(
            {b"masterFXSwitch": Bool(True), b"DrSh": drsh}, classID=Klass.Null
        ),
    )
    return psd


def outer_glow() -> PSDImage:
    """RGB, a Base pixel layer plus an `OrGl` outer-glow pixel layer.

    The effect is the standard object-based `lfx2` `DescriptorBlock2`: a
    top-level `masterFXSwitch` and an `OrGl` object whose keys mirror psd-tools'
    `OuterGlow` accessors. The technique is `GlwT` `BETE`/`PrBL`, the spread is
    `Ckmt` 20, and `blur` (size) is 10.
    """
    psd = PSDImage.new("RGB", (WIDTH, HEIGHT), color=(200, 100, 50))
    psd.create_pixel_layer(
        Image.new("RGBA", (WIDTH, HEIGHT), (200, 100, 50, 255)), name="Base"
    )
    layer = psd.create_pixel_layer(
        Image.new("RGBA", (4, 4), (255, 0, 0, 255)), name="Glowing", left=0, top=0
    )
    orgl = Descriptor(
        {
            Key.Enabled: Bool(True),
            b"present": Bool(True),
            b"showInDialog": Bool(True),
            Key.Mode: Enumerated(b"BlnM", b"Scrn"),
            Key.Color: Descriptor(
                {
                    b"Rd  ": Double(40.0),
                    b"Grn ": Double(80.0),
                    b"Bl  ": Double(120.0),
                },
                classID=b"RGBC",
            ),
            Key.Opacity: UnitFloat(60.0, Unit.Percent),
            Key.GlowTechnique: Enumerated(b"BETE", b"PrBL"),
            Key.ChokeMatte: UnitFloat(20.0, Unit.Pixels),
            Key.Blur: UnitFloat(10.0, Unit.Pixels),
            Key.Noise: UnitFloat(0.0, Unit.Percent),
            Key.ShadingNoise: UnitFloat(0.0, Unit.Percent),
            Key.InputRange: UnitFloat(50.0, Unit.Percent),
            Key.AntiAlias: Bool(True),
            Key.TransferSpec: Descriptor(
                {Key.Name: String("Linear")}, classID=b"TrnS"
            ),
        },
        classID=b"OrGl",
    )
    layer._record.tagged_blocks[Tag.OBJECT_BASED_EFFECTS_LAYER_INFO] = TaggedBlock(
        key=Tag.OBJECT_BASED_EFFECTS_LAYER_INFO,
        data=DescriptorBlock2(
            {b"masterFXSwitch": Bool(True), b"OrGl": orgl}, classID=Klass.Null
        ),
    )
    return psd


def inner_shadow() -> PSDImage:
    """RGB, a Base pixel layer plus an `IrSh` inner-shadow pixel layer.

    The effect is the standard object-based `lfx2` `DescriptorBlock2`: a
    top-level `masterFXSwitch` and an `IrSh` object whose keys mirror psd-tools'
    `InnerShadow` accessors. `Ckmt` is the choke (an erode); `uglg` is off, so
    the authored local `lagl` of 120 is the effective angle. There is no
    `layerConceals` key: Inner Shadow has no knock-out control.
    """
    psd = PSDImage.new("RGB", (WIDTH, HEIGHT), color=(200, 100, 50))
    psd.create_pixel_layer(
        Image.new("RGBA", (WIDTH, HEIGHT), (200, 100, 50, 255)), name="Base"
    )
    layer = psd.create_pixel_layer(
        Image.new("RGBA", (4, 4), (255, 0, 0, 255)), name="Inner", left=0, top=0
    )
    irsh = Descriptor(
        {
            Key.Enabled: Bool(True),
            b"present": Bool(True),
            b"showInDialog": Bool(True),
            Key.Mode: Enumerated(b"BlnM", b"Mltp"),
            Key.Color: Descriptor(
                {
                    b"Rd  ": Double(10.0),
                    b"Grn ": Double(20.0),
                    b"Bl  ": Double(30.0),
                },
                classID=b"RGBC",
            ),
            Key.Opacity: UnitFloat(75.0, Unit.Percent),
            Key.UseGlobalAngle: Bool(False),
            Key.LocalLightingAngle: UnitFloat(120.0, Unit.Angle),
            Key.Distance: UnitFloat(5.0, Unit.Pixels),
            Key.ChokeMatte: UnitFloat(0.0, Unit.Percent),
            Key.Blur: UnitFloat(5.0, Unit.Pixels),
            Key.TransferSpec: Descriptor(
                {Key.Name: String("Linear")}, classID=b"TrnS"
            ),
            Key.AntiAlias: Bool(True),
            Key.Noise: UnitFloat(0.0, Unit.Percent),
        },
        classID=b"IrSh",
    )
    layer._record.tagged_blocks[Tag.OBJECT_BASED_EFFECTS_LAYER_INFO] = TaggedBlock(
        key=Tag.OBJECT_BASED_EFFECTS_LAYER_INFO,
        data=DescriptorBlock2(
            {b"masterFXSwitch": Bool(True), b"IrSh": irsh}, classID=Klass.Null
        ),
    )
    return psd


def inner_glow() -> PSDImage:
    """RGB, a Base pixel layer plus an `IrGl` inner-glow pixel layer.

    The effect is the standard object-based `lfx2` `DescriptorBlock2`: a
    top-level `masterFXSwitch` and an `IrGl` object whose keys mirror psd-tools'
    `InnerGlow` accessors. The source is `glwS` typeID `IGSr` (value `SrcE`); the
    technique is `GlwT` `BETE`/`SfBL`. `Ckmt` is the choke (an erode).
    """
    psd = PSDImage.new("RGB", (WIDTH, HEIGHT), color=(200, 100, 50))
    psd.create_pixel_layer(
        Image.new("RGBA", (WIDTH, HEIGHT), (200, 100, 50, 255)), name="Base"
    )
    layer = psd.create_pixel_layer(
        Image.new("RGBA", (4, 4), (255, 0, 0, 255)), name="Glow", left=0, top=0
    )
    irgl = Descriptor(
        {
            Key.Enabled: Bool(True),
            b"present": Bool(True),
            b"showInDialog": Bool(True),
            Key.Mode: Enumerated(b"BlnM", b"Scrn"),
            Key.Color: Descriptor(
                {
                    b"Rd  ": Double(255.0),
                    b"Grn ": Double(255.0),
                    b"Bl  ": Double(255.0),
                },
                classID=b"RGBC",
            ),
            Key.Opacity: UnitFloat(75.0, Unit.Percent),
            Key.GlowTechnique: Enumerated(b"BETE", b"SfBL"),
            Key.ChokeMatte: UnitFloat(0.0, Unit.Percent),
            Key.Blur: UnitFloat(5.0, Unit.Pixels),
            Key.InnerGlowSource: Enumerated(b"IGSr", b"SrcE"),
            Key.Noise: UnitFloat(0.0, Unit.Percent),
            Key.ShadingNoise: UnitFloat(0.0, Unit.Percent),
            Key.InputRange: UnitFloat(50.0, Unit.Percent),
            Key.AntiAlias: Bool(True),
            Key.TransferSpec: Descriptor(
                {Key.Name: String("Linear")}, classID=b"TrnS"
            ),
        },
        classID=b"IrGl",
    )
    layer._record.tagged_blocks[Tag.OBJECT_BASED_EFFECTS_LAYER_INFO] = TaggedBlock(
        key=Tag.OBJECT_BASED_EFFECTS_LAYER_INFO,
        data=DescriptorBlock2(
            {b"masterFXSwitch": Bool(True), b"IrGl": irgl}, classID=Klass.Null
        ),
    )
    return psd


def stroke() -> PSDImage:
    """RGB, a Base pixel layer plus an `FrFX` solid-colour stroke pixel layer.

    The effect is the standard object-based `lfx2` `DescriptorBlock2`: a
    top-level `masterFXSwitch` and an `FrFX` object whose keys mirror psd-tools'
    `Stroke` accessors. The position is `Styl` typeID `FStl` value `OutF`, the
    fill type is `PntT` typeID `FrFl` value `SClr`, and `Sz  ` is 3 px.
    """
    psd = PSDImage.new("RGB", (WIDTH, HEIGHT), color=(200, 100, 50))
    psd.create_pixel_layer(
        Image.new("RGBA", (WIDTH, HEIGHT), (200, 100, 50, 255)), name="Base"
    )
    layer = psd.create_pixel_layer(
        Image.new("RGBA", (4, 4), (255, 0, 0, 255)), name="Stroked", left=0, top=0
    )
    frfx = Descriptor(
        {
            Key.Enabled: Bool(True),
            b"present": Bool(True),
            b"showInDialog": Bool(True),
            Key.Mode: Enumerated(b"BlnM", b"Nrml"),
            Key.Color: Descriptor(
                {
                    b"Rd  ": Double(0.0),
                    b"Grn ": Double(0.0),
                    b"Bl  ": Double(0.0),
                },
                classID=b"RGBC",
            ),
            Key.Opacity: UnitFloat(100.0, Unit.Percent),
            Key.Style: Enumerated(Type.FrameStyle, Enum.OutsetFrame),
            Key.PaintType: Enumerated(Type.FrameFill, Enum.SolidColor),
            Key.SizeKey: UnitFloat(3.0, Unit.Pixels),
            b"overprint": Bool(False),
            Key.TransferSpec: Descriptor(
                {Key.Name: String("Linear")}, classID=b"TrnS"
            ),
            Key.AntiAlias: Bool(True),
        },
        classID=b"FrFX",
    )
    layer._record.tagged_blocks[Tag.OBJECT_BASED_EFFECTS_LAYER_INFO] = TaggedBlock(
        key=Tag.OBJECT_BASED_EFFECTS_LAYER_INFO,
        data=DescriptorBlock2(
            {b"masterFXSwitch": Bool(True), b"FrFX": frfx}, classID=Klass.Null
        ),
    )
    return psd


def stroke_gradient() -> PSDImage:
    """RGB, a Base pixel layer plus an `FrFX` gradient-fill stroke pixel layer.

    The `FrFX` object mirrors `stroke()` but sets `PntT` (`FrFl`) to
    `GradientFill` and carries the `Grad` `Grdn` content: a black-to-white
    `Clrs` list, `Angl` 45, `Type` `GrdT`/`Lnr `, `Rvrs` true, `Algn` true and
    `Scl ` 100. psd-tools reads it back as a `Stroke` with `fill_type` `GrFl`.
    """
    psd = PSDImage.new("RGB", (WIDTH, HEIGHT), color=(200, 100, 50))
    psd.create_pixel_layer(
        Image.new("RGBA", (WIDTH, HEIGHT), (200, 100, 50, 255)), name="Base"
    )
    layer = psd.create_pixel_layer(
        Image.new("RGBA", (4, 4), (255, 0, 0, 255)), name="Stroked", left=0, top=0
    )
    frfx = Descriptor(
        {
            Key.Enabled: Bool(True),
            b"present": Bool(True),
            b"showInDialog": Bool(True),
            Key.Mode: Enumerated(b"BlnM", b"Nrml"),
            Key.Opacity: UnitFloat(100.0, Unit.Percent),
            Key.Style: Enumerated(Type.FrameStyle, Enum.OutsetFrame),
            Key.PaintType: Enumerated(Type.FrameFill, Enum.GradientFill),
            Key.SizeKey: UnitFloat(3.0, Unit.Pixels),
            Key.Gradient: Descriptor(
                {
                    Key.Name: String("Black to White"),
                    Type.GradientForm: Enumerated(
                        Type.GradientForm, Enum.CustomStops
                    ),
                    b"Intr": Enumerated(Type.Interpolation, b"Lnr "),
                    Key.Colors: List(
                        [
                            _gradient_stop(0, (0, 0, 0)),
                            _gradient_stop(4096, (255, 255, 255)),
                        ]
                    ),
                },
                classID=b"Grdn",
            ),
            Key.Angle: Double(45.0),
            Key.Type: Enumerated(Type.GradientType, Enum.Linear),
            Key.Reverse: Bool(True),
            Key.Alignment: Bool(True),
            Key.Scale: UnitFloat(100.0, Unit.Percent),
            b"overprint": Bool(False),
            Key.AntiAlias: Bool(True),
        },
        classID=b"FrFX",
    )
    layer._record.tagged_blocks[Tag.OBJECT_BASED_EFFECTS_LAYER_INFO] = TaggedBlock(
        key=Tag.OBJECT_BASED_EFFECTS_LAYER_INFO,
        data=DescriptorBlock2(
            {b"masterFXSwitch": Bool(True), b"FrFX": frfx}, classID=Klass.Null
        ),
    )
    return psd


def stroke_pattern() -> PSDImage:
    """RGB, a Base pixel layer plus an `FrFX` pattern-fill stroke pixel layer.

    The `FrFX` object mirrors `stroke()` but sets `PntT` (`FrFl`) to `Pattern`
    and carries the `Ptrn` content (`Nm  `/`Idnt` `pictura-pattern`), `Scl `
    100, `Lnkd` false and `Angl` 0. The 2x2 fixture pattern is written to the
    global `Patt` block, so the renderer resolves the real tile. psd-tools reads
    it back as a `Stroke` with `fill_type` `Ptrn` and `linked` false.
    """
    psd = PSDImage.new("RGB", (WIDTH, HEIGHT), color=(200, 100, 50))
    psd.create_pixel_layer(
        Image.new("RGBA", (WIDTH, HEIGHT), (200, 100, 50, 255)), name="Base"
    )
    psd._record.layer_and_mask_information.tagged_blocks = TaggedBlocks()
    psd.tagged_blocks.set_data(Tag.PATTERNS1, [_fixture_pattern()])
    layer = psd.create_pixel_layer(
        Image.new("RGBA", (4, 4), (255, 0, 0, 255)), name="Stroked", left=0, top=0
    )
    frfx = Descriptor(
        {
            Key.Enabled: Bool(True),
            b"present": Bool(True),
            b"showInDialog": Bool(True),
            Key.Mode: Enumerated(b"BlnM", b"Nrml"),
            Key.Opacity: UnitFloat(100.0, Unit.Percent),
            Key.Style: Enumerated(Type.FrameStyle, Enum.OutsetFrame),
            Key.PaintType: Enumerated(Type.FrameFill, Enum.Pattern),
            Key.SizeKey: UnitFloat(3.0, Unit.Pixels),
            b"Ptrn": Descriptor(
                {
                    b"Nm  ": String("Pictura\x00"),
                    b"Idnt": String("pictura-pattern\x00"),
                },
                classID=b"Ptrn",
            ),
            Key.Scale: UnitFloat(100.0, Unit.Percent),
            b"Lnkd": Bool(False),
            Key.Angle: Double(0.0),
            b"overprint": Bool(False),
            Key.AntiAlias: Bool(True),
        },
        classID=b"FrFX",
    )
    layer._record.tagged_blocks[Tag.OBJECT_BASED_EFFECTS_LAYER_INFO] = TaggedBlock(
        key=Tag.OBJECT_BASED_EFFECTS_LAYER_INFO,
        data=DescriptorBlock2(
            {b"masterFXSwitch": Bool(True), b"FrFX": frfx}, classID=Klass.Null
        ),
    )
    return psd


def color_overlay() -> PSDImage:
    """RGB, a Base pixel layer plus a `SoFi` color-overlay pixel layer.

    The effect object class id is `SoFi` (the *effect* id), not `SoCo` (the
    solid-fill *layer* id). The keys mirror psd-tools' `ColorOverlay` accessors:
    `Md  ` (`BlnM`/`mul `), `Clr ` (`RGBC`), and `Opct` 75.
    """
    psd = PSDImage.new("RGB", (WIDTH, HEIGHT), color=(200, 100, 50))
    psd.create_pixel_layer(
        Image.new("RGBA", (WIDTH, HEIGHT), (200, 100, 50, 255)), name="Base"
    )
    layer = psd.create_pixel_layer(
        Image.new("RGBA", (4, 4), (255, 0, 0, 255)), name="Colored", left=0, top=0
    )
    sofi = Descriptor(
        {
            Key.Enabled: Bool(True),
            b"present": Bool(True),
            b"showInDialog": Bool(True),
            Key.Mode: Enumerated(b"BlnM", b"Mltp"),
            Key.Color: Descriptor(
                {
                    b"Rd  ": Double(10.0),
                    b"Grn ": Double(20.0),
                    b"Bl  ": Double(30.0),
                },
                classID=b"RGBC",
            ),
            Key.Opacity: UnitFloat(75.0, Unit.Percent),
        },
        classID=b"SoFi",
    )
    layer._record.tagged_blocks[Tag.OBJECT_BASED_EFFECTS_LAYER_INFO] = TaggedBlock(
        key=Tag.OBJECT_BASED_EFFECTS_LAYER_INFO,
        data=DescriptorBlock2(
            {b"masterFXSwitch": Bool(True), b"SoFi": sofi}, classID=Klass.Null
        ),
    )
    return psd


def gradient_overlay() -> PSDImage:
    """RGB, a Base pixel layer plus a `GrFl` gradient-overlay pixel layer.

    The effect object class id is `GrFl` (the *effect* id), not `GdFl` (the
    gradient-fill *layer* id). The keys mirror psd-tools' `GradientOverlay`
    accessors; `Ofst` and `Dthr` are authored but ignored by the renderer.
    """
    psd = PSDImage.new("RGB", (WIDTH, HEIGHT), color=(200, 100, 50))
    psd.create_pixel_layer(
        Image.new("RGBA", (WIDTH, HEIGHT), (200, 100, 50, 255)), name="Base"
    )
    layer = psd.create_pixel_layer(
        Image.new("RGBA", (4, 4), (255, 0, 0, 255)), name="Gradient", left=0, top=0
    )
    grfl = Descriptor(
        {
            Key.Enabled: Bool(True),
            b"present": Bool(True),
            b"showInDialog": Bool(True),
            Key.Mode: Enumerated(b"BlnM", b"Mltp"),
            Key.Opacity: UnitFloat(80.0, Unit.Percent),
            Key.Gradient: Descriptor(
                {
                    Key.Name: String("Black to White"),
                    Type.GradientForm: Enumerated(
                        Type.GradientForm, Enum.CustomStops
                    ),
                    b"Intr": Enumerated(Type.Interpolation, b"Lnr "),
                    Key.Colors: List(
                        [
                            _gradient_stop(0, (0, 0, 0)),
                            _gradient_stop(4096, (255, 255, 255)),
                        ]
                    ),
                },
                classID=b"Grdn",
            ),
            Key.Angle: Double(45.0),
            Key.Type: Enumerated(Type.GradientType, Enum.Linear),
            Key.Reverse: Bool(True),
            Key.Alignment: Bool(False),
            Key.Scale: UnitFloat(150.0, Unit.Percent),
            Key.Offset: Descriptor(
                {b"Hrzn": Double(0.0), b"Vrtc": Double(0.0)}, classID=b"Pnt "
            ),
            Key.Dither: Bool(False),
        },
        classID=b"GrFl",
    )
    layer._record.tagged_blocks[Tag.OBJECT_BASED_EFFECTS_LAYER_INFO] = TaggedBlock(
        key=Tag.OBJECT_BASED_EFFECTS_LAYER_INFO,
        data=DescriptorBlock2(
            {b"masterFXSwitch": Bool(True), b"GrFl": grfl}, classID=Klass.Null
        ),
    )
    return psd


def pattern_overlay() -> PSDImage:
    """RGB, a Base pixel layer plus a `patternFill` overlay pixel layer.

    The effect object class id is `patternFill` (the *effect* id), not `PtFl`
    (the pattern-fill *layer* id). The 2x2 fixture pattern is written to the
    global `Patt` block, so the renderer resolves the real tile. The keys mirror
    psd-tools' `PatternOverlay` accessors; `Algn` is "Link With Layer".
    """
    psd = PSDImage.new("RGB", (WIDTH, HEIGHT), color=(200, 100, 50))
    psd.create_pixel_layer(
        Image.new("RGBA", (WIDTH, HEIGHT), (200, 100, 50, 255)), name="Base"
    )
    psd._record.layer_and_mask_information.tagged_blocks = TaggedBlocks()
    psd.tagged_blocks.set_data(Tag.PATTERNS1, [_fixture_pattern()])
    layer = psd.create_pixel_layer(
        Image.new("RGBA", (4, 4), (0, 0, 0, 255)), name="Patterned", left=0, top=0
    )
    pf = Descriptor(
        {
            Key.Enabled: Bool(True),
            b"present": Bool(True),
            b"showInDialog": Bool(True),
            Key.Mode: Enumerated(b"BlnM", b"Scrn"),
            Key.Opacity: UnitFloat(80.0, Unit.Percent),
            b"Ptrn": Descriptor(
                {
                    b"Nm  ": String("Pictura\x00"),
                    b"Idnt": String("pictura-pattern\x00"),
                },
                classID=b"Ptrn",
            ),
            Key.Scale: UnitFloat(50.0, Unit.Percent),
            Key.Alignment: Bool(True),
            Key.Angle: Double(30.0),
        },
        classID=b"patternFill",
    )
    layer._record.tagged_blocks[Tag.OBJECT_BASED_EFFECTS_LAYER_INFO] = TaggedBlock(
        key=Tag.OBJECT_BASED_EFFECTS_LAYER_INFO,
        data=DescriptorBlock2(
            {b"masterFXSwitch": Bool(True), b"patternFill": pf}, classID=Klass.Null
        ),
    )
    return psd


def satin() -> PSDImage:
    """RGB, a Base pixel layer plus a `ChFX` satin pixel layer.

    The effect object class id and top-level key are both `ChFX` (ChromeFX);
    psd-tools registers it as `Satin`. `Invr` is the invert key and `MpgS`
    (not `TrnS`) is the contour key. Every decoded key is non-default so the
    oracle proves it survives.
    """
    psd = PSDImage.new("RGB", (WIDTH, HEIGHT), color=(200, 100, 50))
    psd.create_pixel_layer(
        Image.new("RGBA", (WIDTH, HEIGHT), (200, 100, 50, 255)), name="Base"
    )
    layer = psd.create_pixel_layer(
        Image.new("RGBA", (4, 4), (255, 0, 0, 255)), name="Satin", left=0, top=0
    )
    chfx = Descriptor(
        {
            Key.Enabled: Bool(True),
            b"present": Bool(True),
            b"showInDialog": Bool(True),
            Key.Mode: Enumerated(b"BlnM", b"Mltp"),
            Key.Color: Descriptor(
                {
                    b"Rd  ": Double(10.0),
                    b"Grn ": Double(20.0),
                    b"Bl  ": Double(30.0),
                },
                classID=b"RGBC",
            ),
            Key.Opacity: UnitFloat(50.0, Unit.Percent),
            b"uglg": Bool(False),
            Key.LocalLightingAngle: UnitFloat(120.0, Unit.Angle),
            Key.Distance: UnitFloat(8.0, Unit.Pixels),
            Key.Blur: UnitFloat(6.0, Unit.Pixels),
            Key.Invert: Bool(True),
            Key.AntiAlias: Bool(True),
            Key.MappingShape: Descriptor(
                {Key.Name: String("Linear")}, classID=b"TrnS"
            ),
        },
        classID=b"ChFX",
    )
    layer._record.tagged_blocks[Tag.OBJECT_BASED_EFFECTS_LAYER_INFO] = TaggedBlock(
        key=Tag.OBJECT_BASED_EFFECTS_LAYER_INFO,
        data=DescriptorBlock2(
            {b"masterFXSwitch": Bool(True), b"ChFX": chfx}, classID=Klass.Null
        ),
    )
    return psd


def bevel() -> PSDImage:
    """RGB, a Base pixel layer plus an `ebbl` bevel & emboss pixel layer.

    The effect object class id and the top-level key are both `ebbl`. The
    highlight keys are `hglM`/`hglC`/`hglO` and the shadow keys are
    `sdwM`/`sdwC`/`sdwO` (not `sglm`); the outer style is `OtrB` and the
    directions are `In  `/`Out `. `uglg` is off, so the authored local `lagl`
    of 120 is the effective angle. Every decoded key is non-default so the
    oracle proves it survives.
    """
    psd = PSDImage.new("RGB", (WIDTH, HEIGHT), color=(200, 100, 50))
    psd.create_pixel_layer(
        Image.new("RGBA", (WIDTH, HEIGHT), (200, 100, 50, 255)), name="Base"
    )
    layer = psd.create_pixel_layer(
        Image.new("RGBA", (4, 4), (255, 0, 0, 255)), name="Beveled", left=0, top=0
    )
    ebbl = Descriptor(
        {
            Key.Enabled: Bool(True),
            b"present": Bool(True),
            b"showInDialog": Bool(True),
            Key.HighlightMode: Enumerated(b"BlnM", b"Scrn"),
            Key.HighlightColor: Descriptor(
                {
                    b"Rd  ": Double(250.0),
                    b"Grn ": Double(240.0),
                    b"Bl  ": Double(230.0),
                },
                classID=b"RGBC",
            ),
            Key.HighlightOpacity: UnitFloat(80.0, Unit.Percent),
            Key.ShadowMode: Enumerated(b"BlnM", b"Mltp"),
            Key.ShadowColor: Descriptor(
                {
                    b"Rd  ": Double(10.0),
                    b"Grn ": Double(20.0),
                    b"Bl  ": Double(30.0),
                },
                classID=b"RGBC",
            ),
            Key.ShadowOpacity: UnitFloat(70.0, Unit.Percent),
            Key.BevelStyle: Enumerated(b"BESl", b"InrB"),
            Key.BevelTechnique: Enumerated(b"bvlT", b"SfBL"),
            Key.BevelDirection: Enumerated(b"BESs", b"In  "),
            b"uglg": Bool(False),
            Key.LocalLightingAngle: UnitFloat(120.0, Unit.Angle),
            Key.LocalLightingAltitude: UnitFloat(30.0, Unit.Angle),
            Key.StrengthRatio: UnitFloat(250.0, Unit.Percent),
            Key.Blur: UnitFloat(7.0, Unit.Pixels),
            Key.Softness: UnitFloat(3.0, Unit.Pixels),
            Key.TransferSpec: Descriptor(
                {Key.Name: String("Linear")}, classID=b"TrnS"
            ),
            Key.MappingShape: Descriptor(
                {Key.Name: String("Linear")}, classID=b"TrnS"
            ),
            Key.InputRange: UnitFloat(50.0, Unit.Percent),
            Key.AntiAlias: Bool(True),
            b"useShape": Bool(False),
            b"useTexture": Bool(False),
            b"antialiasGloss": Bool(True),
        },
        classID=b"ebbl",
    )
    layer._record.tagged_blocks[Tag.OBJECT_BASED_EFFECTS_LAYER_INFO] = TaggedBlock(
        key=Tag.OBJECT_BASED_EFFECTS_LAYER_INFO,
        data=DescriptorBlock2(
            {b"masterFXSwitch": Bool(True), b"ebbl": ebbl}, classID=Klass.Null
        ),
    )
    return psd


def legacy_effects() -> PSDImage:
    """RGB, a Base pixel layer plus a `Legacy` layer with an `lrFX` block.

    The legacy (`EFFECTS_LAYER`) block is the fixed `EffectsLayer` binary
    struct, not a descriptor: a `cmnS` common state (visible 1), a `dsdw` drop
    shadow (blur 5, intensity 0, angle 120, distance 5, blend `mul `, opacity
    255) and an `oglw` outer glow (blur 6, intensity 0, blend `scrn`, opacity
    191). The colours are 16-bit `Color` values authored as `v << 8` so the
    decoder's high-byte mapping yields `v`.
    """
    psd = PSDImage.new("RGB", (WIDTH, HEIGHT), color=(200, 100, 50))
    psd.create_pixel_layer(
        Image.new("RGBA", (WIDTH, HEIGHT), (200, 100, 50, 255)), name="Base"
    )
    layer = psd.create_pixel_layer(
        Image.new("RGBA", (4, 4), (255, 0, 0, 255)), name="Legacy", left=0, top=0
    )

    def color(rgb: tuple[int, int, int]) -> Color:
        return Color(ColorSpaceID.RGB, [c << 8 for c in rgb] + [0])

    effects = EffectsLayer(
        version=0,
        items=[
            (EffectOSType.COMMON_STATE, CommonStateInfo(version=0, visible=1)),
            (
                EffectOSType.DROP_SHADOW,
                ShadowInfo(
                    version=0,
                    blur=5,
                    intensity=0,
                    angle=120,
                    distance=5,
                    color=color((10, 20, 30)),
                    blend_mode=BlendMode.MULTIPLY,
                    enabled=1,
                    use_global_angle=0,
                    opacity=255,
                    native_color=color((0, 0, 0)),
                ),
            ),
            (
                EffectOSType.OUTER_GLOW,
                OuterGlowInfo(
                    version=0,
                    blur=6,
                    intensity=0,
                    color=color((40, 80, 120)),
                    blend_mode=BlendMode.SCREEN,
                    enabled=1,
                    opacity=191,
                ),
            ),
        ],
    )
    layer._record.tagged_blocks[Tag.EFFECTS_LAYER] = TaggedBlock(
        key=Tag.EFFECTS_LAYER, data=effects
    )
    return psd


def _engine_string(value: str) -> bytes:
    """A psd-tools EngineData string: a parenthesised UTF-16BE run."""
    return b"(\xfe\xff" + value.encode("utf-16-be") + b")"


def engine_data() -> bytes:
    """A minimal synthetic EngineData blob, canonicalised by psd-tools.

    The oracle (`tests/oracle/type_engine_data.rs`) parses the same bytes with
    both `pictura-codec` and psd-tools and expects the font set
    (`AdobeInvisFont`, `MyriadPro-Regular`), the first run's resolved font
    (`MyriadPro-Regular`), size 150.0, fill [1,1,1,1], tracking 0.0,
    justification 0, and the text "hello world\\r".
    """
    text = b"".join(
        [
            b"<< /EngineDict << ",
            b"/Editor << /Text ",
            _engine_string("hello world\r"),
            b" >> ",
            b"/StyleRun << /RunArray [ << /StyleSheet << /StyleSheetData << ",
            b"/FontSize 150.0 /FillColor << /Values [ 1.0 1.0 1.0 1.0 ] >> ",
            b">> >> >> ] >> ",
            b"/ParagraphRun << /RunArray [ << /ParagraphSheet << /Properties << ",
            b"/Justification 0 >> >> >> ] >> ",
            b">> /ResourceDict << ",
            b"/FontSet [ << /Name ",
            _engine_string("AdobeInvisFont"),
            b" >> << /Name ",
            _engine_string("MyriadPro-Regular"),
            b" >> ] ",
            b"/StyleSheetSet [ << /StyleSheetData << /Font 1 /FontSize 12.0 >> >> ] ",
            b">> >>",
        ]
    )
    return EngineData.frombytes(text).tobytes()


FIXTURES = {
    "two_layers.psd": two_layers,
    "knockout.psd": knockout,
    "knockout_group.psd": knockout_group,
    "knockout_isolated_group.psd": knockout_isolated_group,
    "knockout_shallow_group.psd": knockout_shallow_group,
    "image_resources.psd": image_resources,
    "icc_profile.psd": icc_profile,
    "metadata.psd": metadata,
    "group.psd": group,
    "pass_through_vector_mask.psd": pass_through_vector_mask,
    "masked.psd": masked,
    "gray.psd": gray,
    "indexed.psd": indexed,
    "cmyk.psd": cmyk,
    "lab.psd": lab,
    "bitmap.psd": bitmap,
    "rgb16.psd": rgb16,
    "rgb32.psd": rgb32,
    "rgb16_layered.psd": rgb16_layered,
    "rgb32_layered.psd": rgb32_layered,
    "rgb16_grouped.psd": rgb16_grouped,
    "rgb32_grouped.psd": rgb32_grouped,
    "adjustment.psd": adjustment,
    "channel_mixer.psd": channel_mixer,
    "curves.psd": curves,
    "selective_color.psd": selective_color,
    "color_lookup.psd": color_lookup,
    "gradient_map.psd": gradient_map,
    "solid_fill.psd": solid_fill,
    "vector_mask.psd": vector_mask,
    "vector_fill.psd": vector_fill,
    "gradient_fill.psd": gradient_fill,
    "pattern_fill.psd": pattern_fill,
    "pattern_fill_16bit.psd": pattern_fill_16bit,
    "drop_shadow.psd": drop_shadow,
    "outer_glow.psd": outer_glow,
    "inner_shadow.psd": inner_shadow,
    "inner_glow.psd": inner_glow,
    "stroke.psd": stroke,
    "stroke_gradient.psd": stroke_gradient,
    "stroke_pattern.psd": stroke_pattern,
    "color_overlay.psd": color_overlay,
    "gradient_overlay.psd": gradient_overlay,
    "pattern_overlay.psd": pattern_overlay,
    "satin.psd": satin,
    "bevel.psd": bevel,
    "legacy_effects.psd": legacy_effects,
    "engine_data.bin": engine_data,
}


def _dump_tree(layer, indent: int = 0) -> None:
    kind = "group" if layer.is_group() else "pixel"
    mask = " +mask" if layer.has_mask() else ""
    print(f"    {'  ' * indent}{layer.name!r} [{kind}] bbox={layer.bbox}{mask}")
    if layer.is_group():
        for child in layer:
            _dump_tree(child, indent + 1)


def main() -> None:
    FIXTURE_DIR.mkdir(parents=True, exist_ok=True)
    for name, build in FIXTURES.items():
        built = build()
        if isinstance(built, (bytes, bytearray)):
            data = bytes(built)
        else:
            buffer = io.BytesIO()
            built.save(buffer)
            data = buffer.getvalue()
        path = FIXTURE_DIR / name
        path.write_bytes(data)

        if not name.endswith(".psd"):
            print(f"wrote {path.relative_to(ROOT)} ({len(data)} bytes)")
            continue
        reopened = PSDImage.open(io.BytesIO(data))
        print(f"wrote {path.relative_to(ROOT)} ({len(data)} bytes, "
              f"{reopened.width}x{reopened.height} mode={reopened.color_mode})")
        for layer in reopened:
            _dump_tree(layer)


if __name__ == "__main__":
    main()
