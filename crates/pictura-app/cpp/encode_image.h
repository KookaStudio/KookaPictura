#pragma once

#include "rust/cxx.h"

#include <cstdint>

// Encode tightly packed RGBA8888 of `width` × `height` to `path` with the named
// Qt image writer (`PNG`, `JPG`, `TIF`, `BMP`). `scale` is a percentage
// (100 = unchanged); `quality` is 0–100 for lossy formats and ignored otherwise.
// Formats without alpha (JPEG) are flattened onto opaque white. Returns false
// and writes nothing when the buffer size, path, format, or writer is unusable.
// WebP is not handled here: its Qt writer is an optional plugin, so the app
// encodes it through the engine (`pictura_codec::encode_webp`).
// Defined at global scope because the cxx bridge declares no namespace.
bool encode_image_rgba(::rust::Slice<const uint8_t> rgba, int32_t width, int32_t height,
                       ::rust::Str path, ::rust::Str format, int32_t quality, int32_t scale);

// Rescale tightly packed RGBA8888 of `width` × `height` by `scale` percent with
// a smooth filter and return the rescaled buffer, reporting the new size in
// `out_width`/`out_height`. A scale of 100 (or below 1) returns a copy at the
// input size. Used by the engine WebP encode path so the boundary keeps the
// percent-scale contract the Qt formats already have.
::rust::Vec<uint8_t> scale_rgba(::rust::Slice<const uint8_t> rgba, int32_t width, int32_t height,
                                int32_t scale, int32_t& out_width, int32_t& out_height);
