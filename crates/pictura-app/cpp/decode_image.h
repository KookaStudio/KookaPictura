#pragma once

#include "rust/cxx.h"

#include <QtCore/QByteArray>

#include <cstdint>

// Decode `data` with Qt to tightly packed RGBA8888 and report the decoded size.
// Returns an empty buffer (and zeroes `width`/`height`) when Qt cannot read it.
// Defined at global scope because the cxx bridge declares no namespace.
::rust::Vec<uint8_t> decode_image_rgba(::rust::Slice<const uint8_t> data, int32_t& width,
                                       int32_t& height);

namespace pictura {

// The Qt format name pinned for a signature the engine probe sniffs (PNG, JPEG,
// GIF, BMP, TIFF), or an empty array when the signature is unknown and every
// installed plugin may be probed.
QByteArray qtFormatForSignature(const QByteArray& head);

} // namespace pictura
