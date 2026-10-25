#pragma once

#include <QtCore/QByteArray>
#include <QtCore/QPointer>
#include <QtCore/QString>
#include <QtCore/QStringList>
#include <QtCore/QVector>

#include <rust/cxx.h>

#include <cstdint>
#include <optional>

// The one sanctioned generated-header include outside an adapter TU (#276, kept
// by #403): the shared setting structs below are the type unit's vocabulary.
#include "pictura_app/src/cxxqt_object/type_tools.cxxqt.h"
#include "pictura_app/src/cxxqt_object/type_tools/warp.cxxqt.h"

namespace pictura {
class PictureView;
}

namespace pictura::type_api {

using pictura::CharacterSetting;
using pictura::ParagraphSetting;
using pictura::TypeSetting;

/// Everything that sets a run of type: its text, font, placement setting and
/// character / paragraph attributes. Previews, commits and updates take one.
struct TypeSpec {
    QString text;
    QString font;
    TypeSetting setting;
    CharacterSetting character;
    ParagraphSetting paragraph;
};

/// A type layer's editable state, read in one bundle; defaults with `isType`
/// false for a layer that is not type.
struct LayerType {
    bool isType = false;
    TypeSpec spec;
};

/// The single read of a layer's type state (replaces the five per-field
/// crossings the Type tool and the panels ran on every open/refresh).
LayerType readLayer(const PictureView& view, const QString& path);
/// Whether `path` is type carrying a Warp Text warp (the Layers panel badge).
bool isWarped(const PictureView& view, const QString& path);

QString layerAt(const PictureView& view, double x, double y);
/// The path under `(x, y)`: kind, closed, start, segment count, then eight
/// numbers per cubic segment; empty when no path is hit.
QVector<double> pathAt(const PictureView& view, double x, double y, double tolerance);

CharacterSetting defaultCharacterSetting();
ParagraphSetting defaultParagraphSetting();
bool registerFont(const QString& family, const QString& style,
                  ::rust::Slice<const std::uint8_t> bytes);
void setMissingGlyphProtection(bool on);
bool missingGlyphProtection();
void refreshMissingGlyphProtection(PictureView& view);
/// The Character panel's OpenType features `family`/`style`'s face carries, one bit
/// each in panel order (bit 0 Standard Ligatures … bit 7 Fractions).
std::uint32_t openTypeFeatures(const QString& family, const QString& style);

QVector<int> previewRect(const TypeSpec& spec);
QVector<double> caretStops(const TypeSpec& spec);
QByteArray previewRgba(const TypeSpec& spec);
/// The baseline guide under each non-blank line, flat `x0, y0, x1, y1, …`.
QVector<double> baselines(const TypeSpec& spec);
/// Whether paragraph type has text past its box (the `+` on its handle).
bool overflows(const TypeSpec& spec);
/// The overflow `+` point for on-path type, `[x, y]`; empty otherwise.
QVector<double> pathOverflowPoint(const TypeSpec& spec);
/// `spec`'s box in its own frame (left, top, right, bottom; before the matrix).
QVector<double> localBounds(const TypeSpec& spec);

bool isParagraph(const PictureView& view, const QString& path);
/// Type > Convert To Paragraph Text (`paragraph`) / Convert To Point Text: one
/// state; false for a layer that is not type of the other kind.
bool convert(PictureView& view, const QString& path, bool paragraph);

QString commitLayer(PictureView& view, const TypeSpec& spec);
bool commitMask(PictureView& view, const TypeSpec& spec, const QString& mode);
bool updateLayer(PictureView& view, const QString& path, const TypeSpec& spec);
bool applyStyle(PictureView& view, const QString& path, const QString& name, bool paragraph);

/// One character style, resolved against the style sheet.
struct CharacterStyle {
    QString name;
    QString font;
    CharacterSetting character;
};
/// One paragraph style, resolved against the style sheet.
struct ParagraphStyle {
    QString name;
    QString font;
    CharacterSetting character;
    ParagraphSetting paragraph;
};

/// The document's character styles in creation order, in one read.
QVector<CharacterStyle> characterStyles(const PictureView& view);
/// The document's paragraph styles in panel order (Basic Paragraph first), in
/// one read.
QVector<ParagraphStyle> paragraphStyles(const PictureView& view);

/// The style named `name` in `styles`; an unnamed style with the default
/// attribute sets when there is none.
CharacterStyle findStyle(const QVector<CharacterStyle>& styles, const QString& name);
ParagraphStyle findStyle(const QVector<ParagraphStyle>& styles, const QString& name);

bool createCharacterStyle(PictureView& view, const QString& name, const QString& font,
                          const CharacterSetting& character);
bool editCharacterStyle(PictureView& view, const QString& name, const QString& newName,
                        const QString& font, const CharacterSetting& character);
bool previewCharacterStyle(PictureView& view, const QString& name, const QString& font,
                           const CharacterSetting& character);
bool deleteCharacterStyle(PictureView& view, const QString& name);
bool createParagraphStyle(PictureView& view, const QString& name, const QString& font,
                          const CharacterSetting& character, const ParagraphSetting& paragraph);
bool editParagraphStyle(PictureView& view, const QString& name, const QString& font,
                        const CharacterSetting& character, const ParagraphSetting& paragraph);
bool previewParagraphStyle(PictureView& view, const QString& name, const QString& font,
                           const CharacterSetting& character, const ParagraphSetting& paragraph);
bool deleteParagraphStyle(PictureView& view, const QString& name);

bool rasterize(PictureView& view, const QString& path);

/// Re-editing an existing type layer: `begin` hides the layer under the click
/// (no state) while its text is retyped over it, then exactly one of `commit`
/// (one "Edit Type Layer" state) or `cancel` (shown again unchanged) ends it.
/// A session destroyed while open cancels; one whose view is gone does nothing.
class EditSession {
public:
    /// The open session for the type layer at `path`; none when it is not type.
    static std::optional<EditSession> begin(PictureView& view, const QString& path);

    EditSession(EditSession&& other) noexcept;
    EditSession& operator=(EditSession&& other) noexcept;
    EditSession(const EditSession&) = delete;
    EditSession& operator=(const EditSession&) = delete;
    ~EditSession();

    const QString& path() const { return path_; }
    bool isOpen() const { return open_; }
    /// Re-set the layer from `spec`; ends the session either way (a refused
    /// commit shows the layer again unchanged).
    bool commit(const TypeSpec& spec);
    void cancel();
    /// End without touching the layer: an in-place document replacement or a
    /// history restore already put it back.
    void abandon() { open_ = false; }

private:
    EditSession(PictureView& view, const QString& path);

    QPointer<PictureView> view_;
    QString path_;
    bool open_ = false;
};

/// `Type > Warp Text`: `style` is a `WarpStyle` id (`arc`, `arcLower`, ...),
/// empty for none; the values are percentages.
struct WarpSetting {
    QString style;
    double bend = 0.0;
    double horizontal = 0.0;
    double vertical = 0.0;
    bool verticalOrientation = false;
};

/// Why Warp Text refuses a layer.
enum class WarpRefusal { None, OnPath, FauxBold, NoOutlines, NotType };

WarpSetting layerWarp(const PictureView& view, const QString& path);
WarpRefusal warpRefusal(const PictureView& view, const QString& path);

/// The Warp Text dialog's live preview of one type layer: `preview` re-sets the
/// layer with no state (the first stashes the layer as it was), then exactly
/// one of `commit` (one "Warp Text" state) or `cancel` (the stash put back)
/// ends it. A session destroyed while open cancels; one whose view is gone
/// does nothing.
class WarpSession {
public:
    WarpSession(PictureView& view, const QString& path);
    WarpSession(const WarpSession&) = delete;
    WarpSession& operator=(const WarpSession&) = delete;
    ~WarpSession();

    bool isOpen() const { return open_; }
    bool preview(const WarpSetting& warp);
    bool commit(const WarpSetting& warp);
    /// True when a preview's stash was put back (the canvas changed).
    bool cancel();

private:
    QPointer<PictureView> view_;
    QString path_;
    bool open_ = true;
    bool previewed_ = false;
};

/// Blend Text Colors Using Gamma, for every document (`gamma` 1.00–2.20).
void setTextGamma(bool enabled, double gamma);
/// Recomposite `view` after the text gamma changed; no history.
void refreshTextGamma(PictureView& view);
/// Every type layer's font name, each once.
QStringList documentFonts(const PictureView& view);
/// Whether the document's type layers can be re-set (8-bit RGB only).
bool layersEditable(const PictureView& view);
/// Re-set every type layer using one of `missing` in the default font as one
/// "Replace All Missing Fonts" state; the number of layers changed.
int replaceFonts(PictureView& view, const QStringList& missing);

} // namespace pictura::type_api
