#pragma once

#include "rust/cxx.h"

#include <cstdint>

// Encode tightly packed RGBA8888 of `width` × `height` to `path` with the named
// Qt image writer (`PNG`, `JPG`, `TIF`, `BMP`). `scale` is a percentage
// (100 = unchanged); `quality` is 0–100 for lossy formats and ignored otherwise;
// `compression` is the writer's format-specific level (-1 = the writer default);
// `optimized` and `progressive` apply to JPEG. `matte` is a packed 0xRRGGBB
// background to composite transparency onto, or -1 to keep alpha (JPEG and BMP
// always flatten, onto `matte` else opaque white). Returns false and writes
// nothing when the buffer size, path, format, or writer is unusable.
// WebP is not handled here: its Qt writer is an optional plugin, so the app
// encodes it through the engine (`pictura_codec::encode_webp`).
// Defined at global scope because the cxx bridge declares no namespace.
bool encode_image_rgba(::rust::Slice<const uint8_t> rgba, int32_t width, int32_t height,
                       ::rust::Str path, ::rust::Str format, int32_t quality, int32_t scale,
                       int32_t compression, bool optimized, bool progressive, int32_t matte);

// The same encoding as `encode_image_rgba`, returned as bytes rather than
// written to a path. Used by the JPEG Options dialog's size estimate and
// preview; empty on any refusal.
::rust::Vec<uint8_t> encode_image_bytes(::rust::Slice<const uint8_t> rgba, int32_t width,
                                        int32_t height, ::rust::Str format, int32_t quality,
                                        int32_t scale, int32_t compression, bool optimized,
                                        bool progressive, int32_t matte);

// Rescale tightly packed RGBA8888 of `width` × `height` by `scale` percent with
// a smooth filter and return the rescaled buffer, reporting the new size in
// `out_width`/`out_height`. A scale of 100 (or below 1) returns a copy at the
// input size. Used by the engine WebP encode path so the boundary keeps the
// percent-scale contract the Qt formats already have.
::rust::Vec<uint8_t> scale_rgba(::rust::Slice<const uint8_t> rgba, int32_t width, int32_t height,
                                int32_t scale, int32_t& out_width, int32_t& out_height);

// Encode interleaved 8-bit samples of `width` × `height` with `components`
// 1 (gray), 3 (RGB), or 4 (CMYK, 0 = no ink) as a JPEG at `quality` 0–100 and
// return the bytes; empty on any refusal. The Photoshop PDF writer's DCT
// images come from here so the engine carries no JPEG encoder.
::rust::Vec<uint8_t> encode_jpeg(::rust::Slice<const uint8_t> samples, int32_t width,
                                 int32_t height, int32_t components, int32_t quality);
