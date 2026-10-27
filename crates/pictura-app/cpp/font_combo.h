#pragma once

class QFontComboBox;
class QString;

namespace pictura {

// Whether `family` resolves to a color face: one carrying COLR, CBDT, sbix, or
// SVG glyphs. Emoji faces are color faces, and so are Latin color fonts.
bool isColorFont(const QString& family);

// List `combo`'s families as CS6 does: each name in the UI font, then a short
// sample in the face. A thin or dotted face would make its own name faint.
// Color faces get no sample: previewing one costs ~400 ms per repaint and draws
// garbage, so a list scrolled near one stays blank for seconds.
void useFastFontPreviews(QFontComboBox* combo);

} // namespace pictura
