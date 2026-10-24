#pragma once

#include <QtCore/QByteArray>
#include <QtCore/QString>
#include <QtCore/QStringList>

namespace pictura {

// The sfnt of the face Qt resolves for `family` and `style`, rebuilt from its
// tables (QRawFont exposes tables, not the file); empty when Qt has none. An
// empty style resolves the family's regular face.
QByteArray typeFontBytes(const QString& family, const QString& style);

// Hand `family`/`style`'s face to the engine (`type_api::registerFont`) once per
// process, so type set in it renders in it rather than in the bundled face.
void registerTypeFont(const QString& family, const QString& style = QString());

// Register the bundled UI face and one installed family per common non-Roman
// script once, so Missing Glyph Protection has a covering face to choose.
void registerTypeFallbacks();

// The installed family a type layer's font name means: the name itself when
// Qt knows it, else the family whose spaceless name is a PostScript name's
// stem (`NotoSansMono-Bold` -> `Noto Sans Mono`); the name when neither.
QString familyForFontName(const QString& name);

// Whether a type layer's font name is available: the engine's bundled
// Liberation Sans, an installed family, or a PostScript name whose stem is an
// installed family's spaceless name (an `MT`/`PSMT`/`PS` suffix ignored, so
// `ArialMT` is Arial). Not QRawFont: fontconfig substitutes any family.
bool fontIsInstalled(const QString& name);

// The names in `names` that are not installed, each once, in order.
QStringList missingFonts(const QStringList& names);

} // namespace pictura
