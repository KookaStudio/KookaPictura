#pragma once

#include <QtCore/QByteArray>
#include <QtCore/QString>

namespace pictura {

// The sfnt of the face Qt resolves for `family`, rebuilt from its tables
// (QRawFont exposes tables, not the file); empty when Qt has none.
QByteArray typeFontBytes(const QString& family);

// Hand `family`'s face to the engine (`type_api::registerFont`) once per process,
// so type set in it renders in it rather than in the bundled face.
void registerTypeFont(const QString& family);

// The installed family a type layer's font name means: the name itself when
// Qt knows it, else the family whose spaceless name is a PostScript name's
// stem (`NotoSansMono-Bold` -> `Noto Sans Mono`); the name when neither.
QString familyForFontName(const QString& name);

} // namespace pictura
