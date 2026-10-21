#pragma once

#include <QtCore/QByteArray>
#include <QtCore/QString>
#include <QtCore/QVector>

#include <rust/cxx.h>

#include <cstdint>

#include "pictura_app/src/cxxqt_object/type_tools.cxxqt.h"

namespace pictura {
class PictureView;
}

namespace pictura::type_api {

using pictura::CharacterSetting;
using pictura::ParagraphSetting;
using pictura::TypeSetting;

/// A type layer's editable state, read in one bundle: the five-call sequence
/// the Type tool and the panels used to run on every open/refresh.
struct LayerType {
    bool isType = false;
    QString text;
    QString font;
    TypeSetting setting;
    CharacterSetting character;
    ParagraphSetting paragraph;
};

bool isType(const PictureView& view, const QString& path);
QString layerAt(const PictureView& view, double x, double y);
QString layerText(const PictureView& view, const QString& path);
QString layerFont(const PictureView& view, const QString& path);
TypeSetting layerSetting(const PictureView& view, const QString& path);
CharacterSetting layerCharacterSetting(const PictureView& view, const QString& path);
ParagraphSetting layerParagraphSetting(const PictureView& view, const QString& path);
/// Whether `path` is a type layer plus its text/font/setting/character/paragraph
/// in one bundle; defaults with `isType` false for a non-type layer. The Type
/// tool and the panels used to run these five crossings per open/refresh.
LayerType readLayer(const PictureView& view, const QString& path);

CharacterSetting defaultCharacterSetting();
ParagraphSetting defaultParagraphSetting();
bool registerFont(const QString& family, ::rust::Slice<const std::uint8_t> bytes);
/// The Character panel's OpenType features `family`'s face carries, one bit
/// each in panel order (bit 0 Standard Ligatures … bit 7 Fractions).
std::uint32_t openTypeFeatures(const QString& family);

QVector<int> previewRect(const QString& text, const QString& font, const TypeSetting& setting,
                         const CharacterSetting& character, const ParagraphSetting& paragraph);
QVector<double> caretStops(const QString& text, const QString& font, const TypeSetting& setting,
                           const CharacterSetting& character, const ParagraphSetting& paragraph);
QByteArray previewRgba(const QString& text, const QString& font, const TypeSetting& setting,
                       const CharacterSetting& character, const ParagraphSetting& paragraph);
/// The baseline guide under each non-blank line, flat `x0, y0, x1, y1, …`.
QVector<double> baselines(const QString& text, const QString& font, const TypeSetting& setting,
                          const CharacterSetting& character, const ParagraphSetting& paragraph);
/// Whether paragraph type has text past its box (the `+` on its handle).
bool overflows(const QString& text, const QString& font, const TypeSetting& setting,
               const CharacterSetting& character, const ParagraphSetting& paragraph);

/// `text`'s box in its own frame (left, top, right, bottom; before the matrix).
QVector<double> localBounds(const QString& text, const QString& font, const TypeSetting& setting,
                            const CharacterSetting& character, const ParagraphSetting& paragraph);

bool isParagraph(const PictureView& view, const QString& path);
/// Type > Convert To Paragraph Text (`paragraph`) / Convert To Point Text: one
/// state; false for a layer that is not type of the other kind.
bool convert(PictureView& view, const QString& path, bool paragraph);

QString commitLayer(PictureView& view, const QString& text, const QString& font,
                    const TypeSetting& setting, const CharacterSetting& character,
                    const ParagraphSetting& paragraph);
bool commitMask(PictureView& view, const QString& text, const QString& font,
                const TypeSetting& setting, const CharacterSetting& character,
                const ParagraphSetting& paragraph, const QString& mode);
bool updateLayer(PictureView& view, const QString& path, const QString& text, const QString& font,
                 const TypeSetting& setting, const CharacterSetting& character,
                 const ParagraphSetting& paragraph);
bool applyStyle(PictureView& view, const QString& path, const QString& name, bool paragraph);

bool createCharacterStyle(PictureView& view, const QString& name, const QString& font,
                          const CharacterSetting& character);
bool createParagraphStyle(PictureView& view, const QString& name, const QString& font,
                          const CharacterSetting& character, const ParagraphSetting& paragraph);
bool editParagraphStyle(PictureView& view, const QString& name, const QString& font,
                        const CharacterSetting& character, const ParagraphSetting& paragraph);
bool previewParagraphStyle(PictureView& view, const QString& name, const QString& font,
                           const CharacterSetting& character, const ParagraphSetting& paragraph);
bool deleteParagraphStyle(PictureView& view, const QString& name);

int paragraphStyleCount(const PictureView& view);
QString paragraphStyleName(const PictureView& view, int index);
CharacterSetting paragraphStyleCharacter(const PictureView& view, const QString& name);
ParagraphSetting paragraphStyleParagraph(const PictureView& view, const QString& name);
QString paragraphStyleFont(const PictureView& view, const QString& name);

bool rasterize(PictureView& view, const QString& path);

/// The reopen-hide / re-set / show-again sequence for editing an existing type
/// layer: `beginEdit` on the click hit, `commitEdit` with the retyped text on
/// release, or `cancelEdit` on Esc/click-away.
bool beginEdit(PictureView& view, const QString& path);
void cancelEdit(PictureView& view, const QString& path);
bool commitEdit(PictureView& view, const QString& path, const QString& text, const QString& font,
                const TypeSetting& setting, const CharacterSetting& character,
                const ParagraphSetting& paragraph);

} // namespace pictura::type_api
